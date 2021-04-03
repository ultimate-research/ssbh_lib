use std::{
    error::Error,
    io::{Read, Write},
    ops::Mul,
};

use binread::BinReaderExt;
use binread::{io::Cursor, BinRead};
use binread::{
    io::{Seek, SeekFrom},
    BinResult,
};
use half::f16;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataType, AttributeDataTypeV8, AttributeUsage, DrawElementType, Mesh,
        MeshAttributeV10, MeshAttributeV8, MeshAttributes, MeshObject, MeshRiggingGroup,
    },
    SsbhByteBuffer,
};
use ssbh_lib::{Half, SsbhArray};

pub enum DataType {
    Byte,
    Float,
    HalfFloat,
}

#[derive(BinRead, Debug)]
pub struct VertexWeight {
    vertex_index: i16,
    vertex_weight: f32,
}

impl From<AttributeDataType> for DataType {
    fn from(value: AttributeDataType) -> Self {
        match value {
            AttributeDataType::Float3 => Self::Float,
            AttributeDataType::Byte4 => Self::Byte,
            AttributeDataType::HalfFloat4 => Self::HalfFloat,
            AttributeDataType::HalfFloat2 => Self::HalfFloat,
        }
    }
}

impl From<AttributeDataTypeV8> for DataType {
    fn from(value: AttributeDataTypeV8) -> Self {
        match value {
            AttributeDataTypeV8::Float3 => Self::Float,
            AttributeDataTypeV8::Float2 => Self::Float,
            AttributeDataTypeV8::Byte4 => Self::Byte,
            AttributeDataTypeV8::HalfFloat4 => Self::HalfFloat,
        }
    }
}

/// Read the vertex indices from the buffer in `mesh` for the specified `mesh_object`.
/// Index values are converted to `u32` regardless of the actual data type.
pub fn read_vertex_indices(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<u32>, Box<dyn Error>> {
    let mut reader = Cursor::new(&mesh.index_buffer.elements);
    reader.seek(SeekFrom::Start(mesh_object.index_buffer_offset as u64))?;

    let count = mesh_object.vertex_index_count;
    let indices = match mesh_object.draw_element_type {
        DrawElementType::UnsignedShort => read_indices::<u16>(&mut reader, count),
        DrawElementType::UnsignedInt => read_indices::<u32>(&mut reader, count),
    };

    Ok(indices?)
}

fn read_indices<T: BinRead + Into<u32>>(
    reader: &mut Cursor<&Vec<u8>>,
    count: u32,
) -> BinResult<Vec<u32>> {
    let mut indices = Vec::new();

    for _ in 0..count {
        let index = reader.read_le::<T>()?;
        indices.push(index.into());
    }

    Ok(indices)
}

/// This enforces the component count at compile time.
/// Position0 attributes are always assumed to have 3 components, for example.
/// Technically, the component count for an attribute can only be determined at runtime based on the attribute's data type.
fn read_attribute_data<T, const N: usize>(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    attribute: &MeshAttribute,
) -> Result<Vec<[f32; N]>, Box<dyn Error>> {
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
        DataType::Byte => read_data::<_, u8, N>(&mut reader, count, offset, stride),
        DataType::Float => read_data::<_, f32, N>(&mut reader, count, offset, stride),
        DataType::HalfFloat => read_data::<_, Half, N>(&mut reader, count, offset, stride),
    }?;

    Ok(data)
}

fn read_data<R: Read + Seek, T: Into<f32> + BinRead, const N: usize>(
    reader: &mut R,
    count: usize,
    offset: u64,
    stride: u64,
) -> Result<Vec<[f32; N]>, Box<dyn Error>> {
    let mut result = Vec::new();
    for i in 0..count as u64 {
        // The data type may be smaller than stride to allow interleaving different attributes.
        reader.seek(SeekFrom::Start(offset + i * stride))?;

        let mut element = [0f32; N];
        for j in 0..N {
            element[j] = reader.read_le::<T>()?.into();
        }
        result.push(element);
    }
    Ok(result)
}

