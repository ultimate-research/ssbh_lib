use binread::BinRead;
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::{
    anim::*, formats::mesh::*, matl::*, shdr::*, skel::*, Offset, Ptr, RelPtr64, SsbhArray,
    SsbhByteBuffer, SsbhFile, SsbhWrite,
};

fn round_up(value: u64, n: u64) -> u64 {
    // Find the next largest multiple of n.
    ((value + n - 1) / n) * n
}

fn write_u64<W: Write + Seek>(writer: &mut W, value: u64) -> std::io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_u32<W: Write + Seek>(writer: &mut W, value: u32) -> std::io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_relative_offset<W: Write + Seek>(writer: &mut W, data_ptr: &u64) -> std::io::Result<()> {
    let current_pos = writer.stream_position()?;
    write_u64(writer, *data_ptr - current_pos)?;
    Ok(())
}

macro_rules! ssbh_write_c_enum_impl {
    ($enum_type:ident,$underlying_type:ident) => {
        impl SsbhWrite for $enum_type {
            fn ssbh_write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                _data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                let value = *self as $underlying_type;
                let bytes = value.to_le_bytes();
                writer.write_all(&bytes)?;
                Ok(())
            }

            fn size_in_bytes(&self) -> u64 {
                std::mem::size_of::<$underlying_type>() as u64
            }

            fn alignment_in_bytes(&self) -> u64 {
                std::mem::align_of::<$underlying_type>() as u64
            }
        }
    };
}

// TODO: These can be derived at some point.
// TODO: It may be better to move these next to their definition.
ssbh_write_c_enum_impl!(ShaderType, u32);

ssbh_write_c_enum_impl!(CompressionType, u8);
ssbh_write_c_enum_impl!(TrackType, u8);
ssbh_write_c_enum_impl!(AnimType, u64);

ssbh_write_c_enum_impl!(AttributeDataTypeV8, u32);
ssbh_write_c_enum_impl!(AttributeDataTypeV10, u32);
ssbh_write_c_enum_impl!(AttributeUsageV8, u32);
ssbh_write_c_enum_impl!(AttributeUsageV9, u32);
ssbh_write_c_enum_impl!(RiggingType, u32);
ssbh_write_c_enum_impl!(DrawElementType, u32);

ssbh_write_c_enum_impl!(BlendFactor, u32);
ssbh_write_c_enum_impl!(WrapMode, u32);
ssbh_write_c_enum_impl!(CullMode, u32);
ssbh_write_c_enum_impl!(FillMode, u32);
ssbh_write_c_enum_impl!(MinFilter, u32);
ssbh_write_c_enum_impl!(MagFilter, u32);
ssbh_write_c_enum_impl!(FilteringType, u32);
ssbh_write_c_enum_impl!(ParamId, u64);

ssbh_write_c_enum_impl!(BillboardType, u8);

fn write_array_header<W: Write + Seek>(
    writer: &mut W,
    data_ptr: &mut u64,
    count: usize,
) -> std::io::Result<()> {
    // Arrays are always 8 byte aligned.
    *data_ptr = round_up(*data_ptr, 8);

    // Don't write the offset for empty arrays.
    if count == 0 {
        write_u64(writer, 0u64)?;
    } else {
        write_relative_offset(writer, &data_ptr)?;
    }

    write_u64(writer, count as u64)?;
    Ok(())
}

impl SsbhWrite for SsbhByteBuffer {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos + self.size_in_bytes() {
            *data_ptr += self.size_in_bytes();
        }

        write_array_header(writer, data_ptr, self.elements.len())?;

        let current_pos = writer.stream_position()?;
        writer.seek(SeekFrom::Start(*data_ptr))?;
        // Use a custom implementation to avoid writing bytes individually.
        // Pointers in array elements should point past the end of the array.
        writer.write_all(&self.elements)?;
        *data_ptr += self.elements.len() as u64;

        writer.seek(SeekFrom::Start(current_pos))?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        16
    }
}

