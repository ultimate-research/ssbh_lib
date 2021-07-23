//! # ssbh_lib
//!
//! ssbh_lib is a library for safe and efficient reading and writing of the SSBH binary formats used by Super Smash Bros Ultimate and some other games.
//! The library serves two purposes.
//!
//! The first is to provide high level and unambiguous documentation for the SSBH binary formats.
//! Strongly typed wrapper types such as [RelPtr64] replace ambiguous [u64] offsets. Enums and bitfields provide additional typing information vs [u8] or [u64] fields.
//! The structs and types in each of the format modules fully represent the binary data contained in the file.
//! This ensures the binary output of reading and writing a file without any modifications is identical to the original.
//!
//! The second is to eliminate the need to write tedious and error prone code for parsing and exporting binary data.
//! The use of procedural macros and provided types such as [SsbhString] and [SsbhArray] enforce the conventions used
//! by the SSBH format for calcualating relative offsets and alignment.
//!
//! ## Derive Macros
//! The majority of the reading and writing code is automatically generated from the struct and type definitions using procedural macros.
//! [binread_derive](https://crates.io/crates/binread_derive) generates the parsing code and [ssbh_write_derive](https://crates.io/crates/ssbh_write_derive) generates the exporting code.
//! Any changes to structs, enums, or other types used to define a file format will be automatically reflected in the generated read and write functions when the code is rebuilt.
//!
//! ## Example
//! A traditional struct definition for SSBH data may look like the following.
//! ```rust
//! struct FileData {
//!     name: u64,
//!     name_offset: u64,
//!     values_offset: u64,
//!     values_count: u64
//! }
//!```
//! The `FileData` struct has the correct size to represent the data on disk but has a number of issues.
//! The `values` array doesn't capture the fact that SSBH arrays are strongly typed.
//! It's not clear if the `name_offset` is an offset relative to the current position or some other buffer stored elsewhere in the file.
//!
//! Composing a combination of predefined SSBH types such as [SsbhString] with additional types implementing [SsbhWrite] and [BinRead]
//! improves the amount of type information for the data and makes the usage of offsets less ambiguous.
//! ```rust
//!
//! use ssbh_lib::SsbhArray;
//! use ssbh_lib::RelPtr64;
//! use ssbh_lib::SsbhString;
//! use ssbh_lib::SsbhWrite;
//! # #[macro_use] extern crate ssbh_write_derive;
//! use ssbh_write_derive::SsbhWrite;
//! use binread::BinRead;
//!
//! #[derive(BinRead, SsbhWrite)]
//! struct FileData {
//!     name: SsbhString,
//!     name_offset: RelPtr64<SsbhString>,
//!     values: SsbhArray<u32>    
//! }
//! # fn main() {}
//! ```
//! Now it's clear that `name` and `name_offset` are both null terminated strings, but `name_offset` has one more level of indirection.
//! In addition, `values` now has the correct typing information. The element count can be correctly calculated as `values.elements.len()`.
//! The reading and writing code is generated automatically by adding `#[derive(BinRead, SsbhWrite)]` to the struct.
pub mod formats;

mod export;

use self::formats::*;
use adj::Adj;
use binread::io::Cursor;
use binread::BinReaderExt;
use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, NullString, ReadOptions,
};
use formats::{
    anim::Anim, hlpb::Hlpb, matl::Matl, mesh::Mesh, modl::Modl, nrpd::Nrpd, nufx::Nufx, shdr::Shdr,
    skel::Skel,
};
use half::f16;
use meshex::MeshEx;
use ssbh_write_derive::SsbhWrite;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[cfg(feature = "derive_serde")]
use std::{convert::TryInto, fmt, marker::PhantomData, num::NonZeroU8};

#[cfg(feature = "derive_serde")]
use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::SerializeSeq,
};

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize, Serializer};

/// A trait for exporting types that are part of SSBH formats.
pub trait SsbhWrite: Sized {
    /// Writes the byte representation of `self` to `writer` and update `data_ptr` as needed to ensure the next relative offset is correctly calculated.
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()>;

    /// The offset in bytes between successive elements in an array of this type.
    /// This should include any alignment or padding.
    fn size_in_bytes(&self) -> u64 {
        std::mem::size_of::<Self>() as u64
    }

    /// The alignment of the relative_offset for types stored in a [RelPtr64].
    fn alignment_in_bytes(&self) -> u64 {
        std::mem::align_of::<Self>() as u64
    }
}

