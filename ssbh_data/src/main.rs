use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    match ssbh.data {
        SsbhFile::Mesh(mesh) => {
            for mesh_object in &mesh.objects.elements {
                match &mesh_object.attributes {
                    ssbh_lib::formats::mesh::MeshAttributes::AttributesV8(attributes_v8) => {
                        // TODO: handle the old mesh data as well.
                        // let attribute = attributes_v8.elements.iter().next().unwrap();
                        // let elements = ssbh_mesh_data::read_attribute_data(
                        //     attribute.buffer_index,
                        //     attribute.buffer_offset,
                        //     &mesh,
                        //     &mesh_object,
                        // );
                        // for element in &elements {
                        //     println!("{:?},{:?},{:?}", element.x, element.y, element.z);
                        // }
                    }
                    ssbh_lib::formats::mesh::MeshAttributes::AttributesV10(attributes_v10) => {
                        // TODO: Get the attribute name and buffer offsets for each attribute.
                        for attribute in &attributes_v10.elements {
                            // TODO: avoid unwrap
                            // The actual attribute name is stored in the array.
                            let attribute_name = ssbh_data::mesh_data::get_attribute_name(&attribute);
                            if let Some(text) = attribute_name {
                                if text == "Position0" {
                                    // let elements = ssbh_mesh_data::read_attribute_data(
                                    //     attribute.buffer_index,
                                    //     attribute.buffer_offset,
                                    //     &mesh,
                                    //     &mesh_object,
                                    // );
                                    // for element in &elements {
                                    //     println!("{:?},{:?},{:?}", element.x, element.y, element.z);
                                    // }
                                }
                            }
                        }
                    }
                };

                // TODO: Assume triangles?
                let vertex_index_data = ssbh_data::mesh_data::read_vertex_indices(&mesh, mesh_object);
                for element in vertex_index_data {}
            }
        }
        _ => (),
    }
}
