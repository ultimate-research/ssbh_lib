use std::io::Cursor;

use ssbh_write::SsbhWrite;

#[test]
fn align_enum() {
    #[derive(Debug, SsbhWrite)]
    #[ssbhwrite(align_after = 3)]
    enum TestEnum {
        A(u8),
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A(1)
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn align_enum_field() {
    #[derive(Debug, SsbhWrite)]
    enum TestEnum {
        A(#[ssbhwrite(align_after = 4)] u8),
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A(1)
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn align_enum_variant() {
    #[derive(Debug, SsbhWrite)]
    enum TestEnum {
        #[ssbhwrite(align_after = 3)]
        A(u8),
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A(1)
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn align_enum_variant_named_fields() {
    #[derive(Debug, SsbhWrite)]
    enum TestEnum {
        #[ssbhwrite(align_after = 4)]
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
fn align_enum_named_field() {
    #[derive(Debug, SsbhWrite)]
    enum TestEnum {
        A {
            #[ssbhwrite(align_after = 3)]
            x: u8,
        },
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A { x: 1 }
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn align_struct() {
    #[derive(Debug, SsbhWrite)]
    #[ssbhwrite(align_after = 3)]
    struct TestStruct {
        x: u8,
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestStruct { x: 1 }
        .ssbh_write(&mut writer, &mut data_ptr)
        .unwrap();

    assert_eq!(vec![1u8, 0u8, 0u8], writer.into_inner());
}

#[test]
fn align_struct_field() {
    #[derive(Debug, SsbhWrite)]
    struct TestStruct {
        x: u8,
        #[ssbhwrite(align_after = 4)]
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