impl Ssbh {
    /// Tries to read one of the SSBH types from `path`.
    /// The entire file is buffered for performance.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = Cursor::new(fs::read(path)?);
        let ssbh = file.read_le::<Ssbh>()?;
        Ok(ssbh)
    }

    /// Tries to read one of the SSBH types from `reader`.
    /// For best performance when opening from a file, use `from_file` instead.
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        let ssbh = reader.read_le::<Ssbh>()?;
        Ok(ssbh)
    }

    /// Writes the data to the given writer.
    /// For best performance when writing to a file, use `write_to_file` instead.
    pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
        crate::export::write_ssbh_header_and_data(writer, &self.data)?;
        Ok(())
    }

    /// Writes the data to the given path.
    /// The entire file is buffered for performance.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        crate::export::write_buffered(&mut file, |c| {
            crate::export::write_ssbh_header_and_data(c, &self.data)
        })?;
        Ok(())
    }
}

/// Errors while reading SSBH files.
pub enum ReadSsbhError {
    /// An error occurred while trying to read the file.
    BinRead(binread::error::Error),
    /// An error occurred while trying to read the file.
    Io(std::io::Error),
    /// The type of SSBH file did not match the expected SSBH type.
    InvalidSsbhType,
}

impl std::error::Error for ReadSsbhError {}

impl From<binread::error::Error> for ReadSsbhError {
    fn from(e: binread::error::Error) -> Self {
        Self::BinRead(e)
    }
}

impl From<std::io::Error> for ReadSsbhError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for ReadSsbhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::fmt::Debug for ReadSsbhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadSsbhError::InvalidSsbhType => {
                write!(
                    f,
                    "The type of SSBH file did not match the expected SSBH type."
                )
            }
            ReadSsbhError::BinRead(err) => write!(f, "BinRead Error: {:?}", err),
            ReadSsbhError::Io(err) => write!(f, "IO Error: {:?}", err),
        }
    }
}

macro_rules! ssbh_read_write_impl {
    ($ty:ident, $ty2:path, $magic:expr) => {
        impl $ty {
            /// Tries to read the current SSBH type from `path`.
            /// The entire file is buffered for performance.
            pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ReadSsbhError> {
                let mut file = Cursor::new(fs::read(path)?);
                let ssbh = file.read_le::<Ssbh>()?;
                match ssbh.data {
                    $ty2(v) => Ok(v),
                    _ => Err(ReadSsbhError::InvalidSsbhType),
                }
            }

            /// Tries to read the current SSBH type from `reader`.
            /// For best performance when opening from a file, use `from_file` instead.
            pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, ReadSsbhError> {
                let ssbh = reader.read_le::<Ssbh>()?;
                match ssbh.data {
                    $ty2(v) => Ok(v),
                    _ => Err(ReadSsbhError::InvalidSsbhType),
                }
            }

            /// Tries to write the SSBH type to `writer`.
            /// For best performance when writing to a file, use `write_to_file` instead.
            pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
                crate::export::write_ssbh_file(writer, self, $magic)?;
                Ok(())
            }

            /// Tries to write the current SSBH type to `path`.
            /// The entire file is buffered for performance.
            pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
                let mut file = std::fs::File::create(path)?;
                crate::export::write_buffered(&mut file, |c| {
                    crate::export::write_ssbh_file(c, self, $magic)
                })?;
                Ok(())
            }
        }
    };
}

macro_rules! read_write_impl {
    ($ty:ident) => {
        impl $ty {
            /// Tries to read the type from `path`.
            /// The entire file is buffered for performance.
            pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
                let mut file = Cursor::new(fs::read(path)?);
                let value = file.read_le::<$ty>()?;
                Ok(value)
            }

            /// Tries to read the type from `reader`.
            /// For best performance when opening from a file, use `from_file` instead.
            pub fn read<R: Read + Seek>(
                reader: &mut R,
            ) -> Result<Self, Box<dyn std::error::Error>> {
                let value = reader.read_le::<$ty>()?;
                Ok(value)
            }

            /// Tries to write the type to `writer`.
            /// For best performance when writing to a file, use `write_to_file` instead.
            pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
                let mut data_ptr = 0;
                self.ssbh_write(writer, &mut data_ptr)?;
                Ok(())
            }

            /// Tries to write the type to `path`.
            /// The entire file is buffered for performance.
            pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
                let mut file = std::fs::File::create(path)?;
                crate::export::write_buffered(&mut file, |c| {
                    let mut data_ptr = 0;
                    self.ssbh_write(c, &mut data_ptr)
                })?;
                Ok(())
            }
        }
    };
}

