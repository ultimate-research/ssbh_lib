use binread::{BinRead, BinResult, ReadOptions};
use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};
use modular_bitfield::prelude::*;
use std::{
    io::{Cursor, Read, Seek, Write},
    num::NonZeroU64,
};

use binread::BinReaderExt;
use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags, TrackType},
    Ptr16, Ptr32, Vector3, Vector4,
};

#[derive(Debug, BinRead)]
struct CompressedTrackData<T: CompressedData> {
    pub header: CompressedHeader<T>,
    pub compression: T::Compression,
}

// TODO: It should be possible to derive SsbhWrite.
#[derive(Debug, BinRead)]
struct CompressedHeader<T: CompressedData> {
    // TODO: Modular bitfield for these two u16s.
    pub unk_4: u16,
    pub flags: CompressionFlags,
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

#[derive(Debug, BinRead)]
struct CompressedBuffer(#[br(parse_with = read_to_end)] Vec<u8>);

#[bitfield(bits = 16)]
#[derive(Debug, BinRead)]
#[br(map = Self::from_bytes)]
struct CompressionFlags {
    has_scale: bool,            // TODO: Is this right?
    has_compensate_scale: bool, // 0b11 is apparently compensate scale, so is this a two bit enum?
    has_rotation: bool,
    has_position: bool,
    #[skip]
    __: B12,
}

// TODO: This is probably a UV transform.
// TODO: Test cases.
#[derive(Debug, BinRead)]
pub struct TextureData {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
}

#[derive(Debug, BinRead, PartialEq)]
pub struct Transform {
    pub scale: Vector3,
    pub rotation: Vector4, // TODO: Special type for quaternion?
    pub translation: Vector3,
    pub compensate_scale: f32,
}

#[derive(Debug, BinRead, PartialEq)]
struct ConstantTransform {
    pub scale: Vector3,
    pub rotation: Vector4, // TODO: Special type for quaternion?
    pub translation: Vector3,
    pub compensate_scale: u32,
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

#[derive(Debug, BinRead, Clone)]
struct FloatCompression {
    pub min: f32,
    pub max: f32,
    pub bit_count: u64,
}

#[derive(Debug, BinRead)]
struct Vector3Compression {
    pub x: FloatCompression,
    pub y: FloatCompression,
    pub z: FloatCompression,
}

#[derive(Debug, BinRead)]
struct Vector4Compression {
    pub x: FloatCompression,
    pub y: FloatCompression,
    pub z: FloatCompression,
    pub w: FloatCompression,
}

#[derive(Debug, BinRead)]
struct TransformCompression {
    // TODO: The first component of scale can also be compensate scale?
    pub scale: Vector3Compression,
    // TODO: w for rotation is handled separately.
    pub rotation: Vector3Compression,
    pub translation: Vector3Compression,
}

#[derive(Debug)]
pub enum TrackData {
    Transform(Vec<Transform>),
    Texture(Vec<TextureData>),
    Float(Vec<f32>),
    PatternIndex(Vec<u32>),
    Boolean(Vec<bool>),
    Vector4(Vec<Vector4>),
}

// Shared logic for decompressing track data from a header and collection of bits.
trait CompressedData: BinRead<Args = ()> {
    type Compression: BinRead<Args = ()>;

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

#[derive(Debug, BinRead)]
struct Boolean(u8);

impl CompressedData for Boolean {
    // There are 16 bytes for determining the compression, but all bytes are set to 0.
    type Compression = u128;

    fn read_bits(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> Self {
        // Boolean compression is based on bits per entry, which is usually set to 1 bit.
        let value = stream
            .read_int::<u8>(header.bits_per_entry as usize)
            .unwrap();
        Boolean(value)
    }
}

// TODO: Frame count for const transform?
// TODO: Avoid unwrap and handle errors.
fn read_track_data(track_data: &[u8], flags: TrackFlags, frame_count: usize) -> TrackData {
    // TODO: Organize this match statement.
    // TODO: Are Const, ConstTransform, and Direct all the same?
    // TODO: Can frame count be higher than 1 for the above compression types?

    match (flags.track_type, flags.compression_type) {
        (TrackType::Float, CompressionType::Constant) => {
            // TODO: Frame count?
            let mut reader = Cursor::new(track_data);
            let value: f32 = reader.read_le().unwrap();
            TrackData::Float(vec![value])
        }
        // TODO: Compressed floats?
        (TrackType::Vector4, CompressionType::Compressed) => {
            let mut reader = Cursor::new(track_data);
            let values = read_track_compressed(&mut reader, frame_count);
            TrackData::Vector4(values)
        }
        (TrackType::Vector4, _) => {
            // TODO: read_direct function where T: BinRead.
            let mut reader = Cursor::new(track_data);
            let mut values = Vec::new();
            for _ in 0..frame_count {
                let value: Vector4 = reader.read_le().unwrap();
                values.push(value);
            }
            TrackData::Vector4(values)
        }
        (TrackType::Texture, CompressionType::Constant) => {
            let mut reader = Cursor::new(track_data);
            let value: TextureData = reader.read_le().unwrap();
            TrackData::Texture(vec![value])
        }
        // TODO: Compressed texture?
        (TrackType::PatternIndex, CompressionType::Constant) => {
            let mut reader = Cursor::new(track_data);
            let value: u32 = reader.read_le().unwrap();
            TrackData::PatternIndex(vec![value])
        }
        // TODO: Compressed pattern index?
        (TrackType::Boolean, CompressionType::Compressed) => {
            let mut reader = Cursor::new(track_data);
            let values: Vec<Boolean> = read_track_compressed(&mut reader, frame_count);
            TrackData::Boolean(values.iter().map(|b| b.0 != 0).collect())
        }
        (TrackType::Boolean, _) => {
            let mut reader = Cursor::new(track_data);
            let mut values = Vec::new();
            for _ in 0..frame_count {
                // TODO: from<Boolean> for bool?
                let value: Boolean = reader.read_le().unwrap();
                values.push(value.0 != 0);
            }
            TrackData::Boolean(values)
        }
        (TrackType::Transform, CompressionType::Compressed) => {
            let mut reader = Cursor::new(track_data);
            let values = read_track_compressed(&mut reader, frame_count);
            TrackData::Transform(values)
        }
        (TrackType::Transform, _) => {
            let mut reader = Cursor::new(track_data);
            let mut values = Vec::new();
            for _ in 0..frame_count {
                let value: ConstantTransform = reader.read_le().unwrap();
                values.push(value.into());
            }
            TrackData::Transform(values)
        }
        _ => panic!("Unsupported flags"),
    }
}

fn write_track_data<W: Write + Seek>(
    writer: &mut W,
    track_data: &TrackData,
    compression: CompressionType,
) {
    match compression {
        CompressionType::Direct => todo!(),
        CompressionType::ConstTransform => todo!(),
        CompressionType::Compressed => match track_data {
            TrackData::Transform(_) => todo!(),
            TrackData::Texture(_) => todo!(),
            TrackData::Float(_) => todo!(),
            TrackData::PatternIndex(_) => todo!(),
            TrackData::Boolean(values) => {}
            TrackData::Vector4(_) => todo!(),
        },
        CompressionType::Constant => todo!(),
    }
}

fn read_track_compressed<R: Read + Seek, T: CompressedData>(
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
    let rotation_w = if header.flags.has_rotation() {
        let w_flip = bit_stream.read_bool().unwrap();
        // TODO: Is there a nicer way to express solving for w for a unit quaternion?
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
    };
    let rotation = Vector4::new(rotation.x, rotation.y, rotation.z, rotation_w);
    Transform {
        scale,
        rotation,
        translation,
        compensate_scale,
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
    compression: &FloatCompression,
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
    fn read_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let data = hex_bytes("0000803f0000803f000000000000000000000000");
        let values = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Texture,
                compression_type: CompressionType::Constant,
            },
            1,
        );

        match values {
            TrackData::Texture(values) => {
                assert_eq!(1, values.len());
                assert_eq!(1.0, values[0].unk1);
                assert_eq!(1.0, values[0].unk2);
                assert_eq!(0.0, values[0].unk3);
                assert_eq!(0.0, values[0].unk4);
                assert_eq!(0.0, values[0].unk5);
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
    fn writer_compressed_boolean_multiple_frames() {
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
}
