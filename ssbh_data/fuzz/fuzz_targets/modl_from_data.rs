#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::modl_data::ModlData| {
    ssbh_lib::formats::modl::Modl::try_from(data);
});
