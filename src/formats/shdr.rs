use crate::{SsbhArray, SsbhByteBuffer, SsbhString};
use binread::BinRead;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[repr(u32)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
pub enum ShaderType {
    #[br(magic = 0u32)]
    Vertex = 0,

    #[br(magic = 3u32)]
    Geometry = 3,

    #[br(magic = 4u32)]
    Fragment = 4,

    #[br(magic = 5u32)]
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
