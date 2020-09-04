use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use serde::Serialize;

use binread::BinRead;

#[derive(Serialize, BinRead, Debug)]
enum AnimType {
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
struct AnimTrack {
    name: SsbhString,
    flags: u32,
    frame_count: u32,
    unk3: u32,
    // TODO: Use some sort of pointer type.
    data_offset: u32,
    data_size: u64,
}

#[derive(Serialize, BinRead, Debug)]
struct AnimNode {
    name: SsbhString,
    tracks: SsbhArray<AnimTrack>,
}

#[derive(Serialize, BinRead, Debug)]
struct AnimGroup {
    anim_type: AnimType,
    nodes: SsbhArray<AnimNode>,
}

/// Skeletal and material animation.
#[derive(Serialize, BinRead, Debug)]
pub struct Anim {
    major_version: u16,
    minor_version: u16,
    frame_count: f32,
    unk1: u16,
    unk2: u16,
    name: SsbhString,
    animations: SsbhArray<AnimGroup>,
    buffer: SsbhByteBuffer,
}
