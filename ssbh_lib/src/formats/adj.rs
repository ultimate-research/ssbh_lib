//! [Adj] is a non SSBH format that stores vertex adjacency data.
//! These files typically use the ".adjb" suffix like "model.adjb".
//!
//! Adjacency information is stored in a combined index buffer for all the [MeshObject](crate::mesh::MeshObject) with corresponding entries.
//! The buffer contains indices for all the vertices in adjacent faces to each vertex.
use binrw::{binread, helpers::until_eof, BinRead};
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Adjacency data for a [MeshObject](crate::mesh::MeshObject).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[binread]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq)]
pub struct Adj {
    #[br(temp)]
    entry_count: u32,

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
    /// The section of adjacent vertices for each vertex is padded with the value `-1`.
    /// to ensure all vertices have an equal number of buffer elements.
    /// The shared vertex of each face is implied.
    /// For example, the vertex with index 0 is adjacent to the triangle face with vertex indices (0, 1, 2).
    /// This would be encoded in the buffer as `[1, 2, -1, -1, ...]`.
    ///
    /// Smash Ultimate uses 18 elements for each vertex.
    /// This allows for 9 adjacent triangle faces instead of 6 since we omit the shared vertex.
    #[br(parse_with = until_eof)]
    pub index_buffer: Vec<i16>,
}

// A size_in_bytes implementation isn't necessary since there are no pointers.
impl SsbhWrite for Adj {
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
        (self.entries.len() as u32).ssbh_write(writer, data_ptr)?;
        self.entries.ssbh_write(writer, data_ptr)?;
        self.index_buffer.ssbh_write(writer, data_ptr)?;
        Ok(())
    }
}
