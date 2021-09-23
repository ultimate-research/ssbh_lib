use std::convert::TryInto;

use ssbh_data::{anim_data::AnimData, mesh_data::MeshData, skel_data::SkelData};
use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::Ssbh::from_file(&args[1]).unwrap();
    match &ssbh.data {
        SsbhFile::Mesh(mesh) => {
            // TODO: Structure this like the other types.
            let objects = ssbh_data::mesh_data::read_mesh_objects(mesh).unwrap();
            let new_mesh = ssbh_data::mesh_data::create_mesh(&MeshData {
                major_version: mesh.major_version,
                minor_version: mesh.minor_version,
                objects,
            })
            .unwrap();
            new_mesh.write_to_file(&args[2]).unwrap();
        }
        SsbhFile::Skel(skel) => {
            let start = std::time::Instant::now();
            let data = SkelData::from(skel);
            println!("Skel -> SkelData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("SkelData -> Skel -> File: {:?}", start.elapsed());
        }
        SsbhFile::Anim(anim) => {
            let start = std::time::Instant::now();
            let data: AnimData = anim.try_into().unwrap();
            println!("Anim -> AnimData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("AnimData -> Anim -> File: {:?}", start.elapsed());
        }
        _ => (),
    }
}
