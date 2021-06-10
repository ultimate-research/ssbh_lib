use std::convert::TryInto;

use glam::Mat4;
use ssbh_lib::{
    formats::skel::{BillboardType, Skel, SkelBoneEntry, SkelEntryFlags},
    Matrix4x4,
};

// TODO: Include major and minor version?
pub struct SkelData {
    pub bones: Vec<BoneData>,
}

pub struct BoneData {
    pub name: String,
    pub transform: [[f32; 4]; 4],
    pub parent_index: Option<usize>,
    // TODO: Flags?
}

// TODO: Can this fail?
pub fn create_skel(data: &SkelData) -> Skel {
    // TODO: Support other versions?
    let inv_transform = |m| {
        let m = mat4_from_row2d(m);
        let inv = m.inverse().transpose().to_cols_array_2d();
        Matrix4x4::from_rows_array(&inv)
    };

    let world_transforms: Vec<_> = data
        .bones
        .iter()
        .map(|b| data.calculate_world_transform(b))
        .collect();

    // TODO: Add a test for this with a few bones.
    Skel {
        major_version: 1,
        minor_version: 0,
        bone_entries: data
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| SkelBoneEntry {
                name: b.name.clone().into(),
                index: i as i16,
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
        world_transforms: world_transforms
            .iter()
            .map(|m| Matrix4x4::from_rows_array(&m))
            .collect::<Vec<Matrix4x4>>()
            .into(),
        inv_world_transforms: world_transforms
            .iter()
            .map(|m| inv_transform(m))
            .collect::<Vec<Matrix4x4>>()
            .into(),
        transforms: data
            .bones
            .iter()
            .map(|b| Matrix4x4::from_rows_array(&b.transform))
            .collect::<Vec<Matrix4x4>>()
            .into(),
        inv_transforms: data
            .bones
            .iter()
            .map(|b| inv_transform(&b.transform))
            .collect::<Vec<Matrix4x4>>()
            .into(),
    }
}

impl From<&Skel> for SkelData {
    fn from(skel: &Skel) -> Self {
        Self {
            bones: skel
                .bone_entries
                .elements
                .iter()
                .map(|b| create_bone_data(skel, b))
                .collect(),
        }
    }
}

fn create_bone_data(skel: &Skel, b: &SkelBoneEntry) -> BoneData {
    BoneData {
        name: b.name.to_string_lossy(),
        // TODO: This may panic.
        transform: skel.transforms.elements[b.index as usize].to_rows_array(),
        // TODO: Test that this works?
        // TODO: Should all negative indices be treated as no parent?
        parent_index: b.parent_index.try_into().ok(),
    }
}

fn mat4_from_row2d(elements: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(&elements).transpose()
}

impl SkelData {
    /// Calculates the combined single bind transform matrix for `bone`, which determines the resting position of a single bound mesh object.
    /// Returns the resulting matrix in row-major order or `None` if the parent could not be found.
    ///```rust
    ///# use ssbh_data::skel_data::{BoneData, SkelData};
    ///# let data = SkelData {
    ///#     bones: vec![BoneData {
    ///#         name: "root".to_string(),
    ///#         transform: [[0f32; 4]; 4],
    ///#         parent_index: None,
    ///#     }],
    ///# };
    ///let transform = data.calculate_single_bind_transform(&data.bones[0]);
    ///```
    pub fn calculate_single_bind_transform(&self, bone: &BoneData) -> Option<[[f32; 4]; 4]> {
        // Find the parent's transform.
        let mut bone = self.bones.get(bone.parent_index?)?;
        let mut transform = mat4_from_row2d(&bone.transform);

        // Accumulate transforms by travelling up the bone heirarchy.
        while let Some(parent_index) = bone.parent_index {
            bone = self.bones.get(parent_index)?;

            let parent_transform = mat4_from_row2d(&bone.transform);
            transform = transform.mul_mat4(&parent_transform);
        }

        // Save the result in row-major order.
        Some(transform.transpose().to_cols_array_2d())
    }

    /// Calculates the world transform for `bone` by accumulating the transform with the parents transform recursively.
    /// Unlike [calculate_single_bind_transform](#method.calculate_single_bind_transform), this always includes the 
    /// transform of `bone` even if `bone` has no parent. Returns the resulting matrix in row-major order.
    ///```rust
    ///# use ssbh_data::skel_data::{BoneData, SkelData};
    ///# let data = SkelData {
    ///#     bones: vec![BoneData {
    ///#         name: "root".to_string(),
    ///#         transform: [[0f32; 4]; 4],
    ///#         parent_index: None,
    ///#     }],
    ///# };
    ///let world_transform = data.calculate_world_transform(&data.bones[0]);
    ///```
    pub fn calculate_world_transform(&self, bone: &BoneData) -> [[f32; 4]; 4] {
        match self.calculate_single_bind_transform(bone) {
            Some(parent_world_transform) => {
                // This is similar to the single bind transform but includes the current transform.
                let parent_world_transform = mat4_from_row2d(&parent_world_transform);

                let transform = bone.transform;
                let transform = mat4_from_row2d(&transform);

                let world_transform = (&transform).mul_mat4(&parent_world_transform);
                world_transform.transpose().to_cols_array_2d()
            }
            None => bone.transform,
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::relative_eq;

    use super::*;

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
            [0f32, 1f32, 2f32, 3f32],
            [4f32, 5f32, 6f32, 7f32],
            [8f32, 9f32, 10f32, 11f32],
            [12f32, 13f32, 14f32, 15f32],
        ];
        assert!(matrices_are_relative_eq(transform, transform));
    }

    #[test]
    fn world_transform_no_parent() {
        // Use unique values to make sure the matrix is correct.
        let transform = [
            [0f32, 1f32, 2f32, 3f32],
            [4f32, 5f32, 6f32, 7f32],
            [8f32, 9f32, 10f32, 11f32],
            [12f32, 13f32, 14f32, 15f32],
        ];

        let data = SkelData {
            bones: vec![BoneData {
                name: "root".to_string(),
                transform,
                parent_index: None,
            }],
        };

        assert_eq!(transform, data.calculate_world_transform(&data.bones[0]));
    }

    #[test]
    fn world_transform_multi_parent_chain() {
        // Cloud c00 model.nusktb.
        let data = SkelData {
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
            data.calculate_world_transform(&data.bones[3]),
        ));
    }

    #[test]
    fn single_bind_transform_no_parent() {
        let data = SkelData {
            bones: vec![BoneData {
                name: "root".to_string(),
                transform: [[0f32; 4]; 4],
                parent_index: None,
            }],
        };

        assert_eq!(None, data.calculate_single_bind_transform(&data.bones[0]));
    }

    #[test]
    fn single_bind_transform_single_parent() {
        // Use unique values to make sure the matrix is correct.
        let transform = [
            [0f32, 1f32, 2f32, 3f32],
            [4f32, 5f32, 6f32, 7f32],
            [8f32, 9f32, 10f32, 11f32],
            [12f32, 13f32, 14f32, 15f32],
        ];
        let data = SkelData {
            bones: vec![
                BoneData {
                    name: "parent".to_string(),
                    transform,
                    parent_index: None,
                },
                BoneData {
                    name: "child".to_string(),
                    transform,
                    parent_index: Some(0),
                },
            ],
        };

        assert_eq!(
            Some(transform),
            data.calculate_single_bind_transform(&data.bones[1])
        );
    }

    #[test]
    fn single_bind_transform_multi_parent_chain() {
        // Use non symmetric matrices to check the transpose.
        let data = SkelData {
            bones: vec![
                BoneData {
                    name: "root".to_string(),
                    transform: [
                        [1f32, 0f32, 0f32, 0f32],
                        [0f32, 2f32, 0f32, 0f32],
                        [0f32, 0f32, 3f32, 1f32],
                        [0f32, 0f32, 0f32, 1f32],
                    ],
                    parent_index: Some(1),
                },
                BoneData {
                    name: "parent".to_string(),
                    transform: [
                        [1f32, 0f32, 0f32, 0f32],
                        [0f32, 2f32, 0f32, 0f32],
                        [0f32, 0f32, 3f32, 1f32],
                        [0f32, 0f32, 0f32, 1f32],
                    ],
                    parent_index: Some(2),
                },
                BoneData {
                    name: "grandparent".to_string(),
                    transform: [
                        [1f32, 0f32, 0f32, 0f32],
                        [0f32, 2f32, 0f32, 0f32],
                        [0f32, 0f32, 3f32, 0f32],
                        [0f32, 0f32, 0f32, 4f32],
                    ],
                    parent_index: None,
                },
            ],
        };

        assert_eq!(
            Some([
                [1f32, 0f32, 0f32, 0f32],
                [0f32, 4f32, 0f32, 0f32],
                [0f32, 0f32, 9f32, 4f32],
                [0f32, 0f32, 0f32, 4f32]
            ]),
            data.calculate_single_bind_transform(&data.bones[0])
        );
    }
}
