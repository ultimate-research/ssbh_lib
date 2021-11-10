use binread::{helpers::until_eof, BinRead};
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Adjacency data for a [MeshObject](crate::mesh::MeshObject).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AdjEntry {
    // TODO: Can this be negative?
    /// The index of the [MeshObject](crate::mesh::MeshObject).
    pub mesh_object_index: i32,

    // TODO: Show some example code.
    /// The byte offset for the start of the indices for this [AdjEntry] in [index_buffer](struct.Adj.html#structfield.buffer).
    /// The element count is calculated as the number of [i16] between the current offset and the offset of the next [MeshItem].
    pub index_buffer_offset: u32,
}

/// Mesh adjacency data for model.adjb files.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Adj {
    pub count: u32,

    #[br(count = count)]
    pub entries: Vec<AdjEntry>,

    /// A flattened list of adjacent vertex indices for [entries](#structfield.entries)
    ///
    /// Each vertex for each [MeshObject](crate::mesh::MeshObject) stores the 
    /// indices for vertices from adjacent faces in the corresponding section of the buffer.
    /// The shared vertex for the adjacent face is not explicitly stored, 
    /// so an adjacent triangle can be encoded as just two index values.
    /// 
    /// Each vertex's adjacent vertex list is padded with the value `-1` 
    /// to ensure all per vertex lists have the same number of elements.
    /// For Smash Ultimate, this is 18.
    #[br(parse_with = until_eof)]
    pub index_buffer: Vec<i16>,
}