/// Read the vertex positions for the specified `mesh_object`.
pub fn read_positions(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<AttributeData<3>, Box<dyn Error>> {
    let attributes = get_attributes(&mesh_object, AttributeUsage::Position);
    let buffer_access = attributes.first().ok_or("No position attribute found.")?;
    let data = read_attribute_data::<f32, 3>(mesh, mesh_object, buffer_access)?;
    Ok(AttributeData::<3> {
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
) -> Result<Vec<AttributeData<2>>, Box<dyn Error>> {
    let mut attributes = Vec::new();
    for attribute in &get_attributes(&mesh_object, AttributeUsage::TextureCoordinate) {
        let mut data = read_attribute_data::<f32, 2>(mesh, mesh_object, attribute)?;
        if flip_vertical {
            for element in data.iter_mut() {
                element[1] = 1.0 - element[1];
            }
        }
        attributes.push(AttributeData::<2> {
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
) -> Result<Vec<AttributeData<4>>, Box<dyn Error>> {
    // TODO: Find a cleaner way to do this (define a new enum?).
    let colorsets_v10 = get_attributes(&mesh_object, AttributeUsage::ColorSet);
    let colorsets_v8 = get_attributes(&mesh_object, AttributeUsage::ColorSetV8);

    let mut attributes = Vec::new();
    for attribute in colorsets_v10.iter().chain(colorsets_v8.iter()) {
        let mut data = read_attribute_data::<f32, 4>(mesh, mesh_object, attribute)?;

        if divide_by_2 {
            // Map the range [0.0, 255.0] to [0.0, 2.0].
            for element in data.iter_mut() {
                element[0] /= 128.0;
                element[1] /= 128.0;
                element[2] /= 128.0;
                element[3] /= 128.0;
            }
        } else {
            // Map the range [0.0, 255.0] to [0.0, 1.0].
            for element in data.iter_mut() {
                element[0] /= 255.0;
                element[1] /= 255.0;
                element[2] /= 255.0;
                element[3] /= 255.0;
            }
        }

        attributes.push(AttributeData::<4> {
            name: attribute.name.clone(),
            data,
        });
    }

    Ok(attributes)
}

pub fn read_normals(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<AttributeData<3>, Box<dyn Error>> {
    let attributes = get_attributes(&mesh_object, AttributeUsage::Normal);
    let attribute = attributes.first().ok_or("No normals attribute found.")?;
    let data = read_attribute_data::<f32, 3>(mesh, mesh_object, attribute)?;
    Ok(AttributeData::<3> {
        name: attribute.name.clone(),
        data,
    })
}

pub fn read_tangents(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<AttributeData<4>, Box<dyn Error>> {
    let attributes = get_attributes(&mesh_object, AttributeUsage::Tangent);
    let attribute = attributes.first().ok_or("No tangent attribute found.")?;
    let data = read_attribute_data::<f32, 4>(mesh, mesh_object, attribute)?;
    Ok(AttributeData::<4> {
        name: attribute.name.clone(),
        data,
    })
}

#[derive(Debug)]
pub struct MeshObjectRiggingData {
    pub mesh_object_name: String,
    pub mesh_sub_index: u64,
    pub bone_influences: Vec<BoneInfluence>,
}

/// Reads the rigging data for the specified `mesh`. Rigging data is not a indluded with the `MeshObject`,
/// so each element of the output will need to be associated with the `MeshObject` with matching `name` and `sub_index`.
/// Each vertex will likely be influenced by at most 4 bones, but the format doesn't enforce this.
pub fn read_rigging_data(mesh: &Mesh) -> Result<Vec<MeshObjectRiggingData>, Box<dyn Error>> {
    let mut mesh_object_rigging_data = Vec::new();

    for rigging_group in &mesh.rigging_buffers.elements {
        let mesh_object_name = rigging_group
            .mesh_object_name
            .get_string()
            .ok_or("Failed to read mesh object name.")?;
        let mesh_sub_index = rigging_group.mesh_object_sub_index;

        let bone_influences = read_influences(&rigging_group)?;

        // TODO: Store max influences?
        let rigging_data = MeshObjectRiggingData {
            mesh_object_name: mesh_object_name.to_string(),
            mesh_sub_index,
            bone_influences,
        };

        mesh_object_rigging_data.push(rigging_data);
    }

    Ok(mesh_object_rigging_data)
}

#[derive(Debug)]
pub struct BoneInfluence {
    pub bone_name: String,
    pub vertex_weights: Vec<VertexWeight>,
}

#[derive(Debug, Clone)]
pub struct MeshObjectData {
    pub name: String,
    pub sub_index: i64,
    pub parent_bone_name: String,
    pub vertex_indices: Vec<u32>,
    pub positions: AttributeData<3>,
    pub normals: AttributeData<3>,
    pub tangents: AttributeData<4>,
    pub texture_coordinates: Vec<AttributeData<2>>,
    pub color_sets: Vec<AttributeData<4>>,
}

#[derive(Debug, Clone)]
pub struct AttributeData<const N: usize> {
    pub name: String,
    pub data: Vec<[f32; N]>,
}

pub fn read_mesh_objects(mesh: &Mesh) -> Result<Vec<MeshObjectData>, Box<dyn Error>> {
    let mut mesh_objects = Vec::new();

    for mesh_object in &mesh.objects.elements {
        let indices = read_vertex_indices(&mesh, &mesh_object)?;
        let positions = read_positions(&mesh, &mesh_object)?;
        let normals = read_normals(&mesh, &mesh_object)?;
        let tangents = read_tangents(&mesh, &mesh_object)?;
        let texture_coordinates = read_texture_coordinates(&mesh, &mesh_object, true)?;
        let color_sets = read_colorsets(&mesh, &mesh_object, true)?;

        let data = MeshObjectData {
            name: mesh_object.name.get_string().unwrap_or("").to_string(),
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

fn get_size_in_bytes(data_type: &AttributeDataType) -> usize {
    match data_type {
        AttributeDataType::Float3 => std::mem::size_of::<f32>() * 3,
        AttributeDataType::Byte4 => std::mem::size_of::<u8>() * 4,
        AttributeDataType::HalfFloat4 => std::mem::size_of::<f16>() * 4,
        AttributeDataType::HalfFloat2 => std::mem::size_of::<f16>() * 2,
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

fn calculate_stride0(data: &MeshObjectData) -> usize {
    // TODO: Calculate this based on the data types for Position0, Tangent0, Normal0
    0
}

// TODO: Use a struct for the return type?
fn create_mesh_objects(
    source_mesh: &Mesh,
    mesh_object_data: &[MeshObjectData],
) -> Result<(Vec<MeshObject>, Vec<Vec<u8>>, Vec<u8>), Box<dyn Error>> {
    // TODO: Split this into functions and do some cleanup.
    let mut mesh_objects = Vec::new();

    // TODO: Calculate strides for the mesh objects as well?

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
                // TODO: calculate the attribute list.
                let attributes = MeshAttributes::AttributesV10(SsbhArray {
                    elements: Vec::new(),
                });

                // TODO:calculate the strides based on the attributes.

                let mesh_object = MeshObject {
                    name: data.name.clone().into(),
                    sub_index: data.sub_index,
                    parent_bone_name: data.parent_bone_name.clone().into(),
                    vertex_count: data.positions.data.len() as u32,
                    vertex_index_count: data.vertex_indices.len() as u32,
                    unk2: 3, // triangles?
                    vertex_buffer0_offset: buffer0.position() as u32,
                    vertex_buffer1_offset: buffer1.position() as u32,
                    final_buffer_offset,
                    buffer_index: 0, // TODO: This is always 0
                    stride0: 0,      // TODO: Calculate this
                    stride1: 0,      // TODO: Calculate this
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
                for i in 0..data.positions.data.len() {
                    // Assume Normal0 and Tangent0 will use half precision.
                    write_f32(&mut buffer0, &data.positions.data[i])?;
                    write_f16(&mut buffer0, &data.normals.data[i])?;
                    write_f16(&mut buffer0, &data.tangents.data[i])?;
                }

                // The first buffer has interleaved data for the texture coordinates and colorsets.
                // The attributes differ between meshes, but texture coordinates always precede colorsets.
                // Vertex, Data
                // 0: map1, colorSet1
                // 1: map1, colorSet1
                // ...
                // TODO: Make sure all arrays have the same length.
                for i in 0..data.positions.data.len() {
                    // Assume texture coordinates will use half precision.
                    for attribute in &data.texture_coordinates {
                        write_f16(&mut buffer1, &attribute.data[i])?;
                    }

                    // Assume u8 for color sets.
                    for attribute in &data.color_sets {
                        write_u8(&mut buffer1, &attribute.data[i])?;
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

        // TODO: Is there a way to do this with iterators?
        // There's no stored length for the buffer, so read influences until reaching the end.
        let mut influences = Vec::new();
        let mut reader = Cursor::new(&buffer.data.elements);
        while let Ok(influence) = reader.read_le::<VertexWeight>() {
            influences.push(influence);
        }

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
    fn u8_clamped() {
        assert_eq!(0u8, get_u8_clamped(-1.0f32));
        assert_eq!(0u8, get_u8_clamped(0.0f32));
        assert_eq!(128u8, get_u8_clamped(128u8 as f32 / 255f32));
        assert_eq!(255u8, get_u8_clamped(1.0f32));
        assert_eq!(255u8, get_u8_clamped(2.0f32));
    }

    #[test]
    fn size_in_bytes_attributes_v10() {
        assert_eq!(4, get_size_in_bytes(&AttributeDataType::Byte4));
        assert_eq!(12, get_size_in_bytes(&AttributeDataType::Float3));
        assert_eq!(4, get_size_in_bytes(&AttributeDataType::HalfFloat2));
        assert_eq!(8, get_size_in_bytes(&AttributeDataType::HalfFloat4));
    }

    #[test]
    fn size_in_bytes_attributes_v8() {
        assert_eq!(4, get_size_in_bytes_v8(&AttributeDataTypeV8::Byte4));
        assert_eq!(8, get_size_in_bytes_v8(&AttributeDataTypeV8::Float2));
        assert_eq!(12, get_size_in_bytes_v8(&AttributeDataTypeV8::Float3));
        assert_eq!(8, get_size_in_bytes_v8(&AttributeDataTypeV8::HalfFloat4));
    }
}
