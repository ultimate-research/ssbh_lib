#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_lib::formats::matl::Matl| {
    ssbh_data::matl_data::MatlData::try_from(data);
});
