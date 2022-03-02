use std::io::Cursor;

use ssbh_write::SsbhWrite;

#[test]
fn pad_enum() {
    #[derive(Debug, SsbhWrite)]
    #[ssbhwrite(pad_after = 3)]
    enum TestEnum {
        A(u8),
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A(1)
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn pad_enum_variant() {
    #[derive(Debug, SsbhWrite)]
    enum TestEnum {
        #[ssbhwrite(pad_after = 3)]
        A(u8),
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A(1)
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn pad_enum_variant_named_fields() {
    #[derive(Debug, SsbhWrite)]
    enum TestEnum {
        #[ssbhwrite(pad_after = 3)]
        A { x: u8 },
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A { x: 1 }
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn pad_struct() {
    #[derive(Debug, SsbhWrite)]
    #[ssbhwrite(pad_after = 3)]
    struct TestStruct {
        x: u8,
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestStruct { x: 1 }
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn pad_struct_field() {
    #[derive(Debug, SsbhWrite)]
    struct TestStruct {
        x: u8,
        #[ssbhwrite(pad_after = 2)]
        y: u8,
        z: u8,
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestStruct { x: 1, y: 2, z: 3 }
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 2u8, 0u8, 0u8, 3u8], writer.into_inner());
}
