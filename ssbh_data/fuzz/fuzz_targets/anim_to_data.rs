#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_lib::formats::anim::Anim| {
    ssbh_data::anim_data::AnimData::try_from(data);
});
