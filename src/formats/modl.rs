use crate::SsbhString;
use crate::{RelPtr64, SsbhArray, SsbhString8};
use binread::BinRead;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ModlEntry {
    pub mesh_name: SsbhString,
    pub sub_index: i64,
    pub material_label: SsbhString,
}

/// Defines the mesh, materials, and skeleton used to render a model.
/// Compatible with file version 1.7.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Modl {
    pub major_version: u16,
    pub minor_version: u16,
    pub model_file_name: SsbhString,
    pub skeleton_file_name: SsbhString,
    pub material_file_names: SsbhArray<SsbhString>,
    pub animation_file_name: RelPtr64<SsbhString>,
    pub mesh_string: SsbhString8,
    pub entries: SsbhArray<ModlEntry>,
}
