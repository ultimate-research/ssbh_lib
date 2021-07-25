use binread::{BinRead, NullString};

use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::de::{Error, Visitor};

#[cfg(feature = "derive_serde")]
use std::fmt;

use std::{convert::TryInto, num::NonZeroU8, str::FromStr};

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize, Serializer};

use crate::RelPtr64;

// TODO: There seems to be a bug initializing null terminated strings from byte arrays not ignoring the nulls.
// It shouldn't be possible to initialize inline string or ssbh strings from non checked byte arrays directly.
// TODO: Does this write the null byte correctly?
/// A C string stored inline. This will likely be wrapped in a pointer type.
#[derive(BinRead, Debug, SsbhWrite)]
pub struct InlineString(NullString);

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for InlineString {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let bytes = Vec::<NonZeroU8>::arbitrary(u)?;
        // let bytes = Vec::<u8>::arbitrary(u)?;

        Ok(Self(NullString(
            bytes.iter().map(|x| u8::from(*x)).collect(),
        )))
    }
}

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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SsbhString(RelPtr64<CString<4>>);

/// A null terminated string with a specified alignment.
/// The empty string is represented as `N` null bytes.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug)]
pub struct CString<const N: usize>(InlineString);

impl<const N: usize> crate::SsbhWrite for CString<N> {
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        if self.0 .0.len() == 0 {
            // Handle empty strings.
            writer.write_all(&[0u8; N])?;
        } else {
            // Write the data and null terminator.
            writer.write_all(&self.0 .0)?;
            writer.write_all(&[0u8])?;
        }
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        self.0.size_in_bytes()
    }

    fn alignment_in_bytes(&self) -> u64 {
        N as u64
    }
}

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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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

#[cfg(test)]
mod tests {
    use binread::BinReaderExt;
    use std::io::Cursor;

    use crate::hex_bytes;

    use super::*;

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
    fn ssbh_string_from_str() {
        let s = SsbhString::from_str("abc").unwrap();
        assert_eq!("abc", s.to_str().unwrap());
    }

    #[test]
    fn ssbh_string8_from_str() {
        let s = SsbhString8::from_str("abc").unwrap();
        assert_eq!("abc", s.to_str().unwrap());
    }
}
