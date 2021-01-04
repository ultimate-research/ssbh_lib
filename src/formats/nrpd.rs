use crate::RelPtr64;
use crate::{SsbhArray, SsbhEnum, SsbhString};
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct Framebuffer {
    name: SsbhString,
    width: u32,
    height: u32,
    unk1: u64,
    unk2: u32,
    unk3: u32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct FramebufferContainer {
    // TODO: Is this another enum?
    // unk1 is always 2.
    framebuffer: RelPtr64<Framebuffer>,
    unk1: u64,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdSampler {
    name: SsbhString,
    data: crate::matl::MatlSampler,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdRasterizerState {
    name: SsbhString,
    data: crate::matl::MatlRasterizerState,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdDepthState {
    name: SsbhString,
    unk2: u32, // 4 booleans (1 byte each)?
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
    unk7: u32,
    unk8: u64,
    unk9: u64,
    unk10: u64,
    unk11: u64,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdBlendState {
    name: SsbhString,
    data: crate::matl::MatlBlendState,
}

#[derive(Serialize, BinRead, Debug)]
#[br(import(data_type: u64))]
pub enum NrpdState {
    #[br(pre_assert(data_type == 0u64))]
    Sampler(NrpdSampler),

    #[br(pre_assert(data_type == 1u64))]
    RasterizerState(NrpdRasterizerState),

    #[br(pre_assert(data_type == 2u64))]
    DepthState(NrpdDepthState),

    #[br(pre_assert(data_type == 3u64))]
    BlendState(NrpdBlendState),
}

#[derive(Serialize, BinRead, Debug)]
pub struct StateContainer {
    state: SsbhEnum<NrpdState, u64>,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPassData {
    value: RelPtr64<SsbhString>,
    value_type: u64, // TODO: enum?
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPassContainer {
    name: SsbhString,
    unk1: SsbhArray<RenderPassData>,
    unk2: SsbhArray<RenderPassData>,
    unk3: SsbhString, // name of the next render pass?
    unk3_type: u64, // 0 for strings or 3 if empty
    padding: u64,
}

#[derive(Serialize, BinRead, Debug)]
pub struct UnkItem2 {
    unk1: RelPtr64<(SsbhString, SsbhString)>,
    unk2: u64
}

// This is based on file version 1.6.
/// ???
#[derive(Serialize, BinRead, Debug)]
pub struct Nrpd {
    major_version: u16,
    minor_version: u16,
    frame_buffer_containers: SsbhArray<FramebufferContainer>,
    state_containers: SsbhArray<StateContainer>,
    render_passes: SsbhArray<RenderPassContainer>,
    unk_string_list1: SsbhArray<(SsbhString, SsbhString)>,
    unk_string_list2: SsbhArray<UnkItem2>,
    unk1: u64,
    unk2: u64,
    unk3: u64,
    unk4: u64,
    unk5: u64,
    unk6: u64,
    unk7: u64,
    unk8: u64,
    unk9: u64,
}
