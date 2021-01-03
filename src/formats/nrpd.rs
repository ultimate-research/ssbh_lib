use crate::{SsbhArray, SsbhString};
use crate::RelPtr64;
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum StateType {
    #[br(magic = 0u64)]
    Sampler = 0,
    #[br(magic = 1u64)]
    RasterizerState = 1,
    #[br(magic = 2u64)]
    Unk = 2,
    #[br(magic = 3u64)]
    BlendState = 3,
}

#[derive(Serialize, BinRead, Debug)]
pub struct Framebuffer {
    name: SsbhString,
    width: u32,
    height: u32,
    unk1: u64,
    unk2: u32,
    unk3: u32
}

#[derive(Serialize, BinRead, Debug)]
pub struct FramebufferContainer {
    framebuffer: RelPtr64<Framebuffer>,
    unk1: u64
}

#[derive(Serialize, BinRead, Debug)]
pub struct StateObject {
    name: SsbhString,
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct StateContainer {
    state: RelPtr64<StateObject>,
    state_type: StateType
}

#[derive(Serialize, BinRead, Debug)]
pub struct UnkItem1 {
    unk1: u64,
    unk2: u64,
    unk3: u64,
    unk4: u64,
    unk5: u64,
    unk6: u64,
    unk7: u64,
    unk8: u64, // data type of unk9/unk10?
    unk9: f32,
    unk10: f32
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass {
    name: SsbhString,
    unk1: RelPtr64<UnkItem1>,
    unk2: u64,
    unk3: u64,
    unk4: u64,
    unk5: SsbhString,
    unk6: u64,
    unk7: u64
}

/// ???
#[derive(Serialize, BinRead, Debug)]
pub struct Nrpd {
    major_version: u16,
    minor_version: u16,
    frame_buffer_containers: SsbhArray<FramebufferContainer>,
    state_containers: SsbhArray<StateContainer>,
    render_passes: SsbhArray<RenderPass>
}