ssbh_read_write_impl!(Hlpb, SsbhFile::Hlpb, b"BPLH");
ssbh_read_write_impl!(Matl, SsbhFile::Matl, b"LTAM");
ssbh_read_write_impl!(Modl, SsbhFile::Modl, b"LDOM");
ssbh_read_write_impl!(Mesh, SsbhFile::Mesh, b"HSEM");
ssbh_read_write_impl!(Skel, SsbhFile::Skel, b"LEKS");
ssbh_read_write_impl!(Anim, SsbhFile::Anim, b"MINA");
ssbh_read_write_impl!(Nrpd, SsbhFile::Nrpd, b"DPRN");
ssbh_read_write_impl!(Nufx, SsbhFile::Nufx, b"XFUN");
ssbh_read_write_impl!(Shdr, SsbhFile::Shdr, b"RDHS");

read_write_impl!(MeshEx);
read_write_impl!(Adj);

fn read_ssbh_array<
    R: Read + Seek,
    F: Fn(&mut R, &ReadOptions, u64, C) -> BinResult<BR>,
    BR: BinRead,
    C,
>(
    reader: &mut R,
    read_elements: F,
    options: &ReadOptions,
    args: C,
) -> BinResult<BR> {
    let pos_before_read = reader.stream_position()?;

    let relative_offset = u64::read_options(reader, options, ())?;
    let element_count = u64::read_options(reader, options, ())?;

    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(pos_before_read + relative_offset))?;
    let result = read_elements(reader, options, element_count, args);
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

/// A 64 bit file pointer relative to the start of the reader.

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
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

        let saved_pos = reader.stream_position()?;

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

/// A half precision floating point type used for data in buffers that supports conversions to and from `f32`.
#[derive(Debug)]
#[repr(transparent)]
pub struct Half(f16);

#[cfg(feature = "derive_serde")]
impl Serialize for Half {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f32(self.0.to_f32())
    }
}

#[cfg(feature = "derive_serde")]
struct HalfVisitor;

#[cfg(feature = "derive_serde")]
impl<'de> Visitor<'de> for HalfVisitor {
    type Value = Half;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an f32")
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.into())
    }
}

#[cfg(feature = "derive_serde")]
impl<'de> Deserialize<'de> for Half {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_f32(HalfVisitor)
    }
}

impl BinRead for Half {
    type Args = ();

    fn read_options<R: binread::io::Read + Seek>(
        reader: &mut R,
        options: &binread::ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let bits = u16::read_options(reader, options, args)?;
        let value = f16::from_bits(bits);
        Ok(Self(value))
    }
}

impl From<Half> for f32 {
    fn from(value: Half) -> Self {
        value.0.into()
    }
}

impl From<f32> for Half {
    fn from(value: f32) -> Self {
        Half(f16::from_f32(value))
    }
}

/// A 64 bit file pointer relative to the start of the pointer type.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
#[repr(transparent)]
pub struct RelPtr64<T: BinRead>(Option<T>);

impl<T: BinRead> RelPtr64<T> {
    /// Creates a relative offset for `value` that is not null.
    pub fn new(value: T) -> Self {
        Self(Some(value))
    }

    /// Creates a relative offset for a null value.
    pub fn null() -> Self {
        Self(None)
    }
}

impl<T: BinRead> From<Option<T>> for RelPtr64<T> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => Self::new(v),
            None => Self::null(),
        }
    }
}

impl<T: BinRead> BinRead for RelPtr64<T> {
    type Args = T::Args;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let pos_before_read = reader.stream_position()?;

        let relative_offset = u64::read_options(reader, options, ())?;
        if relative_offset == 0 {
            return Ok(Self(None));
        }

        let saved_pos = reader.stream_position()?;

        reader.seek(SeekFrom::Start(pos_before_read + relative_offset))?;
        let value = T::read_options(reader, options, args)?;

        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(Self(Some(value)))
    }
}

impl<T: BinRead> core::ops::Deref for RelPtr64<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A C string stored inline. This will likely be wrapped in a pointer type.
#[derive(BinRead, Debug, SsbhWrite)]
pub struct InlineString(NullString);

#[cfg(feature = "derive_serde")]
impl Serialize for InlineString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match get_str(&self.0) {
            Some(text) => serializer.serialize_str(text),
            None => serializer.serialize_none(),
        }
    }
}

#[cfg(feature = "derive_serde")]
struct InlineStringVisitor;

