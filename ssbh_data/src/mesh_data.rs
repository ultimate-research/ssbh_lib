use binread::{io::Cursor, BinRead};
use binread::{BinReaderExt, BinResult};
use half::f16;
use itertools::Itertools;
use ssbh_lib::formats::mesh::RiggingType;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataType, AttributeDataTypeV8, AttributeUsageV10, AttributeUsageV8,
        DrawElementType, Mesh, MeshAttributeV10, MeshAttributeV8, MeshAttributes, MeshBoneBuffer,
        MeshObject, MeshRiggingGroup, RiggingFlags, VertexWeightV8, VertexWeights,
    },
    SsbhByteBuffer,
};
use ssbh_lib::{Half, Matrix3x3, SsbhArray, Vector3};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::ops::{Add, Div, Sub};
use std::{error::Error, io::Write, ops::Mul};

use crate::{read_data, read_vector_data};

#[derive(Debug, PartialEq)]
pub enum DataType {
    Float2,
    Float3,
    Float4,
    HalfFloat2,
    HalfFloat4,
    Byte4,
}

#[derive(Debug, PartialEq)]
pub enum AttributeUsage {
    Position,
    Normal,
    Binormal,
    Tangent,
    TextureCoordinate,
    ColorSet,
}

/// An error while reading mesh attribute data.
pub enum AttributeError {
    /// Attempted to read from a nonexistent buffer.
    InvalidBufferIndex(u64),

    /// Failed to find the offset or stride in bytes for the given buffer index.
    NoOffsetOrStride(u64),

    /// An error occurred while reading the data from the buffer.
    Io(std::io::Error),

    /// An error occurred while reading the data from the buffer.
    BinRead(binread::error::Error),
}

impl std::error::Error for AttributeError {}

impl From<std::io::Error> for AttributeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<binread::error::Error> for AttributeError {
    fn from(e: binread::error::Error) -> Self {
        match e {
            binread::Error::Io(io) => Self::Io(io),
            _ => Self::BinRead(e),
        }
    }
}

impl std::fmt::Display for AttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::fmt::Debug for AttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeError::InvalidBufferIndex(index) => {
                write!(f, "No buffer found for index {}.", index)
            }
            AttributeError::NoOffsetOrStride(index) => write!(
                f,
                "Found index {}. Buffer indices higher than 1 are not supported.",
                index
            ),
            AttributeError::Io(err) => write!(f, "IO Error: {:?}", err),
            AttributeError::BinRead(err) => write!(f, "BinRead Error: {:?}", err),
        }
    }
}

