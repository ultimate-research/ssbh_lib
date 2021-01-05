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

// TODO: These are just guesses based on the string values.
#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum RenderPassDataType {
    #[br(magic = 0u64)]
    FramebufferRtp = 0,
    #[br(magic = 1u64)]
    Depth = 1,
    #[br(magic = 2u64)]
    UnkTexture1 = 2,
    #[br(magic = 8u64)]
    Unk8 = 8,
    #[br(magic = 9u64)]
    ColorClear = 9,
    #[br(magic = 10u64)]
    DepthClear = 10,
    #[br(magic = 12u64)]
    Viewport = 12,
    #[br(magic = 13u64)]
    Sampler = 13,
    #[br(magic = 14u64)]
    BlendState = 14,
    #[br(magic = 15u64)]
    RasterizerState = 15,
    #[br(magic = 16u64)]
    DepthStencilState = 16,
    #[br(magic = 17u64)]
    FramebufferRenderTarget = 17,
    #[br(magic = 19u64)]
    UnkTexture2 = 19,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPassData {
    data: RelPtr64<SsbhString>,
    data_type: RenderPassDataType
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPassContainer {
    name: SsbhString,
    unk1: SsbhArray<RenderPassData>,
    unk2: SsbhArray<RenderPassData>,
    unk3: SsbhString, // name of the next render pass?
    #[br(pad_after = 8)]
    unk3_type: u64, // 0 for strings or 3 if empty
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
    offset_to_last_byte: u64,
    unk8: u64,
    unk9: u64,
}
