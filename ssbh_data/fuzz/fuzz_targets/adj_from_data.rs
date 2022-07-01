#![no_main]
use libfuzzer_sys::fuzz_target;
use std::convert::TryFrom;

fuzz_target!(|data: ssbh_data::adj_data::AdjData| {
    ssbh_lib::formats::adj::Adj::try_from(data);
});
