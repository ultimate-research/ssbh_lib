use std::io::Cursor;

use ssbh_write::SsbhWrite;

#[test]
fn repr_u32() {
    #[derive(Debug, SsbhWrite, Clone, Copy)]
    #[ssbhwrite(repr(u32))]
    enum TestEnum {
        A = 1,
        B = 2,
    }

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;

    TestEnum::A.ssbh_write(&mut writer, &mut data_ptr).unwrap();
    TestEnum::B.ssbh_write(&mut writer, &mut data_ptr).unwrap();

    assert_eq!(
        vec![1u8, 0u8, 0u8, 0u8, 2u8, 0u8, 0u8, 0u8],
        writer.into_inner()
    );
    assert_eq!(8, data_ptr);

    assert_eq!(
        std::mem::align_of::<u32>(),
        TestEnum::alignment_in_bytes() as usize
    );
}
