use std::convert::TryFrom;

use itertools::Itertools;
use ssbh_lib::Adj;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::SsbhData;

// For triangle faces if we omit the shared vertex,
// this works out to at most 9 adjacent faces.
const MAX_ADJACENT_VERTICES: usize = 18;

/// The data associated with an [Adj] file.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq)]
pub struct AdjData {
    entries: Vec<AdjEntryData>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq)]
pub struct AdjEntryData {
    pub mesh_object_index: usize,
    pub vertex_adjacency: Vec<i16>,
}

impl AdjEntryData {
    /// Computes the vertex adjacency information from triangle faces.
    /// `vertex_indices` is assumed to contain triangle faces, so `vertex_indices.len()`
    /// should be a multiple of 3, and the largest index should not exceed `vertex_count`.
    pub fn from_triangle_faces(
        mesh_object_index: usize,
        vertex_indices: &[u32],
        vertex_count: usize,
    ) -> Self {
        Self {
            mesh_object_index,
            vertex_adjacency: triangle_adjacency(
                vertex_indices,
                vertex_count,
                MAX_ADJACENT_VERTICES,
            ),
        }
    }
}

impl TryFrom<&Adj> for AdjData {
    type Error = std::convert::Infallible;

    fn try_from(adj: &Adj) -> Result<Self, Self::Error> {
        let offset_to_index = |x| x as usize / std::mem::size_of::<i16>();

        // Assume that the buffer offsets are increasing.
        // This means the end of an entry's data is the start of the next entry's data.
        let mut entries = Vec::new();
        let mut entries_iter = adj.entries.iter().peekable();
        while let Some(entry) = entries_iter.next() {
            entries.push(AdjEntryData {
                mesh_object_index: entry.mesh_object_index as usize,
                vertex_adjacency: if let Some(next_entry) = entries_iter.peek() {
                    // TODO: Handle edge cases like start > end.
                    let start = offset_to_index(entry.index_buffer_offset);
                    let end = offset_to_index(next_entry.index_buffer_offset);
                    adj.index_buffer[start..end].into()
                } else {
                    // The last entry uses the remaining indices.
                    adj.index_buffer[offset_to_index(entry.index_buffer_offset)..].into()
                },
            })
        }

        Ok(AdjData { entries })
    }
}

fn triangle_adjacency(
    vertex_indices: &[u32],
    vertex_count: usize,
    padding_size: usize,
) -> Vec<i16> {
    // TODO: It should be doable to do this in fewer allocations.
    // TODO: Benchmark with smallvec?
    // TODO: Return an error for out of range vertices?
    let mut adjacent_vertices = vec![Vec::new(); vertex_count];

    dbg!(adjacent_vertices.len());
    // Find the vertex indices from the all adjacent faces for each vertex.
    // We'll assume each face is a triangle with 3 distinct vertex indices.
    // TODO: Should there be an error if there is a remainder?

    // The intuitive approach is to loop over the face list for each vertex.
    // It's more efficient to just loop over the faces once.
    // For N vertices and F faces, this takes O(F) instead of O(NF) time.
    for face in vertex_indices.chunks_exact(3) {
        if let [v0, v1, v2] = face {
            // The convention is to ommit the shared vertex from each face.
            adjacent_vertices[*v0 as usize].push(*v1 as i16);
            adjacent_vertices[*v0 as usize].push(*v2 as i16);

            adjacent_vertices[*v1 as usize].push(*v0 as i16);
            adjacent_vertices[*v1 as usize].push(*v2 as i16);

            adjacent_vertices[*v2 as usize].push(*v0 as i16);
            adjacent_vertices[*v2 as usize].push(*v1 as i16);
        }
    }

    // Smash Ultimate adjb files limit the number of adjacent vertices per vertex.
    // The special value of -1 is used for unused entries.
    // TODO: Is a fixed count per vertex required?
    adjacent_vertices
        .into_iter()
        .map(|mut a| {
            a.resize(padding_size, -1);
            a
        })
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use ssbh_lib::formats::adj::AdjEntry;

    use super::*;

    #[test]
    fn create_adj_data_empty() {
        let adj = Adj {
            count: 0,
            entries: Vec::new(),
            index_buffer: Vec::new(),
        };

        let data = AdjData::try_from(&adj).unwrap();
        assert!(data.entries.is_empty());
    }

    #[test]
    fn create_adj_data_single_entry() {
        // Test the handling of offsets.
        let adj = Adj {
            count: 1,
            entries: vec![AdjEntry {
                mesh_object_index: 12,
                index_buffer_offset: 4,
            }],
            index_buffer: vec![-1, -1, 2, 3, 4, 5],
        };

        let data = AdjData::try_from(&adj).unwrap();
        assert_eq!(
            vec![AdjEntryData {
                mesh_object_index: 12,
                vertex_adjacency: vec![2, 3, 4, 5]
            }],
            data.entries
        );
    }

    #[test]
    fn create_adj_data_multiple_entries() {
        // Test the handling of offsets.
        let adj = Adj {
            count: 1,
            entries: vec![
                AdjEntry {
                    mesh_object_index: 0,
                    index_buffer_offset: 0,
                },
                AdjEntry {
                    mesh_object_index: 3,
                    index_buffer_offset: 2,
                },
                AdjEntry {
                    mesh_object_index: 2,
                    index_buffer_offset: 8,
                },
            ],
            index_buffer: vec![0, 1, 1, 1, 2, 2],
        };

        let data = AdjData::try_from(&adj).unwrap();
        assert_eq!(
            vec![
                AdjEntryData {
                    mesh_object_index: 0,
                    vertex_adjacency: vec![0]
                },
                AdjEntryData {
                    mesh_object_index: 3,
                    vertex_adjacency: vec![1, 1, 1]
                },
                AdjEntryData {
                    mesh_object_index: 2,
                    vertex_adjacency: vec![2, 2]
                }
            ],
            data.entries
        );
    }

    // TODO: Is it doable to match the ordering used in Smash Ultimate?
    #[test]
    fn triangle_adjacency_empty() {
        assert!(triangle_adjacency(&[], 0, MAX_ADJACENT_VERTICES).is_empty());
    }

    #[test]
    fn triangle_adjacency_single_vertex_none_adjacent() {
        assert_eq!(
            vec![-1; 18],
            triangle_adjacency(&[], 1, MAX_ADJACENT_VERTICES)
        );
    }

    #[test]
    fn triangle_adjacency_single_face_single_vertex() {
        assert_eq!(vec![1, 2, -1, -1], triangle_adjacency(&[0, 1, 2], 1, 4));
    }

    #[test]
    fn triangle_adjacency_single_face() {
        assert_eq!(
            vec![1, 2, -1, 0, 2, -1, 0, 1, -1],
            triangle_adjacency(&[0, 1, 2], 3, 3)
        );
    }

    #[test]
    fn triangle_adjacency_three_adjacent_faces() {
        assert_eq!(
            vec![1, 2, 2, 1, 1, 2, -1, 0, 2, 2, 0, 0, 2, -1, 0, 1, 0, 1, 1, 0, -1],
            triangle_adjacency(&[0, 1, 2, 2, 0, 1, 1, 0, 2], 3, 7)
        );
    }
}
