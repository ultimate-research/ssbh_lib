pub mod formats;

use self::formats::*;
use adj::Adj;
use binread::io::Cursor;
use binread::BinReaderExt;
use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, NullString, ReadOptions,
};
use meshex::MeshEx;
use serde::{de::{Error, SeqAccess, Visitor}, ser::SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};
use std::{convert::TryInto, marker::PhantomData, path::Path};
use std::{fmt, fs, num::NonZeroU8};

/// Attempts to read one of the SSBH file types based on the file magic.
pub fn read_ssbh<P: AsRef<Path>>(path: P) -> BinResult<Ssbh> {
    let mut file = Cursor::new(fs::read(path)?);
    file.read_le::<Ssbh>()
}

pub fn read_meshex<P: AsRef<Path>>(path: P) -> BinResult<MeshEx> {
    let mut file = Cursor::new(fs::read(path)?);
    file.read_le::<MeshEx>()
}

pub fn read_adjb<P: AsRef<Path>>(path: P) -> BinResult<Adj> {
    let mut file = Cursor::new(fs::read(path)?);
    file.read_le::<Adj>()
}

fn read_ssbh_array<
    R: Read + Seek,
    F: Fn(&mut R, &ReadOptions, u64) -> BinResult<BR>,
    BR: BinRead,
>(
    reader: &mut R,
    f: F,
    options: &ReadOptions,
) -> BinResult<BR> {
    let pos_before_read = reader.seek(SeekFrom::Current(0))?;

    let relative_offset = u64::read_options(reader, options, ())?;
    let element_count = u64::read_options(reader, options, ())?;

    let saved_pos = reader.seek(SeekFrom::Current(0))?;

    reader.seek(SeekFrom::Start(pos_before_read + relative_offset))?;
    let result = f(reader, options, element_count);
    reader.seek(SeekFrom::Start(saved_pos))?;

    result
}

fn read_elements<BR: BinRead<Args = ()>, R: Read + Seek>(
    reader: &mut R,
    options: &ReadOptions,
    count: u64,
) -> BinResult<Vec<BR>> {
    let mut elements = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let element = BR::read_options(reader, options, ())?;
        elements.push(element);
    }

    Ok(elements)
}

fn read_buffer<R: Read + Seek>(
    reader: &mut R,
    _options: &ReadOptions,
    count: u64,
) -> BinResult<Vec<u8>> {
    let mut elements = vec![0u8; count as usize];
    reader.read_exact(&mut elements)?;
    Ok(elements)
}

/// A 64 bit file pointer to some data.
#[derive(Serialize, Debug)]
#[repr(transparent)]
pub struct Ptr64<BR: BinRead>(BR);

impl<BR: BinRead> BinRead for Ptr64<BR> {
    type Args = BR::Args;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let offset = u64::read_options(reader, options, ())?;

        let saved_pos = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(offset))?;
        let value = BR::read_options(reader, options, args)?;

        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(Self(value))
    }
}

impl<BR: BinRead> core::ops::Deref for Ptr64<BR> {
    type Target = BR;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A 64 bit file pointer relative to the start of the pointer type.
#[derive(Serialize, Debug, Deserialize)]
#[repr(transparent)]
pub struct RelPtr64<BR: BinRead>(BR);

impl<BR: BinRead> BinRead for RelPtr64<BR> {
    type Args = BR::Args;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let pos_before_read = reader.seek(SeekFrom::Current(0))?;

        let relative_offset = u64::read_options(reader, options, ())?;

        let saved_pos = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(pos_before_read + relative_offset))?;
        let value = BR::read_options(reader, options, args)?;

        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(Self(value))
    }
}

impl<BR: BinRead> core::ops::Deref for RelPtr64<BR> {
    type Target = BR;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A C string stored inline. This will likely be wrapped in a pointer type.
#[derive(BinRead, Debug)]
pub struct InlineString {
    value: NullString,
}

impl Serialize for InlineString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match get_string(&self.value) {
            Some(text) => serializer.serialize_str(text),
            None => serializer.serialize_none(),
        }
    }
}

impl InlineString {
    pub fn get_string(&self) -> Option<&str> {
        get_string(&self.value)
    }
}

/// A C string with position determined by a relative offset.
#[derive(BinRead, Debug)]
pub struct SsbhString {
    pub value: RelPtr64<NullString>,
}

struct SsbhStringVisitor;

impl<'de> Visitor<'de> for SsbhStringVisitor {
    type Value = SsbhString;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let chars: Vec<NonZeroU8> = v.bytes().filter_map(|b| b.try_into().ok()).collect();
        Ok(Self::Value {
            value: RelPtr64(chars.into()),
        })
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(&v)
    }
}

impl<'de> Deserialize<'de> for SsbhString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(SsbhStringVisitor)
    }
}

impl Serialize for SsbhString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match get_string(&self.value) {
            Some(text) => serializer.serialize_str(text),
            None => serializer.serialize_none(),
        }
    }
}

impl SsbhString {
    pub fn get_string(&self) -> Option<&str> {
        get_string(&self.value)
    }
}

fn get_string(value: &NullString) -> Option<&str> {
    std::str::from_utf8(&value.0).ok()
}

/// A more performant type for parsing arrays of bytes.
#[derive(Debug, Deserialize)]
pub struct SsbhByteBuffer {
    pub elements: Vec<u8>,
}

// TODO: Implement deserialize to parse the hex string into a Vec<u8>

impl BinRead for SsbhByteBuffer {
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let elements = read_ssbh_array(reader, read_buffer, options)?;
        Ok(Self { elements })
    }
}

impl Serialize for SsbhByteBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.elements))
    }
}

