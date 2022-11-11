//! The [Mesh] format stores the geometric data used for model rendering.
//! These files typically use the ".numshb" suffix like "model.numshb".
//!
//! This includes attribute data such as position and normals, vertex skinning, and bounding volume information.
//! [Mesh] files are linked with [Skel](crate::formats::skel::Skel) and [Matl](crate::formats::matl::Matl) files using a [Modl](crate::formats::modl::Modl) file.

use crate::Matrix3x3;
use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use crate::Vector3;
use crate::Version;

use binrw::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// The vertex buffers and associated geometric data for a mesh.
/// Compatible with file version 1.8, 1.9, and 1.10.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
#[ssbhwrite(pad_after = 16, align_after = 8)]
pub enum Mesh {
    /// Mesh version 1.8.
    #[br(pre_assert(major_version == 1 &&  minor_version == 8))]
    V8(MeshInner<AttributeV8, SsbhArray<VertexWeightV8>>),

    /// Mesh version 1.9.
    /// Adds [AttributeUsageV9::Binormal] and names to vertex attributes.
    #[br(pre_assert(major_version == 1 &&  minor_version == 9))]
    V9(MeshInner<AttributeV9, SsbhArray<VertexWeightV8>>),

    /// Mesh version 1.10.
    /// Adds [AttributeDataTypeV10::HalfFloat2].
    /// Vertex indices for skin weights use [u16] instead of [u32].
    #[br(pre_assert(major_version == 1 &&  minor_version == 10))]
    V10(MeshInner<AttributeV10, SsbhByteBuffer>),
}

impl Version for Mesh {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Mesh::V8(_) => (1, 8),
            Mesh::V9(_) => (1, 9),
            Mesh::V10(_) => (1, 10),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshInner<A: BinRead<Args = ()> + SsbhWrite, W1: BinRead<Args = ()> + SsbhWrite> {
    pub model_name: SsbhString,
    pub bounding_info: BoundingInfo,
    pub unk1: u32, // always 0
    pub objects: SsbhArray<MeshObject<A>>,
    pub buffer_sizes: SsbhArray<u32>,
    pub polygon_index_size: u64,
    /// The shared buffers for vertex attribute data such as positions and normals for all the [MeshObject] in [objects](#structfield.objects).
    /// Each [MeshObject] only defines attribute access information for up to 4 buffers.
    pub vertex_buffers: SsbhArray<SsbhByteBuffer>,
    /// The shared buffer for vertex indices for all the [MeshObject] in [objects](#structfield.objects).
    pub index_buffer: SsbhByteBuffer,
    /// A collection of vertex skinning data stored as a one to many mapping from [MeshObject] to [SkelBoneEntry](crate::formats::skel::SkelBoneEntry).
    /// The collection should be sorted in ascending order based on [mesh_object_name](struct.RiggingGroup.html#structfield.mesh_object_name) and
    /// [mesh_object_subindex](struct.RiggingGroup.html#structfield.mesh_object_subindex). This is likely to facilitate an efficient binary search by [MeshObject].
    pub rigging_buffers: SsbhArray<RiggingGroup<W1>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq, Eq)]
pub struct AttributeV8 {
    pub usage: AttributeUsageV8,
    pub data_type: AttributeDataTypeV8,
    pub buffer_index: u32,
    pub buffer_offset: u32,
    /// The index for multiple attributes of the same usage starting from 0.
    pub subindex: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq, Eq)]
pub struct AttributeV9 {
    pub usage: AttributeUsageV9,
    pub data_type: AttributeDataTypeV8,
    pub buffer_index: u32,
    pub buffer_offset: u32,
    /// The index for multiple attributes of the same usage starting from 0.
    pub subindex: u64,
    pub name: SsbhString,
    pub attribute_names: SsbhArray<SsbhString>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq, Eq)]
pub struct AttributeV10 {
    pub usage: AttributeUsageV9,
    pub data_type: AttributeDataTypeV10,
    pub buffer_index: u32,
    pub buffer_offset: u32,
    /// The index for multiple attributes of the same usage starting from 0.
    pub subindex: u64,
    pub name: SsbhString,
    pub attribute_names: SsbhArray<SsbhString>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, Default)]
