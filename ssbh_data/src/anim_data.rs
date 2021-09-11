use binread::{io::StreamPosition, BinRead, BinReaderExt, BinResult, ReadOptions};
use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};
use bitvec::prelude::*;
use modular_bitfield::prelude::*;
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    io::{Cursor, Read, Seek, Write},
    num::NonZeroU64,
    path::Path,
};

use ssbh_write::SsbhWrite;

use ssbh_lib::{
    formats::anim::{
        Anim, AnimGroup, AnimHeader, AnimHeaderV20, AnimNode, AnimTrackV2, AnimType,
        CompressionType, TrackFlags, TrackType,
    },
    Ptr16, Ptr32,
};

pub use ssbh_lib::{Vector3, Vector4};

// TODO: Add module level documentation to show anim <-> data conversions and describe overall structure and design.

/// The data associated with an [Anim] file.
/// The only supported version is 2.0.
#[derive(Debug)]
pub struct AnimData {
    // TODO: Support versions other than 2.x?
    pub major_version: u16,
    pub minor_version: u16,
    pub groups: Vec<GroupData>,
}

// TODO: Restrict the error type?
// TODO: Make this a trait?
impl AnimData {
    /// Tries to read and convert the ANIM from `path`.
    /// The entire file is buffered for performance.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let anim = Anim::from_file(path)?;
        (&anim).try_into()
    }

    /// Tries to read and convert the ANIM from `reader`.
    /// For best performance when opening from a file, use `from_file` instead.
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        let anim = Anim::read(reader)?;
        (&anim).try_into()
    }

    /// Converts the data to ANIM and writes to the given `writer`.
    /// For best performance when writing to a file, use `write_to_file` instead.
    pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> Result<(), Box<dyn Error>> {
        let anim = Anim::try_from(self)?;
        anim.write(writer)?;
        Ok(())
    }

    /// Converts the data to ANIM and writes to the given `path`.
    /// The entire file is buffered for performance.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let anim = Anim::try_from(self)?;
        anim.write_to_file(path)?;
        Ok(())
    }
}

// TODO: Test these conversions.
impl TryFrom<&Anim> for AnimData {
    type Error = Box<dyn Error>;

    fn try_from(anim: &Anim) -> Result<Self, Self::Error> {
        Ok(Self {
            major_version: anim.major_version,
            minor_version: anim.minor_version,
            groups: read_anim_groups(&anim)?,
        })
    }
}

impl TryFrom<&AnimData> for Anim {
    type Error = Box<dyn Error>;

    fn try_from(data: &AnimData) -> Result<Self, Self::Error> {
        create_anim(data)
    }
}

