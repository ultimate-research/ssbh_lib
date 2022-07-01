#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_lib::formats::adj::Adj| {
    ssbh_data::adj_data::AdjData::try_from(data);
});