pub struct BoundingInfo {
    pub bounding_sphere: BoundingSphere,
    pub bounding_volume: BoundingVolume,
    pub oriented_bounding_box: OrientedBoundingBox,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Default)]
pub struct BoundingSphere {
    pub center: Vector3,
    pub radius: f32,
}

/// A region of 3d space that contains a set of points.
/// This is equivalent to an axis-aligned bounding box (abbreviated AABB) for the XYZ axes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, Default)]
pub struct BoundingVolume {
    pub min: Vector3,
    pub max: Vector3,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, Default)]
pub struct OrientedBoundingBox {
    pub center: Vector3,
    pub transform: Matrix3x3,
    pub size: Vector3,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Copy, Clone)]
#[ssbhwrite(pad_after = 6)]
pub struct RiggingFlags {
    pub max_influences: u8,
    #[br(pad_after = 6)]
    pub unk1: u8,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct BoneBuffer<W2: BinRead<Args = ()> + SsbhWrite> {
    pub bone_name: SsbhString,
    pub data: W2,
}

/// Vertex skinning data for the vertices for the [MeshObject]
/// determined by [mesh_object_name](#structfield.mesh_object_name) and [mesh_object_subindex](#structfield.mesh_object_subindex).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RiggingGroup<W3: BinRead<Args = ()> + SsbhWrite> {
    pub mesh_object_name: SsbhString,
    pub mesh_object_subindex: u64,
    pub flags: RiggingFlags,
    pub buffers: SsbhArray<BoneBuffer<W3>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Attributes {
    #[br(pre_assert(major_version == 1 &&  minor_version == 8))]
    V8(SsbhArray<AttributeV8>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 9))]
    V9(SsbhArray<AttributeV9>),

    #[br(pre_assert(major_version == 1 &&  minor_version == 10))]
    V10(SsbhArray<AttributeV10>),
}

/// The type of array element for the vertex skin weights stored in the [SsbhByteBuffer] for [VertexWeightV8].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct VertexWeightV8 {
    pub vertex_index: u32,
    pub vertex_weight: f32,
}

/// The type of array element for the vertex skin weights stored in the [SsbhByteBuffer] for [VertexWeightV10].
#[derive(BinRead, Debug)]
pub struct VertexWeightV10 {
    pub vertex_index: u16,
    pub vertex_weight: f32,
}

/// An indexed vertex collection identified by its [name](#structfield.name) and [subindex](#structfield.subindex).
/// In addition to organizing the model into logical components, material and rigging data are assigned per [MeshObject].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshObject<A: BinRead<Args = ()> + SsbhWrite> {
    /// The name of the [MeshObject] such as `"c00BodyShape"`.
    /// Objects with the same name should have a unique [subindex](#structfield.subindex).
    pub name: SsbhString,
    /// The index for multiple objects of the same name starting from 0.
    pub subindex: u64,
    /// If [use_vertex_skinning](#structfield.use_vertex_skinning) is set to 0,
    /// the object's position is determined by the [SkelBoneEntry](crate::formats::skel::SkelBoneEntry) with matching name.
    /// Otherwise, [parent_bone_name](#structfield.parent_bone_name) is set to an empty string.
    pub parent_bone_name: SsbhString,
    /// The number of elements for this object in the [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers)
    /// Each attribute in [attributes](#structfield.subindex) should have the same number of elements.
    pub vertex_count: u32,
    /// The number of elements for this object in the [index_buffer](struct.Mesh.html#structfield.index_buffer).
    /// This determines the actual number of vertices rendered and will typically be larger than [vertex_count](#structfield.vertex_count).
    pub vertex_index_count: u32,
    pub unk2: u32, // number of indices per face (always 3)?
    pub vertex_buffer0_offset: u32,
    pub vertex_buffer1_offset: u32,
    pub vertex_buffer2_offset: u32,
    pub vertex_buffer3_offset: u32,
    /// The stride in bytes for the first buffer in [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers).
    pub stride0: u32,
    /// The stride in bytes for the second buffer in [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers).
    pub stride1: u32,
    /// The stride in bytes for the third buffer in [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers).
    pub stride2: u32,
    /// The stride in bytes for the fourth buffer in [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers).
    pub stride3: u32,
    /// The byte offset for the start of this object's vertex indices in the [index_buffer](struct.Mesh.html#structfield.index_buffer).
    /// The number of bytes to read is determined by [draw_element_type](#structfield.draw_element_type) and [vertex_index_count](#structfield.vertex_index_count).
    pub index_buffer_offset: u32,
    pub unk8: u32, // always 4
    /// The data type for the vertex indices stored in [index_buffer](struct.Mesh.html#structfield.index_buffer).
    pub draw_element_type: DrawElementType,
    /// A value of `1` uses the weights and influences in [rigging_buffers](struct.Mesh.html#structfield.rigging_buffers)
    /// A value of 0 disables vertex skinning, so vertices inherit the transforms determined by [parent_bone_name](#structfield.parent_bone_name).
    pub use_vertex_skinning: u32,
    /// An offset to affect this object's sort order, which can help resolve alpha blending issues caused by incorrect render order.
    pub sort_bias: i32, // TODO: Investigate this
    /// Flags to control depth testing.
    pub depth_flags: DepthFlags,
    pub bounding_info: BoundingInfo,
    /// Describes how the vertex attribute data for this object is stored in the [vertex_buffers](struct.Mesh.html#structfield.vertex_buffers).
    pub attributes: SsbhArray<A>,
}