// TODO: Test this for a small example?
fn create_anim(data: &AnimData) -> Result<Anim, Box<dyn Error>> {
    // TODO: Check the version similar to mesh?
    let mut buffer = Cursor::new(Vec::new());

    let animations: Vec<_> = data
        .groups
        .iter()
        .map(|g| create_anim_group(g, &mut buffer))
        .collect();

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

fn create_anim_group(g: &GroupData, buffer: &mut Cursor<Vec<u8>>) -> AnimGroup {
    AnimGroup {
        anim_type: g.group_type.into(),
        nodes: g
            .nodes
            .iter()
            .map(|n| create_anim_node(n, buffer))
            .collect::<Vec<_>>()
            .into(),
    }
}

fn create_anim_node(n: &NodeData, buffer: &mut Cursor<Vec<u8>>) -> AnimNode {
    AnimNode {
        name: n.name.as_str().into(), // TODO: Make a convenience method for this?
        tracks: n
            .tracks
            .iter()
            .map(|t| create_anim_track_v2(buffer, t))
            .collect::<Vec<_>>()
            .into(),
    }
}

// TODO: Avoid unwrap()?
fn create_anim_track_v2(buffer: &mut Cursor<Vec<u8>>, t: &TrackData) -> AnimTrackV2 {
    // TODO: Create a function and test cases for compression type inference?
    let compression_type = if t.values.len() <= 1 {
        // The size of the compressed header adds too much overhead for single frame animations.
        // TODO: A smarter implementation would check if the direct data is larger than the equivalent compressed data.
        match t.values {
            TrackValues::Transform(_) => CompressionType::ConstTransform, // TODO: This is sometimes used for non transform types?
            _ => CompressionType::Constant,
        }
    } else {
        // Just compress booleans for now since they can easily be represented as single bits.
        match t.values {
            TrackValues::Boolean(_) => CompressionType::Compressed,
            _ => CompressionType::Direct,
        }
    };

    // Anim tracks are written in the order they appear,
    // so just use the current position as the data offset.
    let pos_before = buffer.stream_pos().unwrap();

    // Pointers for compressed data are relative to the start of the track's data.
    // This requires using a second writer to correctly calculate offsets.
    let mut track_data = Cursor::new(Vec::new());
    t.values.write(&mut track_data, compression_type).unwrap();

    buffer.write_all(&track_data.into_inner()).unwrap();
    let pos_after = buffer.stream_pos().unwrap();

    AnimTrackV2 {
        name: t.name.as_str().into(),
        flags: TrackFlags {
            track_type: t.values.track_type(),
            compression_type,
        },
        frame_count: t.values.len() as u32,
        unk3: 0, // TODO: unk3?
        data_offset: pos_before as u32,
        data_size: pos_after - pos_before,
    }
}

// TODO: Test conversions from anim?
fn read_anim_groups(anim: &Anim) -> Result<Vec<GroupData>, Box<dyn Error>> {
    // TODO: Return a more meaningful error type.
    match &anim.header {
        // TODO: Create fake groups for version 1.0?
        ssbh_lib::formats::anim::AnimHeader::HeaderV1(_) => panic!("Unsupported Version"),
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
) -> Result<Vec<GroupData>, Box<dyn Error>> {
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
) -> Result<TrackData, Box<dyn Error>> {
    let start = anim_track.data_offset as usize;
    let end = start + anim_track.data_size as usize;
    let buffer = &anim_buffer[start..end];
    let values = read_track_values(&buffer, anim_track.flags, anim_track.frame_count as usize)?;
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

// TODO: Put the ANIM buffer compression/decompression in a separate module?
#[derive(Debug, BinRead, SsbhWrite)]
struct CompressedTrackData<T: CompressedData> {
    pub header: CompressedHeader<T>,
    pub compression: T::Compression,
}

#[derive(Debug, BinRead, SsbhWrite)]
struct CompressedHeader<T: CompressedData> {
    pub unk_4: u16,              // TODO: Always 4?
    pub flags: CompressionFlags, // TODO: These are used for texture transforms as well?
    pub default_data: Ptr16<T>,
    pub bits_per_entry: u16,
    pub compressed_data: Ptr32<CompressedBuffer>,
    pub frame_count: u32,
}

// TODO: This could be a shared function/type in lib.rs.
fn read_to_end<R: Read + Seek>(reader: &mut R, _ro: &ReadOptions, _: ()) -> BinResult<Vec<u8>> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}

#[derive(Debug, BinRead, SsbhWrite)]
#[ssbhwrite(alignment = 1)] // TODO: Is 1 byte alignment correct?
struct CompressedBuffer(#[br(parse_with = read_to_end)] Vec<u8>);

#[derive(Debug, Clone, Copy, BitfieldSpecifier)]
#[bits = 2]
enum ScaleType {
    None = 0,
    Scale = 1,
    Unk2 = 2, // TODO: Unk2 is used a lot for scale type.
    CompensateScale = 3,
}

#[bitfield(bits = 16)]
#[derive(Debug, BinRead, Clone, Copy)]
#[br(map = Self::from_bytes)]
struct CompressionFlags {
    #[bits = 2]
    scale_type: ScaleType,
    has_rotation: bool,
    has_position: bool,
    #[skip]
    __: B12,
}

ssbh_write::ssbh_write_modular_bitfield_impl!(CompressionFlags, 2);

// TODO: This is probably scale_u, scale_v, unk, translate_u, translate_v (some tracks use transform names).
#[derive(Debug, BinRead, PartialEq, SsbhWrite)]
pub struct UvTransform {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
}

/// A decomposed transformation consisting of a scale, rotation, and translation.
#[derive(Debug, BinRead, PartialEq, SsbhWrite)]
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

#[derive(Debug, BinRead, Clone, SsbhWrite)]
struct U32Compression {
    pub min: u32,
    pub max: u32,
    pub bit_count: u64,
}

#[derive(Debug, BinRead, Clone, SsbhWrite)]
struct F32Compression {
    pub min: f32,
    pub max: f32,
    pub bit_count: u64,
}

#[derive(Debug, BinRead, SsbhWrite)]
struct Vector3Compression {
    pub x: F32Compression,
    pub y: F32Compression,
    pub z: F32Compression,
}

#[derive(Debug, BinRead, SsbhWrite)]
struct Vector4Compression {
    pub x: F32Compression,
    pub y: F32Compression,
    pub z: F32Compression,
    pub w: F32Compression,
}

#[derive(Debug, BinRead, SsbhWrite)]
struct TransformCompression {
    // TODO: The first component of scale can also be compensate scale?
    pub scale: Vector3Compression,
    // TODO: w for rotation is handled separately.
    pub rotation: Vector3Compression,
    pub translation: Vector3Compression,
}

#[derive(Debug, BinRead, SsbhWrite)]
struct TextureDataCompression {
    pub unk1: F32Compression,
    pub unk2: F32Compression,
    pub unk3: F32Compression,
    pub unk4: F32Compression,
    pub unk5: F32Compression,
    // TODO: no unk5?
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

    fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        compression: CompressionType,
    ) -> std::io::Result<()> {
        // TODO: float compression will be hard to test since bit counts may vary.
        // TODO: Test the binary representation for a fixed bit count (compression level)?
        match compression {
            CompressionType::Compressed => match self {
                // TODO: Support writing compressed data for other track types.
                TrackValues::Transform(_) => todo!(),
                TrackValues::UvTransform(_) => todo!(),
                TrackValues::Float(_) => todo!(),
                TrackValues::PatternIndex(_) => todo!(),
                TrackValues::Boolean(values) => {
                    // TODO: Create a write compressed function?
                    let mut elements = BitVec::<Lsb0, u8>::with_capacity(values.len());
                    for value in values {
                        elements.push(*value);
                    }

                    let data = CompressedTrackData::<Boolean> {
                        header: CompressedHeader::<Boolean> {
                            unk_4: 4,
                            flags: CompressionFlags::new(),
                            default_data: Ptr16::new(Boolean(0u8)),
                            bits_per_entry: 1,
                            compressed_data: Ptr32::new(CompressedBuffer(elements.into_vec())),
                            frame_count: values.len() as u32,
                        },
                        compression: 0,
                    };

                    data.write(writer)?;
                }
                TrackValues::Vector4(_) => todo!(),
            },
            _ => match self {
                // Use the same representation for all the non compressed data.
                // TODO: Is there any difference between these types?
                // TODO: Does it matter if a const or const transform has more than 1 frame?
                // TODO: Can const transform work with non transform data?
                TrackValues::Transform(values) => {
                    let new_values: Vec<ConstantTransform> =
                        values.iter().map(|t| t.into()).collect();
                    new_values.write(writer)?
                }
                TrackValues::UvTransform(values) => values.write(writer)?,
                TrackValues::Float(values) => values.write(writer)?,
                TrackValues::PatternIndex(values) => values.write(writer)?,
                TrackValues::Boolean(values) => {
                    let values: Vec<Boolean> = values.iter().map(|b| b.into()).collect();
                    values.write(writer)?;
                }
                TrackValues::Vector4(values) => values.write(writer)?,
            },
        }

        Ok(())
    }
}

// Shared logic for decompressing track data from a header and collection of bits.
trait CompressedData: BinRead<Args = ()> + SsbhWrite {
    type Compression: BinRead<Args = ()> + SsbhWrite;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self>;
}

impl CompressedData for Transform {
    type Compression = TransformCompression;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_transform_compressed(header, stream, compression, default)
    }
}

impl CompressedData for UvTransform {
    type Compression = TextureDataCompression;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_texture_data_compressed(header, stream, compression, default)
    }
}