#[cfg(feature = "derive_serde")]
impl<'de> Visitor<'de> for InlineStringVisitor {
    type Value = InlineString;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let chars: Vec<NonZeroU8> = v.bytes().filter_map(|b| b.try_into().ok()).collect();
        Ok(InlineString(chars.into()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(&v)
    }
}

#[cfg(feature = "derive_serde")]
impl<'de> Deserialize<'de> for InlineString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(InlineStringVisitor)
    }
}

impl InlineString {
    pub fn get_str(&self) -> Option<&str> {
        get_str(&self.0)
    }
}

/// A 4-byte aligned [CString] with position determined by a relative offset.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SsbhString(RelPtr64<CString<4>>);

/// A null terminated string with a specified alignment.
/// The empty string is represented as `N` null bytes.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct CString<const N: usize>(InlineString);

impl SsbhString {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(RelPtr64::new(CString::<4>(InlineString(NullString(bytes)))))
    }

    /// Converts the underlying buffer to a [str].
    /// The result will be [None] if the offset is null or the conversion failed.
    pub fn to_str(&self) -> Option<&str> {
        match &self.0 .0 {
            Some(value) => value.0.get_str(),
            None => None,
        }
    }

    /// Converts the underlying buffer to a [String].
    /// Empty or null values are converted to empty strings.
    pub fn to_string_lossy(&self) -> String {
        self.to_str().unwrap_or("").to_string()
    }
}

impl FromStr for SsbhString {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<&str> for SsbhString {
    fn from(text: &str) -> Self {
        Self::from_bytes(text.to_string().into_bytes())
    }
}

impl From<String> for SsbhString {
    fn from(text: String) -> Self {
        Self::from_bytes(text.into_bytes())
    }
}

/// An 8-byte aligned [CString] with position determined by a relative offset.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[repr(transparent)]
pub struct SsbhString8(RelPtr64<CString<8>>);

impl SsbhString8 {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(RelPtr64::new(CString::<8>(InlineString(NullString(bytes)))))
    }

    /// Converts the underlying buffer to a [str].
    /// The result will be [None] if the offset is null or the conversion failed.
    pub fn to_str(&self) -> Option<&str> {
        match &self.0 .0 {
            Some(value) => value.0.get_str(),
            None => None,
        }
    }

    /// Converts the underlying buffer to a [String].
    /// Empty or null values are converted to empty strings.
    pub fn to_string_lossy(&self) -> String {
        self.to_str().unwrap_or("").to_string()
    }
}

impl FromStr for SsbhString8 {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<&str> for SsbhString8 {
    fn from(text: &str) -> Self {
        Self::from_bytes(text.to_string().into_bytes())
    }
}

impl From<String> for SsbhString8 {
    fn from(text: String) -> Self {
        Self::from_bytes(text.into_bytes())
    }
}

fn get_str(value: &NullString) -> Option<&str> {
    std::str::from_utf8(&value.0).ok()
}

/// A more performant type for parsing arrays of bytes that should always be preferred over `SsbhArray<u8>`.
#[cfg_attr(
    all(feature = "derive_serde", not(feature = "hex_buffer")),
    derive(Serialize, Deserialize)
)]
#[derive(Debug)]
pub struct SsbhByteBuffer {
    #[cfg_attr(
        all(feature = "derive_serde", not(feature = "hex_buffer")),
        serde(with = "serde_bytes")
    )]
    pub elements: Vec<u8>,
}

impl SsbhByteBuffer {
    pub fn new(elements: Vec<u8>) -> Self {
        Self { elements }
    }
}

impl From<Vec<u8>> for SsbhByteBuffer {
    fn from(v: Vec<u8>) -> Self {
        Self::new(v)
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

#[cfg(feature = "hex_buffer")]
struct SsbhByteBufferVisitor;

#[cfg(feature = "hex_buffer")]
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

#[cfg(feature = "hex_buffer")]
impl<'de> Deserialize<'de> for SsbhByteBuffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(SsbhByteBufferVisitor)
    }
}

#[cfg(feature = "hex_buffer")]
impl Serialize for SsbhByteBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.elements))
    }
}

/// A fixed-size collection of contiguous elements consisting of a relative offset to the array elements and an element count.
/**
```rust
use binread::BinRead;
use ssbh_lib::{SsbhArray, Matrix4x4};
use ssbh_lib::SsbhWrite;
# #[macro_use] extern crate ssbh_write_derive;
use ssbh_write_derive::SsbhWrite;

#[derive(BinRead, SsbhWrite)]
struct Transforms {
    array_relative_offset: u64,
    array_item_count: u64
}
# fn main() {}
```
 */
