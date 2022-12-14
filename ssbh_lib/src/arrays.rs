use binrw::io::Write;

use binrw::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

use crate::{absolute_offset_checked, round_up, write_relative_offset};

// Array element types vary in size, so pick a more consersative value.
const SSBH_ARRAY_MAX_INITIAL_CAPACITY: usize = 1024;

// Limit byte buffers to a max initial allocation of 100 MB.
// This is significantly larger than the largest vertex buffer for Smash Ultimate (< 20 MB).
const SSBH_BYTE_BUFFER_MAX_INITIAL_CAPACITY: usize = 104857600;

/// A more performant type for parsing arrays of bytes that should always be preferred over `SsbhArray<u8>`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SsbhByteBuffer {
    #[cfg_attr(
        all(feature = "serde", not(feature = "serde_hex")),
        serde(with = "serde_bytes")
    )]
    #[cfg_attr(
        feature = "serde_hex",
        serde(serialize_with = "serialize_hex", deserialize_with = "deserialize_hex",)
    )]
    pub elements: Vec<u8>,
}

impl SsbhByteBuffer {
    /// Creates an empty array.
    /**
    ```rust
    # use ssbh_lib::SsbhByteBuffer;
    let array = SsbhByteBuffer::new();
    assert!(array.elements.is_empty());
    ```
    */
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Creates a new array from `elements`.
    /**
    ```rust
    # use ssbh_lib::SsbhArray;
    let array = SsbhArray::from_vec(vec![0, 1, 2]);
    assert_eq!(vec![0, 1, 2], array.elements);
    ```
    */
    pub fn from_vec(elements: Vec<u8>) -> Self {
        Self { elements }
    }
}

impl Default for SsbhByteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<u8>> for SsbhByteBuffer {
    fn from(v: Vec<u8>) -> Self {
        Self::from_vec(v)
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

#[cfg(feature = "serde_hex")]
fn deserialize_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    hex::decode(hex).map_err(serde::de::Error::custom)
}

#[cfg(feature = "serde_hex")]
fn serialize_hex<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(bytes))
}

/// A fixed-size collection of contiguous elements consisting of a relative offset to the array elements and an element count.
/**
```rust
use binrw::BinRead;
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
use binrw::BinRead;
use ssbh_lib::{SsbhArray, Matrix4x4};
use ssbh_write::SsbhWrite;

#[derive(BinRead, SsbhWrite)]
struct Transforms {
    data: SsbhArray<Matrix4x4>,
}
# fn main() {}
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub struct SsbhArray<T> {
    pub elements: Vec<T>,
}

impl<T> Default for SsbhArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for SsbhArray<T> {
    fn clone(&self) -> Self {
        Self::from_vec(self.elements.clone())
    }
}

// TODO: derive_more to automate this?
impl<T: PartialEq> PartialEq for SsbhArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl<T: Eq> Eq for SsbhArray<T> {}

impl<T> SsbhArray<T> {
    /// Creates an empty array.
    /**
    ```rust
    # use ssbh_lib::SsbhArray;
    let array: SsbhArray<u32> = SsbhArray::new();
    assert!(array.elements.is_empty());
    ```
    */
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Creates a new array from `elements`.
    /**
    ```rust
    # use ssbh_lib::SsbhArray;
    let array = SsbhArray::from_vec(vec![0, 1, 2]);
    assert_eq!(vec![0, 1, 2], array.elements);
    ```
    */
    pub fn from_vec(elements: Vec<T>) -> Self {
        Self { elements }
    }
}

impl<T> From<Vec<T>> for SsbhArray<T> {
    fn from(v: Vec<T>) -> Self {
        Self::from_vec(v)
    }
}

impl<T> FromIterator<T> for SsbhArray<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            elements: iter.into_iter().collect::<Vec<_>>(),
        }
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

