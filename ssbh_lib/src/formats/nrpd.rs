use crate::{Color4f, InlineString, RelPtr64, Vector4};
use crate::{SsbhArray, SsbhEnum64, SsbhString};
use binread::BinRead;
use ssbh_write::SsbhWrite;
#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

use super::matl::{BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MinFilter, WrapMode};

// TODO: Why are there slightly smaller variants?
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum FrameBuffer {
    #[br(pre_assert(data_type == 0u64))]
    Framebuffer0(Framebuffer0),

    #[br(pre_assert(data_type == 1u64))]
    Framebuffer1(Framebuffer1),

    #[br(pre_assert(data_type == 2u64))]
    Framebuffer2(Framebuffer2),

    #[br(pre_assert(data_type == 3u64))]
    Framebuffer3(Framebuffer3),

    #[br(pre_assert(data_type == 4u64))]
    Framebuffer4(Framebuffer4),
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer2 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer3 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer4 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk3: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct FramebufferContainer {
    pub frame_buffer: SsbhEnum64<FrameBuffer>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct StateContainer {
    pub state: SsbhEnum64<NrpdState>,
}

// TODO: The variant names are just guesses based on the string values.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum RenderPassData {
    #[br(pre_assert(data_type == 0u64))]
    FramebufferRtp(RenderPassData0),

    #[br(pre_assert(data_type == 1u64))]
    Depth(u64), // TODO

    #[br(pre_assert(data_type == 2u64))]
    UnkTexture1(RenderPassData2),

    #[br(pre_assert(data_type == 3u64))]
    UnkLight(RenderPassData3),

    #[br(pre_assert(data_type == 8u64))]
    Unk8(RenderPassData8),

    #[br(pre_assert(data_type == 9u64))]
    ColorClear(RenderPassData9),

    #[br(pre_assert(data_type == 10u64))]
    DepthStencilClear(RenderPassData10),

    #[br(pre_assert(data_type == 12u64))]
    Viewport(RenderPassData12),

    #[br(pre_assert(data_type == 13u64))]
    Sampler(RenderPassData13),

    #[br(pre_assert(data_type == 14u64))]
    BlendState(StringPair),

    #[br(pre_assert(data_type == 15u64))]
    RasterizerState(StringPair),

    #[br(pre_assert(data_type == 16u64))]
    DepthStencilState(StringPair),

    #[br(pre_assert(data_type == 17u64))]
    FramebufferRenderTarget(SsbhString),

    #[br(pre_assert(data_type == 18u64))]
    FramebufferDepthStencil(SsbhString),

    #[br(pre_assert(data_type == 19u64))]
    UnkTexture2(u64), // TODO
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData0 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData2 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: RelPtr64<u64>,
    unk4: u64,
    unk5: RelPtr64<u64>,
    unk6: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData3 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: RelPtr64<Unk8Data>,
    unk4: u64,
    unk5: RelPtr64<Unk8Data>,
    unk6: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData8 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: RelPtr64<Unk8Data>,
    unk4: u64,
    unk5: RelPtr64<Unk8Data>,
    unk6: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Unk8Data {
    unk1: u32,
    unk2: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData9 {
    unk1: SsbhString,
    unk2: Vector4,
    unk4: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData10 {
    unk1: SsbhString,
    unk2: f32,
    unk3: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData12 {
    unk1: SsbhString,
    unk2: u64,
    unk3: Vector4,
    unk4: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData13 {
    unk1: SsbhString,
    unk2: SsbhString,
    // TODO: Are these arrays?
    unk3: RelPtr64<u64>,
    unk4: u64,
    unk5: RelPtr64<u64>,
    unk6: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 8)]
pub struct RenderPassContainer {
    pub name: SsbhString,
    pub unk1: SsbhArray<SsbhEnum64<RenderPassData>>,
    pub unk2: SsbhArray<SsbhEnum64<RenderPassData>>,
    #[br(pad_after = 8)]
    pub unk3: SsbhEnum64<RenderPassUnkData>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum RenderPassUnkData {
    #[br(pre_assert(data_type == 0u64))]
    Unk0(InlineString),

    #[br(pre_assert(data_type == 3u64))]
    Unk3(Unk3Data),
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Unk3Data {
    pub name: SsbhString,
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct StringPair {
    pub item1: SsbhString,
    pub item2: SsbhString,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem1 {
    pub unk1: SsbhString,
    pub unk2: SsbhArray<UnkItem3>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem2 {
    pub unk1: RelPtr64<StringPair>,
    pub unk2: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem3 {
    pub name: SsbhString,
    pub value: SsbhString,
}

/// Render pipeline data.
/// Compatible with file version 1.6.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Nrpd {
    pub major_version: u16,
    pub minor_version: u16,
    pub frame_buffer_containers: SsbhArray<FramebufferContainer>,
    pub state_containers: SsbhArray<StateContainer>,
    // TODO: The data pointer is too small after writing the render_passes.
    pub render_passes: SsbhArray<RenderPassContainer>,
    pub unk_string_list1: SsbhArray<StringPair>,
    pub unk_string_list2: SsbhArray<UnkItem2>,
    pub unk_list: SsbhArray<UnkItem1>,
    pub unk_width1: u32,
    pub unk_height1: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32, // flags?
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: SsbhString, // empty string?
    pub unk_width2: u32,
    pub unk_height2: u32,
    pub unk10: u64,
}
