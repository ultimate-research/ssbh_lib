//! # ssbh_data
//!
//! ssbh_data provides a more intuitive and minimal API built on ssbh_lib.
//!
//! ## Features
//! The high level nature of ssbh_data makes it easier to integrate with application code than ssbh_lib.
//! Python bindings are also available with [ssbh_data_py](https://github.com/ScanMountGoat/ssbh_data_py).
//! - Automatic decoding and encoding of buffers and compressed data
//! - Usage of standard Rust types like [Vec] and [String]
//! - Support for converting files to and from supported versions
//! - Simpler output when serializing and deserializing
//! - Errors for invalid data such as out of bounds vertex indices
//! - Modifications are less likely to produce an invalid file due to reduced dependencies between fields
//!
//! ## Getting Started
//! The easiest way to access important items like [MeshData](crate::mesh_data::MeshData) is to import the [prelude].
//! For additional reading and writing options, see the [SsbhData] trait.
/*!
```no_run
use ssbh_data::prelude::*;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Read the file from disk.
let mut data = MeshData::from_file("model.numshb")?;

// Make some edits.
data.objects[0].name = "firstMesh".to_string();

// Save the changes.
data.write_to_file("model_new.numshb")?;
# Ok(())
# }
```
 */
//!
//! ## File Differences
//! The reduction in dependencies between fields and decoding and encoding of buffer data means
//! that ssbh_data does not guarantee an unmodified file to be binary identical after saving.
//! Examples include floating point rounding errors, larger file sizes due to different compression settings,
//! or default values used for unresearched flag values.
//! See the module level documentation for each format for details.
//!
//! These differences are minor in practice but may cause issues for some applications.
//! Applications needing a stronger guarantee that all data will be preserved
//! should use [ssbh_lib](https://crates.io/crates/ssbh_lib).
pub mod adj_data;
pub mod anim_data;
pub mod hlpb_data;
pub mod matl_data;
pub mod mesh_data;
pub mod meshex_data;
pub mod modl_data;
#[doc(hidden)]
pub mod shdr_data;
pub mod skel_data;

use binrw::io::{Read, Seek, Write};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::path::Path;

use ssbh_lib::prelude::*;
use ssbh_lib::SsbhArray;

/// Functions for reading and writing supported formats.
pub trait SsbhData: Sized {
    type WriteError: Error;
    // TODO: Also specify the read error type?

    /// Tries to read and convert the data from `reader`.
    /// The entire file is buffered for performance.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>;

    /// Tries to read and convert the data from `reader`.
    /// For best performance when opening from a file, use [SsbhData::from_file] instead.
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>>;

    /// Converts the data and writes to the given `writer`.
    /// For best performance when writing to a file, use [SsbhData::write_to_file] instead.
    fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Self::WriteError>;

    /// Converts the data and writes to the given `path`.
    /// The entire file is buffered for performance.
    fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Self::WriteError>;
}

/// Common imports for supported types and important traits.
pub mod prelude {
    pub use crate::adj_data::AdjData;
    pub use crate::anim_data::AnimData;
    pub use crate::hlpb_data::HlpbData;
    pub use crate::matl_data::MatlData;
    pub use crate::mesh_data::MeshData;
    pub use crate::meshex_data::MeshExData;
    pub use crate::modl_data::ModlData;
    pub use crate::skel_data::SkelData;
    pub use crate::SsbhData;
}

macro_rules! ssbh_data_impl {
    ($ssbh_data:ty, $ssbh_lib:ty, $error:ty) => {
        impl SsbhData for $ssbh_data {
            type WriteError = $error;

            fn from_file<P: AsRef<std::path::Path>>(
                path: P,
            ) -> Result<Self, Box<dyn std::error::Error>> {
                <$ssbh_lib>::from_file(path)?.try_into().map_err(Into::into)
            }

            fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
                <$ssbh_lib>::read(reader)?.try_into().map_err(Into::into)
            }

            fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Self::WriteError> {
                <$ssbh_lib>::try_from(self)?
                    .write(writer)
                    .map_err(Into::into)
            }

            fn write_to_file<P: AsRef<std::path::Path>>(
                &self,
                path: P,
            ) -> Result<(), Self::WriteError> {
                <$ssbh_lib>::try_from(self)?
                    .write_to_file(path)
                    .map_err(Into::into)
            }
        }
    };
}

macro_rules! ssbh_data_infallible_impl {
    ($ssbh_data:ty, $ssbh_lib:ty, $error:ty) => {
        impl SsbhData for $ssbh_data {
            type WriteError = $error;

            fn from_file<P: AsRef<std::path::Path>>(
                path: P,
            ) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(<$ssbh_lib>::from_file(path)?.into())
            }

            fn read<R: std::io::Read + std::io::Seek>(
                reader: &mut R,
            ) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(<$ssbh_lib>::read(reader)?.into())
            }

            fn write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
            ) -> Result<(), Self::WriteError> {
                <$ssbh_lib>::from(self).write(writer)
            }

            fn write_to_file<P: AsRef<std::path::Path>>(
                &self,
                path: P,
            ) -> Result<(), Self::WriteError> {
                <$ssbh_lib>::from(self).write_to_file(path)
            }
        }
    };
}

ssbh_data_impl!(adj_data::AdjData, Adj, adj_data::error::Error);
ssbh_data_impl!(anim_data::AnimData, Anim, anim_data::error::Error);
ssbh_data_impl!(matl_data::MatlData, Matl, matl_data::error::Error);
ssbh_data_impl!(mesh_data::MeshData, Mesh, mesh_data::error::Error);
ssbh_data_infallible_impl!(meshex_data::MeshExData, MeshEx, std::io::Error);
ssbh_data_infallible_impl!(modl_data::ModlData, Modl, std::io::Error);
ssbh_data_infallible_impl!(hlpb_data::HlpbData, Hlpb, std::io::Error);
ssbh_data_impl!(skel_data::SkelData, Skel, skel_data::error::Error);

// TODO: Should this be part of SsbhLib?
fn create_ssbh_array<T, B, F: Fn(&T) -> B>(elements: &[T], create_b: F) -> SsbhArray<B> {
    elements.iter().map(create_b).collect::<Vec<B>>().into()
}

#[cfg(test)]
pub(crate) fn group_hex(a: &str, words_per_line: usize) -> String {
    use itertools::Itertools;

    // TODO: Find a cleaner way of doing this.
    // ex: "FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF..."
    let words = a
        .chars()
        .collect::<Vec<char>>()
        .chunks(8)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<String>>();

    words.chunks(words_per_line).map(|c| c.join(" ")).join("\n")
}

#[cfg(test)]
macro_rules! assert_hex_eq {
    ($a:expr, $b:expr) => {
        pretty_assertions::assert_str_eq!(
            crate::group_hex(&hex::encode($a), 8),
            crate::group_hex(&hex::encode($b), 8)
        )
    };
}

#[cfg(test)]
pub(crate) use assert_hex_eq;
