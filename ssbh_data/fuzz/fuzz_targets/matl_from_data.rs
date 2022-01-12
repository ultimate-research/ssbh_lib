#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::matl_data::MatlData| {
    ssbh_lib::formats::matl::Matl::try_from(data);
});
