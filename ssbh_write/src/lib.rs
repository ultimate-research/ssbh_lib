use std::{
    io::{Seek, Write},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU128, NonZeroU16,
        NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
};

pub use ssbh_write_derive::SsbhWrite;

// TODO: Document what a sample implementation would look like.

/// A trait for writing types that are part of SSBH formats.
pub trait SsbhWrite: Sized {
    /// Writes the byte representation of `self` to `writer` and updates `data_ptr` as needed to ensure the next relative offset is correctly calculated.
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()>;

    /// Writes the byte representation of `self` to `writer`.
    /// This is a convenience method for [ssbh_write](crate::SsbhWrite::ssbh_write) that handles initializing the data pointer.
    fn write<W: std::io::Write + std::io::Seek>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut data_ptr = 0;
        self.ssbh_write(writer, &mut data_ptr)?;
        Ok(())
    }

    /// The offset in bytes between successive elements in an array of this type.
    /// This should include any alignment or padding.
    fn size_in_bytes(&self) -> u64 {
        std::mem::size_of::<Self>() as u64
    }

    /// The alignment for pointers of this type, which is useful for offset calculations.
    fn alignment_in_bytes(&self) -> u64 {
        std::mem::align_of::<Self>() as u64
    }
}

impl<T: SsbhWrite> SsbhWrite for &[T] {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // The data pointer must point past the containing struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr <= current_pos {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        for element in self.iter() {
            element.ssbh_write(writer, data_ptr)?;
        }

        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // TODO: This won't work for Vec<Option<T>> since only the first element is checked.
        match self.first() {
            Some(element) => self.len() as u64 * element.size_in_bytes(),
            None => 0,
        }
    }
}

impl<T: SsbhWrite> SsbhWrite for Option<T> {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        match self {
            Some(value) => value.ssbh_write(writer, data_ptr),
            None => Ok(()),
        }
    }

    fn size_in_bytes(&self) -> u64 {
        // None values are skipped entirely.
        // TODO: Is this a reasonable implementation?
        match self {
            Some(value) => value.size_in_bytes(),
            None => 0u64,
        }
    }

    fn alignment_in_bytes(&self) -> u64 {
        // Use the underlying type's alignment.
        // This is a bit of a hack since None values won't be written anyway.
        match self {
            Some(value) => value.alignment_in_bytes(),
            None => 8,
        }
    }
}

#[macro_export]
macro_rules! ssbh_write_modular_bitfield_impl {
    ($($id:ident),*) => {
        $(
            impl SsbhWrite for $id {
                fn ssbh_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    data_ptr: &mut u64,
                ) -> std::io::Result<()> {
                    // The data pointer must point past the containing struct.
                    let current_pos = writer.stream_position()?;
                    if *data_ptr <= current_pos {
                        *data_ptr = current_pos + self.size_in_bytes();
                    }

                    writer.write_all(&self.into_bytes())?;

                    Ok(())
                }

                fn alignment_in_bytes(&self) -> u64 {
                    self.size_in_bytes()
                }

                fn size_in_bytes(&self) -> u64 {
                    // TODO: Get size at compile time?
                    self.into_bytes().len() as u64
                }
            }
        )*
    }
}

macro_rules! ssbh_write_impl {
    ($($id:ident),*) => {
        $(
            impl SsbhWrite for $id {
                fn ssbh_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    _data_ptr: &mut u64,
                ) -> std::io::Result<()> {
                    writer.write_all(&self.to_le_bytes())?;
                    Ok(())
                }

                fn size_in_bytes(&self) -> u64 {
                    std::mem::size_of::<Self>() as u64
                }

                fn alignment_in_bytes(&self) -> u64 {
                    std::mem::align_of::<Self>() as u64
                }
            }
        )*
    }
}

ssbh_write_impl!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

macro_rules! ssbh_write_nonzero_impl {
    ($($id:ident),*) => {
        $(
            impl SsbhWrite for $id {
                fn ssbh_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    _data_ptr: &mut u64,
                ) -> std::io::Result<()> {
                    writer.write_all(&self.get().to_le_bytes())?;
                    Ok(())
                }

                fn size_in_bytes(&self) -> u64 {
                    std::mem::size_of::<Self>() as u64
                }

                fn alignment_in_bytes(&self) -> u64 {
                    std::mem::align_of::<Self>() as u64
                }
            }
        )*
    }
}

ssbh_write_nonzero_impl!(
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroUsize
);

impl<T: SsbhWrite> SsbhWrite for Vec<T> {
    fn ssbh_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        self.as_slice().ssbh_write(writer, data_ptr)
    }

    fn size_in_bytes(&self) -> u64 {
        if self.is_empty() {
            0
        } else {
            match self.first() {
                Some(first) => self.len() as u64 * first.size_in_bytes(),
                None => 0,
            }
        }
    }
}