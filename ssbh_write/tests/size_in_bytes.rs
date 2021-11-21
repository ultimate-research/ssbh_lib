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
fn struct_size() {
    #[derive(Debug, Default, SsbhWrite)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(3, TestStruct::default().size_in_bytes());
}

#[test]
fn vec_and_slice_size() {
    #[derive(Debug, Default, SsbhWrite, Clone)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(
        3 * 5,
        vec![TestStruct::default(); 5].size_in_bytes()
    );
    assert_eq!(
        3 * 5,
        vec![TestStruct::default(); 5].as_slice().size_in_bytes()
    );
}

#[test]
fn array_size() {
    #[derive(Debug, Default, SsbhWrite, Clone, Copy)]
    struct TestStruct {
        x: u8,
        y: u16,
    }

    assert_eq!(
        3 * 7,
        [TestStruct::default(); 7].size_in_bytes()
    );
}
