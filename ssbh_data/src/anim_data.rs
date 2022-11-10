//! Types for working with [Anim] data in .nuanmb files.
//!
//! # Examples
//! Animation data is stored in a hierarchy.
//! Values for each frame are stored at the [TrackData] level.
/*!
```rust no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use ssbh_data::prelude::*;

let anim = AnimData::from_file("model.nuanmb")?;

for group in anim.groups {
    for node in group.nodes {
        for track in node.tracks {
            println!("Frame Count: {}", track.values.len());
        }
    }
}
# Ok(()) }
```
 */
//! # File Differences
//! Unmodified files are not guaranteed to be binary identical after saving.
//! Compressed animations use lossy compression for all data types except [TrackValues::Boolean].
//! When converting to [Anim], compression is enabled for a track if compression would save space.
//! This may produce differences with the original due to compression differences.
//! These errors are small in practice but may cause gameplay differences such as online desyncs.
use binrw::io::{Cursor, Seek, Write};
use binrw::{BinRead, BinReaderExt};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
pub use ssbh_lib::formats::anim::GroupType;
use ssbh_lib::{
    formats::anim::{
        Anim, CompressionType, Group, Node, TrackFlags, TrackTypeV2, TrackV2,
        TransformFlags as AnimTransformFlags, UnkData,
    },
    SsbhArray, Vector3, Vector4, Version,
};
use ssbh_write::SsbhWrite;
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
};

mod buffers;
use buffers::*;
mod bitutils;
mod compression;

/// Data associated with an [Anim] file.
/// Supported versions are 2.0 and 2.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct AnimData {
    pub major_version: u16,
    pub minor_version: u16,

    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    ///
    /// Constant animations will last for final_frame_index + 1 many frames.
    ///
    /// Frames use floating point to allow the rendering speed to differ from the animation speed.
    /// For example, some animations in Smash Ultimate interpolate when playing the game at 60fps but 1/4 speed.
    pub final_frame_index: f32,
    pub groups: Vec<GroupData>,
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
        let (major_version, minor_version) = anim.major_minor_version();
        Ok(Self {
            major_version,
            minor_version,
            final_frame_index: match &anim {
                Anim::V12 {
                    final_frame_index, ..
                } => *final_frame_index,
                Anim::V20 {
                    final_frame_index, ..
                } => *final_frame_index,
                Anim::V21 {
                    final_frame_index, ..
                } => *final_frame_index,
            },
            groups: read_anim_groups(anim)?,
        })
    }
}

impl TryFrom<AnimData> for Anim {
    type Error = error::Error;

    fn try_from(data: AnimData) -> Result<Self, Self::Error> {
        create_anim(&data)
    }
}

impl TryFrom<&AnimData> for Anim {
    type Error = error::Error;

    fn try_from(data: &AnimData) -> Result<Self, Self::Error> {
        create_anim(data)
    }
}

pub mod error {
    use super::*;
    use thiserror::Error;

    /// Errors while creating an [Anim] from [AnimData].
    #[derive(Debug, Error)]
    pub enum Error {
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
        BinRead(#[from] binrw::error::Error),

        /// An error occurred while reading compressed data from a buffer.
        #[error(transparent)]
        BitError(#[from] bitutils::BitReadError),

        #[error(
            "Compressed header bits per entry of {} does not match expected value of {}.",
            actual,
            expected
        )]
        UnexpectedBitCount { expected: usize, actual: usize },

        #[error(
            "Track data range {0}..{0}+{1} is out of range for a buffer of size {2}.",
            start,
            size,
            buffer_size
        )]
        InvalidTrackDataRange {
            start: usize,
            size: usize,
            buffer_size: usize,
        },

        /// The buffer index is not valid for a version 1.2 anim file.
        #[error(
            "Buffer index {} is out of range for a buffer collection of size {}.",
            buffer_index,
            buffer_count
        )]
        BufferIndexOutOfRange {
            buffer_index: usize,
            buffer_count: usize,
        },

        /// An error occurred while reading the compressed header for version 2.0 or later.
        #[error("The track data compression header is malformed and cannot be read.")]
        MalformedCompressionHeader,
    }
}

enum AnimVersion {
    Version20,
    Version21,
}