impl From<AttributeUsageV10> for AttributeUsage {
    fn from(a: AttributeUsageV10) -> Self {
        match a {
            AttributeUsageV10::Position => Self::Position,
            AttributeUsageV10::Normal => Self::Normal,
            AttributeUsageV10::Binormal => Self::Binormal,
            AttributeUsageV10::Tangent => Self::Tangent,
            AttributeUsageV10::TextureCoordinate => Self::TextureCoordinate,
            AttributeUsageV10::ColorSet => Self::ColorSet,
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

#[derive(BinRead, Debug, Clone)]
pub struct VertexWeight {
    pub vertex_index: u32,
    pub vertex_weight: f32,
}

impl From<AttributeDataType> for DataType {
    fn from(value: AttributeDataType) -> Self {
        match value {
            AttributeDataType::Float3 => Self::Float3,
            AttributeDataType::Byte4 => Self::Byte4,
            AttributeDataType::HalfFloat4 => Self::HalfFloat4,
            AttributeDataType::HalfFloat2 => Self::HalfFloat2,
            AttributeDataType::Float4 => Self::Float4,
            AttributeDataType::Float2 => Self::Float2,
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
        }
    }
}

fn read_vertex_indices(mesh_index_buffer: &[u8], mesh_object: &MeshObject) -> BinResult<Vec<u32>> {
    // Use u32 regardless of the actual data type to simplify conversions.
    let count = mesh_object.vertex_index_count as usize;
    let offset = mesh_object.index_buffer_offset as u64;
    let mut reader = Cursor::new(mesh_index_buffer);
    match mesh_object.draw_element_type {
        DrawElementType::UnsignedShort => read_data::<_, u16, u32>(
            &mut reader,
            count,
            offset,
            std::mem::size_of::<u16>() as u64,
        ),
        DrawElementType::UnsignedInt => read_data::<_, u32, u32>(
            &mut reader,
            count,
            offset,
            std::mem::size_of::<u32>() as u64,
        ),
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
        _ => Err(AttributeError::NoOffsetOrStride(attribute.index)),
    }?;
    Ok((offset, stride))
}

/// Read data for all attributes of the given `usage` for `mesh_object`.
pub fn read_attributes_by_usage(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    usage: AttributeUsage,
) -> Result<Vec<AttributeData>, Box<dyn Error>> {
    let mut attributes = Vec::new();
    for attribute in &get_attributes(&mesh_object, usage) {
        let data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
        attributes.push(AttributeData {
            name: attribute.name.to_string(),
            data,
        })
    }
    Ok(attributes)
}

/// Returns all the texture coordinate attributes for the specified `mesh_object`.
/// The v coordinate is transformed to `1.0 - v` if `flip_vertical` is true.
pub fn read_texture_coordinates(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    flip_vertical: bool,
) -> Result<Vec<AttributeData>, Box<dyn Error>> {
    let mut attributes = Vec::new();
    for attribute in &get_attributes(&mesh_object, AttributeUsage::TextureCoordinate) {
        let mut data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;

        if flip_vertical {
            flip_y(&mut data);
        }

        attributes.push(AttributeData {
            name: attribute.name.to_string(),
            data,
        });
    }

    Ok(attributes)
}

fn flip_y(data: &mut VectorData) {
    match data {
        VectorData::Vector2(v) => {
            for [_, y] in v.iter_mut() {
                *y = 1.0 - *y;
            }
        }
        VectorData::Vector3(v) => {
            for [_, y, _] in v.iter_mut() {
                *y = 1.0 - *y;
            }
        }
        VectorData::Vector4(v) => {
            for [_, y, _, _] in v.iter_mut() {
                *y = 1.0 - *y;
            }
        }
    }
}

/// Returns all the colorset attributes for the specified `mesh_object`.
/// [u8] values are converted to [f32] by normalizing to the range 0.0 to 1.0.
pub fn read_colorsets(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<AttributeData>, Box<dyn Error>> {
    read_attributes_by_usage(mesh, mesh_object, AttributeUsage::ColorSet)
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
        r.mesh_object_name.get_string() == Some(mesh_object_name)
            && r.mesh_object_sub_index == mesh_object_subindex
    }) {
        bone_influences.extend(read_influences(&rigging_group)?);
    }

    Ok(bone_influences)
}

#[derive(Debug, Clone)]
pub struct BoneInfluence {
    pub bone_name: String,
    pub vertex_weights: Vec<VertexWeight>,
}

#[derive(Debug, Clone)]
pub struct MeshObjectData {
    pub name: String,
    pub sub_index: u64,
    pub parent_bone_name: String,
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

#[derive(Debug, Clone)]
pub struct AttributeData {
    pub name: String,
    pub data: VectorData,
}

#[derive(Debug, Clone)]
pub enum VectorData {
    Vector2(Vec<[f32; 2]>),
    Vector3(Vec<[f32; 3]>),
    Vector4(Vec<[f32; 4]>),
}

pub fn read_mesh_objects(mesh: &Mesh) -> Result<Vec<MeshObjectData>, Box<dyn Error>> {
    let mut mesh_objects = Vec::new();

    for mesh_object in &mesh.objects.elements {
        let name = mesh_object.name.get_string().unwrap_or("").to_string();

        let indices = read_vertex_indices(&mesh.index_buffer.elements, &mesh_object)?;
        let positions = read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Position)?;
        let normals = read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Normal)?;
        let tangents = read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Tangent)?;
        let binormals = read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Binormal)?;
        let texture_coordinates = read_texture_coordinates(&mesh, &mesh_object, true)?;
        let color_sets = read_colorsets(&mesh, &mesh_object)?;
        let bone_influences =
            read_rigging_data(&mesh.rigging_buffers.elements, &name, mesh_object.sub_index)?;

        let data = MeshObjectData {
            name,
            sub_index: mesh_object.sub_index,
            parent_bone_name: mesh_object
                .parent_bone_name
                .get_string()
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

pub fn create_mesh(
    major_version: u16,
    minor_version: u16,
    object_data: &[MeshObjectData],
) -> Result<Mesh, Box<dyn Error>> {
    let mesh_vertex_data = create_mesh_objects(major_version, minor_version, object_data)?;

    // TODO: It might be more efficient to reuse the data for mesh object bounding or reuse the generated points.
    let all_positions: Vec<geometry_tools::glam::Vec3A> = object_data
        .iter()
        .map(|o| match o.positions.first() {
            Some(attribute) => vector_data_to_glam(&attribute.data),
            None => Vec::new(),
        })
        .flatten()
        .collect();

    let mesh = Mesh {
        major_version,
        minor_version,
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
            .map(|b| SsbhByteBuffer::new(b))
            .collect::<Vec<SsbhByteBuffer>>()
            .into(),
        index_buffer: mesh_vertex_data.index_buffer.into(),
        rigging_buffers: create_rigging_buffers(major_version, minor_version, object_data)?.into(),
    };

    Ok(mesh)
}

fn calculate_max_influences(influences: &[BoneInfluence]) -> usize {
    // Find the number of influences for the vertex with the most influences.
    let mut influences_by_vertex = HashMap::new();
    for influence in influences {
        for weight in &influence.vertex_weights {
            // Assume influences are uniquely identified by their bone name.
            let entry = influences_by_vertex
                .entry(weight.vertex_index)
                .or_insert(HashSet::new());
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
    major_version: u16,
    minor_version: u16,
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
                data: create_vertex_weights(major_version, minor_version, &i.vertex_weights)?,
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
            k.mesh_object_name.get_string().unwrap_or("").to_string(),
            k.mesh_object_sub_index,
        )
    });

    Ok(rigging_buffers)
}

fn create_vertex_weights(
    major_version: u16,
    minor_version: u16,
    vertex_weights: &[VertexWeight],
) -> std::io::Result<VertexWeights> {
    match (major_version, minor_version) {
        (1, 8) => {
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
        (1, 10) => {
            // Mesh version 1.10 use a byte buffer.
            let mut bytes = Cursor::new(Vec::new());
            for weight in vertex_weights {
                bytes.write_all(&(weight.vertex_index as u16).to_le_bytes())?;
                bytes.write_all(&weight.vertex_weight.to_le_bytes())?;
            }
            Ok(VertexWeights::VertexWeightsV10(bytes.into_inner().into()))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Unsupported MESH version {}.{}",
                major_version, minor_version
            ),
        )),
    }
}

