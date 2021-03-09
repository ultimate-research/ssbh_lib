use binread::NullString;
use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{{Ssbh, SsbhFile, SsbhString}, RelPtr64, SsbhArray, SsbhByteBuffer, SsbhString8, SsbhWrite, anim::*, formats::{mesh::*, nrpd::{FrameBuffer, NrpdState, RenderPassDataType}}, matl::*, shdr::*, skel::*};

fn round_up(value: u64, n: u64) -> u64 {
    // Find the next largest multiple of n.
    ((value + n - 1) / n) * n
}

fn write_relative_offset<W: Write + Seek>(writer: &mut W, data_ptr: &u64) -> std::io::Result<()> {
    let current_pos = writer.seek(SeekFrom::Current(0))?;
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
                writer.write(&bytes)?;
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
ssbh_write_c_enum_impl!(AttributeUsage, u32);
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

macro_rules! ssbh_write_impl {
    ($($id:ident),*) => {
        $(
            impl SsbhWrite for $id {
                fn write_ssbh<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    _data_ptr: &mut u64,
                ) -> std::io::Result<()> {
                    writer.write(&self.to_le_bytes())?;
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
}

impl SsbhWrite for MeshAttributes {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        match self {
            MeshAttributes::AttributesV8(attributes_v8) => {
                attributes_v8.write_ssbh(writer, data_ptr)?
            }
            MeshAttributes::AttributesV10(attributes_v10) => {
                attributes_v10.write_ssbh(writer, data_ptr)?
            }
        };
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        16 // array
    }
}

impl SsbhWrite for Param {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        match self {
            Param::Float(v) => v.write_ssbh(writer, data_ptr),
            Param::Boolean(v) => v.write_ssbh(writer, data_ptr),
            Param::Vector4(v) => v.write_ssbh(writer, data_ptr),
            Param::MatlString(v) => v.write_ssbh(writer, data_ptr),
            Param::Sampler(v) => v.write_ssbh(writer, data_ptr),
            Param::UvTransform(v) => v.write_ssbh(writer, data_ptr),
            Param::BlendState(v) => v.write_ssbh(writer, data_ptr),
            Param::RasterizerState(v) => v.write_ssbh(writer, data_ptr),
        }
    }

    fn size_in_bytes(&self) -> u64 {
        todo!()
    }
}

macro_rules! ssbh_write_bitfield_impl {
    ($($id:ident),*) => {
        $(
            impl SsbhWrite for $id {
                fn write_ssbh<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    _data_ptr: &mut u64,
                ) -> std::io::Result<()> {
                    writer.write(&self.into_bytes())?;
                    Ok(())
                }

                fn size_in_bytes(&self) -> u64 {
                    std::mem::size_of::<Self>() as u64
                }
            }
        )*
    }
}

ssbh_write_bitfield_impl!(SkelEntryFlags, RiggingFlags);

impl SsbhWrite for SsbhByteBuffer {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        *data_ptr = round_up(*data_ptr, 8);

        // Don't write the offset for empty arrays.
        if self.elements.is_empty() {
            writer.write_u64::<LittleEndian>(0u64)?;
        } else {
            write_relative_offset(writer, &data_ptr)?;
        }
        writer.write_u64::<LittleEndian>(self.elements.len() as u64)?;

        let current_pos = writer.seek(SeekFrom::Current(0))?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        // Pointers in array elements should point past the end of the array.
        *data_ptr += self.elements.len() as u64;

        writer.write_all(&self.elements)?;
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
        let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;
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
        // TODO: This logic seems to be shared with all relative offsets?
        let current_pos = writer.seek(SeekFrom::Current(0))?;
        if *data_ptr <= current_pos {
            *data_ptr += self.size_in_bytes();
        }

        *data_ptr = round_up(*data_ptr, self.alignment_in_bytes());

        // Don't write the offset for empty arrays.
        if self.elements.is_empty() {
            writer.write_u64::<LittleEndian>(0u64)?;
        } else {
            write_relative_offset(writer, &data_ptr)?;
        }
        writer.write_u64::<LittleEndian>(self.elements.len() as u64)?;

        let pos_after_length = writer.seek(SeekFrom::Current(0))?;
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
        // TODO: Handle null strings?
        if self.len() == 0 {
            // Handle empty strings.
            writer.write_u32::<LittleEndian>(0u32)?;
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

fn write_ssbh_string_aligned<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhString,
    data_ptr: &mut u64,
    alignment: u64,
) -> std::io::Result<()> {
    write_rel_ptr_aligned(writer, &data.value.0, data_ptr, alignment)?;
    Ok(())
}

fn write_rel_ptr_aligned<W: Write + Seek, T: SsbhWrite>(
    writer: &mut W,
    data: &T,
    data_ptr: &mut u64,
    alignment: u64,
) -> std::io::Result<()> {
    // Calculate the relative offset.
    *data_ptr = round_up(*data_ptr, alignment);
    write_relative_offset(writer, data_ptr)?;

    // Write the data at the specified offset.
    let pos_after_offset = writer.seek(SeekFrom::Current(0))?;
    writer.seek(SeekFrom::Start(*data_ptr))?;

    data.write_ssbh(writer, data_ptr)?;

    // Point the data pointer past the current write.
    // Types with relative offsets will already increment the data pointer.
    let current_pos = writer.seek(SeekFrom::Current(0))?;
    if current_pos > *data_ptr {
        *data_ptr = current_pos;
    }

    writer.seek(SeekFrom::Start(pos_after_offset))?;
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

// TODO: This could just be derived as RelPtr64<NullString> but requires different alignment.
impl SsbhWrite for SsbhString8 {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;
        if *data_ptr <= current_pos {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        write_ssbh_string_aligned(writer, &self.0, data_ptr, 8)?;
        Ok(())
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
        let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;
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

impl SsbhWrite for NrpdState {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        match self {
            NrpdState::Sampler(sampler) => sampler.write_ssbh(writer, data_ptr),
            NrpdState::RasterizerState(rasterizer) => rasterizer.write_ssbh(writer, data_ptr),
            NrpdState::DepthState(depth) => depth.write_ssbh(writer, data_ptr),
            NrpdState::BlendState(blend) => blend.write_ssbh(writer, data_ptr),
        }
    }

    fn size_in_bytes(&self) -> u64 {
        match self {
            NrpdState::Sampler(sampler) => sampler.size_in_bytes(),
            NrpdState::RasterizerState(rasterizer) => rasterizer.size_in_bytes(),
            NrpdState::DepthState(depth) => depth.size_in_bytes(),
            NrpdState::BlendState(blend) => blend.size_in_bytes(),
        }
    }
}

impl SsbhWrite for FrameBuffer {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        match self {
            FrameBuffer::Framebuffer0(v) => v.write_ssbh(writer, data_ptr),
            FrameBuffer::Framebuffer1(v) => v.write_ssbh(writer, data_ptr),
            FrameBuffer::Framebuffer2(v) => v.write_ssbh(writer, data_ptr),
        }
    }

    fn size_in_bytes(&self) -> u64 {
        match self {
            FrameBuffer::Framebuffer0(v) => v.size_in_bytes(),
            FrameBuffer::Framebuffer1(v) => v.size_in_bytes(),
            FrameBuffer::Framebuffer2(v) => v.size_in_bytes(),
        }
    }
}

pub fn write_anim<W: Write + Seek>(writer: &mut W, data: &Anim) -> std::io::Result<()> {
    write_ssbh_header(writer, b"MINA")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += data.size_in_bytes(); // size of fields

    // TODO: Find a less redundant way of handling alignment/padding.
    if data.major_version == 2 && data.minor_version == 1 {
        data_ptr += 32;
    }

    data.write_ssbh(writer, &mut data_ptr)?;

    // Padding was added for version 2.1 compared to 2.0.
    if data.major_version == 2 && data.minor_version == 1 {
        // Pad the header.
        writer.write_all(&[0u8; 32])?;

        // The newer file revision is also aligned to a multiple of 4.
        let total_size = writer.seek(SeekFrom::End(0))?;
        let new_size = round_up(total_size, 4);
        for _ in 0..(new_size - total_size) {
            writer.write_all(&[0u8])?;
        }
    }

    Ok(())
}

pub fn write_ssbh_to_file<P: AsRef<Path>>(path: P, data: &Ssbh) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    write_buffered(&mut file, |c| write_ssbh(c, data))?;
    Ok(())
}

pub fn write_ssbh<W: Write + Seek>(writer: &mut W, data: &Ssbh) -> std::io::Result<()> {
    match &data.data {
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

fn write_buffered<W: Write + Seek, F: Fn(&mut Cursor<Vec<u8>>) -> std::io::Result<()>>(
    writer: &mut W,
    write_data: F,
) -> std::io::Result<()> {
    // The write implementations for relative offsets and arrays seek using large offsets,
    // which can cause lots of flushes with buffered writers
    // Buffer the entire write operation into memory to improve performance.
    let mut cursor = Cursor::new(Vec::new());
    write_data(&mut cursor)?;

    writer.write_all(cursor.get_mut())?;
    Ok(())
}

// TODO: This can probably just be derived.
fn write_ssbh_file<W: Write + Seek, S: SsbhWrite>(
    writer: &mut W,
    data: &S,
    magic: &[u8; 4],
) -> std::io::Result<()> {
    write_ssbh_header(writer, magic)?;
    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

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
    use crate::SsbhEnum64;
    use binread::BinRead;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        // Remove any whitespace used to make the tests more readable.
        let no_whitespace: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(no_whitespace).unwrap()
    }

    #[test]
    fn write_nested_rel_ptr_depth2() {
        let value = RelPtr64::<RelPtr64<u32>>(RelPtr64::<u32>(7u32));

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
    }

    #[test]
    fn write_ssbh_array_ssbh_string() {
        let value = SsbhArray::<SsbhString> {
            elements: vec![
                SsbhString::from("leyes_eye_mario_l_col"),
                SsbhString::from("eye_mario_w_nor"),
            ],
        };

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
            data: RelPtr64::<TestData>(TestData::Float(1.0f32)),
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
            data: RelPtr64::<TestData>(TestData::Unsigned(5u32)),
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
