use crate::{SsbhArray, SsbhByteBuffer, SsbhString};
use binread::BinRead;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[repr(u32)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u32))]
pub enum ShaderType {
    Vertex = 0,
    Geometry = 3,
    Fragment = 4,
    Compute = 5,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Shader {
    pub name: SsbhString,
    pub shader_type: ShaderType,
    pub unk3: u32,
    pub shader_binary: SsbhByteBuffer,
    pub unk4: u64,
    pub unk5: u64,
    pub binary_size: u64,
}

/// A compiled shader container.
/// Compatible with file version 1.2.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Shdr {
    pub major_version: u16,
    pub minor_version: u16,
    pub shaders: SsbhArray<Shader>,
}
