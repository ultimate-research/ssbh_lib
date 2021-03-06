//! The [Mesh] format stores the geometric data used for model rendering.
//! These files typically use the ".numshb" suffix like "model.numshb".
//! This includes attribute data such as position and normals, vertex skinning, and bounding volume information.
//! [Mesh] files are linked with [Skel](crate::formats::skel::Skel) and [Matl](crate::formats::matl::Matl) files using a [Modl](crate::formats::modl::Modl) file.

use crate::Matrix3x3;
use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use crate::Vector3;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};
use ssbh_write_derive::SsbhWrite;

use binread::BinRead;

/// The vertex buffers and associated geometric data for a mesh.
/// Compatible with file version 1.8, 1.9, and 1.10.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 16, align_after = 8)]
pub struct Mesh {
    pub major_version: u16,
    pub minor_version: u16,
    pub model_name: SsbhString,
    pub bounding_info: BoundingInfo,
    pub unk1: u32, // always 0
    #[br(args(major_version, minor_version))]
    pub objects: SsbhArray<MeshObject>,
    pub buffer_sizes: SsbhArray<u32>,
    pub polygon_index_size: u64,
    /// The shared buffers for vertex attribute data such as positions and normals for all the [MeshObject] in [objects](#structfield.objects).
    pub vertex_buffers: SsbhArray<SsbhByteBuffer>,
    /// The shared buffer for vertex indices for all the [MeshObject] in [objects](#structfield.objects).
    pub index_buffer: SsbhByteBuffer,
    /// A collection of vertex skinning data stored as a one to many mapping from [MeshObject] to [SkelBoneEntry](crate::formats::skel::SkelBoneEntry).
    /// The collection should be sorted in ascending order based on [mesh_object_name](struct.MeshRiggingGroup.html#structfield.mesh_object_name) and
    /// [mesh_object_sub_index](struct.MeshRiggingGroup.html#structfield.mesh_object_sub_index). This is likely to facilitate an efficient binary search by [MeshObject].
    #[br(args(major_version, minor_version))]
    pub rigging_buffers: SsbhArray<MeshRiggingGroup>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshAttributeV8 {
    pub usage: AttributeUsageV8,
    pub data_type: AttributeDataTypeV8,
    pub buffer_index: u32,
    pub buffer_offset: u32,
    /// The index for multiple attributes of the same usage starting from 0.
    pub sub_index: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshAttributeV9 {
    pub usage: AttributeUsageV9,
    pub data_type: AttributeDataTypeV8,
    pub buffer_index: u32,
    pub buffer_offset: u32,
    /// The index for multiple attributes of the same usage starting from 0.
    pub sub_index: u64,
    pub name: SsbhString,
    pub attribute_names: SsbhArray<SsbhString>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshAttributeV10 {
    pub usage: AttributeUsageV9,
    pub data_type: AttributeDataTypeV10,
    pub buffer_index: u32,
    pub buffer_offset: u32,
    /// The index for multiple attributes of the same usage starting from 0.
    pub sub_index: u64,
    pub name: SsbhString,
    pub attribute_names: SsbhArray<SsbhString>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy)]
pub struct BoundingInfo {
    pub bounding_sphere: BoundingSphere,
    pub bounding_volume: BoundingVolume,
    pub oriented_bounding_box: OrientedBoundingBox,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Vector3,
    pub radius: f32,
}

/// A region of 3d space that contains a set of points.
/// This is equivalent to an axis-aligned bounding box (abbreviated AABB) for the XYZ axes.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy)]
pub struct BoundingVolume {
    pub min: Vector3,
    pub max: Vector3,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy)]
pub struct OrientedBoundingBox {
    pub center: Vector3,
    pub transform: Matrix3x3,
    pub size: Vector3,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Copy, Clone)]
#[ssbhwrite(pad_after = 6)]
pub struct RiggingFlags {
    pub max_influences: u8,
    #[br(pad_after = 6)]
    pub unk1: u8,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub struct MeshBoneBuffer {
    pub bone_name: SsbhString,
    #[br(args(major_version, minor_version))]
    pub data: VertexWeights,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum VertexWeights {
    #[br(pre_assert(major_version == 1 &&  minor_version == 8))]
    VertexWeightsV8(SsbhArray<VertexWeightV8>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 9))]
    VertexWeightsV9(SsbhArray<VertexWeightV8>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 10))]
    VertexWeightsV10(SsbhByteBuffer),
}

/// Vertex skinning data for the vertices for the [MeshObject]
/// determined by [mesh_object_name](#structfield.mesh_object_name) and [mesh_object_sub_index](#structfield.mesh_object_sub_index).
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub struct MeshRiggingGroup {
    pub mesh_object_name: SsbhString,
    pub mesh_object_sub_index: u64,
    pub flags: RiggingFlags,
    #[br(args(major_version, minor_version))]
    pub buffers: SsbhArray<MeshBoneBuffer>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum MeshAttributes {
    #[br(pre_assert(major_version == 1 &&  minor_version == 8))]
    AttributesV8(SsbhArray<MeshAttributeV8>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 9))]
    AttributesV9(SsbhArray<MeshAttributeV9>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 10))]
    AttributesV10(SsbhArray<MeshAttributeV10>),
}

/// The element type for the vertex skin weights stored in version 1.10 byte buffers.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct VertexWeightV8 {
    pub vertex_index: u32,
    pub vertex_weight: f32,
}

/// The element type for the vertex skin weights stored in the [SsbhByteBuffer] for [VertexWeights::VertexWeightsV10].
#[derive(BinRead, Debug)]
pub struct VertexWeightV10 {
    pub vertex_index: u16,
    pub vertex_weight: f32,
}

