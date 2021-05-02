use binread::BinReaderExt;
use std::{error::Error, io::Write, ops::Mul};

use binread::{io::Cursor, BinRead};
use half::f16;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataType, AttributeDataTypeV8, AttributeUsage, DrawElementType, Mesh,
        MeshAttributeV10, MeshAttributeV8, MeshAttributes, MeshObject, MeshRiggingGroup,
    },
    SsbhByteBuffer,
};
use ssbh_lib::{Half, SsbhArray};

use crate::{read_data, read_vector_data};

pub enum DataType {
    Float2,
    Float3,
    Float4,
    HalfFloat2,
    HalfFloat4,
    Byte4,
}

// TODO: Move this to MESH?
#[derive(BinRead, Debug)]
pub struct VertexWeightV10 {
    pub vertex_index: u16,
    pub vertex_weight: f32,
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

/// Read the vertex positions for the specified `mesh_object`.
pub fn read_positions(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<AttributeData, Box<dyn Error>> {
    let attributes = get_attributes(&mesh_object, AttributeUsage::Position);
    let attribute = attributes.first().ok_or("No position attribute found.")?;
    let data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
    Ok(AttributeData {
        name: "Position0".to_string(),
        data,
    })
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
/// Values are scaled from 0u8 to 255u8 to 0.0f32 to 1.0f32. If `divide_by_2` is `true`,
/// the output range is 0.0f32 to 2.0f32.
pub fn read_colorsets(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    divide_by_2: bool,
) -> Result<Vec<AttributeData>, Box<dyn Error>> {
    // TODO: Find a cleaner way to do this (define a new enum?).
    let colorsets_v10 = get_attributes(&mesh_object, AttributeUsage::ColorSet);
    let colorsets_v8 = get_attributes(&mesh_object, AttributeUsage::ColorSetV8);

    let mut attributes = Vec::new();
    for attribute in colorsets_v10.iter().chain(colorsets_v8.iter()) {
        let mut data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;

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

pub fn read_normals(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<AttributeData, Box<dyn Error>> {
    let attributes = get_attributes(&mesh_object, AttributeUsage::Normal);
    let attribute = attributes.first().ok_or("No normals attribute found.")?;
    let data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
    Ok(AttributeData {
        name: attribute.name.clone(),
        data,
    })
}

pub fn read_tangents(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<AttributeData, Box<dyn Error>> {
    let attributes = get_attributes(&mesh_object, AttributeUsage::Tangent);
    let attribute = attributes.first().ok_or("No tangent attribute found.")?;
    let data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
    Ok(AttributeData {
        name: attribute.name.clone(),
        data,
    })
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
    pub positions: AttributeData,
    pub normals: AttributeData,
    pub tangents: AttributeData,
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

impl AttributeData {
    fn length(&self) -> usize {
        match &self.data {
            VectorData::Vector2(v) => v.len(),
            VectorData::Vector3(v) => v.len(),
            VectorData::Vector4(v) => v.len(),
        }
    }
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
        let positions = read_positions(&mesh, &mesh_object)?;
        let normals = read_normals(&mesh, &mesh_object)?;
        let tangents = read_tangents(&mesh, &mesh_object)?;
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
        create_mesh_objects(mesh, &updated_object_data)?;

    mesh.objects.elements = mesh_objects;

    mesh.vertex_buffers.elements = vertex_buffers
        .into_iter()
        .map(|b| SsbhByteBuffer { elements: b })
        .collect();
    mesh.index_buffer.elements = index_buffer;

    Ok(())
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
    usage: AttributeUsage,
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
    usage: AttributeUsage,
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

fn create_attributes_v8(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
    let mut attributes = Vec::new();
    let mut stride0 = 0u32;

    add_attribute_v8(
        &mut attributes,
        &mut stride0,
        0,
        0,
        AttributeUsage::Position,
        AttributeDataTypeV8::Float3,
    );

    // TODO: There's no known half precision 4 component type.
    let normal_type = match data.normals.data {
        VectorData::Vector2(_) => AttributeDataTypeV8::Float2,
        VectorData::Vector3(_) => AttributeDataTypeV8::Float3,
        VectorData::Vector4(_) => AttributeDataTypeV8::HalfFloat4,
    };
    add_attribute_v8(
        &mut attributes,
        &mut stride0,
        0,
        0,
        AttributeUsage::Normal,
        normal_type,
    );

    let tangent_type = match data.tangents.data {
        VectorData::Vector2(_) => AttributeDataTypeV8::Float2,
        VectorData::Vector3(_) => AttributeDataTypeV8::Float3,
        VectorData::Vector4(_) => AttributeDataTypeV8::HalfFloat4,
    };
    add_attribute_v8(
        &mut attributes,
        &mut stride0,
        0,
        0,
        AttributeUsage::Tangent,
        tangent_type,
    );

    let mut stride1 = 0;
    for (i, _) in data.texture_coordinates.iter().enumerate() {
        add_attribute_v8(
            &mut attributes,
            &mut stride1,
            1,
            i as u32,
            AttributeUsage::TextureCoordinate,
            AttributeDataTypeV8::Float2,
        );
    }

    for (i, _) in data.color_sets.iter().enumerate() {
        add_attribute_v8(
            &mut attributes,
            &mut stride1,
            1,
            i as u32,
            AttributeUsage::ColorSet,
            AttributeDataTypeV8::Byte4,
        );
    }

    (
        stride0,
        stride1,
        MeshAttributes::AttributesV8(SsbhArray::new(attributes)),
    )
}

fn create_attributes_v10(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
    let mut attributes = Vec::new();
    let mut stride0 = 0u32;

    add_attribute_v10(
        &mut attributes,
        &mut stride0,
        &data.positions.name,
        &data.positions.name,
        0,
        0,
        AttributeUsage::Position,
        AttributeDataType::Float3,
    );

    // TODO: There isn't a known type for half precision with 3 components.
    // TODO: Add an option to choose single vs double precision?
    let normal_type = match data.normals.data {
        VectorData::Vector2(_) => AttributeDataType::HalfFloat2,
        VectorData::Vector3(_) => AttributeDataType::Float3,
        VectorData::Vector4(_) => AttributeDataType::HalfFloat4,
    };
    add_attribute_v10(
        &mut attributes,
        &mut stride0,
        &data.normals.name,
        &data.normals.name,
        0,
        0,
        AttributeUsage::Normal,
        normal_type,
    );

    // TODO: There isn't a known type for half precision with 3 components.
    // TODO: Add an option to choose single vs double precision?
    let tangent_type = match data.tangents.data {
        VectorData::Vector2(_) => AttributeDataType::HalfFloat2,
        VectorData::Vector3(_) => AttributeDataType::Float3,
        VectorData::Vector4(_) => AttributeDataType::HalfFloat4,
    };

    // Tangent0 uses map1 for the name by convention.
    add_attribute_v10(
        &mut attributes,
        &mut stride0,
        "map1",
        &data.tangents.name,
        0,
        0,
        AttributeUsage::Tangent,
        tangent_type,
    );

    let mut stride1 = 0;
    for (i, attribute) in data.texture_coordinates.iter().enumerate() {
        add_attribute_v10(
            &mut attributes,
            &mut stride1,
            &attribute.name,
            &attribute.name,
            1,
            i as u64,
            AttributeUsage::TextureCoordinate,
            AttributeDataType::HalfFloat2,
        );
    }

    for (i, attribute) in data.color_sets.iter().enumerate() {
        add_attribute_v10(
            &mut attributes,
            &mut stride1,
            &attribute.name,
            &attribute.name,
            1,
            i as u64,
            AttributeUsage::ColorSet,
            AttributeDataType::Byte4,
        );
    }

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

                let mesh_object = MeshObject {
                    name: data.name.clone().into(),
                    sub_index: data.sub_index,
                    parent_bone_name: data.parent_bone_name.clone().into(),
                    vertex_count: data.positions.length() as u32,
                    vertex_index_count: data.vertex_indices.len() as u32,
                    unk2: 3, // triangles?
                    vertex_buffer0_offset: buffer0.position() as u32,
                    vertex_buffer1_offset: buffer1.position() as u32,
                    final_buffer_offset,
                    buffer_index: 0, // TODO: This is always 0
                    stride0,
                    stride1,
                    unk6: 0,
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
                    index_buffer.write(&(*index as u16).to_le_bytes())?;
                }

                // The first buffer has interleaved data for the required attributes.
                // Vertex, Data
                // 0: Position0, Normal0, Tangent0
                // 1: Position0, Normal0, Tangent0
                // ...
                for i in 0..data.positions.length() {
                    // TODO: How to write the buffers if the component count isn't yet known?

                    match &data.positions.data {
                        VectorData::Vector2(v) => write_f32(&mut buffer0, &v[i])?,
                        VectorData::Vector3(v) => write_f32(&mut buffer0, &v[i])?,
                        VectorData::Vector4(v) => write_f32(&mut buffer0, &v[i])?,
                    }

                    // Assume Normal0 and Tangent0 will use half precision.
                    // TODO: This won't always match the original (add an option?).
                    match &data.normals.data {
                        VectorData::Vector2(v) => write_f16(&mut buffer0, &v[i])?,
                        VectorData::Vector3(v) => write_f16(&mut buffer0, &v[i])?,
                        VectorData::Vector4(v) => write_f16(&mut buffer0, &v[i])?,
                    }
                    match &data.tangents.data {
                        VectorData::Vector2(v) => write_f16(&mut buffer0, &v[i])?,
                        VectorData::Vector3(v) => write_f16(&mut buffer0, &v[i])?,
                        VectorData::Vector4(v) => write_f16(&mut buffer0, &v[i])?,
                    }
                    // TODO: Write binormal.
                }

                // The first buffer has interleaved data for the texture coordinates and colorsets.
                // The attributes differ between meshes, but texture coordinates always precede colorsets.
                // Vertex, Data
                // 0: map1, colorSet1
                // 1: map1, colorSet1
                // ...
                // TODO: Make sure all arrays have the same length.
                for i in 0..data.positions.length() {
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

fn write_f32(writer: &mut Cursor<Vec<u8>>, data: &[f32]) -> Result<(), Box<dyn Error>> {
    for component in data {
        writer.write(&component.to_le_bytes())?;
    }
    Ok(())
}

fn write_u8(writer: &mut Cursor<Vec<u8>>, data: &[f32]) -> Result<(), Box<dyn Error>> {
    for component in data {
        writer.write(&[get_u8_clamped(*component)])?;
    }
    Ok(())
}

fn write_f16(writer: &mut Cursor<Vec<u8>>, data: &[f32]) -> Result<(), Box<dyn Error>> {
    for component in data {
        writer.write(&f16::from_f32(*component).to_le_bytes())?;
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
                while let Ok(influence) = reader.read_le::<VertexWeightV10>() {
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
            .filter(|a| a.usage == usage)
            .map(|a| a.into())
            .collect(),
        MeshAttributes::AttributesV10(attributes_v10) => attributes_v10
            .elements
            .iter()
            .filter(|a| a.usage == usage)
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

    #[test]
    fn create_attributes_mesh_v1_8() {
        let data = MeshObjectData {
            name: "name".into(),
            sub_index: 0,
            parent_bone_name: "".into(),
            vertex_indices: Vec::new(),
            positions: AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            },
            normals: AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            },
            tangents: AttributeData {
                name: "t0".into(),
                data: VectorData::Vector4(Vec::new()),
            },
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
                // Check buffer 0.
                let a0 = &a.elements[0];
                assert_eq!(AttributeUsage::Position, a0.usage);
                assert_eq!(0, a0.buffer_index);
                assert_eq!(0, a0.buffer_offset);
                assert_eq!(0, a0.sub_index);

                let a1 = &a.elements[1];
                assert_eq!(AttributeUsage::Normal, a1.usage);
                assert_eq!(0, a1.buffer_index);
                assert_eq!(12, a1.buffer_offset);
                assert_eq!(0, a1.sub_index);

                let a2 = &a.elements[2];
                assert_eq!(AttributeUsage::Tangent, a2.usage);
                assert_eq!(0, a2.buffer_index);
                assert_eq!(24, a2.buffer_offset);
                assert_eq!(0, a2.sub_index);

                // Check buffer 1.
                let a3 = &a.elements[3];
                assert_eq!(AttributeUsage::TextureCoordinate, a3.usage);
                assert_eq!(1, a3.buffer_index);
                assert_eq!(0, a3.buffer_offset);
                assert_eq!(0, a3.sub_index);

                let a4 = &a.elements[4];
                assert_eq!(AttributeUsage::TextureCoordinate, a4.usage);
                assert_eq!(1, a4.buffer_index);
                assert_eq!(8, a4.buffer_offset);
                assert_eq!(1, a4.sub_index);

                let a5 = &a.elements[5];
                assert_eq!(AttributeUsage::ColorSet, a5.usage);
                assert_eq!(1, a5.buffer_index);
                assert_eq!(16, a5.buffer_offset);
                assert_eq!(0, a5.sub_index);

                let a6 = &a.elements[6];
                assert_eq!(AttributeUsage::ColorSet, a6.usage);
                assert_eq!(1, a6.buffer_index);
                assert_eq!(20, a6.buffer_offset);
                assert_eq!(1, a6.sub_index);
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
            positions: AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            },
            normals: AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            },
            tangents: AttributeData {
                name: "t0".into(),
                data: VectorData::Vector4(Vec::new()),
            },
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
        assert_eq!(32, stride0);
        assert_eq!(16, stride1);

        match attributes {
            MeshAttributes::AttributesV10(a) => {
                // Check buffer 0.
                let a0 = &a.elements[0];
                assert_eq!(AttributeUsage::Position, a0.usage);
                assert_eq!(0, a0.buffer_index);
                assert_eq!(0, a0.buffer_offset);
                assert_eq!(0, a0.sub_index);
                assert_eq!("p0", a0.name.get_string().unwrap());
                assert_eq!("p0", a0.attribute_names.elements[0].get_string().unwrap());

                let a1 = &a.elements[1];
                assert_eq!(AttributeUsage::Normal, a1.usage);
                assert_eq!(0, a1.buffer_index);
                assert_eq!(12, a1.buffer_offset);
                assert_eq!(0, a1.sub_index);
                assert_eq!("n0", a1.name.get_string().unwrap());
                assert_eq!("n0", a1.attribute_names.elements[0].get_string().unwrap());

                let a2 = &a.elements[2];
                assert_eq!(AttributeUsage::Tangent, a2.usage);
                assert_eq!(0, a2.buffer_index);
                assert_eq!(24, a2.buffer_offset);
                assert_eq!(0, a2.sub_index);
                // Using "map1" is a convention with the format for some reason.
                assert_eq!("map1", a2.name.get_string().unwrap());
                assert_eq!("t0", a2.attribute_names.elements[0].get_string().unwrap());

                // Check buffer 1.
                let a3 = &a.elements[3];
                assert_eq!(AttributeUsage::TextureCoordinate, a3.usage);
                assert_eq!(1, a3.buffer_index);
                assert_eq!(0, a3.buffer_offset);
                assert_eq!(0, a3.sub_index);
                assert_eq!("firstUv", a3.name.get_string().unwrap());
                assert_eq!(
                    "firstUv",
                    a3.attribute_names.elements[0].get_string().unwrap()
                );

                let a4 = &a.elements[4];
                assert_eq!(AttributeUsage::TextureCoordinate, a4.usage);
                assert_eq!(1, a4.buffer_index);
                assert_eq!(4, a4.buffer_offset);
                assert_eq!(1, a4.sub_index);
                assert_eq!("secondUv", a4.name.get_string().unwrap());
                assert_eq!(
                    "secondUv",
                    a4.attribute_names.elements[0].get_string().unwrap()
                );

                let a5 = &a.elements[5];
                assert_eq!(AttributeUsage::ColorSet, a5.usage);
                assert_eq!(1, a5.buffer_index);
                assert_eq!(8, a5.buffer_offset);
                assert_eq!(0, a5.sub_index);
                assert_eq!("color1", a5.name.get_string().unwrap());
                assert_eq!(
                    "color1",
                    a5.attribute_names.elements[0].get_string().unwrap()
                );

                let a6 = &a.elements[6];
                assert_eq!(AttributeUsage::ColorSet, a6.usage);
                assert_eq!(1, a6.buffer_index);
                assert_eq!(12, a6.buffer_offset);
                assert_eq!(1, a6.sub_index);
                assert_eq!("color2", a6.name.get_string().unwrap());
                assert_eq!(
                    "color2",
                    a6.attribute_names.elements[0].get_string().unwrap()
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
