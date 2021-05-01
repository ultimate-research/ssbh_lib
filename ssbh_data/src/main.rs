use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::Ssbh::from_file(&args[1]).unwrap();
    match ssbh.data {
        SsbhFile::Mesh(mesh) => {
            let objects = ssbh_data::mesh_data::read_mesh_objects(&mesh).unwrap();
            for object in &objects {
                println!("{:?}", object.name);
            }
        }
        _ => (),
    }
}
