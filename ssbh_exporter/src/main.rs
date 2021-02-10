use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    mem::size_of,
};

use ssbh_lib::{
    formats::{modl::*, skel::*},
    Matrix4x4, SsbhFile, SsbhString, Vector4,
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

fn write_array<W: Write + Seek, T, F: Fn(&mut W, &T, &mut u64)>(
    writer: &mut W,
    elements: &[T],
    data_ptr: &mut u64,
    write_t: F,
    size_of_t: u64,
) {
    // TODO: fix element size for RelPtr64, SsbhString, and SsbhArray.
    println!("Element Size: {:?}", size_of::<T>());
    *data_ptr = round_up(*data_ptr, 4);

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
    // println!("Array after: {:?}", data_ptr);
    writer.seek(SeekFrom::Start(current_pos)).unwrap();
}

fn write_ssbh_string<W: Write + Seek>(writer: &mut W, data: &SsbhString, data_ptr: &mut u64) {
    // 4 byte align strings.
    *data_ptr = round_up(*data_ptr, 4);

    write_relative_offset(writer, data_ptr);

    // string data
    let current_pos = writer.seek(SeekFrom::Current(0)).unwrap();
    writer.seek(SeekFrom::Start(*data_ptr)).unwrap();

    // Write the data and null terminator.
    writer.write(&data.value.0).unwrap();
    writer.write(&[0u8]).unwrap();

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

fn write_modl<W: Write + Seek>(writer: &mut W, data: &Modl) {
    write_ssbh_header(writer, b"LDOM");

    let mut data_ptr = writer.seek(SeekFrom::Current(0)).unwrap();

    // Point past the struct.
    data_ptr += 68; // size of Modl fields

    println!("Modl Size: {:?}", size_of::<Modl>());

    // TODO: size_of(SsbhString) should be 8 (don't use transparent?)
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

fn write_vector4<W: Write + Seek>(writer: &mut W, data: &Vector4, data_ptr: &mut u64) {
    writer.write_f32::<LittleEndian>(data.x).unwrap();
    writer.write_f32::<LittleEndian>(data.y).unwrap();
    writer.write_f32::<LittleEndian>(data.z).unwrap();
    writer.write_f32::<LittleEndian>(data.w).unwrap();
}

fn write_skel<W: Write + Seek>(writer: &mut W, data: &Skel) {
    // TODO: Modify the data pointer in each function.
    // TODO: Create an trait for writing and modifying the data pointer?
    write_ssbh_header(writer, b"LEKS");

    let mut data_ptr = writer.seek(SeekFrom::Current(0)).unwrap();

    // Point past the struct.
    // TODO: This should be known at compile time
    data_ptr += 84; // size of fields

    println!("Skel Size: {:?}", size_of::<Modl>());

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
    write_array(
        writer,
        &data.world_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
    );
    write_array(
        writer,
        &data.inv_world_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
    );
    write_array(
        writer,
        &data.transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
    );
    write_array(
        writer,
        &data.inv_transforms.elements,
        &mut data_ptr,
        write_matrix4x4,
        64,
    );
}

fn main() {
    // TODO: Handle errors.
    // TODO: Serialize JSON.
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    let mut writer = File::create(&args[2]).unwrap();
    match ssbh.data {
        SsbhFile::Modl(modl) => {
            write_modl(&mut writer, &modl);
        }
        SsbhFile::Skel(skel) => {
            write_skel(&mut writer, &skel);
        }
        _ => {}
    }
}
