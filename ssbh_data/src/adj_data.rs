use crate::{
    mesh_data::{MeshObjectData, VectorData},
    SsbhData,
};
use itertools::Itertools;
use ssbh_lib::{formats::adj::AdjEntry, Adj};
use std::convert::{TryFrom, TryInto};

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// For triangle faces if we omit the shared vertex,
// this works out to at most 9 adjacent faces.
const MAX_ADJACENT_VERTICES: usize = 18;

#[derive(Error, Debug)]

pub enum AdjError {
    /// An error occurred while writing data to a buffer.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The data associated with an [Adj] file.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq)]
pub struct AdjData {
    pub entries: Vec<AdjEntryData>,
}

// TODO: Use a macro to generate this?
impl SsbhData for AdjData {
    type WriteError = AdjError;

    fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Adj::from_file(path)?.try_into().map_err(Into::into)
    }

    fn read<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Adj::read(reader)?.try_into().map_err(Into::into)
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
    ) -> Result<(), Self::WriteError> {
        Adj::try_from(self)?.write(writer)?;
        Ok(())
    }

    fn write_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Self::WriteError> {
        Adj::try_from(self)?.write_to_file(path)?;
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq)]
pub struct AdjEntryData {
    pub mesh_object_index: usize,
    pub vertex_adjacency: Vec<i16>,
}

impl AdjEntryData {
    /// Computes the vertex adjacency information from triangle faces.
    /// `vertex_indices.len()` should be a multiple of 3.
    pub fn from_triangle_faces<T: PartialEq>(
        mesh_object_index: usize,
        vertex_positions: &[T],
        vertex_indices: &[u32],
    ) -> Self {
        Self {
            mesh_object_index,
            vertex_adjacency: triangle_adjacency(
                vertex_indices,
                vertex_positions,
                MAX_ADJACENT_VERTICES,
            ),
        }
    }

    /// Computes the vertex adjacency information from triangle faces from the given [MeshObjectData].
    pub fn from_mesh_object(mesh_object_index: usize, object: &MeshObjectData) -> Self {
        object
            .positions
            .first()
            .map(|position| {
                Self::from_vector_data(mesh_object_index, &position.data, &object.vertex_indices)
            })
            .unwrap_or(Self {
                mesh_object_index,
                vertex_adjacency: Vec::new(),
            })
    }

    /// Computes the vertex adjacency information from triangle faces from the given [VectorData].
    pub fn from_vector_data(
        mesh_object_index: usize,
        vertex_positions: &VectorData,
        vertex_indices: &[u32],
    ) -> Self {
        Self {
            mesh_object_index,
            vertex_adjacency: match vertex_positions {
                crate::mesh_data::VectorData::Vector2(v) => {
                    triangle_adjacency(vertex_indices, v, MAX_ADJACENT_VERTICES)
                }
                crate::mesh_data::VectorData::Vector3(v) => {
                    triangle_adjacency(vertex_indices, v, MAX_ADJACENT_VERTICES)
                }
                crate::mesh_data::VectorData::Vector4(v) => {
                    triangle_adjacency(vertex_indices, v, MAX_ADJACENT_VERTICES)
                }
            },
        }
    }
}

impl TryFrom<&AdjData> for Adj {
    type Error = std::io::Error;

    fn try_from(data: &AdjData) -> Result<Self, Self::Error> {
        Ok(Adj {
            entry_count: data.entries.len() as u32,
            entries: data
                .entries
                .iter()
                .scan(0, |offset, e| {
                    let entry = AdjEntry {
                        mesh_object_index: e.mesh_object_index as i32,
                        index_buffer_offset: *offset as u32,
                    };
                    *offset += e.vertex_adjacency.len() * std::mem::size_of::<i16>();
                    Some(entry)
                })
                .collect(),
            index_buffer: data
                .entries
                .iter()
                .map(|e| e.vertex_adjacency.clone())
                .flatten()
                .collect(),
        })
    }
}

impl TryFrom<AdjData> for Adj {
    type Error = std::io::Error;

    fn try_from(data: AdjData) -> Result<Self, Self::Error> {
        Adj::try_from(&data)
    }
}

impl TryFrom<&Adj> for AdjData {
    type Error = std::io::Error;

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

impl TryFrom<Adj> for AdjData {
    type Error = std::io::Error;

