use binread::BinRead;
use std::io::Cursor;

use binread::BinReaderExt;
use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags, TrackType},
    Vector4,
};

#[derive(Debug, BinRead)]
pub struct TextureData {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
}

#[derive(Debug)]
enum TrackData {
    Transform(),
    Texture(TextureData),
    Float(f32),
    PatternIndex(),
    Boolean(bool),
    Vector4(Vector4),
}

// TODO: Frame count?
fn read_track_data(data: &[u8], flags: TrackFlags) -> TrackData {
    match (flags.track_type, flags.compression_type) {
        (TrackType::Vector4, CompressionType::Constant) => {
            let mut reader = Cursor::new(data);
            let value: Vector4 = reader.read_le().unwrap();
            TrackData::Vector4(value)
        }
        (TrackType::Texture, CompressionType::Constant) => {
            let mut reader = Cursor::new(data);
            let value: TextureData = reader.read_le().unwrap();
            TrackData::Texture(value)
        }
        _ => panic!("Unsupported flags"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_vector4_single_frame() {
        // mario/motion/body/c00/a00wait1.nuanmb "EyeL" "CustomVector30"
        let data = hex::decode("cdcccc3e0000c03f0000803f0000803f").unwrap();
        let value = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Constant,
            },
        );

        match value {
            TrackData::Vector4(Vector4 { x, y, z, w }) => {
                assert_eq!(0.4, x);
                assert_eq!(1.5, y);
                assert_eq!(1.0, z);
                assert_eq!(1.0, w);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn constant_texture_single_frame() {
        // mario/motion/body/c00/a00wait1.nuanmb "EyeL" "nfTexture1[0]"
        let data = hex::decode("0000803f0000803f000000000000000000000000").unwrap();
        let value = read_track_data(
            &data,
            TrackFlags {
                track_type: TrackType::Texture,
                compression_type: CompressionType::Constant,
            },
        );

        match value {
            TrackData::Texture(TextureData { unk1, unk2, unk3, unk4, unk5 }) => {
                assert_eq!(1.0, unk1);
                assert_eq!(1.0, unk2);
                assert_eq!(0.0, unk3);
                assert_eq!(0.0, unk4);
                assert_eq!(0.0, unk5);
            }
            _ => panic!("Unexpected variant"),
        }
    }
}