/// This can instead be expressed as the following struct with an explicit array item type.
/// The generated parsing and exporting code will correctly read and write the array data from the appropriate offset.
/**
```rust
use binread::BinRead;
use ssbh_lib::{SsbhArray, Matrix4x4, SsbhWrite};
# #[macro_use] extern crate ssbh_write_derive;
use ssbh_write_derive::SsbhWrite;

#[derive(BinRead, SsbhWrite)]
struct Transforms {
    data: SsbhArray<Matrix4x4>,
}
# fn main() {}
```
 */
#[derive(Debug)]
pub struct SsbhArray<T: BinRead> {
    pub elements: Vec<T>,
}

impl<T: BinRead> SsbhArray<T> {
    /// Creates a new array from `elements`.
    /**
    ```rust
    # use ssbh_lib::SsbhArray;
    let array = SsbhArray::new(vec![0, 1, 2]);
    assert_eq!(vec![0, 1, 2], array.elements);
    ```
    */
    pub fn new(elements: Vec<T>) -> Self {
        Self { elements }
    }
}

impl<T: BinRead> From<Vec<T>> for SsbhArray<T> {
    fn from(v: Vec<T>) -> Self {
        Self::new(v)
    }
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

#[cfg(feature = "derive_serde")]
struct SsbhArrayVisitor<T>
where
    T: BinRead,
{
    phantom: PhantomData<T>,
}

#[cfg(feature = "derive_serde")]
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
use ssbh_lib::SsbhWrite;
# #[macro_use] extern crate ssbh_write_derive;
use ssbh_write_derive::SsbhWrite;

#[derive(BinRead, SsbhWrite, Debug)]
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

# fn main() {}
```
 */
///
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(Debug, SsbhWrite)]
pub struct SsbhEnum64<T: BinRead<Args = (u64,)> + SsbhWrite> {
    pub data: RelPtr64<T>,
    pub data_type: u64,
}

impl<T> BinRead for SsbhEnum64<T>
where
    T: BinRead<Args = (u64,)> + SsbhWrite,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let pos_before_read = reader.stream_position()?;
        let relative_offset = u64::read_options(reader, options, ())?;
        let data_type = u64::read_options(reader, options, ())?;

        if relative_offset == 0 {
            return Ok(SsbhEnum64 {
                data: RelPtr64(None),
                data_type,
            });
        }

        let saved_pos = reader.stream_position()?;

        reader.seek(SeekFrom::Start(pos_before_read + relative_offset))?;
        let value = T::read_options(reader, options, (data_type,))?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(SsbhEnum64 {
            data: RelPtr64::new(value),
            data_type,
        })
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
#[derive(BinRead, Debug, PartialEq, SsbhWrite, Clone, Copy)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3 { x, y, z }
    }
}

impl From<[f32; 3]> for Vector3 {
    fn from(v: [f32; 3]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
        }
    }
}

/// A row-major 3x3 matrix of contiguous floats.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite, Clone, Copy)]
pub struct Matrix3x3 {
    pub row1: Vector3,
    pub row2: Vector3,
    pub row3: Vector3,
}

impl Matrix3x3 {
    /// The identity transformation matrix.
    ///
    /**
    ```rust
    use ssbh_lib::{Vector3, Matrix3x3};

    let m = Matrix3x3::identity();
    assert_eq!(Vector3::new(1f32, 0f32, 0f32), m.row1);
    assert_eq!(Vector3::new(0f32, 1f32, 0f32), m.row2);
    assert_eq!(Vector3::new(0f32, 0f32, 1f32), m.row3);
    ```
    */
    pub fn identity() -> Matrix3x3 {
        Matrix3x3 {
            row1: Vector3::new(1f32, 0f32, 0f32),
            row2: Vector3::new(0f32, 1f32, 0f32),
            row3: Vector3::new(0f32, 0f32, 1f32),
        }
    }

    /// Converts the elements to a 2d array in row-major order.
    /**
    ```rust
    use ssbh_lib::{Vector3, Matrix3x3};

    let m = Matrix3x3 {
        row1: Vector3::new(1f32, 2f32, 3f32),
        row2: Vector3::new(4f32, 5f32, 6f32),
        row3: Vector3::new(7f32, 8f32, 9f32),
    };

