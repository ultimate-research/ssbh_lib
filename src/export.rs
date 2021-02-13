use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    formats::{
        modl::*,
        nufx::*,
        shdr::{Shader, Shdr},
        skel::*,
    },
    Matrix4x4, Ssbh, SsbhFile, SsbhString, Vector4,
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

fn write_ssbh_string_aligned<W: Write + Seek>(
    writer: &mut W,
    data: &SsbhString,
    data_ptr: &mut u64,
    alignment: u64,
) -> std::io::Result<()> {
    // 4 byte align strings.
    *data_ptr = round_up(*data_ptr, alignment);

    write_relative_offset(writer, data_ptr)?;

    // string data
    let current_pos = writer.seek(SeekFrom::Current(0))?;
    writer.seek(SeekFrom::Start(*data_ptr))?;

    // TODO: Handle null strings.
    if data.value.0.len() == 0 {
        // Handle empty strings.
        writer.write_u32::<LittleEndian>(0u32)?;
    } else {
        // Write the data and null terminator.
        writer.write(&data.value.0)?;
        writer.write(&[0u8])?;
    }

    *data_ptr = writer.seek(SeekFrom::Current(0))?;
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

pub fn write_nufx<W: Write + Seek>(writer: &mut W, data: &Nufx) -> std::io::Result<()> {
    write_ssbh_header(writer, b"XFUN")?;

    let mut data_ptr = writer.seek(SeekFrom::Current(0))?;

    // Point past the struct.
    data_ptr += 36; // size of fields

    writer.write_u16::<LittleEndian>(data.major_version)?;
    writer.write_u16::<LittleEndian>(data.minor_version)?;

    // Handle both versions.
    match &data.programs {
        ShaderPrograms::ProgramsV0(programsV0) => {
            write_array_aligned(
                writer,
                &programsV0.elements,
                &mut data_ptr,
                write_shader_program_v0,
                80,
                8,
            )?;
        }
        ShaderPrograms::ProgramsV1(programsV1) => {
            write_array_aligned(
                writer,
                &programsV1.elements,
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
