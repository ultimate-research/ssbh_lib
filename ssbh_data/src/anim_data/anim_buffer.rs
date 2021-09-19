use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};
use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};
use bitvec::prelude::*;
use itertools::Itertools;
use modular_bitfield::prelude::*;
use std::{
    error::Error,
    fmt::Debug,
    io::{Cursor, Read, Seek, Write},
    num::NonZeroU64,
};

use ssbh_write::SsbhWrite;

use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags, TrackType},
    Ptr16, Ptr32, Vector3, Vector4,
};

use super::{ConstantTransform, TrackValues, Transform, UvTransform};

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

#[derive(Debug, BinRead, Clone, SsbhWrite, Default)]
struct U32Compression {
    pub min: u32,
    pub max: u32,
    pub bit_count: u64,
}

#[derive(Debug, BinRead, Clone, SsbhWrite, Default)]
struct F32Compression {
    pub min: f32,
    pub max: f32,
    pub bit_count: u64,
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct Vector3Compression {
    pub x: F32Compression,
    pub y: F32Compression,
    pub z: F32Compression,
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct Vector4Compression {
    pub x: F32Compression,
    pub y: F32Compression,
    pub z: F32Compression,
    pub w: F32Compression,
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct TransformCompression {
    // The first component of scale can also be compensate scale.
    pub scale: Vector3Compression,
    // The w component for rotation is handled separately.
    pub rotation: Vector3Compression,
    pub translation: Vector3Compression,
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct TextureDataCompression {
    pub unk1: F32Compression,
    pub unk2: F32Compression,
    pub unk3: F32Compression,
    pub unk4: F32Compression,
    pub unk5: F32Compression,
}

impl TrackValues {
    pub(crate) fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        compression: CompressionType,
    ) -> std::io::Result<()> {
        // TODO: float compression will be hard to test since bit counts may vary.
        // TODO: Test the binary representation for a fixed bit count (compression level)?

        // TODO: The defaults should use min or max if min == max.

        // TODO: More intelligently choose a bit count
        // For example, if min == max, bit count can be 0, which uses the default.
        let bit_count = 24;

        match compression {
            CompressionType::Compressed => match self {
                // TODO: Support writing compressed data for other track types.
                TrackValues::Transform(values) => {
                    write_compressed(
                        writer,
                        &values,
                        Transform {
                            scale: Vector3::new(1.0, 1.0, 1.0),
                            rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                            translation: Vector3::new(0.0, 0.0, 0.0),
                            compensate_scale: 0.0,
                        },
                        TransformCompression {
                            scale: Vector3Compression {
                                x: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                                y: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                                z: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                            },
                            rotation: Vector3Compression {
                                x: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                                y: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                                z: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                            },
                            translation: Vector3Compression {
                                x: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                                y: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                                z: F32Compression {
                                    min: 0.0,
                                    max: 0.0,
                                    bit_count,
                                },
                            },
                        },
                        compress_transforms,
                    )?;
                }
                TrackValues::UvTransform(_) => todo!(),
                TrackValues::Float(values) => {
                    write_compressed(
                        writer,
                        &values,
                        0.0, // TODO: f32 default for compression?
                        F32Compression {
                            min: find_min_f32(values),
                            max: find_max_f32(values),
                            bit_count,
                        },
                        compress_floats,
                    )?;
                }
                TrackValues::PatternIndex(_) => todo!(),
                TrackValues::Boolean(values) => {
                    write_compressed(
                        writer,
                        &values.iter().map(Boolean::from).collect_vec(),
                        Boolean(0u8),
                        0,
                        compress_booleans,
                    )?;
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
                    let values: Vec<Boolean> = values.iter().map(Boolean::from).collect();
                    values.write(writer)?;
                }
                TrackValues::Vector4(values) => values.write(writer)?,
            },
        }

        Ok(())
    }

    // HACK: Use default since SsbhWrite expects self for size in bytes.
    pub(crate) fn compressed_overhead_in_bytes(&self) -> u64 {
        match self {
            TrackValues::Transform(_) => {
                <Transform as CompressedData>::compressed_overhead_in_bytes()
            }
            TrackValues::UvTransform(_) => {
                <UvTransform as CompressedData>::compressed_overhead_in_bytes()
            }
            TrackValues::Float(_) => <f32 as CompressedData>::compressed_overhead_in_bytes(),
            TrackValues::PatternIndex(_) => <u32 as CompressedData>::compressed_overhead_in_bytes(),
            TrackValues::Boolean(_) => <Boolean as CompressedData>::compressed_overhead_in_bytes(),
            TrackValues::Vector4(_) => <Vector4 as CompressedData>::compressed_overhead_in_bytes(),
        }
    }

    pub(crate) fn data_size_in_bytes(&self) -> u64 {
        match self {
            TrackValues::Transform(_) => Transform::default().size_in_bytes(),
            TrackValues::UvTransform(_) => UvTransform::default().size_in_bytes(),
            TrackValues::Float(_) => f32::default().size_in_bytes(),
            TrackValues::PatternIndex(_) => u32::default().size_in_bytes(),
            TrackValues::Boolean(_) => Boolean::default().size_in_bytes(),
            TrackValues::Vector4(_) => Vector4::default().size_in_bytes(),
        }
    }
}

fn find_min_f32(values: &[f32]) -> f32 {
    // HACK: Just pretend like NaN doesn't exist.
    // TODO: Handle the case where values is empty.
    let mut min = *values.first().unwrap();
    for v in values {
        if *v < min {
            min = *v;
        }
    }

    min
}

fn find_max_f32(values: &[f32]) -> f32 {
    // HACK: Just pretend like NaN doesn't exist.
    let mut max = *values.first().unwrap();
    for v in values {
        if *v > max {
            max = *v;
        }
    }

    max
}

fn write_compressed<W: Write + Seek, T: CompressedData, F: Fn(&[T], &T::Compression) -> Vec<u8>>(
    writer: &mut W,
    values: &[T],
    default: T,
    compression: T::Compression,
    compress_t: F, // TODO: Can this be part of the compressed data trait?
) -> Result<(), std::io::Error> {
    let compressed_data = compress_t(values, &compression);

    let data = CompressedTrackData::<T> {
        header: CompressedHeader::<T> {
            unk_4: 4,
            // TODO: Does this also require passing in flags?
            flags: CompressionFlags::new(),
            default_data: Ptr16::new(default),
            // TODO: Pass in the flags?
            bits_per_entry: T::calculate_bit_count(&compression, &CompressionFlags::new()) as u16, // TODO: This might overflow.
            compressed_data: Ptr32::new(CompressedBuffer(compressed_data)),
            frame_count: values.len() as u32,
        },
        compression,
    };
    data.write(writer)?;
    Ok(())
}

// Shared logic for decompressing track data from a header and collection of bits.
trait CompressedData: BinRead<Args = ()> + SsbhWrite + Default {
    type Compression: BinRead<Args = ()> + SsbhWrite + Default;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self>;

    fn calculate_bit_count(compression: &Self::Compression, flags: &CompressionFlags) -> u64;

    // The size in bytes for the compressed header, default, and a single frame value.
    fn compressed_overhead_in_bytes() -> u64 {
        let header_size = 16;

        // TODO: If SsbhWrite::size_in_bytes didn't take self, we wouldn't need default here.
        // This may cause issues with the Option<T> type.
        header_size + Self::default().size_in_bytes() + Self::Compression::default().size_in_bytes()
    }
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

    fn calculate_bit_count(compression: &Self::Compression, flags: &CompressionFlags) -> u64 {
        // TODO: Different values can be turned on/off based on flags.
        1
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

    fn calculate_bit_count(compression: &Self::Compression, _flags: &CompressionFlags) -> u64 {
        compression.unk1.bit_count
            + compression.unk2.bit_count
            + compression.unk3.bit_count
            + compression.unk4.bit_count
            + compression.unk5.bit_count
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

    fn calculate_bit_count(compression: &Self::Compression, _flags: &CompressionFlags) -> u64 {
        compression.x.bit_count
            + compression.y.bit_count
            + compression.z.bit_count
            + compression.w.bit_count
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

    fn calculate_bit_count(compression: &Self::Compression, _flags: &CompressionFlags) -> u64 {
        compression.bit_count
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

    fn calculate_bit_count(compression: &Self::Compression, _flags: &CompressionFlags) -> u64 {
        compression.bit_count
    }
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
pub struct Boolean(u8);

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

    fn calculate_bit_count(_compression: &Self::Compression, _flags: &CompressionFlags) -> u64 {
        // Return the only bit count that makes sense.
        1
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

pub fn read_track_values(
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
                TrackValues::Boolean(values.iter().map(bool::from).collect())
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
                let values = read_direct(&mut reader, count)?;
                TrackValues::Boolean(values.iter().map(bool::from).collect_vec())
            }
            TrackType::Vector4 => TrackValues::Vector4(read_direct(&mut reader, count)?),
        },
    };

    Ok(values)
}

fn read_compressed<R: Read + Seek, T: CompressedData + std::fmt::Debug>(
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
            data.header.default_data.as_ref().unwrap(),
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
    _default: &u32,
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
        NonZeroU64::new(compression.bit_count as u64),
    ))
}

fn compress_booleans(values: &[Boolean], _: &<Boolean as CompressedData>::Compression) -> Vec<u8> {
    // Use 1 bit per bool.
    let mut elements = BitVec::<Lsb0, u8>::with_capacity(values.len());
    // TODO: The conversion to and from u8 seems really redundant.
    for value in values {
        elements.push(value.0 == 1);
    }
    elements.into_vec()
}

fn compress_transforms(
    values: &[Transform],
    compression: &<Transform as CompressedData>::Compression,
) -> Vec<u8> {
    // TODO: Make flags a parameter?
    let bit_count = Transform::calculate_bit_count(compression, &CompressionFlags::new());

    let mut elements = BitVec::<Lsb0, u8>::new();
    elements.resize(values.len() * bit_count as usize, false);

    for v in values {
        // TODO: Write the transforms.
        // TODO: How to gracefully handle some values being disabled?
    }

    elements.into_vec()
}

fn compress_floats(values: &[f32], compression: &<f32 as CompressedData>::Compression) -> Vec<u8> {
    // Allocate the appropriate number of bits.
    let mut elements = BitVec::<Lsb0, u8>::new();
    elements.resize(values.len() * compression.bit_count as usize, false);

    for (i, v) in values.iter().enumerate() {
        // For each window of bit count many bits, set the compressed value.
        // TODO: There's probably a better way of doing this.
        let start = i * compression.bit_count as usize;
        let end = start + compression.bit_count as usize;
        let compressed_value = compress_f32(
            *v,
            compression.min,
            compression.max,
            NonZeroU64::new(compression.bit_count as u64).unwrap(),
        );
        elements[start..end].store_le(compressed_value);
    }
    elements.into_vec()
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
            Some(1.254_003_3),
            decompress_f32(2350, 0.0, 8.74227, NonZeroU64::new(14))
        );
        assert_eq!(
            Some(1.185_819_5),
            decompress_f32(2654, 0.0, 7.32, NonZeroU64::new(14))
        );
        assert_eq!(
            Some(2.964_048_1),
            decompress_f32(2428, 0.0, 20.0, NonZeroU64::new(14))
        );
        assert_eq!(
            Some(1.218_784_5),
            decompress_f32(2284, 0.0, 8.74227, NonZeroU64::new(14))
        );
    }

    #[test]
    fn compress_float_14bit() {
        // stage/poke_unova/battle/motion/s13_a, D_lightning_B, CustomVector3
        assert_eq!(
            2350,
            compress_f32(1.254_003_3, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2654,
            compress_f32(1.185_819_5, 0.0, 7.32, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2428,
            compress_f32(2.964_048_1, 0.0, 20.0, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2284,
            compress_f32(1.218_784_5, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
    }

    #[test]
    fn read_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let data = hex_bytes("cdcccc3e 0000c03f 0000803f 0000803f");
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
            hex_bytes("cdcccc3e 0000c03f 0000803f 0000803f",)
        );
    }

    #[test]
    fn read_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let data = hex_bytes("0000803f 0000803f 00000000 00000000 00000000");
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
    fn read_compressed_uv_transform_multiple_frames() {
        // stage/kirby_greens/normal/motion/whispy_set/whispy_set_turnblowl3.nuanmb, _sfx_GrdGreensGrassAM1, nfTexture0[0]
        let data = hex_bytes(
            "04000900 60002600 74000000 14000000
             2a8e633e 34a13d3f 0a000000 00000000
             cdcc4c3e 7a8c623f 0a000000 00000000
             00000000 00000000 10000000 00000000
             ec51b8be bc7413bd 09000000 00000000
             a24536be e17a943e 09000000 00000000
             34a13d3f 7a8c623f 00000000 bc7413bd
             a24536be ffffff1f 80b4931a cfc12071
             8de500e6 535555",
        );
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
        let data = hex_bytes(
            "04000000 20000100 24000000 8a020000
             01000000 02000000 01000000 00000000
             01000000 fe",
        );
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
            "04000000 20000200 24000000 05000000
             00000000 00004040 02000000 00000000
             00000000 e403",
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
    fn write_compressed_floats_multiple_frame() {
        // Test that the min/max and bit counts are used properly
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Float(vec![0.5, 2.0]),
            &mut writer,
            CompressionType::Compressed,
        )
        .unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "04000000 20001800 24000000 02000000 0000003F 00000040 18000000 00000000 00000000 
                 000000 FFFFFF",
            )
        );
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
        let data = hex_bytes(
            "04000000 20000100 21000000 03000000
             00000000 00000000 00000000 00000000
             0006",
        );
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
            hex_bytes(
                "04000000 20000100 21000000 01000000 00000000 00000000 00000000 00000000 0001"
            )
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
        // The default data (00000000 00000000 3108ac3d bc74133e) 
        // uses the 0 bit count of one compression entry and the min/max of the next.
        // TODO: Is it worth adding code complexity to support this optimization?

        // fighter/cloud/motion/body/c00/b00guardon.nuanmb, EyeL, CustomVector31
        let data = hex_bytes(
            "04000000 50000300 60000000 08000000 
             0000803f 0000803f 00000000 00000000 
             0000803f 0000803f 00000000 00000000
             3108ac3d bc74133e 03000000 00000000
             00000000 00000000 00000000 00000000
             0000803f 0000803f 3108ac3d 00000000
             88c6fa",
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
            "0000803f 0000803f 0000803f 00000000
             00000000 00000000 0000803f bea4c13f
             79906ebe f641bebe 01000000",
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
                "0000803f 0000803f 0000803f 00000000
                 00000000 00000000 0000803f bea4c13f
                 79906ebe f641bebe 01000000",
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
                z: float_compression,
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
        let data = hex_bytes(
            "04000600 a0002b00 cc000000 02000000
             0000803f 0000803f 10000000 00000000
             0000803f 0000803f 10000000 00000000
             0000803f 0000803f 10000000 00000000
             00000000 b9bc433d 0d000000 00000000
             e27186bd 00000000 0d000000 00000000
             00000000 ada2273f 10000000 00000000
             16a41d40 16a41d40 10000000 00000000
             00000000 00000000 10000000 00000000
             00000000 00000000 10000000 00000000
             0000803f 0000803f 0000803f 00000000
             00000000 00000000 0000803f 16a41d40
             00000000 00000000 00000000 00e0ff03
             00f8ff00 e0ff1f",
        );
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
        let data = hex_bytes(
            "0000803f 0000803f 0000803f 1dca203e
             437216bf a002cbbd 5699493f 9790e5c1
             1f68a040 f7affa40 00000000 0000803f
             0000803f 0000803f c7d8093e 336b19bf
             5513e4bd e3fe473f 6da703c2 dfc3a840
             b8120b41 00000000",
        );
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
}
