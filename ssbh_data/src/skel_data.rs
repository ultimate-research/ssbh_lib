//! Types for working with [Skel] data in .nusktb files.
//!
//! The data for each bone is collected into a [BoneData] struct with a single transformation matrix.
//! The missing matrices are recalculated when converting to [Skel] based on the heirachy of [BoneData].
//!
//! # File Differences
//! Unmodified files are not guaranteed to be binary identical after saving.
//! Calculated matrices may differ from the originals due to slightly different algorithms and floating point errors.
//! These errors are very small in practice but may cause gameplay differences such as online desyncs.
use std::{
    collections::HashSet,
    convert::TryInto,
    io::{Read, Seek},
    path::Path,
};

use glam::Mat4;
use ssbh_lib::{
    formats::skel::{BillboardType, Skel, SkelBoneEntry, SkelEntryFlags},
    Matrix4x4,
};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{create_ssbh_array, SsbhData};

// TODO: Add methods to SkelData to find the index of a given bone?

/// The data associated with a [Skel] file.
/// The supported version is 1.0.
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct SkelData {
    pub major_version: u16,
    pub minor_version: u16,
    pub bones: Vec<BoneData>,
}

/// Data associated with a [SkelBoneEntry].
///
/// Only the bone's transformation relative to its parent is stored.
/// The missing transformation matrices are calculated when converting to [Skel]
/// based on the heirarchy of [BoneData].
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct BoneData {
    /// The name of the bone.
    pub name: String,
    /// A matrix in row-major order representing the transform of the bone relative to its parent.
    /// For using existing world transformations, see [calculate_relative_transform].
    pub transform: [[f32; 4]; 4],
    /// The index of the parent bone in the bones collection or [None] if this is a root bone with no parents.
    pub parent_index: Option<usize>,
    // TODO: Flags?
}

impl SsbhData for SkelData {
    type WriteError = SkelError;

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let skel = Skel::from_file(path)?;
        Ok((&skel).into())
    }

    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        let skel = Skel::read(reader)?;
        Ok((&skel).into())
    }

    fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> Result<(), SkelError> {
        let skel = create_skel(self)?;
        skel.write(writer)?;
        Ok(())
    }

    fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SkelError> {
        let skel = create_skel(self)?;
        skel.write_to_file(path)?;
        Ok(())
    }
}

/// Errors while creating an [Skel] from [SkelData].
#[derive(Error, Debug)]

pub enum SkelError {
    /// Creating a [Skel] file for the given version is not supported.
    #[error(
        "Creating a version {}.{} skel is not supported.",
        major_version,
        minor_version
    )]
    UnsupportedVersion {
        major_version: u16,
        minor_version: u16,
    },

    /// An error occurred while calculating a transformation matrix.
    #[error(transparent)]
    BoneTransform(#[from] BoneTransformError),

    /// An error occurred while writing data to a buffer.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Calculates the transform of `world_transform` relative to `parent_world_transform`.
/// If `parent_world_transform` is [None], a copy of `world_transform` is returned.
/// All matrices are assumed to be in row-major order.
/// # Examples
/// The simplest case is when the parent transform is the identity matrix,
/// so the result is simply the passed in `world_transform`.
/**
```rust
# use ssbh_data::skel_data::calculate_relative_transform;
// A row-major transform with a scale (2, 4, 8) and translation (1, 2, 3).
let world_transform = [
    [2.0, 0.0, 0.0, 0.0],
    [0.0, 4.0, 0.0, 0.0],
    [0.0, 0.0, 8.0, 0.0],
    [1.0, 2.0, 3.0, 1.0],
];
// The 4x4 identity matrix.
let parent_world_transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

assert_eq!(
    world_transform,
    calculate_relative_transform(
        &world_transform,
        Some(&parent_world_transform)
    )
);
```
*/
pub fn calculate_relative_transform(
    world_transform: &[[f32; 4]; 4],
    parent_world_transform: Option<&[[f32; 4]; 4]>,
) -> [[f32; 4]; 4] {
    match parent_world_transform {
        Some(parent_world_transform) => {
            // world_transform = parent_world_transform * relative_transform
            // relative_transform = inverse(parent_world_transform) * world_transform
            let world = mat4_from_row2d(world_transform);
            let parent_world = mat4_from_row2d(parent_world_transform);
            let relative = parent_world.inverse().mul_mat4(&world);
            relative.transpose().to_cols_array_2d()
        }
        None => *world_transform,
    }
}

fn inv_transform(m: &[[f32; 4]; 4]) -> Matrix4x4 {
    let m = mat4_from_row2d(m);
    let inv = m.inverse().transpose().to_cols_array_2d();
    Matrix4x4::from_rows_array(&inv)
}

fn create_skel(data: &SkelData) -> Result<Skel, SkelError> {
    let world_transforms = data
        .bones
        .iter()
        .map(|b| data.calculate_world_transform(b))
        .collect::<Result<Vec<_>, _>>()?;

    // TODO: Add a test for this with a few bones.
    Ok(Skel {
        major_version: data.major_version,
        minor_version: data.minor_version,
        bone_entries: data
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| SkelBoneEntry {
                name: b.name.clone().into(),
                index: i as u16,
                parent_index: match b.parent_index {
                    Some(index) => index as i16,
                    None => -1,
                },
                // TODO: Preserve or calculate flags?
                flags: SkelEntryFlags {
                    unk1: 1,
                    billboard_type: BillboardType::None,
                },
            })
            .collect::<Vec<SkelBoneEntry>>()
            .into(),
        world_transforms: create_ssbh_array(&world_transforms, Matrix4x4::from_rows_array),
        inv_world_transforms: create_ssbh_array(&world_transforms, inv_transform),
        transforms: create_ssbh_array(&data.bones, |b| Matrix4x4::from_rows_array(&b.transform)),
        inv_transforms: create_ssbh_array(&data.bones, |b| inv_transform(&b.transform)),
    })
}

