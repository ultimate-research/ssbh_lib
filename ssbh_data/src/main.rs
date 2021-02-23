use ssbh_data::mesh_data::{read_first_attribute_with_usage, read_normals, read_positions};
use ssbh_lib::{SsbhFile, formats::mesh::AttributeUsage};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    match ssbh.data {
        SsbhFile::Mesh(mesh) => {
            for mesh_object in &mesh.objects.elements {
                let elements = read_first_attribute_with_usage(&mesh, &mesh_object, AttributeUsage::TextureCoordinate).unwrap();
                for (x, y, z) in elements {
                    println!("{:?},{:?},{:?}", x, y, z);
                }

                // TODO: Assume triangles?
                // let vertex_index_data =
                //     ssbh_data::mesh_data::read_vertex_indices(&mesh, mesh_object);
            }
        }
        _ => (),
    }
}
