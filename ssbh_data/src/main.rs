use std::convert::TryInto;

use ssbh_data::{SsbhData, anim_data::AnimData, mesh_data::MeshData, skel_data::SkelData};
use ssbh_lib::SsbhFile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::Ssbh::from_file(&args[1]).unwrap();
    match &ssbh.data {
        SsbhFile::Mesh(mesh) => {
            let start = std::time::Instant::now();
            let data: MeshData = mesh.try_into().unwrap();
            println!("Mesh -> MeshData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("MeshData -> Mesh -> File: {:?}", start.elapsed());
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
