#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::meshex_data::MeshExData| {
    ssbh_lib::formats::meshex::MeshEx::from(data);
});
