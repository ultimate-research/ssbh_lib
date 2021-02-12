use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{BufWriter, Cursor, Seek, SeekFrom, Write},
    mem::size_of,
    path::Path,
};

use crate::{
    formats::{modl::*, nufx::*, skel::*},
    Matrix4x4, SsbhString, Vector4,
};

fn round_up(value: u64, n: u64) -> u64 {
    // Find the next largest multiple of n.
    return ((value + n - 1) / n) * n;
}

fn write_relative_offset<W: Write + Seek>(writer: &mut W, data_ptr: &u64) {
    let current_pos = writer.seek(SeekFrom::Current(0)).unwrap();
    writer
        .write_u64::<LittleEndian>(*data_ptr - current_pos)
        .unwrap();
}

fn write_array_aligned<W: Write + Seek, T, F: Fn(&mut W, &T, &mut u64)>(
    writer: &mut W,
    elements: &[T],
    data_ptr: &mut u64,
    write_t: F,
    size_of_t: u64,
    alignment: u64,
) {
    // TODO: fix element size for RelPtr64, SsbhString, and SsbhArray.
    *data_ptr = round_up(*data_ptr, alignment);

    write_relative_offset(writer, &data_ptr);
    writer
        .write_u64::<LittleEndian>(elements.len() as u64)
        .unwrap();

    let current_pos = writer.seek(SeekFrom::Current(0)).unwrap();
    writer.seek(SeekFrom::Start(*data_ptr)).unwrap();

    // Pointers in array elements should point past the end of the array.
    // TODO: size_of_t should be known at compile time
    *data_ptr += elements.len() as u64 * size_of_t;

    for element in elements {
        write_t(writer, element, data_ptr);
    }
    writer.seek(SeekFrom::Start(current_pos)).unwrap();
}

fn write_array<W: Write + Seek, T, F: Fn(&mut W, &T, &mut u64)>(
    writer: &mut W,
    elements: &[T],
    data_ptr: &mut u64,
    write_t: F,
    size_of_t: u64,
) {
    // strings are 4 byte aligned.
    // TODO: alignment rules for other types?
    write_array_aligned(writer, elements, data_ptr, write_t, size_of_t, 4);
}

fn write_ssbh_string<W: Write + Seek>(writer: &mut W, data: &SsbhString, data_ptr: &mut u64) {
    // 4 byte align strings.
    *data_ptr = round_up(*data_ptr, 4);

    write_relative_offset(writer, data_ptr);

    // string data
    let current_pos = writer.seek(SeekFrom::Current(0)).unwrap();
    writer.seek(SeekFrom::Start(*data_ptr)).unwrap();

    // TODO: Handle null strings.
    if data.value.0.len() == 0 {
        // Handle empty strings.
        writer.write_u32::<LittleEndian>(0u32).unwrap();
    } else {
        // Write the data and null terminator.
        writer.write(&data.value.0).unwrap();
        writer.write(&[0u8]).unwrap();
    }

    *data_ptr = writer.seek(SeekFrom::Current(0)).unwrap();
    writer.seek(SeekFrom::Start(current_pos)).unwrap();
}

fn write_modl_entry<W: Write + Seek>(writer: &mut W, data: &ModlEntry, data_ptr: &mut u64) {
    write_ssbh_string(writer, &data.mesh_name, data_ptr);
    writer.write_i64::<LittleEndian>(data.sub_index).unwrap();
    write_ssbh_string(writer, &data.material_label, data_ptr);
}

fn write_ssbh_header<W: Write + Seek>(writer: &mut W, magic: &[u8; 4]) {
    // Hardcode the header because this is shared for all SSBH formats.
    writer.write(b"HBSS").unwrap();
    writer.write_u64::<LittleEndian>(64).unwrap();
    writer.write_u32::<LittleEndian>(0).unwrap();
    writer.write(magic).unwrap();
}

fn write_vertex_attribute<W: Write + Seek>(
    writer: &mut W,
    data: &VertexAttribute,
    data_ptr: &mut u64,
) {
    write_ssbh_string(writer, &data.name, data_ptr);
    write_ssbh_string(writer, &data.attribute_name, data_ptr);
}

fn write_material_parameter<W: Write + Seek>(
    writer: &mut W,
    data: &MaterialParameter,
    data_ptr: &mut u64,
) {
    writer.write_u64::<LittleEndian>(data.param_id).unwrap();
    write_ssbh_string(writer, &data.parameter_name, data_ptr);
    writer.write_u64::<LittleEndian>(data.padding).unwrap();
}

fn write_shader_program<W: Write + Seek>(writer: &mut W, data: &ShaderProgram, data_ptr: &mut u64) {
    write_ssbh_string(writer, &data.name, data_ptr);
    write_ssbh_string(writer, &data.render_pass, data_ptr);

    write_ssbh_string(writer, &data.vertex_shader, data_ptr);
    write_ssbh_string(writer, &data.unk_shader1, data_ptr);
    write_ssbh_string(writer, &data.unk_shader2, data_ptr);
    write_ssbh_string(writer, &data.unk_shader3, data_ptr);
    write_ssbh_string(writer, &data.pixel_shader, data_ptr);
    write_ssbh_string(writer, &data.unk_shader4, data_ptr);

    write_array(
        writer,
        &data.vertex_attributes.elements,
        data_ptr,
        write_vertex_attribute,
        16,
    );
    write_array(
        writer,
        &data.material_parameters.elements,
        data_ptr,
        write_material_parameter,
        24,
    );
}

