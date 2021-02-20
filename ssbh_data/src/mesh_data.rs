use std::error::Error;

use binread::io::{Seek, SeekFrom};
use binread::BinReaderExt;
use binread::{io::Cursor, BinResult};
use half::f16;
use ssbh_lib::formats::mesh::{AttributeDataType, Mesh, MeshAttributeV10, MeshObject};

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

// TODO: Handle other types?
pub fn read_positions(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    // TODO: It isn't safe to assume attribute indices reflect their usage (position, normal, etc).
    // TODO: Don't hardcode the data type.
    match &mesh_object.attributes {
        ssbh_lib::formats::mesh::MeshAttributes::AttributesV8(attributes_v8) => {
            let position = &attributes_v8.elements[0];
            read_attribute_data(
                position.buffer_index,
                position.buffer_offset,
                AttributeDataType::Float,
                mesh,
                mesh_object,
            )
        }
        ssbh_lib::formats::mesh::MeshAttributes::AttributesV10(attributes_v10) => {
            let position = &attributes_v10.elements[0];
            read_attribute_data(
                position.buffer_index,
                position.buffer_offset,
                AttributeDataType::Float,
                mesh,
                mesh_object,
            )
        }
    }
}

pub fn read_normals(
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Result<Vec<(f32, f32, f32)>, Box<dyn Error>> {
    // TODO: It isn't safe to assume attribute indices reflect their usage (position, normal, etc).
    // TODO: Don't hardcode the data type.
    match &mesh_object.attributes {
        ssbh_lib::formats::mesh::MeshAttributes::AttributesV8(attributes_v8) => {
            let position = &attributes_v8.elements[1];
            read_attribute_data(
                position.buffer_index,
                position.buffer_offset,
                AttributeDataType::HalfFloat,
                mesh,
                mesh_object,
            )
        }
        ssbh_lib::formats::mesh::MeshAttributes::AttributesV10(attributes_v10) => {
            let position = &attributes_v10.elements[1];
            read_attribute_data(
                position.buffer_index,
                position.buffer_offset,
                AttributeDataType::HalfFloat,
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

// TODO: Use a trait instead to allow passing in both attribute types?
// TODO: Support values other than position?
pub fn read_attribute_data(
    buffer_index: u32,
    buffer_offset: u32,
    attribute_data_type: AttributeDataType,
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

        // TODO: Use the attribute data type as well as the component count.
        // Component count is based on the attribute name.
        let element = match attribute_data_type {
            AttributeDataType::Float => (
                reader.read_le::<f32>()?,
                reader.read_le::<f32>()?,
                reader.read_le::<f32>()?,
            ),
            AttributeDataType::Byte => {
                // TODO:
                (0f32, 0f32, 0f32)
            }
            AttributeDataType::HalfFloat => (
                read_half(&mut reader)?,
                read_half(&mut reader)?,
                read_half(&mut reader)?,
            ),
            AttributeDataType::HalfFloat2 => (
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
