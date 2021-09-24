use binread::{io::StreamPosition, BinRead};
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    io::{Cursor, Read, Seek, Write},
    path::Path,
};

use ssbh_write::SsbhWrite;

use ssbh_lib::formats::anim::{
    Anim, AnimGroup, AnimHeader, AnimHeaderV20, AnimNode, AnimTrackV2, AnimType, CompressionType,
    TrackFlags, TrackType,
};

use thiserror::Error;

pub use ssbh_lib::{Vector3, Vector4};

mod anim_buffer;
use anim_buffer::*;

// TODO: Add module level documentation to show anim <-> data conversions and describe overall structure and design.

/// The data associated with an [Anim] file.
/// Supported versions are 2.0 and 2.1.
#[derive(Debug)]
pub struct AnimData {
    pub major_version: u16,
    pub minor_version: u16,
    pub groups: Vec<GroupData>,
}

impl AnimData {
    /// Tries to read and convert the ANIM from `path`.
    /// The entire file is buffered for performance.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Anim::from_file(path)?.try_into()
    }

    /// Tries to read and convert the ANIM from `reader`.
    /// For best performance when opening from a file, use `from_file` instead.
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        Anim::read(reader)?.try_into()
    }

    /// Converts the data to ANIM and writes to the given `writer`.
    /// For best performance when writing to a file, use `write_to_file` instead.
    pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> Result<(), AnimError> {
        let anim = Anim::try_from(self)?;
        anim.write(writer)?;
        Ok(())
    }

    /// Converts the data to ANIM and writes to the given `path`.
    /// The entire file is buffered for performance.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AnimError> {
        let anim = Anim::try_from(self)?;
        anim.write_to_file(path)?;
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

// TODO: Test this for a small example?
fn create_anim(data: &AnimData) -> Result<Anim, AnimError> {
    match (data.major_version, data.minor_version) {
        (2, 0) | (2, 1) => (),
        _ => {
            return Err(AnimError::UnsupportedVersion {
                major_version: data.major_version,
                minor_version: data.minor_version,
            })
        }
    };

    let mut buffer = Cursor::new(Vec::new());

    let animations = data
        .groups
        .iter()
        .map(|g| create_anim_group(g, &mut buffer))
        .collect::<Result<Vec<_>, _>>()?;

    // TODO: How to handle 0 length animations?
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

    let header = AnimHeader::HeaderV20(AnimHeaderV20 {
        final_frame_index: max_frame_count as f32 - 1.0,
        unk1: 1,
        unk2: 3,
        name: "".into(), // TODO: this is usually based on file name?
        animations: animations.into(),
        buffer: buffer.into_inner().into(),
    });

    let anim = Anim {
        major_version: data.major_version,
        minor_version: data.minor_version,
        header,
    };
    Ok(anim)
}

fn create_anim_group(g: &GroupData, buffer: &mut Cursor<Vec<u8>>) -> std::io::Result<AnimGroup> {
    Ok(AnimGroup {
        anim_type: g.group_type.into(),
        nodes: g
            .nodes
            .iter()
            .map(|n| create_anim_node(n, buffer))
            .collect::<Result<Vec<_>, _>>()?
            .into(),
    })
}

