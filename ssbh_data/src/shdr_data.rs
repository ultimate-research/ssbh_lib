use binrw::io::{Cursor, Seek, SeekFrom};
use ssbh_lib::formats::shdr::{ShaderType, Shdr};
use std::convert::{TryFrom, TryInto};

use binrw::{binread, BinRead, BinResult};
// Smush Shaders:
// binary data header is always at offset 2896?
// header for program binary is 80 bytes
// order of strings matches declaration order in shader?
use binrw::{BinReaderExt, NullString};

#[derive(Debug)]
pub struct ShdrData {
    pub shaders: Vec<ShaderEntryData>,
}

#[derive(Debug)]
pub struct ShaderEntryData {
    pub name: String,
    pub shader_type: ShaderType,
    pub unk1: BinaryData,
}

// TODO: Create a type for string offset + length?
// TODO: Parse strings using binrw?
// 108 Bytes
#[derive(Debug, BinRead)]
pub struct UnkEntry {
    pub unk1: u32,
    pub name_offset: u32,
    #[br(pad_after = 32)]
    pub name_length: u32,
    pub used_size_in_bytes: u32, // used size of this uniform buffer?
    pub unk3: u32,               // number of parameters in the buffer?
    pub unk4: u32,
    pub unk5: i32,
    #[br(pad_after = 44)]
    pub unk6: i32,
}

// 164 Bytes
#[derive(Debug, BinRead)]
pub struct UnkEntry2 {
    pub name_offset: u32,
    #[br(pad_after = 32)]
    pub name_length: u32,
    pub data_type: UnkEntry2DataType,
    pub entry1_index: i32, // TODO: associated index into the first section entries?
    pub uniform_buffer_offset: i32,
    pub unk4: u32,
    pub unk5: i32,
    pub unk6: u32,
    pub unk7: u32,
    #[br(pad_after = 92)]
    pub unk8: u32,
}

#[derive(Debug, BinRead)]
#[binread(repr(u32))]
pub enum UnkEntry2DataType {
    Boolean = 0, // 4 bytes
    Unk4 = 4,
    Unk7 = 7,
    Unk20 = 20,
    Unk36 = 36,
    Unk37 = 37,
    Unk38 = 38,
    Vector4 = 39,   // 16 bytes
    Matrix4x4 = 50, // ??? bytes
    Sampler2D = 67,
    SamplerCube = 69,
}

// TODO: Is there a standard name for in/out keywords in shading languages?
// 92 Bytes
#[derive(Debug, BinRead)]
pub struct AttributeEntry {
    pub name_offset: u32,
    #[br(pad_after = 32)]
    pub name_length: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: i32,
    #[br(pad_after = 36)]
    pub unk4: u32,
}

// TODO: Are all relative offsets relative to entry_offset?
#[derive(Debug, BinRead)]
pub struct UnkHeader {
    pub file_end_relative_offset: u32,
    #[br(pad_after = 36)]
    pub entry_offset: u32,

    pub entry1_count: u32,
    pub unk1: u32,
    pub entry2_count: u32,
    pub entry2_relative_offset: u32,
    pub input_count: u32,
    pub input_relative_offset: u32,
    pub output_count: u32,
    pub output_relative_offset: u32,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,

    pub string_info_end_relative_offset: u32,
    pub string_section_length: u32,
    pub string_section_relative_offset: u32,
}

#[derive(Debug)]
pub struct BinaryData {}

impl BinaryData {
    pub fn from_bytes(bytes: &[u8]) -> BinResult<Self> {
        let mut reader = Cursor::new(bytes);

        // Some sort of header for the string section?
        reader.seek(SeekFrom::Start(288))?;
        let header: UnkHeader = reader.read_le()?;
        println!("{:#?}", header);

        let string_section_offset = header.entry_offset + header.string_section_relative_offset;

        // TODO: Handle this using BinRead?
        reader.seek(SeekFrom::Start(header.entry_offset as u64))?;
        for _ in 0..header.entry1_count {
            let before_struct = reader.stream_position()?;
            let entry: UnkEntry = reader.read_le()?;
            let after_struct = reader.stream_position()?;

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.name_offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}, {:?}", text.into_string(), before_struct);
            println!("{:#?}", entry);
            reader.seek(SeekFrom::Start(after_struct))?;
        }
        println!();

        reader.seek(SeekFrom::Start(
            (header.entry_offset + header.entry2_relative_offset) as u64,
        ))?;
        for _ in 0..header.entry2_count {
            let before_struct = reader.stream_position()?;
            let entry: UnkEntry2 = reader.read_le()?;
            let after_struct = reader.stream_position()?;

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.name_offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}, {:?}", text.into_string(), before_struct);
            println!("{:#?}", entry);
            reader.seek(SeekFrom::Start(after_struct))?;
        }
        println!();

        // Inputs
        reader.seek(SeekFrom::Start(
            (header.entry_offset + header.input_relative_offset) as u64,
        ))?;
        for _ in 0..header.input_count {
            let before_struct = reader.stream_position()?;
            let entry: AttributeEntry = reader.read_le()?;
            let after_struct = reader.stream_position()?;

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.name_offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}, {:?}", text.into_string(), before_struct);
            println!("{:#?}", entry);
            reader.seek(SeekFrom::Start(after_struct))?;
        }
        println!();

        // Outputs
        reader.seek(SeekFrom::Start(
            (header.entry_offset + header.output_relative_offset) as u64,
        ))?;
        for _ in 0..header.output_count {
            let before_struct = reader.stream_position()?;
            let entry: AttributeEntry = reader.read_le()?;
            let after_struct = reader.stream_position()?;

            reader.seek(SeekFrom::Start(
                (string_section_offset + entry.name_offset) as u64,
            ))?;
            let text: NullString = reader.read_le()?;

            // TODO: We can use the length to create a custom reader.
            println!("{:?}, {:?}", text.into_string(), before_struct);
            println!("{:#?}", entry);
            reader.seek(SeekFrom::Start(after_struct))?;
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
            shaders: match shdr {
                Shdr::V12 { shaders } => shaders
                    .elements
                    .iter()
                    .map(|s| ShaderEntryData {
                        name: s.name.to_string_lossy(),
                        shader_type: s.shader_type,
                        unk1: BinaryData::from_bytes(&s.shader_binary.elements).unwrap(),
                    })
                    .collect(),
            },
        })
    }
}
