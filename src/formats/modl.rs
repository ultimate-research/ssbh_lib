use crate::SsbhArray;
use crate::SsbhString;
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct ModlEntry {
    mesh_name: SsbhString,
    sub_index: i64,
    material_label: SsbhString,
}

/// Defines the mesh, materials, and skeleton used to render a model.
#[derive(Serialize, BinRead, Debug)]
pub struct Modl {
    major_version: u16,
    minor_version: u16,
    model_file_name: SsbhString,
    skeleton_file_name: SsbhString,
    material_file_names: SsbhArray<SsbhString>,
    unk_file_name: SsbhString,
    mesh_string: SsbhString,
    entries: SsbhArray<ModlEntry>,
}
