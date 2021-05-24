use std::{error::Error, io::Write, ops::Mul};

use binread::BinReaderExt;
use binread::{io::Cursor, BinRead};
use half::f16;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataType, AttributeDataTypeV8, AttributeUsageV10, AttributeUsageV8,
        DrawElementType, Mesh, MeshAttributeV10, MeshAttributeV8, MeshAttributes, MeshBoneBuffer,
        MeshObject, MeshRiggingGroup, RiggingFlags, VertexWeightV8, VertexWeights,
    },
    SsbhByteBuffer,
};
use ssbh_lib::{Half, SsbhArray};

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

fn read_vertex_indices(
    mesh_index_buffer: &[u8],
    mesh_object: &MeshObject,
) -> Result<Vec<u32>, Box<dyn Error>> {
    // Use u32 regardless of the actual data type to simplify conversions.
    let count = mesh_object.vertex_index_count as usize;
    let offset = mesh_object.index_buffer_offset as u64;
    let mut reader = Cursor::new(mesh_index_buffer);
    let indices = match mesh_object.draw_element_type {
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
    };

    Ok(indices?)
}

fn read_attribute_data<T>(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    attribute: &MeshAttribute,
) -> Result<VectorData, Box<dyn Error>> {
    // Get the raw data for the attribute for this mesh object.
    let attribute_buffer = mesh
        .vertex_buffers
        .elements
        .get(attribute.index as usize)
        .ok_or("Invalid buffer index.")?;

    // TODO: Create functions for this?
    let offset = match attribute.index {
        0 => Ok(attribute.offset + mesh_object.vertex_buffer0_offset as u64),
        1 => Ok(attribute.offset + mesh_object.vertex_buffer1_offset as u64),
        _ => Err("Buffer indices higher than 1 are not supported."),
    }? as u64;

    let stride = match attribute.index {
        0 => Ok(mesh_object.stride0),
        1 => Ok(mesh_object.stride1),
        _ => Err("Buffer indices higher than 1 are not supported."),
    }? as u64;

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
        DataType::Byte4 => VectorData::Vector4(read_vector_data::<_, u8, 4>(
            &mut reader,
            count,
            offset,
            stride,
        )?),
    };

    Ok(data)
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

        // TODO: Clean this up with a function?
        if flip_vertical {
            match &mut data {
                VectorData::Vector2(v) => {
                    for element in v.iter_mut() {
                        element[1] = 1.0 - element[1];
                    }
                }
                VectorData::Vector3(v) => {
                    for element in v.iter_mut() {
                        element[1] = 1.0 - element[1];
                    }
                }
                VectorData::Vector4(v) => {
                    for element in v.iter_mut() {
                        element[1] = 1.0 - element[1];
                    }
                }
            }
        }

        attributes.push(AttributeData {
            name: attribute.name.to_string(),
            data,
        });
    }

    Ok(attributes)
}