impl From<&Skel> for SkelData {
    fn from(skel: &Skel) -> Self {
        Self {
            major_version: skel.major_version,
            minor_version: skel.minor_version,
            // TODO: Add additional validation for mismatched array lengths?
            bones: skel
                .bone_entries
                .elements
                .iter()
                .zip(skel.transforms.elements.iter())
                .map(|(b, t)| create_bone_data(b, t))
                .collect(),
        }
    }
}

fn create_bone_data(b: &SkelBoneEntry, transform: &Matrix4x4) -> BoneData {
    BoneData {
        name: b.name.to_string_lossy(),
        transform: transform.to_rows_array(),
        parent_index: b.parent_index.try_into().ok(),
    }
}

fn mat4_from_row2d(elements: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(elements).transpose()
}

impl SkelData {
    /// Calculates the world transform for `bone` by accumulating the transform with the parents transform recursively.
    /// Returns the resulting matrix in row-major order.
    /// # Examples
    /// For single bound mesh objects, the object is transformed by the parent bone's world transform.
    /**
    ```rust
    # use ssbh_data::skel_data::{BoneData, SkelData};
    # let data = SkelData {
    #     major_version: 1,
    #     minor_version: 0,
    #     bones: vec![BoneData {
    #         name: "Head".to_string(),
    #         transform: [[0f32; 4]; 4],
    #         parent_index: None,
    #     }],
    # };
    let parent_bone_name = "Head";
    if let Some(parent_bone) = data.bones.iter().find(|b| b.name == parent_bone_name) {
        let world_transform = data.calculate_world_transform(&parent_bone);
        // Transform the object using the matrix...
    }
    ```
    */
    pub fn calculate_world_transform(
        &self,
        bone: &BoneData,
    ) -> Result<[[f32; 4]; 4], BoneTransformError> {
        let mut bone = bone;
        let mut transform = mat4_from_row2d(&bone.transform);

        // Check for cycles by keeping track of previously visited locations.
        let mut visited = HashSet::new();

        // Accumulate transforms by travelling up the bone heirarchy.
        while let Some(parent_index) = bone.parent_index {
            if !visited.insert(parent_index) {
                return Err(BoneTransformError::CycleDetected {
                    index: parent_index,
                });
            }
            if let Some(parent_bone) = self.bones.get(parent_index) {
                let parent_transform = mat4_from_row2d(&parent_bone.transform);
                transform = transform.mul_mat4(&parent_transform);
                bone = parent_bone;
            } else {
                break;
            }
        }

        // Save the result in row-major order.
        Ok(transform.transpose().to_cols_array_2d())
    }
}

/// Errors while calculating [BoneData] transformation matrices.
#[derive(Error, Debug)]
pub enum BoneTransformError {
    #[error(
        "Cyclical bone chains are not supported. A cycle was detected at index {}.",
        index
    )]
    CycleDetected { index: usize },
}

#[cfg(test)]
mod tests {
    use approx::relative_eq;

    use super::*;

