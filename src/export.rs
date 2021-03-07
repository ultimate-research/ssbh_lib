use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    anim::*,
    formats::{
        mesh::*,
        nrpd::{NrpdState, RenderPassDataType},
    },
    matl::*,
    shdr::*,
    skel::*,
    RelPtr64, SsbhArray, SsbhByteBuffer, SsbhEnum64, SsbhString8, SsbhWrite,
    {Ssbh, SsbhFile, SsbhString},
};

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

impl<T: binread::BinRead + SsbhWrite> SsbhWrite for SsbhArray<T> {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // Arrays are always 8 byte aligned?
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
        // TODO: This isn't T::size_in_bytes() due to difficulties getting SsbhArray<T>
        // instead of SsbhArray::<T> to work with the macro.
        if let Some(element) = self.elements.first() {
            // TODO: This won't work for SsbhArray<Option<T>> since only the first element is checked.
            *data_ptr += self.elements.len() as u64 * element.size_in_bytes();
        }

        for element in &self.elements {
            element.write_ssbh(writer, data_ptr)?;
        }
        writer.seek(SeekFrom::Start(current_pos))?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        16
    }

    fn alignment_in_bytes(&self) -> u64 {
        8
    }
}

impl SsbhWrite for SsbhString {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        write_ssbh_string_aligned(writer, self, data_ptr, 4)?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        8
    }
}

fn write_string_data<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhString,
    _data_ptr: &mut u64,
) -> std::io::Result<()> {
    // TODO: Handle null strings?
    if data.value.0.len() == 0 {
        // Handle empty strings.
        writer.write_u32::<LittleEndian>(0u32)?;
    } else {
        // Write the data and null terminator.
        writer.write_all(&data.value.0)?;
        writer.write_all(&[0u8])?;
    }
    Ok(())
}

fn write_ssbh_string_aligned<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhString,
    data_ptr: &mut u64,
    alignment: u64,
) -> std::io::Result<()> {
    write_rel_ptr_aligned(writer, data, data_ptr, write_string_data, alignment)?;
    Ok(())
}

fn write_rel_ptr_aligned<W: Write + Seek, T, F: Fn(&mut W, &T, &mut u64) -> std::io::Result<()>>(
    writer: &mut W,
    data: &T,
    data_ptr: &mut u64,
    write_t: F,
    alignment: u64,
) -> std::io::Result<()> {
    // Calculate the relative offset.
    let initial_pos = writer.seek(SeekFrom::Current(0))?;
    *data_ptr = round_up(*data_ptr, alignment);
    if *data_ptr == initial_pos {
        // HACK: workaround to fix nested relative offsets such as RelPtr64<SsbhString>.
        // This fixes the case where the current data pointer is identical to the writer position.
        *data_ptr += std::mem::size_of::<u64>() as u64;
    }
    write_relative_offset(writer, data_ptr)?;

    // Write the data at the specified offset.
    let current_pos = writer.seek(SeekFrom::Current(0))?;
    writer.seek(SeekFrom::Start(*data_ptr))?;

    // TODO: Does this correctly update the data pointer?
    write_t(writer, data, data_ptr)?;

    // Point the data pointer past the current write.
    // Types with relative offsets will already increment the data pointer.
    let pos_after_write = writer.seek(SeekFrom::Current(0))?;
    if pos_after_write > *data_ptr {
        *data_ptr = pos_after_write;
    }

    writer.seek(SeekFrom::Start(current_pos))?;
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

impl SsbhWrite for SsbhString8 {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
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
        // Calculate the relative offset.
        let initial_pos = writer.seek(SeekFrom::Current(0))?;
        *data_ptr = round_up(*data_ptr, self.0.alignment_in_bytes());
        if *data_ptr == initial_pos {
            // HACK: workaround to fix nested relative offsets such as RelPtr64<SsbhString>.
            // This fixes the case where the current data pointer is identical to the writer position.
            *data_ptr += 8u64;
        }
        write_relative_offset(writer, data_ptr)?;

        // Write the data at the specified offset.
        let current_pos = writer.seek(SeekFrom::Current(0))?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        // The inner type may also update the data pointer.
        self.0.write_ssbh(writer, data_ptr)?;

        // Point the data pointer past the current write.
        // Types with relative offsets will already increment the data pointer.
        let pos_after_write = writer.seek(SeekFrom::Current(0))?;
        if pos_after_write > *data_ptr {
            *data_ptr = pos_after_write;
        }

        writer.seek(SeekFrom::Start(current_pos))?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        8
    }
}

impl<T: SsbhWrite + binread::BinRead<Args = (u64,)>> SsbhWrite for SsbhEnum64<T> {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // TODO: Avoid duplicating code with other relative offsets?
        // Calculate the relative offset.
        *data_ptr = round_up(*data_ptr, self.data.alignment_in_bytes());

        write_relative_offset(writer, data_ptr)?;
        writer.write_u64::<LittleEndian>(self.data_type)?;

        // Write the data at the specified offset.
        let current_pos = writer.seek(SeekFrom::Current(0))?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        // TODO: Does this correctly update the data pointer?
        self.data.write_ssbh(writer, data_ptr)?;

        // Point the data pointer past the current write.
        // Types with relative offsets will already increment the data pointer.
        let pos_after_write = writer.seek(SeekFrom::Current(0))?;
        if pos_after_write > *data_ptr {
            *data_ptr = pos_after_write;
        }

        writer.seek(SeekFrom::Start(current_pos))?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        16
    }
}

// TODO: Macro to implement SsbhWrite for tuples?
impl SsbhWrite for (SsbhString, SsbhString) {
    fn write_ssbh<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        self.0.write_ssbh(writer, data_ptr)?;
        self.1.write_ssbh(writer, data_ptr)?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        self.0.size_in_bytes() + self.1.size_in_bytes()
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