/// Returns all the colorset attributes for the specified `mesh_object`.
/// [u8] values are converted to [f32] by normalizing to the range 0.0 to 1.0. 
/// If `divide_by_2` is `true`, the output range is 0.0f32 to 2.0f32.
pub fn read_colorsets(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    divide_by_2: bool,
) -> Result<Vec<AttributeData>, Box<dyn Error>> {
    let colorsets = get_attributes(&mesh_object, AttributeUsage::ColorSet);

    let mut attributes = Vec::new();
    for attribute in &colorsets {
        let mut data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;

        // TODO: The documentation for divide_by_2 is confusing.
        // There should be corresponding option when saving the mesh object to scale the values appropriately.

        // Normalize integral values by converting the range [0.0, 255.0] to [0.0, 2.0] or [0.0, 1.0].
        // TODO: Make this optional?
        // TODO: Find a cleaner/safer way to do this.
        let divisor = if divide_by_2 { 128.0f32 } else { 255.0f32 };
        match &mut data {
            VectorData::Vector2(v) => {
                for element in v.iter_mut() {
                    element[0] /= divisor;
                    element[1] /= divisor;
                }
            }
            VectorData::Vector3(v) => {
                for element in v.iter_mut() {
                    element[0] /= divisor;
                    element[1] /= divisor;
                    element[2] /= divisor;
                }
            }
            VectorData::Vector4(v) => {
                for element in v.iter_mut() {
                    element[0] /= divisor;
                    element[1] /= divisor;
                    element[2] /= divisor;
                    element[3] /= divisor;
                }
            }
        }

        attributes.push(AttributeData {
            name: attribute.name.clone(),
            data,
        });
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
        let color_sets = read_colorsets(&mesh, &mesh_object, true)?;
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

// TODO: Create a new mesh instead.
pub fn update_mesh(
    mesh: &mut Mesh,
    updated_object_data: &[MeshObjectData],
) -> Result<(), Box<dyn Error>> {
    let (mesh_objects, vertex_buffers, index_buffer) =
        create_mesh_objects(mesh, updated_object_data)?;

    mesh.objects.elements = mesh_objects;

    mesh.vertex_buffers.elements = vertex_buffers
        .into_iter()
        .map(|b| SsbhByteBuffer { elements: b })
        .collect();

    mesh.index_buffer.elements = index_buffer;

    mesh.rigging_buffers.elements = create_rigging_buffers(mesh, updated_object_data)?;

    Ok(())
}

fn create_rigging_buffers(
    source_mesh: &Mesh,
    updated_object_data: &[MeshObjectData],
) -> std::io::Result<Vec<MeshRiggingGroup>> {
    let mut rigging_buffers = Vec::new();

    for mesh_object in updated_object_data {
        // Just use the existing flags for now.
        // TODO: Properly recreate flags.
        let flags = match source_mesh.rigging_buffers.elements.iter().find(|r| {
            r.mesh_object_name.get_string() == Some(&mesh_object.name)
                && r.mesh_object_sub_index == mesh_object.sub_index
        }) {
            Some(r) => r.flags,
            None => RiggingFlags::new().with_max_influences(4),
        };

        //TODO: Find a way to convert &str or &String without an additional clone?
        let mut buffers = Vec::new();
        for i in &mesh_object.bone_influences {
            let buffer = MeshBoneBuffer {
                bone_name: i.bone_name.clone().into(),
                data: create_vertex_weights(
                    source_mesh.major_version,
                    source_mesh.minor_version,
                    &i.vertex_weights,
                )?,
            };
            buffers.push(buffer);
        }

        let buffer = MeshRiggingGroup {
            mesh_object_name: mesh_object.name.clone().into(),
            mesh_object_sub_index: mesh_object.sub_index,
            flags: flags,
            buffers: buffers.into(),
        };

        rigging_buffers.push(buffer)
    }

    Ok(rigging_buffers)
}

// TODO: Test both versions.
fn create_vertex_weights(
    major_version: u16,
    minor_version: u16,
    vertex_weights: &[VertexWeight],
) -> std::io::Result<VertexWeights> {
    // TODO: Create the weights (1.8) or write the buffer (1.10).
    match (major_version, minor_version) {
        (1, 8) => {
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
            let mut bytes = Cursor::new(Vec::new());
            for weight in vertex_weights {
                bytes.write_all(&(weight.vertex_index as u16).to_le_bytes())?;
                bytes.write_all(&weight.vertex_weight.to_le_bytes())?;
            }
            Ok(VertexWeights::VertexWeightsV10(bytes.into_inner().into()))
        }
        _ => panic!(
            "Unsupported MESH version {}.{}",
            major_version, minor_version
        ),
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
        // TODO: Don't assume the precision.
        let data_type = match attribute.data {
            VectorData::Vector2(_) => AttributeDataTypeV8::Float2,
            VectorData::Vector3(_) => AttributeDataTypeV8::Float3,
            VectorData::Vector4(_) => {
                if usage == AttributeUsageV8::ColorSet {
                    AttributeDataTypeV8::Byte4
                } else {
                    AttributeDataTypeV8::HalfFloat4
                }
            }
        };

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
        // TODO: Don't assume the precision.
        let data_type = match attribute.data {
            VectorData::Vector2(_) => {
                if usage == AttributeUsageV10::TextureCoordinate {
                    AttributeDataType::HalfFloat2
                } else {
                    AttributeDataType::Float2
                }
            }
            VectorData::Vector3(_) => AttributeDataType::Float3,
            VectorData::Vector4(_) => {
                if usage == AttributeUsageV10::ColorSet {
                    AttributeDataType::Byte4
                } else {
                    AttributeDataType::HalfFloat4
                }
            }
        };

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

// TODO: Use a struct for the return type?
fn create_mesh_objects(
    source_mesh: &Mesh,
    mesh_object_data: &[MeshObjectData],
) -> Result<(Vec<MeshObject>, Vec<Vec<u8>>, Vec<u8>), Box<dyn Error>> {
    // TODO: Split this into functions and do some cleanup.
    let mut mesh_objects = Vec::new();

    let mut final_buffer_offset = 0;

    let mut index_buffer = Cursor::new(Vec::new());

    let mut buffer0 = Cursor::new(Vec::new());
    let mut buffer1 = Cursor::new(Vec::new());

    for data in mesh_object_data {
        // This should probably use the existing mesh object data when possible.
        let source_object = source_mesh
            .objects
            .elements
            .iter()
            .find(|o| o.name.get_string() == Some(&data.name) && o.sub_index == data.sub_index);

        match source_object {
            Some(source_object) => {
                let (stride0, stride1, attributes) =
                    create_attributes(data, source_mesh.major_version, source_mesh.minor_version);

                // TODO: Make sure all attributes have the same length.
                let position_data = data
                    .positions
                    .get(0)
                    .ok_or("Missing position attribute. Failed to determine vertex count.")?;
                let vertex_count = match &position_data.data {
                    VectorData::Vector2(v) => v.len(),
                    VectorData::Vector3(v) => v.len(),
                    VectorData::Vector4(v) => v.len(),
                } as u32;

                // TODO: What does this value do?
                let unk6 = if source_mesh.major_version == 1 && source_mesh.minor_version == 8 {
                    32
                } else {
                    0
                };

                let mesh_object = MeshObject {
                    name: data.name.clone().into(),
                    sub_index: data.sub_index,
                    parent_bone_name: data.parent_bone_name.clone().into(),
                    vertex_count: vertex_count as u32,
                    vertex_index_count: data.vertex_indices.len() as u32,
                    unk2: 3, // triangles?
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
                    draw_element_type: source_object.draw_element_type,
                    rigging_type: source_object.rigging_type,
                    unk11: 0,
                    unk12: 0,
                    bounding_info: source_object.bounding_info, // TODO: Calculate this
                    attributes,
                };

                // Assume unsigned short for vertex indices.
                // TODO: How to handle out of range values?
                for index in &data.vertex_indices {
                    index_buffer.write_all(&(*index as u16).to_le_bytes())?;
                }

                // The first buffer (buffer1) has interleaved data for the required attributes.
                // Vertex, Data
                // 0: Position0, Normal0, Tangent0
                // 1: Position0, Normal0, Tangent0
                // ...
                for i in 0..vertex_count as usize {
                    // TODO: How to write the buffers if the component count isn't yet known?

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
            None => {
                // TODO: Temporary workaround for not being able to rebuild all the data.
                // This assumes new mesh objects will not be added and bounding info, rigging info, etc does not change.
                continue;
            }
        }
    }

    // There are always four vertex buffers, but only the first two contain data.
    // The remaining two vertex buffers are empty.
    Ok((
        mesh_objects,
        vec![
            buffer0.into_inner(),
            buffer1.into_inner(),
            Vec::new(),
            Vec::new(),
        ],
        index_buffer.into_inner(),
    ))
}

fn write_f32<W: Write>(writer: &mut W, data: &[f32]) -> Result<(), Box<dyn Error>> {
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
) -> Result<(), Box<dyn Error>> {
    for attribute in attributes {
        match &attribute.data {
            VectorData::Vector2(v) => write_f32(writer, &v[index])?,
            VectorData::Vector3(v) => write_f32(writer, &v[index])?,
            VectorData::Vector4(v) => write_f32(writer, &v[index])?,
        }
    }
    Ok(())
}

fn write_u8<W: Write>(writer: &mut W, data: &[f32]) -> Result<(), Box<dyn Error>> {
    for component in data {
        writer.write_all(&[get_u8_clamped(*component)])?;
    }
    Ok(())
}

fn write_all_u8<W: Write>(
    writer: &mut W,
    attributes: &[AttributeData],
    index: usize,
) -> Result<(), Box<dyn Error>> {
    for attribute in attributes {
        match &attribute.data {
            VectorData::Vector2(v) => write_u8(writer, &v[index])?,
            VectorData::Vector3(v) => write_u8(writer, &v[index])?,
            VectorData::Vector4(v) => write_u8(writer, &v[index])?,
        }
    }
    Ok(())
}

fn write_f16<W: Write>(writer: &mut W, data: &[f32]) -> Result<(), Box<dyn Error>> {
    for component in data {
        writer.write_all(&f16::from_f32(*component).to_le_bytes())?;
    }
    Ok(())
}

fn write_all_f16<W: Write>(
    writer: &mut W,
    attributes: &[AttributeData],
    index: usize,
) -> Result<(), Box<dyn Error>> {
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
        // TODO: Come up with a default name based on usage and subindex.
        let name = "".to_string();
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
    attribute
        .attribute_names
        .elements
        .iter()
        .next()
        .unwrap()
        .get_string()
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

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Normal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(12, a.buffer_offset);
                assert_eq!(0, a.sub_index);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Tangent, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(24, a.buffer_offset);
                assert_eq!(0, a.sub_index);

                // Check buffer 1.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(8, a.buffer_offset);
                assert_eq!(1, a.sub_index);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(16, a.buffer_offset);
                assert_eq!(0, a.sub_index);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(20, a.buffer_offset);
                assert_eq!(1, a.sub_index);
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
                assert_eq!("p0", a.name.get_string().unwrap());
                assert_eq!("p0", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Normal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(12, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!("n0", a.name.get_string().unwrap());
                assert_eq!("n0", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Binormal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(24, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                // Using "map1" is a convention with the format for some reason.
                assert_eq!("map1", a.name.get_string().unwrap());
                assert_eq!("b1", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Binormal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(36, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                // Using "uvSet" is a convention with the format for some reason.
                assert_eq!("uvSet", a.name.get_string().unwrap());
                assert_eq!("b2", a.attribute_names.elements[0].get_string().unwrap());

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::Tangent, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(48, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                // Using "map1" is a convention with the format for some reason.
                assert_eq!("map1", a.name.get_string().unwrap());
                assert_eq!("t0", a.attribute_names.elements[0].get_string().unwrap());

                // Check buffer 1.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV10::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
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
}