/// A contigous, fixed size collection of elements with position determined by a relative offset.
/**
```rust
#[derive(BinRead)]
struct ArrayData {
    array_relative_offset: u64,
    array_item_count: u64
}
```
 */
/// This can instead be expressed as the following struct.
/**
```rust
#[derive(BinRead)]
struct ArrayData {
    data: SsbhArray<ArrayItemType>,
}
```
 */
#[derive(Debug)]
pub struct SsbhArray<T: BinRead<Args = ()>> {
    pub elements: Vec<T>,
}

impl<T> BinRead for SsbhArray<T>
where
    T: BinRead<Args = ()>,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let elements = read_ssbh_array(reader, read_elements, options)?;
        Ok(Self { elements })
    }
}

struct SsbhArrayVisitor<T> where T: BinRead<Args = ()> {
    phantom: PhantomData<T>
}

impl<T: BinRead<Args = ()>> SsbhArrayVisitor<T> {
    pub fn new() -> Self {
        Self { phantom: PhantomData}
    }
}

impl<'de, T: BinRead<Args = ()> + Deserialize<'de>> Visitor<'de> for SsbhArrayVisitor<T> {
    type Value = SsbhArray<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("ArrayKeyedMap key value sequence.")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut elements = Vec::new();
        while let Some(value) = seq.next_element()? {
            elements.push(value);
        }

        Ok(SsbhArray {elements})
    }
}

impl<'de, T: BinRead<Args = ()> + Deserialize<'de>> Deserialize<'de> for SsbhArray<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(SsbhArrayVisitor::new())
    }
}

impl<T> Serialize for SsbhArray<T>
where
    T: BinRead<Args = ()> + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.elements.len()))?;
        for e in &self.elements {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

/// Parses a struct with a relative offset to a structure of type T with some data type.
/**
```rust
#[derive(BinRead)]
struct EnumData {
    data_relative_offset: u64,
    data_type: u64
}
```
 */
/// This can instead be expressed as the following struct.
/// The T type should have line to specify that it takes the data type as an argument.
/**
```rust
#[derive(BinRead)]
#[br(import(data_type: u64))]
pub enum Data {
    #[br(pre_assert(data_type == 01u64))]
    Float(f32),
    #[br(pre_assert(data_type == 02u64))]
    Boolean(u32),
    // Add additional variants as needed.
}

#[derive(BinRead)]
pub struct EnumData {
    data: SsbhEnum64<Data>,
}
```
 */
///
#[derive(Serialize, Deserialize, Debug)]
pub struct SsbhEnum64<T: BinRead<Args = (u64,)>> {
    pub data: T,
}

impl<T> BinRead for SsbhEnum64<T>
where
    T: BinRead<Args = (u64,)>,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let pos_before_read = reader.seek(SeekFrom::Current(0))?;
        let ptr = u64::read_options(reader, options, ())?;
        let data_type = u64::read_options(reader, options, ())?;
        let saved_pos = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(pos_before_read + ptr))?;
        let value = T::read_options(reader, options, (data_type,))?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(SsbhEnum64 { data: value })
    }
}

/// The container type for the various SSBH formats.
#[derive(Serialize, Deserialize, BinRead, Debug)]
#[br(magic = b"HBSS")]
pub struct Ssbh {
    #[br(align_before = 0x10)]
    pub data: SsbhFile,
}

/// The associated magic and format for each SSBH type.
#[derive(Serialize, Deserialize, BinRead, Debug)]
pub enum SsbhFile {
    #[br(magic = b"BPLH")]
    Hlpb(hlpb::Hlpb),

    #[br(magic = b"LTAM")]
    Matl(matl::Matl),

    #[br(magic = b"LDOM")]
    Modl(modl::Modl),

    #[br(magic = b"HSEM")]
    Mesh(mesh::Mesh),

    #[br(magic = b"LEKS")]
    Skel(skel::Skel),

    #[br(magic = b"MINA")]
    Anim(anim::Anim),

    #[br(magic = b"DPRN")]
    Nprd(nrpd::Nrpd),

    #[br(magic = b"XFUN")]
    Nufx(nufx::Nufx),

    #[br(magic = b"RDHS")]
    Shdr,
}

/// 3 contiguous floats for encoding XYZ or RGB data.
#[derive(BinRead, Serialize, Deserialize, Debug)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A row-major 3x3 matrix of contiguous floats.
#[derive(BinRead, Serialize, Deserialize, Debug)]
pub struct Matrix3x3 {
    pub row1: Vector3,
    pub row2: Vector3,
    pub row3: Vector3,
}

/// 4 contiguous floats for encoding XYZW or RGBA data.
#[derive(BinRead, Serialize, Deserialize, Debug)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// 4 contiguous floats for encoding RGBA data.
#[derive(BinRead, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Color4f {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// A row-major 4x4 matrix of contiguous floats.
#[derive(BinRead, Serialize, Deserialize, Debug)]
pub struct Matrix4x4 {
    pub row1: Vector4,
    pub row2: Vector4,
    pub row3: Vector4,
    pub row4: Vector4,
}

/// A wrapper type that serializes the value and absolute offset of the start of the value
/// to aid in debugging.
#[derive(Debug, Serialize)]
pub struct DebugPosition<T: BinRead<Args = ()> + Serialize> {
    val: T,
    pos: u64,
}

impl<T> BinRead for DebugPosition<T>
where
    T: BinRead<Args = ()> + Serialize,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let pos = reader.seek(SeekFrom::Current(0))?;
        let val = T::read_options(reader, options, ())?;
        Ok(Self { val, pos })
    }
}
