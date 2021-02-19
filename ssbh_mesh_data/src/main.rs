use binread::io::Cursor;
use binread::io::{Seek, SeekFrom};
use binread::BinReaderExt;
use ssbh_lib::{
    formats::mesh::{Mesh, MeshAttributeV10, MeshObject},
    SsbhFile, Vector3,
};

fn get_vertex_index_data<'a>(mesh: &'a Mesh, mesh_object: &MeshObject) -> Option<&'a [u8]> {
    // TODO: This should probably return the type as well.
    let element_size = match mesh_object.draw_element_type {
        ssbh_lib::formats::mesh::DrawElementType::UnsignedShort => 2,
        ssbh_lib::formats::mesh::DrawElementType::UnsignedInt => 4,
    };

    // Calculate the start and end offset for the vertex indices in the byte buffer.
    let start = mesh_object.element_offset as usize;
    let end = start + mesh_object.vertex_count as usize * element_size as usize;
    mesh.polygon_buffer.elements.get(start..end)
}

// TODO: Use a trait instead to allow passing in both attribute types?
fn read_attribute_data(
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

fn get_attribute_name(attribute: &MeshAttributeV10) -> Option<&str> {
    attribute
        .attribute_names
        .elements
        .iter()
        .next()
        .unwrap()
        .get_string()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    match ssbh.data {
        SsbhFile::Mesh(mesh) => {
            for mesh_object in &mesh.objects.elements {
                match &mesh_object.attributes {
                    ssbh_lib::formats::mesh::MeshAttributes::AttributesV8(attributes_v8) => {
                        // TODO: handle the old mesh data as well.
                        let attribute = attributes_v8.elements.iter().next().unwrap();
                        let elements = read_attribute_data(
                            attribute.buffer_index,
                            attribute.buffer_offset,
                            &mesh,
                            &mesh_object,
                        );
                        for element in &elements {
                            println!("{:?},{:?},{:?}", element.x, element.y, element.z);
                        }
                    }
                    ssbh_lib::formats::mesh::MeshAttributes::AttributesV10(attributes_v10) => {
                        // TODO: Get the attribute name and buffer offsets for each attribute.
                        for attribute in &attributes_v10.elements {
                            // TODO: avoid unwrap
                            // The actual attribute name is stored in the array.
                            let attribute_name = get_attribute_name(&attribute);
                            if let Some(text) = attribute_name {
                                if text == "Position0" {
                                    let elements = read_attribute_data(
                                        attribute.buffer_index,
                                        attribute.buffer_offset,
                                        &mesh,
                                        &mesh_object,
                                    );
                                    for element in &elements {
                                        println!("{:?},{:?},{:?}", element.x, element.y, element.z);
                                    }
                                }
                            }
                        }
                    }
                };

                // let vertex_index_data = get_vertex_index_data(&mesh, &mesh_object);
                // println!("Vertex Indices: {:?}", vertex_index_data.unwrap().len());
            }
        }
        _ => (),
    }
}
