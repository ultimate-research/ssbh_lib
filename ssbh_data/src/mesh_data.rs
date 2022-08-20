//! Types for working with [Mesh] data in .numshb files.
//!
//! # File Differences
//! Unmodified files are not guaranteed to be binary identical after saving.
//! [VectorData] uses [f32], which has enough precision to encode all known data types used for [Mesh] buffers.
//! When converting to [Mesh], the buffers are rebuilt using data types selected to balance
//! precision and space based on the attribute's usage. The resulting buffer is often identical in practice,
//! but this depends on the original file's data types.
//!
//! Bounding information is recalculated on export and is unlikely to match the original file
//! due to algorithmic differences and floating point errors.
use ahash::{AHashMap, AHashSet};
use binrw::io::Seek;
use binrw::{io::Cursor, BinRead};
use binrw::{BinReaderExt, BinResult};
use half::f16;
use itertools::Itertools;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_lib::formats::mesh::{
    AttributeV9, BoundingInfo, BoundingSphere, BoundingVolume, DepthFlags, MeshInner,
    OrientedBoundingBox,
};
use ssbh_lib::{
    formats::mesh::{
        AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
        AttributeV10, AttributeV8, BoneBuffer, DrawElementType, Mesh, MeshObject, RiggingFlags,
        RiggingGroup, VertexWeightV8,
    },
    SsbhByteBuffer,
};
use ssbh_lib::{Matrix3x3, SsbhArray, Vector3, Version};
use ssbh_write::SsbhWrite;
use std::convert::{TryFrom, TryInto};
use std::io::{Read, SeekFrom};
use std::{error::Error, io::Write};

mod vector_data;
pub use vector_data::VectorData;

mod mesh_attributes;
use mesh_attributes::*;

// A union of data types across all mesh versions.
#[derive(Debug, PartialEq)]
pub(crate) enum DataType {
    Float2,
    Float3,
    Float4,
    HalfFloat2,
    HalfFloat4,
    Byte4,
}

// A union of usages across all mesh versions.
#[derive(Debug, PartialEq)]
enum AttributeUsage {
    Position,
    Normal,
    Binormal,
    Tangent,
    TextureCoordinate,
    ColorSet,
}

pub mod error {
    use thiserror::Error;

    /// Errors while converting [Mesh](super::Mesh) to and from [MeshData](super::MeshData).
    #[derive(Debug, Error)]
    pub enum Error {
        /// The attributes have a different number of elements, so the vertex count cannot be determined.
        #[error("Attribute data lengths do not match. Failed to determined vertex count.")]
        AttributeDataLengthMismatch,

        /// A vertex index was detected that would result in an out of bounds access when rendering.
        /// All vertex indices should be strictly less than the vertex count.
        /// For mesh objects with a vertex count of 0 due to having no vertices, the vertex indices collection should be empty.
        #[error(
            "Vertex index {} is out of range for a vertex collection of size {}.",
            vertex_index,
            vertex_count
        )]
        VertexIndexOutOfRange {
            vertex_index: usize,
            vertex_count: usize,
        },

        #[error(
            "Vertex index count {} is not a multiple of 3. Only triangles are supported.",
            vertex_index_count
        )]
        NonTriangulatedFaces { vertex_index_count: usize },

        /// `vertex_index` exceeds the representable limit for skin weight indices.
        /// Version 1.8 and 1.9 have a limit of [u32::MAX].
        /// Version 1.10 has a limit of [u16::MAX].
        #[error(
            "Vertex index {} exceeds the limit of {} supported by mesh version {}.{}.",
            vertex_index,
            limit,
            major_version,
            minor_version
        )]
        SkinWeightVertexIndexExceedsLimit {
            vertex_index: usize,
            limit: usize,
            major_version: u16,
            minor_version: u16,
        },

        /// Creating a [Mesh](super::Mesh) file for the given version is not supported.
        #[error(
            "Creating a version {}.{} mesh is not supported.",
            major_version,
            minor_version
        )]
        UnsupportedVersion {
            major_version: u16,
            minor_version: u16,
        },

        /// An error occurred while writing data to a buffer.
        #[error(transparent)]
        Io(#[from] std::io::Error),
    }

    /// Errors while reading mesh attribute data.
    #[derive(Debug, Error)]
    pub enum AttributeError {
        /// An attribute buffer index was detected that does not refer to an available vertex buffer.
        #[error(
            "Buffer index {} is out of range for a buffer collection of size {}.",
            buffer_index,
            buffer_count
        )]
        BufferIndexOutOfRange {
            buffer_index: usize,
            buffer_count: usize,
        },

        /// Failed to find the offset or stride in bytes for the given buffer index.
        #[error("Found index {0}. Buffer indices higher than 4 are not supported.")]
        NoOffsetOrStride(u64),

        /// An error occurred while reading the data from the buffer.
        #[error(transparent)]
        Io(#[from] std::io::Error),

        /// An error occurred while reading the data from the buffer.
        #[error(transparent)]
        BinRead(#[from] binrw::error::Error),
    }
}

/// Assigns a weight to a particular vertex.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone)]
pub struct VertexWeight {
    pub vertex_index: u32,
    pub vertex_weight: f32,
}

impl From<AttributeDataTypeV10> for DataType {
    fn from(value: AttributeDataTypeV10) -> Self {
        match value {
            AttributeDataTypeV10::Float3 => Self::Float3,
            AttributeDataTypeV10::Byte4 => Self::Byte4,
            AttributeDataTypeV10::HalfFloat4 => Self::HalfFloat4,
            AttributeDataTypeV10::HalfFloat2 => Self::HalfFloat2,
            AttributeDataTypeV10::Float4 => Self::Float4,
            AttributeDataTypeV10::Float2 => Self::Float2,
        }
    }
}

impl From<AttributeDataTypeV8> for DataType {
    fn from(value: AttributeDataTypeV8) -> Self {
        match value {
            AttributeDataTypeV8::Float3 => Self::Float3,
            AttributeDataTypeV8::Float2 => Self::Float2,
            AttributeDataTypeV8::Byte4 => Self::Byte4,
            AttributeDataTypeV8::HalfFloat4 => Self::HalfFloat4,
            AttributeDataTypeV8::Float4 => Self::Float4,
        }
    }
}