/// A vertex collection identified by its [name](#structfield.name) and [sub_index](#structfield.sub_index).
/// In addition to organizing the model into logical components, material and rigging data are assigned per [MeshObject].
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub struct MeshObject {
    /// The name of the [MeshObject] such as `"c00BodyShape"`.
    /// Objects with the same name should have a unique [sub_index](#structfield.sub_index).
    pub name: SsbhString,
    /// The index for multiple objects of the same name starting from 0.
    pub sub_index: u64,
    /// If [rigging_type](#structfield.rigging_type) is set to [RiggingType::SingleBound],
    /// the object's position is determined by the [SkelBoneEntry](crate::formats::skel::SkelBoneEntry) with matching name.
    /// Otherwise, [parent_bone_name](#structfield.parent_bone_name) is set to an empty string.
    pub parent_bone_name: SsbhString,
    /// The number of elements for this object in the [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers)
    /// Each attribute in [attributes](#structfield.sub_index) should have the same number of elements.
    pub vertex_count: u32,
    /// The number of elements for this object in the [index_buffer](struct.Mesh.html#structfield.index_buffer).
    /// This determines the actual number of vertices rendered and will typically be larger than [vertex_count](#structfield.vertex_count).
    pub vertex_index_count: u32,
    pub unk2: u32, // number of indices per face (always 3)?
    pub vertex_buffer0_offset: u32,
    pub vertex_buffer1_offset: u32,
    pub final_buffer_offset: u32,
    pub buffer_index: u32, // always 0?
    pub stride0: u32,
    pub stride1: u32,
    pub unk6: u32, // set to 32 for version 1.8 and 0 otherwise
    pub unk7: u32, // always 0
    /// The byte offset for the start of this object's vertex indices in the [index_buffer](struct.Mesh.html#structfield.index_buffer).
    /// The number of bytes to read is determined by [draw_element_type](#structfield.draw_element_type) and [vertex_index_count](#structfield.vertex_index_count).
    pub index_buffer_offset: u32,
    pub unk8: u32, // always 4
    /// The data type for the vertex indices stored in [index_buffer](struct.Mesh.html#structfield.index_buffer).
    pub draw_element_type: DrawElementType,
    /// Determines how vertex transformations are influenced by bones.
    pub rigging_type: RiggingType,
    pub unk11: i32, // unk index
    pub unk12: u32, // unk flags (0,1,256,257)
    pub bounding_info: BoundingInfo,
    /// Describes how the vertex attribute data for this object is stored in the [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers).
    #[br(args(major_version, minor_version))]
    pub attributes: MeshAttributes,
}

/// Possible values for [draw_element_type](struct.MeshObject.html#structfield.draw_element_type).
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq)]
#[br(repr(u32))]
pub enum DrawElementType {
    /// Vertex indices stored as [u16].
    UnsignedShort = 0,
    /// Vertex indices stored as [u32].
    UnsignedInt = 1,
}

/// Possible values for [rigging_type](struct.MeshObject.html#structfield.rigging_type).
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq)]
#[br(repr(u32))]
pub enum RiggingType {
    /// Vertices are parented to a parent bone and inherit the parent's transforms.
    SingleBound = 0,
    /// Vertices are influenced by one or more bones based on assigned vertex weights.
    /// Weight values are grouped by bone in the [rigging_buffers](struct.Mesh.html#structfield.rigging_buffers).
    Weighted = 1,
}

/// The data type and component count for the attribute's data.
/// This determines the stride and offset between attributes.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq)]
#[br(repr(u32))]
pub enum AttributeDataTypeV10 {
    /// 3 component (xyz or rgb) vector of [f32].
    Float3 = 0,
    /// 4 component (rgba) vector of [u8].
    Byte4 = 2,
    /// 4 component (xyzw or rgba) vector of [f32].
    Float4 = 4,
    /// 4 component (xyzw or rgba) vector of [f16](half::f16).
    HalfFloat4 = 5,
    /// 2 component (xy or uv) vector of [f32].
    Float2 = 7,
    /// 2 component (xy or uv) vector of [f16](half::f16).
    HalfFloat2 = 8,
}

/// The data type and component count for the attribute's data.
/// This determines the stride and offset between attributes.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq)]
#[br(repr(u32))]
pub enum AttributeDataTypeV8 {
    /// 3 component (xyz or rgb) vector of [f32].
    Float3 = 820,
    /// 4 component (rgba) vector of [f32].
    Float4 = 1076,
    /// 4 component (xyzw or rgba) vector of [f16](half::f16).
    HalfFloat4 = 1077,
    /// 2 component (xy or uv) vector of [f32].
    Float2 = 1079,
    /// 4 component (rgba) vector of [u8].
    Byte4 = 1024,
}

/// Determines how the attribute data will be used by the shaders for [Mesh] version 1.9 and 1.10.
/// Attributes with an identical usage should each have a unique [sub_index](struct.MeshAttributeV10.html#structfield.sub_index).
/// Smash Ultimate also considers [name](struct.MeshAttributeV10.html#structfield.name) and
/// [attribute_names](struct.MeshAttributeV10.html#structfield.attribute_names) when determing the usage in some cases.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq)]
#[br(repr(u32))]
pub enum AttributeUsageV9 {
    Position = 0,
    Normal = 1,
    Binormal = 2,
    Tangent = 3,
    TextureCoordinate = 4,
    ColorSet = 5,
}

/// Determines how the attribute data will be used by the shaders for [Mesh] version 1.8.
/// Attributes with an identical usage should each have a unique [sub_index](struct.MeshAttributeV8.html#structfield.sub_index).
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq)]
#[br(repr(u32))]
pub enum AttributeUsageV8 {
    Position = 0,
    Normal = 1,
    Tangent = 3,
    TextureCoordinate = 4,
    ColorSet = 8,
}
