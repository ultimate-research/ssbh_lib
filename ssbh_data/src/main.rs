use ssbh_lib::{SsbhFile};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    match ssbh.data {
        SsbhFile::Mesh(mesh) => {
            for object in &mesh.objects.elements {
                let data = ssbh_data::mesh_data::read_positions(&mesh, &object).unwrap();
                println!("{:?}", data);
            }
        }
        _ => (),
    }
}