    assert_eq!(
        [
            [1f32, 2f32, 3f32],
            [4f32, 5f32, 6f32],
            [7f32, 8f32, 9f32],
        ],
        m.to_rows_array(),
    );
    ```
    */
    pub fn to_rows_array(&self) -> [[f32; 3]; 3] {
        [
            [self.row1.x, self.row1.y, self.row1.z],
            [self.row2.x, self.row2.y, self.row2.z],
            [self.row3.x, self.row3.y, self.row3.z],
        ]
    }

    /// Creates the matrix from a 2d array in row-major order.
    /**
    ```rust
    # use ssbh_lib::Matrix3x3;
    let elements = [
        [1f32, 2f32, 3f32],
        [4f32, 5f32, 6f32],
        [7f32, 8f32, 9f32],
    ];
    let m = Matrix3x3::from_rows_array(&elements);
    assert_eq!(elements, m.to_rows_array());
    ```
    */
    pub fn from_rows_array(rows: &[[f32; 3]; 3]) -> Matrix3x3 {
        Matrix3x3 {
            row1: rows[0].into(),
            row2: rows[1].into(),
            row3: rows[2].into(),
        }
    }
}

/// 4 contiguous floats for encoding XYZW or RGBA data.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite, Clone, Copy)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vector4 {
        Vector4 { x, y, z, w }
    }
}

impl From<[f32; 4]> for Vector4 {
    fn from(v: [f32; 4]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
            w: v[3],
        }
    }
}

/// 4 contiguous floats for encoding RGBA data.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
pub struct Color4f {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// A row-major 4x4 matrix of contiguous floats.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite)]
pub struct Matrix4x4 {
    pub row1: Vector4,
    pub row2: Vector4,
    pub row3: Vector4,
    pub row4: Vector4,
}

impl Matrix4x4 {
    /// The identity transformation matrix.
    ///
    /**
    ```rust
    use ssbh_lib::{Vector4, Matrix4x4};

    let m = Matrix4x4::identity();
    assert_eq!(Vector4::new(1f32, 0f32, 0f32, 0f32), m.row1);
    assert_eq!(Vector4::new(0f32, 1f32, 0f32, 0f32), m.row2);
    assert_eq!(Vector4::new(0f32, 0f32, 1f32, 0f32), m.row3);
    assert_eq!(Vector4::new(0f32, 0f32, 0f32, 1f32), m.row4);
    ```
    */
    pub fn identity() -> Matrix4x4 {
        Matrix4x4 {
            row1: Vector4::new(1f32, 0f32, 0f32, 0f32),
            row2: Vector4::new(0f32, 1f32, 0f32, 0f32),
            row3: Vector4::new(0f32, 0f32, 1f32, 0f32),
            row4: Vector4::new(0f32, 0f32, 0f32, 1f32),
        }
    }

    /// Converts the elements to a 2d array in row-major order.
    /**
    ```rust
    use ssbh_lib::{Vector4, Matrix4x4};

    let m = Matrix4x4 {
        row1: Vector4::new(1f32, 2f32, 3f32, 4f32),
        row2: Vector4::new(5f32, 6f32, 7f32, 8f32),
        row3: Vector4::new(9f32, 10f32, 11f32, 12f32),
        row4: Vector4::new(13f32, 14f32, 15f32, 16f32),
    };

