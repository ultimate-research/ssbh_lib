use crate::RelPtr64;
use crate::{SsbhArray, SsbhEnum64, SsbhString};
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct Framebuffer {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u64,
    pub unk2: u32,
    pub unk3: u32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct FramebufferContainer {
    // TODO: Is this another enum?
    // unk1 is always 2.
    pub framebuffer: RelPtr64<Framebuffer>,
    pub unk1: u64,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdSampler {
    pub name: SsbhString,
    pub data: crate::matl::MatlSampler,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdRasterizerState {
    pub name: SsbhString,
    pub data: crate::matl::MatlRasterizerState,
}

/// A state type similar to `NrpdBlendState`. 
/// There is only a single instance of this struct, 
/// which make it's fields difficult to determine.
#[derive(Serialize, BinRead, Debug)]
pub struct NrpdDepthState {
    pub name: SsbhString,
    pub unk2: u32, // 4 booleans (1 byte each)?
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u64,
    pub unk9: u64,
    pub unk10: u64,
    pub unk11: u64,
}

#[derive(Serialize, BinRead, Debug)]
pub struct NrpdBlendState {
    pub name: SsbhString,
    pub data: crate::matl::MatlBlendState,
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
    pub state: SsbhEnum64<NrpdState>,
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
    pub data: RelPtr64<SsbhString>,
    pub data_type: RenderPassDataType,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPassContainer {
    pub name: SsbhString,
    pub unk1: SsbhArray<RenderPassData>,
    pub unk2: SsbhArray<RenderPassData>,
    pub unk3: SsbhString, // name of the next render pass?
    #[br(pad_after = 8)]
    pub unk3_type: u64, // 0 for strings or 3 if empty
}

#[derive(Serialize, BinRead, Debug)]
pub struct UnkItem2 {
    pub unk1: RelPtr64<(SsbhString, SsbhString)>,
    pub unk2: u64,
}

// This is based on file version 1.6.
/// Render pipeline data. 
#[derive(Serialize, BinRead, Debug)]
pub struct Nrpd {
    pub major_version: u16,
    pub minor_version: u16,
    pub frame_buffer_containers: SsbhArray<FramebufferContainer>,
    pub state_containers: SsbhArray<StateContainer>,
    pub render_passes: SsbhArray<RenderPassContainer>,
    pub unk_string_list1: SsbhArray<(SsbhString, SsbhString)>,
    pub unk_string_list2: SsbhArray<UnkItem2>,
    pub unk1: u64,
    pub unk2: u64,
    pub unk3: u64,
    pub unk4: u64,
    pub unk5: u64,
    pub unk6: u64,
    pub offset_to_last_byte: u64,
    pub unk8: u64,
    pub unk9: u64,
}
