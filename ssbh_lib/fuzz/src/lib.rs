use binrw::io::Cursor;
use binrw::BinReaderExt;

pub fn test_write_read_write<T>(input: &T)
where
    T: for<'a> binrw::BinRead<Args<'a> = ()> + ssbh_write::SsbhWrite + serde::Serialize,
{
    // The input represents user assigned data and is randomly generated.
    // Writing to an in memory file converts the data to its binary representation.
    let mut writer = Cursor::new(Vec::new());
    input.write(&mut writer).unwrap();
    let before = writer.into_inner();

    // Check that the data can be read.
    // Failures indicate unreadable data was written.
    let mut reader = Cursor::new(before.clone());
    let output = reader.read_le::<T>().unwrap();

    // Converting to binary again should give the same output.
    // Failures indicate that the exporter isn't correct.
    let mut writer = Cursor::new(Vec::new());
    output.write(&mut writer).unwrap();
    let after = writer.into_inner();

    assert_eq!(before, after, "{}", serde_json::to_string(&input).unwrap());
}
