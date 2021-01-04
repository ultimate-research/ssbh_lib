use crate::{SsbhArray, SsbhString, SsbhEnum};
use crate::RelPtr64;
use binread::BinRead;
use serde::Serialize;

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
    // TODO: Is this another enum?
    // unk1 is always 2.
    framebuffer: RelPtr64<Framebuffer>,
    unk1: u64 
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdSampler {
    name: SsbhString,
    data: crate::matl::MatlSampler
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdRasterizerState {
    name: SsbhString,
    data: crate::matl::MatlRasterizerState
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdUnkData {
    name: SsbhString
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdBlendState {
    name: SsbhString,
    data: crate::matl::MatlBlendState
}

#[derive(Serialize, BinRead, Debug)]
#[br(import(data_type: u64))]
pub enum NrpdState {
    #[br(pre_assert(data_type == 0u64))]
    Sampler(NrpdSampler),

    #[br(pre_assert(data_type == 1u64))]
    RasterizerState(NrpdRasterizerState),

    #[br(pre_assert(data_type == 2u64))]
    Unk(NrpdUnkData),

    #[br(pre_assert(data_type == 3u64))]
    BlendState(NrpdBlendState)
}

#[derive(Serialize, BinRead, Debug)]
pub struct StateContainer {
    state: SsbhEnum<NrpdState, u64>,
}

#[derive(Serialize, BinRead, Debug)]
#[br(import(data_type: u64))]
pub enum RenderPassData {
    #[br(pre_assert(data_type == 0u64))]
    Unk0(),

    #[br(pre_assert(data_type == 1u64))]
    Unk1,    

    #[br(pre_assert(data_type == 2u64))]
    Unk2,   

    #[br(pre_assert(data_type == 3u64))]
    Unk3,

    #[br(pre_assert(data_type == 4u64))]
    Unk4,

    #[br(pre_assert(data_type == 5u64))]
    Unk5,

    #[br(pre_assert(data_type == 6u64))]
    Unk6,

    #[br(pre_assert(data_type == 7u64))]
    Unk7,

    #[br(pre_assert(data_type == 8u64))]
    Unk8,

    #[br(pre_assert(data_type == 9u64))]
    Unk9,

    #[br(pre_assert(data_type == 10u64))]
    Unk10,

    #[br(pre_assert(data_type == 11u64))]
    Unk11,

    #[br(pre_assert(data_type == 12u64))]
    Unk12,

    #[br(pre_assert(data_type == 14u64))]
    Unk14,

    #[br(pre_assert(data_type == 17u64))]
    Unk17,
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
    data1: SsbhEnum<RenderPassData, u64>,
    data2: SsbhEnum<RenderPassData, u64>,
    data3: SsbhEnum<RenderPassData, u64>,
    padding: u64
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
