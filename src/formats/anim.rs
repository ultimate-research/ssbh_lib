use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

use binread::BinRead;

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimTrack {
    pub name: SsbhString,
    pub flags: TrackFlags,
    pub frame_count: u32,
    pub unk3: u32,
    pub data_offset: u32,
    pub data_size: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimNode {
    pub name: SsbhString,
    pub tracks: SsbhArray<AnimTrack>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimGroup {
    pub anim_type: AnimType,
    pub nodes: SsbhArray<AnimNode>,
}

/// Skeletal and material animation.
/// Compatible with file version 2.0 and 2.1.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Anim {
    pub major_version: u16,
    pub minor_version: u16,
    pub final_frame_index: f32,
    pub unk1: u16, // always 1
    pub unk2: u16, // always 3
    pub name: SsbhString,
    pub animations: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct TrackFlags {
    // TODO: Is this the best way to handle flags?
    pub track_type: TrackType,
    pub compression_type: CompressionType,
    #[cfg_attr(feature = "derive_serde", serde(skip))]
    pub padding: u16,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
pub enum TrackType {
    #[br(magic = 1u8)]
    Transform = 1,

    #[br(magic = 2u8)]
    Texture = 2,

    #[br(magic = 3u8)]
    Float = 3,

    #[br(magic = 5u8)]
    PatternIndex = 5,

    #[br(magic = 8u8)]
    Boolean = 8,

    #[br(magic = 9u8)]
    Vector4 = 9,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
pub enum CompressionType {
    #[br(magic = 1u8)]
    Direct = 1,

    #[br(magic = 2u8)]
    ConstTransform = 2,

    #[br(magic = 4u8)]
    Compressed = 4,

    #[br(magic = 5u8)]
    Constant = 5,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
pub enum AnimType {
    #[br(magic = 1u64)]
    Transform = 1,

    #[br(magic = 2u64)]
    Visibility = 2,

    #[br(magic = 4u64)]
    Material = 4,

    #[br(magic = 5u64)]
    Camera = 5,
}