fn get_size_in_bytes_v10(data_type: &AttributeDataType) -> usize {
    match data_type {
        AttributeDataType::Float3 => std::mem::size_of::<f32>() * 3,
        AttributeDataType::Byte4 => std::mem::size_of::<u8>() * 4,
        AttributeDataType::HalfFloat4 => std::mem::size_of::<f16>() * 4,
        AttributeDataType::HalfFloat2 => std::mem::size_of::<f16>() * 2,
        AttributeDataType::Float4 => std::mem::size_of::<f32>() * 4,
        AttributeDataType::Float2 => std::mem::size_of::<f32>() * 2,
    }
}

fn get_size_in_bytes_v8(data_type: &AttributeDataTypeV8) -> usize {
    match data_type {
        AttributeDataTypeV8::Float3 => std::mem::size_of::<f32>() * 3,
        AttributeDataTypeV8::HalfFloat4 => std::mem::size_of::<f16>() * 4,
        AttributeDataTypeV8::Float2 => std::mem::size_of::<f32>() * 2,
        AttributeDataTypeV8::Byte4 => std::mem::size_of::<u8>() * 4,
    }
}

// TODO: This is almost identical version 1.10.
// Find a way to share code.
fn add_attribute_v8(
    attributes: &mut Vec<MeshAttributeV8>,
    current_stride: &mut u32,
    buffer_index: u32,
    sub_index: u32,
    usage: AttributeUsageV8,
    data_type: AttributeDataTypeV8,
) {
    let attribute = MeshAttributeV8 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: *current_stride,
        sub_index,
    };

    *current_stride += get_size_in_bytes_v8(&attribute.data_type) as u32;
    attributes.push(attribute);
}

fn add_attribute_v10(
    attributes: &mut Vec<MeshAttributeV10>,
    current_stride: &mut u32,
    name: &str,
    attribute_array_name: &str,
    buffer_index: u32,
    sub_index: u64,
    usage: AttributeUsageV10,
    data_type: AttributeDataType,
) {
    let attribute = MeshAttributeV10 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: *current_stride,
        sub_index,
        name: name.into(),
        attribute_names: SsbhArray::new(vec![attribute_array_name.into()]),
    };

    *current_stride += get_size_in_bytes_v10(&attribute.data_type) as u32;
    attributes.push(attribute);
}

fn create_attributes(
    data: &MeshObjectData,
    version_major: u16,
    version_minor: u16,
) -> (u32, u32, MeshAttributes) {
    match (version_major, version_minor) {
        (1, 8) => create_attributes_v8(data),
        (1, 10) => create_attributes_v10(data),
        _ => panic!(
            "Unsupported MESH version {}.{}",
            version_major, version_minor
        ),
    }
}

fn add_attributes_v8(
    attributes: &mut Vec<MeshAttributeV8>,
    attributes_to_add: &[AttributeData],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV8,
) {
    for (i, attribute) in attributes_to_add.iter().enumerate() {
        let data_type = infer_data_type_v8(attribute, usage);

        add_attribute_v8(
            attributes,
            current_stride,
            buffer_index,
            i as u32,
            usage,
            data_type,
        );
    }
}

fn infer_data_type_v8(attribute: &AttributeData, usage: AttributeUsageV8) -> AttributeDataTypeV8 {
    match (usage, &attribute.data) {
        (AttributeUsageV8::ColorSet, VectorData::Vector4(_)) => AttributeDataTypeV8::Byte4,
        (_, VectorData::Vector2(_)) => AttributeDataTypeV8::Float2,
        (_, VectorData::Vector3(_)) => AttributeDataTypeV8::Float3,
        (_, VectorData::Vector4(_)) => AttributeDataTypeV8::HalfFloat4,
    }
}

fn create_attributes_v8(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
    let mut attributes = Vec::new();

    let mut stride0 = 0u32;
    add_attributes_v8(
        &mut attributes,
        &data.positions,
        &mut stride0,
        0,
        AttributeUsageV8::Position,
    );
    add_attributes_v8(
        &mut attributes,
        &data.normals,
        &mut stride0,
        0,
        AttributeUsageV8::Normal,
    );

    // TODO: It's unclear what the usage enum for binormal is for version 1.8, so skip it for now.

    add_attributes_v8(
        &mut attributes,
        &data.tangents,
        &mut stride0,
        0,
        AttributeUsageV8::Tangent,
    );

    let mut stride1 = 0;
    add_attributes_v8(
        &mut attributes,
        &data.texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV8::TextureCoordinate,
    );
    add_attributes_v8(
        &mut attributes,
        &data.color_sets,
        &mut stride1,
        1,
        AttributeUsageV8::ColorSet,
    );

    (
        stride0,
        stride1,
        MeshAttributes::AttributesV8(SsbhArray::new(attributes)),
    )
}

