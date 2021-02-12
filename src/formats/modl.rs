use crate::SsbhArray;
use crate::SsbhString;
use binread::BinRead;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct ModlEntry {
    pub mesh_name: SsbhString,
    pub sub_index: i64,
    pub material_label: SsbhString,
}

/// Defines the mesh, materials, and skeleton used to render a model.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct Modl {
    pub major_version: u16,
    pub minor_version: u16,
    pub model_file_name: SsbhString,
    pub skeleton_file_name: SsbhString,
    pub material_file_names: SsbhArray<SsbhString>,
    pub unk1: u64,
    pub mesh_string: SsbhString,
    pub entries: SsbhArray<ModlEntry>,
}
