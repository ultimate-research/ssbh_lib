use binread::{io::StreamPosition, BinRead};
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    io::{Cursor, Read, Seek, Write},
    path::Path,
};

use ssbh_write::SsbhWrite;

use ssbh_lib::formats::anim::{
    Anim, AnimGroup, AnimHeader, AnimHeaderV20, AnimHeaderV21, AnimNode, AnimTrackV2,
    CompressionType, TrackFlags, TrackType, UnkData, UnkTrackFlags,
};

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use ssbh_lib::{Vector3, Vector4, formats::anim::GroupType};

mod buffers;
use buffers::*;
mod compression;

use crate::SsbhData;

// TODO: Add module level documentation to show anim <-> data conversions and describe overall structure and design.

/// The data associated with an [Anim] file.
/// Supported versions are 2.0 and 2.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct AnimData {
    pub major_version: u16,
    pub minor_version: u16,
    // TODO: Is float the best choice here?
    // TODO: Just use a usize but return an error on invalid lengths?
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    ///
    /// Constant animations will last for final_frame_index + 1 many frames.
    pub final_frame_index: f32,
    pub groups: Vec<GroupData>,
}

impl SsbhData for AnimData {
    type WriteError = AnimError;

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Anim::from_file(path)?.try_into()
    }

    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        Anim::read(reader)?.try_into()
    }

    fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> Result<(), AnimError> {
        Anim::try_from(self)?.write(writer)?;
        Ok(())
    }

    fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AnimError> {
        Anim::try_from(self)?.write_to_file(path)?;
        Ok(())
    }
}

// TODO: Test these conversions.
impl TryFrom<Anim> for AnimData {
    type Error = Box<dyn Error>;

    fn try_from(anim: Anim) -> Result<Self, Self::Error> {
        (&anim).try_into()
    }
}

impl TryFrom<&Anim> for AnimData {
    type Error = Box<dyn Error>;

    fn try_from(anim: &Anim) -> Result<Self, Self::Error> {
        Ok(Self {
            major_version: anim.major_version,
            minor_version: anim.minor_version,
            final_frame_index: match &anim.header {
                AnimHeader::HeaderV1(h) => h.final_frame_index,
                AnimHeader::HeaderV20(h) => h.final_frame_index,
                AnimHeader::HeaderV21(h) => h.final_frame_index,
            },
            groups: read_anim_groups(anim)?,
        })
    }
}

impl TryFrom<&AnimData> for Anim {
    type Error = AnimError;

    fn try_from(data: &AnimData) -> Result<Self, Self::Error> {
        create_anim(data)
    }
}

/// Errors while creating an [Anim] from [AnimData].
#[derive(Error, Debug)]

