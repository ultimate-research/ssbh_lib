use crate::{SsbhArray, SsbhString};
use crate::RelPtr64;
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct MeshItem {
    mesh_index: i32,
    buffer_offset: u32
}

/// Mesh adjacency data for model.adjb files.
#[derive(Serialize, BinRead, Debug)]
pub struct Adj {
    count: u32,
    #[br(count = count)]
    items: Vec<MeshItem>
    // TODO: The offsets start from here
    // The remainder of the file is a buffer of u16's
    // Each mesh item's buffer starts at its offset and continues until the next item's offset
}