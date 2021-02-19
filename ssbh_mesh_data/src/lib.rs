use binread::io::Cursor;
use binread::io::{Seek, SeekFrom};
use binread::BinReaderExt;
use ssbh_lib::{
    formats::mesh::{Mesh, MeshAttributeV10, MeshObject},
    Vector3,
};

// TODO: develop a nicer public API.
// This is just placeholder for now.

pub fn get_vertex_index_data<'a>(mesh: &'a Mesh, mesh_object: &MeshObject) -> Option<&'a [u8]> {
    // TODO: This should probably return the type as well.
    let element_size = match mesh_object.draw_element_type {
        ssbh_lib::formats::mesh::DrawElementType::UnsignedShort => 2,
        ssbh_lib::formats::mesh::DrawElementType::UnsignedInt => 4,
    };

    // Calculate the start and end offset for the vertex indices in the byte buffer.
    let start = mesh_object.element_offset as usize;
    let end = start + mesh_object.vertex_index_count as usize * element_size as usize;
    mesh.polygon_buffer.elements.get(start..end)
}

// TODO: Handle errors.
pub fn read_vertex_indices(mesh: &Mesh, mesh_object: &MeshObject) -> Vec<(u32, u32, u32)> {
    let mut indices = Vec::new();

    let vertex_index_data = get_vertex_index_data(&mesh, &mesh_object).unwrap();
    let mut reader = Cursor::new(&vertex_index_data);

    match mesh_object.draw_element_type {
        ssbh_lib::formats::mesh::DrawElementType::UnsignedShort => {
            // TODO: nicer way to assume triangle data other than dividing by 3?
            // Convert to a larger type for convenience.
            // Performance critical applications will want to use the raw buffers as is.
            for _ in 0..(mesh_object.vertex_index_count / 3) {
                let v0 = reader.read_le::<u16>().unwrap() as u32;
                let v1 = reader.read_le::<u16>().unwrap() as u32;
                let v2 = reader.read_le::<u16>().unwrap() as u32;
                indices.push((v0, v1, v2));
            }
        }
        ssbh_lib::formats::mesh::DrawElementType::UnsignedInt => {
            for _ in 0..(mesh_object.vertex_index_count / 3) {
                let v0 = reader.read_le::<u32>().unwrap();
                let v1 = reader.read_le::<u32>().unwrap();
                let v2 = reader.read_le::<u32>().unwrap();
                indices.push((v0, v1, v2));
            }
        }
    }

    indices
}

// TODO: Use a trait instead to allow passing in both attribute types?
pub fn read_attribute_data(
    buffer_index: u32,
    buffer_offset: u32,
    mesh: &Mesh,
    mesh_object: &MeshObject,
) -> Vec<Vector3> {
    // TODO: create a read_attribute function and return result?
    // Get the raw data for the attribute for this mesh object.
    let attribute_buffer = mesh
        .vertex_buffers
        .elements
        .get(buffer_index as usize)
        .unwrap();

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
    reader.seek(SeekFrom::Start(data_offset)).unwrap();

    for i in 0..mesh_object.vertex_count as u64 {
        reader
            .seek(SeekFrom::Start(data_offset + i * stride))
            .unwrap();

        // TODO: Use the attribute data type as well as the component count.
        // Component count is based on the attribute name.
        let element = reader.read_le::<Vector3>().unwrap();
        elements.push(element);
    }

    elements
}

pub fn get_attribute_name(attribute: &MeshAttributeV10) -> Option<&str> {
    attribute
        .attribute_names
        .elements
        .iter()
        .next()
        .unwrap()
        .get_string()
}
