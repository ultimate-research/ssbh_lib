use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};
use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};
use bitvec::prelude::*;
use modular_bitfield::prelude::*;
use std::{
    io::{Cursor, Read, Seek, Write},
    num::NonZeroU64,
};

use ssbh_write::SsbhWrite;

use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags, TrackType},
    Ptr16, Ptr32, Vector3, Vector4,
};

#[derive(Debug, BinRead, SsbhWrite)]
struct CompressedTrackData<T: CompressedData> {
    pub header: CompressedHeader<T>,
    pub compression: T::Compression,
}

// TODO: It should be possible to derive SsbhWrite.
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

#[bitfield(bits = 16)]
#[derive(Debug, BinRead, Clone, Copy)]
#[br(map = Self::from_bytes)]
struct CompressionFlags {
    has_scale: bool,            // TODO: Is this right?
    has_compensate_scale: bool, // 0b11 is apparently compensate scale, so is this a two bit enum?
    has_rotation: bool,
    has_position: bool,
    #[skip]
    __: B12,
}

impl SsbhWrite for CompressionFlags {
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        writer.write_all(&self.into_bytes())?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // TODO: Get size at compile time?
        self.into_bytes().len() as u64
    }
}

// TODO: This is probably scale_u, scale_v, unk, translate_u, translate_v (some tracks use transform names).
#[derive(Debug, BinRead, PartialEq, SsbhWrite)]
pub struct UvTransform {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
}

#[derive(Debug, BinRead, PartialEq, SsbhWrite)]
pub struct Transform {
    pub scale: Vector3,
    pub rotation: Vector4, // TODO: Special type for quaternion?
    pub translation: Vector3,
    pub compensate_scale: f32,
}

#[derive(Debug, BinRead, PartialEq, SsbhWrite)]
struct ConstantTransform {
    pub scale: Vector3,
    pub rotation: Vector4, // TODO: Special type for quaternion?
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

#[derive(Debug)]
pub enum TrackData {
    Transform(Vec<Transform>),
    UvTransform(Vec<UvTransform>),
    Float(Vec<f32>),
    PatternIndex(Vec<u32>),
    Boolean(Vec<bool>),
    Vector4(Vec<Vector4>),
}

// Shared logic for decompressing track data from a header and collection of bits.
trait CompressedData: BinRead<Args = ()> + SsbhWrite {
    type Compression: BinRead<Args = ()> + SsbhWrite;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> Self;
}

impl CompressedData for Transform {
    type Compression = TransformCompression;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> Self {
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
    ) -> Self {
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
    ) -> Self {
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
    ) -> Self {
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
    ) -> Self {
        read_compressed_f32(stream, compression).unwrap_or(*default)
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
        default: &Self,
    ) -> Self {
        // Boolean compression is based on bits per entry, which is usually set to 1 bit.
        // TODO: 0 bits uses the default?
        let value = stream
            .read_int::<u8>(header.bits_per_entry as usize)
            .unwrap();
        Boolean(value)
    }
}

fn read_direct<R: Read + Seek, T: BinRead>(reader: &mut R, frame_count: usize) -> Vec<T> {
    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value: T = reader.read_le().unwrap();
        values.push(value);
    }
    values
}

// TODO: Frame count for const transform?
// TODO: Avoid unwrap and handle errors.
fn read_track_data(track_data: &[u8], flags: TrackFlags, count: usize) -> TrackData {
    // TODO: Are Const, ConstTransform, and Direct all the same?
    // TODO: Can frame count be higher than 1 for the above compression types?
    let mut reader = Cursor::new(track_data);

    match flags.compression_type {
        CompressionType::Compressed => match flags.track_type {
            TrackType::Transform => TrackData::Transform(read_compressed(&mut reader, count)),
            TrackType::UvTransform => TrackData::UvTransform(read_compressed(&mut reader, count)),
            TrackType::Float => TrackData::Float(read_compressed(&mut reader, count)),
            TrackType::PatternIndex => TrackData::PatternIndex(read_compressed(&mut reader, count)),
            TrackType::Boolean => {
                let values: Vec<Boolean> = read_compressed(&mut reader, count);
                TrackData::Boolean(values.iter().map(|b| b.into()).collect())
            }
            TrackType::Vector4 => TrackData::Vector4(read_compressed(&mut reader, count)),
        },
        _ => match flags.track_type {
            TrackType::Transform => {
                let mut values = Vec::new();
                for _ in 0..count {
                    let value: ConstantTransform = reader.read_le().unwrap();
                    values.push(value.into());
                }
                TrackData::Transform(values)
            }
            TrackType::UvTransform => TrackData::UvTransform(read_direct(&mut reader, count)),
            TrackType::Float => TrackData::Float(read_direct(&mut reader, count)),
            TrackType::PatternIndex => TrackData::PatternIndex(read_direct(&mut reader, count)),
            TrackType::Boolean => {
                let mut values = Vec::new();
                for _ in 0..count {
                    // TODO: from<Boolean> for bool?
                    let value: Boolean = reader.read_le().unwrap();
                    values.push(value.0 != 0);
                }
                TrackData::Boolean(values)
            }
            TrackType::Vector4 => TrackData::Vector4(read_direct(&mut reader, count)),
        },
    }
}

