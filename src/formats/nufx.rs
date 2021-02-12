use crate::{SsbhArray, SsbhString};
use binread::BinRead;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct VertexAttribute {
    pub name: SsbhString,
    pub attribute_name: SsbhString,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MaterialParameter {
    pub param_id: u64,
    pub parameter_name: SsbhString,
    #[serde(skip)]
    pub padding: u64
}

/// Describes the program's name, the shaders used for each shader stage, and its inputs.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct ShaderProgram {
    pub name: SsbhString,
    pub render_pass: SsbhString,
    pub vertex_shader: SsbhString,
    pub unk_shader1: SsbhString, // The missing stages could be compute, tesselation, etc.
    pub unk_shader2: SsbhString,
    pub unk_shader3: SsbhString,
    pub pixel_shader: SsbhString,
    pub unk_shader4: SsbhString,
    pub vertex_attributes: SsbhArray<VertexAttribute>,
    pub material_parameters: SsbhArray<MaterialParameter>,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct UnkItem {
    pub name: SsbhString,
    pub unk1: SsbhArray<SsbhString>,
}

/// A shader effects library that describes shader programs and their associated inputs.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct Nufx {
    pub major_version: u16,
    pub minor_version: u16,
    pub programs: SsbhArray<ShaderProgram>, // TODO: This only works for version 1.1
    pub unk_string_list: SsbhArray<UnkItem>, // TODO: This only works for version 1.1
}
