use crate::DataType;
use crate::{Color4f, InlineString, RelPtr64, Vector4};
use crate::{SsbhArray, SsbhEnum64, SsbhString};
use binread::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

use super::matl::{BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MinFilter, WrapMode};

// TODO: Why are there slightly smaller variants?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

impl DataType for FrameBuffer {
    fn data_type(&self) -> u64 {
        match self {
            FrameBuffer::Framebuffer0(_) => 0,
            FrameBuffer::Framebuffer1(_) => 1,
            FrameBuffer::Framebuffer2(_) => 2,
            FrameBuffer::Framebuffer3(_) => 3,
            FrameBuffer::Framebuffer4(_) => 4,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer2 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Framebuffer4 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk3: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct FramebufferContainer {
    pub frame_buffer: SsbhEnum64<FrameBuffer>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Sampler {
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RasterizerState {
    pub name: SsbhString,
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct BlendState {
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

// TODO: There is only a single instance of this struct?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct DepthState {
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum State {
    #[br(pre_assert(data_type == 0u64))]
    Sampler(Sampler),

    #[br(pre_assert(data_type == 1u64))]
    RasterizerState(RasterizerState),

    #[br(pre_assert(data_type == 2u64))]
    DepthState(DepthState),

    #[br(pre_assert(data_type == 3u64))]
    BlendState(BlendState),
}

impl DataType for State {
    fn data_type(&self) -> u64 {
        match self {
            State::Sampler(_) => 0,
            State::RasterizerState(_) => 1,
            State::DepthState(_) => 2,
            State::BlendState(_) => 3,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct StateContainer {
    pub state: SsbhEnum64<State>,
}

// TODO: The variant names are just guesses based on the string values.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

impl DataType for RenderPassData {
    fn data_type(&self) -> u64 {
        match self {
            RenderPassData::FramebufferRtp(_) => 0,
            RenderPassData::Depth(_) => 1,
            RenderPassData::UnkTexture1(_) => 2,
            RenderPassData::UnkLight(_) => 3,
            RenderPassData::Unk8(_) => 8,
            RenderPassData::ColorClear(_) => 9,
            RenderPassData::DepthStencilClear(_) => 10,
            RenderPassData::Viewport(_) => 12,
            RenderPassData::Sampler(_) => 13,
            RenderPassData::BlendState(_) => 14,
            RenderPassData::RasterizerState(_) => 15,
            RenderPassData::DepthStencilState(_) => 16,
            RenderPassData::FramebufferRenderTarget(_) => 17,
            RenderPassData::FramebufferDepthStencil(_) => 18,
            RenderPassData::UnkTexture2(_) => 19,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData0 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Unk8Data {
    unk1: u32,
    unk2: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData9 {
    unk1: SsbhString,
    unk2: Vector4,
    unk4: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData10 {
    unk1: SsbhString,
    unk2: f32,
    unk3: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RenderPassData12 {
    unk1: SsbhString,
    unk2: u64,
    unk3: Vector4,
    unk4: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum RenderPassUnkData {
    #[br(pre_assert(data_type == 0u64))]
    Unk0(InlineString),

    #[br(pre_assert(data_type == 3u64))]
    Unk3(Unk3Data),
}

impl DataType for RenderPassUnkData {
    fn data_type(&self) -> u64 {
        match self {
            RenderPassUnkData::Unk0(_) => 0,
            RenderPassUnkData::Unk3(_) => 3,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Unk3Data {
    pub name: SsbhString,
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: f32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct StringPair {
    pub item1: SsbhString,
    pub item2: SsbhString,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem1 {
    pub unk1: SsbhString,
    pub unk2: SsbhArray<UnkItem3>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem2 {
    pub unk1: RelPtr64<StringPair>,
    pub unk2: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem3 {
    pub name: SsbhString,
    pub value: SsbhString,
}

/// Render pipeline data.
/// Compatible with file version 1.6.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