impl<T: binread::BinRead + SsbhWrite + Sized> SsbhWrite for SsbhArray<T> {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos + self.size_in_bytes() {
            *data_ptr += self.size_in_bytes();
        }

        write_array_header(writer, data_ptr, self.elements.len())?;

        let pos_after_length = writer.stream_position()?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        self.elements.as_slice().ssbh_write(writer, data_ptr)?;

        writer.seek(SeekFrom::Start(pos_after_length))?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // A 64 bit relative offset and 64 bit length
        16
    }

    fn alignment_in_bytes(&self) -> u64 {
        // Arrays are always 8 byte aligned.
        8
    }
}

fn write_rel_ptr_aligned_specialized<
    W: Write + Seek,
    T,
    F: Fn(&T, &mut W, &mut u64) -> std::io::Result<()>,
>(
    writer: &mut W,
    data: &Option<T>,
    data_ptr: &mut u64,
    alignment: u64,
    write_t: F,
) -> std::io::Result<()> {
    match data {
        Some(value) => {
            // Calculate the relative offset.
            *data_ptr = round_up(*data_ptr, alignment);
            write_relative_offset(writer, data_ptr)?;

            // Write the data at the specified offset.
            let pos_after_offset = writer.stream_position()?;
            writer.seek(SeekFrom::Start(*data_ptr))?;

            // Allow custom write functions for performance reasons.
            write_t(&value, writer, data_ptr)?;

            // Point the data pointer past the current write.
            // Types with relative offsets will already increment the data pointer.
            let current_pos = writer.stream_position()?;
            if current_pos > *data_ptr {
                *data_ptr = round_up(current_pos, alignment);
            }

            writer.seek(SeekFrom::Start(pos_after_offset))?;
            Ok(())
        }
        None => {
            // Null offsets don't increment the data pointer.
            write_u64(writer, 0u64)?;
            Ok(())
        }
    }
}

fn write_rel_ptr_aligned<W: Write + Seek, T: SsbhWrite>(
    writer: &mut W,
    data: &Option<T>,
    data_ptr: &mut u64,
    alignment: u64,
) -> std::io::Result<()> {
    write_rel_ptr_aligned_specialized(writer, data, data_ptr, alignment, T::ssbh_write)?;
    Ok(())
}

fn write_ssbh_header<W: Write + Seek>(writer: &mut W, magic: &[u8; 4]) -> std::io::Result<()> {
    // Hardcode the header because this is shared for all SSBH formats.
    writer.write_all(b"HBSS")?;
    write_u64(writer, 64)?;
    write_u32(writer, 0)?;
    writer.write_all(magic)?;
    Ok(())
}

impl<P: Offset, T: SsbhWrite + BinRead<Args = ()>> SsbhWrite for Ptr<P, T> {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // TODO: This is nearly identical to the relative pointer function.
        let alignment = self.0.alignment_in_bytes();

        // The data pointer must point past the containing type.
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        // Calculate the absolute offset.
        *data_ptr = round_up(*data_ptr, alignment);

        let offset = P::try_from(*data_ptr)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "oh no!"))?;
        P::ssbh_write(&offset, writer, data_ptr)?;

        // Write the data at the specified offset.
        let pos_after_offset = writer.stream_position()?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        self.0.ssbh_write(writer, data_ptr)?;

        // Point the data pointer past the current write.
        // Types with relative offsets will already increment the data pointer.
        let current_pos = writer.stream_position()?;
        if current_pos > *data_ptr {
            *data_ptr = round_up(current_pos, alignment);
        }

        writer.seek(SeekFrom::Start(pos_after_offset))?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // TODO: Use the size_in_bytes already defined for P?
        std::mem::size_of::<P>() as u64
    }
}

impl<T: SsbhWrite + binread::BinRead> SsbhWrite for RelPtr64<T> {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        write_rel_ptr_aligned(writer, &self.0, data_ptr, self.0.alignment_in_bytes())?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        8
    }
}