impl CompressedData for Vector4 {
    type Compression = Vector4Compression;

    fn read_bits(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_vector4_compressed(stream, compression, default)
    }
}

// TODO: Create a newtype for PatternIndex(u32)?
impl CompressedData for u32 {
    type Compression = U32Compression;

    fn read_bits(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_pattern_index_compressed(stream, compression, default)
    }
}

impl CompressedData for f32 {
    type Compression = F32Compression;

    fn read_bits(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        Ok(read_compressed_f32(stream, compression)?.unwrap_or(*default))
    }
}

#[derive(Debug, BinRead, SsbhWrite)]
struct Boolean(u8);

impl From<bool> for Boolean {
    fn from(v: bool) -> Self {
        Self::from(&v)
    }
}

impl From<&bool> for Boolean {
    fn from(v: &bool) -> Self {
        if *v {
            Self(1u8)
        } else {
            Self(0u8)
        }
    }
}

impl From<&Boolean> for bool {
    fn from(v: &Boolean) -> Self {
        v.0 != 0u8
    }
}

impl From<Boolean> for bool {
    fn from(v: Boolean) -> Self {
        Self::from(&v)
    }
}

impl CompressedData for Boolean {
    // There are 16 bytes for determining the compression, but all bytes are set to 0.
    type Compression = u128;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        _compression: &Self::Compression,
        _default: &Self,
    ) -> bitbuffer::Result<Self> {
        // Boolean compression is based on bits per entry, which is usually set to 1 bit.
        // TODO: 0 bits uses the default?
        let value = stream.read_int::<u8>(header.bits_per_entry as usize)?;
        Ok(Boolean(value))
    }
}

fn read_direct<R: Read + Seek, T: BinRead>(
    reader: &mut R,
    frame_count: usize,
) -> BinResult<Vec<T>> {
    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value: T = reader.read_le()?;
        values.push(value);
    }
    Ok(values)
}

fn read_track_values(
    track_data: &[u8],
    flags: TrackFlags,
    count: usize,
) -> Result<TrackValues, Box<dyn Error>> {
    // TODO: Are Const, ConstTransform, and Direct all the same?
    // TODO: Can frame count be higher than 1 for Const and ConstTransform?
    let mut reader = Cursor::new(track_data);

    let values = match flags.compression_type {
        CompressionType::Compressed => match flags.track_type {
            TrackType::Transform => TrackValues::Transform(read_compressed(&mut reader, count)?),
            TrackType::UvTransform => {
                TrackValues::UvTransform(read_compressed(&mut reader, count)?)
            }
            TrackType::Float => TrackValues::Float(read_compressed(&mut reader, count)?),
            TrackType::PatternIndex => {
                TrackValues::PatternIndex(read_compressed(&mut reader, count)?)
            }
            TrackType::Boolean => {
                let values: Vec<Boolean> = read_compressed(&mut reader, count)?;
                TrackValues::Boolean(values.iter().map(|b| b.into()).collect())
            }
            TrackType::Vector4 => TrackValues::Vector4(read_compressed(&mut reader, count)?),
        },
        _ => match flags.track_type {
            TrackType::Transform => {
                let mut values = Vec::new();
                for _ in 0..count {
                    let value: ConstantTransform = reader.read_le()?;
                    values.push(value.into());
                }
                TrackValues::Transform(values)
            }
            TrackType::UvTransform => TrackValues::UvTransform(read_direct(&mut reader, count)?),
            TrackType::Float => TrackValues::Float(read_direct(&mut reader, count)?),
            TrackType::PatternIndex => TrackValues::PatternIndex(read_direct(&mut reader, count)?),
            TrackType::Boolean => {
                let mut values = Vec::new();
                for _ in 0..count {
                    // TODO: from<Boolean> for bool?
                    let value: Boolean = reader.read_le()?;
                    values.push(value.0 != 0);
                }
                TrackValues::Boolean(values)
            }
            TrackType::Vector4 => TrackValues::Vector4(read_direct(&mut reader, count)?),
        },
    };

    Ok(values)
}

