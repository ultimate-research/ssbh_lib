use crate::{SsbhArray, SsbhString, SsbhEnum};
use crate::RelPtr64;
use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};
use serde::Serialize;

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum StateDataType {
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
#[br(import(data_type: StateDataType))]
pub enum NrpdState {
    #[br(pre_assert(data_type == StateDataType::Sampler))]
    Sampler(NrpdSampler),

    #[br(pre_assert(data_type == StateDataType::RasterizerState))]
    RasterizerState(NrpdRasterizerState),

    #[br(pre_assert(data_type == StateDataType::Unk))]
    Unk(NrpdUnkData),

    #[br(pre_assert(data_type == StateDataType::BlendState))]
    BlendState(NrpdBlendState)
}

#[derive(Serialize, Debug)]
pub struct StateData {
    data: NrpdState
}

impl BinRead for StateData {
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let pos_before_read = reader.seek(SeekFrom::Current(0))?;
        let ptr = u64::read_options(reader, options, ())?;
        let data_type = StateDataType::read_options(reader, options, ())?;
        let saved_pos = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(pos_before_read + ptr))?;
        let value = NrpdState::read_options(reader, options, (data_type,))?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(StateData { data: value })
    }
}

#[derive(Serialize, BinRead, Debug)]
pub struct StateContainer {
    state: StateData,
}

#[derive(Serialize, BinRead, Debug)]
#[br(import(data_type: u64))]
pub enum RenderPassData {
    #[br(magic = 0u64)]
    String(SsbhString),

    #[br(magic = 1u64)]
    Unk1,    

    #[br(magic = 2u64)]
    Unk2,   

    #[br(magic = 3u64)]
    Unk3,

    #[br(magic = 4u64)]
    Unk4,

    #[br(magic = 5u64)]
    Unk5,

    #[br(magic = 6u64)]
    Unk6,

    #[br(magic = 7u64)]
    Unk7,

    #[br(magic = 8u64)]
    Unk8,

    #[br(magic = 9u64)]
    Unk9,

    #[br(magic = 10u64)]
    Unk10,

    #[br(magic = 11u64)]
    Unk11,

    #[br(magic = 12u64)]
    Unk12,

    #[br(magic = 14u64)]
    Unk14,

    #[br(magic = 17u64)]
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
    // data1: SsbhEnum<RenderPassData, u64>,
    // data2: SsbhEnum<RenderPassData, u64>,
    // data3: SsbhEnum<RenderPassData, u64>,
    unk1: u64,
    unk2: u64,
    unk3: u64,
    unk4: u64,
    unk5: u64,
    unk6: u64,
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
