use ssbh_write::SsbhWrite;

#[test]
fn alignment_attribute() {
    #[derive(Debug, Default, SsbhWrite)]
    #[ssbhwrite(alignment = 7)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(7, TestStruct::alignment_in_bytes() as usize);
}

#[test]
fn struct_derive_uses_align_of() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(
        std::mem::align_of::<TestStruct>(),
        TestStruct::alignment_in_bytes() as usize
    );
}

#[test]
fn vec_and_slice_use_element_alignment() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(
        std::mem::align_of::<TestStruct>(),
        TestStruct::alignment_in_bytes() as usize
    );
    assert_eq!(
        std::mem::align_of::<TestStruct>(),
        <&[TestStruct]>::alignment_in_bytes() as usize
    );
}

#[test]
fn arrays_use_element_alignment() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(
        std::mem::align_of::<TestStruct>(),
        <[TestStruct; 7]>::alignment_in_bytes() as usize
    );
}
