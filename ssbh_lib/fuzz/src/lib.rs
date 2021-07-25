use std::io::Cursor;
use binread::BinReaderExt;

pub fn test_write_read_write<T: binread::BinRead + ssbh_lib::SsbhWrite + serde::Serialize>(input: &T) {
    let mut writer = Cursor::new(Vec::new());
    input.write(&mut writer).unwrap();
    let before = writer.into_inner();

    let mut reader = Cursor::new(before.clone());
    let output = reader.read_le::<T>().unwrap();
    
    let mut writer = Cursor::new(Vec::new());
    output.write(&mut writer).unwrap();
    let after = writer.into_inner();

    assert_eq!(before, after, "{}", serde_json::to_string(&input).unwrap());
}