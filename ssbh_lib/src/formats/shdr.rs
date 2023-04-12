use crate::{SsbhArray, SsbhByteBuffer, SsbhString, Version};
use binrw::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// A list of compiled shaders and their associated metadata.
/// Compatible with file version 1.2.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Shdr {
    #[br(pre_assert(major_version == 1 && minor_version == 2))]
    V12 { shaders: SsbhArray<Shader> },
}

impl Version for Shdr {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Shdr::V12 { .. } => (1, 2),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[ssbhwrite(pad_after = 16)]
pub struct Shader {
    pub name: SsbhString,
    pub shader_stage: ShaderStage,
    pub unk3: u32, // always 2
    /// The compiled shader code as well as metadata describing
    /// uniforms, buffers, textures, and input/output attributes.
    pub shader_binary: SsbhByteBuffer,
    #[br(pad_after = 16)]
    pub binary_size: u64,
}

#[repr(u32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum ShaderStage {
    Vertex = 0,
    Geometry = 3,
    Fragment = 4,
    Compute = 5,
}