// TODO: Test this for a small example?
fn create_anim(data: &AnimData) -> Result<Anim, error::Error> {
    let version = match (data.major_version, data.minor_version) {
        (2, 0) => Ok(AnimVersion::Version20),
        (2, 1) => Ok(AnimVersion::Version21),
        _ => Err(error::Error::UnsupportedVersion {
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
        Err(error::Error::InvalidFinalFrameIndex {
            final_frame_index: data.final_frame_index,
        })
    }?;

    match version {
        AnimVersion::Version20 => Ok(Anim::V20 {
            final_frame_index,
            unk1: 1,
            unk2: 3,
            name: "".into(), // TODO: this is usually based on file name?
            groups: animations.into(),
            buffer: buffer.into_inner().into(),
        }),
        AnimVersion::Version21 => Ok(Anim::V21 {
            final_frame_index,
            unk1: 1,
            unk2: 3,
            name: "".into(), // TODO: this is usually based on file name?
            groups: animations.into(),
            buffer: buffer.into_inner().into(),
            // TODO: Research how to rebuild the extra header data.
            unk_data: UnkData {
                unk1: SsbhArray::new(),
                unk2: SsbhArray::new(),
            },
        }),
    }
}

fn create_anim_group(g: &GroupData, buffer: &mut Cursor<Vec<u8>>) -> Result<Group, error::Error> {
    Ok(Group {
        group_type: g.group_type,
        nodes: g
            .nodes
            .iter()
            .map(|n| create_anim_node(n, buffer))
            .collect::<Result<Vec<_>, _>>()?
            .into(),
    })
}

fn create_anim_node(n: &NodeData, buffer: &mut Cursor<Vec<u8>>) -> Result<Node, error::Error> {
    Ok(Node {
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
) -> Result<TrackV2, error::Error> {
    let compression_type = infer_optimal_compression_type(&t.values);

    // The current stream position matches the offsets used for Smash Ultimate's anim files.
    // This assumes we traverse the hierarchy (group -> node -> track) in DFS order.
    let pos_before = buffer.stream_position()?;

    // Pointers for compressed data are relative to the start of the track's data.
    // This requires using a second writer due to how SsbhWrite is implemented.
    let mut track_data = Cursor::new(Vec::new());

    // TODO: Add tests for preserving scale compensation?.
    t.values.write(
        &mut track_data,
        compression_type,
        t.scale_options.compensate_scale,
    )?;

    buffer.write_all(&track_data.into_inner())?;
    let pos_after = buffer.stream_position()?;

    Ok(TrackV2 {
        name: t.name.as_str().into(),
        flags: TrackFlags {
            track_type: t.values.track_type(),
            compression_type,
        },
        frame_count: t.values.len() as u32,
        transform_flags: t.transform_flags.into(),
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
fn read_anim_groups(anim: &Anim) -> Result<Vec<GroupData>, error::Error> {
    match anim {
        // TODO: Create fake groups for version 1.0?
        ssbh_lib::prelude::Anim::V12 {
            tracks, buffers, ..
        } => {
            // TODO: Group by type?
            // TODO: Assign a single node to each track with the track name as the name?
            // TODO: Use the track type as the track name like "Transform"?
            for track in &tracks.elements {
                create_track_data_v12(track, buffers)?;
            }
            Ok(Vec::new())
        }
        ssbh_lib::formats::anim::Anim::V20 { groups, buffer, .. } => {
            read_groups_v20(&groups.elements, &buffer.elements)
        }
        ssbh_lib::formats::anim::Anim::V21 { groups, buffer, .. } => {
            read_groups_v20(&groups.elements, &buffer.elements)
        }
    }
}

fn create_track_data_v12(
    track: &ssbh_lib::formats::anim::TrackV1,
    buffers: &ssbh_lib::SsbhArray<ssbh_lib::SsbhByteBuffer>,
) -> Result<TrackData, error::Error> {
    // TODO: Add tests for this to buffers.rs.
    for property in &track.properties.elements {
        let data = buffers.elements.get(property.buffer_index as usize).ok_or(
            error::Error::BufferIndexOutOfRange {
                buffer_index: property.buffer_index as usize,
                buffer_count: buffers.elements.len(),
            },
        )?;

        let mut reader = Cursor::new(&data.elements);
        let header: u32 = reader.read_le()?;

        println!("{:?},{:x?}", property.name.to_string_lossy(), header);

        match header {
            0x1003 => {
                println!("{:x?}", reader.read_le::<f32>()?);
            }
            0x2003 => {
                println!("{:?}", reader.read_le::<(f32, f32)>()?);
            }
            0x3003 => {
                println!("{:?}", reader.read_le::<Vector3>()?);
            }
            0x4003 => {
                println!("{:?}", reader.read_le::<Vector4>()?);
            }
            0x1013 => {
                println!("{:x?}", reader.read_le::<u16>()?);
            }
            x => println!("Unrecognized header: {:?}", x),
        }
    }
    println!();

    // TODO: Set the track data based on type?
    Ok(TrackData {
        name: track.name.to_string_lossy(),
        scale_options: ScaleOptions::default(),
        values: TrackValues::Float(Vec::new()),
        transform_flags: TransformFlags::default(),
    })
}

fn read_groups_v20(
    anim_groups: &[ssbh_lib::formats::anim::Group],
    anim_buffer: &[u8],
) -> Result<Vec<GroupData>, error::Error> {
    let mut groups = Vec::new();

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

fn create_track_data_v20(
    track: &ssbh_lib::formats::anim::TrackV2,
    buffer: &[u8],
) -> Result<TrackData, error::Error> {
    let start = track.data_offset as usize;
    let end =
        start
            .checked_add(track.data_size as usize)
            .ok_or(error::Error::InvalidTrackDataRange {
                start: track.data_offset as usize,
                size: track.data_size as usize,
                buffer_size: buffer.len(),
            })?;
    let buffer = buffer
        .get(start..end)
        .ok_or(error::Error::InvalidTrackDataRange {
            start: track.data_offset as usize,
            size: track.data_size as usize,
            buffer_size: buffer.len(),
        })?;

    let (values, inherit_scale, compensate_scale) =
        read_track_values(buffer, track.flags, track.frame_count as usize)?;

    // The compensate scale override is included in scale options instead.
    Ok(TrackData {
        name: track.name.to_string_lossy(),
        values,
        scale_options: ScaleOptions {
            compensate_scale: compensate_scale
                && !track.transform_flags.override_compensate_scale(),
        },
        transform_flags: track.transform_flags.into(),
    })
}

/// Data associated with a [Group].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct GroupData {
    /// The usage type for all the [NodeData] in [nodes](#structfield.nodes)
    pub group_type: GroupType,
    pub nodes: Vec<NodeData>,
}

/// Data associated with a [Node].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NodeData {
    pub name: String,
    pub tracks: Vec<TrackData>,
}

/// The data associated with a [TrackV2].
///
/// # Examples
/// The scale settings and transform flags should usually use their default value.
/**
```rust
use ssbh_data::anim_data::{TrackData, TrackValues, ScaleOptions, Transform, TransformFlags};

let track = TrackData {
    name: "Transform".to_string(),
    values: TrackValues::Transform(vec![Transform::IDENTITY]),
    scale_options: ScaleOptions::default(),
    transform_flags: TransformFlags::default()
};
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct TrackData {
    /// The name of the property to animate.
    ///
    /// For tracks in a group of type [GroupType::Material], this is the name of the material parameter like "CustomVector31".
    /// Other group types tend to use the name of the group type like "Transform" or "Visibility".
    pub name: String,

    pub scale_options: ScaleOptions,

    pub transform_flags: TransformFlags,

    /// The frame values for the property specified by [name](#structfield.name).
    ///
    /// Each element in the [TrackValues] provides the value for a single frame.
    /// If the [TrackValues] contains a single element, this track will be considered constant
    /// and repeat that element for each frame in the animation
    /// up to and including [final_frame_index](struct.AnimData.html#structfield.final_frame_index).
    pub values: TrackValues,
}

/// Determines how scaling is calculated for bone chains. Only applies to [TrackValues::Transform].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ScaleOptions {
    /// Revert the scaling of the immediate parent when `true`.
    ///
    /// The final scale relative to the parent is `current_scale * (1 / parent_scale)`.
    /// For Smash Ultimate, this is not applied recursively on the parent,
    /// so only the immediate parent's scaling is taken into account.
    /// This matches the behavior of scale compensation in Autodesk Maya.
    pub compensate_scale: bool,
}

impl Default for ScaleOptions {
    fn default() -> Self {
        // Uncompressed tracks don't allow disabling scale inheritance.
        // Defaulting to true avoids a potential error.
        Self {
            compensate_scale: false,
        }
    }
}

/// See [ssbh_lib::formats::anim::TransformFlags].
// Including compensate scale would be redundant with ScaleOptions.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct TransformFlags {
    pub override_translation: bool,
    pub override_rotation: bool,
    pub override_scale: bool,
}

impl From<TransformFlags> for AnimTransformFlags {
    fn from(f: TransformFlags) -> Self {
        Self::new()
            .with_override_translation(f.override_translation)
            .with_override_rotation(f.override_rotation)
            .with_override_scale(f.override_scale)
    }
}

impl From<AnimTransformFlags> for TransformFlags {
    fn from(f: AnimTransformFlags) -> Self {
        Self {
            override_translation: f.override_translation(),
            override_rotation: f.override_rotation(),
            override_scale: f.override_compensate_scale(),
        }
    }
}

// TODO: Investigate if the names based on the Anim 1.2 property names are accurate.
/// A decomposed 2D transformation for texture coordinates.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, SsbhWrite, Default, Clone, Copy)]
pub struct UvTransform {
    pub scale_u: f32,
    pub scale_v: f32,
    pub rotation: f32,
    pub translate_u: f32,
    pub translate_v: f32,
}

/// A decomposed 3D transformation consisting of a scale, rotation, and translation.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
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

    fn track_type(&self) -> TrackTypeV2 {
        match self {
            TrackValues::Transform(_) => TrackTypeV2::Transform,
            TrackValues::UvTransform(_) => TrackTypeV2::UvTransform,
            TrackValues::Float(_) => TrackTypeV2::Float,
            TrackValues::PatternIndex(_) => TrackTypeV2::PatternIndex,
            TrackValues::Boolean(_) => TrackTypeV2::Boolean,
            TrackValues::Vector4(_) => TrackTypeV2::Vector4,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_hex_eq;
    use hexlit::hex;

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
            anim,
            Anim::V20 {
                final_frame_index,
                ..
            } if final_frame_index == 1.5
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

        assert!(matches!(anim, Anim::V21 {
            final_frame_index, 
            ..
        } if final_frame_index == 2.5));
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
            Err(error::Error::InvalidFinalFrameIndex {
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
                        transform_flags: TransformFlags::default(),
                    }],
                }],
            }],
        });

        // A value of at least 3.0 is expected.
        assert!(matches!(
            result,
            Err(error::Error::InvalidFinalFrameIndex {
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

        assert!(matches!(anim, Anim::V21 {
            final_frame_index, 
            ..
        } if final_frame_index == 0.0));
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
            Err(error::Error::UnsupportedVersion {
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
                    transform_flags: TransformFlags::default(),
                },
                TrackData {
                    name: "t2".to_string(),
                    values: TrackValues::PatternIndex(vec![4, 5]),
                    scale_options: ScaleOptions::default(),
                    transform_flags: TransformFlags::default(),
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
                track_type: TrackTypeV2::Float,
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
                track_type: TrackTypeV2::PatternIndex,
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
    fn read_v20_track_invalid_offset() {
        let result = create_track_data_v20(
            &TrackV2 {
                name: "abc".into(),
                flags: TrackFlags {
                    track_type: TrackTypeV2::Transform,
                    compression_type: CompressionType::Compressed,
                },
                frame_count: 2,
                transform_flags: AnimTransformFlags::new(),
                data_offset: 5,
                data_size: 1,
            },
            &[0u8; 4],
        );

        assert!(matches!(
            result,
            Err(error::Error::InvalidTrackDataRange {
                start: 5,
                size: 1,
                buffer_size: 4
            })
        ));
    }

    #[test]
    fn read_v20_track_offset_overflow() {
        let result = create_track_data_v20(
            &TrackV2 {
                name: "abc".into(),
                flags: TrackFlags {
                    track_type: TrackTypeV2::Transform,
                    compression_type: CompressionType::Compressed,
                },
                frame_count: 2,
                transform_flags: AnimTransformFlags::new(),
                data_offset: u32::MAX,
                data_size: 1,
            },
            &[0u8; 4],
        );

        assert!(matches!(
            result,
            Err(error::Error::InvalidTrackDataRange {
                start: 4294967295,
                size: 1,
                buffer_size: 4
            })
        ));
    }

    #[test]
    fn read_v20_track_invalid_size() {
        let result = create_track_data_v20(
            &TrackV2 {
                name: "abc".into(),
                flags: TrackFlags {
                    track_type: TrackTypeV2::Transform,
                    compression_type: CompressionType::Compressed,
                },
                frame_count: 2,
                transform_flags: AnimTransformFlags::new(),
                data_offset: 0,
                data_size: 5,
            },
            &[0u8; 3],
        );

        assert!(matches!(
            result,
            Err(error::Error::InvalidTrackDataRange {
                start: 0,
                size: 5,
                buffer_size: 3
            })
        ));
    }
}
