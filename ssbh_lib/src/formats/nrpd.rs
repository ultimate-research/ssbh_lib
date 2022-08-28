//! The [Nrpd] format describes render pass data.
//! These files typically use the ".nurpdb" suffix.
use super::matl::{BlendFactor, CullMode, FillMode, Sampler};
use crate::enums::ssbh_enum;
use crate::{Color4f, RelPtr64, Version};
use crate::{SsbhArray, SsbhEnum64, SsbhString};
use binrw::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;
#[cfg(feature = "strum")]
use strum::{Display, EnumString, EnumVariantNames, FromRepr};

/// Render pass data.
/// Compatible with file version 1.6.
// TODO: Strings at the end don't need to be aligned?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Nrpd {
    #[br(pre_assert(major_version == 1 && minor_version == 6))]
    V16 {
        frame_buffers: SsbhArray<SsbhEnum64<FrameBuffer>>,
        state_containers: SsbhArray<SsbhEnum64<State>>,
        // TODO: The data pointer is too small after writing the render_passes.
        render_passes: SsbhArray<RenderPassContainer>,
        unk_string_list1: SsbhArray<StringPair>,
        unk_string_list2: SsbhArray<SsbhEnum64<UnkItem2>>,
        unk_list: SsbhArray<UnkItem1>,
        unk_width1: u32,
        unk_height1: u32,
        unk3: u32,
        unk4: u32,
        unk5: u32,
        unk6: u32, // flags?
        unk7: u32,
        unk8: u32,
        unk9: SsbhString, // empty string?
        unk_width2: u32,
        unk_height2: u32,
        unk10: u64,
    },
}

// TODO: Support 1.3
impl Version for Nrpd {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Nrpd::V16 { .. } => (1, 6),
        }
    }
}

// TODO: Inputs?
// TODO: These can just use named fields?
ssbh_enum!(
    FrameBuffer,
    0u64 => Framebuffer0(Framebuffer0),
    1u64 => Framebuffer1(Framebuffer1),
    2u64 => UniformBuffer(UniformBuffer),
    3u64 => Framebuffer3(Framebuffer3),
    4u64 => Framebuffer4(Framebuffer4)
);

