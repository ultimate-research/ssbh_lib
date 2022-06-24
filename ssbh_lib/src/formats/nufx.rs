//! The [Nufx] format stores data about the shader programs used for rendering.
//! These files typically use the ".nufxlb" suffix like "nuc2effectlibrary.nufxlb".
//! [Nufx] files reference required attributes from [Mesh](crate::formats::mesh::Mesh) files and required parameters from [Matl](crate::formats::matl::Matl) files.

use crate::{SsbhArray, SsbhString, SsbhString8, Version};
use binrw::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// A required vertex attribute.
/// The [name](#structfield.name) and [attribute_name](#structfield.attribute_name) should match the values for a corresponding [AttributeV10][crate::formats::mesh::AttributeV10].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct VertexAttribute {
    pub name: SsbhString,
    pub attribute_name: SsbhString,
}

/// A required material parameter. The [param_id](#structfield.param_id) and [parameter_name](#structfield.parameter_name) match one of the variants in [ParamId](crate::formats::matl::ParamId).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 8)]
pub struct MaterialParameter {
    // TODO: These values are identical to the matl ones but there are some missing variants.
    pub param_id: u64,
    #[br(pad_after = 8)]
    pub parameter_name: SsbhString8,
}

/// Describes the shaders used for each of the stages in the rendering pipeline.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ShaderStages {
    pub vertex_shader: SsbhString,
    pub unk_shader1: SsbhString, // The missing stages could be tesselation, etc.
    pub unk_shader2: SsbhString,
    pub geometry_shader: SsbhString,
    pub pixel_shader: SsbhString,
    pub compute_shader: SsbhString,
}

/// Describes the name and associated information for a set of compiled shaders linked into a program.
/// Each [ShaderProgramV0] has a corresponding shader program object in the underlying rendering API such as OpenGL, Vulkan, etc.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ShaderProgramV0 {
    /// The unique identifier of the shader program, including its [render_pass](#structfield.render_pass).
    pub name: SsbhString,
    /// Programs are grouped into passes to determine the render order.
    /// Possible values for Smash Ultimate are "nu::Final", "nu::Opaque", "nu::Sort", "nu::Near", and "nu::Far".
    pub render_pass: SsbhString,
    /// The shaders to compile and link for this program.
    pub shaders: ShaderStages,
    /// The required parameters from the [Matl](crate::formats::matl::Matl) materials.
    pub material_parameters: SsbhArray<MaterialParameter>,
}

/// Describes the name and associated information for a set of compiled shaders linked into a program.
/// Each [ShaderProgramV1] has a corresponding shader program object in the underlying rendering API such as OpenGL, Vulkan, etc.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ShaderProgramV1 {
    /// The unique identifier of the shader program, including its [render_pass](#structfield.render_pass).
    pub name: SsbhString,
    /// Programs are grouped into passes to determine the render order.
    /// Possible values for Smash Ultimate are "nu::Final", "nu::Opaque", "nu::Sort", "nu::Near", and "nu::Far".
    pub render_pass: SsbhString,
    /// The shaders to compile and link for this program.
    pub shaders: ShaderStages,
    /// The required attributes from the [MeshObject](crate::formats::mesh::MeshObject) such as "Position0".
    pub vertex_attributes: SsbhArray<VertexAttribute>,
    /// The required parameters from the [MatlEntry](crate::formats::matl::MatlEntryV16) such as "RasterizerState0".
    pub material_parameters: SsbhArray<MaterialParameter>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem {
    pub name: SsbhString,
    pub unk1: SsbhArray<SsbhString>,
}

/// A shader effects library that describes shader programs and their associated inputs.
/// Compatible with file version 1.0 and 1.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Nufx {
    // TODO: Add support for named enum fields to SsbhWrite.
    #[br(pre_assert(major_version == 1 &&  minor_version == 0))]
    V0(NufxV0),

    #[br(pre_assert(major_version == 1 &&  minor_version == 1))]
    V1(NufxV1),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct NufxV0 {
    pub programs: SsbhArray<ShaderProgramV0>,
    pub unk_string_list: SsbhArray<UnkItem>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct NufxV1 {
    pub programs: SsbhArray<ShaderProgramV1>,
    pub unk_string_list: SsbhArray<UnkItem>,
}

impl Version for Nufx {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Nufx::V0(_) => (1, 0),
            Nufx::V1(_) => (1, 1),
        }
    }
}
