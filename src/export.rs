use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
    iter::Filter,
    path::Path,
};

use crate::{
    anim::*,
    formats::{
        mesh::*,
        nrpd::{NrpdState, RenderPassDataType},
    },
    matl::*,
    modl::*,
    nufx::*,
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

// TODO: Macro to generate bitfield implementations?
impl SsbhWrite for SkelEntryFlags {
    fn write_ssbh<W: Write + Seek>(
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

impl SsbhWrite for RiggingFlags {
    fn write_ssbh<W: Write + Seek>(
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
        // TODO: arrays are always 8 byte aligned?
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

fn write_byte_buffer<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhByteBuffer,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    *data_ptr = round_up(*data_ptr, 8);

    // Don't write the offset for empty arrays.
    if data.elements.is_empty() {
        writer.write_u64::<LittleEndian>(0u64)?;
    } else {
        write_relative_offset(writer, &data_ptr)?;
    }
    writer.write_u64::<LittleEndian>(data.elements.len() as u64)?;

    let current_pos = writer.seek(SeekFrom::Current(0))?;
    writer.seek(SeekFrom::Start(*data_ptr))?;

    // Pointers in array elements should point past the end of the array.
    *data_ptr += data.elements.len() as u64;

    writer.write_all(&data.elements)?;
    writer.seek(SeekFrom::Start(current_pos))?;

    Ok(())
}

fn write_array_aligned<W: Write + Seek, T, F: Fn(&mut W, &T, &mut u64) -> std::io::Result<()>>(
    writer: &mut W,
    elements: &[T],
    data_ptr: &mut u64,
    write_t: F,
    size_of_t: u64,
    alignment: u64,
) -> std::io::Result<()> {
    // TODO: arrays are always 8 byte aligned?
    *data_ptr = round_up(*data_ptr, alignment);

    // Don't write the offset for empty arrays.
    if elements.is_empty() {
        writer.write_u64::<LittleEndian>(0u64)?;
    } else {
        write_relative_offset(writer, &data_ptr)?;
    }
    writer.write_u64::<LittleEndian>(elements.len() as u64)?;

    let current_pos = writer.seek(SeekFrom::Current(0))?;
    writer.seek(SeekFrom::Start(*data_ptr))?;

    // Pointers in array elements should point past the end of the array.
    // TODO: size_of_t should be known at compile time
    *data_ptr += elements.len() as u64 * size_of_t;

    for element in elements {
        write_t(writer, element, data_ptr)?;
    }
    writer.seek(SeekFrom::Start(current_pos))?;

    Ok(())
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

fn write_ssbh_string<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhString,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    // Strings are typically 4 byte aligned.
    write_ssbh_string_aligned(writer, data, data_ptr, 4)?;
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

fn write_param<W: Write + Seek>(
    writer: &mut W,
    data: &Param,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    match data {
        Param::Float(f) => writer.write_f32::<LittleEndian>(*f),
        Param::Boolean(b) => writer.write_u32::<LittleEndian>(*b),
        Param::Vector4(v) => v.write_ssbh(writer, data_ptr),
        Param::MatlString(text) => write_ssbh_string(writer, text, data_ptr),
        Param::Sampler(sampler) => sampler.write_ssbh(writer, data_ptr),
        Param::UvTransform(transform) => transform.write_ssbh(writer, data_ptr),
        Param::BlendState(blend_state) => write_matl_blend_state(writer, &blend_state, data_ptr),
        Param::RasterizerState(rasterizer_state) => {
            write_matl_rasterizer_state(writer, &rasterizer_state, data_ptr)
        }
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
            *data_ptr += std::mem::size_of::<u64>() as u64;
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
        // TODO: Avoid duplicating code?
        // Calculate the relative offset.
        let initial_pos = writer.seek(SeekFrom::Current(0))?;
        *data_ptr = round_up(*data_ptr, self.data.alignment_in_bytes());
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
        self.data.write_ssbh(writer, data_ptr)?;

        // Point the data pointer past the current write.
        // Types with relative offsets will already increment the data pointer.
        let pos_after_write = writer.seek(SeekFrom::Current(0))?;
        if pos_after_write > *data_ptr {
            *data_ptr = pos_after_write;
        }

        writer.seek(SeekFrom::Start(current_pos))?;

        writer.write_u64::<LittleEndian>(self.data_type)?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        16
    }
}

// TODO: Macro to implement SsbhWrite for enums?
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

fn write_matl_attribute<W: Write + Seek>(
    writer: &mut W,
    data: &MatlAttribute,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    // Different param types are aligned differently.
    // TODO: Just store data_type with the SsbhEnum?

    // Params have a param_id, offset, and type.
    writer.write_u64::<LittleEndian>(data.param_id as u64)?;
    write_rel_ptr_aligned(writer, &data.param.data, data_ptr, write_param, 8)?;
    writer.write_u64::<LittleEndian>(data.param.data_type)?;

    Ok(())
}

fn write_matl_blend_state<W: Write + Seek>(
    writer: &mut W,
    data: &MatlBlendState,
    _data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u32::<LittleEndian>(data.source_color as u32)?;
    writer.write_u32::<LittleEndian>(data.unk2)?;
    writer.write_u32::<LittleEndian>(data.destination_color as u32)?;
    writer.write_u32::<LittleEndian>(data.unk4)?;
    writer.write_u32::<LittleEndian>(data.unk5)?;
    writer.write_u32::<LittleEndian>(data.unk6)?;
    writer.write_u32::<LittleEndian>(data.unk7)?;
    writer.write_u32::<LittleEndian>(data.unk8)?;
    writer.write_u32::<LittleEndian>(data.unk9)?;
    writer.write_u32::<LittleEndian>(data.unk10)?;

    // TODO: make padding part of the struct definition?
    writer.write_u64::<LittleEndian>(0u64)?;
    Ok(())
}

fn write_matl_rasterizer_state<W: Write + Seek>(
    writer: &mut W,
    data: &MatlRasterizerState,
    _data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u32::<LittleEndian>(data.fill_mode as u32)?;
    writer.write_u32::<LittleEndian>(data.cull_mode as u32)?;
    writer.write_f32::<LittleEndian>(data.depth_bias)?;
    writer.write_f32::<LittleEndian>(data.unk4)?;
    writer.write_f32::<LittleEndian>(data.unk5)?;
    writer.write_u32::<LittleEndian>(data.unk6)?;

    // TODO: make padding part of the struct definition?
    writer.write_u64::<LittleEndian>(0u64)?;
    Ok(())
}

fn write_matl_entry<W: Write + Seek>(
    writer: &mut W,
    data: &MatlEntry,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.material_label, data_ptr)?;
    write_array_aligned(
        writer,
        &data.attributes.elements,
        data_ptr,
        write_matl_attribute,
        24,
        8,
    )?;
    write_ssbh_string(writer, &data.shader_label, data_ptr)?;
    Ok(())
}

pub fn write_matl<W: Write + Seek>(writer: &mut W, data: &Matl) -> std::io::Result<()> {
    write_ssbh_header(writer, b"LTAM")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += 20; // size of fields

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;

    write_array_aligned(
        writer,
        &data.entries.elements,
        &mut data_ptr,
        write_matl_entry,
        32,
        8,
    )?;
    Ok(())
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

fn write_mesh_bone_buffer<W: Write + Seek>(
    writer: &mut W,
    data: &MeshBoneBuffer,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.bone_name, data_ptr)?;
    write_byte_buffer(writer, &data.data, data_ptr)?;
    Ok(())
}

fn write_mesh_rigging_group<W: Write + Seek>(
    writer: &mut W,
    data: &MeshRiggingGroup,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.mesh_object_name, data_ptr)?;
    writer.write_u64::<LittleEndian>(data.mesh_object_sub_index)?;
    writer.write(&data.flags.into_bytes())?;
    write_array_aligned(
        writer,
        &data.buffers.elements,
        data_ptr,
        write_mesh_bone_buffer,
        8,
        8,
    )?;
    Ok(())
}

fn write_mesh_attribute_v8<W: Write + Seek>(
    writer: &mut W,
    data: &MeshAttributeV8,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u32::<LittleEndian>(data.usage as u32)?;
    writer.write_u32::<LittleEndian>(data.data_type as u32)?;
    writer.write_u32::<LittleEndian>(data.buffer_index)?;
    writer.write_u32::<LittleEndian>(data.buffer_offset)?;
    writer.write_u32::<LittleEndian>(data.sub_index)?;
    Ok(())
}

fn write_mesh_attribute_v10<W: Write + Seek>(
    writer: &mut W,
    data: &MeshAttributeV10,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u32::<LittleEndian>(data.usage as u32)?;
    writer.write_u32::<LittleEndian>(data.data_type as u32)?;
    writer.write_u32::<LittleEndian>(data.buffer_index)?;
    writer.write_u32::<LittleEndian>(data.buffer_offset)?;
    writer.write_u64::<LittleEndian>(data.sub_index)?;
    write_ssbh_string(writer, &data.name, data_ptr)?;
    write_array_aligned(
        writer,
        &data.attribute_names.elements,
        data_ptr,
        write_ssbh_string,
        8,
        8,
    )?;
    Ok(())
}

fn write_u32<W: Write + Seek>(
    writer: &mut W,
    data: &u32,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u32::<LittleEndian>(*data)?;
    Ok(())
}

fn write_mesh_object<W: Write + Seek>(
    writer: &mut W,
    data: &MeshObject,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    writer.write_i64::<LittleEndian>(data.sub_index)?;
    write_ssbh_string(writer, &data.parent_bone_name, data_ptr)?;
    writer.write_u32::<LittleEndian>(data.vertex_count)?;
    writer.write_u32::<LittleEndian>(data.vertex_index_count)?;
    writer.write_u32::<LittleEndian>(data.unk2)?;
    writer.write_u32::<LittleEndian>(data.vertex_offset)?;
    writer.write_u32::<LittleEndian>(data.vertex_offset2)?;
    writer.write_u32::<LittleEndian>(data.final_buffer_offset)?;
    writer.write_i32::<LittleEndian>(data.buffer_index)?;
    writer.write_u32::<LittleEndian>(data.stride)?;
    writer.write_u32::<LittleEndian>(data.stride2)?;
    writer.write_u32::<LittleEndian>(data.unk6)?;
    writer.write_u32::<LittleEndian>(data.unk7)?;
    writer.write_u32::<LittleEndian>(data.element_offset)?;
    writer.write_u32::<LittleEndian>(data.unk8)?;
    writer.write_u32::<LittleEndian>(data.draw_element_type as u32)?;
    writer.write_u32::<LittleEndian>(data.rigging_type as u32)?;
    writer.write_i32::<LittleEndian>(data.unk11)?;
    writer.write_u32::<LittleEndian>(data.unk12)?;
    data.bounding_info.write_ssbh(writer, data_ptr)?;
    match &data.attributes {
        MeshAttributes::AttributesV8(attributes_v8) => {
            write_array_aligned(
                writer,
                &attributes_v8.elements,
                data_ptr,
                write_mesh_attribute_v8,
                20,
                8,
            )?;
        }
        MeshAttributes::AttributesV10(attributes_v10) => {
            write_array_aligned(
                writer,
                &attributes_v10.elements,
                data_ptr,
                write_mesh_attribute_v10,
                48,
                8,
            )?;
        }
    }
    Ok(())
}

pub fn write_mesh<W: Write + Seek>(writer: &mut W, data: &Mesh) -> std::io::Result<()> {
    write_ssbh_header(writer, b"HSEM")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += 244; // size of fields

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;

    write_ssbh_string(writer, &data.model_name, &mut data_ptr)?;
    data.bounding_info.write_ssbh(writer, &mut data_ptr)?;
    writer.write_u32::<LittleEndian>(data.unk1)?;
    write_array_aligned(
        writer,
        &data.objects.elements,
        &mut data_ptr,
        write_mesh_object,
        208,
        8,
    )?;
    write_array_aligned(
        writer,
        &data.buffer_sizes.elements,
        &mut data_ptr,
        write_u32,
        4,
        8,
    )?;
    writer.write_u64::<LittleEndian>(data.polygon_index_size)?;
    write_array_aligned(
        writer,
        &data.vertex_buffers.elements,
        &mut data_ptr,
        write_byte_buffer,
        16,
        8,
    )?;
    write_byte_buffer(writer, &data.polygon_buffer, &mut data_ptr)?;
    write_array_aligned(
        writer,
        &data.rigging_buffers.elements,
        &mut data_ptr,
        write_mesh_rigging_group,
        16,
        8,
    )?;
    writer.write_u64::<LittleEndian>(data.unknown_offset)?;
    writer.write_u64::<LittleEndian>(data.unknown_size)?;

    Ok(())
}

pub fn write_ssbh_to_file<P: AsRef<Path>>(path: P, data: &Ssbh) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    write_buffered(&mut file, |c| write_ssbh(c, data))?;
    Ok(())
}

pub fn write_ssbh<W: Write + Seek>(writer: &mut W, data: &Ssbh) -> std::io::Result<()> {
    match &data.data {
        SsbhFile::Modl(modl) => write_ssbh_file(writer, modl, b"LDOM")?,
        SsbhFile::Skel(skel) => write_ssbh_file(writer, skel, b"LEKS")?,
        SsbhFile::Nufx(nufx) => write_ssbh_file(writer, nufx, b"XFUN")?,
        SsbhFile::Shdr(shdr) => write_ssbh_file(writer, shdr, b"RDHS")?,
        SsbhFile::Matl(matl) => write_matl(writer, &matl)?,
        SsbhFile::Anim(anim) => write_anim(writer, &anim)?,
        SsbhFile::Hlpb(_) => {}
        SsbhFile::Mesh(mesh) => write_ssbh_file(writer, mesh, b"HSEM")?,
        SsbhFile::Nrpd(nrpd) => write_ssbh_file(writer, nrpd, b"DPRN")?,
    }
    Ok(())
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
