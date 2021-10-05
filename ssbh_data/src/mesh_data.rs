use binread::{io::Cursor, BinRead};
use binread::{BinReaderExt, BinResult};
use half::f16;
use itertools::Itertools;
use ssbh_lib::formats::mesh::{
    BoundingInfo, BoundingSphere, BoundingVolume, MeshAttributeV9, OrientedBoundingBox, RiggingType,
};
use ssbh_lib::{
    formats::mesh::{
        AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
        DrawElementType, Mesh, MeshAttributeV10, MeshAttributeV8, MeshAttributes, MeshBoneBuffer,
        MeshObject, MeshRiggingGroup, RiggingFlags, VertexWeightV8, VertexWeights,
    },
    SsbhByteBuffer,
};
use ssbh_lib::{Half, Matrix3x3, Vector3};
use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::io::{Read, Seek};
use std::ops::{Add, Div, Sub};
use std::path::Path;
use std::{error::Error, io::Write};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{read_data, read_vector_data, SsbhData};

mod mesh_attributes;
use mesh_attributes::*;

#[derive(Debug, PartialEq)]
enum DataType {
    Float2,
    Float3,
    Float4,
    HalfFloat2,
    HalfFloat4,
    Byte4,
}

#[derive(Debug, PartialEq)]
enum AttributeUsage {
    Position,
    Normal,
    Binormal,
    Tangent,
    TextureCoordinate,
    ColorSet,
}

/// Errors while creating a [Mesh] from [MeshObjectData].
#[derive(Error, Debug)]

pub enum MeshError {
    /// The attributes for a [MeshObject] would have different number of elements,
    /// so the vertex count cannot be determined.
    #[error("Attribute data lengths do not match. Failed to determined vertex count.")]
    AttributeDataLengthMismatch,

    /// Creating a [Mesh] file for the given version is not supported.
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
#[derive(Error, Debug)]
pub enum AttributeError {
    /// Attempted to read from a nonexistent buffer.
    #[error("No buffer found for index {0}.")]
    InvalidBufferIndex(u64),

    /// Failed to find the offset or stride in bytes for the given buffer index.
    #[error("Found index {0}. Buffer indices higher than 4 are not supported.")]
    NoOffsetOrStride(u64),

    /// An error occurred while reading the data from the buffer.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error occurred while reading the data from the buffer.
    #[error(transparent)]
    BinRead(#[from] binread::error::Error),
}

impl From<AttributeUsageV9> for AttributeUsage {
    fn from(a: AttributeUsageV9) -> Self {
        match a {
            AttributeUsageV9::Position => Self::Position,
            AttributeUsageV9::Normal => Self::Normal,
            AttributeUsageV9::Binormal => Self::Binormal,
            AttributeUsageV9::Tangent => Self::Tangent,
            AttributeUsageV9::TextureCoordinate => Self::TextureCoordinate,
            AttributeUsageV9::ColorSet => Self::ColorSet,
        }
    }
}

impl From<AttributeUsageV8> for AttributeUsage {
    fn from(a: AttributeUsageV8) -> Self {
        match a {
            AttributeUsageV8::Position => Self::Position,
            AttributeUsageV8::Normal => Self::Normal,
            AttributeUsageV8::Tangent => Self::Tangent,
            AttributeUsageV8::TextureCoordinate => Self::TextureCoordinate,
            AttributeUsageV8::ColorSet => Self::ColorSet,
        }
    }
}

/// Assigns a weight to a particular vertex.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

fn read_vertex_indices(mesh_index_buffer: &[u8], mesh_object: &MeshObject) -> BinResult<Vec<u32>> {
    // Use u32 regardless of the actual data type to simplify conversions.
    let count = mesh_object.vertex_index_count as usize;
    let offset = mesh_object.index_buffer_offset as u64;
    let mut reader = Cursor::new(mesh_index_buffer);
    match mesh_object.draw_element_type {
        DrawElementType::UnsignedShort => read_data::<_, u16, u32>(&mut reader, count, offset),
        DrawElementType::UnsignedInt => read_data::<_, u32, u32>(&mut reader, count, offset),
    }
}

fn read_attribute_data<T>(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    attribute: &MeshAttribute,
) -> Result<VectorData, AttributeError> {
    // Get the raw data for the attribute for this mesh object.
    let attribute_buffer = mesh
        .vertex_buffers
        .elements
        .get(attribute.index as usize)
        .ok_or(AttributeError::InvalidBufferIndex(attribute.index))?;

    let (offset, stride) = calculate_offset_stride(attribute, mesh_object)?;

    let count = mesh_object.vertex_count as usize;

    let mut reader = Cursor::new(&attribute_buffer.elements);

    let data = match attribute.data_type {
        DataType::Float2 => VectorData::Vector2(read_vector_data::<_, f32, 2>(
            &mut reader,
            count,
            offset,
            stride,
        )?),
        DataType::Float3 => VectorData::Vector3(read_vector_data::<_, f32, 3>(
            &mut reader,
            count,
            offset,
            stride,
        )?),
        DataType::Float4 => VectorData::Vector4(read_vector_data::<_, f32, 4>(
            &mut reader,
            count,
            offset,
            stride,
        )?),
        DataType::HalfFloat2 => VectorData::Vector2(read_vector_data::<_, Half, 2>(
            &mut reader,
            count,
            offset,
            stride,
        )?),
        DataType::HalfFloat4 => VectorData::Vector4(read_vector_data::<_, Half, 4>(
            &mut reader,
            count,
            offset,
            stride,
        )?),
        DataType::Byte4 => {
            let mut elements = read_vector_data::<_, u8, 4>(&mut reader, count, offset, stride)?;
            // Normalize the values by converting from the range [0u8, 255u8] to [0.0f32, 1.0f32].
            for [x, y, z, w] in elements.iter_mut() {
                *x /= 255f32;
                *y /= 255f32;
                *z /= 255f32;
                *w /= 255f32;
            }
            VectorData::Vector4(elements)
        }
    };

    Ok(data)
}

fn calculate_offset_stride(
    attribute: &MeshAttribute,
    mesh_object: &MeshObject,
) -> Result<(u64, u64), AttributeError> {
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
        _ => Err(AttributeError::NoOffsetOrStride(attribute.index)),
    }?;
    Ok((offset, stride))
}

fn read_attributes(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    usage: AttributeUsage,
) -> Result<Vec<AttributeData>, AttributeError> {
    let mut attributes = Vec::new();
    for attribute in &get_attributes(mesh_object, usage) {
        let data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
        attributes.push(AttributeData {
            name: attribute.name.to_string(),
            data,
        })
    }
    Ok(attributes)
}

fn read_rigging_data(
    rigging_buffers: &[MeshRiggingGroup],
    mesh_object_name: &str,
    mesh_object_subindex: u64,
) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
    // Collect the influences for the corresponding mesh object.
    // The mesh object will likely only be listed once,
    // but check all the rigging groups just in case.
    let mut bone_influences = Vec::new();
    for rigging_group in rigging_buffers.iter().filter(|r| {
        r.mesh_object_name.to_str() == Some(mesh_object_name)
            && r.mesh_object_sub_index == mesh_object_subindex
    }) {
        bone_influences.extend(read_influences(rigging_group)?);
    }

