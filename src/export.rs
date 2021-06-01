use binread::NullString;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::{
    anim::*,
    formats::{mesh::*, nrpd::RenderPassDataType},
    matl::*,
    shdr::*,
    skel::*,
    InlineString, RelPtr64, SsbhArray, SsbhByteBuffer, SsbhFile, SsbhString, SsbhString8,
    SsbhWrite,
};

fn round_up(value: u64, n: u64) -> u64 {
    // Find the next largest multiple of n.
    ((value + n - 1) / n) * n
}

fn write_relative_offset<W: Write + Seek>(writer: &mut W, data_ptr: &u64) -> std::io::Result<()> {
    let current_pos = writer.stream_position()?;
    writer.write_u64::<LittleEndian>(*data_ptr - current_pos)?;
    Ok(())
}

macro_rules! ssbh_write_c_enum_impl {
    ($enum_type:ident,$underlying_type:ident) => {
        impl SsbhWrite for $enum_type {
            fn write_ssbh<W: std::io::Write + std::io::Seek>(
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
ssbh_write_c_enum_impl!(AttributeDataType, u32);
ssbh_write_c_enum_impl!(AttributeUsageV8, u32);
ssbh_write_c_enum_impl!(AttributeUsageV10, u32);
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

ssbh_write_c_enum_impl!(RenderPassDataType, u64);

ssbh_write_c_enum_impl!(BillboardType, u8);

macro_rules! ssbh_write_impl {
    ($($id:ident),*) => {
        $(
            impl SsbhWrite for $id {
                fn write_ssbh<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    _data_ptr: &mut u64,
                ) -> std::io::Result<()> {
                    writer.write_all(&self.to_le_bytes())?;
                    Ok(())
                }

                fn size_in_bytes(&self) -> u64 {
                    std::mem::size_of::<Self>() as u64
                }
            }
        )*
    }
}

ssbh_write_impl!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

impl<T: binread::BinRead + SsbhWrite> SsbhWrite for Option<T> {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        match self {
            Some(value) => value.write_ssbh(writer, data_ptr),
            None => Ok(()),
        }
    }

    fn size_in_bytes(&self) -> u64 {
        // None values are skipped entirely.
        // TODO: Is this a reasonable implementation?
        match self {
            Some(value) => value.size_in_bytes(),
            None => 0u64,
        }
    }

    fn alignment_in_bytes(&self) -> u64 {
        // Use the underlying type's alignment.
        // This is a bit of a hack since None values won't be written anyway.
        match self {
            Some(value) => value.alignment_in_bytes(),
            None => 8,
        }
    }
}

fn write_array_header<W: Write + Seek>(
    writer: &mut W,
    data_ptr: &mut u64,
    count: usize,
) -> std::io::Result<()> {
    // Arrays are always 8 byte aligned.
    *data_ptr = round_up(*data_ptr, 8);

    // Don't write the offset for empty arrays.
    if count == 0 {
        writer.write_u64::<LittleEndian>(0u64)?;
    } else {
        write_relative_offset(writer, &data_ptr)?;
    }

    writer.write_u64::<LittleEndian>(count as u64)?;
    Ok(())
}

impl SsbhWrite for SsbhByteBuffer {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos {
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

impl<T: SsbhWrite + binread::BinRead> SsbhWrite for &[T] {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        for element in self.iter() {
            element.write_ssbh(writer, data_ptr)?;
        }

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // TODO: This won't work for Vec<Option<T>> since only the first element is checked.
        match self.first() {
            Some(element) => self.len() as u64 * element.size_in_bytes(),
            None => 0,
        }
    }
}

impl<T: binread::BinRead + SsbhWrite + Sized> SsbhWrite for SsbhArray<T> {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos {
            *data_ptr += self.size_in_bytes();
        }

        write_array_header(writer, data_ptr, self.elements.len())?;

        let pos_after_length = writer.stream_position()?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        self.elements.as_slice().write_ssbh(writer, data_ptr)?;

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

impl SsbhWrite for NullString {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        _data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        if self.len() == 0 {
            // Handle empty strings.
            writer.write_all(&[0u8; 4])?;
        } else {
            // Write the data and null terminator.
            writer.write_all(&self)?;
            writer.write_all(&[0u8])?;
        }
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // Include the null byte in the length.
        self.len() as u64 + 1
    }

    fn alignment_in_bytes(&self) -> u64 {
        4
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
            writer.write_u64::<LittleEndian>(0u64)?;
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
    write_rel_ptr_aligned_specialized(writer, data, data_ptr, alignment, T::write_ssbh)?;
    Ok(())
}

fn write_ssbh_header<W: Write + Seek>(writer: &mut W, magic: &[u8; 4]) -> std::io::Result<()> {
    // Hardcode the header because this is shared for all SSBH formats.
    writer.write_all(b"HBSS")?;
    writer.write_u64::<LittleEndian>(64)?;
    writer.write_u32::<LittleEndian>(0)?;
    writer.write_all(magic)?;
    Ok(())
}

// TODO: This could be derived.
impl SsbhWrite for SsbhString {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr < current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        // TODO: This is shared with ssbh string8 but has 4 byte alignment and 4 byte empty strings.
        match &self.0 .0 {
            Some(value) => {
                // Calculate the relative offset.
                *data_ptr = round_up(*data_ptr, 4);
                write_relative_offset(writer, data_ptr)?;

                // Write the data at the specified offset.
                let pos_after_offset = writer.stream_position()?;
                writer.seek(SeekFrom::Start(*data_ptr))?;

                // TODO: Find a nicer way to handle this.
                if value.0.is_empty() {
                    // 4 byte empty strings.
                    writer.write_all(&[0u8; 4])?;
                } else {
                    value.write_ssbh(writer, data_ptr)?;
                }

                // Point the data pointer past the current write.
                // Types with relative offsets will already increment the data pointer.
                let current_pos = writer.stream_position()?;
                if current_pos > *data_ptr {
                    *data_ptr = round_up(current_pos, 4);
                }

                writer.seek(SeekFrom::Start(pos_after_offset))?;
                Ok(())
            }
            None => {
                // Null offsets don't increment the data pointer.
                writer.write_u64::<LittleEndian>(0u64)?;
                Ok(())
            }
        }
    }

    fn size_in_bytes(&self) -> u64 {
        self.0.size_in_bytes()
    }
}

// TODO: This could be derived.
impl SsbhWrite for InlineString {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        self.0.write_ssbh(writer, data_ptr)
    }

    fn size_in_bytes(&self) -> u64 {
        self.0.size_in_bytes()
    }
}

// TODO: This could just be derived as RelPtr64<NullString> but requires different alignment.
impl SsbhWrite for SsbhString8 {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr < current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        // TODO: This is shared with ssbh string but has 8 byte alignment and 8 byte empty strings.
        match &self.0 .0 .0 {
            Some(value) => {
                // Calculate the relative offset.
                *data_ptr = round_up(*data_ptr, 8);
                write_relative_offset(writer, data_ptr)?;

                // Write the data at the specified offset.
                let pos_after_offset = writer.stream_position()?;
                writer.seek(SeekFrom::Start(*data_ptr))?;

                // TODO: Find a nicer way to handle this.
                if value.0.is_empty() {
                    //8 byte empty strings.
                    writer.write_all(&[0u8; 8])?;
                } else {
                    value.write_ssbh(writer, data_ptr)?;
                }

                // Point the data pointer past the current write.
                // Types with relative offsets will already increment the data pointer.
                let current_pos = writer.stream_position()?;
                if current_pos > *data_ptr {
                    *data_ptr = round_up(current_pos, 8);
                }

                writer.seek(SeekFrom::Start(pos_after_offset))?;
                Ok(())
            }
            None => {
                // Null offsets don't increment the data pointer.
                writer.write_u64::<LittleEndian>(0u64)?;
                Ok(())
            }
        }
    }

    fn size_in_bytes(&self) -> u64 {
        self.0.size_in_bytes()
    }
}

impl<T: SsbhWrite + binread::BinRead> SsbhWrite for RelPtr64<T> {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        write_rel_ptr_aligned(writer, &self.0, data_ptr, self.0.alignment_in_bytes())?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        8
    }
}