    assert_eq!(
        [
            [1f32, 2f32, 3f32, 4f32],
            [5f32, 6f32, 7f32, 8f32],
            [9f32, 10f32, 11f32, 12f32],
            [13f32, 14f32, 15f32, 16f32],
        ],
        m.to_rows_array(),
    );
    ```
    */
    pub fn to_rows_array(&self) -> [[f32; 4]; 4] {
        [
            [self.row1.x, self.row1.y, self.row1.z, self.row1.w],
            [self.row2.x, self.row2.y, self.row2.z, self.row2.w],
            [self.row3.x, self.row3.y, self.row3.z, self.row3.w],
            [self.row4.x, self.row4.y, self.row4.z, self.row4.w],
        ]
    }

    /// Creates the matrix from a 2d array in row-major order.
    /**
    ```rust
    # use ssbh_lib::Matrix4x4;
    let elements = [
        [1f32, 2f32, 3f32, 4f32],
        [5f32, 6f32, 7f32, 8f32],
        [9f32, 10f32, 11f32, 12f32],
        [13f32, 14f32, 15f32, 16f32],
    ];
    let m = Matrix4x4::from_rows_array(&elements);
    assert_eq!(elements, m.to_rows_array());
    ```
    */
    pub fn from_rows_array(rows: &[[f32; 4]; 4]) -> Matrix4x4 {
        Matrix4x4 {
            row1: rows[0].into(),
            row2: rows[1].into(),
            row3: rows[2].into(),
            row4: rows[3].into(),
        }
    }
}

/// A wrapper type that serializes the value and absolute offset of the start of the value
/// to aid in debugging.
#[cfg(feature = "derive_serde")]
#[derive(Debug, Serialize, Deserialize, SsbhWrite)]
pub struct DebugPosition<T: BinRead<Args = ()> + Serialize + SsbhWrite> {
    val: T,
    pos: u64,
}

#[cfg(feature = "derive_serde")]
impl<T> BinRead for DebugPosition<T>
where
    T: BinRead<Args = ()> + Serialize + SsbhWrite,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let pos = reader.stream_position()?;
        let val = T::read_options(reader, options, ())?;
        Ok(Self { val, pos })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        // Remove any whitespace used to make the tests more readable.
        let no_whitespace: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(no_whitespace).unwrap()
    }

    #[test]
    fn new_ssbh_array() {
        let array = SsbhArray::new(vec![1, 2, 3]);
        assert_eq!(vec![1, 2, 3], array.elements);
    }

    #[test]
    fn new_ssbh_byte_buffer() {
        let array = SsbhByteBuffer::new(vec![1, 2, 3]);
        assert_eq!(vec![1, 2, 3], array.elements);
    }

    #[test]
    fn ssbh_byte_buffer_from_vec() {
        let array = SsbhByteBuffer::new(vec![1, 2, 3]);
        assert_eq!(vec![1, 2, 3], array.elements);
    }

    #[test]
    fn ssbh_array_from_vec() {
        let array: SsbhArray<_> = vec![1, 2, 3].into();
        assert_eq!(vec![1, 2, 3], array.elements);
    }

    #[test]
    fn new_relptr64() {
        let ptr = RelPtr64::new(5u32);
        assert_eq!(Some(5u32), ptr.0);
    }

    #[test]
    fn read_half() {
        let mut reader = Cursor::new(hex_bytes("003C00B4 00000000"));

        let value = reader.read_le::<Half>().unwrap();
        assert_eq!(1.0f32, value.into());

        let value = reader.read_le::<Half>().unwrap();
        assert_eq!(-0.25f32, value.into());

        let value = reader.read_le::<Half>().unwrap();
        assert_eq!(0.0f32, value.into());
    }

    #[test]
    fn read_relptr() {
        let mut reader = Cursor::new(hex_bytes("09000000 00000000 05070000"));
        let value = reader.read_le::<RelPtr64<u8>>().unwrap();
        assert_eq!(7u8, value.unwrap());

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(5u8, value);
    }

    #[test]
    fn read_null_relptr() {
        let mut reader = Cursor::new(hex_bytes("00000000 00000000 05070000"));
        let value = reader.read_le::<RelPtr64<u8>>().unwrap();
        assert_eq!(None, value.0);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(5u8, value);
    }

    #[test]
    fn read_ssbh_string() {
        let mut reader = Cursor::new(hex_bytes(
            "08000000 00000000 616C705F 6D617269 6F5F3030 325F636F 6C000000",
        ));
        let value = reader.read_le::<SsbhString>().unwrap();
        assert_eq!("alp_mario_002_col", value.to_str().unwrap());

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(0x61u8, value);
    }

    #[test]
    fn read_ssbh_string_empty() {
        let mut reader = Cursor::new(hex_bytes("08000000 00000000 00000000"));
        let value = reader.read_le::<SsbhString>().unwrap();
        assert_eq!("", value.to_str().unwrap());

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(0u8, value);
    }

    #[test]
    fn read_ssbh_array() {
        let mut reader = Cursor::new(hex_bytes(
            "12000000 00000000 03000000 00000000 01000200 03000400",
        ));
        let value = reader.read_le::<SsbhArray<u16>>().unwrap();
        assert_eq!(vec![2u16, 3u16, 4u16], value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_empty_ssbh_array() {
        let mut reader = Cursor::new(hex_bytes(
            "12000000 00000000 00000000 00000000 01000200 03000400",
        ));
        let value = reader.read_le::<SsbhArray<u16>>().unwrap();
        assert_eq!(Vec::<u16>::new(), value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_null_ssbh_array() {
        let mut reader = Cursor::new(hex_bytes(
            "00000000 00000000 00000000 00000000 01000200 03000400",
        ));
        let value = reader.read_le::<SsbhArray<u16>>().unwrap();
        assert_eq!(Vec::<u16>::new(), value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_byte_buffer() {
        let mut reader = Cursor::new(hex_bytes("11000000 00000000 03000000 00000000 01020304"));
        let value = reader.read_le::<SsbhByteBuffer>().unwrap();
        assert_eq!(vec![2u8, 3u8, 4u8], value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(1u8, value);
    }

    #[test]
    fn ssbh_string_from_str() {
        let s = SsbhString::from_str("abc").unwrap();
        assert_eq!("abc", s.to_str().unwrap());
    }

    #[test]
    fn ssbh_string8_from_str() {
        let s = SsbhString8::from_str("abc").unwrap();
        assert_eq!("abc", s.to_str().unwrap());
    }

    #[derive(BinRead, PartialEq, Debug, SsbhWrite)]
    #[br(import(data_type: u64))]
    pub enum TestData {
        #[br(pre_assert(data_type == 01u64))]
        Float(f32),
        #[br(pre_assert(data_type == 02u64))]
        Unsigned(u32),
    }

    #[test]
    fn read_ssbh_enum_float() {
        let mut reader = Cursor::new(hex_bytes("10000000 00000000 01000000 00000000 0000803F"));
        let value = reader.read_le::<SsbhEnum64<TestData>>().unwrap();
        assert_eq!(TestData::Float(1.0f32), value.data.0.unwrap());
        assert_eq!(1u64, value.data_type);

        // Make sure the reader position is restored.
        let value = reader.read_le::<f32>().unwrap();
        assert_eq!(1.0f32, value);
    }

    #[test]
    fn read_ssbh_enum_unsigned() {
        let mut reader = Cursor::new(hex_bytes("10000000 00000000 02000000 00000000 04000000"));
        let value = reader.read_le::<SsbhEnum64<TestData>>().unwrap();
        assert_eq!(TestData::Unsigned(4u32), value.data.0.unwrap());
        assert_eq!(2u64, value.data_type);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u32>().unwrap();
        assert_eq!(4u32, value);
    }

    #[test]
    fn read_vector3() {
        let mut reader = Cursor::new(hex_bytes("0000803F 000000C0 0000003F"));
        let value = reader.read_le::<Vector3>().unwrap();
        assert_eq!(1.0f32, value.x);
        assert_eq!(-2.0f32, value.y);
        assert_eq!(0.5f32, value.z);
    }

    #[test]
    fn read_vector4() {
        let mut reader = Cursor::new(hex_bytes("0000803F 000000C0 0000003F 0000803F"));
        let value = reader.read_le::<Vector4>().unwrap();
        assert_eq!(1.0f32, value.x);
        assert_eq!(-2.0f32, value.y);
        assert_eq!(0.5f32, value.z);
        assert_eq!(1.0f32, value.w);
    }

    #[test]
    fn read_color4f() {
        let mut reader = Cursor::new(hex_bytes("0000803E 0000003F 0000003E 0000803F"));
        let value = reader.read_le::<Vector4>().unwrap();
        assert_eq!(0.25f32, value.x);
        assert_eq!(0.5f32, value.y);
        assert_eq!(0.125f32, value.z);
        assert_eq!(1.0f32, value.w);
    }

    #[test]
    fn read_matrix4x4_identity() {
        let mut reader = Cursor::new(hex_bytes(
            "0000803F 00000000 00000000 00000000 
             00000000 0000803F 00000000 00000000 
             00000000 00000000 0000803F 00000000 
             00000000 00000000 00000000 0000803F",
        ));
        let value = reader.read_le::<Matrix4x4>().unwrap();
        assert_eq!(Vector4::new(1f32, 0f32, 0f32, 0f32), value.row1);
        assert_eq!(Vector4::new(0f32, 1f32, 0f32, 0f32), value.row2);
        assert_eq!(Vector4::new(0f32, 0f32, 1f32, 0f32), value.row3);
        assert_eq!(Vector4::new(0f32, 0f32, 0f32, 1f32), value.row4);
    }

    #[test]
    fn read_matrix3x3_identity() {
        let mut reader = Cursor::new(hex_bytes(
            "0000803F 00000000 00000000 
             00000000 0000803F 00000000 
             00000000 00000000 0000803F",
        ));
        let value = reader.read_le::<Matrix3x3>().unwrap();
        assert_eq!(Vector3::new(1f32, 0f32, 0f32), value.row1);
        assert_eq!(Vector3::new(0f32, 1f32, 0f32), value.row2);
        assert_eq!(Vector3::new(0f32, 0f32, 1f32), value.row3);
    }
}
