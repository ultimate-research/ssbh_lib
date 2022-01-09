#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: ssbh_lib::formats::mesh::Mesh| {
    ssbh_lib_fuzz::test_write_read_write(&ssbh_lib::Versioned { data });
});