fn add_attributes_v10(
    attributes: &mut Vec<MeshAttributeV10>,
    attributes_to_add: &[AttributeData],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV10,
) {
    for (i, attribute) in attributes_to_add.iter().enumerate() {
        let data_type = infer_data_type_v10(attribute, usage);

        // This is a convention in games such as Smash Ultimate and Pokemon Snap.
        let name = match (usage, i) {
            (AttributeUsageV10::Tangent, 0) => "map1",
            (AttributeUsageV10::Binormal, 0) => "map1",
            (AttributeUsageV10::Binormal, 1) => "uvSet",
            _ => &attribute.name,
        };

        add_attribute_v10(
            attributes,
            current_stride,
            name,
            &attribute.name,
            buffer_index,
            i as u64,
            usage,
            data_type,
        );
    }
}

fn infer_data_type_v10(attribute: &AttributeData, usage: AttributeUsageV10) -> AttributeDataType {
    match (usage, &attribute.data) {
        // Some data is less sensitive to the lower precision of f16.
        (AttributeUsageV10::Normal, VectorData::Vector2(_)) => AttributeDataType::HalfFloat2,
        (AttributeUsageV10::Normal, VectorData::Vector4(_)) => AttributeDataType::HalfFloat4,
        (AttributeUsageV10::Tangent, VectorData::Vector2(_)) => AttributeDataType::HalfFloat2,
        (AttributeUsageV10::Tangent, VectorData::Vector4(_)) => AttributeDataType::HalfFloat4,
        (AttributeUsageV10::TextureCoordinate, VectorData::Vector2(_)) => {
            AttributeDataType::HalfFloat2
        }
        (AttributeUsageV10::TextureCoordinate, VectorData::Vector4(_)) => {
            AttributeDataType::HalfFloat4
        }
        (AttributeUsageV10::ColorSet, VectorData::Vector2(_)) => AttributeDataType::HalfFloat2,
        (AttributeUsageV10::ColorSet, VectorData::Vector4(_)) => AttributeDataType::Byte4,
        // Default to using the largest available precision.
        (_, VectorData::Vector2(_)) => AttributeDataType::Float2,
        (_, VectorData::Vector3(_)) => AttributeDataType::Float3,
        (_, VectorData::Vector4(_)) => AttributeDataType::Float4,
    }
}

fn create_attributes_v10(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
    let mut attributes = Vec::new();

    let mut stride0 = 0u32;
    add_attributes_v10(
        &mut attributes,
        &data.positions,
        &mut stride0,
        0,
        AttributeUsageV10::Position,
    );
    add_attributes_v10(
        &mut attributes,
        &data.normals,
        &mut stride0,
        0,
        AttributeUsageV10::Normal,
    );
    add_attributes_v10(
        &mut attributes,
        &data.binormals,
        &mut stride0,
        0,
        AttributeUsageV10::Binormal,
    );
    add_attributes_v10(
        &mut attributes,
        &data.tangents,
        &mut stride0,
        0,
        AttributeUsageV10::Tangent,
    );

    let mut stride1 = 0;
    add_attributes_v10(
        &mut attributes,
        &data.texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV10::TextureCoordinate,
    );
    add_attributes_v10(
        &mut attributes,
        &data.color_sets,
        &mut stride1,
        1,
        AttributeUsageV10::ColorSet,
    );

    (
        stride0,
        stride1,
        MeshAttributes::AttributesV10(SsbhArray::new(attributes)),
    )
}

struct MeshVertexData {
    mesh_objects: Vec<MeshObject>,
    vertex_buffers: Vec<Vec<u8>>,
    index_buffer: Vec<u8>,
}

