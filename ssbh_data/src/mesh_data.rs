use std::error::Error;

use binread::io::{Seek, SeekFrom};
use binread::BinReaderExt;
use binread::{io::Cursor, BinResult,BinRead};
use half::f16;
use ssbh_lib::formats::mesh::{
    AttributeDataType, AttributeDataTypeV8, AttributeUsage, Mesh, MeshAttributeV10, MeshObject,
};

pub enum DataType {
    Byte,
    Float,
    HalfFloat,
}

#[derive(BinRead, Debug)]
pub struct MeshInfluence {
    vertex_index: i16,
    vertex_weight: f32,
}

impl From<AttributeDataType> for DataType {
    fn from(value: AttributeDataType) -> Self {
        match value {
            AttributeDataType::Float => Self::Float,
            AttributeDataType::Byte => Self::Byte,
            AttributeDataType::HalfFloat => Self::HalfFloat,
            AttributeDataType::HalfFloat2 => Self::HalfFloat,
        }
    }
}

impl From<AttributeDataTypeV8> for DataType {
    fn from(value: AttributeDataTypeV8) -> Self {
        match value {
            AttributeDataTypeV8::Float => Self::Float,
            AttributeDataTypeV8::Float2 => Self::Float,
            AttributeDataTypeV8::Byte => Self::Byte,
            AttributeDataTypeV8::HalfFloat => Self::HalfFloat,
        }
    }
}

/// Read the vertex indices from the buffer in `mesh` for the specified `mesh_object`.
/// Index values are converted to `u32` regardless of the actual data type.
pub fn read_vertex_indices(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<u32>, Box<dyn Error>> {
    let mut indices = Vec::new();

    let mut reader = Cursor::new(&mesh.polygon_buffer.elements);
    reader.seek(SeekFrom::Start(mesh_object.element_offset as u64))?;

    match mesh_object.draw_element_type {
        ssbh_lib::formats::mesh::DrawElementType::UnsignedShort => {
            for _ in 0..mesh_object.vertex_index_count {
                let index = reader.read_le::<u16>()? as u32;
                indices.push(index);
            }
        }
        ssbh_lib::formats::mesh::DrawElementType::UnsignedInt => {
            for _ in 0..mesh_object.vertex_index_count {
                let index = reader.read_le::<u32>()?;
                indices.push(index);
            }
        }
    }

    Ok(indices)
}

pub fn read_positions(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    read_first_attribute_with_usage(&mesh, &mesh_object, AttributeUsage::Position)
}

pub fn read_texture_coordinates(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    read_first_attribute_with_usage(&mesh, &mesh_object, AttributeUsage::TextureCoordinate)
}

pub fn read_normals(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    read_first_attribute_with_usage(&mesh, &mesh_object, AttributeUsage::Normal)
}

/// Reads data for the first mesh attribute with the given `usage`. 
pub fn read_first_attribute_with_usage(
    mesh: &Mesh,
    mesh_object: &MeshObject,
    usage: AttributeUsage
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    match &mesh_object.attributes {
        ssbh_lib::formats::mesh::MeshAttributes::AttributesV8(attributes_v8) => {
            let attribute = &attributes_v8
                .elements
                .iter()
                .find(|a| a.usage == usage)
                .ok_or("No attribute with the given usage found.")?;
            read_attribute_data(
                attribute.buffer_index,
                attribute.buffer_offset,
                attribute.data_type.into(),
                mesh,
                mesh_object,
            )
        }
        ssbh_lib::formats::mesh::MeshAttributes::AttributesV10(attributes_v10) => {
            let attribute = &attributes_v10
                .elements
                .iter()
                .find(|a| a.usage == usage)
                .ok_or("No attribute with the given usage found.")?;
            read_attribute_data(
                attribute.buffer_index,
                attribute.buffer_offset,
                attribute.data_type.into(),
                mesh,
                mesh_object,
            )
        }
    }
}

fn read_half(reader: &mut Cursor<&Vec<u8>>) -> BinResult<f32> {
    let value = f16::from_bits(reader.read_le::<u16>()?).to_f32();
    Ok(value)
}

pub fn read_attribute_data(
    buffer_index: u32,
    buffer_offset: u32,
    attribute_data_type: DataType,
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    // Get the raw data for the attribute for this mesh object.
    let attribute_buffer = mesh
        .vertex_buffers
        .elements
        .get(buffer_index as usize)
        .ok_or("Invalid buffer index.")?;

    // TODO: Handle invalid indices and return some sort of error.
    let data_offset = if buffer_index == 0 {
        buffer_offset + mesh_object.vertex_offset
    } else {
        buffer_offset + mesh_object.vertex_offset2
    } as u64;

    let stride = if buffer_index == 0 {
        mesh_object.stride
    } else {
        mesh_object.stride2
    } as u64;

    // TODO: is this correct?
    let mut elements = Vec::new();

    // Stride may be larger than the size of the element being read to allow for interleaving data,
    // so it may not work to simply do buffer[offset..offset+count*stride].
    let mut reader = Cursor::new(&attribute_buffer.elements);
    reader.seek(SeekFrom::Start(data_offset))?;

    // TODO: use generics and move condition out of the loop?
    for i in 0..mesh_object.vertex_count as u64 {
        reader.seek(SeekFrom::Start(data_offset + i * stride))?;

        // TODO: Component count is based on the attribute name.
        let element = match attribute_data_type {
            DataType::Float => (
                reader.read_le::<f32>()?,
                reader.read_le::<f32>()?,
                reader.read_le::<f32>()?,
            ),
            DataType::Byte => (
                reader.read_le::<u8>()? as f32 / 255f32,
                reader.read_le::<u8>()? as f32 / 255f32,
                reader.read_le::<u8>()? as f32 / 255f32,
            ),
            DataType::HalfFloat => (
                read_half(&mut reader)?,
                read_half(&mut reader)?,
                read_half(&mut reader)?,
            ),
        };
        elements.push(element);
    }

    Ok(elements)
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