    Ok(bone_influences)
}

/// A collection of vertex weights for all the vertices influenced by a bone.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct BoneInfluence {
    pub bone_name: String,
    pub vertex_weights: Vec<VertexWeight>,
}

/// The data associated with a [Mesh] file.
/// Supported versions are 1.8, 1.9, and 1.10.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct MeshData {
    pub major_version: u16,
    pub minor_version: u16,
    pub objects: Vec<MeshObjectData>,
}

impl SsbhData for MeshData {
    type WriteError = MeshError;

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Mesh::from_file(path)?.try_into()
    }

    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        Mesh::read(reader)?.try_into()
    }

    fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> Result<(), MeshError> {
        let mesh = create_mesh(self)?;
        mesh.write(writer)?;
        Ok(())
    }

    fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), MeshError> {
        let mesh = create_mesh(self)?;
        mesh.write_to_file(path)?;
        Ok(())
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
        Ok(Self {
            major_version: mesh.major_version,
            minor_version: mesh.minor_version,
            objects: read_mesh_objects(mesh)?,
        })
    }
}

/// The data associated with a [MeshObject].
///
/// Vertex attribute data is stored in collections of [AttributeData] grouped by usage.
/// Each [AttributeData] will have its data indexed by [vertex_indices](struct.MeshAttributeV10.html.#structfield.vertex_indices),
/// so all [data](struct.AttributeData.html#structfield.data) should have the number of elements.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct MeshObjectData {
    /// The name of this object.
    pub name: String,
    /// An additional identifier to differentiate multiple [MeshObjectData] with the same name.
    pub sub_index: u64,
    /// The name of the parent bone. The empty string represents no parent for mesh objects that are not single bound.
    pub parent_bone_name: String,
    /// Vertex indices for the data for all [AttributeData] for this [MeshObjectData].
    pub vertex_indices: Vec<u32>,
    pub positions: Vec<AttributeData>,
    pub normals: Vec<AttributeData>,
    pub binormals: Vec<AttributeData>,
    pub tangents: Vec<AttributeData>,
    pub texture_coordinates: Vec<AttributeData>,
    pub color_sets: Vec<AttributeData>,
    /// Vertex weights grouped by bone name.
    /// Each vertex will likely be influenced by at most 4 bones, but the format doesn't enforce this.
    /// For single bound objects, [bone_influences](#structfield.bone_influences) should be an empty list.
    pub bone_influences: Vec<BoneInfluence>,
}

