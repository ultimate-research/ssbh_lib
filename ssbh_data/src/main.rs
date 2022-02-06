use std::convert::TryInto;

use ssbh_data::{
    anim_data::AnimData, matl_data::MatlData, mesh_data::MeshData, skel_data::SkelData, SsbhData,
};
use ssbh_lib::Ssbh;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::SsbhFile::from_file(&args[1]).unwrap();
    match &ssbh.data {
        Ssbh::Mesh(mesh) => {
            let start = std::time::Instant::now();
            let data: MeshData = (&mesh.data).try_into().unwrap();
            println!("Mesh -> MeshData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("MeshData -> Mesh -> File: {:?}", start.elapsed());
        }
        Ssbh::Skel(skel) => {
            let start = std::time::Instant::now();
            let data: SkelData = (&skel.data).into();
            println!("Skel -> SkelData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("SkelData -> Skel -> File: {:?}", start.elapsed());
        }
        Ssbh::Anim(anim) => {
            let start = std::time::Instant::now();
            let data: AnimData = (&anim.data).try_into().unwrap();
            println!("Anim -> AnimData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("AnimData -> Anim -> File: {:?}", start.elapsed());
        }
        Ssbh::Matl(matl) => {
            let start = std::time::Instant::now();
            let data: MatlData = (&matl.data).try_into().unwrap();
            println!("Matl -> MatlData: {:?}", start.elapsed());

            let start = std::time::Instant::now();
            data.write_to_file(&args[2]).unwrap();
            println!("MatlData -> Matl -> File: {:?}", start.elapsed());
        }
        _ => (),
    }
}
