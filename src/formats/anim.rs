use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

use binread::BinRead;

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct AnimTrack {
    pub name: SsbhString,
    pub flags: u32,
    pub frame_count: u32,
    pub unk3: u32,
    pub data_offset: u32,
    pub data_size: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct AnimNode {
    pub name: SsbhString,
    pub tracks: SsbhArray<AnimTrack>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct AnimGroup {
    pub anim_type: AnimType,
    pub nodes: SsbhArray<AnimNode>,
}

/// Skeletal and material animation.
/// Compatible with file version 2.0 and 2.1.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct Anim {
    pub major_version: u16,
    pub minor_version: u16,
    pub final_frame_index: f32,
    pub unk1: u16,
    pub unk2: u16,
    pub name: SsbhString,
    pub animations: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
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
