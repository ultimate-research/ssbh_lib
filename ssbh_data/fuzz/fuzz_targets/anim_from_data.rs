#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::anim_data::AnimData| {
    ssbh_lib::formats::anim::Anim::try_from(data);
});
