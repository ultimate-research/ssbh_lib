use crate::Matrix3x3;
use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use crate::Vector3;
use serde::{Deserialize, Serialize};

use binread::BinRead;

#[derive(Serialize, Deserialize, BinRead, Debug, Copy, Clone, PartialEq)]
pub enum DrawElementType {
    #[br(magic = 0u32)]
    UnsignedShort,
    #[br(magic = 1u32)]
    UnsignedInt,
}

#[derive(Serialize, Deserialize, BinRead, Debug, Copy, Clone, PartialEq)]
pub enum RiggingType {
    #[br(magic = 0x0u32)]
    SingleBound,
    #[br(magic = 0x1u32)]
    Regular,
}

#[derive(Serialize, Deserialize, BinRead, Debug, Copy, Clone, PartialEq)]
pub enum AttributeDataType {
    #[br(magic = 0u32)]
    Float,
    #[br(magic = 2u32)]
    Byte,
    #[br(magic = 5u32)]
    HalfFloat,
    #[br(magic = 8u32)]
    HalfFloat2,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MeshAttribute {
    pub index: i32,
    pub data_type: AttributeDataType,
    pub buffer_index: i32,
    pub buffer_offset: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub name: SsbhString,
    pub attribute_names: SsbhArray<SsbhString>,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MeshInfluence {
    vertex_index: i16,
    vertex_weight: f32,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MeshBoneBuffer {
    bone_name: SsbhString,
    // TODO: Map this to MeshInfluences
    data: SsbhByteBuffer,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MeshRiggingGroup {
    mesh_name: SsbhString,
    mesh_sub_index: i64,
    flags: u64,
    buffers: SsbhArray<MeshBoneBuffer>,
}

#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct MeshObject {
    pub name: SsbhString,
    pub sub_index: i64,
    pub parent_bone_name: SsbhString,
    pub vertex_count: u32,
    pub index_count: u32,
    pub unk2: u32,
    pub vertex_offset: u32,
    pub vertex_offset2: u32,
    pub final_buffer_offset: u32,
    pub buffer_index: i32,
    pub stride: u32,
    pub stride2: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub element_offset: u32,
    pub unk8: i32,
    pub draw_element_type: DrawElementType,
    pub rigging_type: RiggingType,
    pub unk11: u32,
    pub unk12: u32,
    pub bounding_sphere_center: Vector3,
    pub bounding_sphere_radius: f32,
    pub bounding_box_min: Vector3,
    pub bounding_box_max: Vector3,
    pub oriented_bounding_box_center: Vector3,
    pub oriented_bounding_box_transform: Matrix3x3,
    pub oriented_bounding_box_size: Vector3,
    pub attributes: SsbhArray<MeshAttribute>,
}

/// The vertex buffers and associated geometric data for a mesh.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub struct Mesh {
    pub major_version: u16,
    pub minor_version: u16,
    pub model_name: SsbhString,
    pub bounding_sphere_center: Vector3,
    pub bounding_sphere_radius: f32,
    pub bounding_box_min: Vector3,
    pub bounding_box_max: Vector3,
    pub oriented_bounding_box_center: Vector3,
    pub oriented_bounding_box_transform: Matrix3x3,
    pub oriented_bounding_box_size: Vector3,
    pub unk1: f32,
    pub objects: SsbhArray<MeshObject>,
    pub buffer_sizes: SsbhArray<u32>,
    pub polygon_index_size: u64,
    pub vertex_buffers: SsbhArray<SsbhByteBuffer>,
    pub polygon_buffer: SsbhByteBuffer,
    pub rigging_buffer: SsbhArray<MeshRiggingGroup>,
    pub unknown_offset: u64,
    pub unknown_size: u64,
}