impl<T: SsbhWrite> SsbhWrite for Vec<T> {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        for elem in self.iter() {
            elem.write_ssbh(writer, data_ptr)?;
        }
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        if self.is_empty() {
            0
        } else {
            match self.first() {
                Some(first) => self.len() as u64 * first.size_in_bytes(),
                None => 0,
            }
        }
    }
}

pub fn write_anim<W: Write + Seek>(writer: &mut W, data: &Anim) -> std::io::Result<()> {
    write_ssbh_header(writer, b"MINA")?;

    let mut data_ptr = writer.stream_position()?;

    // Point past the struct.
    data_ptr += data.size_in_bytes(); // size of fields

    data.write_ssbh(writer, &mut data_ptr)?;

    // Padding was added for version 2.1 compared to 2.0.
    if data.major_version == 2 && data.minor_version == 1 {
        // The newer file revision is aligned to a multiple of 8.
        let total_size = writer.seek(SeekFrom::End(0))?;
        let new_size = round_up(total_size, 8);
        for _ in 0..(new_size - total_size) {
            writer.write_all(&[0u8])?;
        }
    }

    Ok(())
}

pub fn write_ssbh<W: Write + Seek>(writer: &mut W, data: &SsbhFile) -> std::io::Result<()> {
    match &data {
        SsbhFile::Modl(modl) => write_ssbh_file(writer, modl, b"LDOM"),
        SsbhFile::Skel(skel) => write_ssbh_file(writer, skel, b"LEKS"),
        SsbhFile::Nufx(nufx) => write_ssbh_file(writer, nufx, b"XFUN"),
        SsbhFile::Shdr(shdr) => write_ssbh_file(writer, shdr, b"RDHS"),
        SsbhFile::Matl(matl) => write_ssbh_file(writer, matl, b"LTAM"),
        SsbhFile::Anim(anim) => write_anim(writer, &anim),
        SsbhFile::Hlpb(hlpb) => write_ssbh_file(writer, hlpb, b"BPLH"),
        SsbhFile::Mesh(mesh) => write_ssbh_file(writer, mesh, b"HSEM"),
        SsbhFile::Nrpd(nrpd) => write_ssbh_file(writer, nrpd, b"DPRN"),
    }
}

