use std::io::{Read, Write};
use std::ops::Mul;

use binread::io::{Seek, SeekFrom};
use binread::BinReaderExt;
use binread::{BinRead, BinResult};
use half::f16;

pub fn read_data<R: Read + Seek, TIn: BinRead, TOut: From<TIn>>(
    reader: &mut R,
    count: usize,
    offset: u64,
) -> BinResult<Vec<TOut>> {
    let mut result = Vec::new();
    reader.seek(SeekFrom::Start(offset))?;
    for _ in 0..count as u64 {
        result.push(reader.read_le::<TIn>()?.into());
    }
    Ok(result)
}

pub fn read_vector_data<R: Read + Seek, T: Into<f32> + BinRead, const N: usize>(
    reader: &mut R,
    count: usize,
    offset: u64,
    stride: u64,
) -> BinResult<Vec<[f32; N]>> {
    let mut result = Vec::new();
    for i in 0..count as u64 {
        // The data type may be smaller than stride to allow interleaving different attributes.
        reader.seek(SeekFrom::Start(offset + i * stride))?;

        let mut element = [0f32; N];
        for e in element.iter_mut() {
            *e = reader.read_le::<T>()?.into();
        }
        result.push(element);
    }
    Ok(result)
}

pub fn get_u8_clamped(f: f32) -> u8 {
    f.clamp(0.0f32, 1.0f32).mul(255.0f32).round() as u8
}

pub fn write_f32<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

pub fn write_u8<W: Write>(writer: &mut W, data: &[u8]) -> std::io::Result<()> {
    writer.write_all(data)
}

pub fn write_f16<W: Write>(writer: &mut W, data: &[f16]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

pub fn write_vector_data<
    T,
    W: Write + Seek,
    F: Fn(&mut W, &[T]) -> std::io::Result<()>,
    const N: usize,
>(
    writer: &mut W,
    elements: &[[T; N]],
    offset: u64,
    stride: u64,
    write_t: F,
) -> Result<(), std::io::Error> {
    for (i, element) in elements.iter().enumerate() {
        writer.seek(SeekFrom::Start(offset + i as u64 * stride))?;
        write_t(writer, element)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;
    use std::io::Cursor;

    #[test]
    fn read_data_count0() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, u16>(&mut reader, 0, 0).unwrap();
        assert_eq!(Vec::<u16>::new(), values);
    }

    #[test]
    fn read_data_count4() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, u32>(&mut reader, 4, 0).unwrap();
        assert_eq!(vec![1u32, 2u32, 3u32, 4u32], values);
    }

    #[test]
    fn read_data_offset() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, f32>(&mut reader, 2, 1).unwrap();
        assert_eq!(vec![2f32, 3f32], values);
    }

    #[test]
    fn read_vector_data_count0() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_vector_data::<_, u8, 4>(&mut reader, 0, 0, 0).unwrap();
        assert_eq!(Vec::<[f32; 4]>::new(), values);
    }

    #[test]
    fn read_vector_data_count1() {
        let mut reader = Cursor::new(hex!("00010203"));
        let values = read_vector_data::<_, u8, 4>(&mut reader, 1, 0, 0).unwrap();
        assert_eq!(vec![[0.0f32, 1.0f32, 2.0f32, 3.0f32]], values);
    }

    #[test]
    fn read_vector_data_stride_equals_size() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 3, 0, 2).unwrap();
        assert_eq!(
            vec![[0.0f32, 1.0f32], [2.0f32, 3.0f32], [4.0f32, 5.0f32]],
            values
        );
    }

    #[test]
    fn read_vector_data_stride_equals_size_offset() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 3, 2, 2).unwrap();
        assert_eq!(
            vec![[2.0f32, 3.0f32], [4.0f32, 5.0f32], [6.0f32, 7.0f32],],
            values
        );
    }

    #[test]
    fn read_vector_data_stride_exceeds_size() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 2, 0, 4).unwrap();
        assert_eq!(vec![[0.0f32, 1.0f32], [4.0f32, 5.0f32]], values);
    }

    #[test]
    fn read_vector_data_stride_exceeds_size_offset() {
        // offset + (stride * count) points past the buffer,
        // but we only read 2 bytes from the last block of size stride = 4
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 2, 2, 4).unwrap();
        assert_eq!(vec![[2.0f32, 3.0f32], [6.0f32, 7.0f32]], values);
    }

    #[test]
    fn write_vector_data_count0() {
        let mut writer = Cursor::new(Vec::new());
        write_vector_data::<f32, _, _, 1>(&mut writer, &[], 0, 4, write_f32).unwrap();
        assert!(writer.get_ref().is_empty());
    }

    #[test]
    fn write_vector_data_count1() {
        let mut writer = Cursor::new(Vec::new());
        write_vector_data(&mut writer, &[[1f32, 2f32]], 0, 8, write_f32).unwrap();
        assert_eq!(*writer.get_ref(), hex!("0000803F 00000040"),);
    }

    #[test]
    fn write_vector_stride_offset() {
        let mut writer = Cursor::new(Vec::new());
        write_vector_data(
            &mut writer,
            &[[1f32, 2f32, 3f32], [1f32, 0f32, 0f32]],
            4,
            16,
            write_f32,
        )
        .unwrap();

        // The last 4 bytes of padding from stride should be missing.
        // This matches the behavior of read_vector_data.
        assert_eq!(
            *writer.get_ref(),
            hex!(
                "00000000 
                 0000803F 00000040 00004040 00000000 
                 0000803F 00000000 00000000"
            )
        );
    }

    #[test]
    fn u8_clamped() {
        assert_eq!(0u8, get_u8_clamped(-1.0f32));

        for u in 0..=255u8 {
            assert_eq!(u, get_u8_clamped(u as f32 / 255.0f32));
        }

        assert_eq!(255u8, get_u8_clamped(2.0f32));
    }
}