pub enum AnimError {
    /// Creating an [Anim] file for the given version is not supported.
    #[error(
        "Creating a version {}.{} anim is not supported.",
        major_version,
        minor_version
    )]
    UnsupportedVersion {
        major_version: u16,
        minor_version: u16,
    },

    /// The final frame index is negative or smaller than the
    // index of the final frame in the longest track.
    #[error(
        "Final frame index {} must be non negative and at least as 
         large as the index of the final frame in the longest track.",
        final_frame_index
    )]
    InvalidFinalFrameIndex { final_frame_index: f32 },

    #[error(
        "Scale options of {:?} cannot be preserved for a {} track.",
        scale_options,
        if *compressed {"compressed"} else { "uncompressed"}
    )]
    UnsupportedTrackScaleOptions {
        scale_options: ScaleOptions,
        compressed: bool,
    },

    /// An error occurred while writing data to a buffer.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error occurred while reading data from a buffer.
    #[error(transparent)]
    BinRead(#[from] binread::error::Error),

    /// An error occurred while reading compressed data from a buffer.
    #[error(transparent)]
    BitError(#[from] bitbuffer::BitError),
}

enum AnimVersion {
    Version20,
    Version21,
}

// TODO: Test this for a small example?
fn create_anim(data: &AnimData) -> Result<Anim, AnimError> {
    let version = match (data.major_version, data.minor_version) {
        (2, 0) => Ok(AnimVersion::Version20),
        (2, 1) => Ok(AnimVersion::Version21),
        _ => Err(AnimError::UnsupportedVersion {
            major_version: data.major_version,
            minor_version: data.minor_version,
        }),
    }?;

    let mut buffer = Cursor::new(Vec::new());

    let animations = data
        .groups
        .iter()
        .map(|g| create_anim_group(g, &mut buffer))
        .collect::<Result<Vec<_>, _>>()?;

    let max_frame_count = animations
        .iter()
        .filter_map(|a| {
            a.nodes
                .elements
                .iter()
                .filter_map(|n| n.tracks.elements.iter().map(|t| t.frame_count).max())
                .max()
        })
        .max()
        .unwrap_or(0);

    // Make sure the final frame index is at least as large as the final frame of the longest animation.
    let final_frame_index = if data.final_frame_index >= 0.0
        && data.final_frame_index >= max_frame_count as f32 - 1.0
    {
        Ok(data.final_frame_index)
    } else {
        Err(AnimError::InvalidFinalFrameIndex {
            final_frame_index: data.final_frame_index,
        })
    }?;

    let header = match version {
        AnimVersion::Version20 => AnimHeader::HeaderV20(AnimHeaderV20 {
            final_frame_index,
            unk1: 1,
            unk2: 3,
            name: "".into(), // TODO: this is usually based on file name?
            groups: animations.into(),
            buffer: buffer.into_inner().into(),
        }),
        AnimVersion::Version21 => AnimHeader::HeaderV21(AnimHeaderV21 {
            final_frame_index,
            unk1: 1,
            unk2: 3,
            name: "".into(), // TODO: this is usually based on file name?
            groups: animations.into(),
            buffer: buffer.into_inner().into(),
            // TODO: Research how to rebuild the extra header data.
            unk_data: UnkData {
                unk1: Vec::new().into(),
                unk2: Vec::new().into(),
            },
        }),
    };

    // TODO: Check that the header matches the version number?
    let anim = Anim {
        major_version: data.major_version,
        minor_version: data.minor_version,
        header,
    };
    Ok(anim)
}

fn create_anim_group(g: &GroupData, buffer: &mut Cursor<Vec<u8>>) -> Result<AnimGroup, AnimError> {
    Ok(AnimGroup {
        group_type: g.group_type.into(),
        nodes: g
            .nodes
            .iter()
            .map(|n| create_anim_node(n, buffer))
            .collect::<Result<Vec<_>, _>>()?
            .into(),
    })
}

fn create_anim_node(n: &NodeData, buffer: &mut Cursor<Vec<u8>>) -> Result<AnimNode, AnimError> {
    Ok(AnimNode {
        name: n.name.as_str().into(), // TODO: Make a convenience method for this?
        tracks: n
            .tracks
            .iter()
            .map(|t| create_anim_track_v2(buffer, t))
            .collect::<Result<Vec<_>, _>>()?
            .into(),
    })
}

fn create_anim_track_v2(
    buffer: &mut Cursor<Vec<u8>>,
    t: &TrackData,
) -> Result<AnimTrackV2, AnimError> {
    let compression_type = infer_optimal_compression_type(&t.values);

    // The current stream position matches the offsets used for Smash Ultimate's anim files.
    // This assumes we traverse the heirarchy (group -> node -> track) in DFS order.
    let pos_before = buffer.stream_pos()?;

    // Pointers for compressed data are relative to the start of the track's data.
    // This requires using a second writer due to how SsbhWrite is implemented.
    let mut track_data = Cursor::new(Vec::new());

    // TODO: Add tests for preserving scale compensation?.
    t.values.write(
        &mut track_data,
        compression_type,
        t.scale_options.inherit_scale,
        t.scale_options.compensate_scale,
    )?;

    buffer.write_all(&track_data.into_inner())?;
    let pos_after = buffer.stream_pos()?;

    Ok(AnimTrackV2 {
        name: t.name.as_str().into(),
        flags: TrackFlags {
            track_type: t.values.track_type(),
            compression_type,
        },
        frame_count: t.values.len() as u32,
        unk_flags: UnkTrackFlags::new(), // TODO: preserve these flags?
        data_offset: pos_before as u32,
        data_size: pos_after - pos_before,
    })
}

fn infer_optimal_compression_type(values: &TrackValues) -> CompressionType {
    match (values, values.len()) {
        // Single frame animations use a special compression type.
        (TrackValues::Transform(_), 0..=1) => CompressionType::ConstTransform,
        (_, 0..=1) => CompressionType::Constant,
        _ => {
            // The compressed header adds some overhead, so we need to also check frame count.
            // Once there are enough elements to exceed the header size, compression starts to save space.

            // TODO: Is integer division correct here?
            let uncompressed_frames_per_header =
                values.compressed_overhead_in_bytes() / values.data_size_in_bytes();

            // Some tracks overlap the default data with the compression to save space.
            // This calculation assumes we aren't performing that optimization.
            if values.len() > uncompressed_frames_per_header as usize + 1 {
                CompressionType::Compressed
            } else {
                CompressionType::Direct
            }
        }
    }
}

// TODO: Test conversions from anim?
fn read_anim_groups(anim: &Anim) -> Result<Vec<GroupData>, AnimError> {
    match &anim.header {
        // TODO: Create fake groups for version 1.0?
        ssbh_lib::formats::anim::AnimHeader::HeaderV1(_) => Err(AnimError::UnsupportedVersion {
            major_version: anim.major_version,
            minor_version: anim.minor_version,
        }),
        ssbh_lib::formats::anim::AnimHeader::HeaderV20(header) => {
            read_anim_groups_v20(&header.groups.elements, &header.buffer.elements)
        }
        ssbh_lib::formats::anim::AnimHeader::HeaderV21(header) => {
            read_anim_groups_v20(&header.groups.elements, &header.buffer.elements)
        }
    }
}

fn read_anim_groups_v20(
    anim_groups: &[ssbh_lib::formats::anim::AnimGroup],
    anim_buffer: &[u8],
) -> Result<Vec<GroupData>, AnimError> {
    let mut groups = Vec::new();

    // TODO: Return a more meaningful error type.
    for anim_group in anim_groups {
        let mut nodes = Vec::new();

        for anim_node in &anim_group.nodes.elements {
            let mut tracks = Vec::new();
            for anim_track in &anim_node.tracks.elements {
                // Find and read the track data.
                let track = create_track_data_v20(anim_track, anim_buffer)?;
                tracks.push(track);
            }

            let node = NodeData {
                name: anim_node.name.to_string_lossy(),
                tracks,
            };
            nodes.push(node);
        }

        let group = GroupData {
            group_type: anim_group.group_type,
            nodes,
        };
        groups.push(group);
    }

    Ok(groups)
}

// TODO: Add tests for preserving scale inheritance and compensate scale.
fn create_track_data_v20(
    anim_track: &ssbh_lib::formats::anim::AnimTrackV2,
    anim_buffer: &[u8],
) -> Result<TrackData, AnimError> {
    let start = anim_track.data_offset as usize;
    let end = start + anim_track.data_size as usize;
    let buffer = &anim_buffer[start..end];
    let (values, inherit_scale, compensate_scale) =
        read_track_values(buffer, anim_track.flags, anim_track.frame_count as usize)?;
    Ok(TrackData {
        name: anim_track.name.to_string_lossy(),
        values,
        scale_options: ScaleOptions {
            inherit_scale,
            compensate_scale,
        },
    })
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct GroupData {
    /// The usage type for all the [NodeData] in [nodes](#structfield.nodes)
    pub group_type: GroupType,
    pub nodes: Vec<NodeData>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct NodeData {
    pub name: String,
    pub tracks: Vec<TrackData>,
}

/// The data associated with an [AnimTrackV2].
///
/// # Examples
/// The scale settings can usually be left at the default value.
/**
```rust
use ssbh_data::anim_data::{TrackData, TrackValues, ScaleOptions, Transform};

let track = TrackData {
    name: "Transform".to_string(),
    values: TrackValues::Transform(vec![Transform::IDENTITY]),
    scale_options: ScaleOptions::default()
};
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct TrackData {
    /// The name of the property to animate.
    pub name: String,

    pub scale_options: ScaleOptions,

    /// The frame values for the property specified by [name](#structfield.name).
    ///
    /// Each element in the [TrackValues] provides the value for a single frame.
    /// If the [TrackValues] contains a single element, this track will be considered constant
    /// and repeat that element for each frame in the animation
    /// up to and including [final_frame_index](struct.AnimData.html#structfield.final_frame_index).
    pub values: TrackValues,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct ScaleOptions {
    /// Accumulate the parent's scaling when `true`.
    ///
    /// The global scale in world space, is `current_scale * parent_scale` applied recursively on the parent.
    pub inherit_scale: bool,

    /// Revert the scaling of the immediate parent when `true`.
    ///
    /// The final scale relative to the parent is `current_scale * (1 / parent_scale)`.
    /// For Smash Ultimate, this is not applied recursively on the parent,
    /// so only the immediate parent's scaling is taken into account.
    pub compensate_scale: bool,
}

impl Default for ScaleOptions {
    fn default() -> Self {
        // Uncompressed tracks don't allow disabling scale inheritance.
        // Defaulting to true avoids a potential error.
        Self {
            inherit_scale: true,
            compensate_scale: false,
        }
    }
}

// TODO: Investigate if the names based on the Anim 1.2 property names are accurate.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, BinRead, PartialEq, SsbhWrite, Default, Clone, Copy)]
pub struct UvTransform {
    pub scale_u: f32,
    pub scale_v: f32,
    pub rotation: f32,
    pub translate_u: f32,
    pub translate_v: f32,
}

/// A decomposed transformation consisting of a scale, rotation, and translation.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Transform {
    /// XYZ scale
    pub scale: Vector3,
    /// An XYZW unit quaternion where XYZ represent the axis component
    /// and w represents the angle component.
    pub rotation: Vector4,
    /// XYZ translation
    pub translation: Vector3,
}

impl Transform {
    /// An identity transformation representing no scale, rotation, or translation.
    pub const IDENTITY: Transform = Transform {
        scale: Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        rotation: Vector4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        translation: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    };
}

/// A value collection with an element for each frame of the animation.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub enum TrackValues {
    /// Transformations used for camera or skeletal animations.
    Transform(Vec<Transform>),
    /// Transformations applied to UV coordinates for texture animations.
    UvTransform(Vec<UvTransform>),
    /// Animated scalar parameter values.
    Float(Vec<f32>),
    PatternIndex(Vec<u32>),
    /// Visibility animations or animated boolean parameters.
    Boolean(Vec<bool>),
    /// Material animations or animated vector parameters.
    Vector4(Vec<Vector4>),
}

impl TrackValues {
    /// Returns the number of elements, which is equivalent to the number of frames.
    /// # Examples
    /**
    ```rust
    # use ssbh_data::anim_data::TrackValues;
    assert_eq!(3, TrackValues::Boolean(vec![true, false, true]).len());
    ```
     */
    pub fn len(&self) -> usize {
        match self {
            TrackValues::Transform(v) => v.len(),
            TrackValues::UvTransform(v) => v.len(),
            TrackValues::Float(v) => v.len(),
            TrackValues::PatternIndex(v) => v.len(),
            TrackValues::Boolean(v) => v.len(),
            TrackValues::Vector4(v) => v.len(),
        }
    }

    /// Returns `true` there are no elements.
    /**
    ```rust
    # use ssbh_data::anim_data::TrackValues;
    assert!(TrackValues::Transform(Vec::new()).is_empty());
    ```
     */
    pub fn is_empty(&self) -> bool {
        match self {
            TrackValues::Transform(v) => v.is_empty(),
            TrackValues::UvTransform(v) => v.is_empty(),
            TrackValues::Float(v) => v.is_empty(),
            TrackValues::PatternIndex(v) => v.is_empty(),
            TrackValues::Boolean(v) => v.is_empty(),
            TrackValues::Vector4(v) => v.is_empty(),
        }
    }

    fn track_type(&self) -> TrackType {
        match self {
            TrackValues::Transform(_) => TrackType::Transform,
            TrackValues::UvTransform(_) => TrackType::UvTransform,
            TrackValues::Float(_) => TrackType::Float,
            TrackValues::PatternIndex(_) => TrackType::PatternIndex,
            TrackValues::Boolean(_) => TrackType::Boolean,
            TrackValues::Vector4(_) => TrackType::Vector4,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_hex_eq;
    use hexlit::hex;
    use ssbh_lib::formats::anim::AnimHeaderV21;

    use super::*;

    // TODO: Test the conversions more thoroughly.

    #[test]
    fn create_empty_anim_v_2_0() {
        let anim = create_anim(&AnimData {
            major_version: 2,
            minor_version: 0,
            final_frame_index: 1.5,
            groups: Vec::new(),
        })
        .unwrap();

        assert!(matches!(
            anim.header,
            AnimHeader::HeaderV20(AnimHeaderV20 {
                final_frame_index,
                ..
            }) if final_frame_index == 1.5
        ));
    }

    #[test]
    fn create_empty_anim_v_2_1() {
        let anim = create_anim(&AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: 2.5,
            groups: Vec::new(),
        })
        .unwrap();

        assert!(matches!(anim.header, AnimHeader::HeaderV21(AnimHeaderV21 {
            final_frame_index, 
            ..
        }) if final_frame_index == 2.5));
    }

    #[test]
    fn create_anim_negative_frame_index() {
        let result = create_anim(&AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: -1.0,
            groups: Vec::new(),
        });

        assert!(matches!(
            result,
            Err(AnimError::InvalidFinalFrameIndex {
                final_frame_index
            }) if final_frame_index == -1.0
        ));
    }

    #[test]
    fn create_anim_insufficient_frame_index() {
        let result = create_anim(&AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: 2.0,
            groups: vec![GroupData {
                group_type: GroupType::Visibility,
                nodes: vec![NodeData {
                    name: String::new(),
                    tracks: vec![TrackData {
                        name: String::new(),
                        values: TrackValues::Boolean(vec![true; 4]),
                        scale_options: ScaleOptions::default(),
                    }],
                }],
            }],
        });

        // A value of at least 3.0 is expected.
        assert!(matches!(
            result,
            Err(AnimError::InvalidFinalFrameIndex {
                final_frame_index
            }) if final_frame_index == 2.0
        ));
    }

    #[test]
    fn create_anim_zero_frame_index() {
        let anim = create_anim(&AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: 0.0,
            groups: Vec::new(),
        })
        .unwrap();

        assert!(matches!(anim.header, AnimHeader::HeaderV21(AnimHeaderV21 {
            final_frame_index, 
            ..
        }) if final_frame_index == 0.0));
    }

    #[test]
    fn create_empty_anim_invalid_version() {
        let result = create_anim(&AnimData {
            major_version: 1,
            minor_version: 2,
            final_frame_index: 0.0,
            groups: Vec::new(),
        });

        assert!(matches!(
            result,
            Err(AnimError::UnsupportedVersion {
                major_version: 1,
                minor_version: 2
            })
        ));
    }

    #[test]
    fn create_node_no_tracks() {
        let node = NodeData {
            name: "empty".to_string(),
            tracks: Vec::new(),
        };

        let mut buffer = Cursor::new(Vec::new());

        let anim_node = create_anim_node(&node, &mut buffer).unwrap();
        assert_eq!("empty", anim_node.name.to_str().unwrap());
        assert!(anim_node.tracks.elements.is_empty());
    }

    #[test]
    fn create_node_multiple_tracks() {
        let node = NodeData {
            name: "empty".to_string(),
            tracks: vec![
                TrackData {
                    name: "t1".to_string(),
                    values: TrackValues::Float(vec![1.0, 2.0, 3.0]),
                    scale_options: ScaleOptions::default(),
                },
                TrackData {
                    name: "t2".to_string(),
                    values: TrackValues::PatternIndex(vec![4, 5]),
                    scale_options: ScaleOptions::default(),
                },
            ],
        };

        let mut buffer = Cursor::new(Vec::new());

        let anim_node = create_anim_node(&node, &mut buffer).unwrap();
        assert_eq!("empty", anim_node.name.to_str().unwrap());
        assert_eq!(2, anim_node.tracks.elements.len());

        let t1 = &anim_node.tracks.elements[0];
        assert_eq!("t1", t1.name.to_str().unwrap());
        assert_eq!(
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Direct
            },
            t1.flags
        );
        assert_eq!(3, t1.frame_count);
        assert_eq!(0, t1.data_offset);
        assert_eq!(12, t1.data_size);

        let t2 = &anim_node.tracks.elements[1];
        assert_eq!("t2", t2.name.to_str().unwrap());
        assert_eq!(
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Direct
            },
            t2.flags
        );
        assert_eq!(2, t2.frame_count);
        assert_eq!(12, t2.data_offset);
        assert_eq!(8, t2.data_size);
    }

    #[test]
    fn compression_type_empty() {
        assert_eq!(
            CompressionType::ConstTransform,
            infer_optimal_compression_type(&TrackValues::Transform(Vec::new()))
        );
        assert_eq!(
            CompressionType::Constant,
            infer_optimal_compression_type(&TrackValues::UvTransform(Vec::new()))
        );
        assert_eq!(
            CompressionType::Constant,
            infer_optimal_compression_type(&TrackValues::Float(Vec::new()))
        );
        assert_eq!(
            CompressionType::Constant,
            infer_optimal_compression_type(&TrackValues::PatternIndex(Vec::new()))
        );
        assert_eq!(
            CompressionType::Constant,
            infer_optimal_compression_type(&TrackValues::Boolean(Vec::new()))
        );
        assert_eq!(
            CompressionType::Constant,
            infer_optimal_compression_type(&TrackValues::Vector4(Vec::new()))
        );
    }

    #[test]
    fn compression_type_boolean_multiple_frames() {
        // The compression adds 33 bytes of overhead.
        // The uncompressed representation for a bool is 1 byte.
        // We need more than (33 / 1 + 1) frames for compression to save space.
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Boolean(vec![true; 8]))
        );
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Boolean(vec![true; 34]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Boolean(vec![true; 35]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Boolean(vec![true; 100]))
        );
    }

    #[test]
    fn compression_type_float_multiple_frames() {
        // The compression adds 36 bytes of overhead.
        // The uncompressed representation for a float is 4 bytes.
        // We need more than 10 (36 / 4 + 1) frames for compression to save space.
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Float(vec![0.0; 8]))
        );
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Float(vec![0.0; 10]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Float(vec![0.0; 11]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Float(vec![0.0; 100]))
        );
    }

    #[test]
    fn compression_type_pattern_index_multiple_frames() {
        // The compression adds 36 bytes of overhead.
        // The uncompressed representation for a float is 4 bytes.
        // We need more than 10 (36 / 4 + 1) frames for compression to save space.
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::PatternIndex(vec![0; 8]))
        );
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::PatternIndex(vec![0; 10]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::PatternIndex(vec![0; 11]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::PatternIndex(vec![0; 100]))
        );
    }

    #[test]
    fn compression_type_uv_transform_multiple_frames() {
        // The compression adds 116 bytes of overhead.
        // The uncompressed representation for a UV transform is 20 bytes.
        // We need more than 6.8 (116 / 20 + 1) frames for compression to save space.
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::UvTransform(vec![
                UvTransform::default();
                3
            ]))
        );
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::UvTransform(vec![
                UvTransform::default();
                6
            ]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::UvTransform(vec![
                UvTransform::default();
                7
            ]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::UvTransform(vec![
                UvTransform::default();
                100
            ]))
        );
    }

    #[test]
    fn compression_type_vector4_multiple_frames() {
        // The compression adds 96 bytes of overhead.
        // The uncompressed representation for a UV transform is 20 bytes.
        // We need more than 7 (96 / 16 + 1) frames for compression to save space.
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Vector4(vec![Vector4::default(); 3]))
        );
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Vector4(vec![Vector4::default(); 7]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Vector4(vec![Vector4::default(); 8]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Vector4(vec![Vector4::default(); 100]))
        );
    }

    #[test]
    fn compression_type_transform_multiple_frames() {
        // The compression adds 204 bytes of overhead.
        // The uncompressed representation for a transform is 44 bytes.
        // We need more than 5.63 (204 / 44 + 1) frames for compression to save space.
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Transform(vec![Transform::default(); 3]))
        );
        assert_eq!(
            CompressionType::Direct,
            infer_optimal_compression_type(&TrackValues::Transform(vec![Transform::default(); 5]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Transform(vec![Transform::default(); 6]))
        );
        assert_eq!(
            CompressionType::Compressed,
            infer_optimal_compression_type(&TrackValues::Transform(vec![
                Transform::default();
                100
            ]))
        );
    }

    #[test]
    fn read_v20_track_compressed_inherit_scale() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        // Scale type set to 2 to test for scale inheritance.
        let buffer = hex!(
            // header
            04000600 a0002b00 cc000000 02000000
            // scale compression
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            // rotation compression
            00000000 b9bc433d 0d000000 00000000
            e27186bd 00000000 0d000000 00000000
            00000000 ada2273f 10000000 00000000
            // translation compression
            16a41d40 16a41d40 10000000 00000000
            00000000 00000000 10000000 00000000
            00000000 00000000 10000000 00000000
            // default value
            0000803f 0000803f 0000803f
            00000000 00000000 00000000 0000803f
            16a41d40 00000000 00000000 00000000 00e0ff03
            // compressed values
            00f8ff00 e0ff1f
        );

        let data = create_track_data_v20(
            &AnimTrackV2 {
                name: "abc".into(),
                flags: TrackFlags {
                    track_type: TrackType::Transform,
                    compression_type: CompressionType::Compressed,
                },
                frame_count: 2,
                unk_flags: UnkTrackFlags::new(),
                data_offset: 0,
                data_size: buffer.len() as u64,
            },
            &buffer,
        )
        .unwrap();

        // TODO: This should test the values, but this overlaps with anim_buffer tests?
        assert_eq!("abc", data.name);
        assert_eq!(true, data.scale_options.inherit_scale);
    }

    #[test]
    fn write_v20_track_compressed_inherit_scale() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        // The header scale type should be 2 here instead of 1.
        let buffer = hex!(
            // header
            04000e00 a0002b00 cc000000 02000000
            // scale compression
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            // rotation compression
            00000000 b9bc433d 0d000000 00000000
            e27186bd 00000000 0d000000 00000000
            00000000 ada2273f 10000000 00000000
            // translation compression
            16a41d40 16a41d40 10000000 00000000
            00000000 00000000 10000000 00000000
            00000000 00000000 10000000 00000000
            // default value
            0000803f 0000803f 0000803f
            00000000 00000000 00000000 0000803f
            16a41d40 00000000 00000000 00000000 00e0ff03
            // compressed values
            00f8ff00 e0ff1f
        );

        // Use a large enough frame count to ensure the writer chooses compression.
        let mut writer = Cursor::new(buffer.to_vec());
        let track = create_anim_track_v2(
            &mut writer,
            &TrackData {
                name: "abc".into(),
                values: TrackValues::Transform(vec![Transform::default(); 64]),
                scale_options: ScaleOptions {
                    inherit_scale: true,
                    compensate_scale: false,
                },
            },
        )
        .unwrap();

        assert_eq!("abc", track.name.to_string_lossy());
        assert_eq!(64, track.frame_count);
        // TODO: This overlaps with anim_buffer tests?
        // Just check the header flags for now.
        assert_hex_eq!(&writer.get_ref()[..4], &buffer[..4]);
    }

    #[test]
    fn read_v20_track_compressed_no_scale_inheritance() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        // The header scale type was changed to 1.
        let buffer = hex!(
            // header
            04000500 a0002b00 cc000000 02000000
            // scale compression
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            // rotation compression
            00000000 b9bc433d 0d000000 00000000
            e27186bd 00000000 0d000000 00000000
            00000000 ada2273f 10000000 00000000
            // translation compression
            16a41d40 16a41d40 10000000 00000000
            00000000 00000000 10000000 00000000
            00000000 00000000 10000000 00000000
            // default value
            0000803f 0000803f 0000803f
            00000000 00000000 00000000 0000803f
            16a41d40 00000000 00000000 00000000 00e0ff03
            // compressed values
            00f8ff00 e0ff1f
        );

        let data = create_track_data_v20(
            &AnimTrackV2 {
                name: "abc".into(),
                flags: TrackFlags {
                    track_type: TrackType::Transform,
                    compression_type: CompressionType::Compressed,
                },
                frame_count: 2,
                unk_flags: UnkTrackFlags::new(),
                data_offset: 0,
                data_size: buffer.len() as u64,
            },
            &buffer,
        )
        .unwrap();

        // TODO: This should test the values, but this overlaps with anim_buffer tests?
        assert_eq!("abc", data.name);
        assert_eq!(false, data.scale_options.inherit_scale);
    }

    #[test]
    fn write_v20_track_compressed_no_scale_inheritance() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        // The header scale type should be 1 here instead of 2.
        let buffer = hex!(
            // header
            04000d00 a0002b00 cc000000 02000000
            // scale compression
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            // rotation compression
            00000000 b9bc433d 0d000000 00000000
            e27186bd 00000000 0d000000 00000000
            00000000 ada2273f 10000000 00000000
            // translation compression
            16a41d40 16a41d40 10000000 00000000
            00000000 00000000 10000000 00000000
            00000000 00000000 10000000 00000000
            // default value
            0000803f 0000803f 0000803f
            00000000 00000000 00000000 0000803f
            16a41d40 00000000 00000000 00000000 00e0ff03
            // compressed values
            00f8ff00 e0ff1f
        );

        // Use a large enough frame count to ensure the writer chooses compression.
        let mut writer = Cursor::new(buffer.to_vec());
        let track = create_anim_track_v2(
            &mut writer,
            &TrackData {
                name: "abc".into(),
                values: TrackValues::Transform(vec![Transform::default(); 64]),
                scale_options: ScaleOptions {
                    inherit_scale: false,
                    compensate_scale: false,
                },
            },
        )
        .unwrap();

        assert_eq!("abc", track.name.to_string_lossy());
        assert_eq!(64, track.frame_count);
        // Just check the header flags for now.
        assert_hex_eq!(&writer.get_ref()[..4], &buffer[..4]);
    }

    #[test]
    fn read_v20_track_uncompressed() {
        // assist/shovelknight/model/body/c00/model.nuanmb, FingerL11, Transform
        let buffer = hex!(
            0000803f 0000803f 0000803f          // scale
            00000000 00000000 00000000          // translation
            0000803f bea4c13f_79906ebe f641bebe // rotation
            01000000                            // compensate scale
        );

        let data = create_track_data_v20(
            &AnimTrackV2 {
                name: "abc".into(),
                flags: TrackFlags {
                    track_type: TrackType::Transform,
                    compression_type: CompressionType::ConstTransform,
                },
                frame_count: 1,
                unk_flags: UnkTrackFlags::new(),
                data_offset: 0,
                data_size: buffer.len() as u64,
            },
            &buffer,
        )
        .unwrap();

        // TODO: This should test the values, but this overlaps with anim_buffer tests?
        assert_eq!("abc", data.name);
        // Uncompressed transforms seem to inherit scale with ConstTransform.
        // TODO: Investigate if uncompressed transforms always inherit scale.
        assert_eq!(true, data.scale_options.inherit_scale);
    }

    #[test]
    fn write_v20_track_uncompressed_inherit_scale() {
        // assist/shovelknight/model/body/c00/model.nuanmb, FingerL11, Transform
        let mut buffer = Cursor::new(
            hex!(
                0000803f 0000803f 0000803f          // scale
                00000000 00000000 00000000          // translation
                0000803f bea4c13f_79906ebe f641bebe // rotation
                01000000                            // compensate scale
            )
            .to_vec(),
        );
        let track = create_anim_track_v2(
            &mut buffer,
            &TrackData {
                name: "abc".into(),
                values: TrackValues::Transform(vec![Transform {
                    translation: Vector3::new(1.51284, -0.232973, -0.371597),
                    rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                }]),
                scale_options: ScaleOptions {
                    inherit_scale: true,
                    compensate_scale: true,
                },
            },
        )
        .unwrap();

        assert_eq!("abc", track.name.to_string_lossy());
        assert_eq!(1, track.frame_count);
        // TODO: Test additional fields.
    }

    #[test]
    fn write_v20_track_uncompressed_no_scale_inheritance() {
        // Uncompressed tracks always use scale inheritance.
        // Single frame transform tracks won't be compressed,
        // so this is not a valid operation.
        let result = create_anim_track_v2(
            &mut Cursor::new(Vec::new()),
            &TrackData {
                name: "abc".into(),
                values: TrackValues::Transform(vec![Transform::default()]),
                scale_options: ScaleOptions {
                    inherit_scale: false,
                    compensate_scale: false,
                },
            },
        );

        assert!(matches!(
            result,
            Err(AnimError::UnsupportedTrackScaleOptions {
                scale_options: ScaleOptions {
                    inherit_scale: false,
                    compensate_scale: false
                },
                compressed: false
            })
        ));
    }
}
