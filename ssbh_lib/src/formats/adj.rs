use std::io::{Read, Seek};
use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};


#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshItem {
    pub mesh_index: i32,
    /// The byte offset for the start of the indices for this [MeshItem] in [buffer](struct.Adj.html#structfield.buffer).
    /// The element count is calculated as the number of [i16] between the current offset and the offset of the next [MeshItem].
    pub buffer_offset: u32,
}

/// Mesh adjacency data for model.adjb files.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Adj {
    pub count: u32,
    #[br(count = count)]
    pub items: Vec<MeshItem>,
    /// A shared buffer of indices for [items](#structfield.items)
    #[br(parse_with = read_to_end)]
    pub buffer: Vec<i16>,
}

// TODO: This could be a shared function in lib.rs.
fn read_to_end<R: Read + Seek>(reader: &mut R, _ro: &ReadOptions, _: ()) -> BinResult<Vec<i16>> {
    let mut buf = Vec::new();
    // TODO: Read until EOF?
    while let Ok(v) = reader.read_le::<i16>() {
        buf.push(v);
    }
    Ok(buf)
}
