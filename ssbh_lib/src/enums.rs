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
# use binread::BinRead;
# use ssbh_write::SsbhWrite;
#[derive(Debug, BinRead, SsbhWrite)]
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
# use binread::BinRead;
# use ssbh_lib::SsbhEnum64;
# use ssbh_write::SsbhWrite;
#[derive(Debug, BinRead, SsbhWrite)]
#[br(import(data_type: u64))]
pub enum Data {
    #[br(pre_assert(data_type == 1u64))]
    Float(f32),
    #[br(pre_assert(data_type == 2u64))]
    Boolean(u32),
}

impl ssbh_lib::DataType for Data {
    fn data_type(&self) -> u64 {
        match self {
            Data::Float(_) => 1,
            Data::Boolean(_) => 2
        }
    }
}

#[derive(Debug, BinRead, SsbhWrite)]
pub struct EnumData {
    data: SsbhEnum64<Data>,
}

# fn main() {}
```
 */
///
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub struct SsbhEnum64<T: DataType> {
    pub data: RelPtr64<T>,
}

// TODO: Find a way to avoid specifying variants for BinRead and for this trait.
pub trait DataType {
    fn data_type(&self) -> u64;
}

impl<T: DataType + PartialEq> PartialEq for SsbhEnum64<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T> BinRead for SsbhEnum64<T>
where
    T: DataType + BinRead<Args = (u64,)> + crate::SsbhWrite,
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
            });
        }

        let saved_pos = reader.stream_position()?;

        let seek_pos = absolute_offset_checked(pos_before_read, relative_offset)?;
        reader.seek(SeekFrom::Start(seek_pos))?;
        let value = T::read_options(reader, options, (data_type,))?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(SsbhEnum64 {
            data: RelPtr64::new(value),
        })
    }
}

impl<T: DataType + SsbhWrite> SsbhWrite for SsbhEnum64<T> {
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // Ensure the next pointer won't point inside this struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr < current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }
        // Write all the fields.
        self.data.ssbh_write(writer, data_ptr)?;
        // TODO: How to handle null?
        self.data
            .as_ref()
            .map(DataType::data_type)
            .unwrap_or(0)
            .ssbh_write(writer, data_ptr)?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // Relative offset + data type
        8 + 8
    }
}

// Use a macro to avoid specifying the data type in multiple places for variants.
macro_rules! ssbh_enum {
    ($(#[$attr1:meta])* $name:ident, $($(#[$attr2:meta])* $tag:literal => $variant:ident($body:tt)),*) => {
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
        #[derive(Debug, BinRead, SsbhWrite, PartialEq)]
        #[br(import(data_type: u64))]
        $(#[$attr1])*
        pub enum $name {
            $(
                $(#[$attr2])*
                #[br(pre_assert(data_type == $tag))]
                $variant($body)
            ),*
        }

        impl crate::DataType for $name {
            fn data_type(&self) -> u64 {
                match self {
                    $(
                        Self::$variant(_) => $tag
                    ),*
                }
            }

        }
    };
}

pub(crate) use ssbh_enum;

#[cfg(test)]
mod tests {
    use super::*;
    use binread::BinReaderExt;
    use hexlit::hex;
    use std::io::Cursor;

    ssbh_enum!(
        /// Enum comment.
        TestData,
        1 => Float(f32),
        /// Variants can have comments.
        2 => Unsigned(u32)
    );

    #[test]
    fn read_ssbh_enum_float() {
        let mut reader = Cursor::new(hex!("10000000 00000000 01000000 00000000 0000803F"));
        let value = reader.read_le::<SsbhEnum64<TestData>>().unwrap();
        assert_eq!(TestData::Float(1.0f32), value.data.0.unwrap());

        // Make sure the reader position is restored.
        let value = reader.read_le::<f32>().unwrap();
        assert_eq!(1.0f32, value);
    }

    #[test]
    fn read_ssbh_enum_unsigned() {
        let mut reader = Cursor::new(hex!("10000000 00000000 02000000 00000000 04000000"));
        let value = reader.read_le::<SsbhEnum64<TestData>>().unwrap();
        assert_eq!(TestData::Unsigned(4u32), value.data.0.unwrap());
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

    #[test]
    fn ssbh_write_enum_float() {
        let value = SsbhEnum64::<TestData> {
            data: RelPtr64::new(TestData::Float(1.0f32)),
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("10000000 00000000 01000000 00000000 0000803F")
        );
    }

    #[test]
    fn ssbh_write_enum_unsigned() {
        let value = SsbhEnum64::<TestData> {
            data: RelPtr64::new(TestData::Unsigned(5u32)),
        };

        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        value.ssbh_write(&mut writer, &mut data_ptr).unwrap();

        assert_eq!(
            writer.into_inner(),
            hex!("10000000 00000000 02000000 00000000 05000000")
        );
    }
}
