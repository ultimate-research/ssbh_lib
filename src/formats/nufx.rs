use crate::RelPtr64;
use crate::{SsbhArray, SsbhString};
use binread::BinRead;
use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct VertexAttribute {
    pub name: SsbhString,
    pub attribute_name: SsbhString,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MaterialParameter {
    pub param_id: u64,
    #[br(pad_after = 8)]
    pub parameter_name: SsbhString,
}

/// Describes the program's name, the shaders used for each shader stage, and its inputs. 
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct ShaderProgram {
    pub name: SsbhString,
    pub render_pass: SsbhString,
    pub vertex_shader: SsbhString,

    // This missing stages could be compute, tesselation, etc. 
    pub unk_shader1: SsbhString,
    pub unk_shader2: SsbhString,
    pub unk_shader3: SsbhString,
    
    pub pixel_shader: SsbhString,
    pub unk_shader4: SsbhString,
    pub vertex_attributes: SsbhArray<VertexAttribute>,
    pub material_parameters: SsbhArray<MaterialParameter>,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct UnkItem {
    pub text: SsbhString,
    pub unk1: RelPtr64<SsbhString>,
    pub unk2: u64,
}

/// A shader effects library that describes shader programs and their associated inputs.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct Nufx {
    pub major_version: u16,
    pub minor_version: u16,
    pub programs: SsbhArray<ShaderProgram>, // TODO: This only works for version 1.1
    pub unk_string_list: SsbhArray<UnkItem>, // TODO: This only works for version 1.1
}
