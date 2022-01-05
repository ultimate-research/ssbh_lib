use std::convert::{TryFrom, TryInto};
use std::io::{Cursor, Seek, SeekFrom};

use binread::io::StreamPosition;
use ssbh_lib::{formats::shdr::ShaderType, Shdr};

use binread::{BinRead, BinResult};
// Smush Shaders:
// binary data header is always at offset 2896?
// header for program binary is 80 bytes
// order of strings matches declaration order in shader?
use binread::{BinReaderExt, NullString};

#[derive(Debug)]
pub struct ShdrData {
    pub shaders: Vec<ShaderEntryData>,
}

#[derive(Debug)]
pub struct ShaderEntryData {
    name: String,
    shader_type: ShaderType,
    unk1: BinaryData,
}

// 108 Bytes
#[derive(Debug, BinRead)]
struct UnkEntry {
    unk1: u32,
    offset: u32,
    #[br(pad_after = 32)]
    length: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: i32,
    #[br(pad_after = 44)]
    unk6: i32
}

// 164 Bytes
#[derive(Debug, BinRead)]
struct UnkEntry2 {
    offset: u32,
    #[br(pad_after = 32)]
    length: u32,
    unk1: u32, // TODO: Data type?
    unk2: i32, // TODO: associated index into the first section entries?
    uniform_buffer_offset: i32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
    unk7: u32,
    #[br(pad_after = 92)]
    unk8: u32,
}

// 92 Bytes
#[derive(Debug, BinRead)]
struct UnkEntry3 {
    offset: u32,
    #[br(pad_after = 84)]
    length: u32,
}

#[derive(Debug, BinRead)]
struct UnkHeader {
    file_end_relative_offset: u32,
    #[br(pad_after = 36)]
    string_info_section_offset: u32,

    count1: u32,
    unk1: u32,
    count2: u32,
    relative_offset2: u32,
    #[br(pad_after = 32)]
    count3: u32,

    string_info_end_relative_offset: u32,
    string_section_length: u32,
    string_section_relative_offset: u32,
}

#[derive(Debug)]
pub struct BinaryData {}

impl BinaryData {
    pub fn from_bytes(bytes: &[u8]) -> BinResult<Self> {
        let mut reader = Cursor::new(bytes);

        // Some sort of header for the string section?
        reader.seek(SeekFrom::Start(288))?;
        let header: UnkHeader = reader.read_le()?;

        let string_info_section_offset2 =
            header.string_info_section_offset + header.relative_offset2;
        let string_section_offset =
            header.string_info_section_offset + header.string_section_relative_offset;

        // TODO: Handle this using BinRead?
        reader.seek(SeekFrom::Start(header.string_info_section_offset as u64))?;
        for i in 0..header.count1 {
            let before_struct = reader.stream_pos()?;
            let entry: UnkEntry = reader.read_le()?;
            let current_pos = reader.stream_pos()?;
            // println!("{:?}", current_pos);

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}, {:?}", text.into_string(), before_struct);
            println!("{:#?}", entry);
            reader.seek(SeekFrom::Start(current_pos))?;
        }

        println!();

        reader.seek(SeekFrom::Start(string_info_section_offset2 as u64))?;
        for i in 0..header.count2 {
            let before_struct = reader.stream_pos()?;
            let entry: UnkEntry2 = reader.read_le()?;
            let current_pos = reader.stream_pos()?;

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}, {:?}", text.into_string(), before_struct);
            println!("{:#?}", entry);
            reader.seek(SeekFrom::Start(current_pos))?;
        }

        println!();

        // What determines the output count?
        // HACK: Add 1 for the output.
        for i in 0..(header.count3 + 1) {
            let entry: UnkEntry3 = reader.read_le()?;
            let current_pos = reader.stream_pos()?;
            // println!("{:#?}", current_pos);

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}", text.into_string());

            reader.seek(SeekFrom::Start(current_pos))?;
        }

        Ok(BinaryData {})
    }
}

impl TryFrom<Shdr> for ShdrData {
    type Error = std::convert::Infallible;

    fn try_from(shdr: Shdr) -> Result<Self, Self::Error> {
        shdr.try_into()
    }
}

impl TryFrom<&Shdr> for ShdrData {
    type Error = std::convert::Infallible;

    fn try_from(shdr: &Shdr) -> Result<Self, Self::Error> {
        Ok(Self {
            shaders: shdr
                .shaders
                .elements
                .iter()
                .map(|s| ShaderEntryData {
                    name: s.name.to_string_lossy(),
                    shader_type: s.shader_type,
                    unk1: BinaryData::from_bytes(&s.shader_binary.elements).unwrap(),
                })
                .collect(),
        })
    }
}