/// Data corresponding to a named vertex attribute such as `"Position0"` or `"colorSet1"`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct AttributeData {
    pub name: String,
    pub data: VectorData,
}

/// The data for a vertex attribute.
///
/// The precision when saving is inferred based on supported data types for the version specified in the [MeshData].
/// For example, position attributes will prefer the highest available precision, and color sets will prefer the lowest available precision.
/// *The data type selected for saving may change between releases but will always retain the specified component count such as [VectorData::Vector2] vs [VectorData::Vector4].*
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorData {
    Vector2(Vec<[f32; 2]>),
    Vector3(Vec<[f32; 3]>),
    Vector4(Vec<[f32; 4]>),
}

impl VectorData {
    /// The number of vectors.
    /**
    ```rust
    # use ssbh_data::mesh_data::VectorData;
    let data = VectorData::Vector2(vec![[0f32, 1f32], [0f32, 1f32], [0f32, 1f32]]);
    assert_eq!(3, data.len());
    ```
    */
    pub fn len(&self) -> usize {
        match self {
            VectorData::Vector2(v) => v.len(),
            VectorData::Vector3(v) => v.len(),
            VectorData::Vector4(v) => v.len(),
        }
    }

    /// Returns `true` if there are no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pads the data to 4 components per vector with a specified w component.
    /// This includes replacing the w component for [VectorData::Vector4].
    /**
    ```rust
    # use ssbh_data::mesh_data::VectorData;
    let data2 = VectorData::Vector2(vec![[1.0, 2.0]]);
    assert_eq!(vec![[1.0, 2.0, 0.0, 4.0]], data2.to_vec4_with_w(4.0));

    let data3 = VectorData::Vector3(vec![[1.0, 2.0, 3.0]]);
    assert_eq!(vec![[1.0, 2.0, 3.0, 4.0]], data3.to_vec4_with_w(4.0));

    let data4 = VectorData::Vector4(vec![[1.0, 2.0, 3.0, 5.0]]);
    assert_eq!(vec![[1.0, 2.0, 3.0, 4.0]], data4.to_vec4_with_w(4.0));
    ```
     */
    pub fn to_vec4_with_w(&self, w: f32) -> Vec<[f32; 4]> {
        // Allow conversion to homogeneous coordinates by specifying the w component.
        match self {
            VectorData::Vector2(data) => data.iter().map(|[x, y]| [*x, *y, 0f32, w]).collect(),
            VectorData::Vector3(data) => data.iter().map(|[x, y, z]| [*x, *y, *z, w]).collect(),
            VectorData::Vector4(data) => data.iter().map(|[x, y, z, _]| [*x, *y, *z, w]).collect(),
        }
    }

    fn to_glam_vec3a(&self) -> Vec<geometry_tools::glam::Vec3A> {
        match self {
            VectorData::Vector2(data) => data
                .iter()
                .map(|[x, y]| geometry_tools::glam::Vec3A::new(*x, *y, 0f32))
                .collect(),
            VectorData::Vector3(data) => data
                .iter()
                .map(|[x, y, z]| geometry_tools::glam::Vec3A::new(*x, *y, *z))
                .collect(),
            VectorData::Vector4(data) => data
                .iter()
                .map(|[x, y, z, _]| geometry_tools::glam::Vec3A::new(*x, *y, *z))
                .collect(),
        }
    }

    fn to_glam_vec4_with_w(&self, w: f32) -> Vec<geometry_tools::glam::Vec4> {
        // Allow conversion to homogeneous coordinates by specifying the w component.
        match self {
            VectorData::Vector2(data) => data
                .iter()
                .map(|[x, y]| geometry_tools::glam::Vec4::new(*x, *y, 0f32, w))
                .collect(),
            VectorData::Vector3(data) => data
                .iter()
                .map(|[x, y, z]| geometry_tools::glam::Vec4::new(*x, *y, *z, w))
                .collect(),
            VectorData::Vector4(data) => data
                .iter()
                .map(|[x, y, z, _]| geometry_tools::glam::Vec4::new(*x, *y, *z, w))
                .collect(),
        }
    }
}

fn read_mesh_objects(mesh: &Mesh) -> Result<Vec<MeshObjectData>, Box<dyn Error>> {
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
            read_rigging_data(&mesh.rigging_buffers.elements, &name, mesh_object.sub_index)?;

        let data = MeshObjectData {
            name,
            sub_index: mesh_object.sub_index,
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
        };

        mesh_objects.push(data);
    }

    Ok(mesh_objects)
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MeshVersion {
    Version110,
    Version108,
    Version109,
}

