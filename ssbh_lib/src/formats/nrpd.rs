use crate::{Color4f, RelPtr64};
use crate::{SsbhArray, SsbhEnum64, SsbhString};
use binread::BinRead;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

use super::matl::{BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MinFilter, WrapMode};

// TODO: Why are there slightly smaller variants?
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum FrameBuffer {
    #[br(pre_assert(data_type == 0u64))]
    Framebuffer0(Framebuffer0),

    #[br(pre_assert(data_type == 1u64))]
    Framebuffer1(Framebuffer1),

    #[br(pre_assert(data_type == 2u64))]
    Framebuffer2(Framebuffer2),
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer0 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u64,
    pub unk2: u32,
    pub unk3: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer1 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u64,
    pub unk2: u32,
    pub unk3: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer2 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct FramebufferContainer {
    pub frame_buffer: SsbhEnum64<FrameBuffer>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct NrpdSampler {
    pub name: SsbhString,
    pub wraps: WrapMode,
    pub wrapt: WrapMode,
    pub wrapr: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub texture_filtering_type: FilteringType,
    pub border_color: Color4f,
    pub unk11: u32,
    pub unk12: u32,
    pub lod_bias: f32,
    pub max_anisotropy: u32,
    pub unk13: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct NrpdRasterizerState {
    pub name: SsbhString,
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct NrpdBlendState {
    pub name: SsbhString,
    pub source_color: BlendFactor,
    pub unk2: u32,
    pub destination_color: BlendFactor,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
}

/// A state type similar to `NrpdBlendState`.
/// There is only a single instance of this struct,
/// which make it's fields difficult to determine.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
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

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
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

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct StateContainer {
    pub state: SsbhEnum64<NrpdState>,
}

// TODO: These are just guesses based on the string values.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u64))]
pub enum RenderPassDataType {
    FramebufferRtp = 0,
    Depth = 1,
    UnkTexture1 = 2,
    Unk8 = 8,
    ColorClear = 9,
    DepthClear = 10,
    Viewport = 12,
    Sampler = 13,
    BlendState = 14,
    RasterizerState = 15,
    DepthStencilState = 16,
    FramebufferRenderTarget = 17,
    UnkTexture2 = 19,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData {
    pub data: RelPtr64<SsbhString>,
    pub data_type: RenderPassDataType,
}

// TODO: Is there an easy way to handle all of this indirection?
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 8)]
pub struct RenderPassContainer {
    pub name: SsbhString,
    pub unk1: SsbhArray<RenderPassData>,
    pub unk2: SsbhArray<RenderPassData>,
    pub unk3: SsbhString, // name of the next render pass? TODO: this shouldn't write an extra string.
    #[br(pad_after = 8)]
    pub unk3_type: u64, // 0 for strings or 3 for an offset to existing SsbhArray<RenderPassData>
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct StringPair {
    pub item1: SsbhString,
    pub item2: SsbhString,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem2 {
    pub unk1: RelPtr64<StringPair>,
    pub unk2: u64,
}

/// Render pipeline data.
/// Compatible with file version 1.6.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Nrpd {
    pub major_version: u16,
    pub minor_version: u16,
    pub frame_buffer_containers: SsbhArray<FramebufferContainer>,
    pub state_containers: SsbhArray<StateContainer>,
    pub render_passes: SsbhArray<RenderPassContainer>, // TODO: Fix this
    pub unk_string_list1: SsbhArray<StringPair>,
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
