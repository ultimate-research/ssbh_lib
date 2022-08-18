//! Types for working with [Shdr] data in .nushdb files.
use binrw::io::{Cursor, Seek, SeekFrom};
use binrw::BinReaderExt;
use binrw::{binread, BinRead, BinResult, VecArgs};
use ssbh_lib::formats::shdr::{ShaderType, Shdr};
use std::convert::{TryFrom, TryInto};
use std::io::Read;

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

#[derive(Debug)]
pub struct BinaryData {
    pub buffers: Vec<Buffer>,
    pub uniforms: Vec<Uniform>,
    pub inputs: Vec<Attribute>,
    pub outputs: Vec<Attribute>,
}

impl BinaryData {
    fn new<R: Read + Seek>(reader: &mut R, shader: &ShaderBinary) -> Self {
        // TODO: Avoid unwrap.
        Self {
            buffers: shader
                .header
                .buffer_entries
                .0
                .iter()
                .map(|e| Buffer::new(reader, &shader.header, e))
                .collect(),
            uniforms: shader
                .header
                .uniforms
                .0
                .iter()
                .map(|e| Uniform::new(reader, &shader.header, e))
                .collect(),
            inputs: shader
                .header
                .inputs
                .0
                .iter()
                .map(|e| Attribute::new(reader, &shader.header, e))
                .collect(),
            outputs: shader
                .header
                .outputs
                .0
                .iter()
                .map(|e| Attribute::new(reader, &shader.header, e))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    pub name: String,
    pub used_size_in_bytes: u32,
}

impl Buffer {
    fn new<R: Read + Seek>(reader: &mut R, header: &UnkHeader, e: &BufferEntry) -> Self {
        // TODO: Avoid unwrap.
        Self {
            name: read_string(reader, header, &e.name).unwrap(),
            used_size_in_bytes: e.used_size_in_bytes,
        }
    }
}

#[derive(Debug)]
pub struct Uniform {
    pub name: String,
    pub data_type: DataType,
    pub buffer_slot: i32,
    pub uniform_buffer_offset: i32,
    pub unk11: i32,
}

impl Uniform {
    fn new<R: Read + Seek>(reader: &mut R, header: &UnkHeader, e: &UniformEntry) -> Self {
        // TODO: Avoid unwrap.
        Self {
            name: read_string(reader, header, &e.name).unwrap(),
            data_type: e.data_type,
            buffer_slot: e.buffer_slot,
            uniform_buffer_offset: e.uniform_buffer_offset,
            unk11: e.unk11,
        }
    }
}

#[derive(Debug)]
pub struct Attribute {
    pub name: String,
    pub data_type: DataType,
    pub location: i32, // TODO: Use None for builtins?
}

impl Attribute {
    fn new<R: Read + Seek>(reader: &mut R, header: &UnkHeader, e: &AttributeEntry) -> Self {
        // TODO: Avoid unwrap.
        Self {
            name: read_string(reader, header, &e.name).unwrap(),
            data_type: e.data_type,
            location: e.location,
        }
    }
}

// TODO: Shader binary to binary data
impl BinaryData {
    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        let shader: ShaderBinary = reader.read_le()?;
        Ok(Self::new(&mut reader, &shader))
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        let shader: ShaderBinary = reader.read_le()?;
        Ok(Self::new(reader, &shader))
    }
}

// Smush Shaders:
// binary data header is always at offset 2896?
// header for program binary is 80 bytes
// TODO: Separate module for binary parsing?
// TODO: Represent the entire binary data using binrw?
#[derive(BinRead)]
struct ShaderBinary {
    #[br(seek_before = SeekFrom::Start(288))]
    header: UnkHeader,
}

// TODO: Get name information after parsing?
// TODO: Are all relative offsets relative to entry_offset?
#[allow(dead_code)]
#[derive(BinRead)]
struct UnkHeader {
    file_end_relative_offset: u32,
    entry_offset: u32,
    // All zeros?
    #[br(pad_after = 32)]
    unk1: u32,

    // TODO: Use RelPtr?
    // TODO: Make the counts temp fields?
    buffer_count: u32,
    #[br(args(entry_offset, buffer_count))]
    buffer_entries: UnkPtr<BufferEntry>,

    uniform_count: u32,
    #[br(args(entry_offset, uniform_count))]
    uniforms: UnkPtr<UniformEntry>,

    input_count: u32,
    #[br(args(entry_offset, input_count))]
    inputs: UnkPtr<AttributeEntry>,

    output_count: u32,
    #[br(args(entry_offset, output_count))]
    outputs: UnkPtr<AttributeEntry>,

    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
    unk7: u32,
    string_info_end_relative_offset: u32,
    string_section_length: u32,
    string_section_relative_offset: u32,
}

// TODO: Allow custom starting offset for RelPtr?
#[derive(Debug)]
struct UnkPtr<T>(Vec<T>);

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
#[allow(dead_code)]
#[derive(BinRead)]
struct BufferEntry {
    #[br(pad_after = 32)]
    name: EntryString,
    used_size_in_bytes: u32, // used size of this uniform buffer?
    unk3: i32,               // number of parameters in the buffer?
    unk4: i32,
    unk5: i32,
    unk6: i32,
    unk7: i32,
    unk8: i32,
    unk9: i32,
    #[br(pad_after = 32)]
    unk10: i32,
}

// 164 Bytes
#[allow(dead_code)]
#[derive(BinRead)]
struct UniformEntry {
    #[br(pad_after = 32)]
    name: EntryString,
    data_type: DataType,
    buffer_slot: i32,
    uniform_buffer_offset: i32,
    unk4: i32,
    unk5: i32,
    unk6: i32,
    unk7: i32,
    unk8: i32,
    unk10: i32,
    unk11: i32, // -1 for non textures, index of the texture in nufxlb (how to account for shadow map?)
    unk12: i32,
    unk13: i32,
    unk14: i32,
    unk15: i32,
    unk16: i32,
    #[br(pad_after = 60)]
    unk17: i32, // 0 = texture, 1 = ???, 257 = element 0 of matrix, struct, array?
}

// TODO: Is there better name for in/out keywords in shading languages?
// 92 Bytes
#[allow(dead_code)]
#[derive(BinRead)]
struct AttributeEntry {
    #[br(pad_after = 32)]
    name: EntryString,
    data_type: DataType,
    unk2: i32,
    // The attribute location like `layout (location = 1)` in GLSL.
    // Builtin variables like `gl_Position` use a value of `-1`.
    location: i32,
    unk4: i32,
    #[br(pad_after = 32)]
    unk5: u32, // 0, 1, or 2
}

#[derive(BinRead)]
struct EntryString {
    offset: u32,
    length: u32,
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

fn read_string<R: Read + Seek>(
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
        // TODO: Rebuild Shdr from ShdrData?
        // TODO: Avoid unwrap.
        Ok(Self {
            shaders: match shdr {
                Shdr::V12 { shaders } => shaders
                    .elements
                    .iter()
                    .map(|s| {
                        let mut reader = Cursor::new(&s.shader_binary.elements);
                        let shader: ShaderBinary = reader.read_le().unwrap();
                        ShaderEntryData {
                            name: s.name.to_string_lossy(),
                            shader_type: s.shader_type,
                            binary_data: BinaryData::new(&mut reader, &shader),
                        }
                    })
                    .collect(),
            },
        })
    }
}

impl ShdrData {
    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Shdr::from_file(path)?.try_into().map_err(Into::into)
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        Shdr::read(reader)?.try_into().map_err(Into::into)
    }
}

// TODO: Convert ShdrData -> Shdr.
// TODO: Tests.
