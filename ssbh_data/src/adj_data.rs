use std::convert::TryFrom;

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
#[derive(Debug)]
pub struct AdjData {
    entries: Vec<AdjEntryData>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct AdjEntryData {
    pub vertex_adjacency: Vec<i16>,
}

impl AdjEntryData {
    /// Computes the vertex adjacency information from triangle faces.
    /// `vertex_indices` is assumed to contain triangle faces, so `vertex_indices.len()` 
    /// should be a multiple of 3, and the largest index should not exceed `vertex_count`.
    pub fn from_triangle_faces(vertex_indices: &[u32], vertex_count: usize) -> Self {
        Self {
            vertex_adjacency: triangle_adjacency(
                vertex_indices,
                vertex_count,
                MAX_ADJACENT_VERTICES,
            ),
        }
    }
}

// impl TryFrom<&Adj> for AdjData {
//     type Error;

//     fn try_from(adj: &Adj) -> Result<Self, Self::Error> {
//         // TODO: We want to find the indices for each mesh object.
//         // TODO: Return an error if the offsets aren't in ascending order?
//         Ok(AdjData {
//             entries: adj.entries.iter().map(|e| AdjEntryData {
//                 vertex_adjacency: todo!(),
//             }).collect(),
//         })
//     }
// }

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
    use super::*;

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