fn create_anim_node(n: &NodeData, buffer: &mut Cursor<Vec<u8>>) -> std::io::Result<AnimNode> {
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
) -> std::io::Result<AnimTrackV2> {
    let compression_type = infer_optimal_compression_type(&t.values);

    // Anim tracks are written in the order they appear,
    // so just use the current position as the data offset.
    let pos_before = buffer.stream_pos()?;

    // Pointers for compressed data are relative to the start of the track's data.
    // This requires using a second writer to correctly calculate offsets.
    let mut track_data = Cursor::new(Vec::new());
    t.values.write(&mut track_data, compression_type)?;

    buffer.write_all(&track_data.into_inner())?;
    let pos_after = buffer.stream_pos()?;

    Ok(AnimTrackV2 {
        name: t.name.as_str().into(),
        flags: TrackFlags {
            track_type: t.values.track_type(),
            compression_type,
        },
        frame_count: t.values.len() as u32,
        unk3: 0, // TODO: unk3?
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
        // TODO: Rename the ANIM fields to be more consistent (animations -> groups)?
        ssbh_lib::formats::anim::AnimHeader::HeaderV20(header) => {
            read_anim_groups_v20(&header.animations.elements, &header.buffer.elements)
        }
        ssbh_lib::formats::anim::AnimHeader::HeaderV21(header) => {
            read_anim_groups_v20(&header.animations.elements, &header.buffer.elements)
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
            group_type: anim_group.anim_type.into(),
            nodes,
        };
        groups.push(group);
    }

    Ok(groups)
}

fn create_track_data_v20(
    anim_track: &ssbh_lib::formats::anim::AnimTrackV2,
    anim_buffer: &[u8],
) -> Result<TrackData, AnimError> {
    let start = anim_track.data_offset as usize;
    let end = start + anim_track.data_size as usize;
    let buffer = &anim_buffer[start..end];
    let values = read_track_values(buffer, anim_track.flags, anim_track.frame_count as usize)?;
    Ok(TrackData {
        name: anim_track.name.to_string_lossy(),
        values,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupType {
    Transform = 1,
    Visibility = 2,
    Material = 4,
    Camera = 5,
}

impl From<AnimType> for GroupType {
    fn from(t: AnimType) -> Self {
        match t {
            AnimType::Transform => GroupType::Transform,
            AnimType::Visibility => GroupType::Visibility,
            AnimType::Material => GroupType::Material,
            AnimType::Camera => GroupType::Camera,
        }
    }
}

impl From<GroupType> for AnimType {
    fn from(t: GroupType) -> Self {
        match t {
            GroupType::Transform => AnimType::Transform,
            GroupType::Visibility => AnimType::Visibility,
            GroupType::Material => AnimType::Material,
            GroupType::Camera => AnimType::Camera,
        }
    }
}

#[derive(Debug)]
pub struct GroupData {
    /// The usage type for all the [NodeData] in [nodes](#structfield.nodes)
    pub group_type: GroupType,
    pub nodes: Vec<NodeData>,
}

#[derive(Debug)]
pub struct NodeData {
    pub name: String,
    pub tracks: Vec<TrackData>,
}

#[derive(Debug)]
pub struct TrackData {
    pub name: String,
    pub values: TrackValues,
}

// TODO: This is probably scale_u, scale_v, unk, translate_u, translate_v (some tracks use transform names).
#[derive(Debug, BinRead, PartialEq, SsbhWrite, Default, Clone, Copy)]
pub struct UvTransform {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
}

/// A decomposed transformation consisting of a scale, rotation, and translation.
// TODO: Derive default and also add identity transforms?
#[derive(Debug, BinRead, PartialEq, SsbhWrite, Clone, Copy, Default)]
pub struct Transform {
    /// XYZ scale
    pub scale: Vector3,
    /// An XYZW unit quaternion where XYZ represent the axis component
    /// and w represents the angle component.
    pub rotation: Vector4,
    /// XYZ translation
    pub translation: Vector3,
    pub compensate_scale: f32,
}

#[derive(Debug, BinRead, PartialEq, SsbhWrite)]
struct ConstantTransform {
    pub scale: Vector3,
    pub rotation: Vector4,
    pub translation: Vector3,
    pub compensate_scale: u32,
}

impl From<&Transform> for ConstantTransform {
    fn from(value: &Transform) -> Self {
        Self {
            scale: value.scale,
            rotation: value.rotation,
            translation: value.translation,
            // TODO: Why does const transform use an integer type.
            // TODO: This cast may panic.
            compensate_scale: value.compensate_scale as u32,
        }
    }
}

impl From<ConstantTransform> for Transform {
    fn from(value: ConstantTransform) -> Self {
        Self {
            scale: value.scale,
            rotation: value.rotation,
            translation: value.translation,
            // TODO: Why does const transform use an integer type.
            // TODO: This cast may panic.
            compensate_scale: value.compensate_scale as f32,
        }
    }
}

/// A value collection with an element for each frame of the animation.
#[derive(Debug)]
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

    // TODO: Is it worth making this public?
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
    use super::*;

    #[test]
    fn create_empty_anim_v_2_0() {
        create_anim(&AnimData {
            major_version: 2,
            minor_version: 0,
            groups: Vec::new(),
        })
        .unwrap();
    }

    #[test]
    fn create_empty_anim_v_2_1() {
        create_anim(&AnimData {
            major_version: 2,
            minor_version: 1,
            groups: Vec::new(),
        })
        .unwrap();
    }

    #[test]
    fn create_empty_anim_invalid_version() {
        let result = create_anim(&AnimData {
            major_version: 1,
            minor_version: 2,
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
                },
                TrackData {
                    name: "t2".to_string(),
                    values: TrackValues::PatternIndex(vec![4, 5]),
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
}
