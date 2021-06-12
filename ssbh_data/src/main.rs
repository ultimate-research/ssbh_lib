use ssbh_data::mesh_data::MeshData;
use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::Ssbh::from_file(&args[1]).unwrap();
    match &ssbh.data {
        SsbhFile::Mesh(mesh) => {
            let objects = ssbh_data::mesh_data::read_mesh_objects(&mesh).unwrap();
            let new_mesh = ssbh_data::mesh_data::create_mesh(&MeshData {
                major_version: mesh.major_version,
                minor_version: mesh.minor_version,
                objects,
            })
            .unwrap();
            new_mesh.write_to_file(&args[2]).unwrap();
        }
        SsbhFile::Skel(skel) => {
            let data: ssbh_data::skel_data::SkelData = skel.into();
            let new_skel = ssbh_data::skel_data::create_skel(&data);
            new_skel.write_to_file(&args[2]).unwrap();
        }
        _ => (),
    }
}
