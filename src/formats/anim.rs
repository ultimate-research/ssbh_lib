use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use serde::Serialize;

use binread::BinRead;

#[derive(Serialize, BinRead, Debug)]
pub enum AnimType {
    #[br(magic = 1u64)]
    Transform,
    #[br(magic = 2u64)]
    Visibility,
    #[br(magic = 4u64)]
    Material,
    #[br(magic = 5u64)]
    Camera,
}

#[derive(Serialize, BinRead, Debug)]
pub struct AnimTrack {
    pub name: SsbhString,
    pub flags: u32,
    pub frame_count: u32,
    pub unk3: u32,
    // TODO: Use some sort of pointer type.
    pub data_offset: u32,
    pub data_size: u64,
}

#[derive(Serialize, BinRead, Debug)]
pub struct AnimNode {
    pub name: SsbhString,
    pub tracks: SsbhArray<AnimTrack>,
}

#[derive(Serialize, BinRead, Debug)]
pub struct AnimGroup {
    pub anim_type: AnimType,
    pub nodes: SsbhArray<AnimNode>,
}

/// Skeletal and material animation.
#[derive(Serialize, BinRead, Debug)]
pub struct Anim {
    pub major_version: u16,
    pub minor_version: u16,
    pub frame_count: f32,
    pub unk1: u16,
    pub unk2: u16,
    pub name: SsbhString,
    pub animations: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
}