fn create_mesh(data: &MeshData) -> Result<Mesh, MeshError> {
    let version = match (data.major_version, data.minor_version) {
        (1, 10) => Ok(MeshVersion::Version110),
        (1, 8) => Ok(MeshVersion::Version108),
        (1, 9) => Ok(MeshVersion::Version109),
        _ => Err(MeshError::UnsupportedVersion {
            major_version: data.major_version,
            minor_version: data.minor_version,
        }),
    }?;

    let mesh_vertex_data = create_mesh_objects(version, &data.objects)?;

    // TODO: It might be more efficient to reuse the data for mesh object bounding or reuse the generated points.
    let all_positions: Vec<geometry_tools::glam::Vec3A> = data
        .objects
        .iter()
        .map(|o| match o.positions.first() {
            Some(attribute) => attribute.data.to_glam_vec3a(),
            None => Vec::new(),
        })
        .flatten()
        .collect();

    let mesh = Mesh {
        major_version: data.major_version,
        minor_version: data.minor_version,
        model_name: "".into(),
        bounding_info: calculate_bounding_info(&all_positions),
        unk1: 0,
        objects: mesh_vertex_data.mesh_objects.into(),
        // There are always at least 4 buffer entries even if only 2 are used.
        buffer_sizes: mesh_vertex_data
            .vertex_buffers
            .iter()
            .map(|b| b.len() as u32)
            .pad_using(4, |_| 0u32)
            .collect::<Vec<u32>>()
            .into(),
        polygon_index_size: mesh_vertex_data.index_buffer.len() as u64,
        vertex_buffers: mesh_vertex_data
            .vertex_buffers
            .into_iter()
            .map(SsbhByteBuffer::new)
            .collect::<Vec<SsbhByteBuffer>>()
            .into(),
        index_buffer: mesh_vertex_data.index_buffer.into(),
        rigging_buffers: create_rigging_buffers(version, &data.objects)?.into(),
    };

    Ok(mesh)
}

fn calculate_max_influences(influences: &[BoneInfluence]) -> usize {
    // TODO: Optimize this to use a vec.
    // This requires validating the vertex count ahead of time?

    // Find the number of influences for the vertex with the most influences.
    let mut influences_by_vertex = HashMap::new();
    for influence in influences {
        for weight in &influence.vertex_weights {
            // Assume influences are uniquely identified by their bone name.
            let entry = influences_by_vertex
                .entry(weight.vertex_index)
                .or_insert_with(HashSet::new);
            entry.insert(&influence.bone_name);
        }
    }

    influences_by_vertex
        .values()
        .map(|s| s.len())
        .max()
        .unwrap_or(0)
}

fn create_rigging_buffers(
    version: MeshVersion,
    object_data: &[MeshObjectData],
) -> std::io::Result<Vec<MeshRiggingGroup>> {
    let mut rigging_buffers = Vec::new();

    for mesh_object in object_data {
        // TODO: unk1 is sometimes set to 0 for singlebound mesh objects, which isn't currently preserved.
        let flags = RiggingFlags {
            max_influences: calculate_max_influences(&mesh_object.bone_influences) as u8,
            unk1: 1,
        };

        let mut buffers = Vec::new();
        for i in &mesh_object.bone_influences {
            let buffer = MeshBoneBuffer {
                bone_name: i.bone_name.clone().into(),
                data: create_vertex_weights(version, &i.vertex_weights)?,
            };
            buffers.push(buffer);
        }

        let buffer = MeshRiggingGroup {
            mesh_object_name: mesh_object.name.clone().into(),
            mesh_object_sub_index: mesh_object.sub_index,
            flags,
            buffers: buffers.into(),
        };

        rigging_buffers.push(buffer)
    }

    // Rigging buffers need to be sorted in ascending order by name and sub_index.
    // TODO: Using a default may impact sorting if mesh_object_name is a null offset.
    rigging_buffers.sort_by_key(|k| {
        (
            k.mesh_object_name.to_string_lossy(),
            k.mesh_object_sub_index,
        )
    });

    Ok(rigging_buffers)
}

fn create_vertex_weights(
    version: MeshVersion,
    vertex_weights: &[VertexWeight],
) -> std::io::Result<VertexWeights> {
    match version {
        MeshVersion::Version108 => {
            // Mesh version 1.8 uses an array of structs.
            let weights: Vec<VertexWeightV8> = vertex_weights
                .iter()
                .map(|v| VertexWeightV8 {
                    vertex_index: v.vertex_index,
                    vertex_weight: v.vertex_weight,
                })
                .collect();
            Ok(VertexWeights::VertexWeightsV8(weights.into()))
        }
        MeshVersion::Version109 => {
            // Mesh version 1.9 uses an array of structs.
            let weights: Vec<VertexWeightV8> = vertex_weights
                .iter()
                .map(|v| VertexWeightV8 {
                    vertex_index: v.vertex_index,
                    vertex_weight: v.vertex_weight,
                })
                .collect();
            Ok(VertexWeights::VertexWeightsV8(weights.into()))
        }
        MeshVersion::Version110 => {
            // Mesh version 1.10 uses a byte buffer.
            let mut bytes = Cursor::new(Vec::new());
            for weight in vertex_weights {
                bytes.write_all(&(weight.vertex_index as u16).to_le_bytes())?;
                bytes.write_all(&weight.vertex_weight.to_le_bytes())?;
            }
            Ok(VertexWeights::VertexWeightsV10(bytes.into_inner().into()))
        }
    }
}

