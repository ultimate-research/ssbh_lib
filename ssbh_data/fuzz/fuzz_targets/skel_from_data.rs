#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::skel_data::SkelData| {
    ssbh_lib::formats::skel::Skel::try_from(data);
});
