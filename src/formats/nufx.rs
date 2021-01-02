use crate::{SsbhArray, SsbhString};
use crate::RelPtr64;
use binread::BinRead;
use serde::Serialize;
use binread::NullString;

#[derive(Serialize, BinRead, Debug)]
pub struct UnkData {
    unk1: SsbhString,
}

#[derive(Serialize, BinRead, Debug)]
pub struct UnkItem {
    name: SsbhString,
    render_pass: SsbhString,
    vertex_shader: SsbhString,
    unk1: SsbhString,
    unk2: SsbhString,
    unk3: SsbhString,
    pixel_shader: SsbhString,
    unk4: SsbhString,
    unk5: RelPtr64<UnkData>,
    unk6: u64,
    unk7: u64,
    unk8: u64,
}

/// 
#[derive(Serialize, BinRead, Debug)]
pub struct Nufx {
    major_version: u16,
    minor_version: u16,
    unk1: u64,
    unk2: u64,
    unk3: u64,
    unk4: u64,
    #[br(count = 32)]
    unk_list: Vec<UnkItem>
}
