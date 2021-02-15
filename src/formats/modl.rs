use crate::SsbhArray;
use crate::SsbhString;
use binread::BinRead;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct ModlEntry {
    pub mesh_name: SsbhString,
    pub sub_index: i64,
    pub material_label: SsbhString,
}

/// Defines the mesh, materials, and skeleton used to render a model.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
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