fn write_track_data<W: Write + Seek>(
    writer: &mut W,
    track_data: &TrackData,
    compression: CompressionType,
) {
    // TODO: float compression will be hard to test since bit counts may vary.
    // TODO: Test the binary representation for a fixed bit count (compression level)?
    match compression {
        CompressionType::Compressed => match track_data {
            TrackData::Transform(_) => todo!(),
            TrackData::UvTransform(_) => todo!(),
            TrackData::Float(_) => todo!(),
            TrackData::PatternIndex(_) => todo!(),
            TrackData::Boolean(values) => {
                // TODO: Create a write compressed function?
                let mut elements = BitVec::<Lsb0, u8>::with_capacity(values.len());
                for value in values {
                    elements.push(*value);
                }

                // TODO: Is there a nicer way to align to u8 without manual padding?
                for _ in 0..5 {
                    elements.push(false);
                }

                // TODO: How to get the bits aligned to u8 and in the right order?
                let buffer_bytes = match elements.domain() {
                    bitvec::domain::Domain::Region {
                        head: _,
                        body,
                        tail: _,
                    } => body,
                    _ => panic!("TODO"),
                };

                let data = CompressedTrackData::<Boolean> {
                    header: CompressedHeader::<Boolean> {
                        unk_4: 4,
                        flags: CompressionFlags::new(),
                        default_data: Ptr16::new(Boolean(0u8)),
                        bits_per_entry: 1,
                        compressed_data: Ptr32::new(CompressedBuffer(buffer_bytes.to_vec())),
                        frame_count: values.len() as u32,
                    },
                    compression: 0,
                };

                data.write(writer).unwrap();
            }
            TrackData::Vector4(_) => todo!(),
        },
        _ => match track_data {
            // Use the same representation for all the non compressed data.
            // TODO: Is there any difference between these types?
            // TODO: Does it matter if a const or const transform has more than 1 frame?
            // TODO: Can const transform work with non transform data?
            TrackData::Transform(values) => {
                let new_values: Vec<ConstantTransform> = values.iter().map(|t| t.into()).collect();
                new_values.write(writer).unwrap()
            }
            TrackData::UvTransform(values) => values.write(writer).unwrap(),
            TrackData::Float(values) => values.write(writer).unwrap(),
            TrackData::PatternIndex(values) => values.write(writer).unwrap(),
            TrackData::Boolean(values) => {
                let values: Vec<Boolean> = values.iter().map(|b| b.into()).collect();
                values.write(writer).unwrap();
            }
            TrackData::Vector4(values) => values.write(writer).unwrap(),
        },
    }
}

fn read_compressed<R: Read + Seek, T: CompressedData>(
    reader: &mut R,
    frame_count: usize,
) -> Vec<T> {
    let data: CompressedTrackData<T> = reader.read_le().unwrap();

    // Decompress values.
    let bit_buffer = BitReadBuffer::new(&data.header.compressed_data.0, bitbuffer::LittleEndian);
    let mut bit_reader = BitReadStream::new(bit_buffer);

    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value = T::read_bits(
            &data.header,
            &mut bit_reader,
            &data.compression,
            &data.header.default_data,
        );
        values.push(value);
    }

    values
}

fn read_transform_compressed(
    header: &CompressedHeader<Transform>,
    bit_stream: &mut BitReadStream<LittleEndian>,
    compression: &TransformCompression,
    default: &Transform,
) -> Transform {
    let compensate_scale = if header.flags.has_compensate_scale() && header.flags.has_scale() {
        read_compressed_f32(bit_stream, &compression.scale.x).unwrap_or(0.0)
    } else {
        0.0
    };
    let scale = if header.flags.has_scale() {
        read_compressed_vector3(bit_stream, &compression.scale, &default.scale)
    } else {
        default.scale
    };
    let rotation = if header.flags.has_rotation() {
        // TODO: Add basic vector conversions and swizzling.
        // TODO: The w component is handled separately.
        let default_rotation_xyz =
            Vector3::new(default.rotation.x, default.rotation.y, default.rotation.z);

        let rotation_xyz =
            read_compressed_vector3(bit_stream, &compression.rotation, &default_rotation_xyz);
        Vector4::new(rotation_xyz.x, rotation_xyz.y, rotation_xyz.z, f32::NAN)
    } else {
        default.rotation
    };
    let translation = if header.flags.has_position() {
        read_compressed_vector3(bit_stream, &compression.translation, &default.translation)
    } else {
        default.translation
    };
    let rotation_w = calculate_rotation_w(header, bit_stream, rotation, default);
    let rotation = Vector4::new(rotation.x, rotation.y, rotation.z, rotation_w);
    Transform {
        scale,
        rotation,
        translation,
        compensate_scale,
    }
}

