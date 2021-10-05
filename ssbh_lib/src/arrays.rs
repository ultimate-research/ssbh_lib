use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};

#[cfg(feature = "serde")]
use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::SerializeSeq,
};

#[cfg(feature = "serde")]
use std::{fmt, marker::PhantomData};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize, Serializer};

use crate::absolute_offset_checked;

// Array element types vary in size, so pick a more consersative value.
const SSBH_ARRAY_MAX_INITIAL_CAPACITY: usize = 1024;

// Limit byte buffers to a max initial allocation of 100 MB.
// This is significantly larger than the largest vertex buffer for Smash Ultimate (< 20 MB).
const SSBH_BYTE_BUFFER_MAX_INITIAL_CAPACITY: usize = 104857600;

/// A more performant type for parsing arrays of bytes that should always be preferred over `SsbhArray<u8>`.
#[cfg_attr(
    all(feature = "serde", not(feature = "hex_buffer")),
    derive(Serialize, Deserialize)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub struct SsbhByteBuffer {
    #[cfg_attr(
        all(feature = "serde", not(feature = "hex_buffer")),
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
use ssbh_write::SsbhWrite;
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
use ssbh_lib::{SsbhArray, Matrix4x4};
use ssbh_write::SsbhWrite;

#[derive(BinRead, SsbhWrite)]
struct Transforms {
    data: SsbhArray<Matrix4x4>,
}
# fn main() {}
```
 */
#[derive(Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct SsbhArray<T: BinRead> {
    pub elements: Vec<T>,
}

// TODO: derive_more to automate this?
impl<T: BinRead + PartialEq> PartialEq for SsbhArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl<T: BinRead + Eq> Eq for SsbhArray<T> {}

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

#[cfg(feature = "serde")]
struct SsbhArrayVisitor<T>
where
    T: BinRead,
{
    phantom: PhantomData<T>,
}

#[cfg(feature = "serde")]
impl<T: BinRead> SsbhArrayVisitor<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
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

#[cfg(feature = "serde")]
impl<'de, T: BinRead + Deserialize<'de>> Deserialize<'de> for SsbhArray<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(SsbhArrayVisitor::new())
    }
}

#[cfg(feature = "serde")]
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

fn read_elements<C: Copy + 'static, BR: BinRead<Args = C>, R: Read + Seek>(
    reader: &mut R,
    options: &ReadOptions,
    count: u64,
    args: C,
) -> BinResult<Vec<BR>> {
    // Reduce the risk of failed allocations due to malformed array lengths (ex: -1 in two's complement).
    // This only bounds the initial capacity, so large elements can still resize the vector as needed.
    // This won't impact performance or memory usage for array lengths within the bound.
    let mut elements = Vec::with_capacity(std::cmp::min(
        count as usize,
        SSBH_ARRAY_MAX_INITIAL_CAPACITY,
    ));
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
    // Reduce the risk of failed allocations due to malformed array lengths (ex: -1 in two's complement).
    // Similar to SsbhArray, this won't impact performance for lengths within the initial capacity.
    let mut elements = Vec::with_capacity(std::cmp::min(
        count as usize,
        SSBH_BYTE_BUFFER_MAX_INITIAL_CAPACITY,
    ));
    let bytes_read = reader.take(count).read_to_end(&mut elements)?;
    if bytes_read != count as usize {
        Err(binread::error::Error::AssertFail {
            pos: reader.stream_position()?,
            message: format!(
                "Failed to read entire buffer. Expected {} bytes but found {} bytes.",
                count, bytes_read
            ),
        })
    } else {
        Ok(elements)
    }
}

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
    // The length occurs after the offset, so it's difficult to just derive BinRead.
    let pos_before_read = reader.stream_position()?;

    let relative_offset = u64::read_options(reader, options, ())?;
    let element_count = u64::read_options(reader, options, ())?;

    let saved_pos = reader.stream_position()?;

    let seek_pos = absolute_offset_checked(pos_before_read, relative_offset)?;
    reader.seek(SeekFrom::Start(seek_pos))?;
    let result = read_elements(reader, options, element_count, args);
    reader.seek(SeekFrom::Start(saved_pos))?;

    result
}

#[cfg(test)]
mod tests {
    use binread::BinReaderExt;
    use std::io::Cursor;

    use hex_literal::hex;

    use super::*;

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
    fn read_ssbh_array() {
        let mut reader = Cursor::new(hex!(
            "12000000 00000000 03000000 00000000 01000200 03000400"
        ));
        let value = reader.read_le::<SsbhArray<u16>>().unwrap();
        assert_eq!(vec![2u16, 3u16, 4u16], value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_array_empty() {
        let mut reader = Cursor::new(hex!(
            "12000000 00000000 00000000 00000000 01000200 03000400"
        ));
        let value = reader.read_le::<SsbhArray<u16>>().unwrap();
        assert_eq!(Vec::<u16>::new(), value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_array_null() {
        let mut reader = Cursor::new(hex!(
            "00000000 00000000 00000000 00000000 01000200 03000400"
        ));
        let value = reader.read_le::<SsbhArray<u16>>().unwrap();
        assert_eq!(Vec::<u16>::new(), value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_array_offset_overflow() {
        let mut reader = Cursor::new(hex!(
            "00000000 FFFFFFFF FFFFFFFF 03000000 00000000 01000200 03000400"
        ));
        reader.seek(SeekFrom::Start(4)).unwrap();

        // Make sure this just returns an error instead.
        let result = reader.read_le::<SsbhArray<u16>>();
        assert!(matches!(
            result,
            Err(binread::error::Error::AssertFail { pos: 4, message })
            if message == format!(
                "Overflow occurred while computing relative offset {}",
                0xFFFFFFFFFFFFFFFFu64
            )
        ));

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_array_extreme_allocation_size() {
        // Attempting to allocate usize::MAX elements will almost certainly panic.
        let mut reader = Cursor::new(hex!(
            "10000000 00000000 FFFFFFFF FFFFFFFF 01000200 03000400"
        ));

        // Make sure this just returns an error instead.
        // TODO: Check the actual error?
        let value = reader.read_le::<SsbhArray<u16>>();
        assert!(value.is_err());

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_byte_buffer() {
        let mut reader = Cursor::new(hex!("11000000 00000000 03000000 00000000 01020304"));
        let value = reader.read_le::<SsbhByteBuffer>().unwrap();
        assert_eq!(vec![2u8, 3u8, 4u8], value.elements);

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(1u8, value);
    }

    #[test]
    fn read_ssbh_byte_buffer_offset_overflow() {
        let mut reader = Cursor::new(hex!(
            "00000000 FFFFFFFF FFFFFFFF 03000000 00000000 01000200 03000400"
        ));
        reader.seek(SeekFrom::Start(4)).unwrap();

        // Make sure this just returns an error instead.
        let result = reader.read_le::<SsbhByteBuffer>();
        assert!(matches!(
            result,
            Err(binread::error::Error::AssertFail { pos: 4, message })
            if message == format!(
                "Overflow occurred while computing relative offset {}",
                0xFFFFFFFFFFFFFFFFu64
            )
        ));

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn read_ssbh_byte_buffer_not_enough_bytes() {
        // Attempting to allocate usize::MAX bytes will almost certainly panic.
        let mut reader = Cursor::new(hex!("10000000 00000000 05000000 00000000 01020304"));

        // Make sure this just returns an error instead.
        match reader.read_le::<SsbhByteBuffer>() {
            Err(binread::error::Error::AssertFail { pos, message }) => {
                assert_eq!(20, pos);
                assert_eq!(
                    format!(
                        "Failed to read entire buffer. Expected {} bytes but found {} bytes.",
                        5, 4
                    ),
                    message
                );
            }
            _ => panic!("Unexpected variant"),
        }

        // Make sure the reader position is restored.
        let value = reader.read_le::<u8>().unwrap();
        assert_eq!(1u8, value);
    }

    #[test]
    fn read_ssbh_byte_buffer_extreme_allocation_size() {
        // Attempting to allocate usize::MAX bytes will almost certainly panic.
        let mut reader = Cursor::new(hex!(
            "10000000 00000000 FFFFFFFF FFFFFFFF 01000200 03000400"
        ));

        // Make sure this just returns an error instead.
        let result = reader.read_le::<SsbhByteBuffer>();
        assert!(matches!(
            result,
            Err(binread::error::Error::AssertFail { pos: 24, message }) 
            if message == format!(
                "Failed to read entire buffer. Expected {} bytes but found {} bytes.",
                0xFFFFFFFFFFFFFFFFu64, 8)));

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }
}