/// Flags for controlling depth testing.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 2)]
pub struct DepthFlags {
    /// Disables writes to the depth buffer for this object when set to 1.
    pub disable_depth_write: u8,
    /// Disables depth testing for this object when set to 1.
    #[br(pad_after = 2)]
    pub disable_depth_test: u8,
}

/// Possible values for [draw_element_type](struct.MeshObject.html#structfield.draw_element_type).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum DrawElementType {
    /// Vertex indices stored as [u16].
    UnsignedShort = 0,
    /// Vertex indices stored as [u32].
    UnsignedInt = 1,
}

/// The data type and component count for the attribute's data.
/// This determines the stride and offset between attributes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum AttributeDataTypeV10 {
    /// 3 component (xyz or rgb) vector of [f32].
    Float3 = 0,
    /// 4 component (rgba) vector of [u8].
    Byte4 = 2,
    /// 4 component (xyzw or rgba) vector of [f32].
    Float4 = 4,
    /// 4 component (xyzw or rgba) vector of half precision floating point values.
    HalfFloat4 = 5,
    /// 2 component (xy or uv) vector of [f32].
    Float2 = 7,
    /// 2 component (xy or uv) vector of half precision floating point values.
    HalfFloat2 = 8,
}

/// The data type and component count for the attribute's data.
/// This determines the stride and offset between attributes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum AttributeDataTypeV8 {
    /// 3 component (xyz or rgb) vector of [f32].
    Float3 = 820,
    /// 4 component (rgba) vector of [f32].
    Float4 = 1076,
    /// 4 component (xyzw or rgba) vector of half precision floating point values.
    HalfFloat4 = 1077,
    /// 2 component (xy or uv) vector of [f32].
    Float2 = 1079,
    /// 4 component (rgba) vector of [u8].
    Byte4 = 1024,
}

/// Determines how the attribute data will be used by the shaders for [Mesh] version 1.9 and 1.10.
/// Attributes with an identical usage should each have a unique [subindex](struct.MeshAttributeV10.html#structfield.subindex).
/// Smash Ultimate also considers [name](struct.MeshAttributeV10.html#structfield.name) and
/// [attribute_names](struct.MeshAttributeV10.html#structfield.attribute_names) when determing the usage in some cases.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum AttributeUsageV9 {
    Position = 0,
    Normal = 1,
    Binormal = 2,
    Tangent = 3,
    TextureCoordinate = 4,
    ColorSet = 5,
}

/// Determines how the attribute data will be used by the shaders for [Mesh] version 1.8.
/// Attributes with an identical usage should each have a unique [subindex](struct.MeshAttributeV8.html#structfield.subindex).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum AttributeUsageV8 {
    Position = 0,
    Normal = 1,
    Tangent = 3,
    TextureCoordinate = 4,
    ColorSet = 8,
}