fn get_size_in_bytes_v10(data_type: &AttributeDataTypeV10) -> usize {
    match data_type {
        AttributeDataTypeV10::Float3 => std::mem::size_of::<f32>() * 3,
        AttributeDataTypeV10::Byte4 => std::mem::size_of::<u8>() * 4,
        AttributeDataTypeV10::HalfFloat4 => std::mem::size_of::<f16>() * 4,
        AttributeDataTypeV10::HalfFloat2 => std::mem::size_of::<f16>() * 2,
        AttributeDataTypeV10::Float4 => std::mem::size_of::<f32>() * 4,
        AttributeDataTypeV10::Float2 => std::mem::size_of::<f32>() * 2,
    }
}

fn get_size_in_bytes_v8(data_type: &AttributeDataTypeV8) -> usize {
    match data_type {
        AttributeDataTypeV8::Float3 => std::mem::size_of::<f32>() * 3,
        AttributeDataTypeV8::HalfFloat4 => std::mem::size_of::<f16>() * 4,
        AttributeDataTypeV8::Float2 => std::mem::size_of::<f32>() * 2,
        AttributeDataTypeV8::Byte4 => std::mem::size_of::<u8>() * 4,
        AttributeDataTypeV8::Float4 => std::mem::size_of::<f32>() * 4,
    }
}

struct MeshVertexData {
    mesh_objects: Vec<MeshObject>,
    vertex_buffers: Vec<Vec<u8>>,
    index_buffer: Vec<u8>,
}

#[derive(Debug, PartialEq)]
enum VertexIndices {
    UnsignedInt(Vec<u32>),
    UnsignedShort(Vec<u16>),
}

fn create_attributes(
    data: &MeshObjectData,
    version: MeshVersion,
) -> ([(u32, AttributeBufferData); 4], MeshAttributes) {
    match version {
        MeshVersion::Version108 => create_attributes_v8(data),
        MeshVersion::Version109 => create_attributes_v9(data),
        MeshVersion::Version110 => create_attributes_v10(data),
    }
}