pub(crate) fn write_ssbh_header_and_data<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhFile,
) -> std::io::Result<()> {
    match &data {
        SsbhFile::Modl(modl) => write_ssbh_file(writer, modl, b"LDOM"),
        SsbhFile::Skel(skel) => write_ssbh_file(writer, skel, b"LEKS"),
        SsbhFile::Nufx(nufx) => write_ssbh_file(writer, nufx, b"XFUN"),
        SsbhFile::Shdr(shdr) => write_ssbh_file(writer, shdr, b"RDHS"),
        SsbhFile::Matl(matl) => write_ssbh_file(writer, matl, b"LTAM"),
        SsbhFile::Anim(anim) => write_ssbh_file(writer, anim, b"MINA"),
        SsbhFile::Hlpb(hlpb) => write_ssbh_file(writer, hlpb, b"BPLH"),
        SsbhFile::Mesh(mesh) => write_ssbh_file(writer, mesh, b"HSEM"),
        SsbhFile::Nrpd(nrpd) => write_ssbh_file(writer, nrpd, b"DPRN"),
    }
}

pub(crate) fn write_buffered<
    W: Write + Seek,
    F: Fn(&mut Cursor<Vec<u8>>) -> std::io::Result<()>,
>(
    writer: &mut W,
    write_data: F,
) -> std::io::Result<()> {
    // Buffer the entire write operation into memory to improve performance.
    // The seeks used to write relative offsets cause flushes for BufWriter.
    let mut cursor = Cursor::new(Vec::new());
    write_data(&mut cursor)?;

    writer.write_all(cursor.get_mut())?;
    Ok(())
}

