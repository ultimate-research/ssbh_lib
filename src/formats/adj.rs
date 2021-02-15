use binread::BinRead;

#[cfg(feature = "derive_serde")]
use serde::Serialize;

#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(BinRead, Debug)]
pub struct MeshItem {
    pub mesh_index: i32,
    pub buffer_offset: u32,
}

/// Mesh adjacency data for model.adjb files.
#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(BinRead, Debug)]
pub struct Adj {
    pub count: u32,
    #[br(count = count)]
    pub items: Vec<MeshItem>,
    // TODO: The offsets start from here
    // The remainder of the file is a buffer of u16's
    // Each mesh item's buffer starts at its offset and continues until the next item's offset
}