fn read_compressed<R: Read + Seek, T: CompressedData>(
    reader: &mut R,
    frame_count: usize,
) -> Result<Vec<T>, Box<dyn Error>> {
    let data: CompressedTrackData<T> = reader.read_le()?;

    // TODO: Return an error if the header has null pointers.
    // Decompress values.
    let bit_buffer = BitReadBuffer::new(
        &data.header.compressed_data.as_ref().unwrap().0,
        bitbuffer::LittleEndian,
    );
    let mut bit_reader = BitReadStream::new(bit_buffer);

    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value = T::read_bits(
            &data.header,
            &mut bit_reader,
            &data.compression,
            &data.header.default_data.as_ref().unwrap(),
        )?;
        values.push(value);
    }

    Ok(values)
}

fn read_transform_compressed(
    header: &CompressedHeader<Transform>,
    bit_stream: &mut BitReadStream<LittleEndian>,
    compression: &TransformCompression,
    default: &Transform,
) -> bitbuffer::Result<Transform> {
    let (compensate_scale, scale) = match header.flags.scale_type() {
        ScaleType::Scale => (
            0.0,
            read_compressed_vector3(bit_stream, &compression.scale, &default.scale)?,
        ),
        ScaleType::CompensateScale => (
            read_compressed_f32(bit_stream, &compression.scale.x)?.unwrap_or(0.0),
            default.scale,
        ),
        // TODO: Unk2?
        _ => (0.0, default.scale),
    };

    let rotation = if header.flags.has_rotation() {
        // TODO: Add basic vector conversions and swizzling.
        // TODO: The w component is handled separately.
        let default_rotation_xyz =
            Vector3::new(default.rotation.x, default.rotation.y, default.rotation.z);

        let rotation_xyz =
            read_compressed_vector3(bit_stream, &compression.rotation, &default_rotation_xyz)?;
        Vector4::new(rotation_xyz.x, rotation_xyz.y, rotation_xyz.z, f32::NAN)
    } else {
        default.rotation
    };

    let translation = if header.flags.has_position() {
        read_compressed_vector3(bit_stream, &compression.translation, &default.translation)?
    } else {
        default.translation
    };

    let rotation_w = if header.flags.has_rotation() {
        calculate_rotation_w(bit_stream, rotation)
    } else {
        default.rotation.w
    };

    let rotation = Vector4::new(rotation.x, rotation.y, rotation.z, rotation_w);
    Ok(Transform {
        scale,
        rotation,
        translation,
        compensate_scale,
    })
}

fn calculate_rotation_w(bit_stream: &mut BitReadStream<LittleEndian>, rotation: Vector4) -> f32 {
    // Rotations are encoded as xyzw unit quaternions,
    // so x^2 + y^2 + z^2 + w^2 = 1.
    // Solving for the missing w gives two expressions:
    // w = sqrt(1 - x^2 + y^2 + z^2) or -sqrt(1 - x^2 + y^2 + z^2).
    // Thus, we need only need to store the sign bit to determine w.
    let flip_w = bit_stream.read_bool().unwrap();

    let w = f32::sqrt(
        1.0 - (rotation.x * rotation.x + rotation.y * rotation.y + rotation.z * rotation.z),
    );

    if flip_w {
        -w
    } else {
        w
    }
}

fn read_pattern_index_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &U32Compression,
    default: &u32,
) -> bitbuffer::Result<u32> {
    // TODO: There's only a single track in game that uses this, so this is just a guess.
    // TODO: How to compress a u32 with min, max, and bitcount?
    let value: u32 = bit_stream.read_int(compression.bit_count as usize)?;
    Ok(value + compression.min)
}

fn read_texture_data_compressed(
    header: &CompressedHeader<UvTransform>,
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &TextureDataCompression,
    default: &UvTransform,
) -> bitbuffer::Result<UvTransform> {
    // TODO: Is this correct?
    let (unk1, unk2) = match header.flags.scale_type() {
        ScaleType::Scale => (
            read_compressed_f32(bit_stream, &compression.unk1)?.unwrap_or(default.unk1),
            read_compressed_f32(bit_stream, &compression.unk2)?.unwrap_or(default.unk2),
        ),
        _ => (default.unk1, default.unk2),
    };

    // TODO: What toggles unk3?
    let unk3 = if header.flags.has_rotation() {
        read_compressed_f32(bit_stream, &compression.unk3)?.unwrap_or(default.unk3)
    } else {
        default.unk3
    };

    let unk4 = if header.flags.has_position() {
        read_compressed_f32(bit_stream, &compression.unk4)?.unwrap_or(default.unk4)
    } else {
        default.unk4
    };

    let unk5 = if header.flags.has_position() {
        read_compressed_f32(bit_stream, &compression.unk5)?.unwrap_or(default.unk5)
    } else {
        default.unk5
    };

    Ok(UvTransform {
        unk1,
        unk2,
        unk3,
        unk4,
        unk5,
    })
}