    fn try_from(adj: Adj) -> Result<Self, Self::Error> {
        AdjData::try_from(&adj)
    }
}

fn triangle_adjacency<T: PartialEq>(
    vertex_indices: &[u32],
    vertex_positions: &[T],
    padding_size: usize,
) -> Vec<i16> {
    // TODO: It should be doable to do this in fewer allocations.
    // TODO: This could be done with tinyvec or maintaining a separate count list.
    // TODO: Return an error for out of range vertices?
    // TODO: Should there be an error if there is a remainder?

    // Find the vertex indices from the all adjacent faces for each vertex.
    // We'll assume each face is a triangle with 3 distinct vertex indices.
    let mut adjacent_vertices = vec![Vec::new(); vertex_positions.len()];

    // The intuitive approach is to loop over the face list for each vertex.
    // It's more efficient to just loop over the faces once.
    // For N vertices and F faces, this takes O(F) instead of O(NF) time.
    for face in vertex_indices.chunks_exact(3) {
        if let [v0, v1, v2] = face {
            // TODO: Is this based on some sort of vertex winding order?
            // The shared vertex is omitted from each face.
            adjacent_vertices[*v0 as usize].push(*v1 as i16);
            adjacent_vertices[*v0 as usize].push(*v2 as i16);

            adjacent_vertices[*v1 as usize].push(*v2 as i16);
            adjacent_vertices[*v1 as usize].push(*v0 as i16);

            adjacent_vertices[*v2 as usize].push(*v0 as i16);
            adjacent_vertices[*v2 as usize].push(*v1 as i16);
        }
    }

    // Smash Ultimate adjb also use adjacent faces from split edges.
    // This prevents seams when recalculating normals.
    // TODO: Can this be done without a second vec?
    // TODO: Can this be done faster than O(N^2)?
    let mut adjacent_vertices_with_seams = vec![Vec::new(); vertex_positions.len()];
    for (i, _) in adjacent_vertices.iter().enumerate() {
        // TODO: Does this also include the vertex itself?
        // TODO: Avoid strict equality?
        for duplicate_index in vertex_positions
            .iter()
            .positions(|p| *p == vertex_positions[i])
        {
            adjacent_vertices_with_seams[i].extend_from_slice(&adjacent_vertices[duplicate_index]);
        }
    }

    // Smash Ultimate adjb files limit the number of adjacent vertices per vertex.
    // The special value of -1 is used for unused entries.
    // TODO: Is a fixed count per vertex required?
    adjacent_vertices_with_seams
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
    use ssbh_lib::formats::adj::AdjEntry;

    #[test]
    fn convert_adj_empty() {
        let adj = Adj {
            entry_count: 0,
            entries: Vec::new(),
            index_buffer: Vec::new(),
        };

        let data = AdjData {
            entries: Vec::new(),
        };

        assert_eq!(data, AdjData::try_from(&adj).unwrap());
        assert_eq!(adj, Adj::try_from(&data).unwrap());
    }

    #[test]
    fn convert_adj_single_entry() {
        let adj = Adj {
            entry_count: 1,
            entries: vec![AdjEntry {
                mesh_object_index: 12,
                index_buffer_offset: 0,
            }],
            index_buffer: vec![2, 3, 4, 5],
        };

        let data = AdjData {
            entries: vec![AdjEntryData {
                mesh_object_index: 12,
                vertex_adjacency: vec![2, 3, 4, 5],
            }],
        };

        assert_eq!(data, AdjData::try_from(&adj).unwrap());
        assert_eq!(adj, Adj::try_from(&data).unwrap());
    }

    #[test]
    fn convert_adj_multiple_entries() {
        let adj = Adj {
            entry_count: 3,
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

        let data = AdjData {
            entries: vec![
                AdjEntryData {
                    mesh_object_index: 0,
                    vertex_adjacency: vec![0],
                },
                AdjEntryData {
                    mesh_object_index: 3,
                    vertex_adjacency: vec![1, 1, 1],
                },
                AdjEntryData {
                    mesh_object_index: 2,
                    vertex_adjacency: vec![2, 2],
                },
            ],
        };

        assert_eq!(data, AdjData::try_from(&adj).unwrap());
        assert_eq!(adj, Adj::try_from(&data).unwrap());
    }

    fn flatten<T, const N: usize>(x: Vec<[T; N]>) -> Vec<T> {
        // Allow for visually grouping indices.
        x.into_iter().flatten().collect()
    }

    #[test]
    fn triangle_adjacency_empty() {
        assert!(triangle_adjacency(&[], &[0.0; 0], MAX_ADJACENT_VERTICES).is_empty());
    }

    #[test]
    fn triangle_adjacency_single_vertex_none_adjacent() {
        assert_eq!(
            vec![-1; 18],
            triangle_adjacency(&[], &[0.0], MAX_ADJACENT_VERTICES)
        );
    }

    #[test]
    #[ignore]
    fn triangle_adjacency_single_face_single_vertex() {
        // TODO: Should this be an error?
        triangle_adjacency(&[0, 1, 2], &[0.0], 4);
    }

    #[test]
    fn triangle_adjacency_single_face() {
        assert_eq!(
            flatten(vec![[1, 2, -1], [2, 0, -1], [0, 1, -1]]),
            triangle_adjacency(&[0, 1, 2], &[0.0, 0.5, 1.0], 3)
        );
    }

    #[test]
    fn triangle_adjacency_three_adjacent_faces() {
        assert_eq!(
            flatten(vec![
                [1, 2, 1, 2, 2, 1, -1],
                [2, 0, 2, 0, 0, 2, -1],
                [0, 1, 0, 1, 1, 0, -1]
            ]),
            triangle_adjacency(&[0, 1, 2, 2, 0, 1, 1, 0, 2], &[0.0, 0.5, 1.0], 7)
        );
    }

    #[test]
    fn triangle_adjacency_two_adjacent_faces_split_vertex() {
        // Vertex 0 and vertex 3 are the same.
        // This means they each have two adjacent faces.
        assert_eq!(
            flatten(vec![
                [1, 2, 4, 5, -1],
                [2, 0, -1, -1, -1],
                [0, 1, -1, -1, -1],
                [1, 2, 4, 5, -1],
                [5, 3, -1, -1, -1],
                [3, 4, -1, -1, -1],
            ]),
            triangle_adjacency(&[0, 1, 2, 3, 4, 5], &[0.0, 0.5, 1.0, 0.0, 1.5, 2.0], 5)
        );
    }
}
