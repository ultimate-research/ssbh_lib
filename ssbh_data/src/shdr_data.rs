use binrw::io::{Cursor, Seek, SeekFrom};
use ssbh_lib::formats::shdr::{ShaderType, Shdr};
use std::convert::{TryFrom, TryInto};
use std::io::Read;

use binrw::{binread, BinRead, BinResult, VecArgs};
// Smush Shaders:
// binary data header is always at offset 2896?
// header for program binary is 80 bytes
// order of strings matches declaration order in shader?
use binrw::{BinReaderExt, NullString};

#[derive(Debug)]
pub struct ShdrData {
    pub shaders: Vec<ShaderEntryData>,
}

// TODO: Convert the binary data to another format?
#[derive(Debug)]
pub struct ShaderEntryData {
    pub name: String,
    pub shader_type: ShaderType,
    pub binary_data: BinaryData,
}

// TODO: Represent the entire binary data using binrw?
#[derive(Debug, BinRead)]
pub struct BinaryData {
    #[br(seek_before = SeekFrom::Start(288))]
    pub header: UnkHeader,
}

// TODO: Get name information after parsing?
// TODO: Are all relative offsets relative to entry_offset?
#[derive(Debug, BinRead)]
pub struct UnkHeader {
    pub file_end_relative_offset: u32,
    pub entry_offset: u32,
    // All zeros?
    #[br(pad_after = 32)]
    pub unk_header_1: u32,

    // TODO: Use RelPtr?
    // TODO: Make the counts temp fields?
    pub unk_entry_count: u32,
    #[br(args(entry_offset, unk_entry_count))]
    pub unk_entries: UnkPtr<BufferEntry>,

    pub uniform_count: u32,
    #[br(args(entry_offset, uniform_count))]
    pub uniforms: UnkPtr<UniformEntry>,

    pub input_count: u32,
    #[br(args(entry_offset, input_count))]
    pub inputs: UnkPtr<AttributeEntry>,

    pub output_count: u32,
    #[br(args(entry_offset, output_count))]
    pub outputs: UnkPtr<AttributeEntry>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub string_info_end_relative_offset: u32,
    pub string_section_length: u32,
    pub string_section_relative_offset: u32,
}

// TODO: Allow custom starting offset for RelPtr?
#[derive(Debug)]
pub struct UnkPtr<T>(pub Vec<T>);

impl<T: BinRead<Args = ()>> BinRead for UnkPtr<T> {
    type Args = (u32, u32);

    fn read_options<R: std::io::Read + Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let (entry_offset, count) = args;
        let relative_offset = u32::read_options(reader, options, ())?;
        let saved_pos = reader.stream_position()?;

        reader.seek(SeekFrom::Start(
            entry_offset as u64 + relative_offset as u64,
        ))?;
        let value = <Vec<T>>::read_options(
            reader,
            options,
            VecArgs {
                count: count as usize,
                inner: (),
            },
        )?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(UnkPtr(value))
    }
}

// TODO: Create a type for string offset + length?
// TODO: Parse strings using binrw?
// 108 Bytes
#[derive(Debug, BinRead)]
pub struct BufferEntry {
    #[br(pad_after = 32)]
    pub name: EntryString,
    pub used_size_in_bytes: u32, // used size of this uniform buffer?
    pub unk3: i32,               // number of parameters in the buffer?
    pub unk4: i32,
    pub unk5: i32,
    pub unk6: i32,
    pub unk7: i32,
    pub unk8: i32,
    pub unk9: i32,
    #[br(pad_after = 32)]
    pub unk10: i32,
}

// 164 Bytes
#[derive(Debug, BinRead)]
pub struct UniformEntry {
    #[br(pad_after = 32)]
    pub name: EntryString,
    pub data_type: DataType,
    pub buffer_slot: i32,
    pub uniform_buffer_offset: i32,
    pub unk4: i32,
    pub unk5: i32,
    pub unk6: i32,
    pub unk7: i32,
    pub unk8: i32,
    pub unk10: i32,
    pub unk11: i32, // -1 for non textures, index of the texture in nufxlb (how to account for shadow map?)
    pub unk12: i32,
    pub unk13: i32,
    pub unk14: i32,
    pub unk15: i32,
    pub unk16: i32,
    #[br(pad_after = 60)]
    pub unk17: i32, // 0 = texture, 1 = ???, 257 = element 0 of matrix, struct, array?
}

// TODO: Is there better name for in/out keywords in shading languages?
// 92 Bytes
#[derive(Debug, BinRead)]
pub struct AttributeEntry {
    #[br(pad_after = 32)]
    pub name: EntryString,
    pub data_type: DataType,
    pub unk2: i32,
    /// The attribute location like `layout (location = 1)` in GLSL.
    /// Builtin variables like `gl_Position` use a value of `-1`.
    pub location: i32,
    pub unk4: i32,
    #[br(pad_after = 32)]
    pub unk5: u32, // 0, 1, or 2
}

#[derive(Debug, BinRead)]
pub struct EntryString {
    pub offset: u32,
    pub length: u32,
}

// TODO: Types are all aligned/padded?
#[derive(Debug, BinRead, PartialEq, Eq, Clone, Copy)]
#[binread(repr(u32))]
pub enum DataType {
    Boolean = 0, // 4 bytes
    /// A single 32-bit signed integer like gl_InstanceID.
    Int = 4,
    // TODO: What is this type?
    Unk7 = 7,
    /// A single 32-bit unsigned integer.
    UnsignedInt = 20,
    /// 3 32-bit unsigned integers like gl_GlobalInvocationID .
    UVec3 = 22,
    /// A single 32-bit float.
    Float = 36,
    /// 2 32-bit floats.
    Vector2 = 37,
    /// 3 32-bit floats.
    Vector3 = 38,
    // 4 32-bit floats like gl_Position.
    Vector4 = 39,
    Matrix4x4 = 50, // TODO: Is this actually a full matrix?
    /// sampler2D uniform in GLSL.
    Sampler2d = 67,
    /// sampler3D uniform in GLSL.
    Sampler3d = 68,
    /// samplerCube uniform in GLSL.
    SamplerCube = 69,
    /// sampler2DArray uniform in GLSL.
    Sampler2dArray = 73,
    /// image2D uniform in GLSL.
    Image2d = 103,
}

pub fn read_string<R: Read + Seek>(
    reader: &mut R,
    header: &UnkHeader,
    s: &EntryString,
) -> BinResult<String> {
    let strings_start = header.entry_offset as u64 + header.string_section_relative_offset as u64;
    reader.seek(SeekFrom::Start(strings_start + s.offset as u64))?;

    let mut bytes = vec![0u8; (s.length as usize).saturating_sub(1)];
    reader.read_exact(&mut bytes)?;

    Ok(String::from_utf8_lossy(&bytes).to_string())
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
        // TODO: Convert the binary data to a higher level representation.
        // TODO: How to include strings?
        // TODO: Rebuild Shdr from ShdrData?
        // TODO: Avoid unwrap.
        Ok(Self {
            shaders: match shdr {
                Shdr::V12 { shaders } => shaders
                    .elements
                    .iter()
                    .map(|s| {
                        let mut reader = Cursor::new(&s.shader_binary.elements);

                        ShaderEntryData {
                            name: s.name.to_string_lossy(),
                            shader_type: s.shader_type,
                            binary_data: reader.read_le().unwrap(),
                        }
                    })
                    .collect(),
            },
        })
    }
}