fn read_vector4_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &Vector4Compression,
    default: &Vector4,
) -> bitbuffer::Result<Vector4> {
    let x = read_compressed_f32(bit_stream, &compression.x)?.unwrap_or(default.x);
    let y = read_compressed_f32(bit_stream, &compression.y)?.unwrap_or(default.y);
    let z = read_compressed_f32(bit_stream, &compression.z)?.unwrap_or(default.z);
    let w = read_compressed_f32(bit_stream, &compression.w)?.unwrap_or(default.w);
    Ok(Vector4::new(x, y, z, w))
}

fn read_compressed_vector3(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &Vector3Compression,
    default: &Vector3,
) -> bitbuffer::Result<Vector3> {
    let x = read_compressed_f32(bit_stream, &compression.x)?.unwrap_or(default.x);
    let y = read_compressed_f32(bit_stream, &compression.y)?.unwrap_or(default.y);
    let z = read_compressed_f32(bit_stream, &compression.z)?.unwrap_or(default.z);
    Ok(Vector3::new(x, y, z))
}

fn read_compressed_f32(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &F32Compression,
) -> bitbuffer::Result<Option<f32>> {
    let value: u32 = bit_stream.read_int(compression.bit_count as usize)?;

    Ok(decompress_f32(
        value,
        compression.min,
        compression.max,
        NonZeroU64::new(compression.bit_count),
    ))
}

fn bit_mask(bit_count: NonZeroU64) -> u64 {
    // Get a mask of bit_count many bits set to 1.
    // Don't allow zero to avoid overflow.
    (1u64 << bit_count.get()) - 1u64
}

fn compress_f32(value: f32, min: f32, max: f32, bit_count: NonZeroU64) -> u32 {
    // The inverse operation of decompression.
    let scale = bit_mask(bit_count);

    // TODO: Divide by 0.0?
    // There could be large errors due to cancellations when the absolute difference of max and min is small.
    // This is likely rare in practice.
    let ratio = (value - min) / (max - min);
    let compressed = ratio * scale as f32;
    compressed as u32
}

// TODO: Is it safe to assume u32?
// TODO: It should be possible to test the edge cases by debugging Smash running in an emulator.
// Ex: Create a vector4 animation with all frames set to the same compressed value and inspect the uniform buffer.
fn decompress_f32(value: u32, min: f32, max: f32, bit_count: Option<NonZeroU64>) -> Option<f32> {
    // Anim supports custom ranges and non standard bit counts for fine tuning compression.
    // Unsigned normalized u8 would use min: 0.0, max: 1.0, and bit_count: 8.
    // This produces 2 ^ 8 evenly spaced floating point values between 0.0 and 1.0,
    // so 0b00000000 corresponds to 0.0 and 0b11111111 corresponds to 1.0.

    // Use an option to prevent division by zero when bit count is zero.
    let scale = bit_mask(bit_count?);

    // TODO: There may be some edge cases with this implementation of linear interpolation.
    // TODO: What happens when value > scale?
    let lerp = |a, b, t| a * (1.0 - t) + b * t;
    let value = lerp(min, max, value as f32 / scale as f32);
    Some(value)
}

#[cfg(test)]
mod tests {
    use crate::hex_bytes;

    use super::*;

    #[test]
    fn bit_masks() {
        assert_eq!(0b1u64, bit_mask(NonZeroU64::new(1).unwrap()));
        assert_eq!(0b11u64, bit_mask(NonZeroU64::new(2).unwrap()));
        assert_eq!(0b111111111u64, bit_mask(NonZeroU64::new(9).unwrap()));
    }

    #[test]
    fn decompress_float_0bit() {
        // fighter/cloud/motion/body/c00/b00guardon.nuanmb, EyeL, CustomVector31
        assert_eq!(None, decompress_f32(0, 1.0, 1.0, None));
        assert_eq!(None, decompress_f32(0, 0.0, 0.0, None));
    }

    #[test]
    fn compress_float_8bit() {
        let bit_count = NonZeroU64::new(8).unwrap();
        for i in 0..=255u8 {
            assert_eq!(
                i as u32,
                compress_f32(i as f32 / u8::MAX as f32, 0.0, 1.0, bit_count)
            );
        }
    }

    #[test]
    fn decompress_float_8bit() {
        let bit_count = NonZeroU64::new(8);
        for i in 0..=255u8 {
            assert_eq!(
                Some(i as f32 / u8::MAX as f32),
                decompress_f32(i as u32, 0.0, 1.0, bit_count)
            );
        }
    }

