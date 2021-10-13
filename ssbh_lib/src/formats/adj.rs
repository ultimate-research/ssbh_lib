use binread::{helpers::until_eof, BinRead};
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshItem {
    pub mesh_index: i32,
    /// The byte offset for the start of the indices for this [MeshItem] in [buffer](struct.Adj.html#structfield.buffer).
    /// The element count is calculated as the number of [i16] between the current offset and the offset of the next [MeshItem].
    pub buffer_offset: u32,
}

/// Mesh adjacency data for model.adjb files.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Adj {
    pub count: u32,
    #[br(count = count)]
    pub items: Vec<MeshItem>,
    /// A shared buffer of indices for [items](#structfield.items)
    #[br(parse_with = until_eof)]
    pub buffer: Vec<i16>,
}
