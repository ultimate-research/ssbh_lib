use binread::BinRead;
use bitbuffer::{BitReadBuffer, BitReadStream};
use std::io::{Cursor, Read, Seek, SeekFrom};

use binread::BinReaderExt;
use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags, TrackType},
    Vector4,
};

// TODO: This is read at the start of the track's data for compressed data.
#[derive(Debug, BinRead)]
pub struct CompressedHeader {
    pub unk_4: u16,
    pub flags: u16,
    pub default_data_offset: u16,
    pub bits_per_entry: u16,
    pub compressed_data_offset: u32,
    pub frame_count: u32,
}

#[derive(Debug, BinRead)]
pub struct TextureData {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
}

#[derive(Debug, BinRead)]
pub struct ComponentCompression {
    pub min: f32,
    pub max: f32,
    pub bit_count: u64,
}

#[derive(Debug, BinRead)]
pub struct Vector4Compression {
    pub x: ComponentCompression,
    pub y: ComponentCompression,
    pub z: ComponentCompression,
    pub w: ComponentCompression,
}

#[derive(Debug)]
pub enum TrackData {
    Transform(),
    Texture(Vec<TextureData>),
    Float(Vec<f32>),
    PatternIndex(Vec<u32>),
    Boolean(Vec<bool>),
    Vector4(Vec<Vector4>),
}

// TODO: Frame count?
// TODO: Avoid unwrap.
fn read_track_data(track_data: &[u8], flags: TrackFlags, frame_count: usize) -> TrackData {
    // TODO: Organize this match statement.
    match (flags.track_type, flags.compression_type) {
        (TrackType::Vector4, CompressionType::Constant) => {
            let mut reader = Cursor::new(track_data);
            let value: Vector4 = reader.read_le().unwrap();
            TrackData::Vector4(vec![value])
        }
        (TrackType::Vector4, CompressionType::Compressed) => {
            let values = read_vector4_compressed(track_data, frame_count);
            TrackData::Vector4(values)
        }
        (TrackType::Texture, CompressionType::Constant) => {
            let mut reader = Cursor::new(track_data);
            let value: TextureData = reader.read_le().unwrap();
            TrackData::Texture(vec![value])
        }
        (TrackType::PatternIndex, CompressionType::Constant) => {
            let mut reader = Cursor::new(track_data);
            let value: u32 = reader.read_le().unwrap();
            TrackData::PatternIndex(vec![value])
        }
        (TrackType::Boolean, CompressionType::Constant) => {
            let mut reader = Cursor::new(track_data);
            let value: u8 = reader.read_le().unwrap();
            TrackData::Boolean(vec![value != 0])
        }
        _ => panic!("Unsupported flags"),
    }
}

fn read_vector4_compressed(track_data: &[u8], frame_count: usize) -> Vec<Vector4> {
    // Header.
    let mut reader = Cursor::new(track_data);
    let header: CompressedHeader = reader.read_le().unwrap();
    let compression: Vector4Compression = reader.read_le().unwrap();

    // Default values.
    reader
        .seek(SeekFrom::Start(header.default_data_offset as u64))
        .unwrap();
    let default: Vector4 = reader.read_le().unwrap();

    // Compressed values.
    reader
        .seek(SeekFrom::Start(header.compressed_data_offset as u64))
        .unwrap();
    let mut compressed_data = Vec::new();
    reader.read_to_end(&mut compressed_data).unwrap();

    // Decompress values.
    let bit_reader = BitReadBuffer::new(&compressed_data, bitbuffer::LittleEndian);
    let mut bit_stream = BitReadStream::new(bit_reader);
    let mut values = Vec::new();
    for _ in 0..frame_count {
        let x = read_compressed_f32(&mut bit_stream, &compression.x).unwrap_or(default.x);
        let y = read_compressed_f32(&mut bit_stream, &compression.y).unwrap_or(default.y);
        let z = read_compressed_f32(&mut bit_stream, &compression.z).unwrap_or(default.z);
        let w = read_compressed_f32(&mut bit_stream, &compression.w).unwrap_or(default.w);

        values.push(Vector4::new(x, y, z, w));
    }

    values
}

fn read_compressed_f32(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &ComponentCompression,
) -> Option<f32> {
    let value: u32 = bit_stream.read_int(compression.bit_count as usize).unwrap();
    println!("{:?} {:?}", value, compression);
    decompress_f32(
        value,
        compression.min,
        compression.max,
        compression.bit_count as usize,
    )
}

// TODO: Option<NonZeroUsize> for bit count?
fn decompress_f32(value: u32, min: f32, max: f32, bit_count: usize) -> Option<f32> {
    // A bit_count of 0 should use a default value.
    if bit_count == 0 {
        return None;
    }

    // This is similar to the conversion between floating point and integer values in RGB colors
    // The difference is that anim compression supports custom ranges and non standard bit counts.

    // Get a mask of bit_count many bits set to 1.
    let scale = (1 << bit_count) - 1;

    // TODO: There may be some edge cases with this implementation of linear interpolation.
    // TODO: Divide by 0.0?
    let lerp = |a, b, t| a * (1.0 - t) + b * t;
    let value = lerp(min, max, value as f32 / scale as f32);
    Some(value)
}

#[cfg(test)]
mod tests {
    use crate::hex_bytes;

    use super::*;

    #[test]
    fn decompress_float_0bit() {
        // fighter/cloud/motion/body/c00/b00guardon.nuanmb, EyeL, CustomVector31
        assert_eq!(None, decompress_f32(0, 1.0, 1.0, 0));
        assert_eq!(None, decompress_f32(0, 0.0, 0.0, 0));
    }

    #[test]
    fn decompress_float_14bit() {
        // stage/poke_unova/battle/motion/s13_a, D_lightning_B, CustomVector3
        assert_eq!(Some(1.25400329), decompress_f32(2350, 0.0, 8.74227, 14));
        assert_eq!(Some(1.18581951), decompress_f32(2654, 0.0, 7.32, 14));
        assert_eq!(Some(2.96404815), decompress_f32(2428, 0.0, 20.0, 14));
        assert_eq!(Some(1.21878445), decompress_f32(2284, 0.0, 8.74227, 14));
    }

    #[test]
    fn constant_vector4_single_frame() {
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

        // TODO: Compare two lists using partialeq?
        match values {
            TrackData::Vector4(values) => {
                assert_eq!(vec![Vector4::new(0.4, 1.5, 1.0, 1.0)], values);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn constant_texture_single_frame() {
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
    fn constant_pattern_index_single_frame() {
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
    fn constant_boolean_single_frame_true() {
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
    fn constant_boolean_single_frame_false() {
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
    fn compressed_vector4_multiple_frames() {
        // fighter/cloud/motion/body/c00/b00guardon.nuanmb, EyeL, CustomVector31
        let data = hex_bytes("040000005000030060000000080000000000803f0000803f00000000000000000000803f0000803f0000000
            0000000008fc2f5bd295c8fbd0300000000000000000000000000000000000000000000000000803f0000803f295c8fbd00000000773905");
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
}
