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
#[br(import(data_type: u64))]
pub enum RenderPassData {
    #[br(pre_assert(data_type == 0u64))]
    RenderPass0(),

    #[br(pre_assert(data_type == 1u64))]
    RenderPass1(RenderPass1),

    #[br(pre_assert(data_type == 2u64))]
    RenderPass2(RenderPass2),

    #[br(pre_assert(data_type == 3u64))]
    RenderPass3(RenderPass3),

    #[br(pre_assert(data_type == 4u64))]
    RenderPass4(RenderPass4),

    #[br(pre_assert(data_type == 5u64))]
    RenderPass5(RenderPass5),

    #[br(pre_assert(data_type == 6u64))]
    RenderPass6(RenderPass6),

    #[br(pre_assert(data_type == 7u64))]
    RenderPass7(RenderPass7),

    #[br(pre_assert(data_type == 8u64))]
    RenderPass8(RenderPass8),

    #[br(pre_assert(data_type == 9u64))]
    RenderPass9,

    #[br(pre_assert(data_type == 10u64))]
    RenderPass10,

    #[br(pre_assert(data_type == 11u64))]
    RenderPass11,

    #[br(pre_assert(data_type == 12u64))]
    RenderPass12,

    #[br(pre_assert(data_type == 14u64))]
    RenderPass14,

    #[br(pre_assert(data_type == 17u64))]
    RenderPass17,
}

// Any fields that comes after width/height might not be part of these structs.
// It's unclear how big some of the structs are.
#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass1 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: SsbhString,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass2 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: RelPtr64<SsbhString>,
    unk4: u64,
    unk5: u64,
    unk6: u64,
    width: f32,
    height: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass3 {
    unk1: RelPtr64<SsbhString>,
    unk2: SsbhString,
    unk3: f32,
    unk4: f32,
    unk5: f32,
    unk6: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass4 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: RelPtr64<SsbhString>,
    unk4: u64,
    unk5: RelPtr64<SsbhString>,
    unk6: u64,
    unk7: RelPtr64<SsbhString>,
    unk8: u64,
    unk9: SsbhString,
    unk10: SsbhString,
    width: f32,
    height: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass5 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: RelPtr64<SsbhString>,
    unk4: u64,
    unk5: RelPtr64<SsbhString>,
    unk6: u64,
    unk7: RelPtr64<SsbhString>,
    unk8: u64,
    unk9: RelPtr64<SsbhString>,
    unk10: u64,
    unk11: SsbhString,
    unk12: u64,
    width: f32,
    height: f32,
    unk13: f32,
    unk14: f32,
    unk15: u64
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass6 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: RelPtr64<SsbhString>,
    unk4: u64,
    unk5: RelPtr64<SsbhString>,
    unk6: u64,
    unk7: RelPtr64<SsbhString>,
    unk8: u64,
    unk9: RelPtr64<SsbhString>,
    unk10: u64,
    unk11: RelPtr64<SsbhString>,
    unk12: u64,
    unk13: SsbhString,
    unk14: SsbhString,
    width: f32,
    height: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass7 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: RelPtr64<SsbhString>,
    unk4: u64,
    unk5: RelPtr64<SsbhString>,
    unk6: u64,
    unk7: RelPtr64<SsbhString>,
    unk8: u64,
    unk9: RelPtr64<SsbhString>,
    unk10: u64,
    unk11: RelPtr64<SsbhString>,
    unk12: u64,
    unk13: RelPtr64<SsbhString>,
    unk14: u64,
    unk15: SsbhString,
    unk16: u64,
    width: f32,
    height: f32,
    unk17: f32,
    unk18: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass8 {
    unk1: RelPtr64<SsbhString>,
    unk2: u64,
    unk3: RelPtr64<SsbhString>,
    unk4: u64,
    unk5: RelPtr64<SsbhString>,
    unk6: u64,
    unk7: RelPtr64<SsbhString>,
    unk8: u64,
    unk9: RelPtr64<SsbhString>,
    unk10: u64,
    unk11: RelPtr64<SsbhString>,
    unk12: u64,
    unk13: RelPtr64<SsbhString>,
    unk14: u64,
    unk15: SsbhString,
    unk16: u64,
    unk17: SsbhString,
    unk18: u64,
    width: f32,
    height: f32,
    unk19: f32,
    unk20: f32,
    unk21: u64
}

#[derive(Serialize, BinRead, Debug)]
pub struct RenderPass {
    name: SsbhString,
    data1: SsbhEnum<RenderPassData, u64>,
    data2: SsbhEnum<RenderPassData, u64>,
    data3: SsbhEnum<RenderPassData, u64>,
    padding: u64,
}

// This is based on file version 1.6.

/// ???
#[derive(Serialize, BinRead, Debug)]
pub struct Nrpd {
    major_version: u16,
    minor_version: u16,
    frame_buffer_containers: SsbhArray<FramebufferContainer>,
    state_containers: SsbhArray<StateContainer>,
    render_passes: SsbhArray<RenderPass>,
}
