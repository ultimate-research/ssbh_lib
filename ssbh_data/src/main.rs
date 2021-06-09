use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut ssbh = ssbh_lib::Ssbh::from_file(&args[1]).unwrap();
    if let SsbhFile::Mesh(mesh) = &mut ssbh.data {
        let objects = ssbh_data::mesh_data::read_mesh_objects(&mesh).unwrap();
        let new_mesh =
            ssbh_data::mesh_data::create_mesh(mesh.major_version, mesh.minor_version, &objects)
                .unwrap();
        new_mesh.write_to_file(&args[2]).unwrap();
    }
}