// TODO: Add tests for this.
fn read_vertex_indices<A: Attribute>(
    mesh_index_buffer: &[u8],
    mesh_object: &MeshObject<A>,
) -> BinResult<Vec<u32>> {
    // Use u32 regardless of the actual data type to simplify conversions.
    let count = mesh_object.vertex_index_count as usize;
    let offset = mesh_object.index_buffer_offset as u64;
    let mut reader = Cursor::new(mesh_index_buffer);
    match mesh_object.draw_element_type {
        DrawElementType::UnsignedShort => read_data::<_, u16, u32>(&mut reader, count, offset),
        DrawElementType::UnsignedInt => read_data::<_, u32, u32>(&mut reader, count, offset),
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct Half(f16);

impl BinRead for Half {
    type Args = ();

    fn read_options<R: binrw::io::Read + Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let bits = u16::read_options(reader, options, args)?;
        let value = f16::from_bits(bits);
        Ok(Self(value))
    }
}

impl From<Half> for f32 {
    fn from(value: Half) -> Self {
        value.0.into()
    }
}

impl From<f32> for Half {
    fn from(value: f32) -> Self {
        Half(f16::from_f32(value))
    }
}

trait Attribute: BinRead<Args = ()> + SsbhWrite {
    fn to_attribute(&self) -> MeshAttribute;
    fn usage(&self) -> AttributeUsage;
}
impl Attribute for AttributeV8 {
    fn to_attribute(&self) -> MeshAttribute {
        // Version 1.8 doesn't have names.
        // Generate a name based on conventions for Smash Ultimate and New Pokemon Snap.
        let name = match self.usage {
            AttributeUsageV8::Position => format!("Position{}", self.subindex),
            AttributeUsageV8::Normal => format!("Normal{}", self.subindex),
            AttributeUsageV8::Tangent => format!("Tangent{}", self.subindex),
            AttributeUsageV8::TextureCoordinate => format!("TextureCoordinate{}", self.subindex),
            AttributeUsageV8::ColorSet => format!("colorSet{}", self.subindex),
        };

        MeshAttribute {
            name,
            index: self.buffer_index as u64,
            offset: self.buffer_offset as u64,
            data_type: self.data_type.into(),
        }
    }

    fn usage(&self) -> AttributeUsage {
        match self.usage {
            AttributeUsageV8::Position => AttributeUsage::Position,
            AttributeUsageV8::Normal => AttributeUsage::Normal,
            AttributeUsageV8::Tangent => AttributeUsage::Tangent,
            AttributeUsageV8::TextureCoordinate => AttributeUsage::TextureCoordinate,
            AttributeUsageV8::ColorSet => AttributeUsage::ColorSet,
        }
    }
}
impl Attribute for AttributeV9 {
    fn to_attribute(&self) -> MeshAttribute {
        MeshAttribute {
            name: get_attribute_name_v9(self).unwrap_or("").to_string(),
            index: self.buffer_index as u64,
            offset: self.buffer_offset as u64,
            data_type: self.data_type.into(),
        }
    }

    fn usage(&self) -> AttributeUsage {
        match self.usage {
            AttributeUsageV9::Position => AttributeUsage::Position,
            AttributeUsageV9::Normal => AttributeUsage::Normal,
            AttributeUsageV9::Binormal => AttributeUsage::Binormal,
            AttributeUsageV9::Tangent => AttributeUsage::Tangent,
            AttributeUsageV9::TextureCoordinate => AttributeUsage::TextureCoordinate,
            AttributeUsageV9::ColorSet => AttributeUsage::ColorSet,
        }
    }
}
impl Attribute for AttributeV10 {
    fn to_attribute(&self) -> MeshAttribute {
        MeshAttribute {
            name: get_attribute_name_v10(self).unwrap_or("").to_string(),
            index: self.buffer_index as u64,
            offset: self.buffer_offset as u64,
            data_type: self.data_type.into(),
        }
    }

    fn usage(&self) -> AttributeUsage {
        match self.usage {
            AttributeUsageV9::Position => AttributeUsage::Position,
            AttributeUsageV9::Normal => AttributeUsage::Normal,
            AttributeUsageV9::Binormal => AttributeUsage::Binormal,
            AttributeUsageV9::Tangent => AttributeUsage::Tangent,
            AttributeUsageV9::TextureCoordinate => AttributeUsage::TextureCoordinate,
            AttributeUsageV9::ColorSet => AttributeUsage::ColorSet,
        }
    }
}

trait Weight: BinRead<Args = ()> + SsbhWrite {
    fn from_weights(weights: &[VertexWeight]) -> Result<Self, error::Error>;
    fn to_weights(&self) -> Vec<VertexWeight>;
}

impl Weight for SsbhArray<VertexWeightV8> {
    fn from_weights(weights: &[VertexWeight]) -> Result<Self, error::Error> {
        create_vertex_weights_v8(weights)
    }

    fn to_weights(&self) -> Vec<VertexWeight> {
        self.elements
            .iter()
            .map(|influence| VertexWeight {
                vertex_index: influence.vertex_index,
                vertex_weight: influence.vertex_weight,
            })
            .collect()
    }
}

impl Weight for SsbhByteBuffer {
    fn from_weights(weights: &[VertexWeight]) -> Result<Self, error::Error> {
        create_vertex_weights_v10(weights)
    }

    fn to_weights(&self) -> Vec<VertexWeight> {
        let mut elements = Vec::new();
        let mut reader = Cursor::new(&self.elements);
        // TODO: Handle errors before reaching eof?
        while let Ok(influence) = reader.read_le::<ssbh_lib::formats::mesh::VertexWeightV10>() {
            elements.push(VertexWeight {
                vertex_index: influence.vertex_index as u32,
                vertex_weight: influence.vertex_weight,
            });
        }
        elements
    }
}

fn read_attribute_data<T, A: Attribute, W: Weight>(
    mesh: &MeshInner<A, W>,
    mesh_object: &MeshObject<A>,
    attribute: &MeshAttribute,
) -> Result<VectorData, error::AttributeError> {
    // Get the raw data for the attribute for this mesh object.
    let attribute_buffer = mesh
        .vertex_buffers
        .elements
        .get(attribute.index as usize)
        .ok_or(error::AttributeError::BufferIndexOutOfRange {
            buffer_index: attribute.index as usize,
            buffer_count: mesh.vertex_buffers.elements.len(),
        })?;

    let (offset, stride) = calculate_offset_stride(attribute, mesh_object)?;
    let count = mesh_object.vertex_count as usize;
    let mut reader = Cursor::new(&attribute_buffer.elements);

    VectorData::read(&mut reader, count, offset, stride, &attribute.data_type).map_err(Into::into)
}

fn calculate_offset_stride<A: Attribute>(
    attribute: &MeshAttribute,
    mesh_object: &MeshObject<A>,
) -> Result<(u64, u64), error::AttributeError> {
    let (offset, stride) = match attribute.index {
        0 => Ok((
            attribute.offset + mesh_object.vertex_buffer0_offset as u64,
            mesh_object.stride0 as u64,
        )),
        1 => Ok((
            attribute.offset + mesh_object.vertex_buffer1_offset as u64,
            mesh_object.stride1 as u64,
        )),
        2 => Ok((
            attribute.offset + mesh_object.vertex_buffer2_offset as u64,
            mesh_object.stride2 as u64,
        )),
        3 => Ok((
            attribute.offset + mesh_object.vertex_buffer3_offset as u64,
            mesh_object.stride3 as u64,
        )),
        _ => Err(error::AttributeError::NoOffsetOrStride(attribute.index)),
    }?;
    Ok((offset, stride))
}

fn read_attributes<A: Attribute, W: Weight>(
    mesh: &MeshInner<A, W>,
    mesh_object: &MeshObject<A>,
    usage: AttributeUsage,
) -> Result<Vec<AttributeData>, error::AttributeError> {
    let mut attributes = Vec::new();
    for attribute in &get_attributes(mesh_object, usage) {
        let data = read_attribute_data::<f32, _, _>(mesh, mesh_object, attribute)?;
        attributes.push(AttributeData {
            name: attribute.name.to_string(),
            data,
        })
    }
    Ok(attributes)
}

// TODO: Find ways to test this?
fn read_rigging_data<W: Weight>(
    rigging_buffers: &[RiggingGroup<W>],
    mesh_object_name: &str,
    mesh_object_subindex: u64,
) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
    // Collect the influences for the corresponding mesh object.
    // The mesh object will likely only be listed once,
    // but check all the rigging groups just in case.
    // TODO: Does in game handle duplicate subindices?
    // TODO: Can this work on read but error on save?
    // The goal is to be able to fix meshes by recalculating subindices and resaving.
    // TODO: A single buffer shared with multiple mesh objects can't be fixed?
    let mut bone_influences = Vec::new();
    for rigging_group in rigging_buffers.iter().filter(|r| {
        r.mesh_object_name.to_str() == Some(mesh_object_name)
            && r.mesh_object_subindex == mesh_object_subindex
    }) {
        bone_influences.extend(read_influences(rigging_group)?);
    }

    Ok(bone_influences)
}

/// A collection of vertex weights for all the vertices influenced by a bone.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub struct BoneInfluence {
    pub bone_name: String,
    pub vertex_weights: Vec<VertexWeight>,
}

/// The data associated with a [Mesh] file.
/// Supported versions are 1.8, 1.9, and 1.10.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub struct MeshData {
    pub major_version: u16,
    pub minor_version: u16,
    pub objects: Vec<MeshObjectData>,
}

impl TryFrom<MeshData> for Mesh {
    type Error = error::Error;

    fn try_from(data: MeshData) -> Result<Self, Self::Error> {
        create_mesh(&data)
    }
}

impl TryFrom<&MeshData> for Mesh {
    type Error = error::Error;

    fn try_from(data: &MeshData) -> Result<Self, Self::Error> {
        create_mesh(data)
    }
}

impl TryFrom<Mesh> for MeshData {
    type Error = Box<dyn Error>;

    fn try_from(mesh: Mesh) -> Result<Self, Self::Error> {
        (&mesh).try_into()
    }
}

impl TryFrom<&Mesh> for MeshData {
    type Error = Box<dyn Error>;

    fn try_from(mesh: &Mesh) -> Result<Self, Self::Error> {
        let (major_version, minor_version) = mesh.major_minor_version();
        Ok(Self {
            major_version,
            minor_version,
            objects: read_mesh_objects(mesh)?,
        })
    }
}

