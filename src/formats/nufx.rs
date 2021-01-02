use crate::{SsbhArray, SsbhString};
use crate::RelPtr64;
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct VertexAttribute {
    name: SsbhString,
    attribute_name: SsbhString
}

#[derive(Serialize, BinRead, Debug)]
pub struct MaterialParameter {
    param_id: u64,
    parameter_name: SsbhString,
    padding: u64
}

#[derive(Serialize, BinRead, Debug)]
pub struct ShaderProgram {
    name: SsbhString,
    render_pass: SsbhString,
    vertex_shader: SsbhString,
    unk_shader1: SsbhString,
    unk_shader2: SsbhString,
    unk_shader3: SsbhString,
    pixel_shader: SsbhString,
    unk_shader4: SsbhString,
    vertex_attributes: SsbhArray<VertexAttribute>,
    material_parameters: SsbhArray<MaterialParameter>,
}

#[derive(Serialize, BinRead, Debug)]
pub struct UnkItem {
    text: SsbhString,
    unk1: RelPtr64<u64>,
    unk2: u64
}

/// A shader effects library that describes shader programs and their associated inputs.
#[derive(Serialize, BinRead, Debug)]
pub struct Nufx {
    major_version: u16,
    minor_version: u16,
    programs: SsbhArray<ShaderProgram>, // TODO: This only works for version 1.1
    unk_string_list: SsbhArray<UnkItem>, // TODO: This only works for version 1.1
}
