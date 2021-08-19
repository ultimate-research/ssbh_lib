use ssbh_write::SsbhWrite;

#[test]
fn struct_derive_uses_align_of() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
        y: u16
    }

    let test = TestStruct::default();
    assert_eq!(std::mem::align_of::<TestStruct>(), test.alignment_in_bytes() as usize);
}

// TODO: Test empty slices and vecs.
#[test]
fn vec_and_slice_use_element_alignment() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
        y: u16
    }

    let test = vec![TestStruct::default()];
    assert_eq!(std::mem::align_of::<TestStruct>(), test.alignment_in_bytes() as usize);
    assert_eq!(std::mem::align_of::<TestStruct>(), test.as_slice().alignment_in_bytes() as usize);
}