/// The data associated with a [MeshObject].
///
/// Vertex attribute data is stored in collections of [AttributeData] grouped by usage.
/// Each [AttributeData] will have its data indexed by [vertex_indices](struct.MeshAttributeV10.html.#structfield.vertex_indices),
/// so all [VectorData] should have the number of elements.
///
/// # Examples
/**
```rust
use ssbh_data::mesh_data::{MeshObjectData, AttributeData, VectorData};

let object = MeshObjectData {
    name: "triangle".to_string(),
    vertex_indices: vec![0, 1, 2],
    positions: vec![
        AttributeData {
            name: "Position0".into(),
            data: VectorData::Vector3(vec![
                [-1.0, 1.0, 0.0],
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0]
            ])
        }
    ],
    ..MeshObjectData::default()
};
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, Default)]
pub struct MeshObjectData {
    /// The name of this object.
    pub name: String,
    /// An additional identifier to differentiate multiple [MeshObjectData] with the same name.
    pub subindex: u64,
    /// The name of the parent bone. The empty string represents no parent for mesh objects that are not single bound.
    pub parent_bone_name: String,
    pub sort_bias: i32,
    pub disable_depth_write: bool,
    pub disable_depth_test: bool,
    /// Vertex indices for the data for all [AttributeData] for this [MeshObjectData].
    pub vertex_indices: Vec<u32>,
    pub positions: Vec<AttributeData>,
    pub normals: Vec<AttributeData>,
    pub binormals: Vec<AttributeData>,
    pub tangents: Vec<AttributeData>,
    pub texture_coordinates: Vec<AttributeData>,
    pub color_sets: Vec<AttributeData>,
    /// Vertex weights grouped by bone name.
    ///
    /// Each vertex should be influenced by at most 4 bones for most games, but the format doesn't enforce this.
    /// For meshes without vertex skinning, [bone_influences](#structfield.bone_influences) should be an empty list.
    pub bone_influences: Vec<BoneInfluence>,
}

/// Data corresponding to a named vertex attribute such as `"Position0"` or `"colorSet1"`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub struct AttributeData {
    pub name: String,
    pub data: VectorData,
}

impl MeshObjectData {
    // TODO: Document error conditions.
    // TODO: Tests?
    /// Calculates the vertex count.
    /// Returns an error if the lengths of the data in the [AttributeData] are not all equal.
    pub fn vertex_count(&self) -> Result<usize, error::Error> {
        // Make sure all the attributes have the same length.
        // This ensures the vertex indices do not cause any out of bounds accesses.
        let sizes: Vec<_> = self
            .positions
            .iter()
            .map(|a| a.data.len())
            .chain(self.normals.iter().map(|a| a.data.len()))
            .chain(self.binormals.iter().map(|a| a.data.len()))
            .chain(self.tangents.iter().map(|a| a.data.len()))
            .chain(self.texture_coordinates.iter().map(|a| a.data.len()))
            .chain(self.color_sets.iter().map(|a| a.data.len()))
            .collect();

        if sizes.iter().all_equal() {
            // TODO: Does zero length cause issues in game?
            match sizes.first() {
                Some(size) => Ok(*size),
                None => Ok(0),
            }
        } else {
            // TODO: Add the attribute lengths to the error?
            Err(error::Error::AttributeDataLengthMismatch)
        }
    }
}

fn read_mesh_objects(mesh: &Mesh) -> Result<Vec<MeshObjectData>, Box<dyn Error>> {
    match mesh {
        Mesh::V8(mesh) => read_mesh_objects_inner(mesh),
        Mesh::V9(mesh) => read_mesh_objects_inner(mesh),
        Mesh::V10(mesh) => read_mesh_objects_inner(mesh),
    }
}

fn read_mesh_objects_inner<A: Attribute, W: Weight>(
    mesh: &MeshInner<A, W>,
) -> Result<Vec<MeshObjectData>, Box<dyn Error>> {
    let mut mesh_objects = Vec::new();
    for mesh_object in &mesh.objects.elements {
        let name = mesh_object.name.to_string_lossy();

        let indices = read_vertex_indices(&mesh.index_buffer.elements, mesh_object)?;
        let positions = read_attributes(mesh, mesh_object, AttributeUsage::Position)?;
        let normals = read_attributes(mesh, mesh_object, AttributeUsage::Normal)?;
        let tangents = read_attributes(mesh, mesh_object, AttributeUsage::Tangent)?;
        let binormals = read_attributes(mesh, mesh_object, AttributeUsage::Binormal)?;
        let texture_coordinates =
            read_attributes(mesh, mesh_object, AttributeUsage::TextureCoordinate)?;
        let color_sets = read_attributes(mesh, mesh_object, AttributeUsage::ColorSet)?;
        let bone_influences =
            read_rigging_data(&mesh.rigging_buffers.elements, &name, mesh_object.subindex)?;

        let data = MeshObjectData {
            name,
            subindex: mesh_object.subindex,
            parent_bone_name: mesh_object
                .parent_bone_name
                .to_str()
                .unwrap_or("")
                .to_string(),
            vertex_indices: indices,
            positions,
            normals,
            tangents,
            binormals,
            texture_coordinates,
            color_sets,
            bone_influences,
            sort_bias: mesh_object.sort_bias,
            disable_depth_test: mesh_object.depth_flags.disable_depth_test != 0,
            disable_depth_write: mesh_object.depth_flags.disable_depth_write != 0,
        };

        mesh_objects.push(data);
    }
    Ok(mesh_objects)
}

fn create_mesh(data: &MeshData) -> Result<Mesh, error::Error> {
    // TODO: It might be more efficient to reuse the data for mesh object bounding or reuse the generated points.
    let all_positions: Vec<geometry_tools::glam::Vec3A> = data
        .objects
        .iter()
        .flat_map(|o| match o.positions.first() {
            Some(attribute) => attribute.data.to_glam_vec3a(),
            None => Vec::new(),
        })
        .collect();

    match (data.major_version, data.minor_version) {
        (1, 10) => Ok(Mesh::V10(create_mesh_inner(
            &all_positions,
            create_mesh_objects(&data.objects, create_attributes_v10)?,
            data,
        )?)),
        (1, 8) => Ok(Mesh::V8(create_mesh_inner(
            &all_positions,
            create_mesh_objects(&data.objects, create_attributes_v8)?,
            data,
        )?)),
        (1, 9) => Ok(Mesh::V9(create_mesh_inner(
            &all_positions,
            create_mesh_objects(&data.objects, create_attributes_v9)?,
            data,
        )?)),
        _ => Err(error::Error::UnsupportedVersion {
            major_version: data.major_version,
            minor_version: data.minor_version,
        }),
    }
}

fn create_mesh_inner<A: Attribute, W: Weight>(
    all_positions: &[glam::Vec3A],
    mesh_vertex_data: MeshVertexData<A>,
    data: &MeshData,
) -> Result<MeshInner<A, W>, error::Error> {
    Ok(MeshInner {
        model_name: "".into(),
        bounding_info: calculate_bounding_info(all_positions),
        unk1: 0,
        objects: mesh_vertex_data.mesh_objects.into(),
        // There are always at least 4 buffer entries even if only 2 are used.
        buffer_sizes: mesh_vertex_data
            .vertex_buffers
            .iter()
            .map(|b| b.len() as u32)
            // TODO: This is handled differently for v1.8.
            .pad_using(4, |_| 0u32)
            .collect(),
        polygon_index_size: mesh_vertex_data.index_buffer.len() as u64,
        vertex_buffers: mesh_vertex_data
            .vertex_buffers
            .into_iter()
            .map(SsbhByteBuffer::from_vec)
            .collect(),
        index_buffer: mesh_vertex_data.index_buffer.into(),
        rigging_buffers: create_rigging_buffers(&data.objects)?.into(),
    })
}

fn calculate_max_influences(influences: &[BoneInfluence], vertex_index_count: usize) -> usize {
    let mut influences_by_vertex = AHashMap::with_capacity(vertex_index_count);
    for influence in influences {
        // TODO: This can be even faster if we can assume no duplicate vertex indices for each influence.
        let mut influenced_vertices = AHashSet::new();
        for influence in &influence.vertex_weights {
            influenced_vertices.insert(influence.vertex_index);
        }

        for vertex in influenced_vertices {
            let entry = influences_by_vertex.entry(vertex).or_insert_with(|| 0);
            *entry += 1;
        }
    }

    influences_by_vertex.values().copied().max().unwrap_or(0)
}

fn create_rigging_buffers<W: Weight>(
    object_data: &[MeshObjectData],
) -> Result<Vec<RiggingGroup<W>>, error::Error> {
    let mut rigging_buffers = Vec::new();

    for mesh_object in object_data {
        // TODO: unk1 is sometimes set to 0 for singlebound mesh objects, which isn't currently preserved.
        let flags = RiggingFlags {
            max_influences: calculate_max_influences(
                &mesh_object.bone_influences,
                mesh_object.vertex_indices.len(),
            ) as u8,
            unk1: 1,
        };

        let mut buffers = Vec::new();
        for i in &mesh_object.bone_influences {
            let buffer = BoneBuffer {
                bone_name: i.bone_name.clone().into(),
                data: W::from_weights(&i.vertex_weights)?,
            };
            buffers.push(buffer);
        }

        let buffer = RiggingGroup {
            mesh_object_name: mesh_object.name.clone().into(),
            mesh_object_subindex: mesh_object.subindex,
            flags,
            buffers: buffers.into(),
        };

        rigging_buffers.push(buffer)
    }

    // Rigging buffers need to be sorted in ascending order by name and subindex.
    // TODO: Using a default may impact sorting if mesh_object_name is a null offset.
    // TODO: Check for duplicate subindices?
    rigging_buffers.sort_by_key(|k| (k.mesh_object_name.to_string_lossy(), k.mesh_object_subindex));

    Ok(rigging_buffers)
}

fn create_vertex_weights_v10(
    vertex_weights: &[VertexWeight],
) -> Result<SsbhByteBuffer, error::Error> {
    let mut bytes = Cursor::new(Vec::new());
    for weight in vertex_weights {
        let index: u16 = weight.vertex_index.try_into().map_err(|_| {
            error::Error::SkinWeightVertexIndexExceedsLimit {
                vertex_index: weight.vertex_index as usize,
                limit: u16::MAX as usize,
                major_version: 1,
                minor_version: 10,
            }
        })?;
        bytes.write_all(&index.to_le_bytes())?;
        bytes.write_all(&weight.vertex_weight.to_le_bytes())?;
    }
    Ok(bytes.into_inner().into())
}

fn create_vertex_weights_v8(
    vertex_weights: &[VertexWeight],
) -> Result<SsbhArray<VertexWeightV8>, error::Error> {
    Ok(vertex_weights
        .iter()
        .map(|v| VertexWeightV8 {
            vertex_index: v.vertex_index,
            vertex_weight: v.vertex_weight,
        })
        .collect())
}

// TODO: Make these methods.
trait AttributeDataTypeV10Ext {
    fn get_size_in_bytes_v10(&self) -> usize;
}

impl AttributeDataTypeV10Ext for AttributeDataTypeV10 {
    fn get_size_in_bytes_v10(&self) -> usize {
        match self {
            AttributeDataTypeV10::Float3 => std::mem::size_of::<f32>() * 3,
            AttributeDataTypeV10::Byte4 => std::mem::size_of::<u8>() * 4,
            AttributeDataTypeV10::HalfFloat4 => std::mem::size_of::<f16>() * 4,
            AttributeDataTypeV10::HalfFloat2 => std::mem::size_of::<f16>() * 2,
            AttributeDataTypeV10::Float4 => std::mem::size_of::<f32>() * 4,
            AttributeDataTypeV10::Float2 => std::mem::size_of::<f32>() * 2,
        }
    }
}

trait AttributeDataTypeV8Ext {
    fn get_size_in_bytes_v8(&self) -> usize;
}

impl AttributeDataTypeV8Ext for AttributeDataTypeV8 {
    fn get_size_in_bytes_v8(&self) -> usize {
        match self {
            AttributeDataTypeV8::Float3 => std::mem::size_of::<f32>() * 3,
            AttributeDataTypeV8::HalfFloat4 => std::mem::size_of::<f16>() * 4,
            AttributeDataTypeV8::Float2 => std::mem::size_of::<f32>() * 2,
            AttributeDataTypeV8::Byte4 => std::mem::size_of::<u8>() * 4,
            AttributeDataTypeV8::Float4 => std::mem::size_of::<f32>() * 4,
        }
    }
}

struct MeshVertexData<A: Attribute> {
    mesh_objects: Vec<MeshObject<A>>,
    vertex_buffers: Vec<Vec<u8>>,
    index_buffer: Vec<u8>,
}

#[derive(Debug, PartialEq)]
enum VertexIndices {
    UnsignedInt(Vec<u32>),
    UnsignedShort(Vec<u16>),
}

fn create_mesh_objects<A: Attribute, F: Fn(&MeshObjectData) -> MeshAttributes<A> + Copy>(
    mesh_object_data: &[MeshObjectData],
    create_attributes: F,
) -> Result<MeshVertexData<A>, error::Error> {
    let mut mesh_objects = Vec::new();

    let mut index_buffer = Cursor::new(Vec::new());

    // It's possible to preallocate the sizes by summing vertex counts and strides.
    // TODO: Investigate if this is actually has any performance benefit.
    let mut buffer0 = Cursor::new(Vec::new());
    let mut buffer1 = Cursor::new(Vec::new());
    let mut buffer2 = Cursor::new(Vec::new());
    let mut buffer3 = Cursor::new(Vec::new());

    // Don't just use the buffer position since different mesh versions handle this differently.
    let mut vertex_buffer2_offset = 0u64;

    for data in mesh_object_data {
        let mesh_object = create_mesh_object(
            data,
            &mut [&mut buffer0, &mut buffer1, &mut buffer2, &mut buffer3],
            &mut vertex_buffer2_offset,
            &mut index_buffer,
            create_attributes,
        )?;

        mesh_objects.push(mesh_object);
    }

    Ok(MeshVertexData {
        mesh_objects,
        vertex_buffers: vec![
            buffer0.into_inner(),
            buffer1.into_inner(),
            buffer2.into_inner(),
            buffer3.into_inner(),
        ],
        index_buffer: index_buffer.into_inner(),
    })
}

fn create_mesh_object<A: Attribute, F: Fn(&MeshObjectData) -> MeshAttributes<A>>(
    data: &MeshObjectData,
    buffers: &mut [&mut Cursor<Vec<u8>>; 4],
    vertex_buffer2_offset: &mut u64,
    index_buffer: &mut Cursor<Vec<u8>>,
    create_attributes: F,
) -> Result<MeshObject<A>, error::Error> {
    if data.vertex_indices.len() % 3 != 0 {
        return Err(error::Error::NonTriangulatedFaces {
            vertex_index_count: data.vertex_indices.len(),
        });
    }

    let vertex_count = data.vertex_count()?;

    // Check for out of bounds vertex accesses.
    // This helps prevent a potential source of errors when rendering.
    if let Some(max_value) = data.vertex_indices.iter().max() {
        if *max_value as usize >= vertex_count {
            return Err(error::Error::VertexIndexOutOfRange {
                vertex_index: *max_value as usize,
                vertex_count,
            });
        }
    }

    let vertex_indices = convert_indices(&data.vertex_indices);

    let draw_element_type = match vertex_indices {
        VertexIndices::UnsignedInt(_) => DrawElementType::UnsignedInt,
        VertexIndices::UnsignedShort(_) => DrawElementType::UnsignedShort,
    };

    let vertex_buffer0_offset = buffers[0].position();
    let vertex_buffer1_offset = buffers[1].position();
    let vertex_buffer3_offset = buffers[3].position();

    // TODO: This is pretty convoluted.
    let MeshAttributes {
        buffer_info,
        attributes,
        use_buffer2,
    } = create_attributes(data);

    let stride0 = buffer_info[0].0;
    let stride1 = buffer_info[1].0;
    let stride2 = buffer_info[2].0;
    let stride3 = buffer_info[3].0;

    // TODO: Version 1.10 sets the offset for buffer2 but sets stride to 0 and doesn't write to the buffer.
    write_attributes(
        &buffer_info,
        buffers,
        &[
            vertex_buffer0_offset,
            vertex_buffer1_offset,
            *vertex_buffer2_offset,
            vertex_buffer3_offset,
        ],
    )?;

    // Just write dummy data to buffer2 to match in game meshes for v1.8 and v.1.9.
    // Mesh v1.10 calculates offsets for this buffer but zeros stride and writes no data.
    if use_buffer2 {
        buffers[2].write_all(&vec![0u8; stride2 as usize * vertex_count])?;
    }

    let positions = match data.positions.first() {
        Some(attribute) => attribute.data.to_glam_vec3a(),
        None => Vec::new(),
    };

    let mesh_object = MeshObject {
        name: data.name.clone().into(),
        subindex: data.subindex,
        parent_bone_name: data.parent_bone_name.clone().into(),
        vertex_count: vertex_count as u32,
        vertex_index_count: data.vertex_indices.len() as u32,
        unk2: 3, // TODO: Does this mean triangle faces?
        vertex_buffer0_offset: vertex_buffer0_offset as u32,
        vertex_buffer1_offset: vertex_buffer1_offset as u32,
        vertex_buffer2_offset: *vertex_buffer2_offset as u32,
        vertex_buffer3_offset: vertex_buffer3_offset as u32,
        stride0,
        stride1,
        stride2: if use_buffer2 { stride2 } else { 0 },
        stride3,
        index_buffer_offset: index_buffer.position() as u32,
        unk8: 4, // TODO: index stride?
        draw_element_type,
        use_vertex_skinning: if data.bone_influences.is_empty() {
            0
        } else {
            1
        },
        sort_bias: data.sort_bias,
        depth_flags: DepthFlags {
            disable_depth_write: if data.disable_depth_write { 1 } else { 0 },
            disable_depth_test: if data.disable_depth_test { 1 } else { 0 },
        },
        bounding_info: calculate_bounding_info(&positions),
        attributes,
    };

    write_vertex_indices(&vertex_indices, index_buffer)?;

    // Assume stride2 is non zero for all versions.
    *vertex_buffer2_offset += vertex_count as u64 * stride2 as u64;

    Ok(mesh_object)
}

fn write_vertex_indices(
    indices: &VertexIndices,
    index_buffer: &mut Cursor<Vec<u8>>,
) -> Result<(), std::io::Error> {
    // Check if the indices could be successfully converted to u16.
    match indices {
        VertexIndices::UnsignedInt(indices) => {
            for index in indices {
                index_buffer.write_all(&index.to_le_bytes())?;
            }
        }
        VertexIndices::UnsignedShort(indices) => {
            for index in indices {
                index_buffer.write_all(&index.to_le_bytes())?;
            }
        }
    }
    Ok(())
}

fn convert_indices(indices: &[u32]) -> VertexIndices {
    // Try and convert the vertex indices to a smaller type.
    let u16_indices: Result<Vec<u16>, _> = indices.iter().map(|i| u16::try_from(*i)).collect();
    match u16_indices {
        Ok(indices) => VertexIndices::UnsignedShort(indices),
        Err(_) => VertexIndices::UnsignedInt(indices.into()),
    }
}

// TODO: Make a separate module for vector functions?
fn transform_inner(data: &VectorData, transform: &[[f32; 4]; 4], w: f32) -> VectorData {
    let mut points = data.to_glam_vec4_with_w(w);

    // Transform is assumed to be column-major.
    // Skip tranposing when converting to ensure the correct result inside the loop.
    let matrix = glam::Mat4::from_cols_array_2d(transform);
    for point in points.iter_mut() {
        *point = matrix.mul_vec4(*point);
    }

    // Preserve the original component count.
    match data {
        VectorData::Vector2(_) => VectorData::Vector2(points.iter().map(|p| [p.x, p.y]).collect()),
        VectorData::Vector3(_) => {
            VectorData::Vector3(points.iter().map(|p| [p.x, p.y, p.z]).collect())
        }
        // Preserve the original w component.
        // For example, tangents often store a sign component in the w component.
        VectorData::Vector4(original) => VectorData::Vector4(
            original
                .iter()
                .zip(points)
                .map(|(old, new)| [new.x, new.y, new.z, old[3]])
                .collect(),
        ),
    }
}

/// Transform the elements in `data` with `transform`.
/// Transform is assumed to be in column-major order.
/// The elements are treated as points in homogeneous coordinates by temporarily setting the 4th component to `1.0f32`.
/// The returned result has the same component count as `data`.
/// For [VectorData::Vector4], the 4th component is preserved for the returned result.
/**
```rust
# use ssbh_data::mesh_data::{VectorData, AttributeData, MeshObjectData, transform_points};
# let mesh_object_data = MeshObjectData {
#     name: "abc".into(),
#     positions: vec![AttributeData {
#         name: "Position0".into(),
#         data: VectorData::Vector3(Vec::new())
#     }],
#     ..MeshObjectData::default()
# };
// A scaling matrix for x, y, and z.
let transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 2.0, 0.0, 0.0],
    [0.0, 0.0, 3.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];
let transformed_positions = transform_points(&mesh_object_data.positions[0].data, &transform);
```
*/
pub fn transform_points(data: &VectorData, transform: &[[f32; 4]; 4]) -> VectorData {
    transform_inner(data, transform, 1.0)
}