fn read_elements<C: Copy + 'static, BR: BinRead<Args = C>, R: Read + Seek>(
    reader: &mut R,
    options: &ReadOptions,
    count: u64,
    args: C,
) -> BinResult<Vec<BR>> {
    // Reduce the risk of failed allocations due to malformed array lengths (ex: -1 in two's complement).
    // This only bounds the initial capacity, so large elements can still resize the vector as needed.
    // This won't impact performance or memory usage for array lengths within the bound.
    // TODO: Use try_reserve from rust 1.57+ for added stability on windows?
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
        Err(binrw::error::Error::AssertFail {
            pos: reader.stream_position()?,
            message: format!(
                "Failed to read entire buffer. Expected {count} bytes but found {bytes_read} bytes."
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

fn write_array_header<W: Write + Seek>(
    writer: &mut W,
    data_ptr: &mut u64,
    count: usize,
) -> std::io::Result<()> {
    // Arrays are always 8 byte aligned.
    *data_ptr = round_up(*data_ptr, 8);

    // Don't write the offset for empty arrays.
    if count == 0 {
        u64::write(&0u64, writer)?;
    } else {
        write_relative_offset(writer, data_ptr)?;
    }

    (count as u64).write(writer)?;
    Ok(())
}

impl SsbhWrite for SsbhByteBuffer {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        let current_pos = writer.stream_position()?;
        if *data_ptr < current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        write_array_header(writer, data_ptr, self.elements.len())?;

        let current_pos = writer.stream_position()?;
        writer.seek(SeekFrom::Start(*data_ptr))?;
        // Use a custom implementation to avoid writing bytes individually.
        // Pointers in array elements should point past the end of the array.
        writer.write_all(&self.elements)?;
        *data_ptr += self.elements.len() as u64;

        writer.seek(SeekFrom::Start(current_pos))?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        16
    }
}

impl<T: SsbhWrite> SsbhWrite for SsbhArray<T> {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // TODO: Create a macro or function for this?
        let current_pos = writer.stream_position()?;
        if *data_ptr < current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        write_array_header(writer, data_ptr, self.elements.len())?;

        let pos_after_length = writer.stream_position()?;
        writer.seek(SeekFrom::Start(*data_ptr))?;

        self.elements.as_slice().ssbh_write(writer, data_ptr)?;

        writer.seek(SeekFrom::Start(pos_after_length))?;

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // A 64 bit relative offset and 64 bit length
        16
    }

    fn alignment_in_bytes() -> u64 {
        // Arrays are always 8 byte aligned.
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::SsbhString;
    use binrw::io::Cursor;
    use binrw::BinReaderExt;
    use hexlit::hex;

    #[test]
    fn ssbh_array_from_vec() {
        let array: SsbhArray<_> = vec![1, 2, 3].into();
        assert_eq!(vec![1, 2, 3], array.elements);
    }

    #[test]
    fn ssbh_array_from_iterator() {
        let array: SsbhArray<_> = [1, 2, 3].into_iter().collect();
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
    #[ignore]
    fn read_ssbh_array_null_nonzero_count() {
        // TODO: How would in game parsers handle this case?
        let mut reader = Cursor::new(hex!(
            "00000000 00000000 03000000 00000000 01000200 03000400"
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
            Err(binrw::error::Error::AssertFail { pos: 4, message })
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
            Err(binrw::error::Error::AssertFail { pos: 4, message })
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
            Err(binrw::error::Error::AssertFail { pos, message }) => {
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
            Err(binrw::error::Error::AssertFail { pos: 24, message }) 
            if message == format!(
                "Failed to read entire buffer. Expected {} bytes but found {} bytes.",
                0xFFFFFFFFFFFFFFFFu64, 8)));

        // Make sure the reader position is restored.
        let value = reader.read_le::<u16>().unwrap();
        assert_eq!(1u16, value);
    }

    #[test]
    fn ssbh_write_array_ssbh_string() {
        let value = SsbhArray::from_vec(vec![
            SsbhString::from("leyes_eye_mario_l_col"),
            SsbhString::from("eye_mario_w_nor"),
        ]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Check that the relative offsets point past the array.
        // Check that string data is aligned to 4.
        assert_eq!(
            writer.into_inner(),
            hex!(
                "10000000 00000000 02000000 00000000
                 10000000 00000000 20000000 00000000
                 6C657965 735F6579 655F6D61 72696F5F 
                 6C5F636F 6C000000 6579655F 6D617269 
                 6F5F775F 6E6F7200"
            )
        );
    }

    #[test]
    fn write_empty_array() {
        let value = SsbhArray::<u32>::from_vec(Vec::new());

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Null and empty arrays seem to use 0 offset and 0 length.
        assert_eq!(
            writer.into_inner(),
            hex!("00000000 00000000 00000000 00000000")
        );
        assert_eq!(16, data_ptr);
    }

    #[test]
    fn write_byte_buffer() {
        let value = SsbhByteBuffer::from_vec(vec![1u8, 2u8, 3u8, 4u8, 5u8]);

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("10000000 00000000 05000000 00000000 01020304 05")
        );
        assert_eq!(21, data_ptr);
    }

    #[test]
    fn write_vec() {
        let value = vec![1u8, 2u8, 3u8, 4u8, 5u8];

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(writer.into_inner(), hex!("01020304 05"));
        assert_eq!(5, data_ptr);
    }

    #[test]
    fn write_empty_byte_buffer() {
        let value = SsbhByteBuffer::from_vec(Vec::new());

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        // Null and empty arrays seem to use 0 offset and 0 length.
        assert_eq!(
            writer.into_inner(),
            hex!("00000000 00000000 00000000 00000000")
        );
        assert_eq!(16, data_ptr);
    }
}