    #[test]
    fn decompress_float_14bit() {
        // stage/poke_unova/battle/motion/s13_a, D_lightning_B, CustomVector3
        assert_eq!(
            Some(1.25400329),
            decompress_f32(2350, 0.0, 8.74227, NonZeroU64::new(14))
        );
        assert_eq!(
            Some(1.18581951),
            decompress_f32(2654, 0.0, 7.32, NonZeroU64::new(14))
        );
        assert_eq!(
            Some(2.96404815),
            decompress_f32(2428, 0.0, 20.0, NonZeroU64::new(14))
        );
        assert_eq!(
            Some(1.21878445),
            decompress_f32(2284, 0.0, 8.74227, NonZeroU64::new(14))
        );
    }

    #[test]
    fn compress_float_14bit() {
        // stage/poke_unova/battle/motion/s13_a, D_lightning_B, CustomVector3
        assert_eq!(
            2350,
            compress_f32(1.25400329, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2654,
            compress_f32(1.18581951, 0.0, 7.32, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2428,
            compress_f32(2.96404815, 0.0, 20.0, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2284,
            compress_f32(1.21878445, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
    }

    #[test]
    fn read_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let data = hex_bytes("cdcccc3e0000c03f0000803f0000803f");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::Vector4(values) => {
                assert_eq!(vec![Vector4::new(0.4, 1.5, 1.0, 1.0)], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Vector4(vec![Vector4::new(0.4, 1.5, 1.0, 1.0)]),
            &mut writer,
            CompressionType::Constant,
        )
        .unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("cdcccc3e0000c03f0000803f0000803f",)
        );
    }

    #[test]
    fn read_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let data = hex_bytes("0000803f0000803f000000000000000000000000");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::UvTransform(values) => {
                assert_eq!(
                    vec![UvTransform {
                        unk1: 1.0,
                        unk2: 1.0,
                        unk3: 0.0,
                        unk4: 0.0,
                        unk5: 0.0
                    }],
                    values
                );
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::UvTransform(vec![UvTransform {
                unk1: 1.0,
                unk2: 1.0,
                unk3: 0.0,
                unk4: 0.0,
                unk5: 0.0,
            }]),
            &mut writer,
            CompressionType::Constant,
        )
        .unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("0000803f0000803f000000000000000000000000",)
        );
    }

    #[test]
    fn read_compressed_texture_multiple_frames() {
        // stage/kirby_greens/normal/motion/whispy_set/whispy_set_turnblowl3.nuanmb, _sfx_GrdGreensGrassAM1, nfTexture0[0]
        let data = hex_bytes("040009006000260074000000140000002a8e633e34a13d3f0a00000000000000cdcc4c3e7a8c623f0a000000000000000000
            0000000000001000000000000000ec51b8bebc7413bd0900000000000000a24536bee17a943e09000000000
            0000034a13d3f7a8c623f00000000bc7413bda24536be
            ffffff1f80b4931acfc120718de500e6535555");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Compressed,
            },
            4,
        )
        .unwrap();

        // TODO: This is just a guess based on the flags.
        match values {
            TrackValues::UvTransform(values) => {
                assert_eq!(
                    vec![
                        UvTransform {
                            unk1: 0.740741,
                            unk2: 0.884956,
                            unk3: 0.0,
                            unk4: -0.036,
                            unk5: -0.178
                        },
                        UvTransform {
                            unk1: 0.5881758,
                            unk2: 0.6412375,
                            unk3: 0.0,
                            unk4: -0.0721409,
                            unk5: -0.12579648
                        },
                        UvTransform {
                            unk1: 0.4878173,
                            unk2: 0.5026394,
                            unk3: 0.0,
                            unk4: -0.1082818,
                            unk5: -0.07359296
                        },
                        UvTransform {
                            unk1: 0.4168567,
                            unk2: 0.41291887,
                            unk3: 0.0,
                            unk4: -0.14378865,
                            unk5: -0.02230528
                        }
                    ],
                    values
                );
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_constant_pattern_index_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture0[0].PatternIndex
        let data = hex_bytes("01000000");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::PatternIndex(values) => {
                assert_eq!(vec![1], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_pattern_index_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture0[0].PatternIndex
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::PatternIndex(vec![1]),
            &mut writer,
            CompressionType::Constant,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("01000000",));
    }

    #[test]
    fn read_compressed_pattern_index_multiple_frames() {
        // stage/fzero_mutecity3ds/normal/motion/s05_course/s05_course__l00b.nuanmb, phong32__S_CUS_0xa3c00501___NORMEXP16_, DiffuseUVTransform.PatternIndex.
        // Shortened from 650 to 8 frames.
        let data =
            hex_bytes("0400000020000100240000008a0200000100000002000000010000000000000001000000fe");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();

        // TODO: This is just a guess for min: 1, max: 2, bit_count: 1.
        match values {
            TrackValues::PatternIndex(values) => {
                assert_eq!(vec![1, 2, 2, 2, 2, 2, 2, 2], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let data = hex_bytes("cdcccc3e");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::Float(values) => {
                assert_eq!(vec![0.4], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Float(vec![0.4]),
            &mut writer,
            CompressionType::Constant,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("cdcccc3e",));
    }

    #[test]
    fn read_compressed_float_multiple_frames() {
        // pacman/model/body/c00/model.nuanmb, phong3__phong0__S_CUS_0xa2001001___7__AT_GREATER128___VTC__NORMEXP16___CULLNONE_A_AB_SORT, CustomFloat2
        let data = hex_bytes(
            "040000002000020024000000050000000000000000004040020000000000000000000000e403",
        );
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Compressed,
            },
            5,
        )
        .unwrap();

        match values {
            TrackValues::Float(values) => {
                assert_eq!(vec![0.0, 1.0, 2.0, 3.0, 3.0], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_constant_boolean_single_frame_true() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean1
        let data = hex_bytes("01");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::Boolean(values) => {
                assert_eq!(vec![true], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_boolean_single_frame_true() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean1
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![true]),
            &mut writer,
            CompressionType::Constant,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("01"));
    }

    #[test]
    fn read_constant_boolean_single_frame_false() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean11
        let data = hex_bytes("00");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::Boolean(values) => {
                assert_eq!(vec![false], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_compressed_boolean_multiple_frames() {
        // assist/ashley/motion/body/c00/vis.nuanmb, magic, Visibility
        let data =
            hex_bytes("04000000200001002100000003000000000000000000000000000000000000000006");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Compressed,
            },
            3,
        )
        .unwrap();

        match values {
            TrackValues::Boolean(values) => {
                assert_eq!(vec![false, true, true], values)
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_compressed_boolean_single_frame() {
        // Test writing a single bit.
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![true]),
            &mut writer,
            CompressionType::Compressed,
        )
        .unwrap();
        assert_eq!(
            *writer.get_ref(),
            hex_bytes("04000000200001002100000001000000000000000000000000000000000000000001")
        );
    }

    #[test]
    fn write_compressed_boolean_three_frames() {
        // assist/ashley/motion/body/c00/vis.nuanmb, magic, Visibility
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![false, true, true]),
            &mut writer,
            CompressionType::Compressed,
        )
        .unwrap();
        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "04000000 20000100 21000000 03000000 00000000 00000000 00000000 00000000 0006"
            )
        );
    }

    #[test]
    fn write_compressed_boolean_multiple_frames() {
        // fighter/mario/motion/body/c00/a00wait3.nuanmb, MarioFaceN, Visibility
        // Shortened from 96 to 11 frames.
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![true; 11]),
            &mut writer,
            CompressionType::Compressed,
        )
        .unwrap();
        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "04000000 20000100 21000000 0B000000 00000000 00000000 00000000 00000000 00FF07"
            )
        );
    }

    #[test]
    fn read_compressed_vector4_multiple_frames() {
        // fighter/cloud/motion/body/c00/b00guardon.nuanmb, EyeL, CustomVector31
        let data = hex_bytes(
            "040000005000030060000000080000000000803f0000803f000000000000000000
            00803f0000803f00000000000000003108ac3dbc74133e03000000000000000000000000000000000000
            00000000000000803f0000803f3108ac3d0000000088c6fa",
        );
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();

        match values {
            TrackValues::Vector4(values) => {
                assert_eq!(
                    vec![
                        Vector4::new(1.0, 1.0, 0.084, 0.0),
                        Vector4::new(1.0, 1.0, 0.09257142, 0.0),
                        Vector4::new(1.0, 1.0, 0.10114286, 0.0),
                        Vector4::new(1.0, 1.0, 0.109714285, 0.0),
                        Vector4::new(1.0, 1.0, 0.118285716, 0.0),
                        Vector4::new(1.0, 1.0, 0.12685715, 0.0),
                        Vector4::new(1.0, 1.0, 0.13542856, 0.0),
                        Vector4::new(1.0, 1.0, 0.144, 0.0)
                    ],
                    values
                )
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_constant_transform_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, FingerL11, Transform
        let data = hex_bytes(
            "0000803f0000803f0000803f000000000000000
            0000000000000803fbea4c13f79906ebef641bebe01000000",
        );
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::ConstTransform,
            },
            1,
        )
        .unwrap();

        match values {
            TrackValues::Transform(values) => {
                assert_eq!(
                    vec![Transform {
                        translation: Vector3::new(1.51284, -0.232973, -0.371597),
                        rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                        scale: Vector3::new(1.0, 1.0, 1.0),
                        compensate_scale: 1.0
                    }],
                    values
                )
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_transform_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, FingerL11, Transform
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Transform(vec![Transform {
                translation: Vector3::new(1.51284, -0.232973, -0.371597),
                rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                scale: Vector3::new(1.0, 1.0, 1.0),
                compensate_scale: 1.0,
            }]),
            &mut writer,
            CompressionType::Constant,
        )
        .unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "0000803f0000803f0000803f000000000000000
            0000000000000803fbea4c13f79906ebef641bebe01000000",
            )
        );
    }

    fn read_compressed_transform_with_flags(flags: CompressionFlags, data_hex: &str) {
        let default = Transform {
            scale: Vector3::new(1.0, 1.0, 1.0),
            rotation: Vector4::new(2.0, 2.0, 2.0, 2.0),
            translation: Vector3::new(3.0, 3.0, 3.0),
            compensate_scale: 4.0,
        };

        let header = CompressedHeader::<Transform> {
            unk_4: 4,
            flags,
            default_data: Ptr16::new(default),
            // TODO: Bits per entry shouldn't matter?
            bits_per_entry: 16,
            compressed_data: Ptr32::new(CompressedBuffer(hex_bytes(data_hex))),
            frame_count: 1,
        };
        let float_compression = F32Compression {
            min: 0.0,
            max: 1.0,
            bit_count: 8,
        };
        let compression = TransformCompression {
            scale: Vector3Compression {
                x: float_compression.clone(),
                y: float_compression.clone(),
                z: float_compression.clone(),
            },
            rotation: Vector3Compression {
                x: float_compression.clone(),
                y: float_compression.clone(),
                z: float_compression.clone(),
            },
            translation: Vector3Compression {
                x: float_compression.clone(),
                y: float_compression.clone(),
                z: float_compression.clone(),
            },
        };
        let data = hex_bytes(data_hex);
        let bit_buffer = BitReadBuffer::new(&data, bitbuffer::LittleEndian);
        let mut bit_reader = BitReadStream::new(bit_buffer);

        let default = Transform {
            scale: Vector3::new(1.0, 1.0, 1.0),
            rotation: Vector4::new(2.0, 2.0, 2.0, 2.0),
            translation: Vector3::new(3.0, 3.0, 3.0),
            compensate_scale: 4.0,
        };
        read_transform_compressed(&header, &mut bit_reader, &compression, &default).unwrap();
    }

    #[test]
    fn read_compressed_transform_flags() {
        read_compressed_transform_with_flags(CompressionFlags::new(), "");
        read_compressed_transform_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::CompensateScale),
            "FF",
        );
        read_compressed_transform_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::Scale),
            "FFFFFF",
        );

        read_compressed_transform_with_flags(
            CompressionFlags::new()
                .with_scale_type(ScaleType::Scale)
                .with_has_rotation(true)
                .with_has_position(true),
            "FFFFFF FFFFFF FFFFFF 01",
        );

        // It's possible to have scale or compensate scale but not both.
        read_compressed_transform_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::None),
            "",
        );

        read_compressed_transform_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::Scale),
            "FFFFFF",
        );

        read_compressed_transform_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::CompensateScale),
            "FF",
        );
    }

    #[test]
    fn read_compressed_transform_multiple_frames() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        let data = hex_bytes("04000600a0002b00cc000000020000000000803f0000803f1000000
            0000000000000803f0000803f10000000000000000000803f0000803f100000000000000000000
            000b9bc433d0d00000000000000e27186bd000000000d0000000000000000000000ada2273f100000000000
            000016a41d4016a41d401000000000000000000000000000000010000000000000000000000000000000100000000
            00000000000803f0000803f0000803f0000000000000000000000000000803f16a41d400000000000000000000000000
            0e0ff0300f8ff00e0ff1f");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::Compressed,
            },
            2,
        )
        .unwrap();

        match values {
            TrackValues::Transform(values) => {
                assert_eq!(
                    vec![
                        Transform {
                            translation: Vector3::new(2.46314, 0.0, 0.0),
                            rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                            scale: Vector3::new(1.0, 1.0, 1.0),
                            compensate_scale: 0.0
                        },
                        Transform {
                            translation: Vector3::new(2.46314, 0.0, 0.0),
                            rotation: Vector4::new(0.0477874, -0.0656469, 0.654826, 0.7514052),
                            scale: Vector3::new(1.0, 1.0, 1.0),
                            compensate_scale: 0.0
                        }
                    ],
                    values
                )
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_direct_transform_multiple_frames() {
        // camera/fighter/ike/c00/d02finalstart.nuanmb, gya_camera, Transform
        // Shortened from 8 to 2 frames.
        let data = hex_bytes("0000803f0000803f0000803f1dca203e437216bfa002cbbd5699493f9790e5c11f68a040f7affa40000000000000803f0000803f0000803fc7d8
            093e336b19bf5513e4bde3fe473f6da703c2dfc3a840b8120b4100000000");
        let values = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::Direct,
            },
            2,
        )
        .unwrap();

        match values {
            TrackValues::Transform(values) => {
                assert_eq!(
                    vec![
                        Transform {
                            translation: Vector3::new(-28.6956, 5.01271, 7.83398),
                            rotation: Vector4::new(0.157021, -0.587681, -0.0991261, 0.787496),
                            scale: Vector3::new(1.0, 1.0, 1.0),
                            compensate_scale: 0.0
                        },
                        Transform {
                            translation: Vector3::new(-32.9135, 5.27391, 8.69207),
                            rotation: Vector4::new(0.134616, -0.599292, -0.111365, 0.781233),
                            scale: Vector3::new(1.0, 1.0, 1.0),
                            compensate_scale: 0.0
                        },
                    ],
                    values
                )
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn create_node_no_tracks() {
        let node = NodeData {
            name: "empty".to_string(),
            tracks: Vec::new(),
        };

        let mut buffer = Cursor::new(Vec::new());

        let anim_node = create_anim_node(&node, &mut buffer);
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

        let anim_node = create_anim_node(&node, &mut buffer);
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
}