/// Transform the elements in `data` with `transform`.
/// Transform is assumed to be in column-major order.
/// The elements are treated as vectors in homogeneous coordinates by temporarily setting the 4th component to `0.0f32`.
/// The returned result has the same component count as `data`.
/// For [VectorData::Vector4], the 4th component is preserved for the returned result.
/**
```rust
# use ssbh_data::mesh_data::{VectorData, AttributeData, MeshObjectData, transform_vectors};
# let mesh_object_data = MeshObjectData {
#     name: "abc".into(),
#     normals: vec![AttributeData {
#         name: "Normal0".into(),
#         data: VectorData::Vector3(Vec::new())
#     }],
#     ..MeshObjectData::default()
# };
// A scaling matrix for x, y, and z.
let transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 2.0, 0.0, 0.0],
    [0.0, 0.0, 3.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];
let transformed_normals = transform_vectors(&mesh_object_data.normals[0].data, &transform);
```
*/
pub fn transform_vectors(data: &VectorData, transform: &[[f32; 4]; 4]) -> VectorData {
    transform_inner(data, transform, 0.0)
}

// TODO: Add tests for these?
/// Calculates smooth per-vertex normals by by averaging over the vertices in each face.
/// See [geometry_tools::vectors::calculate_smooth_normals](geometry_tools::vectors::calculate_smooth_normals).
pub fn calculate_smooth_normals(positions: &VectorData, vertex_indices: &[u32]) -> Vec<[f32; 3]> {
    let normals = geometry_tools::vectors::calculate_smooth_normals(
        &positions.to_glam_vec3a(),
        vertex_indices,
    );

    normals.iter().map(|t| t.to_array()).collect()
}