fn calculate_rotation_w(
    header: &CompressedHeader<Transform>,
    bit_stream: &mut BitReadStream<LittleEndian>,
    rotation: Vector4,
    default: &Transform,
) -> f32 {
    if header.flags.has_rotation() {
        let w_flip = bit_stream.read_bool().unwrap();

        // TODO: Is there a nicer way to express solving for w for a unit quaternion?
        // The compression assumes unit quaternions, so we can solve for w.
        let w = f32::sqrt(
            1.0 - (rotation.x * rotation.x + rotation.y * rotation.y + rotation.z * rotation.z),
        );

        if w_flip {
            -w
        } else {
            w
        }
    } else {
        default.rotation.w
    }
}

fn read_pattern_index_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &U32Compression,
    default: &u32,
) -> u32 {
    // TODO: There's only a single track in game that uses this, so this is just a guess.
    // TODO: How to compress a u32 with min, max, and bitcount?
    let value: u32 = bit_stream.read_int(compression.bit_count as usize).unwrap();
    value + compression.min
}

fn read_texture_data_compressed(
    header: &CompressedHeader<UvTransform>,
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &TextureDataCompression,
    default: &UvTransform,
) -> UvTransform {
    // TODO: Is this correct?
    let unk1 = if header.flags.has_scale() {
        read_compressed_f32(bit_stream, &compression.unk1).unwrap_or(default.unk1)
    } else {
        default.unk1
    };

    let unk2 = if header.flags.has_scale() {
        read_compressed_f32(bit_stream, &compression.unk2).unwrap_or(default.unk2)
    } else {
        default.unk2
    };

    // TODO: What toggles unk3?
    let unk3 = if header.flags.has_rotation() || header.flags.has_compensate_scale() {
        read_compressed_f32(bit_stream, &compression.unk3).unwrap_or(default.unk3)
    } else {
        default.unk3
    };

    let unk4 = if header.flags.has_position() {
        read_compressed_f32(bit_stream, &compression.unk4).unwrap_or(default.unk4)
    } else {
        default.unk4
    };

    let unk5 = if header.flags.has_position() {
        read_compressed_f32(bit_stream, &compression.unk5).unwrap_or(default.unk5)
    } else {
        default.unk5
    };

    UvTransform {
        unk1,
        unk2,
        unk3,
        unk4,
        unk5,
    }
}

fn read_vector4_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &Vector4Compression,
    default: &Vector4,
) -> Vector4 {
    let x = read_compressed_f32(bit_stream, &compression.x).unwrap_or(default.x);
    let y = read_compressed_f32(bit_stream, &compression.y).unwrap_or(default.y);
    let z = read_compressed_f32(bit_stream, &compression.z).unwrap_or(default.z);
    let w = read_compressed_f32(bit_stream, &compression.w).unwrap_or(default.w);
    Vector4::new(x, y, z, w)
}

fn read_compressed_vector3(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &Vector3Compression,
    default: &Vector3,
) -> Vector3 {
    let x = read_compressed_f32(bit_stream, &compression.x).unwrap_or(default.x);
    let y = read_compressed_f32(bit_stream, &compression.y).unwrap_or(default.y);
    let z = read_compressed_f32(bit_stream, &compression.z).unwrap_or(default.z);
    Vector3::new(x, y, z)
}

