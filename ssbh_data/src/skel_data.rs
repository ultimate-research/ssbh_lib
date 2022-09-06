//! Types for working with [Skel] data in .nusktb files.
//!
//! The data for each bone is collected into a [BoneData] struct with a single transformation matrix.
//! The missing matrices are recalculated when converting to [Skel] based on the hierarchy of [BoneData].
//!
//! # File Differences
//! Unmodified files are not guaranteed to be binary identical after saving.
//! Calculated matrices may differ from the originals due to slightly different algorithms and floating point errors.
//! These errors are very small in practice but may cause gameplay differences such as online desyncs.
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
};

use glam::Mat4;
pub use ssbh_lib::formats::skel::BillboardType;
use ssbh_lib::{
    formats::skel::{Skel, SkelBoneEntry, SkelEntryFlags},
    Matrix4x4, Version,
};

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// TODO: Add methods to SkelData to find the index of a given bone?

/// The data associated with a [Skel] file.
/// The supported version is 1.0.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct SkelData {
    pub major_version: u16,
    pub minor_version: u16,
    pub bones: Vec<BoneData>,
}

/// Data associated with a [SkelBoneEntry].
///
/// Only the bone's transformation relative to its parent is stored.
/// The missing transformation matrices are calculated when converting to [Skel]
/// based on the hierarchy of [BoneData].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct BoneData {
    /// The name of the bone.
    pub name: String,
    /// A matrix in column-major order representing the transform of the bone relative to its parent.
    /// For using existing world transformations, see [calculate_relative_transform].
    pub transform: [[f32; 4]; 4],
    /// The index of the parent bone in the bones collection or [None] if this is a root bone with no parents.
    pub parent_index: Option<usize>,
    // TODO: Make this an Option for clarity?
    pub billboard_type: BillboardType,
    // TODO: Flags?
}

pub mod error {
    use super::*;
    use thiserror::Error;