/// Calculates smooth per-vertex tangents by averaging over the vertices in each face.
/// See [geometry_tools::vectors::calculate_tangents](geometry_tools::vectors::calculate_tangents).
pub fn calculate_tangents_vec4(
    positions: &VectorData,
    normals: &VectorData,
    uvs: &VectorData,
    vertex_indices: &[u32],
) -> Result<Vec<[f32; 4]>, Box<dyn Error>> {
    let tangents = geometry_tools::vectors::calculate_tangents(
        &positions.to_glam_vec3a(),
        &normals.to_glam_vec3a(),
        &uvs.to_glam_vec2(),
        vertex_indices,
    )?;

    Ok(tangents.iter().map(|t| t.to_array()).collect())
}

fn calculate_bounding_info(positions: &[geometry_tools::glam::Vec3A]) -> BoundingInfo {
    // Calculate bounding info based on the current points.
    let (sphere_center, sphere_radius) =
        geometry_tools::bounding::calculate_bounding_sphere_from_points(positions);
    let (aabb_min, aabb_max) = geometry_tools::bounding::calculate_aabb_from_points(positions);

    // TODO: Compute a better oriented bounding box.
    let obb_center = (aabb_min + aabb_max) / 2.0;
    let obb_size = (aabb_max - aabb_min) / 2.0;

    BoundingInfo {
        bounding_sphere: BoundingSphere {
            center: Vector3::new(sphere_center.x, sphere_center.y, sphere_center.z),
            radius: sphere_radius,
        },
        bounding_volume: BoundingVolume {
            min: Vector3::new(aabb_min.x, aabb_min.y, aabb_min.z),
            max: Vector3::new(aabb_max.x, aabb_max.y, aabb_max.z),
        },
        oriented_bounding_box: OrientedBoundingBox {
            center: Vector3::new(obb_center.x, obb_center.y, obb_center.z),
            transform: Matrix3x3::identity(),
            size: Vector3::new(obb_size.x, obb_size.y, obb_size.z),
        },
    }
}

fn read_influences<W: Weight>(
    rigging_group: &RiggingGroup<W>,
) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
    let mut bone_influences = Vec::new();
    for buffer in &rigging_group.buffers.elements {
        let bone_name = buffer
            .bone_name
            .to_str()
            .ok_or("Failed to read bone name.")?;

        // TODO: Find a way to test reading influence data.
        let bone_influence = BoneInfluence {
            bone_name: bone_name.to_string(),
            vertex_weights: buffer.data.to_weights(),
        };
        bone_influences.push(bone_influence);
    }

    Ok(bone_influences)
}

struct MeshAttribute {
    pub name: String,
    pub index: u64,
    pub offset: u64,
    pub data_type: DataType,
}

fn get_attributes<A: Attribute>(
    mesh_object: &MeshObject<A>,
    usage: AttributeUsage,
) -> Vec<MeshAttribute> {
    mesh_object
        .attributes
        .elements
        .iter()
        .filter(|a| a.usage() == usage)
        .map(|a| a.to_attribute())
        .collect()
}

fn get_attribute_name_v9(attribute: &AttributeV9) -> Option<&str> {
    attribute.attribute_names.elements.get(0)?.to_str()
}

