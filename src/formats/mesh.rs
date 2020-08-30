use crate::Matrix3x3;
use crate::SsbhArray;
use crate::SsbhString;
use crate::Vector3;
use serde::Serialize;

use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};

#[derive(Serialize, BinRead, Debug, Copy, Clone, PartialEq)]
enum DrawElementType {
    #[br(magic = 0u32)]
    UnsignedShort,
    #[br(magic = 1u32)]
    UnsignedInt,
}

#[derive(Serialize, BinRead, Debug, Copy, Clone, PartialEq)]
enum RiggingType {
    #[br(magic = 0x0u32)]
    SingleBound,
    #[br(magic = 0x1u32)]
    Regular,
}

#[derive(Serialize, BinRead, Debug, Copy, Clone, PartialEq)]
enum AttributeDataType {
    #[br(magic = 0u32)]
    Float,
    #[br(magic = 2u32)]
    Byte,
    #[br(magic = 5u32)]
    HalfFloat,
    #[br(magic = 8u32)]
    HalfFloat2,
}

#[derive(Serialize, BinRead, Debug)]
struct MeshAttribute {
    index: i32,
    data_type: AttributeDataType,
    buffer_index: i32,
    buffer_offset: u32,
    unk4: u32,
    unk5: u32,
    name: SsbhString,
    attribute_names: SsbhArray<SsbhString>,
}

#[derive(Serialize, BinRead, Debug)]
struct MeshBuffer {
    data: SsbhArray<u8>,
}

#[derive(Serialize, BinRead, Debug)]
struct MeshInfluence {
    vertex_index: i16,
    vertex_weight: f32,
}

#[derive(Serialize, BinRead, Debug)]
struct MeshBoneBuffer {
    bone_name: SsbhString,
    // TODO: Map this to MeshInfluences
    data: SsbhArray<u8>,
}

#[derive(Serialize, BinRead, Debug)]
struct MeshRiggingGroup {
    mesh_name: SsbhString,
    mesh_sub_index: i64,
    flags: u64,
    buffers: SsbhArray<MeshBoneBuffer>,
}

#[derive(Serialize, BinRead, Debug)]
struct MeshObject {
    name: SsbhString,
    sub_index: i64,
    parent_bone_name: SsbhString,
    vertex_count: u32,
    index_count: u32,
    unk2: u32,
    vertex_offset: u32,
    vertex_offset2: u32,
    final_buffer_offset: u32,
    buffer_index: i32,
    stride: u32,
    stride2: u32,
    unk6: u32,
    unk7: u32,
    element_offset: u32,
    unk8: i32,
    draw_element_type: DrawElementType,
    rigging_type: RiggingType,
    unk11: u32,
    unk12: u32,
    bounding_sphere_center: Vector3,
    bounding_sphere_radius: f32,
    bounding_box_min: Vector3,
    bounding_box_max: Vector3,
    oriented_bounding_box_center: Vector3,
    oriented_bounding_box_transform: Matrix3x3,
    oriented_bounding_box_size: Vector3,
    attributes: SsbhArray<MeshAttribute>,
}

#[derive(Serialize, BinRead, Debug)]
#[br(magic = b"HBSS")]
pub struct Mesh {
    #[br(magic = b"HSEM", align_before = 0x10)]
    magic: [char; 4],
    major_version: u16,
    minor_version: u16,
    model_name: SsbhString,
    bounding_sphere_center: Vector3,
    bounding_sphere_radius: f32,
    bounding_box_min: Vector3,
    bounding_box_max: Vector3,
    oriented_bounding_box_center: Vector3,
    oriented_bounding_box_transform: Matrix3x3,
    oriented_bounding_box_size: Vector3,
    unk1: f32,
    objects: SsbhArray<MeshObject>,
    buffer_sizes: SsbhArray<u32>,
    polygon_index_size: u64,
    vertex_buffers: SsbhArray<MeshBuffer>,
    polygon_buffer: SsbhArray<u8>,
    rigging_buffer: SsbhArray<MeshRiggingGroup>,
    unknown_offset: u64,
    unknown_size: u64,
}
