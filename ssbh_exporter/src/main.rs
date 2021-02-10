use byteorder::{LittleEndian, WriteBytesExt};
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    mem::size_of,
};

use ssbh_lib::{formats::modl::*, RelPtr64, SsbhFile, SsbhString};

fn round_up(value: u64, n: u64) -> u64 {
    return ((value + n - 1) / n) * n;
}

fn write_relative_offset(output: &mut File, data_ptr: &u64) {
    let current_pos = output.seek(SeekFrom::Current(0)).unwrap();
    output
        .write_u64::<LittleEndian>(*data_ptr - current_pos)
        .unwrap();
}

fn write_array<T, F: Fn(&mut File, &T, &mut u64)>(
    output: &mut File,
    elements: &[T],
    data_ptr: &mut u64,
    write_t: F,
    size_of_t: u64,
) {
    println!("Element Size: {:?}", size_of::<T>());
    *data_ptr = round_up(*data_ptr, 4);

    write_relative_offset(output, &data_ptr);
    output
        .write_u64::<LittleEndian>(elements.len() as u64)
        .unwrap();

    let current_pos = output.seek(SeekFrom::Current(0)).unwrap();
    output.seek(SeekFrom::Start(*data_ptr)).unwrap();

    // Pointers in array elements should point past the end of the array.
    // TODO: size_of_t should be known at compile time
    *data_ptr += elements.len() as u64 * size_of_t;

    for element in elements {
        write_t(output, element, data_ptr);
    }
    // println!("Array after: {:?}", data_ptr);
    output.seek(SeekFrom::Start(current_pos)).unwrap();
}

fn write_ssbh_string(output: &mut File, data: &SsbhString, data_ptr: &mut u64) {
    // 4 byte align strings.
    *data_ptr = round_up(*data_ptr, 4);

    write_relative_offset(output, data_ptr);

    // string data
    let current_pos = output.seek(SeekFrom::Current(0)).unwrap();
    output.seek(SeekFrom::Start(*data_ptr)).unwrap();

    // Write the data and null terminator.
    output.write(&data.value.0).unwrap();
    output.write(&[0u8]).unwrap();

    *data_ptr = output.seek(SeekFrom::Current(0)).unwrap();
    output.seek(SeekFrom::Start(current_pos)).unwrap();
}

fn write_modl_entry(output: &mut File, data: &ModlEntry, data_ptr: &mut u64) {
    write_ssbh_string(output, &data.mesh_name, data_ptr);
    output.write_i64::<LittleEndian>(data.sub_index).unwrap();
    write_ssbh_string(output, &data.material_label, data_ptr);
}

fn write_modl(modl: &Modl, output: &mut File) {
    // TODO: write the ssbh header

    // Write header.
    output.write(b"HBSS").unwrap();
    output.write_u64::<LittleEndian>(64).unwrap();
    output.write_u32::<LittleEndian>(0).unwrap();
    output.write(b"LDOM").unwrap();

    // Point past the struct.
    let mut data_ptr = output.seek(SeekFrom::Current(0)).unwrap();
    data_ptr += 68; // size of Modl fields

    // TODO: size_of(SsbhString) should be 8 (don't use transparent?)
    output
        .write_u16::<LittleEndian>(modl.major_version)
        .unwrap();
    output
        .write_u16::<LittleEndian>(modl.minor_version)
        .unwrap();

    write_ssbh_string(output, &modl.model_file_name, &mut data_ptr);
    write_ssbh_string(output, &modl.skeleton_file_name, &mut data_ptr);

    write_array(
        output,
        &modl.material_file_names.elements,
        &mut data_ptr,
        write_ssbh_string,
        8,
    );

    output.write_u64::<LittleEndian>(modl.unk1).unwrap();
    write_ssbh_string(output, &modl.mesh_string, &mut data_ptr);
    write_array(
        output,
        &modl.entries.elements,
        &mut data_ptr,
        write_modl_entry,
        24,
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ssbh = ssbh_lib::read_ssbh(&args[1]).unwrap();
    let mut output = File::create(&args[2]).unwrap();
    match ssbh.data {
        SsbhFile::Modl(modl) => {
            write_modl(&modl, &mut output);
        }
        _ => {}
    }
}
