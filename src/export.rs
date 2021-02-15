use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    anim::*,
    matl::*,
    modl::*,
    nufx::*,
    shdr::*,
    skel::*,
    {Color4f, Matrix4x4, Ssbh, SsbhFile, SsbhString, Vector4},
};

fn round_up(value: u64, n: u64) -> u64 {
    // Find the next largest multiple of n.
    return ((value + n - 1) / n) * n;
}

fn write_relative_offset<W: Write + Seek>(writer: &mut W, data_ptr: &u64) -> std::io::Result<()> {
    let current_pos = writer.seek(SeekFrom::Current(0))?;
    writer.write_u64::<LittleEndian>(*data_ptr - current_pos)?;
    Ok(())
}

fn write_byte_buffer_aligned<W: Write + Seek>(
    writer: &mut W,
    elements: &[u8],
    data_ptr: &mut u64,
    alignment: u64,
) -> std::io::Result<()> {
    *data_ptr = round_up(*data_ptr, alignment);

    // Don't write the offset for empty arrays.
    if elements.len() == 0 {
        writer.write_u64::<LittleEndian>(0u64)?;
    } else {
        write_relative_offset(writer, &data_ptr)?;
    }
    writer.write_u64::<LittleEndian>(elements.len() as u64)?;

    let current_pos = writer.seek(SeekFrom::Current(0))?;
    writer.seek(SeekFrom::Start(*data_ptr))?;

    // Pointers in array elements should point past the end of the array.
    *data_ptr += elements.len() as u64;

    writer.write(elements)?;
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
    if elements.len() == 0 {
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
        writer.write(&data.value.0)?;
        writer.write(&[0u8])?;
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

fn write_modl_entry<W: Write + Seek>(
    writer: &mut W,
    data: &ModlEntry,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.mesh_name, data_ptr)?;
    writer.write_i64::<LittleEndian>(data.sub_index)?;
    write_ssbh_string(writer, &data.material_label, data_ptr)?;
    Ok(())
}

fn write_ssbh_header<W: Write + Seek>(writer: &mut W, magic: &[u8; 4]) -> std::io::Result<()> {
    // Hardcode the header because this is shared for all SSBH formats.
    writer.write(b"HBSS")?;
    writer.write_u64::<LittleEndian>(64)?;
    writer.write_u32::<LittleEndian>(0)?;
    writer.write(magic)?;
    Ok(())
}

fn write_vertex_attribute<W: Write + Seek>(
    writer: &mut W,
    data: &VertexAttribute,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    write_ssbh_string(writer, &data.attribute_name, data_ptr)?;
    Ok(())
}

fn write_material_parameter<W: Write + Seek>(
    writer: &mut W,
    data: &MaterialParameter,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    // TODO: Why does the alignment change for some strings?
    writer.write_u64::<LittleEndian>(data.param_id)?;
    write_ssbh_string_aligned(writer, &data.parameter_name, data_ptr, 8)?;
    writer.write_u64::<LittleEndian>(data.padding)?;
    Ok(())
}

fn write_shader_program_v0<W: Write + Seek>(
    writer: &mut W,
    data: &ShaderProgramV0,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string_aligned(writer, &data.name, data_ptr, 8)?;
    write_ssbh_string(writer, &data.render_pass, data_ptr)?;

    write_ssbh_string(writer, &data.shaders.vertex_shader, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.unk_shader1, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.unk_shader2, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.geometry_shader, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.pixel_shader, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.compute_shader, data_ptr)?;

    write_array_aligned(
        writer,
        &data.material_parameters.elements,
        data_ptr,
        write_material_parameter,
        24,
        8,
    )?;
    Ok(())
}

fn write_shader_program_v1<W: Write + Seek>(
    writer: &mut W,
    data: &ShaderProgramV1,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string_aligned(writer, &data.name, data_ptr, 8)?;
    write_ssbh_string(writer, &data.render_pass, data_ptr)?;

    write_ssbh_string(writer, &data.shaders.vertex_shader, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.unk_shader1, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.unk_shader2, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.geometry_shader, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.pixel_shader, data_ptr)?;
    write_ssbh_string(writer, &data.shaders.compute_shader, data_ptr)?;

    write_array_aligned(
        writer,
        &data.vertex_attributes.elements,
        data_ptr,
        write_vertex_attribute,
        16,
        8,
    )?;
    write_array_aligned(
        writer,
        &data.material_parameters.elements,
        data_ptr,
        write_material_parameter,
        24,
        8,
    )?;
    Ok(())
}

fn write_nufx_unk_item<W: Write + Seek>(
    writer: &mut W,
    data: &UnkItem,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    write_array_aligned(
        writer,
        &data.unk1.elements,
        data_ptr,
        write_ssbh_string,
        8,
        8,
    )?;
    Ok(())
}

fn write_param<W: Write + Seek>(
    writer: &mut W,
    data: &Param,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    match data {
        Param::Float(f) => writer.write_f32::<LittleEndian>(*f)?,
        Param::Boolean(b) => writer.write_u32::<LittleEndian>(*b)?,
        Param::Vector4(v) => write_vector4(writer, v, data_ptr)?,
        Param::MatlString(text) => write_ssbh_string(writer, text, data_ptr)?,
        Param::Sampler(sampler) => write_matl_sampler(writer, &sampler, data_ptr)?,
        Param::UvTransform(transform) => write_matl_uv_transform(writer, &transform, data_ptr)?,
        Param::BlendState(blend_state) => write_matl_blend_state(writer, &blend_state, data_ptr)?,
        Param::RasterizerState(rasterizer_state) => {
            write_matl_rasterizer_state(writer, &rasterizer_state, data_ptr)?
        }
    }
    Ok(())
}

fn write_matl_attribute<W: Write + Seek>(
    writer: &mut W,
    data: &MatlAttribute,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    // Different param types are aligned differently.
    // TODO: Just store data_type with the SsbhEnum?
    let (data_type, alignment) = match data.param.data {
        crate::formats::matl::Param::Float(_) => (0x1u64, 8),
        crate::formats::matl::Param::Boolean(_) => (0x2u64, 8),
        crate::formats::matl::Param::Vector4(_) => (0x5u64, 8),
        crate::formats::matl::Param::MatlString(_) => (0xBu64, 8),
        crate::formats::matl::Param::Sampler(_) => (0xEu64, 8),
        crate::formats::matl::Param::UvTransform(_) => (0x10u64, 8),
        crate::formats::matl::Param::BlendState(_) => (0x11u64, 8),
        crate::formats::matl::Param::RasterizerState(_) => (0x12u64, 8),
    };

    // Params have a param_id, offset, and type.
    writer.write_u64::<LittleEndian>(data.param_id as u64)?;
    write_rel_ptr_aligned(writer, &data.param.data, data_ptr, write_param, alignment)?;
    writer.write_u64::<LittleEndian>(data_type)?;

    Ok(())
}

fn write_matl_uv_transform<W: Write + Seek>(
    writer: &mut W,
    data: &MatlUvTransform,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_f32::<LittleEndian>(data.x)?;
    writer.write_f32::<LittleEndian>(data.y)?;
    writer.write_f32::<LittleEndian>(data.z)?;
    writer.write_f32::<LittleEndian>(data.w)?;
    writer.write_f32::<LittleEndian>(data.v)?;
    Ok(())
}

fn write_matl_blend_state<W: Write + Seek>(
    writer: &mut W,
    data: &MatlBlendState,
    data_ptr: &mut u64,
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
    data_ptr: &mut u64,
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

fn write_matl_sampler<W: Write + Seek>(
    writer: &mut W,
    data: &MatlSampler,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u32::<LittleEndian>(data.wraps as u32)?;
    writer.write_u32::<LittleEndian>(data.wrapt as u32)?;
    writer.write_u32::<LittleEndian>(data.wrapr as u32)?;
    writer.write_u32::<LittleEndian>(data.min_filter as u32)?;
    writer.write_u32::<LittleEndian>(data.mag_filter as u32)?;
    writer.write_u32::<LittleEndian>(data.texture_filtering_type as u32)?;
    write_color4f(writer, &data.border_color, data_ptr)?;
    writer.write_u32::<LittleEndian>(data.unk11)?;
    writer.write_u32::<LittleEndian>(data.unk12)?;
    writer.write_f32::<LittleEndian>(data.lod_bias)?;
    writer.write_u32::<LittleEndian>(data.max_anisotropy)?;
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

pub fn write_nufx<W: Write + Seek>(writer: &mut W, data: &Nufx) -> std::io::Result<()> {
    write_ssbh_header(writer, b"XFUN")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += 36; // size of fields

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;

    // Handle both versions.
    match &data.programs {
        ShaderPrograms::ProgramsV0(programs_v0) => {
            write_array_aligned(
                writer,
                &programs_v0.elements,
                &mut data_ptr,
                write_shader_program_v0,
                80,
                8,
            )?;
        }
        ShaderPrograms::ProgramsV1(programs_v1) => {
            write_array_aligned(
                writer,
                &programs_v1.elements,
                &mut data_ptr,
                write_shader_program_v1,
                96,
                8,
            )?;
        }
    }

    write_array_aligned(
        writer,
        &data.unk_string_list.elements,
        &mut data_ptr,
        write_nufx_unk_item,
        24,
        8,
    )?;

    Ok(())
}

fn write_anim_track<W: Write + Seek>(
    writer: &mut W,
    data: &AnimTrack,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    writer.write_u32::<LittleEndian>(data.flags)?;
    writer.write_u32::<LittleEndian>(data.frame_count)?;
    writer.write_u32::<LittleEndian>(data.unk3)?;
    writer.write_u32::<LittleEndian>(data.data_offset)?;
    writer.write_u64::<LittleEndian>(data.data_size)?;
    Ok(())
}

fn write_anim_node<W: Write + Seek>(
    writer: &mut W,
    data: &AnimNode,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    write_array_aligned(
        writer,
        &data.tracks.elements,
        data_ptr,
        write_anim_track,
        32,
        8,
    )?;
    Ok(())
}

fn write_anim_group<W: Write + Seek>(
    writer: &mut W,
    data: &AnimGroup,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_u64::<LittleEndian>(data.anim_type as u64)?;
    write_array_aligned(
        writer,
        &data.nodes.elements,
        data_ptr,
        write_anim_node,
        24,
        8,
    )?;
    Ok(())
}

pub fn write_anim<W: Write + Seek>(writer: &mut W, data: &Anim) -> std::io::Result<()> {
    write_ssbh_header(writer, b"MINA")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += 52; // size of fields

    // TODO: Find a less redundant way of handling alignment/padding.
    if data.major_version == 2 && data.minor_version == 1 {
        data_ptr += 32;
    }

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;
    writer.write_f32::<LittleEndian>(data.final_frame_index)?;
    writer.write_u16::<LittleEndian>(data.unk1)?;
    writer.write_u16::<LittleEndian>(data.unk2)?;
    write_ssbh_string(writer, &data.name, &mut data_ptr)?;
    write_array_aligned(
        writer,
        &data.animations.elements,
        &mut data_ptr,
        write_anim_group,
        24,
        8,
    )?;
    write_byte_buffer_aligned(writer, &data.buffer.elements, &mut data_ptr, 8)?;

    // Padding was added for version 2.1 compared to 2.0.
    if data.major_version == 2 && data.minor_version == 1 {
        // Pad the header.
        writer.write(&[0u8; 32])?;

        // The newer file revision is also aligned to a multiple of 4.
        let total_size = writer.seek(SeekFrom::End(0))?;
        let new_size = round_up(total_size, 4);
        for _ in 0..(new_size - total_size) {
            writer.write(&[0u8])?;
        }
    }

    Ok(())
}

pub fn write_modl<W: Write + Seek>(writer: &mut W, data: &Modl) -> std::io::Result<()> {
    write_ssbh_header(writer, b"LDOM")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += 68; // size of Modl fields

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;

    write_ssbh_string(writer, &data.model_file_name, &mut data_ptr)?;
    write_ssbh_string(writer, &data.skeleton_file_name, &mut data_ptr)?;

    write_array_aligned(
        writer,
        &data.material_file_names.elements,
        &mut data_ptr,
        write_ssbh_string,
        8,
        8,
    )?;

    writer.write_u64::<LittleEndian>(data.unk1)?;
    write_ssbh_string(writer, &data.mesh_string, &mut data_ptr)?;
    write_array_aligned(
        writer,
        &data.entries.elements,
        &mut data_ptr,
        write_modl_entry,
        24,
        8,
    )?;
    Ok(())
}

fn write_skel_bone_entry<W: Write + Seek>(
    writer: &mut W,
    data: &SkelBoneEntry,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    writer.write_i16::<LittleEndian>(data.id)?;
    writer.write_i16::<LittleEndian>(data.parent_id)?;
    writer.write_u32::<LittleEndian>(data.unk_type)?;
    Ok(())
}

fn write_matrix4x4<W: Write + Seek>(
    writer: &mut W,
    data: &Matrix4x4,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_vector4(writer, &data.row1, data_ptr)?;
    write_vector4(writer, &data.row2, data_ptr)?;
    write_vector4(writer, &data.row3, data_ptr)?;
    write_vector4(writer, &data.row4, data_ptr)?;
    Ok(())
}

fn write_vector4<W: Write + Seek>(
    writer: &mut W,
    data: &Vector4,
    _data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_f32::<LittleEndian>(data.x)?;
    writer.write_f32::<LittleEndian>(data.y)?;
    writer.write_f32::<LittleEndian>(data.z)?;
    writer.write_f32::<LittleEndian>(data.w)?;
    Ok(())
}

fn write_color4f<W: Write + Seek>(
    writer: &mut W,
    data: &Color4f,
    _data_ptr: &mut u64,
) -> std::io::Result<()> {
    writer.write_f32::<LittleEndian>(data.r)?;
    writer.write_f32::<LittleEndian>(data.g)?;
    writer.write_f32::<LittleEndian>(data.b)?;
    writer.write_f32::<LittleEndian>(data.a)?;
    Ok(())
}

pub fn write_ssbh_to_file<P: AsRef<Path>>(path: P, data: &Ssbh) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    write_buffered(&mut file, |c| write_ssbh(c, data))?;
    Ok(())
}

pub fn write_ssbh<W: Write + Seek>(writer: &mut W, data: &Ssbh) -> std::io::Result<()> {
    match &data.data {
        SsbhFile::Modl(modl) => write_modl(writer, &modl)?,
        SsbhFile::Skel(skel) => write_skel(writer, &skel)?,
        SsbhFile::Nufx(nufx) => write_nufx(writer, &nufx)?,
        SsbhFile::Shdr(shdr) => write_shdr(writer, &shdr)?,
        SsbhFile::Matl(matl) => write_matl(writer, &matl)?,
        SsbhFile::Anim(anim) => write_anim(writer, &anim)?,
        _ => (),
    }
    Ok(())
}

fn write_buffered<W: Write + Seek, F: Fn(&mut Cursor<Vec<u8>>) -> std::io::Result<()>>(
    writer: &mut W,
    write_data: F,
) -> std::io::Result<()> {
    // The relative offset and array writers seek using large offsets.
    // Buffer the entire write operation into memory to enable writing the final result in order.
    // This greatly improves performance.
    let mut cursor = Cursor::new(Vec::new());
    write_data(&mut cursor)?;

    writer.write(cursor.get_mut())?;
    Ok(())
}

fn write_shader<W: Write + Seek>(
    writer: &mut W,
    data: &Shader,
    data_ptr: &mut u64,
) -> std::io::Result<()> {
    write_ssbh_string(writer, &data.name, data_ptr)?;
    writer.write_u32::<LittleEndian>(data.shader_type as u32)?;
    writer.write_u32::<LittleEndian>(data.unk3)?;
    write_byte_buffer_aligned(writer, &data.shader_binary.elements, data_ptr, 8)?;
    writer.write_u64::<LittleEndian>(data.unk4)?;
    writer.write_u64::<LittleEndian>(data.unk5)?;
    writer.write_u64::<LittleEndian>(data.binary_size)?;
    Ok(())
}

pub fn write_shdr<W: Write + Seek>(writer: &mut W, data: &Shdr) -> std::io::Result<()> {
    // TODO: Modify the data pointer in each function.
    // TODO: Create an trait for writing and modifying the data pointer?
    write_ssbh_header(writer, b"RDHS")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    // TODO: This should be known at compile time
    data_ptr += 20; // size of fields

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;
    write_array_aligned(
        writer,
        &data.shaders.elements,
        &mut data_ptr,
        write_shader,
        56,
        8,
    )?;
    Ok(())
}

pub fn write_skel<W: Write + Seek>(writer: &mut W, data: &Skel) -> std::io::Result<()> {
    // TODO: Modify the data pointer in each function.
    // TODO: Create an trait for writing and modifying the data pointer?
    write_ssbh_header(writer, b"LEKS")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    // TODO: This should be known at compile time
    data_ptr += 84; // size of fields

    // TODO: size_of(SsbhString) should be 8 (don't use transparent?)
    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;
    write_array_aligned(
        writer,
        &data.bone_entries.elements,
        &mut data_ptr,
        write_skel_bone_entry,
        16,
        8,
    )?;
    write_array_aligned(
        writer,
        &data.world_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        8,
    )?;
    write_array_aligned(
        writer,
        &data.inv_world_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        8,
    )?;
    write_array_aligned(
        writer,
        &data.transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        8,
    )?;
    write_array_aligned(
        writer,
        &data.inv_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        8,
    )?;
    Ok(())
}