// TODO: This can probably just be derived.
pub(crate) fn write_ssbh_file<W: Write + Seek, S: SsbhWrite>(
    writer: &mut W,
    data: &S,
    magic: &[u8; 4],
) -> std::io::Result<()> {
    write_ssbh_header(writer, magic)?;
    let mut data_ptr = writer.stream_position()?;

    // Point past the struct.
    data_ptr += data.size_in_bytes(); // size of fields

    data.ssbh_write(writer, &mut data_ptr)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // The tests are designed to check the SSBH offset rules.
    // It's unclear if these rules are strictly required by the format or in game parsers,
    // but following these rules creates 1:1 export for all formats except NRPD.

    use super::*;
    use crate::{Ptr16, Ptr32, Ptr64, SsbhEnum64, SsbhString, SsbhString8};
    use binread::BinRead;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        // Remove any whitespace used to make the tests more readable.
        let no_whitespace: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(no_whitespace).unwrap()
    }

    #[test]
    fn write_ptr16() {
        let value = Ptr16::<u8>::new(5u8);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("0200 05"));
        assert_eq!(3, data_ptr);
    }

    #[test]
    fn write_ptr32() {
        let value = Ptr32::<u8>::new(5u8);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("04000000 05"));
        assert_eq!(5, data_ptr);
    }

    #[test]
    fn write_ptr64() {
        let value = Ptr64::<u8>::new(5u8);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("08000000 00000000 05"));
        assert_eq!(9, data_ptr);
    }

    #[test]
    fn write_ptr64_vec_u8() {
        // Check that the alignment uses the inner type's alignment.
        let value = Ptr64::new(vec![5u8]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("08000000 00000000 05"));
        assert_eq!(9, data_ptr);
    }

    #[test]
    fn write_ptr64_vec_u32() {
        // Check that the alignment uses the inner type's alignment.
        let value = Ptr64::new(vec![5u32]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("08000000 00000000 05000000"));
        assert_eq!(12, data_ptr);
    }

    #[test]
    fn write_null_rel_ptr() {
        let value = RelPtr64::<u32>(None);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("00000000 00000000"));
        assert_eq!(8, data_ptr);
    }

    #[test]
    fn write_nested_rel_ptr_depth2() {
        let value = RelPtr64::new(RelPtr64::new(7u32));

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "08000000 00000000 
                 08000000 00000000 
                 07000000"
            )
        );
        assert_eq!(20, data_ptr);
    }

    #[test]
    fn ssbh_write_string() {
        let value = SsbhString::from("scouter1Shape");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 73636F75 74657231 53686170 6500")
        );
        // The data pointer should be aligned to 4.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn ssbh_write_string_empty() {
        let value = SsbhString::from("");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("08000000 00000000 00000000"));
        // The data pointer should be aligned to 4.
        assert_eq!(12, data_ptr);
    }

    #[test]
    fn ssbh_write_string_non_zero_data_ptr() {
        let value = SsbhString::from("scouter1Shape");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 5;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 73636F75 74657231 53686170 6500")
        );
        // The data pointer should be aligned to 4.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn ssbh_write_array_ssbh_string() {
        let value = SsbhArray::new(vec![
            SsbhString::from("leyes_eye_mario_l_col"),
            SsbhString::from("eye_mario_w_nor"),
        ]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Check that the relative offsets point past the array.
        // Check that string data is aligned to 4.
        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "10000000 00000000 02000000 00000000
                 10000000 00000000 20000000 00000000
                 6C657965 735F6579 655F6D61 72696F5F 
                 6C5F636F 6C000000 6579655F 6D617269 
                 6F5F775F 6E6F7200"
            )
        );
    }

    #[test]
    fn write_empty_array() {
        let value = SsbhArray::<u32>::new(Vec::new());

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Null and empty arrays seem to use 0 offset and 0 length.
        assert_eq!(
            *writer.get_ref(),
            hex_bytes("00000000 00000000 00000000 00000000")
        );
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn write_byte_buffer() {
        let value = SsbhByteBuffer::new(vec![1u8, 2u8, 3u8, 4u8, 5u8]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("10000000 00000000 05000000 00000000 01020304 05")
        );
        assert_eq!(21, data_ptr);
    }

    #[test]
    fn write_vec() {
        let value = vec![1u8, 2u8, 3u8, 4u8, 5u8];

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("01020304 05"));
        assert_eq!(5, data_ptr);
    }

    #[test]
    fn write_empty_byte_buffer() {
        let value = SsbhByteBuffer::new(Vec::new());

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Null and empty arrays seem to use 0 offset and 0 length.
        assert_eq!(
            *writer.get_ref(),
            hex_bytes("00000000 00000000 00000000 00000000")
        );
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn ssbh_write_string_tuple() {
        #[derive(SsbhWrite)]
        struct StringPair {
            item1: SsbhString,
            item2: SsbhString,
        }

        // NRPD data.
        let value = StringPair {
            item1: SsbhString::from("RTV_FRAME_BUFFER_COPY"),
            item2: SsbhString::from("FB_FRAME_BUFFER_COPY"),
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Check that the pointers don't overlap.
        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "10000000 00000000 20000000 00000000 
                 5254565F 4652414D 455F4255 46464552 
                 5F434F50 59000000 46425F46 52414D45 
                 5F425546 4645525F 434F5059 00"
            )
        );
    }

    #[test]
    fn ssbh_write_string8() {
        let value = SsbhString8::from("BlendState0");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 426C656E 64537461 74653000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn ssbh_write_string8_empty() {
        let value = SsbhString8::from("");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 00000000 00000000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn ssbh_write_string8_non_zero_data_ptr() {
        let value = SsbhString8::from("BlendState0");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 5;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 426C656E 64537461 74653000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(24, data_ptr);
    }

    #[derive(BinRead, PartialEq, Debug, SsbhWrite)]
    #[br(import(data_type: u64))]
    pub enum TestData {
        #[br(pre_assert(data_type == 1u64))]
        Float(f32),
        #[br(pre_assert(data_type == 2u64))]
        Unsigned(u32),
    }

    #[test]
    fn ssbh_write_enum_float() {
        let value = SsbhEnum64::<TestData> {
            data: RelPtr64::new(TestData::Float(1.0f32)),
            data_type: 1u64,
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("10000000 00000000 01000000 00000000 0000803F")
        );
    }

    #[test]
    fn ssbh_write_enum_unsigned() {
        let value = SsbhEnum64::<TestData> {
            data: RelPtr64::new(TestData::Unsigned(5u32)),
            data_type: 2u64,
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("10000000 00000000 02000000 00000000 05000000")
        );
    }
}
