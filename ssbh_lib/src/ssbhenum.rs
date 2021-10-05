use binread::{BinRead, BinResult, ReadOptions};

use ssbh_write::SsbhWrite;
use std::io::{Read, Seek, SeekFrom};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{absolute_offset_checked, RelPtr64};

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
use ssbh_write::SsbhWrite;
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, SsbhWrite)]
pub struct SsbhEnum64<T: BinRead<Args = (u64,)> + crate::SsbhWrite> {
    pub data: RelPtr64<T>,
    pub data_type: u64,
}

impl<T> BinRead for SsbhEnum64<T>
where
    T: BinRead<Args = (u64,)> + crate::SsbhWrite,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        // The data type occurs after the offset, so it's difficult to just derive BinRead.
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

        let seek_pos = absolute_offset_checked(pos_before_read, relative_offset)?;
        reader.seek(SeekFrom::Start(seek_pos))?;
        let value = T::read_options(reader, options, (data_type,))?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(SsbhEnum64 {
            data: RelPtr64::new(value),
            data_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binread::BinReaderExt;
    use hexlit::hex;
    use std::io::Cursor;

    #[derive(BinRead, PartialEq, Debug, SsbhWrite)]
    #[br(import(data_type: u64))]
    pub enum TestData {
        #[br(pre_assert(data_type == 1u64))]
        Float(f32),
        #[br(pre_assert(data_type == 2u64))]
        Unsigned(u32),
    }

    #[test]
    fn read_ssbh_enum_float() {
        let mut reader = Cursor::new(hex!("10000000 00000000 01000000 00000000 0000803F"));
        let value = reader.read_le::<SsbhEnum64<TestData>>().unwrap();
        assert_eq!(TestData::Float(1.0f32), value.data.0.unwrap());
        assert_eq!(1u64, value.data_type);

        // Make sure the reader position is restored.
        let value = reader.read_le::<f32>().unwrap();
        assert_eq!(1.0f32, value);
    }

    #[test]
    fn read_ssbh_enum_unsigned() {
        let mut reader = Cursor::new(hex!("10000000 00000000 02000000 00000000 04000000"));
        let value = reader.read_le::<SsbhEnum64<TestData>>().unwrap();
        assert_eq!(TestData::Unsigned(4u32), value.data.0.unwrap());
        assert_eq!(2u64, value.data_type);
    }

    #[test]
    fn read_ssbh_enum_offset_overflow() {
        let mut reader = Cursor::new(hex!(
            "00000000 FFFFFFFF FFFFFFFF 02000000 00000000 04000000"
        ));
        reader.seek(SeekFrom::Start(4)).unwrap();

        // Make sure this just returns an error instead.
        let result = reader.read_le::<SsbhEnum64<TestData>>();
        assert!(matches!(
            result,
            Err(binread::error::Error::AssertFail { pos: 4, message })
            if message == format!(
                "Overflow occurred while computing relative offset {}",
                0xFFFFFFFFFFFFFFFFu64
            )
        ));

        // Make sure the reader position is restored.
        let value = reader.read_le::<u32>().unwrap();
        assert_eq!(4u32, value);
    }
}
