use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut ssbh = ssbh_lib::Ssbh::from_file(&args[1]).unwrap();
    match &mut ssbh.data {
        SsbhFile::Mesh(mesh) => {
            let objects = ssbh_data::mesh_data::read_mesh_objects(&mesh).unwrap();
            ssbh_data::mesh_data::update_mesh(mesh, &objects).unwrap();
            ssbh_lib::write_mesh_to_file(&args[2], mesh).unwrap();
        }
        _ => (),
    }
}
