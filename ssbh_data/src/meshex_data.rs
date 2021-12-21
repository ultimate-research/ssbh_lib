use itertools::Itertools;
use ssbh_lib::formats::meshex::AllData;
pub use ssbh_lib::{CString, Vector4};
use ssbh_lib::{MeshEx, Ptr64};

use crate::mesh_data::MeshObjectData;
use crate::SsbhData;

// TODO: Documentation.
#[derive(Debug, PartialEq)]
pub struct MeshExData {
    pub mesh_object_groups: Vec<MeshObjectGroupData>,
}

#[derive(Debug, PartialEq)]
pub struct MeshObjectGroupData {
    pub bounding_sphere: Vector4,
    pub mesh_object_full_name: String,
    pub mesh_object_name: String,
    // One entry for each mesh object?
    pub entry_flags: Vec<EntryFlags>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EntryFlags {
    pub draw_model: bool,
    pub cast_shadow: bool,
    // TODO: Preserve remaining flags?
}

impl SsbhData for MeshExData {
    type WriteError = std::io::Error;

    fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }

    fn read<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
    ) -> Result<(), Self::WriteError> {
        todo!()
    }

    fn write_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Self::WriteError> {
        todo!()
    }
}

impl MeshExData {
    pub fn from_mesh_objects(objects: &[MeshObjectData]) -> Self {
        todo!()
    }
}

impl MeshObjectGroupData {
    fn from_points() -> Self {
        todo!()
    }
}

// We shouldn't have any errors here?
// TODO: Do we care about null pointers?
// TODO: How will we calculate the file length?
// Just buffer into a vec and use the length?
impl From<&MeshEx> for MeshExData {
    fn from(m: &MeshEx) -> Self {
        Self {
            mesh_object_groups: m
                .mesh_object_groups
                .as_ref()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, g)| MeshObjectGroupData {
                    bounding_sphere: g.bounding_sphere,
                    mesh_object_full_name: g
                        .mesh_object_full_name
                        .as_ref()
                        .unwrap()
                        .to_string_lossy(),
                    mesh_object_name: g.mesh_object_name.as_ref().unwrap().to_string_lossy(),
                    entry_flags: m
                        .entries
                        .as_ref()
                        .unwrap()
                        .iter()
                        .positions(|e| e.mesh_object_group_index as usize == i)
                        .map(|entry_index| {
                            // TODO: Error handling here for invalid indices?
                            let entry_flags = m.entry_flags.as_ref().unwrap().0[entry_index];
                            EntryFlags {
                                draw_model: entry_flags.draw_model(),
                                cast_shadow: entry_flags.cast_shadow(),
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl From<&MeshExData> for MeshEx {
    fn from(m: &MeshExData) -> Self {
        Self {
            // TODO: How do we calculate length without a writer?
            // Is there some sort of heuristic for calculating it?
            file_length: 0,
            entry_count: m
                .mesh_object_groups
                .iter()
                .map(|g| g.entry_flags.len() as u32)
                .sum(),
            mesh_object_group_count: m.mesh_object_groups.len() as u32,
            all_data: Ptr64::new(AllData {
                // TODO: Calculate the correct bounding sphere.
                bounding_sphere: Vector4::ZERO,
                name: Ptr64::new("All".into()),
            }),
            mesh_object_groups: Ptr64::new(Vec::new()),
            entries: Ptr64::new(Vec::new()),
            entry_flags: Ptr64::new(ssbh_lib::formats::meshex::EntryFlags(Vec::new())),
            unk1: 0, // TODO: Preserve this value?
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ssbh_lib::{
        formats::meshex::{AllData, MeshEntry, MeshObjectGroup},
        MeshEx, Ptr64, Vector3, Vector4,
    };

    #[test]
    fn convert_mesh_ex_data() {
        // TODO: Test a case with valid indices.
        let meshex = MeshEx {
            file_length: 0,
            entry_count: 2,
            mesh_object_group_count: 2,
            all_data: Ptr64::new(AllData {
                bounding_sphere: Vector4::ZERO,
                name: Ptr64::new("All".into()),
            }),
            mesh_object_groups: Ptr64::new(vec![
                MeshObjectGroup {
                    bounding_sphere: Vector4::ZERO,
                    mesh_object_full_name: Ptr64::new("a_VIS".into()),
                    mesh_object_name: Ptr64::new("a".into()),
                },
                MeshObjectGroup {
                    bounding_sphere: Vector4::ZERO,
                    mesh_object_full_name: Ptr64::new("b_VIS".into()),
                    mesh_object_name: Ptr64::new("b".into()),
                },
            ]),
            entries: Ptr64::new(vec![
                MeshEntry {
                    mesh_object_group_index: 0,
                    unk1: Vector3::new(0.0, 1.0, 0.0),
                },
                MeshEntry {
                    mesh_object_group_index: 0,
                    unk1: Vector3::new(0.0, 1.0, 0.0),
                },
                MeshEntry {
                    mesh_object_group_index: 1,
                    unk1: Vector3::new(0.0, 1.0, 0.0),
                },
            ]),
            entry_flags: Ptr64::new(ssbh_lib::formats::meshex::EntryFlags(vec![
                // TODO: Test different flags
                ssbh_lib::formats::meshex::EntryFlag::new(),
                ssbh_lib::formats::meshex::EntryFlag::new(),
                ssbh_lib::formats::meshex::EntryFlag::new(),
            ])),
            unk1: 0,
        };

        let data = MeshExData {
            mesh_object_groups: vec![
                MeshObjectGroupData {
                    bounding_sphere: Vector4::ZERO,
                    mesh_object_full_name: "a_VIS".to_string(),
                    mesh_object_name: "a".to_string(),
                    entry_flags: vec![
                        EntryFlags {
                            draw_model: false,
                            cast_shadow: false,
                        },
                        EntryFlags {
                            draw_model: false,
                            cast_shadow: false,
                        },
                    ],
                },
                MeshObjectGroupData {
                    bounding_sphere: Vector4::ZERO,
                    mesh_object_full_name: "b_VIS".to_string(),
                    mesh_object_name: "b".to_string(),
                    entry_flags: vec![EntryFlags {
                        draw_model: false,
                        cast_shadow: false,
                    }],
                },
            ],
        };

        assert_eq!(data, MeshExData::from(&meshex));

        let new_meshex = MeshEx::from(&data);
        // TODO: Test the other direction?
        // TODO: How to test file length?
        // TODO: Test bounding spheres?
        assert_eq!(3, new_meshex.entry_count);
        assert_eq!(2, new_meshex.mesh_object_group_count);
        assert_eq!(
            "All",
            new_meshex
                .all_data
                .as_ref()
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .to_string_lossy()
        );
        // TODO: Tests groups, flags, etc.
    }
}