fn write_nufx_unk_item<W: Write + Seek>(writer: &mut W, data: &UnkItem, data_ptr: &mut u64) {
    write_ssbh_string(writer, &data.name, data_ptr);
    write_array(writer, &data.unk1.elements, data_ptr, write_ssbh_string, 8);
}

// TODO: avoid unwrap
pub fn write_nufx_to_file<P: AsRef<Path>>(path: P, data: &Nufx) {
    let mut file = File::create(path).unwrap();
    write_buffered(&mut file, |c| write_nufx(c, data));
}

pub fn write_nufx<W: Write + Seek>(writer: &mut W, data: &Nufx) {
    write_ssbh_header(writer, b"XFUN");

    let mut data_ptr = writer.seek(SeekFrom::Current(0)).unwrap();

    // Point past the struct.
    data_ptr += 36; // size of fields

    writer
        .write_u16::<LittleEndian>(data.major_version)
        .unwrap();
    writer
        .write_u16::<LittleEndian>(data.minor_version)
        .unwrap();

    write_array(
        writer,
        &data.programs.elements,
        &mut data_ptr,
        write_shader_program,
        96,
    );

    write_array(
        writer,
        &data.unk_string_list.elements,
        &mut data_ptr,
        write_nufx_unk_item,
        24,
    );
}

// TODO: avoid unwrap
pub fn write_modl_to_file<P: AsRef<Path>>(path: P, data: &Modl) {
    let mut file = File::create(path).unwrap();
    write_buffered(&mut file, |c| write_modl(c, data));
}

pub fn write_modl<W: Write + Seek>(writer: &mut W, data: &Modl) {
    write_ssbh_header(writer, b"LDOM");

    let mut data_ptr = writer.seek(SeekFrom::Current(0)).unwrap();

    // Point past the struct.
    data_ptr += 68; // size of Modl fields

    writer
        .write_u16::<LittleEndian>(data.major_version)
        .unwrap();
    writer
        .write_u16::<LittleEndian>(data.minor_version)
        .unwrap();

    write_ssbh_string(writer, &data.model_file_name, &mut data_ptr);
    write_ssbh_string(writer, &data.skeleton_file_name, &mut data_ptr);

    write_array(
        writer,
        &data.material_file_names.elements,
        &mut data_ptr,
        write_ssbh_string,
        8,
    );

    writer.write_u64::<LittleEndian>(data.unk1).unwrap();
    write_ssbh_string(writer, &data.mesh_string, &mut data_ptr);
    write_array(
        writer,
        &data.entries.elements,
        &mut data_ptr,
        write_modl_entry,
        24,
    );
}

fn write_skel_bone_entry<W: Write + Seek>(
    writer: &mut W,
    data: &SkelBoneEntry,
    data_ptr: &mut u64,
) {
    write_ssbh_string(writer, &data.name, data_ptr);
    writer.write_u16::<LittleEndian>(data.id).unwrap();
    writer.write_u16::<LittleEndian>(data.parent_id).unwrap();
    writer.write_u32::<LittleEndian>(data.unk_type).unwrap();
}

fn write_matrix4x4<W: Write + Seek>(writer: &mut W, data: &Matrix4x4, data_ptr: &mut u64) {
    write_vector4(writer, &data.row1, data_ptr);
    write_vector4(writer, &data.row2, data_ptr);
    write_vector4(writer, &data.row3, data_ptr);
    write_vector4(writer, &data.row4, data_ptr);
}

fn write_vector4<W: Write + Seek>(writer: &mut W, data: &Vector4, _data_ptr: &mut u64) {
    writer.write_f32::<LittleEndian>(data.x).unwrap();
    writer.write_f32::<LittleEndian>(data.y).unwrap();
    writer.write_f32::<LittleEndian>(data.z).unwrap();
    writer.write_f32::<LittleEndian>(data.w).unwrap();
}

fn write_buffered<W: Write + Seek, F: Fn(&mut Cursor<Vec<u8>>)>(writer: &mut W, write_data: F) {
    // The relative offset and array writers seek using large offsets.
    // Buffer the entire write operation into memory to enable writing the final result in order.
    // This greatly improves performance.
    let mut cursor = Cursor::new(Vec::new());
    write_data(&mut cursor);

    writer.write(cursor.get_mut()).unwrap();
}

pub fn write_skel_to_file<P: AsRef<Path>>(path: P, data: &Skel) {
    let mut file = File::create(path).unwrap();
    write_buffered(&mut file, |c| write_skel(c, data));
}

pub fn write_skel<W: Write + Seek>(writer: &mut W, data: &Skel) {
    // TODO: Modify the data pointer in each function.
    // TODO: Create an trait for writing and modifying the data pointer?
    write_ssbh_header(writer, b"LEKS");

    let mut data_ptr = writer.seek(SeekFrom::Current(0)).unwrap();

    // Point past the struct.
    // TODO: This should be known at compile time
    data_ptr += 84; // size of fields

    // TODO: size_of(SsbhString) should be 8 (don't use transparent?)
    writer
        .write_u16::<LittleEndian>(data.major_version)
        .unwrap();
    writer
        .write_u16::<LittleEndian>(data.minor_version)
        .unwrap();
    write_array(
        writer,
        &data.bone_entries.elements,
        &mut data_ptr,
        write_skel_bone_entry,
        16,
    );
    write_array_aligned(
        writer,
        &data.world_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        64,
    );
    write_array_aligned(
        writer,
        &data.inv_world_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        64,
    );
    write_array_aligned(
        writer,
        &data.transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        64,
    );
    write_array_aligned(
        writer,
        &data.inv_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
        64,
    );
}
