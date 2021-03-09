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
#[br(repr(u8))]
pub enum TrackType {
    Transform = 1,
    Texture = 2,
    Float = 3,
    PatternIndex = 5,
    Boolean = 8,
    Vector4 = 9,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u8))]
pub enum CompressionType {
    Direct = 1,
    ConstTransform = 2,
    Compressed = 4,
    Constant = 5,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u64))]
pub enum AnimType {
    Transform = 1,
    Visibility = 2,
    Material = 4,
    Camera = 5,
}
