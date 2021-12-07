//! The [Adj] is a non SSBH format that stores vertex adjacency data.
//! These files typically use the ".adjb" suffix like "model.adjb".
//!
//! Adjacency information is stored in a combined index buffer for all the [MeshObject](crate::mesh::MeshObject) with corresponding entries.
//! The buffer contains indices for all the vertices in adjacent faces to each vertex.
use binread::{helpers::until_eof, BinRead};
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Adjacency data for a [MeshObject](crate::mesh::MeshObject).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct AdjEntry {
    // TODO: Can this be negative?
    /// The index of the [MeshObject](crate::mesh::MeshObject).
    pub mesh_object_index: i32,

    // TODO: Show some example code.
    /// The byte offset for the start of the indices for this [AdjEntry] in [index_buffer](struct.Adj.html#structfield.index_buffer).
    /// The element count is calculated as the number of [i16] between the current offset and the offset of the next [AdjEntry].
    pub index_buffer_offset: u32,
}

/// Mesh adjacency data for model.adjb files.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Eq)]
pub struct Adj {
    pub entry_count: u32,

    /// A collection containing an entry for each [MeshObject](crate::mesh::MeshObject)
    /// with adjacency information in [index_buffer](#structfield.index_buffer)
    #[br(count = entry_count)]
    pub entries: Vec<AdjEntry>,

    /// A flattened list of adjacent vertex indices for [entries](#structfield.entries)
    ///
    /// Each vertex for each [MeshObject](crate::mesh::MeshObject) stores the
    /// indices for vertices from adjacent faces in the corresponding section of the buffer.
    /// The shared vertex for the adjacent face is not explicitly stored,
    /// so an adjacent triangle can be encoded as just two index values.
    ///
    /// The section of adjacent vertices for each vertex is padded with the value `-1`
    /// to ensure all vertices have an equal number of buffer elements.
    /// For example, suppose the vertex with index 0 is adjacent to the triangle face with vertex indices (0, 1, 2).
    /// This would be encoded in the buffer as `[1, 2, -1, -1, ...]` padded to the appropriate size.
    #[br(parse_with = until_eof)]
    pub index_buffer: Vec<i16>,
}