fn get_attribute_name_v10(attribute: &AttributeV10) -> Option<&str> {
    attribute.attribute_names.elements.get(0)?.to_str()
}

pub fn read_data<R: Read + Seek, TIn: BinRead<Args = ()>, TOut: From<TIn>>(
    reader: &mut R,
    count: usize,
    offset: u64,
) -> BinResult<Vec<TOut>> {
    let mut result = Vec::new();
    reader.seek(SeekFrom::Start(offset))?;
    for _ in 0..count as u64 {
        result.push(reader.read_le::<TIn>()?.into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;

    #[test]
    fn read_data_count0() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, u16>(&mut reader, 0, 0).unwrap();
        assert_eq!(Vec::<u16>::new(), values);
    }

    #[test]
    fn read_data_count4() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, u32>(&mut reader, 4, 0).unwrap();
        assert_eq!(vec![1u32, 2u32, 3u32, 4u32], values);
    }

    #[test]
    fn read_data_offset() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, f32>(&mut reader, 2, 1).unwrap();
        assert_eq!(vec![2f32, 3f32], values);
    }

    #[test]
    fn read_half() {
        let mut reader = Cursor::new(hex!("003C00B4 00000000"));

        let value = reader.read_le::<Half>().unwrap();
        assert_eq!(1.0f32, f32::from(value));

        let value = reader.read_le::<Half>().unwrap();
        assert_eq!(-0.25f32, f32::from(value));

        let value = reader.read_le::<Half>().unwrap();
        assert_eq!(0.0f32, f32::from(value));
    }

    #[test]
    fn attribute_from_attribute_v10() {
        let attribute_v10 = AttributeV10 {
            usage: AttributeUsageV9::Normal,
            data_type: AttributeDataTypeV10::HalfFloat2,
            buffer_index: 2,
            buffer_offset: 10,
            subindex: 3,
            name: "custom_name".into(),
            attribute_names: vec!["name1".into()].into(),
        };

        let attribute: MeshAttribute = (&attribute_v10).to_attribute();
        assert_eq!("name1", attribute.name);
        assert_eq!(DataType::HalfFloat2, attribute.data_type);
        assert_eq!(2, attribute.index);
        assert_eq!(10, attribute.offset);
    }

    #[test]
    fn attribute_from_attribute_v8() {
        let attribute_v8 = AttributeV8 {
            usage: AttributeUsageV8::Normal,
            data_type: AttributeDataTypeV8::Float2,
            buffer_index: 1,
            buffer_offset: 8,
            subindex: 3,
        };

        let attribute: MeshAttribute = (&attribute_v8).to_attribute();
        assert_eq!("Normal3", attribute.name);
        assert_eq!(DataType::Float2, attribute.data_type);
        assert_eq!(1, attribute.index);
        assert_eq!(8, attribute.offset);
    }

    #[test]
    fn create_vertex_weights_mesh_v1_8() {
        // Version 1.8 uses an SsbhArray to store the weights.
        let weights = vec![
            VertexWeight {
                vertex_index: 0,
                vertex_weight: 0.0f32,
            },
            VertexWeight {
                vertex_index: 1,
                vertex_weight: 1.0f32,
            },
        ];

        let result = create_vertex_weights_v8(&weights).unwrap();

        assert_eq!(2, result.elements.len());

        assert_eq!(0, result.elements[0].vertex_index);
        assert_eq!(0.0f32, result.elements[0].vertex_weight);

        assert_eq!(1, result.elements[1].vertex_index);
        assert_eq!(1.0f32, result.elements[1].vertex_weight);
    }

    #[test]
    fn create_vertex_weights_mesh_v1_10() {
        // Version 1.10 writes the weights to a byte array.
        // u16 for index and f32 for weight.
        let weights = vec![
            VertexWeight {
                vertex_index: 0,
                vertex_weight: 0.0f32,
            },
            VertexWeight {
                vertex_index: 1,
                vertex_weight: 1.0f32,
            },
        ];

        let result = create_vertex_weights_v10(&weights).unwrap();
        assert_eq!(&result.elements[..], &hex!("0000 00000000 01000 000803f"));
    }

    #[test]
    fn draw_element_type_u16() {
        // The indices are always stored as u32 by the object data wrapper type.
        // In this case, it's safe to convert to a smaller type.
        assert_eq!(
            VertexIndices::UnsignedShort(vec![0, 1, u16::MAX]),
            convert_indices(&[0, 1, u16::MAX as u32])
        )
    }

    #[test]
    fn draw_element_type_empty() {
        assert_eq!(
            VertexIndices::UnsignedShort(Vec::new()),
            convert_indices(&[])
        )
    }

    #[test]
    fn draw_element_type_u32() {
        // Add elements not representable by u16.
        assert_eq!(
            VertexIndices::UnsignedInt(vec![0, 1, u16::MAX as u32 + 1]),
            convert_indices(&[0, 1, u16::MAX as u32 + 1])
        )
    }

    #[test]
    fn size_in_bytes_attributes_v10() {
        assert_eq!(4, AttributeDataTypeV10::Byte4.get_size_in_bytes_v10());
        assert_eq!(8, AttributeDataTypeV10::Float2.get_size_in_bytes_v10());
        assert_eq!(12, AttributeDataTypeV10::Float3.get_size_in_bytes_v10());
        assert_eq!(16, AttributeDataTypeV10::Float4.get_size_in_bytes_v10());
        assert_eq!(4, AttributeDataTypeV10::HalfFloat2.get_size_in_bytes_v10());
        assert_eq!(8, AttributeDataTypeV10::HalfFloat4.get_size_in_bytes_v10());
    }

    #[test]
    fn size_in_bytes_attributes_v8() {
        assert_eq!(4, AttributeDataTypeV8::Byte4.get_size_in_bytes_v8());
        assert_eq!(8, AttributeDataTypeV8::Float2.get_size_in_bytes_v8());
        assert_eq!(12, AttributeDataTypeV8::Float3.get_size_in_bytes_v8());
        assert_eq!(16, AttributeDataTypeV8::Float4.get_size_in_bytes_v8());
        assert_eq!(8, AttributeDataTypeV8::HalfFloat4.get_size_in_bytes_v8());
    }

    #[test]
    fn max_influences_no_bones() {
        assert_eq!(0, calculate_max_influences(&[], 0));
    }

    #[test]
    fn max_influences_one_bone_no_weights() {
        let influences = vec![BoneInfluence {
            bone_name: "a".to_string(),
            vertex_weights: Vec::new(),
        }];
        assert_eq!(0, calculate_max_influences(&influences, 0));
    }

    #[test]
    fn max_influences_one_bone() {
        // Check that only influences are counted and not occurrences within an influence.
        let influences = vec![BoneInfluence {
            bone_name: "a".to_string(),
            vertex_weights: vec![
                VertexWeight {
                    vertex_index: 0,
                    vertex_weight: 0f32,
                },
                VertexWeight {
                    vertex_index: 0,
                    vertex_weight: 0f32,
                },
            ],
        }];
        // This is 1 and not 2 since there is only a single bone.
        assert_eq!(1, calculate_max_influences(&influences, 0));
        assert_eq!(1, calculate_max_influences(&influences, 2));
    }

    #[test]
    fn max_influences_three_bones() {
        // Check that only influences are counted and not occurrences within an influence.
        let influences = vec![
            BoneInfluence {
                bone_name: "a".to_string(),
                vertex_weights: vec![
                    VertexWeight {
                        vertex_index: 0,
                        vertex_weight: 0f32,
                    },
                    VertexWeight {
                        vertex_index: 0,
                        vertex_weight: 0f32,
                    },
                    VertexWeight {
                        vertex_index: 0,
                        vertex_weight: 0f32,
                    },
                    VertexWeight {
                        vertex_index: 3,
                        vertex_weight: 0f32,
                    },
                ],
            },
            BoneInfluence {
                bone_name: "b".to_string(),
                vertex_weights: vec![
                    VertexWeight {
                        vertex_index: 2,
                        vertex_weight: 0f32,
                    },
                    VertexWeight {
                        vertex_index: 1,
                        vertex_weight: 0f32,
                    },
                    VertexWeight {
                        vertex_index: 3,
                        vertex_weight: 0f32,
                    },
                ],
            },
            BoneInfluence {
                bone_name: "c".to_string(),
                vertex_weights: vec![
                    VertexWeight {
                        vertex_index: 0,
                        vertex_weight: 0f32,
                    },
                    VertexWeight {
                        vertex_index: 3,
                        vertex_weight: 0f32,
                    },
                ],
            },
        ];

        // The vertex index count shouldn't need to be exact.
        assert_eq!(3, calculate_max_influences(&influences, 0));
        assert_eq!(3, calculate_max_influences(&influences, 4));
    }

    #[test]
    fn create_empty_mesh_1_10() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 10,
            objects: Vec::new(),
        })
        .unwrap();
        assert!(matches!(mesh,
            Mesh::V10(MeshInner { objects, rigging_buffers, index_buffer, .. })
            if objects.elements.is_empty() && rigging_buffers.elements.is_empty() && index_buffer.elements.is_empty()
        ));
    }

    #[test]
    fn create_empty_mesh_1_8() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 8,
            objects: Vec::new(),
        })
        .unwrap();

        assert!(matches!(mesh,
            Mesh::V8(MeshInner { objects, rigging_buffers, index_buffer, .. })
            if objects.elements.is_empty() && rigging_buffers.elements.is_empty() && index_buffer.elements.is_empty()
        ));
    }

    #[test]
    fn create_empty_mesh_v_1_9() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 9,
            objects: Vec::new(),
        })
        .unwrap();

        assert!(matches!(mesh,
            Mesh::V9(MeshInner { objects, rigging_buffers, index_buffer, .. })
            if objects.elements.is_empty() && rigging_buffers.elements.is_empty() && index_buffer.elements.is_empty()
        ));
    }

    #[test]
    fn create_empty_mesh_invalid_version() {
        let result = create_mesh(&MeshData {
            major_version: 2,
            minor_version: 301,
            objects: Vec::new(),
        });

        assert!(matches!(
            result,
            Err(error::Error::UnsupportedVersion {
                major_version: 2,
                minor_version: 301
            })
        ));
    }

    #[test]
    fn create_mesh_1_10() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    positions: vec![AttributeData {
                        name: String::new(),
                        data: VectorData::Vector3(vec![[0.0; 3]; 12]),
                    }],
                    bone_influences: vec![BoneInfluence {
                        bone_name: "a".to_owned(),
                        vertex_weights: vec![VertexWeight {
                            vertex_index: u16::MAX as u32,
                            vertex_weight: 1.0,
                        }],
                    }],
                    ..Default::default()
                },
                MeshObjectData {
                    positions: vec![AttributeData {
                        name: String::new(),
                        data: VectorData::Vector3(vec![[0.0; 3]; 12]),
                    }],
                    bone_influences: vec![BoneInfluence {
                        bone_name: "b".to_owned(),
                        vertex_weights: vec![VertexWeight {
                            vertex_index: u16::MAX as u32,
                            vertex_weight: 1.0,
                        }],
                    }],
                    ..Default::default()
                },
            ],
        })
        .unwrap();

        // Different mesh versions have different conventions for unused vertex buffers.
        // TODO: Test other values?
        assert!(matches!(mesh,
            Mesh::V10(MeshInner { objects, vertex_buffers, .. })
            if vertex_buffers.elements
            == vec![
                vec![0u8; 4 * 3 * 12 * 2].into(),
                SsbhByteBuffer::new(),
                SsbhByteBuffer::new(),
                SsbhByteBuffer::new(),
            ]
            && objects.elements[0].vertex_buffer0_offset == 0
            && objects.elements[0].vertex_buffer1_offset == 0
            && objects.elements[0].vertex_buffer2_offset == 0
            && objects.elements[0].vertex_buffer3_offset == 0
            && objects.elements[0].stride0 == 12
            && objects.elements[0].stride1 == 0
            && objects.elements[0].stride2 == 0
            && objects.elements[0].stride3 == 0
            && objects.elements[1].vertex_buffer0_offset == 12*12
            && objects.elements[1].vertex_buffer1_offset == 0
            && objects.elements[1].vertex_buffer2_offset == 32*12
            && objects.elements[1].vertex_buffer3_offset == 0
            && objects.elements[1].stride0 == 12
            && objects.elements[1].stride1 == 0
            && objects.elements[1].stride2 == 0
            && objects.elements[1].stride3 == 0
        ));
    }

    #[test]
    fn create_mesh_1_10_too_many_vertices() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                positions: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector3(vec![[0.0; 3]; 3]),
                }],
                bone_influences: vec![BoneInfluence {
                    bone_name: "a".to_owned(),
                    vertex_weights: vec![VertexWeight {
                        vertex_index: u16::MAX as u32 + 1,
                        vertex_weight: 1.0,
                    }],
                }],
                ..Default::default()
            }],
        });

        // TODO: Test version 1.8 and 1.9?
        assert!(matches!(
            mesh,
            Err(error::Error::SkinWeightVertexIndexExceedsLimit {
                vertex_index: 65536,
                limit: 65535,
                major_version: 1,
                minor_version: 10,
            })
        ));
    }

    #[test]
    fn create_mesh_1_8() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 8,
            objects: vec![
                MeshObjectData {
                    positions: vec![AttributeData {
                        name: String::new(),
                        data: VectorData::Vector3(vec![[0.0; 3]; 12]),
                    }],
                    bone_influences: vec![BoneInfluence {
                        bone_name: "a".to_owned(),
                        vertex_weights: vec![VertexWeight {
                            vertex_index: u32::MAX as u32,
                            vertex_weight: 1.0,
                        }],
                    }],
                    ..Default::default()
                },
                MeshObjectData {
                    positions: vec![AttributeData {
                        name: String::new(),
                        data: VectorData::Vector3(vec![[0.0; 3]; 12]),
                    }],
                    bone_influences: vec![BoneInfluence {
                        bone_name: "b".to_owned(),
                        vertex_weights: vec![VertexWeight {
                            vertex_index: u32::MAX as u32,
                            vertex_weight: 1.0,
                        }],
                    }],
                    ..Default::default()
                },
            ],
        })
        .unwrap();

        // Different mesh versions have different conventions for unused vertex buffers.
        // TODO: Test other values?
        assert!(matches!(mesh,
            Mesh::V8(MeshInner { objects, vertex_buffers, .. })
            if vertex_buffers.elements == vec![
                vec![0u8; 4 * 3 * 12 * 2].into(),
                SsbhByteBuffer::new(),
                vec![0u8; 32 * 12 * 2].into(),
                SsbhByteBuffer::new(),
            ]
            && objects.elements[0].vertex_buffer0_offset == 0
            && objects.elements[0].vertex_buffer1_offset == 0
            && objects.elements[0].vertex_buffer2_offset == 0
            && objects.elements[0].vertex_buffer3_offset == 0
            && objects.elements[0].stride0 == 12
            && objects.elements[0].stride1 == 0
            && objects.elements[0].stride2 == 32
            && objects.elements[0].stride3 == 0
            && objects.elements[1].vertex_buffer0_offset == 12*12
            && objects.elements[1].vertex_buffer1_offset == 0
            && objects.elements[1].vertex_buffer2_offset == 32*12
            && objects.elements[1].vertex_buffer3_offset == 0
            && objects.elements[1].stride0 == 12
            && objects.elements[1].stride1 == 0
            && objects.elements[1].stride2 == 32
            && objects.elements[1].stride3 == 0
        ));
    }

    #[test]
    fn create_mesh_v_1_9() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 9,
            objects: vec![
                MeshObjectData {
                    positions: vec![AttributeData {
                        name: String::new(),
                        data: VectorData::Vector3(vec![[0.0; 3]; 12]),
                    }],
                    bone_influences: vec![BoneInfluence {
                        bone_name: "a".to_owned(),
                        vertex_weights: vec![VertexWeight {
                            vertex_index: u32::MAX as u32,
                            vertex_weight: 1.0,
                        }],
                    }],
                    ..Default::default()
                },
                MeshObjectData {
                    positions: vec![AttributeData {
                        name: String::new(),
                        data: VectorData::Vector3(vec![[0.0; 3]; 12]),
                    }],
                    bone_influences: vec![BoneInfluence {
                        bone_name: "b".to_owned(),
                        vertex_weights: vec![VertexWeight {
                            vertex_index: u32::MAX as u32,
                            vertex_weight: 1.0,
                        }],
                    }],
                    ..Default::default()
                },
            ],
        })
        .unwrap();

        // Different mesh versions have different conventions for unused vertex buffers.
        // TODO: Test other values?
        assert!(matches!(mesh,
            Mesh::V9(MeshInner { objects, vertex_buffers, .. })
            if vertex_buffers.elements == vec![
                vec![0u8; 4 * 3 * 12 * 2].into(),
                SsbhByteBuffer::new(),
                vec![0u8; 32 * 12 * 2].into(),
                SsbhByteBuffer::new(),
            ]
            && objects.elements[0].vertex_buffer0_offset == 0
            && objects.elements[0].vertex_buffer1_offset == 0
            && objects.elements[0].vertex_buffer2_offset == 0
            && objects.elements[0].vertex_buffer3_offset == 0
            && objects.elements[0].stride0 == 12
            && objects.elements[0].stride1 == 0
            && objects.elements[0].stride2 == 32
            && objects.elements[0].stride3 == 0
            && objects.elements[1].vertex_buffer0_offset == 12*12
            && objects.elements[1].vertex_buffer1_offset == 0
            && objects.elements[1].vertex_buffer2_offset == 32*12
            && objects.elements[1].vertex_buffer3_offset == 0
            && objects.elements[1].stride0 == 12
            && objects.elements[1].stride1 == 0
            && objects.elements[1].stride2 == 32
            && objects.elements[1].stride3 == 0
        ));
    }

    #[test]
    fn transform_points_vec2() {
        let data = VectorData::Vector2(vec![[0.0, 1.0], [2.0, 3.0]]);
        let transform = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 6.0, 0.0],
            [0.0, 0.0, 4.0, 5.0],
        ];
        let transformed = transform_points(&data, &transform);
        let expected = VectorData::Vector2(vec![[0.0, 3.0], [4.0, 9.0]]);
        assert_eq!(expected, transformed)
    }

    #[test]
    fn transform_points_vec4() {
        let data = VectorData::Vector4(vec![[0.0, 1.0, 0.0, -1.0], [2.0, 3.0, 0.0, 5.0]]);
        let transform = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 4.0, 5.0],
        ];
        let transformed = transform_points(&data, &transform);
        let expected = VectorData::Vector4(vec![[0.0, 3.0, 4.0, -1.0], [4.0, 9.0, 4.0, 5.0]]);
        assert_eq!(expected, transformed)
    }

    #[test]
    fn transform_vectors_vec2() {
        let data = VectorData::Vector2(vec![[0.0, 1.0], [2.0, 3.0]]);
        let transform = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 6.0, 0.0],
            [0.0, 0.0, 4.0, 5.0],
        ];
        let transformed = transform_vectors(&data, &transform);
        let expected = VectorData::Vector2(vec![[0.0, 3.0], [4.0, 9.0]]);
        assert_eq!(expected, transformed)
    }

    #[test]
    fn transform_vectors_vec4() {
        let data = VectorData::Vector4(vec![[0.0, 1.0, 0.0, -1.0], [2.0, 3.0, 0.0, 5.0]]);
        let transform = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 4.0, 5.0],
        ];
        let transformed = transform_vectors(&data, &transform);
        // This is similar to the points test, but the translation should have no effect since w is set to 0.0.
        let expected = VectorData::Vector4(vec![[0.0, 3.0, 0.0, -1.0], [4.0, 9.0, 0.0, 5.0]]);
        assert_eq!(expected, transformed)
    }

    #[test]
    fn calculate_offset_stride_buffer_indices() {
        let mesh_object = MeshObject::<AttributeV10> {
            name: String::new().into(),
            subindex: 0,
            parent_bone_name: String::new().into(),
            vertex_count: 0,
            vertex_index_count: 0,
            unk2: 0,
            vertex_buffer0_offset: 1,
            vertex_buffer1_offset: 2,
            vertex_buffer2_offset: 3,
            vertex_buffer3_offset: 4,
            stride0: 11,
            stride1: 22,
            stride2: 33,
            stride3: 44,
            index_buffer_offset: 0,
            unk8: 0,
            draw_element_type: DrawElementType::UnsignedInt,
            use_vertex_skinning: 0,
            sort_bias: 0,
            depth_flags: DepthFlags {
                disable_depth_write: 0,
                disable_depth_test: 0,
            },
            bounding_info: BoundingInfo::default(),
            attributes: SsbhArray::new(),
        };

        assert_eq!(
            (2, 11),
            calculate_offset_stride(
                &MeshAttribute {
                    name: String::new(),
                    index: 0,
                    offset: 1,
                    data_type: DataType::Byte4,
                },
                &mesh_object
            )
            .unwrap()
        );

        assert_eq!(
            (3, 22),
            calculate_offset_stride(
                &MeshAttribute {
                    name: String::new(),
                    index: 1,
                    offset: 1,
                    data_type: DataType::Byte4,
                },
                &mesh_object
            )
            .unwrap()
        );

        assert_eq!(
            (4, 33),
            calculate_offset_stride(
                &MeshAttribute {
                    name: String::new(),
                    index: 2,
                    offset: 1,
                    data_type: DataType::Byte4,
                },
                &mesh_object
            )
            .unwrap()
        );

        assert_eq!(
            (5, 44),
            calculate_offset_stride(
                &MeshAttribute {
                    name: String::new(),
                    index: 3,
                    offset: 1,
                    data_type: DataType::Byte4,
                },
                &mesh_object
            )
            .unwrap()
        );

        // Invalid index.
        let result = calculate_offset_stride(
            &MeshAttribute {
                name: String::new(),
                index: 4,
                offset: 0,
                data_type: DataType::Byte4,
            },
            &mesh_object,
        );
        assert!(matches!(
            result,
            Err(error::AttributeError::NoOffsetOrStride(4))
        ));
    }

    #[test]
    fn create_empty_mesh_object() {
        let object = create_mesh_object(
            &MeshObjectData {
                name: String::new(),
                subindex: 1,
                sort_bias: -5,
                disable_depth_test: true,
                disable_depth_write: false,
                ..MeshObjectData::default()
            },
            &mut [
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
            ],
            &mut 0,
            &mut Cursor::new(Vec::new()),
            create_attributes_v10,
        )
        .unwrap();

        assert_eq!(1, object.subindex);
        assert_eq!(-5, object.sort_bias);
        assert_eq!(0, object.depth_flags.disable_depth_write);
        assert_eq!(1, object.depth_flags.disable_depth_test);
    }

    #[test]
    fn create_mesh_object_vertex_count_mismatch() {
        // The vertex count can't be determined since 1 != 2.
        let result = create_mesh_object(
            &MeshObjectData {
                positions: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                tangents: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0]]),
                }],
                ..MeshObjectData::default()
            },
            &mut [
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
            ],
            &mut 0,
            &mut Cursor::new(Vec::new()),
            create_attributes_v10,
        );

        assert!(matches!(
            result,
            Err(error::Error::AttributeDataLengthMismatch)
        ));
    }

    #[test]
    fn create_mesh_object_valid_indices() {
        create_mesh_object(
            &MeshObjectData {
                vertex_indices: vec![0, 1, 1],
                positions: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                tangents: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                ..MeshObjectData::default()
            },
            &mut [
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
            ],
            &mut 0,
            &mut Cursor::new(Vec::new()),
            create_attributes_v10,
        )
        .unwrap();
    }

    #[test]
    fn create_mesh_object_quad_faces() {
        // Currently only triangles are supported.
        let result = create_mesh_object(
            &MeshObjectData {
                vertex_indices: vec![0, 2, 1, 0, 2, 1, 0, 0],
                positions: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                tangents: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                ..MeshObjectData::default()
            },
            &mut [
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
            ],
            &mut 0,
            &mut Cursor::new(Vec::new()),
            create_attributes_v10,
        );

        assert!(matches!(
            result,
            Err(error::Error::NonTriangulatedFaces {
                vertex_index_count: 8,
            })
        ));
    }

    #[test]
    fn create_mesh_object_invalid_indices() {
        // Index 2 is out of bounds for the vertex attribute arrays.
        let result = create_mesh_object(
            &MeshObjectData {
                vertex_indices: vec![0, 2, 1],
                positions: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                tangents: vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector2(vec![[0.0, 0.0], [0.0, 0.0]]),
                }],
                ..MeshObjectData::default()
            },
            &mut [
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
            ],
            &mut 0,
            &mut Cursor::new(Vec::new()),
            create_attributes_v10,
        );

        assert!(matches!(
            result,
            Err(error::Error::VertexIndexOutOfRange {
                vertex_index: 2,
                vertex_count: 2
            })
        ));
    }

    #[test]
    fn create_empty_mesh_object_invalid_indices() {
        // Index 0 is out of bounds for the vertex attribute arrays.
        let result = create_mesh_object(
            &MeshObjectData {
                vertex_indices: vec![0, 0, 0],
                ..MeshObjectData::default()
            },
            &mut [
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
                &mut Cursor::new(Vec::new()),
            ],
            &mut 0,
            &mut Cursor::new(Vec::new()),
            create_attributes_v10,
        );

        assert!(matches!(
            result,
            Err(error::Error::VertexIndexOutOfRange {
                vertex_index: 0,
                vertex_count: 0
            })
        ));
    }
}
