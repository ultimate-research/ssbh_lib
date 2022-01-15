use crate::{SsbhArray, SsbhByteBuffer, SsbhString, Version};
use binread::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

#[repr(u32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum ShaderType {
    Vertex = 0,
    Geometry = 3,
    Fragment = 4,
    Compute = 5,
}

// TODO: The binary seems to contain names for uniforms.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Shader {
    pub name: SsbhString,
    pub shader_type: ShaderType,
    pub unk3: u32,
    pub shader_binary: SsbhByteBuffer, // TODO: Additional parsing for this?
    pub binary_size: u64,
    pub unk4: u64,
    pub unk5: u64,
}

/// A compiled shader container.
/// Compatible with file version 1.2.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Shdr {
    V12 { shaders: SsbhArray<Shader> },
}

impl Version for Shdr {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Shdr::V12 { .. } => (1, 2),
        }
    }
}
