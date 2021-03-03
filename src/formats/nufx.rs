use crate::{SsbhArray, SsbhString, SsbhString8};
use binread::BinRead;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct VertexAttribute {
    pub name: SsbhString,
    pub attribute_name: SsbhString,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MaterialParameter {
    pub param_id: u64,
    pub parameter_name: SsbhString8,
    #[cfg_attr(feature = "derive_serde", serde(skip))]
    pub padding: u64,
}

/// Describes the shader used for the compute shader, fragment shader, etc.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ShaderStages {
    pub vertex_shader: SsbhString,
    pub unk_shader1: SsbhString, // The missing stages could be tesselation, etc.
    pub unk_shader2: SsbhString,
    pub geometry_shader: SsbhString,
    pub pixel_shader: SsbhString,
    pub compute_shader: SsbhString,
}

/// Describes the program's name, the shaders used for each shader stage, and its inputs.
/// Version 1.0 does not contain vertex attributes.
#[br(import(major_version: u16, minor_version: u16))]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ShaderProgram {
    pub name: SsbhString8,
    pub render_pass: SsbhString,
    pub shaders: ShaderStages,

    // TODO: Find a cleaner way to handle serializing.
    #[cfg_attr(
        feature = "derive_serde",
        serde(skip_serializing_if = "Option::is_none")
    )]
    #[cfg_attr(feature = "derive_serde", serde(default))]
    #[br(if(major_version == 1 && minor_version == 1))]
    pub vertex_attributes: Option<SsbhArray<VertexAttribute>>,

    pub material_parameters: SsbhArray<MaterialParameter>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem {
    pub name: SsbhString,
    pub unk1: SsbhArray<SsbhString>,
}

/// A shader effects library that describes shader programs and their associated inputs.
/// Compatible with file version 1.0 and 1.1.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Nufx {
    pub major_version: u16,
    pub minor_version: u16,
    #[br(args(major_version, minor_version))]
    pub programs: SsbhArray<ShaderProgram>,
    pub unk_string_list: SsbhArray<UnkItem>,
}