    /// Errors while creating an [Skel] from [SkelData].
    #[derive(Debug, Error)]
    pub enum Error {
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
}

/// Calculates the transform of `world_transform` relative to `parent_world_transform`.
/// If `parent_world_transform` is [None], a copy of `world_transform` is returned.
/// All matrices are assumed to be in column-major order.
/// # Examples
/// The simplest case is when the parent transform is the identity matrix,
/// so the result is simply the passed in `world_transform`.
/**
```rust
# use ssbh_data::skel_data::calculate_relative_transform;
// A column-major transform with a scale (2, 4, 8) and translation (1, 2, 3).
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
            // Given two world transforms, solve for the relative transform.
            let world = Mat4::from_cols_array_2d(world_transform);
            let parent_world = Mat4::from_cols_array_2d(parent_world_transform);
            (world * parent_world.inverse()).to_cols_array_2d()
        }
        None => *world_transform,
    }
}

fn inv_transform(m: &[[f32; 4]; 4]) -> Matrix4x4 {
    let m = Mat4::from_cols_array_2d(m);
    let inv = m.inverse().to_cols_array_2d();
    Matrix4x4::from_cols_array(&inv)
}

impl TryFrom<SkelData> for Skel {
    type Error = error::Error;

    fn try_from(data: SkelData) -> Result<Self, Self::Error> {
        Self::try_from(&data)
    }
}

impl TryFrom<&SkelData> for Skel {
    type Error = error::Error;

    fn try_from(data: &SkelData) -> Result<Self, Self::Error> {
        let world_transforms = data
            .bones
            .iter()
            .map(|b| data.calculate_world_transform(b))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Skel::V10 {
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
                        billboard_type: b.billboard_type,
                    },
                })
                .collect(),
            world_transforms: world_transforms
                .iter()
                .map(Matrix4x4::from_cols_array)
                .collect(),
            inv_world_transforms: world_transforms.iter().map(inv_transform).collect(),
            transforms: data
                .bones
                .iter()
                .map(|b| Matrix4x4::from_cols_array(&b.transform))
                .collect(),
            inv_transforms: data
                .bones
                .iter()
                .map(|b| inv_transform(&b.transform))
                .collect(),
        })
    }
}

impl From<Skel> for SkelData {
    fn from(skel: Skel) -> Self {
        Self::from(&skel)
    }
}

impl From<&Skel> for SkelData {
    fn from(skel: &Skel) -> Self {
        let (major_version, minor_version) = skel.major_minor_version();
        Self {
            major_version,
            minor_version,
            // TODO: Add additional validation for mismatched array lengths?
            bones: match skel {
                Skel::V10 {
                    bone_entries,
                    transforms,
                    ..
                } => bone_entries
                    .elements
                    .iter()
                    .zip(transforms.elements.iter())
                    .map(|(b, t)| create_bone_data(b, t))
                    .collect(),
            },
        }
    }
}

fn create_bone_data(b: &SkelBoneEntry, transform: &Matrix4x4) -> BoneData {
    BoneData {
        name: b.name.to_string_lossy(),
        transform: transform.to_cols_array(),
        parent_index: b.parent_index.try_into().ok(),
        billboard_type: b.flags.billboard_type,
    }
}

impl SkelData {
    /// Calculates the world transform for `bone` by accumulating the transform with the parents transform recursively.
    /// Returns the resulting matrix in column-major order.
    /// # Examples
    /// For mesh objects with a parent bone and no influences,
    /// the object is transformed by the parent bone's world transform.
    /**
    ```rust
    # use ssbh_data::skel_data::{BoneData, SkelData, BillboardType};
    # let data = SkelData {
    #     major_version: 1,
    #     minor_version: 0,
    #     bones: vec![BoneData {
    #         name: "Head".to_string(),
    #         transform: [[0f32; 4]; 4],
    #         parent_index: None,
    #         billboard_type: BillboardType::Disabled,
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
        let mut transform = Mat4::from_cols_array_2d(&bone.transform);

        // Check for cycles by keeping track of previously visited locations.
        let mut visited = HashSet::new();

        // Accumulate transforms by travelling up the bone hierarchy.
        while let Some(parent_index) = bone.parent_index {
            if !visited.insert(parent_index) {
                return Err(BoneTransformError::CycleDetected {
                    index: parent_index,
                });
            }
            if let Some(parent_bone) = self.bones.get(parent_index) {
                let parent_transform = Mat4::from_cols_array_2d(&parent_bone.transform);
                transform = parent_transform * transform;
                bone = parent_bone;
            } else {
                break;
            }
        }

        // Save the result in column-major order.
        Ok(transform.to_cols_array_2d())
    }
}

/// Errors while calculating [BoneData] transformation matrices.
#[derive(Debug, Error)]
pub enum BoneTransformError {
    #[error(
        "Cyclical bone chains are not supported. A cycle was detected at index {}.",
        index
    )]
    CycleDetected { index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_matrix_relative_eq {
        ($a:expr, $b:expr) => {
            assert!(
                $a.iter()
                    .flatten()
                    .zip($b.iter().flatten())
                    .all(|(a, b)| approx::relative_eq!(a, b, epsilon = 0.0001f32)),
                "Matrices not equal to within 0.0001.\nleft = {:?}\nright = {:?}",
                $a,
                $b
            )
        };
    }

    #[test]
    fn create_skel_no_bones() {
        let data = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: Vec::new(),
        };

        let skel = Skel::try_from(data).unwrap();
        assert!(matches!(
            skel,
            Skel::V10 {
                bone_entries,
                world_transforms,
                inv_world_transforms,
                transforms,
                inv_transforms
            }
            if bone_entries.elements.is_empty()
                && world_transforms.elements.is_empty()
                && inv_world_transforms.elements.is_empty()
                && transforms.elements.is_empty()
                && inv_transforms.elements.is_empty()
        ));
    }

    #[test]
    fn create_skel_mario_three_bone_chain() {
        // The first three bones of /fighter/mario/model/body/c00/model.nusktb.
        // Test for correct accumulation and inverting of transforms.
        let data = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "Trans".to_owned(),
                    transform: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "Rot".to_owned(),
                    transform: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 6.23395, 0.0, 1.0],
                    ],
                    parent_index: Some(0),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "Hip".to_owned(),
                    transform: [
                        [0.0, 0.999626, 0.0273582, 0.0],
                        [0.0, -0.0273582, 0.999626, 0.0],
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, -0.742259, 0.0, 1.0],
                    ],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
            ],
        };

        let skel = Skel::try_from(data).unwrap();
        match skel {
            Skel::V10 {
                bone_entries,
                world_transforms,
                inv_world_transforms,
                transforms,
                inv_transforms,
            } => {
                assert_eq!(
                    bone_entries.elements,
                    vec![
                        SkelBoneEntry {
                            name: "Trans".into(),
                            index: 0,
                            parent_index: -1,
                            flags: SkelEntryFlags {
                                unk1: 1,
                                billboard_type: BillboardType::Disabled
                            },
                        },
                        SkelBoneEntry {
                            name: "Rot".into(),
                            index: 1,
                            parent_index: 0,
                            flags: SkelEntryFlags {
                                unk1: 1,
                                billboard_type: BillboardType::Disabled
                            },
                        },
                        SkelBoneEntry {
                            name: "Hip".into(),
                            index: 2,
                            parent_index: 1,
                            flags: SkelEntryFlags {
                                unk1: 1,
                                billboard_type: BillboardType::Disabled
                            },
                        },
                    ]
                );

                // The transforms should be preserved exactly.
                assert_eq!(
                    transforms.elements,
                    vec![
                        Matrix4x4::from_cols_array(&[
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0, 0.0, 0.0, 1.0],
                        ]),
                        Matrix4x4::from_cols_array(&[
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0, 6.23395, 0.0, 1.0],
                        ]),
                        Matrix4x4::from_cols_array(&[
                            [0.0, 0.999626, 0.0273582, 0.0],
                            [0.0, -0.0273582, 0.999626, 0.0],
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, -0.742259, 0.0, 1.0],
                        ])
                    ]
                );

                // These probably won't match due to rounding errors.
                assert_matrix_relative_eq!(
                    inv_transforms.elements[0].to_cols_array(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ]
                );
                assert_matrix_relative_eq!(
                    inv_transforms.elements[1].to_cols_array(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, -6.23395, 0.0, 1.0],
                    ]
                );
                assert_matrix_relative_eq!(
                    inv_transforms.elements[2].to_cols_array(),
                    [
                        [-0.0, 0.0, 1.0, -0.0],
                        [0.9996254, -0.027358184, 0.0, 0.0],
                        [0.027358184, 0.9996254, 0.0, 0.0],
                        [0.74198097, -0.02030686, 0.0, 1.0]
                    ]
                );

                assert_matrix_relative_eq!(
                    world_transforms.elements[0].to_cols_array(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ]
                );
                assert_matrix_relative_eq!(
                    world_transforms.elements[1].to_cols_array(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 6.23395, 0.0, 1.0],
                    ]
                );
                assert_matrix_relative_eq!(
                    world_transforms.elements[2].to_cols_array(),
                    [
                        [0.0, 0.999626, 0.0273582, 0.0],
                        [0.0, -0.0273582, 0.999626, 0.0],
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 5.49169, 0.0, 1.0],
                    ]
                );

                assert_matrix_relative_eq!(
                    inv_world_transforms.elements[0].to_cols_array(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ]
                );
                assert_matrix_relative_eq!(
                    inv_world_transforms.elements[1].to_cols_array(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, -6.23395, 0.0, 1.0],
                    ]
                );
                assert_matrix_relative_eq!(
                    inv_world_transforms.elements[2].to_cols_array(),
                    [
                        [0.0, 0.0, 1.0, -0.0],
                        [0.9996254, -0.02735818, 0.0, 0.0],
                        [0.027358184, 0.9996254, 0.0, 0.0],
                        [-5.4896326, 0.15024267, 0.0, 1.0],
                    ]
                );
            }
        }
    }

    #[test]
    fn create_bone_data_no_parent() {
        let b = SkelBoneEntry {
            name: "abc".into(),
            index: 2,
            parent_index: -1,
            flags: SkelEntryFlags {
                unk1: 1,
                billboard_type: BillboardType::XYAxisViewPointAligned,
            },
        };

        assert_eq!(
            BoneData {
                name: "abc".to_owned(),
                transform: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0]
                ],
                parent_index: None,
                billboard_type: BillboardType::XYAxisViewPointAligned
            },
            create_bone_data(&b, &Matrix4x4::identity())
        );
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
                billboard_type: BillboardType::Disabled,
            },
        };

        assert_eq!(
            BoneData {
                name: "abc".to_owned(),
                transform: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0]
                ],
                parent_index: None,
                billboard_type: BillboardType::Disabled
            },
            create_bone_data(&b, &Matrix4x4::identity())
        );
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
                name: "root".to_owned(),
                transform,
                parent_index: None,
                billboard_type: BillboardType::Disabled,
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
                name: "root".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
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
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(2),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "d".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(2),
                    billboard_type: BillboardType::Disabled,
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
                    name: "Trans".to_owned(),
                    transform: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "Rot".to_owned(),
                    transform: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 11.241, 0.268775, 1.0],
                    ],
                    parent_index: Some(0),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "Hip".to_owned(),
                    transform: [
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "Waist".to_owned(),
                    transform: [
                        [0.999954, -0.00959458, 0.0, 0.0],
                        [0.00959458, 0.999954, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [1.38263, 0.0, 0.0, 1.0],
                    ],
                    parent_index: Some(2),
                    billboard_type: BillboardType::Disabled,
                },
            ],
        };

        assert_matrix_relative_eq!(
            [
                [0.0, 0.999954, -0.00959458, 0.0],
                [0.0, 0.00959458, 0.999954, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 12.6236, 0.268775, 1.0],
            ],
            data.calculate_world_transform(&data.bones[3]).unwrap()
        );
    }
}