// TODO: texture formats that match up with nutexb?
// num channels, format, type (unorm, srgb, etc)?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u64))]
#[ssbhwrite(repr(u64))]
pub enum UnkFormat {
    UnkFormat0 = 0x100,
    UnkFormat1 = 0x134,
    UnkFormat2 = 0x144,
    UnkFormat3 = 0x210,
    UnkFormat4 = 0x214,
    UnkFormat5 = 0x246,
    UnkFormat6 = 0x321,
    UnkFormat7 = 0x344,
    UnkFormat8 = 0x400,
    UnkFormat9 = 0x401,
    UnkFormat10 = 0x405,
    UnkFormat11 = 0x440,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct Framebuffer0 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk1: UnkFormat,
    pub unk2: u32,
    pub unk3: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
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
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UniformBuffer {
    pub name: SsbhString,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u64, // size in bytes?
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
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
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct Framebuffer4 {
    pub name: SsbhString,
    pub width: u32,
    pub height: u32,
    pub unk3: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
pub struct NrpdSampler {
    pub name: SsbhString,
    pub data: Sampler,
    pub unk13: u64, // 3 or 7
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
pub struct NrpdRasterizerState {
    pub name: SsbhString,
    // TODO: Use RasterizerStatev16 without the padding?
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct NrpdBlendState {
    pub name: SsbhString,
    // TODO: Use MatlBlendStateV16 without the padding?
    pub source_color: BlendFactor,
    pub unk2: u32,
    pub destination_color: BlendFactor,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    /// 1 = enabled, 0 = disabled
    pub alpha_sample_to_coverage: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
}

// TODO: There is only a single instance of this struct?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
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

ssbh_enum!(
    State,
    0u64 => Sampler(NrpdSampler),
    1u64 => RasterizerState(NrpdRasterizerState),
    2u64 => DepthState(DepthState),
    3u64 => BlendState(NrpdBlendState)
);

// TODO: The variant names are just guesses based on the string values.
ssbh_enum!(
    RenderPassData,
    0u64 =>  FramebufferRtp(RenderPassData0),
    1u64 =>  PassUnk1(RenderPassData1), // TODO
    2u64 =>  UnkTexture1(RenderPassData2),
    3u64 =>  UnkLight(RenderPassData3),
    8u64 =>  Unk8(RenderPassData8),
    9u64 =>  ColorClear(ColorClear),
    10u64 => DepthStencilClear(DepthStencilClear),
    12u64 => Viewport(Viewport),
    13u64 => Sampler(RenderPassData13),
    14u64 => BlendState(StringPair),
    15u64 => RasterizerState(StringPair),
    16u64 => DepthStencilState(StringPair),
    17u64 => FramebufferRenderTarget(SsbhString),
    18u64 => FramebufferDepthStencil(SsbhString),
    19u64 => UnkTexture2(RenderPassData19)
);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct RenderPassData0 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct RenderPassData1 {
    unk1: SsbhString,
    unk2: SsbhString,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct RenderPassData2 {
    unk1: SsbhString,
    unk2: SsbhString,
    // TODO: Arrays?
    unk3: RelPtr64<u64>,
    unk4: u64,
    unk5: RelPtr64<u64>,
    unk6: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
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
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct RenderPassData8 {
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: RelPtr64<u64>,
    unk4: u64,
    unk5: RelPtr64<u64>,
    unk6: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct RenderPassData19 {
    unk1: SsbhString,
    unk2: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct Unk8Data {
    unk1: u32,
    unk2: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
pub struct ColorClear {
    name: SsbhString,
    color: Color4f,
    unk1: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
pub struct DepthStencilClear {
    name: SsbhString,
    depth: f32,
    stencil: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
pub struct Viewport {
    name: SsbhString,
    unk2: u64,
    width: f32,
    height: f32,
    unk_min: f32, // depth min?
    unk_max: f32, // depth max?
    unk4: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
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
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
#[ssbhwrite(pad_after = 8)]
pub struct RenderPassContainer {
    pub name: SsbhString,
    pub unk1: SsbhArray<SsbhEnum64<RenderPassData>>,
    pub unk2: SsbhArray<SsbhEnum64<RenderPassData>>,
    #[br(pad_after = 8)]
    pub unk3: SsbhEnum64<RenderPassUnkData>,
}

// TODO: Find a better way to handle shared offsets.
ssbh_enum!(
    RenderPassUnkData,
    0 => UnkDataUnk0(()), // TODO: These offsets can be shared?
    3 => UnkDataUnk3(Unk3Data)
);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq)]
pub struct Unk3Data {
    pub unk1: SsbhString,
    pub unk2: SsbhString,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: f32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct StringPair {
    pub item1: SsbhString,
    pub item2: SsbhString,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UnkItem1 {
    pub unk1: SsbhString,
    pub unk2: SsbhArray<UnkItem3>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UnkItem20 {
    pub unk1: SsbhString,
    pub unk2: SsbhString,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UnkItem21 {
    pub unk1: SsbhString,
    pub unk2: SsbhString,
    pub unk3: [u32; 2],
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UnkItem22 {
    pub unk1: SsbhString,
    pub unk2: SsbhString,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: UnkFormat,
    pub unk8: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UnkItem24 {
    pub unk1: SsbhString,
    pub unk2: SsbhString,
    pub unk3: [u32; 6],
}

ssbh_enum!(
    UnkItem2,
    0u64 => UnkItem20(UnkItem20),
    1u64 => UnkItem21(UnkItem21),
    2u64 => UnkItem22(UnkItem22),
    4u64 => UnkItem24(UnkItem24)
);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct UnkItem3 {
    pub name: SsbhString,
    pub value: SsbhString,
}