fn read_compressed_f32(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &F32Compression,
) -> Option<f32> {
    let value: u32 = bit_stream.read_int(compression.bit_count as usize).unwrap();
    decompress_f32(
        value,
        compression.min,
        compression.max,
        NonZeroU64::new(compression.bit_count),
    )
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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::Vector4(values) => {
                assert_eq!(vec![Vector4::new(0.4, 1.5, 1.0, 1.0)], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let mut writer = Cursor::new(Vec::new());
        write_track_data(
            &mut writer,
            &TrackData::Vector4(vec![Vector4::new(0.4, 1.5, 1.0, 1.0)]),
            CompressionType::Constant,
        );

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("cdcccc3e0000c03f0000803f0000803f",)
        );
    }

    #[test]
    fn read_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let data = hex_bytes("0000803f0000803f000000000000000000000000");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::UvTransform(values) => {
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
        write_track_data(
            &mut writer,
            &TrackData::UvTransform(vec![UvTransform {
                unk1: 1.0,
                unk2: 1.0,
                unk3: 0.0,
                unk4: 0.0,
                unk5: 0.0,
            }]),
            CompressionType::Constant,
        );

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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Compressed,
            },
            4,
        );

        // TODO: This is just a guess based on the flags.
        match values {
            TrackData::UvTransform(values) => {
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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::PatternIndex(values) => {
                assert_eq!(vec![1], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_pattern_index_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture0[0].PatternIndex
        let mut writer = Cursor::new(Vec::new());
        write_track_data(
            &mut writer,
            &TrackData::PatternIndex(vec![1]),
            CompressionType::Constant,
        );

        assert_eq!(*writer.get_ref(), hex_bytes("01000000",));
    }

    #[test]
    fn read_compressed_pattern_index_multiple_frames() {
        // stage/fzero_mutecity3ds/normal/motion/s05_course/s05_course__l00b.nuanmb, phong32__S_CUS_0xa3c00501___NORMEXP16_, DiffuseUVTransform.PatternIndex.
        // Shortened from 650 to 8 frames.
        let data =
            hex_bytes("0400000020000100240000008a0200000100000002000000010000000000000001000000fe");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Compressed,
            },
            8,
        );

        // TODO: This is just a guess for min: 1, max: 2, bit_count: 1.
        match values {
            TrackData::PatternIndex(values) => {
                assert_eq!(vec![1, 2, 2, 2, 2, 2, 2, 2], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let data = hex_bytes("cdcccc3e");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::Float(values) => {
                assert_eq!(vec![0.4], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let mut writer = Cursor::new(Vec::new());
        write_track_data(
            &mut writer,
            &TrackData::Float(vec![0.4]),
            CompressionType::Constant,
        );

        assert_eq!(*writer.get_ref(), hex_bytes("cdcccc3e",));
    }

    #[test]
    fn read_compressed_float_multiple_frames() {
        // pacman/model/body/c00/model.nuanmb, phong3__phong0__S_CUS_0xa2001001___7__AT_GREATER128___VTC__NORMEXP16___CULLNONE_A_AB_SORT, CustomFloat2
        let data = hex_bytes(
            "040000002000020024000000050000000000000000004040020000000000000000000000e403",
        );
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Compressed,
            },
            5,
        );

        match values {
            TrackData::Float(values) => {
                assert_eq!(vec![0.0, 1.0, 2.0, 3.0, 3.0], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_constant_boolean_single_frame_true() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean1
        let data = hex_bytes("01");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::Boolean(values) => {
                assert_eq!(vec![true], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_constant_boolean_single_frame_true() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean1
        let mut writer = Cursor::new(Vec::new());
        write_track_data(
            &mut writer,
            &TrackData::Boolean(vec![true]),
            CompressionType::Constant,
        );

        assert_eq!(*writer.get_ref(), hex_bytes("01"));
    }

    #[test]
    fn read_constant_boolean_single_frame_false() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean11
        let data = hex_bytes("00");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::Boolean(values) => {
                assert_eq!(vec![false], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn read_compressed_boolean_multiple_frames() {
        // assist\ashley\motion\body\c00, magic, Visibility
        let data =
            hex_bytes("04000000200001002100000003000000000000000000000000000000000000000006");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Compressed,
            },
            3,
        );

        match values {
            TrackData::Boolean(values) => {
                assert_eq!(vec![false, true, true], values)
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn write_compressed_boolean_multiple_frames() {
        // assist\ashley\motion\body\c00, magic, Visibility
        let mut writer = Cursor::new(Vec::new());
        write_track_data(
            &mut writer,
            &TrackData::Boolean(vec![false, true, true]),
            CompressionType::Compressed,
        );
        assert_eq!(
            *writer.get_ref(),
            hex_bytes("04000000200001002100000003000000000000000000000000000000000000000006")
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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Compressed,
            },
            8,
        );

        match values {
            TrackData::Vector4(values) => {
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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::ConstTransform,
            },
            1,
        );

        match values {
            TrackData::Transform(values) => {
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
        write_track_data(
            &mut writer,
            &TrackData::Transform(vec![Transform {
                translation: Vector3::new(1.51284, -0.232973, -0.371597),
                rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                scale: Vector3::new(1.0, 1.0, 1.0),
                compensate_scale: 1.0,
            }]),
            CompressionType::Constant,
        );

        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "0000803f0000803f0000803f000000000000000
            0000000000000803fbea4c13f79906ebef641bebe01000000",
            )
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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::Compressed,
            },
            2,
        );

        match values {
            TrackData::Transform(values) => {
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
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::Direct,
            },
            2,
        );

        match values {
            TrackData::Transform(values) => {
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