fn create_mesh_objects(
    version: MeshVersion,
    mesh_object_data: &[MeshObjectData],
) -> Result<MeshVertexData, MeshError> {
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
        let vertex_count = calculate_vertex_count(data)?;

        // Assume generated bounding data isn't critical if there are no points.
        // Use an empty list if there are no position attributes.
        let positions = match data.positions.first() {
            Some(attribute) => attribute.data.to_glam_vec3a(),
            None => Vec::new(),
        };

        // Most meshes use 16 bit integral types for vertex indices.
        // Check if a 32 bit type is needed just in case.
        let vertex_indices = convert_indices(&data.vertex_indices);
        let draw_element_type = match vertex_indices {
            VertexIndices::UnsignedInt(_) => DrawElementType::UnsignedInt,
            VertexIndices::UnsignedShort(_) => DrawElementType::UnsignedShort,
        };

        let vertex_buffer0_offset = buffer0.position();
        let vertex_buffer1_offset = buffer1.position();
        let vertex_buffer3_offset = buffer3.position();

        let (buffer_info, attributes) = create_attributes(data, version);
        // TODO: Find a cleaner way to write this.
        let stride0 = buffer_info[0].0;
        let stride1 = buffer_info[1].0;
        let stride2 = buffer_info[2].0;
        let stride3 = buffer_info[3].0;

        // TODO: How to test this?
        write_attributes(
            &buffer_info,
            &mut [&mut buffer0, &mut buffer1, &mut buffer2, &mut buffer3],
            &[
                vertex_buffer0_offset,
                vertex_buffer1_offset,
                vertex_buffer2_offset,
                vertex_buffer3_offset,
            ],
        )?;

        // Older mesh versions write 0's to this buffer despite not having any attributes referencing the data.
        match version {
            MeshVersion::Version108 | MeshVersion::Version109 => {
                buffer2.write_all(&vec![0u8; stride2 as usize * vertex_count])?;
            }
            _ => (),
        }

        // TODO: Investigate default values for unknown values.
        let mesh_object = MeshObject {
            name: data.name.clone().into(),
            sub_index: data.sub_index,
            parent_bone_name: data.parent_bone_name.clone().into(),
            vertex_count: vertex_count as u32,
            vertex_index_count: data.vertex_indices.len() as u32,
            unk2: 3,
            vertex_buffer0_offset: vertex_buffer0_offset as u32,
            vertex_buffer1_offset: vertex_buffer1_offset as u32,
            vertex_buffer2_offset: vertex_buffer2_offset as u32,
            vertex_buffer3_offset: vertex_buffer3_offset as u32,
            stride0,
            stride1,
            stride2,
            stride3,
            index_buffer_offset: index_buffer.position() as u32,
            unk8: 4, // TODO: index stride?
            draw_element_type,
            rigging_type: if data.bone_influences.is_empty() {
                RiggingType::SingleBound
            } else {
                RiggingType::Weighted
            },
            unk11: 0,
            unk12: 0,
            bounding_info: calculate_bounding_info(&positions),
            attributes,
        };

        write_vertex_indices(&vertex_indices, &mut index_buffer)?;

        // Mesh 1.10 in Smash Ultimate still calculates an offset despite not saving the buffer data.
        vertex_buffer2_offset = match version {
            MeshVersion::Version110 => vertex_buffer2_offset + vertex_count as u64 * 32,
            _ => buffer2.position(),
        };

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

fn calculate_vertex_count(data: &MeshObjectData) -> Result<usize, MeshError> {
    // Make sure all the attributes have the same length.
    // This ensures the vertex indices do not cause any out of bounds accesses.
    let sizes: Vec<_> = data
        .positions
        .iter()
        .map(|a| a.data.len())
        .chain(data.normals.iter().map(|a| a.data.len()))
        .chain(data.binormals.iter().map(|a| a.data.len()))
        .chain(data.tangents.iter().map(|a| a.data.len()))
        .chain(data.texture_coordinates.iter().map(|a| a.data.len()))
        .chain(data.color_sets.iter().map(|a| a.data.len()))
        .collect();

    if sizes.iter().all_equal() {
        // TODO: Does zero length cause issues in game?
        match sizes.first() {
            Some(size) => Ok(*size),
            None => Ok(0),
        }
    } else {
        Err(MeshError::AttributeDataLengthMismatch)
    }
}

fn convert_indices(indices: &[u32]) -> VertexIndices {
    // Try and convert the vertex indices to a smaller type.
    let u16_indices: Result<Vec<u16>, _> = indices.iter().map(|i| u16::try_from(*i)).collect();
    match u16_indices {
        Ok(indices) => VertexIndices::UnsignedShort(indices),
        Err(_) => VertexIndices::UnsignedInt(indices.into()),
    }
}

fn transform_inner(data: &VectorData, transform: &[[f32; 4]; 4], w: f32) -> VectorData {
    let mut points = data.to_glam_vec4_with_w(w);

    // Transform is assumed to be row-major.
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
/// Transform is assumed to be in row-major order.
/// The elements are treated as points in homogeneous coordinates by temporarily setting the 4th component to `1.0f32`.
/// The returned result has the same component count as `data`.
/// For [VectorData::Vector4], the 4th component is preserved for the returned result.
/**
```rust
# use ssbh_data::mesh_data::{VectorData, AttributeData, MeshObjectData, transform_points};
# let mesh_object_data = MeshObjectData {
#     name: "abc".into(),
#     sub_index: 0,
#     parent_bone_name: "".into(),
#     vertex_indices: Vec::new(),
#     positions: vec![AttributeData {
#         name: "Position0".into(),
#         data: VectorData::Vector3(Vec::new())
#     }],
#     normals: Vec::new(),
#     binormals: Vec::new(),
#     tangents: Vec::new(),
#     texture_coordinates: Vec::new(),
#     color_sets: Vec::new(),
#     bone_influences: Vec::new(),
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
/// Transform is assumed to be in row-major order.
/// The elements are treated as vectors in homogeneous coordinates by temporarily setting the 4th component to `0.0f32`.
/// The returned result has the same component count as `data`.
/// For [VectorData::Vector4], the 4th component is preserved for the returned result.
/**
```rust
# use ssbh_data::mesh_data::{VectorData, AttributeData, MeshObjectData, transform_vectors};
# let mesh_object_data = MeshObjectData {
#     name: "abc".into(),
#     sub_index: 0,
#     parent_bone_name: "".into(),
#     vertex_indices: Vec::new(),
#     positions: Vec::new(),
#     normals: vec![AttributeData {
#         name: "Normal0".into(),
#         data: VectorData::Vector3(Vec::new())
#     }],
#     binormals: Vec::new(),
#     tangents: Vec::new(),
#     texture_coordinates: Vec::new(),
#     color_sets: Vec::new(),
#     bone_influences: Vec::new(),
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

fn calculate_bounding_info(positions: &[geometry_tools::glam::Vec3A]) -> BoundingInfo {
    // Calculate bounding info based on the current points.
    let (sphere_center, sphere_radius) =
        geometry_tools::bounding::calculate_bounding_sphere_from_points(positions);
    let (aabb_min, aabb_max) = geometry_tools::bounding::calculate_aabb_from_points(positions);

    // TODO: Compute a better oriented bounding box.
    let obb_center = aabb_min.add(aabb_max).div(2f32);
    let obb_size = aabb_max.sub(aabb_min).div(2f32);

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

fn read_influences(rigging_group: &MeshRiggingGroup) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
    let mut bone_influences = Vec::new();
    for buffer in &rigging_group.buffers.elements {
        let bone_name = buffer
            .bone_name
            .to_str()
            .ok_or("Failed to read bone name.")?;

        // TODO: Find a way to test reading influence data.
        let influences = match &buffer.data {
            VertexWeights::VertexWeightsV8(v) | VertexWeights::VertexWeightsV9(v) => v
                .elements
                .iter()
                .map(|influence| VertexWeight {
                    vertex_index: influence.vertex_index,
                    vertex_weight: influence.vertex_weight,
                })
                .collect(),
            VertexWeights::VertexWeightsV10(v) => {
                // Version 1.10 uses a byte buffer instead of storing an array of vertex weights directly.
                // The vertex index now uses 32 bits instead of 16 bits.
                read_vertex_weights_v9(v)
            }
        };

        let bone_influence = BoneInfluence {
            bone_name: bone_name.to_string(),
            vertex_weights: influences,
        };
        bone_influences.push(bone_influence);
    }

    Ok(bone_influences)
}

fn read_vertex_weights_v9(v: &SsbhByteBuffer) -> Vec<VertexWeight> {
    let mut elements = Vec::new();
    let mut reader = Cursor::new(&v.elements);
    while let Ok(influence) = reader.read_le::<ssbh_lib::formats::mesh::VertexWeightV10>() {
        elements.push(VertexWeight {
            vertex_index: influence.vertex_index as u32,
            vertex_weight: influence.vertex_weight,
        });
    }
    elements
}

struct MeshAttribute {
    pub name: String,
    pub index: u64,
    pub offset: u64,
    pub data_type: DataType,
}

impl From<&MeshAttributeV9> for MeshAttribute {
    fn from(a: &MeshAttributeV9) -> Self {
        MeshAttribute {
            name: get_attribute_name_v9(a).unwrap_or("").to_string(),
            index: a.buffer_index as u64,
            offset: a.buffer_offset as u64,
            data_type: a.data_type.into(),
        }
    }
}

impl From<&MeshAttributeV10> for MeshAttribute {
    fn from(a: &MeshAttributeV10) -> Self {
        MeshAttribute {
            name: get_attribute_name_v10(a).unwrap_or("").to_string(),
            index: a.buffer_index as u64,
            offset: a.buffer_offset as u64,
            data_type: a.data_type.into(),
        }
    }
}

impl From<&MeshAttributeV8> for MeshAttribute {
    fn from(a: &MeshAttributeV8) -> Self {
        // Version 1.8 doesn't have names.
        // Generate a name based on conventions for Smash Ultimate and New Pokemon Snap.
        let name = match a.usage {
            AttributeUsageV8::Position => format!("Position{}", a.sub_index),
            AttributeUsageV8::Normal => format!("Normal{}", a.sub_index),
            AttributeUsageV8::Tangent => format!("Tangent{}", a.sub_index),
            AttributeUsageV8::TextureCoordinate => format!("TextureCoordinate{}", a.sub_index),
            AttributeUsageV8::ColorSet => format!("colorSet{}", a.sub_index),
        };

        MeshAttribute {
            name,
            index: a.buffer_index as u64,
            offset: a.buffer_offset as u64,
            data_type: a.data_type.into(),
        }
    }
}

fn get_attributes(mesh_object: &MeshObject, usage: AttributeUsage) -> Vec<MeshAttribute> {
    match &mesh_object.attributes {
        MeshAttributes::AttributesV8(attributes) => attributes
            .elements
            .iter()
            .filter(|a| AttributeUsage::from(a.usage) == usage)
            .map(|a| a.into())
            .collect(),
        MeshAttributes::AttributesV10(attributes) => attributes
            .elements
            .iter()
            .filter(|a| AttributeUsage::from(a.usage) == usage)
            .map(|a| a.into())
            .collect(),
        MeshAttributes::AttributesV9(attributes) => attributes
            .elements
            .iter()
            .filter(|a| AttributeUsage::from(a.usage) == usage)
            .map(|a| a.into())
            .collect(),
    }
}

fn get_attribute_name_v9(attribute: &MeshAttributeV9) -> Option<&str> {
    attribute.attribute_names.elements.get(0)?.to_str()
}

fn get_attribute_name_v10(attribute: &MeshAttributeV10) -> Option<&str> {
    attribute.attribute_names.elements.get(0)?.to_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;

    #[test]
    fn attribute_from_attribute_v10() {
        let attribute_v10 = MeshAttributeV10 {
            usage: AttributeUsageV9::Normal,
            data_type: AttributeDataTypeV10::HalfFloat2,
            buffer_index: 2,
            buffer_offset: 10,
            sub_index: 3,
            name: "custom_name".into(),
            attribute_names: vec!["name1".into()].into(),
        };

        let attribute: MeshAttribute = (&attribute_v10).into();
        assert_eq!("name1", attribute.name);
        assert_eq!(DataType::HalfFloat2, attribute.data_type);
        assert_eq!(2, attribute.index);
        assert_eq!(10, attribute.offset);
    }

    #[test]
    fn attribute_from_attribute_v8() {
        let attribute_v8 = MeshAttributeV8 {
            usage: AttributeUsageV8::Normal,
            data_type: AttributeDataTypeV8::Float2,
            buffer_index: 1,
            buffer_offset: 8,
            sub_index: 3,
        };

        let attribute: MeshAttribute = (&attribute_v8).into();
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

        let result = create_vertex_weights(MeshVersion::Version108, &weights).unwrap();
        match result {
            VertexWeights::VertexWeightsV8(v) => {
                assert_eq!(2, v.elements.len());

                assert_eq!(0, v.elements[0].vertex_index);
                assert_eq!(0.0f32, v.elements[0].vertex_weight);

                assert_eq!(1, v.elements[1].vertex_index);
                assert_eq!(1.0f32, v.elements[1].vertex_weight);
            }
            _ => panic!("Invalid version"),
        }
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

        let result = create_vertex_weights(MeshVersion::Version110, &weights).unwrap();
        match result {
            VertexWeights::VertexWeightsV10(v) => {
                assert_eq!(&v.elements[..], &hex!("0000 00000000 01000 000803f"));
            }
            _ => panic!("Invalid version"),
        }
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
        assert_eq!(4, get_size_in_bytes_v10(&AttributeDataTypeV10::Byte4));
        assert_eq!(8, get_size_in_bytes_v10(&AttributeDataTypeV10::Float2));
        assert_eq!(12, get_size_in_bytes_v10(&AttributeDataTypeV10::Float3));
        assert_eq!(16, get_size_in_bytes_v10(&AttributeDataTypeV10::Float4));
        assert_eq!(4, get_size_in_bytes_v10(&AttributeDataTypeV10::HalfFloat2));
        assert_eq!(8, get_size_in_bytes_v10(&AttributeDataTypeV10::HalfFloat4));
    }

    #[test]
    fn size_in_bytes_attributes_v8() {
        assert_eq!(4, get_size_in_bytes_v8(&AttributeDataTypeV8::Byte4));
        assert_eq!(8, get_size_in_bytes_v8(&AttributeDataTypeV8::Float2));
        assert_eq!(12, get_size_in_bytes_v8(&AttributeDataTypeV8::Float3));
        assert_eq!(16, get_size_in_bytes_v8(&AttributeDataTypeV8::Float4));
        assert_eq!(8, get_size_in_bytes_v8(&AttributeDataTypeV8::HalfFloat4));
    }

    #[test]
    fn max_influences_no_bones() {
        assert_eq!(0, calculate_max_influences(&[]));
    }

    #[test]
    fn max_influences_one_bone_no_weights() {
        let influences = vec![BoneInfluence {
            bone_name: "a".to_string(),
            vertex_weights: Vec::new(),
        }];
        assert_eq!(0, calculate_max_influences(&influences));
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
        assert_eq!(1, calculate_max_influences(&influences));
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

        assert_eq!(3, calculate_max_influences(&influences));
    }

    #[test]
    fn create_empty_mesh_1_10() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 10,
            objects: Vec::new(),
        })
        .unwrap();
        assert_eq!(1, mesh.major_version);
        assert_eq!(10, mesh.minor_version);
        assert!(mesh.objects.elements.is_empty());
        assert!(mesh.rigging_buffers.elements.is_empty());
        assert!(mesh.index_buffer.elements.is_empty());
    }

    #[test]
    fn create_empty_mesh_1_8() {
        let mesh = create_mesh(&MeshData {
            major_version: 1,
            minor_version: 8,
            objects: Vec::new(),
        })
        .unwrap();

        assert_eq!(1, mesh.major_version);
        assert_eq!(8, mesh.minor_version);
        assert!(mesh.objects.elements.is_empty());
        assert!(mesh.rigging_buffers.elements.is_empty());
        assert!(mesh.index_buffer.elements.is_empty());
    }

    #[test]
    fn create_empty_mesh_v_1_9() {
        create_mesh(&MeshData {
            major_version: 1,
            minor_version: 9,
            objects: Vec::new(),
        })
        .unwrap();
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
            Err(MeshError::UnsupportedVersion {
                major_version: 2,
                minor_version: 301
            })
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
        let mesh_object = MeshObject {
            name: String::new().into(),
            sub_index: 0,
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
            rigging_type: RiggingType::SingleBound,
            unk11: 0,
            unk12: 0,
            bounding_info: BoundingInfo::default(),
            attributes: MeshAttributes::AttributesV10(Vec::new().into()),
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
        assert!(matches!(result, Err(AttributeError::NoOffsetOrStride(4))));
    }
}