    #[test]
    fn create_bone_data_no_parent() {
        let b = SkelBoneEntry {
            name: "abc".into(),
            index: 2,
            parent_index: -1,
            flags: SkelEntryFlags {
                unk1: 1,
                billboard_type: BillboardType::None,
            },
        };
        let data = create_bone_data(&b, &Matrix4x4::identity());
        assert_eq!("abc", data.name);
        assert_eq!(
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ],
            data.transform
        );
        assert_eq!(None, data.parent_index);
    }

    #[test]
    fn create_bone_data_negative_parent() {
        // The convention is for -1 to indicate no parent.
        // We convert to usize, so just treat all negative indices as no parent.
        let b = SkelBoneEntry {
            name: "abc".into(),
            index: 2,
            parent_index: -5,
            flags: SkelEntryFlags {
                unk1: 1,
                billboard_type: BillboardType::None,
            },
        };
        let data = create_bone_data(&b, &Matrix4x4::identity());
        assert_eq!("abc", data.name);
        assert_eq!(None, data.parent_index);
    }

    #[test]
    fn calculate_relative_transform_with_parent() {
        let world_transform = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, 8.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let parent_world_transform = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [1.0, 2.0, 3.0, 1.0],
        ];
        let relative_transform = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, 8.0, 0.0],
            [-2.0, -8.0, -24.0, 1.0],
        ];
        assert_eq!(
            relative_transform,
            calculate_relative_transform(&world_transform, Some(&parent_world_transform))
        );
    }

    #[test]
    fn calculate_relative_transform_no_parent() {
        let world_transform = [
            [0.0, 1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0, 7.0],
            [8.0, 9.0, 10.0, 11.0],
            [12.0, 13.0, 14.0, 15.0],
        ];
        assert_eq!(
            world_transform,
            calculate_relative_transform(&world_transform, None)
        );
    }

    // TODO: There might be a way that gives better output on failure.
    fn matrices_are_relative_eq(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> bool {
        a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(a, b)| relative_eq!(a, b, epsilon = 0.0001f32))
    }

    #[test]
    fn test_matrix_relative_eq() {
        let transform = [
            [0.0, 1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0, 7.0],
            [8.0, 9.0, 10.0, 11.0],
            [12.0, 13.0, 14.0, 15.0],
        ];
        assert!(matrices_are_relative_eq(transform, transform));
    }

    #[test]
    fn world_transform_no_parent() {
        // Use unique values to make sure the matrix is correct.
        let transform = [
            [0.0, 1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0, 7.0],
            [8.0, 9.0, 10.0, 11.0],
            [12.0, 13.0, 14.0, 15.0],
        ];

        let data = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![BoneData {
                name: "root".to_string(),
                transform,
                parent_index: None,
            }],
        };

        assert_eq!(
            transform,
            data.calculate_world_transform(&data.bones[0]).unwrap()
        );
    }

    #[test]
    fn world_transform_self_referential_bone() {
        let data = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![BoneData {
                name: "root".to_string(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
            }],
        };

        // This should still terminate.
        let result = data.calculate_world_transform(&data.bones[0]);
        assert!(matches!(
            result,
            Err(BoneTransformError::CycleDetected { index: 0 })
        ));
    }

    #[test]
    fn world_transform_bone_cycle() {
        let data = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "a".to_string(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                },
                BoneData {
                    name: "b".to_string(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(2),
                },
                BoneData {
                    name: "c".to_string(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(1),
                },
                BoneData {
                    name: "d".to_string(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(2),
                },
            ],
        };

        // This should still terminate.
        let result = data.calculate_world_transform(&data.bones[2]);
        assert!(matches!(
            result,
            Err(BoneTransformError::CycleDetected { index: 1 })
        ));
    }

    #[test]
    fn world_transform_multi_parent_chain() {
        // Cloud c00 model.nusktb.
        let data = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "Trans".to_string(),
                    transform: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ],
                    parent_index: None,
                },
                BoneData {
                    name: "Rot".to_string(),
                    transform: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 11.241, 0.268775, 1.0],
                    ],
                    parent_index: Some(0),
                },
                BoneData {
                    name: "Hip".to_string(),
                    transform: [
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ],
                    parent_index: Some(1),
                },
                BoneData {
                    name: "Waist".to_string(),
                    transform: [
                        [0.999954, -0.00959458, 0.0, 0.0],
                        [0.00959458, 0.999954, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [1.38263, 0.0, 0.0, 1.0],
                    ],
                    parent_index: Some(2),
                },
            ],
        };

        assert!(matrices_are_relative_eq(
            [
                [0.0, 0.999954, -0.00959458, 0.0],
                [0.0, 0.00959458, 0.999954, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 12.6236, 0.268775, 1.0]
            ],
            data.calculate_world_transform(&data.bones[3]).unwrap(),
        ));
    }
}
