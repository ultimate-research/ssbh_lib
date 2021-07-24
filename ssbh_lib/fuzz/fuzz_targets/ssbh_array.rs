#![no_main]
use libfuzzer_sys::fuzz_target;
use binread::BinReaderExt;

fuzz_target!(|data: &[u8]| {
    // Test that this doesn't panic on errors.
    let mut reader = std::io::Cursor::new(data);
    let _result = reader.read_le::<ssbh_lib::SsbhArray::<u32>>();
});