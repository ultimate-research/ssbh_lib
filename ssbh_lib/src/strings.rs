use crate::RelPtr64;
use binread::BinRead;
use ssbh_write::SsbhWrite;
use std::{io::Read, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// TODO: There seems to be a bug initializing null terminated strings from byte arrays not ignoring the nulls.
// It shouldn't be possible to initialize inline string or ssbh strings from non checked byte arrays directly.
// TODO: Does this write the null byte correctly?
/// A C string stored inline. This will likely be wrapped in a pointer type.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, SsbhWrite, PartialEq, Eq)]
pub struct InlineString(
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "serialize_str_bytes",
            deserialize_with = "deserialize_str_bytes"
        )
    )]
    Vec<u8>,
);

impl BinRead for InlineString {
    type Args = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        _: &binread::ReadOptions,
        _: Self::Args,
    ) -> binread::BinResult<Self> {
        let bytes: Vec<u8> = reader
            .bytes()
            .filter_map(|b| b.ok())
            .take_while(|b| *b != 0)
            .collect();

        Ok(Self::from_bytes(&bytes))
    }
}

impl InlineString {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.iter().copied().take_while(|b| *b != 0u8).collect())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn to_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for InlineString {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let bytes = Vec::<u8>::arbitrary(u)?;
        Ok(Self::from_bytes(&bytes))
    }
}

#[cfg(feature = "serde")]
fn serialize_str_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // TODO: This should check for null bytes?
    match InlineString::from_bytes(bytes).to_str() {
        Some(text) => serializer.serialize_str(text),
        None => serializer.serialize_none(),
    }
}

#[cfg(feature = "serde")]
fn deserialize_str_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;

    // TODO: This should check for null bytes?
    Ok(string.as_bytes().to_vec())
}

/// A 4-byte aligned [CString] with position determined by a relative offset.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SsbhString(RelPtr64<CString<4>>);

// TODO: Implement PartialEq for RelPtr64<T>?
impl PartialEq for SsbhString {
    fn eq(&self, other: &Self) -> bool {
        self.0 .0 == other.0 .0
    }
}

impl Eq for SsbhString {}

/// A null terminated string with a specified alignment.
/// The empty string is represented as `N` null bytes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, PartialEq, Eq)]
pub struct CString<const N: usize>(InlineString);

impl<const N: usize> CString<N> {
    /// Converts the underlying buffer to a [str].
    /// The result will be [None] if the the conversion failed.
    pub fn to_str(&self) -> Option<&str> {
        self.0.to_str()
    }

    /// Converts the underlying buffer to a [String].
    pub fn to_string_lossy(&self) -> String {
        self.to_str().unwrap_or("").to_string()
    }
}

impl<const N: usize> crate::SsbhWrite for CString<N> {
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        if self.0 .0.is_empty() {
            // Handle empty strings.
            writer.write_all(&[0u8; N])?;
        } else {
            // Write the data and null terminator.
            writer.write_all(&self.0.to_bytes())?;
            writer.write_all(&[0u8])?;
        }
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        self.0.size_in_bytes()
    }

    fn alignment_in_bytes() -> u64 {
        N as u64
    }
}

// TODO: Avoid redundant code?
impl<const N: usize> CString<N> {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(InlineString::from_bytes(bytes))
    }
}

