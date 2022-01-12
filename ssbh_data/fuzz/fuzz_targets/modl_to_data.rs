#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: ssbh_lib::formats::modl::Modl| {
    ssbh_data::modl_data::ModlData::from(data);
});
