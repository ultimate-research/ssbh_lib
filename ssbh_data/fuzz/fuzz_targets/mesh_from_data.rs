#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::mesh_data::MeshData| {
    ssbh_lib::formats::mesh::Mesh::try_from(data);
});
