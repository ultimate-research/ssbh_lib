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
        SsbhFile::Anim(anim) => match &anim.header {
            ssbh_lib::formats::anim::AnimHeader::HeaderV20(header) => {
                for animation in &header.animations.elements {
                    for node in &animation.nodes.elements {
                        for track in &node.tracks.elements {
                            let start = track.data_offset as usize;
                            let end = start + track.data_size as usize;
                            let buffer = &header.buffer.elements[start..end];
                            println!("Node: {:?}, Name: {:?}, Flags: {:?}, Frame Count: {:?}, Size: {:?}, Data: {:?}", node.name.to_string_lossy(), track.name.to_string_lossy(), track.flags, track.frame_count, track.data_size, hex::encode(buffer));
                        }
                    }
                }
            }
            _ => todo!(),
        },
        _ => (),
    }
}