// TODO: Use a better error type.
fn create_mesh_objects(
    major_version: u16,
    minor_version: u16,
    mesh_object_data: &[MeshObjectData],
) -> std::io::Result<MeshVertexData> {
    // TODO: Split this into functions and do some cleanup.
    let mut mesh_objects = Vec::new();

    let mut final_buffer_offset = 0;

    let mut index_buffer = Cursor::new(Vec::new());

    let mut buffer0 = Cursor::new(Vec::new());
    let mut buffer1 = Cursor::new(Vec::new());

    for data in mesh_object_data {
        // TODO: Link ssbh_lib attributes to attribute data?
        // TODO: Find a way to guarantee that the generated attribute data type is used for the buffer writes.
        let (stride0, stride1, attributes) = create_attributes(data, major_version, minor_version);

        // TODO: Make sure all attributes have the same length and return an error if not.
        // TODO: Allow no position attribute as long as the remaining attributes have equal counts.
        let position_data = data.positions.get(0).ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Missing position attribute. Failed to determine vertex count.",
        ))?;
        let vertex_count = match &position_data.data {
            VectorData::Vector2(v) => v.len(),
            VectorData::Vector3(v) => v.len(),
            VectorData::Vector4(v) => v.len(),
        } as u32;

        // TODO: What does this value do?
        let unk6 = if major_version == 1 && minor_version == 8 {
            32
        } else {
            0
        };

        // Assume generated bounding data isn't critical if there are no points,
        // so use an empty list if there are no position attributes.
        let positions = match data.positions.first() {
            Some(attribute) => vector_data_to_glam(&attribute.data),
            None => Vec::new(),
        };

        let (u16_indices, draw_element_type) = try_convert_indices(&data.vertex_indices);

        let mesh_object = MeshObject {
            name: data.name.clone().into(),
            sub_index: data.sub_index,
            parent_bone_name: data.parent_bone_name.clone().into(),
            vertex_count: vertex_count as u32,
            vertex_index_count: data.vertex_indices.len() as u32,
            unk2: 3, // TODO: triangles?
            vertex_buffer0_offset: buffer0.position() as u32,
            vertex_buffer1_offset: buffer1.position() as u32,
            final_buffer_offset,
            buffer_index: 0, // TODO: This is always 0
            stride0,
            stride1,
            unk6,
            unk7: 0,
            index_buffer_offset: index_buffer.position() as u32,
            unk8: 4,
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

        // Check if the indices could be successfully converted to u16.
        match u16_indices {
            Some(u16_indices) => {
                for index in u16_indices {
                    index_buffer.write_all(&index.to_le_bytes())?;
                }
            }
            None => {
                for index in &data.vertex_indices {
                    index_buffer.write_all(&index.to_le_bytes())?;
                }
            }
        }

        // The first buffer (buffer1) has interleaved data for the required attributes.
        // Vertex, Data
        // 0: Position0, Normal0, Tangent0
        // 1: Position0, Normal0, Tangent0
        // ...
        for i in 0..vertex_count as usize {
            write_all_f32(&mut buffer0, &data.positions, i)?;

            // Assume Normal0 and Tangent0 will use half precision.
            // Binormal uses single precision for some games.
            // TODO: This won't always match the original (add an option?).
            write_all_f16(&mut buffer0, &data.normals, i)?;
            write_all_f32(&mut buffer0, &data.binormals, i)?;
            write_all_f16(&mut buffer0, &data.tangents, i)?;
        }

        // The second buffer (buffer1) has interleaved data for the texture coordinates and colorsets.
        // The attributes differ between meshes, but texture coordinates always precede colorsets.
        // Vertex, Data
        // 0: map1, colorSet1
        // 1: map1, colorSet1
        // ...
        // TODO: Make sure all arrays have the same length.
        for i in 0..vertex_count as usize {
            // Assume texture coordinates will use half precision.
            for attribute in &data.texture_coordinates {
                // TODO: Flipping UVs should be configurable.

                match &attribute.data {
                    // TODO: Find a better way to handle flipping.
                    VectorData::Vector2(v) => {
                        write_f16(&mut buffer1, &[v[i][0], 1.0f32 - v[i][1]])?
                    }
                    // TODO: There is no HalfFloat3.
                    VectorData::Vector3(v) => {
                        write_f16(&mut buffer1, &[v[i][0], 1.0f32 - v[i][1]])?
                    }
                    VectorData::Vector4(v) => {
                        write_f16(&mut buffer1, &[v[i][0], 1.0f32 - v[i][1]])?
                    }
                }
            }

            // Assume u8 for color sets.
            for attribute in &data.color_sets {
                match &attribute.data {
                    VectorData::Vector2(v) => write_u8(&mut buffer1, &v[i])?,
                    VectorData::Vector3(v) => write_u8(&mut buffer1, &v[i])?,
                    VectorData::Vector4(v) => write_u8(&mut buffer1, &v[i])?,
                }
            }
        }

        // TODO: Why is this 32?
        final_buffer_offset += 32 * mesh_object.vertex_count;

        mesh_objects.push(mesh_object);
    }

    // There are always four vertex buffers, but only the first two contain data.
    // The remaining two vertex buffers are empty.
    Ok(MeshVertexData {
        mesh_objects,
        vertex_buffers: vec![
            buffer0.into_inner(),
            buffer1.into_inner(),
            Vec::new(),
            Vec::new(),
        ],
        index_buffer: index_buffer.into_inner(),
    })
}

fn try_convert_indices(indices: &[u32]) -> (Option<Vec<u16>>, DrawElementType) {
    // Try and convert the vertex indices to a smaller type.
    let u16_indices: Result<Vec<u16>, _> = indices.iter().map(|i| u16::try_from(*i)).collect();
    match u16_indices {
        Ok(indices) => (Some(indices), DrawElementType::UnsignedShort),
        Err(_) => (None, DrawElementType::UnsignedInt),
    }
}

fn vector_data_to_glam(points: &VectorData) -> Vec<geometry_tools::glam::Vec3A> {
    match points {
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

fn calculate_bounding_info(
    positions: &[geometry_tools::glam::Vec3A],
) -> ssbh_lib::formats::mesh::BoundingInfo {
    // Calculate bounding info based on the current points.
    let (sphere_center, sphere_radius) =
        geometry_tools::calculate_bounding_sphere_from_points(&positions);
    let (aabb_min, aabb_max) = geometry_tools::calculate_aabb_from_points(&positions);

    // TODO: Compute a better oriented bounding box.
    let obb_center = aabb_min.add(aabb_max).div(2f32);
    let obb_size = aabb_max.sub(aabb_min).div(2f32);

    ssbh_lib::formats::mesh::BoundingInfo {
        bounding_sphere: ssbh_lib::formats::mesh::BoundingSphere {
            center: Vector3::new(sphere_center.x, sphere_center.y, sphere_center.z),
            radius: sphere_radius,
        },
        bounding_volume: ssbh_lib::formats::mesh::BoundingVolume {
            min: Vector3::new(aabb_min.x, aabb_min.y, aabb_min.z),
            max: Vector3::new(aabb_max.x, aabb_max.y, aabb_max.z),
        },
        oriented_bounding_box: ssbh_lib::formats::mesh::OrientedBoundingBox {
            center: Vector3::new(obb_center.x, obb_center.y, obb_center.z),
            transform: Matrix3x3::identity(),
            size: Vector3::new(obb_size.x, obb_size.y, obb_size.z),
        },
    }
}

fn write_f32<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

// TODO: Writing one index at a time might not be most elegant approach.
fn write_all_f32<W: Write>(
    writer: &mut W,
    attributes: &[AttributeData],
    index: usize,
) -> std::io::Result<()> {
    for attribute in attributes {
        match &attribute.data {
            VectorData::Vector2(v) => write_f32(writer, &v[index])?,
            VectorData::Vector3(v) => write_f32(writer, &v[index])?,
            VectorData::Vector4(v) => write_f32(writer, &v[index])?,
        }
    }
    Ok(())
}

fn write_u8<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&[get_u8_clamped(*component)])?;
    }
    Ok(())
}

