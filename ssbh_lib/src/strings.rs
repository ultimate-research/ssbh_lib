use crate::RelPtr64;
use binrw::BinRead;
use ssbh_write::SsbhWrite;
use std::{io::Read, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An N-byte aligned [CString] with position determined by a relative offset.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq, Eq, Clone)]
pub struct SsbhStringN<const N: usize>(RelPtr64<CString<N>>);

/// A 4-byte aligned [CString] with position determined by a relative offset.
pub type SsbhString = SsbhStringN<4>;

/// An 8-byte aligned [CString] with position determined by a relative offset.
pub type SsbhString8 = SsbhStringN<8>;

/// A null terminated string without additional alignment requirements.
pub type CString1 = CString<1>;

/// A null terminated string with a specified alignment.
/// The empty string is represented as `N` null bytes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CString<const N: usize>(
    // Don't make this public to prevent inserting null bytes.
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "serialize_str_bytes",
            deserialize_with = "deserialize_str_bytes"
        )
    )]
    Vec<u8>,
);

impl<const N: usize> CString<N> {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.iter().copied().take_while(|b| *b != 0u8).collect())
    }

    /// Converts the underlying buffer to a [str].
    /// The result will be [None] if the the conversion failed.
    pub fn to_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }

    /// Converts the underlying buffer to a [String].
    pub fn to_string_lossy(&self) -> String {
        self.to_str().unwrap_or("").to_string()
    }
}

impl<const N: usize> BinRead for CString<N> {
    type Args = ();

    fn read_options<R: Read + std::io::Seek>(
        reader: &mut R,
        _options: &binrw::ReadOptions,
        _args: Self::Args,
    ) -> binrw::BinResult<Self> {
        // TODO: Does this correctly handle eof?
        let bytes: Vec<u8> = reader
            .bytes()
            .filter_map(|b| b.ok())
            .take_while(|b| *b != 0)
            .collect();

        Ok(Self(bytes))
    }
}

#[cfg(feature = "serde")]
fn serialize_str_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let text = CString::<1>::from_bytes(bytes).to_string_lossy();
    serializer.serialize_str(&text)
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

#[cfg(feature = "arbitrary")]
impl<'a, const N: usize> arbitrary::Arbitrary<'a> for CString<N> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let bytes = Vec::<u8>::arbitrary(u)?;
        Ok(Self::from_bytes(&bytes))
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

impl<const N: usize> crate::SsbhWrite for CString<N> {
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        if self.0.is_empty() {
            // Handle empty strings.
            writer.write_all(&[0u8; N])?;
        } else {
            // Write the data and null terminator.
            writer.write_all(&self.0)?;
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

impl<const N: usize> SsbhStringN<N> {
    /// Creates the string by reading from `bytes` until the first null byte.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(RelPtr64::new(CString::from_bytes(bytes)))
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

impl<const N: usize> FromStr for SsbhStringN<N> {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<const N: usize> From<&str> for SsbhStringN<N> {
    fn from(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl<const N: usize> From<&String> for SsbhStringN<N> {
    fn from(text: &String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

impl<const N: usize> From<String> for SsbhStringN<N> {
    fn from(text: String) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;
    use binrw::BinReaderExt;

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
        assert_eq!(Some("abc"), CString::<4>::from_bytes(b"abc\0").to_str());
        assert_eq!(
            "abc".to_string(),
            CString::<4>::from_bytes(b"abc\0").to_string_lossy()
        );
    }

    #[test]
    fn ssbh_string_to_string_conversion() {
        assert_eq!(Some("abc"), SsbhString::from("abc").to_str());
        assert_eq!("abc".to_string(), SsbhString::from("abc").to_string_lossy());
    }

    #[test]
    fn ssbh_string8_to_string_conversion() {
        assert_eq!(Some("abc"), SsbhString8::from("abc").to_str());
        assert_eq!(
            "abc".to_string(),
            SsbhString8::from("abc").to_string_lossy()
        );
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
