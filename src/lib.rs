pub mod export;
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
use std::{convert::TryInto, marker::PhantomData, path::Path};
use std::{fmt, fs, num::NonZeroU8};

#[cfg(feature = "derive_serde")]
use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::SerializeSeq,
};

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize, Serializer};

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
    F: Fn(&mut R, &ReadOptions, u64, C) -> BinResult<BR>,
    BR: BinRead,
    C,
>(
    reader: &mut R,
    read_element: F,
    options: &ReadOptions,
    args: C,
) -> BinResult<BR> {
    let pos_before_read = reader.seek(SeekFrom::Current(0))?;

    let relative_offset = u64::read_options(reader, options, ())?;
    let element_count = u64::read_options(reader, options, ())?;

    let saved_pos = reader.seek(SeekFrom::Current(0))?;

    reader.seek(SeekFrom::Start(pos_before_read + relative_offset))?;
    let result = read_element(reader, options, element_count, args);
    reader.seek(SeekFrom::Start(saved_pos))?;

    result
}

fn read_elements<C: Copy + 'static, BR: BinRead<Args = C>, R: Read + Seek>(
    reader: &mut R,
    options: &ReadOptions,
    count: u64,
    args: C,
) -> BinResult<Vec<BR>> {
    let mut elements = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let element = BR::read_options(reader, options, args)?;
        elements.push(element);
    }

    Ok(elements)
}

fn read_buffer<C, R: Read + Seek>(
    reader: &mut R,
    _options: &ReadOptions,
    count: u64,
    _args: C,
) -> BinResult<Vec<u8>> {
    let mut elements = vec![0u8; count as usize];
    reader.read_exact(&mut elements)?;
    Ok(elements)
}

/// A 64 bit file pointer to some data.

#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(Debug)]
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
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
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

#[cfg(feature = "derive_serde")]
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

#[cfg(feature = "derive_serde")]
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

#[cfg(feature = "derive_serde")]
impl<'de> Deserialize<'de> for SsbhString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(SsbhStringVisitor)
    }
}

#[cfg(feature = "derive_serde")]
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
#[derive(Debug)]
pub struct SsbhByteBuffer {
    pub elements: Vec<u8>,
}

struct SsbhByteBufferVisitor;

#[cfg(feature = "derive_serde")]
impl<'de> Visitor<'de> for SsbhByteBufferVisitor {
    type Value = SsbhByteBuffer;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Self::Value {
            elements: hex::decode(v)
                .map_err(|_| serde::de::Error::custom("Error decoding byte buffer hex string."))?,
        })
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(&v)
    }
}

#[cfg(feature = "derive_serde")]
impl<'de> Deserialize<'de> for SsbhByteBuffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(SsbhByteBufferVisitor)
    }
}

impl BinRead for SsbhByteBuffer {
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let elements = read_ssbh_array(reader, read_buffer, options, ())?;
        Ok(Self { elements })
    }
}

#[cfg(feature = "derive_serde")]
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
use binread::BinRead;
use ssbh_lib::{SsbhArray, Matrix4x4};

#[derive(BinRead)]
struct Transforms {
    array_relative_offset: u64,
    array_item_count: u64
}
```
 */
/// This can instead be expressed as the following struct with the array's item type being more explicit.
/**
```rust
use binread::BinRead;
use ssbh_lib::{SsbhArray, Matrix4x4};

#[derive(BinRead)]
struct Transforms {
    data: SsbhArray<Matrix4x4>,
}
```
 */
#[derive(Debug)]
pub struct SsbhArray<T: BinRead> {
    pub elements: Vec<T>,
}

impl<C: Copy + 'static, T: BinRead<Args = C>> BinRead for SsbhArray<T> {
    type Args = C;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: C,
    ) -> BinResult<Self> {
        let elements = read_ssbh_array(reader, read_elements, options, args)?;
        Ok(Self { elements })
    }
}

struct SsbhArrayVisitor<T>
where
    T: BinRead,
{
    phantom: PhantomData<T>,
}

impl<T: BinRead> SsbhArrayVisitor<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "derive_serde")]
impl<'de, T: BinRead + Deserialize<'de>> Visitor<'de> for SsbhArrayVisitor<T> {
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

        Ok(SsbhArray { elements })
    }
}

#[cfg(feature = "derive_serde")]
impl<'de, T: BinRead + Deserialize<'de>> Deserialize<'de> for SsbhArray<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(SsbhArrayVisitor::new())
    }
}

#[cfg(feature = "derive_serde")]
impl<T> Serialize for SsbhArray<T>
where
    T: BinRead + Serialize,
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
/// Parsing will fail if there is no matching variant for `data_type`.
/**
```rust
use binread::BinRead;

#[derive(BinRead)]
struct EnumData {
    data_relative_offset: u64,
    data_type: u64
}
```
 */
/// This can instead be expressed as the following struct.
/// The `T` type should have line to specify that it takes the data type as an argument.
/// `data_type` is automatically passed as an argument when reading `T`.
/**
```rust
use binread::BinRead;
use ssbh_lib::SsbhEnum64;

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
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
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
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
#[br(magic = b"HBSS")]
pub struct Ssbh {
    #[br(align_before = 0x10)]
    pub data: SsbhFile,
}

/// The associated magic and format for each SSBH type.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
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
    Nrpd(nrpd::Nrpd),

    #[br(magic = b"XFUN")]
    Nufx(nufx::Nufx),

    #[br(magic = b"RDHS")]
    Shdr(shdr::Shdr),
}

/// 3 contiguous floats for encoding XYZ or RGB data.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A row-major 3x3 matrix of contiguous floats.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct Matrix3x3 {
    pub row1: Vector3,
    pub row2: Vector3,
    pub row3: Vector3,
}

/// 4 contiguous floats for encoding XYZW or RGBA data.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// 4 contiguous floats for encoding RGBA data.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Color4f {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// A row-major 4x4 matrix of contiguous floats.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct Matrix4x4 {
    pub row1: Vector4,
    pub row2: Vector4,
    pub row3: Vector4,
    pub row4: Vector4,
}

/// A wrapper type that serializes the value and absolute offset of the start of the value
/// to aid in debugging.
#[cfg(feature = "derive_serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DebugPosition<T: BinRead<Args = ()> + Serialize> {
    val: T,
    pos: u64,
}

#[cfg(feature = "derive_serde")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_vector3() {
        let mut reader = Cursor::new(b"\x3f\x80\0\0\xc0\0\0\0\x3f\0\0\0");
        let value = reader.read_be::<Vector3>().unwrap();
        assert_eq!(1.0f32, value.x);
        assert_eq!(-2.0f32, value.y);
        assert_eq!(0.5f32, value.z);
    }

    #[test]
    fn read_vector4() {
        let mut reader = Cursor::new(b"\x3f\x80\0\0\xc0\0\0\0\x3f\0\0\0\x3f\x80\0\0");
        let value = reader.read_be::<Vector4>().unwrap();
        assert_eq!(1.0f32, value.x);
        assert_eq!(-2.0f32, value.y);
        assert_eq!(0.5f32, value.z);
        assert_eq!(1.0f32, value.w);
    }
}