fn write_f16<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&f16::from_f32(*component).to_le_bytes())?;
    }
    Ok(())
}

fn write_all_f16<W: Write>(
    writer: &mut W,
    attributes: &[AttributeData],
    index: usize,
) -> std::io::Result<()> {
    for attribute in attributes {
        match &attribute.data {
            VectorData::Vector2(v) => write_f16(writer, &v[index])?,
            VectorData::Vector3(v) => write_f16(writer, &v[index])?,
            VectorData::Vector4(v) => write_f16(writer, &v[index])?,
        }
    }
    Ok(())
}

fn read_influences(rigging_group: &MeshRiggingGroup) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
    let mut bone_influences = Vec::new();
    for buffer in &rigging_group.buffers.elements {
        let bone_name = buffer
            .bone_name
            .get_string()
            .ok_or("Failed to read bone name.")?;

        // TODO: Find a way to test this.
        let influences = match &buffer.data {
            ssbh_lib::formats::mesh::VertexWeights::VertexWeightsV8(v) => v
                .elements
                .iter()
                .map(|influence| VertexWeight {
                    vertex_index: influence.vertex_index,
                    vertex_weight: influence.vertex_weight,
                })
                .collect(),
            ssbh_lib::formats::mesh::VertexWeights::VertexWeightsV10(v) => {
                // Version 1.10 using a byte buffer instead of storing an array of vertex weights directly.
                // The format is slightly different than version 1.8.
                let mut elements = Vec::new();
                let mut reader = Cursor::new(&v.elements);
                while let Ok(influence) =
                    reader.read_le::<ssbh_lib::formats::mesh::VertexWeightV10>()
                {
                    elements.push(VertexWeight {
                        vertex_index: influence.vertex_index as u32,
                        vertex_weight: influence.vertex_weight,
                    });
                }
                elements
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

struct MeshAttribute {
    pub name: String,
    pub index: u64,
    pub offset: u64,
    pub data_type: DataType,
}

impl From<&MeshAttributeV10> for MeshAttribute {
    fn from(a: &MeshAttributeV10) -> Self {
        MeshAttribute {
            name: get_attribute_name(a).unwrap_or("").to_string(),
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
        MeshAttributes::AttributesV8(attributes_v8) => attributes_v8
            .elements
            .iter()
            .filter(|a| AttributeUsage::from(a.usage) == usage)
            .map(|a| a.into())
            .collect(),
        MeshAttributes::AttributesV10(attributes_v10) => attributes_v10
            .elements
            .iter()
            .filter(|a| AttributeUsage::from(a.usage) == usage)
            .map(|a| a.into())
            .collect(),
    }
}

fn get_u8_clamped(f: f32) -> u8 {
    f.clamp(0.0f32, 1.0f32).mul(255.0f32).round() as u8
}

/// Gets the name of the mesh attribute. This uses the attribute names array,
/// which can be assumed to contain a single value that is unique with respect to the other attributes for the mesh object.
pub fn get_attribute_name(attribute: &MeshAttributeV10) -> Option<&str> {
    attribute.attribute_names.elements.get(0)?.get_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        // Remove any whitespace used to make the tests more readable.
        let no_whitespace: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(no_whitespace).unwrap()
    }

    #[test]
    fn attribute_from_attribute_v10() {
        let attribute_v10 = MeshAttributeV10 {
            usage: AttributeUsageV10::Normal,
            data_type: AttributeDataType::HalfFloat2,
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

        let result = create_vertex_weights(1, 8, &weights).unwrap();
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

        let result = create_vertex_weights(1, 10, &weights).unwrap();
        match result {
            VertexWeights::VertexWeightsV10(v) => {
                assert_eq!(hex_bytes("0000 00000000 01000 000803f"), v.elements);
            }
            _ => panic!("Invalid version"),
        }
    }

    #[test]
    fn create_attributes_mesh_v1_8() {
        let data = MeshObjectData {
            name: "name".into(),
            sub_index: 0,
            parent_bone_name: "".into(),
            vertex_indices: Vec::new(),
            positions: vec![AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            normals: vec![AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            binormals: Vec::new(),
            tangents: vec![AttributeData {
                name: "t0".into(),
                data: VectorData::Vector4(Vec::new()),
            }],
            texture_coordinates: vec![
                AttributeData {
                    name: "firstUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
                AttributeData {
                    name: "secondUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
            ],
            color_sets: vec![
                AttributeData {
                    name: "color1".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
                AttributeData {
                    name: "color2".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
            ],
            bone_influences: Vec::new(),
        };

        // TODO: Add option to choose single or double precision.
        let (stride0, stride1, attributes) = create_attributes(&data, 1, 8);
        assert_eq!(32, stride0);
        assert_eq!(24, stride1);

        match attributes {
            MeshAttributes::AttributesV8(a) => {
                let mut attributes = a.elements.iter();

                // Check buffer 0.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Position, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float3, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Normal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(12, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float3, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Tangent, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(24, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::HalfFloat4, a.data_type);

                // Check buffer 1.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float2, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(8, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float2, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(16, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Byte4, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(20, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Byte4, a.data_type);
            }
            _ => panic!("invalid version"),
        };
    }

    #[test]
    fn create_attributes_mesh_v1_10() {
        let data = MeshObjectData {
            name: "name".into(),
            sub_index: 0,
            parent_bone_name: "".into(),
            vertex_indices: Vec::new(),
            positions: vec![AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            normals: vec![AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            binormals: vec![
                AttributeData {
                    name: "b1".into(),
                    data: VectorData::Vector3(Vec::new()),
                },
                AttributeData {
                    name: "b2".into(),
                    data: VectorData::Vector3(Vec::new()),
                },
            ],
            tangents: vec![AttributeData {
                name: "t0".into(),
                data: VectorData::Vector4(Vec::new()),
            }],
            texture_coordinates: vec![
                AttributeData {
                    name: "firstUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
                AttributeData {
                    name: "secondUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
            ],
            color_sets: vec![
                AttributeData {
                    name: "color1".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
                AttributeData {
                    name: "color2".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
            ],
            bone_influences: Vec::new(),
        };

        // TODO: Add option to choose single or double precision.
        let (stride0, stride1, attributes) = create_attributes(&data, 1, 10);
        assert_eq!(56, stride0);
        assert_eq!(16, stride1);

        match attributes {
            MeshAttributes::AttributesV10(a) => {
                let mut attributes = a.elements.iter();
                // Check buffer 0.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Position, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataType::Float3, a.data_type);
                assert_eq!("p0", a.name.get_string().unwrap());
                assert_eq!("p0", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Normal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(12, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataType::Float3, a.data_type);
                assert_eq!("n0", a.name.get_string().unwrap());
                assert_eq!("n0", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Binormal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(24, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataType::Float3, a.data_type);
                // Using "map1" is a convention with the format for some reason.
                assert_eq!("map1", a.name.get_string().unwrap());
                assert_eq!("b1", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Binormal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(36, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataType::Float3, a.data_type);
                // Using "uvSet" is a convention with the format for some reason.
                assert_eq!("uvSet", a.name.get_string().unwrap());
                assert_eq!("b2", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Tangent, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(48, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataType::HalfFloat4, a.data_type);
                // Using "map1" is a convention with the format for some reason.
                assert_eq!("map1", a.name.get_string().unwrap());
                assert_eq!("t0", a.attribute_names.elements[0].get_string().unwrap());

                // Check buffer 1.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataType::HalfFloat2, a.data_type);
                assert_eq!("firstUv", a.name.get_string().unwrap());
                assert_eq!(
                    "firstUv",
                    a.attribute_names.elements[0].get_string().unwrap()
                );

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(4, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataType::HalfFloat2, a.data_type);
                assert_eq!("secondUv", a.name.get_string().unwrap());
                assert_eq!(
                    "secondUv",
                    a.attribute_names.elements[0].get_string().unwrap()
                );

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(8, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataType::Byte4, a.data_type);
                assert_eq!("color1", a.name.get_string().unwrap());
                assert_eq!(
                    "color1",
                    a.attribute_names.elements[0].get_string().unwrap()
                );

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(12, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataType::Byte4, a.data_type);
                assert_eq!("color2", a.name.get_string().unwrap());
                assert_eq!(
                    "color2",
                    a.attribute_names.elements[0].get_string().unwrap()
                );
            }
            _ => panic!("invalid version"),
        };
    }

    #[test]
    fn draw_element_type_u16() {
        // The indices are always stored as u32 by the object data wrapper type.
        // In this case, it's safe to convert to a smaller type.
        assert_eq!(
            (Some(vec![0, 1, u16::MAX]), DrawElementType::UnsignedShort),
            try_convert_indices(&[0, 1, u16::MAX as u32])
        )
    }

    #[test]
    fn draw_element_type_empty() {
        assert_eq!(
            (Some(Vec::new()), DrawElementType::UnsignedShort),
            try_convert_indices(&[])
        )
    }

    #[test]
    fn draw_element_type_u32() {
        // Add elements not representable by u16.
        assert_eq!(
            (None, DrawElementType::UnsignedInt),
            try_convert_indices(&[0, 1, u16::MAX as u32 + 1])
        )
    }

    #[test]
    fn u8_clamped() {
        assert_eq!(0u8, get_u8_clamped(-1.0f32));
        assert_eq!(0u8, get_u8_clamped(0.0f32));
        assert_eq!(128u8, get_u8_clamped(128u8 as f32 / 255f32));
        assert_eq!(255u8, get_u8_clamped(1.0f32));
        assert_eq!(255u8, get_u8_clamped(2.0f32));
    }

    #[test]
    fn size_in_bytes_attributes_v10() {
        assert_eq!(4, get_size_in_bytes_v10(&AttributeDataType::Byte4));
        assert_eq!(8, get_size_in_bytes_v10(&AttributeDataType::Float2));
        assert_eq!(12, get_size_in_bytes_v10(&AttributeDataType::Float3));
        assert_eq!(16, get_size_in_bytes_v10(&AttributeDataType::Float4));
        assert_eq!(4, get_size_in_bytes_v10(&AttributeDataType::HalfFloat2));
        assert_eq!(8, get_size_in_bytes_v10(&AttributeDataType::HalfFloat4));
    }

    #[test]
    fn size_in_bytes_attributes_v8() {
        assert_eq!(4, get_size_in_bytes_v8(&AttributeDataTypeV8::Byte4));
        assert_eq!(8, get_size_in_bytes_v8(&AttributeDataTypeV8::Float2));
        assert_eq!(12, get_size_in_bytes_v8(&AttributeDataTypeV8::Float3));
        assert_eq!(8, get_size_in_bytes_v8(&AttributeDataTypeV8::HalfFloat4));
    }

    fn get_data_type_v10(data: VectorData, usage: AttributeUsageV10) -> AttributeDataType {
        let a = AttributeData {
            name: "".to_string(),
            data,
        };
        infer_data_type_v10(&a, usage)
    }

    #[test]
    fn infer_position_type_v10() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            AttributeDataType::Float2,
            get_data_type_v10(VectorData::Vector2(Vec::new()), AttributeUsageV10::Position)
        );
        assert_eq!(
            AttributeDataType::Float3,
            get_data_type_v10(VectorData::Vector3(Vec::new()), AttributeUsageV10::Position)
        );
        assert_eq!(
            AttributeDataType::Float4,
            get_data_type_v10(VectorData::Vector4(Vec::new()), AttributeUsageV10::Position)
        );
    }

    #[test]
    fn infer_normal_type_v10() {
        // Check that normals use the smallest available floating point type.
        assert_eq!(
            AttributeDataType::HalfFloat2,
            get_data_type_v10(VectorData::Vector2(Vec::new()), AttributeUsageV10::Normal)
        );
        assert_eq!(
            AttributeDataType::Float3,
            get_data_type_v10(VectorData::Vector3(Vec::new()), AttributeUsageV10::Normal)
        );
        assert_eq!(
            AttributeDataType::HalfFloat4,
            get_data_type_v10(VectorData::Vector4(Vec::new()), AttributeUsageV10::Normal)
        );
    }

    #[test]
    fn infer_texcoord_type_v10() {
        // Check that texture coordinates use the smallest available floating point type.
        assert_eq!(
            AttributeDataType::HalfFloat2,
            get_data_type_v10(
                VectorData::Vector2(Vec::new()),
                AttributeUsageV10::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataType::Float3,
            get_data_type_v10(
                VectorData::Vector3(Vec::new()),
                AttributeUsageV10::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataType::HalfFloat4,
            get_data_type_v10(
                VectorData::Vector4(Vec::new()),
                AttributeUsageV10::TextureCoordinate
            )
        );
    }

    #[test]
    fn infer_colorset_type_v10() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            AttributeDataType::HalfFloat2,
            get_data_type_v10(VectorData::Vector2(Vec::new()), AttributeUsageV10::ColorSet)
        );
        assert_eq!(
            AttributeDataType::Float3,
            get_data_type_v10(VectorData::Vector3(Vec::new()), AttributeUsageV10::ColorSet)
        );
        assert_eq!(
            AttributeDataType::Byte4,
            get_data_type_v10(VectorData::Vector4(Vec::new()), AttributeUsageV10::ColorSet)
        );
    }

    fn get_data_type_v8(data: VectorData, usage: AttributeUsageV8) -> AttributeDataTypeV8 {
        let a = AttributeData {
            name: "".to_string(),
            data,
        };
        infer_data_type_v8(&a, usage)
    }

    #[test]
    fn infer_position_type_v8() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(VectorData::Vector2(Vec::new()), AttributeUsageV8::Position)
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(VectorData::Vector3(Vec::new()), AttributeUsageV8::Position)
        );
        assert_eq!(
            AttributeDataTypeV8::HalfFloat4,
            get_data_type_v8(VectorData::Vector4(Vec::new()), AttributeUsageV8::Position)
        );
    }

    #[test]
    fn infer_normal_type_v8() {
        // Check that normals use the smallest available floating point type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(VectorData::Vector2(Vec::new()), AttributeUsageV8::Normal)
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(VectorData::Vector3(Vec::new()), AttributeUsageV8::Normal)
        );
        assert_eq!(
            AttributeDataTypeV8::HalfFloat4,
            get_data_type_v8(VectorData::Vector4(Vec::new()), AttributeUsageV8::Normal)
        );
    }

    #[test]
    fn infer_texcoord_type_v8() {
        // Check that texture coordinates use the smallest available floating point type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(
                VectorData::Vector2(Vec::new()),
                AttributeUsageV8::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(
                VectorData::Vector3(Vec::new()),
                AttributeUsageV8::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataTypeV8::HalfFloat4,
            get_data_type_v8(
                VectorData::Vector4(Vec::new()),
                AttributeUsageV8::TextureCoordinate
            )
        );
    }

    #[test]
    fn infer_colorset_type_v8() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(VectorData::Vector2(Vec::new()), AttributeUsageV8::ColorSet)
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(VectorData::Vector3(Vec::new()), AttributeUsageV8::ColorSet)
        );
        assert_eq!(
            AttributeDataTypeV8::Byte4,
            get_data_type_v8(VectorData::Vector4(Vec::new()), AttributeUsageV8::ColorSet)
        );
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
}