impl<const N: usize> FromStr for CString<N> {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<const N: usize> From<&str> for CString<N> {
    fn from(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl<const N: usize> From<&String> for CString<N> {
    fn from(text: &String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl<const N: usize> From<String> for CString<N> {
    fn from(text: String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl SsbhString {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(RelPtr64::new(CString::<4>(InlineString::from_bytes(bytes))))
    }

    /// Converts the underlying buffer to a [str].
    /// The result will be [None] if the offset is null or the conversion failed.
    pub fn to_str(&self) -> Option<&str> {
        self.0.as_ref()?.to_str()
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
        Self::from_bytes(text.as_bytes())
    }
}

impl From<&String> for SsbhString {
    fn from(text: &String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl From<String> for SsbhString {
    fn from(text: String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

/// An 8-byte aligned [CString] with position determined by a relative offset.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[repr(transparent)]
pub struct SsbhString8(RelPtr64<CString<8>>);

impl SsbhString8 {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(RelPtr64::new(CString::<8>(InlineString::from_bytes(bytes))))
    }

    /// Converts the underlying buffer to a [str].
    /// The result will be [None] if the offset is null or the conversion failed.
    pub fn to_str(&self) -> Option<&str> {
        self.0.as_ref()?.to_str()
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
        Self::from_bytes(text.as_bytes())
    }
}

impl From<&String> for SsbhString8 {
    fn from(text: &String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl From<String> for SsbhString8 {
    fn from(text: String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use binread::BinReaderExt;
    use std::io::Cursor;

    use hexlit::hex;

    use super::*;

    #[test]
    fn read_ssbh_string() {
        let mut reader = Cursor::new(hex!(
            "08000000 00000000 616C705F 6D617269 6F5F3030 325F636F 6C000000"
        ));
        let value = reader.read_le::<SsbhString>().unwrap();
        assert_eq!("alp_mario_002_col", value.to_str().unwrap());

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(0x61u8, value);
    }

    #[test]
    fn read_ssbh_string_empty() {
        let mut reader = Cursor::new(hex!("08000000 00000000 00000000"));
        let value = reader.read_le::<SsbhString>().unwrap();
        assert_eq!("", value.to_str().unwrap());

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(0u8, value);
    }

    #[test]
    fn cstring_to_string_conversion() {
        assert_eq!(Some("abc"), CString::<4>(InlineString::from_bytes(b"abc\0")).to_str());
        assert_eq!("abc".to_string(), CString::<4>(InlineString::from_bytes(b"abc\0")).to_string_lossy());
    }

    #[test]
    fn ssbh_string_to_string_conversion() {
        assert_eq!(Some("abc"), SsbhString::from("abc").to_str());
        assert_eq!("abc".to_string(), SsbhString::from("abc").to_string_lossy());
    }

    #[test]
    fn ssbh_string8_to_string_conversion() {
        assert_eq!(Some("abc"), SsbhString8::from("abc").to_str());
        assert_eq!("abc".to_string(), SsbhString8::from("abc").to_string_lossy());
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

    #[test]
    fn ssbh_write_string() {
        let value = SsbhString::from("scouter1Shape");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("08000000 00000000 73636F75 74657231 53686170 6500")
        );
        // The data pointer should be aligned to 4.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn ssbh_write_string_empty() {
        let value = SsbhString::from("");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(writer.into_inner(), hex!("08000000 00000000 00000000"));
        // The data pointer should be aligned to 4.
        assert_eq!(12, data_ptr);
    }

    #[test]
    fn ssbh_write_string_non_zero_data_ptr() {
        let value = SsbhString::from("scouter1Shape");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 5;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("08000000 00000000 73636F75 74657231 53686170 6500")
        );
        // The data pointer should be aligned to 4.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn ssbh_write_string_tuple() {
        #[derive(SsbhWrite)]
        struct StringPair {
            item1: SsbhString,
            item2: SsbhString,
        }

        // NRPD data.
        let value = StringPair {
            item1: SsbhString::from("RTV_FRAME_BUFFER_COPY"),
            item2: SsbhString::from("FB_FRAME_BUFFER_COPY"),
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Check that the pointers don't overlap.
        assert_eq!(
            writer.into_inner(),
            hex!(
                "10000000 00000000 20000000 00000000 
                 5254565F 4652414D 455F4255 46464552 
                 5F434F50 59000000 46425F46 52414D45 
                 5F425546 4645525F 434F5059 00"
            )
        );
    }

    #[test]
    fn ssbh_write_string8() {
        let value = SsbhString8::from("BlendState0");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("08000000 00000000 426C656E 64537461 74653000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(24, data_ptr);
    }

    #[test]
    fn ssbh_write_string8_empty() {
        let value = SsbhString8::from("");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("08000000 00000000 00000000 00000000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn ssbh_write_string8_non_zero_data_ptr() {
        let value = SsbhString8::from("BlendState0");

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 5;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("08000000 00000000 426C656E 64537461 74653000")
        );
        // The data pointer should be aligned to 8.
        assert_eq!(24, data_ptr);
    }
}
