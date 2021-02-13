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
    pub padding: u64,
}

/// Describes the shader used for the compute shader, fragment shader, etc.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct ShaderStages {
    pub vertex_shader: SsbhString,
    pub unk_shader1: SsbhString, // The missing stages could be tesselation, etc.
    pub unk_shader2: SsbhString,
    pub geometry_shader: SsbhString,
    pub pixel_shader: SsbhString,
    pub compute_shader: SsbhString,
}

/// Describes the program's name, the shaders used for each shader stage, and its inputs.
/// Identical to `ShaderProgramV0` but adds vertex attributes.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct ShaderProgramV1 {
    pub name: SsbhString,
    pub render_pass: SsbhString,
    pub shaders: ShaderStages,
    pub vertex_attributes: SsbhArray<VertexAttribute>,
    pub material_parameters: SsbhArray<MaterialParameter>,
}

/// Describes the program's name, the shaders used for each shader stage, and its inputs.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct ShaderProgramV0 {
    pub name: SsbhString,
    pub render_pass: SsbhString,
    pub shaders: ShaderStages,
    pub material_parameters: SsbhArray<MaterialParameter>,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct UnkItem {
    pub name: SsbhString,
    pub unk1: SsbhArray<SsbhString>,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum ShaderPrograms {
    #[br(pre_assert(major_version == 1 &&  minor_version == 0))]
    ProgramsV0(SsbhArray<ShaderProgramV0>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 1))]
    ProgramsV1(SsbhArray<ShaderProgramV1>),
}

/// A shader effects library that describes shader programs and their associated inputs.
/// Compatible with file version 1.0 and 1.1.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct Nufx {
    pub major_version: u16,
    pub minor_version: u16,
    #[br(args(major_version, minor_version))]
    pub programs: ShaderPrograms,
    pub unk_string_list: SsbhArray<UnkItem>,
}