pub fn write_buffered<W: Write + Seek, F: Fn(&mut Cursor<Vec<u8>>) -> std::io::Result<()>>(
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
pub fn write_ssbh_file<W: Write + Seek, S: SsbhWrite>(
    writer: &mut W,
    data: &S,
    magic: &[u8; 4],
) -> std::io::Result<()> {
    write_ssbh_header(writer, magic)?;
    let mut data_ptr = writer.stream_position()?;

    // Point past the struct.
    data_ptr += data.size_in_bytes(); // size of fields

    data.write_ssbh(writer, &mut data_ptr)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // The tests are designed to check the SSBH offset rules.
    // 1. Offsets point past the containing struct.
    // 2. Offsets in array elements point past the containing array.
    // 3. Offsets obey the alignment rules of the data's type.

    use super::*;
    use crate::{SsbhEnum64, SsbhString};
    use binread::BinRead;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        // Remove any whitespace used to make the tests more readable.
        let no_whitespace: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(no_whitespace).unwrap()
    }

    #[test]
    fn write_null_rel_ptr() {
        let value = RelPtr64::<u32>(None);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("00000000 00000000"));
        assert_eq!(8, data_ptr);
    }

    #[test]
    fn write_nested_rel_ptr_depth2() {
        let value = RelPtr64::new(RelPtr64::new(7u32));

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes(
                "08000000 00000000 
                 08000000 00000000 
                 07000000"
            )
        );
    }

    #[test]
    fn write_ssbh_string() {
        let value = SsbhString::from("scouter1Shape");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 73636F75 74657231 53686170 6500")
        );
        // The data pointer should be aligned to 4.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn write_ssbh_string_empty() {
        let value = SsbhString::from("");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(*writer.get_ref(), hex_bytes("08000000 00000000 00000000"));
        // The data pointer should be aligned to 4.
        assert_eq!(12, data_ptr);
    }

    #[test]
    fn write_ssbh_string_non_zero_data_ptr() {
        let value = SsbhString::from("scouter1Shape");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 5;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 73636F75 74657231 53686170 6500")
        );
        // The data pointer should be aligned to 4.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn write_ssbh_array_ssbh_string() {
        let value = SsbhArray::new(vec![
            SsbhString::from("leyes_eye_mario_l_col"),
            SsbhString::from("eye_mario_w_nor"),
        ]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

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
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

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
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("10000000 00000000 05000000 00000000 01020304 05")
        );
        assert_eq!(21, data_ptr);
    }

    #[test]
    fn write_empty_byte_buffer() {
        let value = SsbhByteBuffer::new(Vec::new());

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        // Null and empty arrays seem to use 0 offset and 0 length.
        assert_eq!(
            *writer.get_ref(),
            hex_bytes("00000000 00000000 00000000 00000000")
        );
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn write_ssbh_string_tuple() {
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
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

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
    fn write_ssbh_string8() {
        let value = SsbhString8::from("BlendState0");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 426C656E 64537461 74653000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn write_ssbh_string8_empty() {
        let value = SsbhString8::from("");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 00000000 00000000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn write_ssbh_string8_non_zero_data_ptr() {
        let value = SsbhString8::from("BlendState0");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 5;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("08000000 00000000 426C656E 64537461 74653000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(24, data_ptr);
    }

    #[derive(BinRead, PartialEq, Debug)]
    #[br(import(data_type: u64))]
    pub enum TestData {
        #[br(pre_assert(data_type == 01u64))]
        Float(f32),
        #[br(pre_assert(data_type == 02u64))]
        Unsigned(u32),
    }

    impl SsbhWrite for TestData {
        fn write_ssbh<W: Write + Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            match self {
                TestData::Float(f) => f.write_ssbh(writer, data_ptr),
                TestData::Unsigned(u) => u.write_ssbh(writer, data_ptr),
            }
        }

        fn size_in_bytes(&self) -> u64 {
            todo!()
        }
    }

    #[test]
    fn write_ssbh_enum_float() {
        let value = SsbhEnum64::<TestData> {
            data: RelPtr64::new(TestData::Float(1.0f32)),
            data_type: 1u64,
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("10000000 00000000 01000000 00000000 0000803F")
        );
    }

    #[test]
    fn write_ssbh_enum_unsigned() {
        let value = SsbhEnum64::<TestData> {
            data: RelPtr64::new(TestData::Unsigned(5u32)),
            data_type: 2u64,
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.write_ssbh(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            *writer.get_ref(),
            hex_bytes("10000000 00000000 02000000 00000000 05000000")
        );
    }
}
