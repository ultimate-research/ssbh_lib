use ssbh_data::mesh_data::{read_first_attribute_with_usage, read_normals, read_positions, read_rigging_data};
use ssbh_lib::{SsbhFile, formats::mesh::AttributeUsage};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    match ssbh.data {
        SsbhFile::Mesh(mesh) => {
            let rigging_data = read_rigging_data(&mesh).unwrap();
            println!("{:?}", rigging_data);
        }
        _ => (),
    }
}
