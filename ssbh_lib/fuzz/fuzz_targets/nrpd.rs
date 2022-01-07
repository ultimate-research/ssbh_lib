#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: ssbh_lib::formats::nrpd::Nrpd| {
    ssbh_lib_fuzz::test_write_read_write(&data);
});
