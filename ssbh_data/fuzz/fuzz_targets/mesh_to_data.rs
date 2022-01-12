#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_lib::formats::mesh::Mesh| {
    ssbh_data::mesh_data::MeshData::try_from(data);
});
