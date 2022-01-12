#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_lib::formats::skel::Skel| {
    ssbh_data::skel_data::SkelData::from(data);
});
