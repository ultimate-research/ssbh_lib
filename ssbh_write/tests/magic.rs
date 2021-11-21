use std::io::Cursor;

use ssbh_write::SsbhWrite;

#[test]
fn magic_attribute() {
    #[derive(Debug, Default, SsbhWrite)]
    #[ssbhwrite(magic = b"abc")]
    struct TestStruct {
        x: u8,
    }

    let mut writer = Cursor::new(Vec::new());
    TestStruct::default().write(&mut writer).unwrap();
    assert_eq!(vec![97, 98, 99, 0], writer.into_inner());

    // Magic should be treated like a normal field in the computed size.
    assert_eq!(4, TestStruct::default().size_in_bytes())
}

#[test]
fn no_magic_attribute() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
    }

    let mut writer = Cursor::new(Vec::new());
    TestStruct::default().write(&mut writer).unwrap();
    assert_eq!(vec![0u8], writer.into_inner());

    assert_eq!(1, TestStruct::default().size_in_bytes())
}
