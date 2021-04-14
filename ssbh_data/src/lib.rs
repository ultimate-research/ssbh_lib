pub mod mesh_data;
pub mod skel_data;

use std::{error::Error, io::Read};

use binread::io::{Seek, SeekFrom};
use binread::BinRead;
use binread::BinReaderExt;

fn read_data<R: Read + Seek, T: Into<f32> + BinRead, const N: usize>(
    reader: &mut R,
    count: usize,
    offset: u64,
    stride: u64,
) -> Result<Vec<[f32; N]>, Box<dyn Error>> {
    let mut result = Vec::new();
    for i in 0..count as u64 {
        // The data type may be smaller than stride to allow interleaving different attributes.
        reader.seek(SeekFrom::Start(offset + i * stride))?;

        // TODO: const generics?
        let mut element = [0f32; N];
        for j in 0..N {
            element[j] = reader.read_le::<T>()?.into();
        }
        result.push(element);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        // Remove any whitespace used to make the tests more readable.
        let no_whitespace: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(no_whitespace).unwrap()
    }

    #[test]
    fn read_data_count0() {
        let mut reader = Cursor::new(hex_bytes("01020304"));
        let values = read_data::<_, u8, 4>(&mut reader, 0, 0, 0).unwrap();
        assert_eq!(Vec::<[f32; 4]>::new(), values);
    }

    #[test]
    fn read_data_count1() {
        let mut reader = Cursor::new(hex_bytes("00010203"));
        let values = read_data::<_, u8, 4>(&mut reader, 1, 0, 0).unwrap();
        assert_eq!(vec![[0.0f32, 1.0f32, 2.0f32, 3.0f32]], values);
    }

    #[test]
    fn read_data_stride_equals_size() {
        let mut reader = Cursor::new(hex_bytes("00010203 04050607"));
        let values = read_data::<_, u8, 2>(&mut reader, 3, 0, 2).unwrap();
        assert_eq!(
            vec![[0.0f32, 1.0f32], [2.0f32, 3.0f32], [4.0f32, 5.0f32]],
            values
        );
    }

    #[test]
    fn read_data_stride_equals_size_offset() {
        let mut reader = Cursor::new(hex_bytes("00010203 04050607"));
        let values = read_data::<_, u8, 2>(&mut reader, 3, 2, 2).unwrap();
        assert_eq!(
            vec![[2.0f32, 3.0f32], [4.0f32, 5.0f32], [6.0f32, 7.0f32],],
            values
        );
    }

    #[test]
    fn read_data_stride_exceeds_size() {
        let mut reader = Cursor::new(hex_bytes("00010203 04050607"));
        let values = read_data::<_, u8, 2>(&mut reader, 2, 0, 4).unwrap();
        assert_eq!(vec![[0.0f32, 1.0f32], [4.0f32, 5.0f32]], values);
    }

    #[test]
    fn read_data_stride_exceeds_size_offset() {
        // offset + (stride * count) points past the buffer,
        // but we only read 2 bytes from the last block of size stride = 4
        let mut reader = Cursor::new(hex_bytes("00010203 04050607"));
        let values = read_data::<_, u8, 2>(&mut reader, 2, 2, 4).unwrap();
        assert_eq!(vec![[2.0f32, 3.0f32], [6.0f32, 7.0f32]], values);
    }
}
