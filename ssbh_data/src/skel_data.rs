use glam::Mat4;
use ssbh_lib::{formats::skel::Skel, Matrix4x4};

// TODO: Include major and minor version?
pub struct SkelData {
    pub bones: Vec<SkelBoneEntryData>,
}

pub struct SkelBoneEntryData {
    pub name: String,
    pub transform: [[f32; 4]; 4],
    pub world_transform: [[f32; 4]; 4],
    pub parent_index: Option<usize>, // TODO: Flags?
}

fn matrix4x4_to_glam(matrix: &Matrix4x4) -> Mat4 {
    // Glam uses column-major instead of row-major order.
    Mat4::from_cols_array_2d(&[
        [matrix.row1.x, matrix.row1.y, matrix.row1.z, matrix.row1.w],
        [matrix.row2.x, matrix.row2.y, matrix.row2.z, matrix.row2.w],
        [matrix.row3.x, matrix.row3.y, matrix.row3.z, matrix.row3.w],
        [matrix.row4.x, matrix.row4.y, matrix.row4.z, matrix.row4.w],
    ])
    .transpose()
}

// TODO: Should this work with skel data instead?

/// Calculates the combined single bind transform matrix, which determines the resting position of a single bound mesh object.
/// Each bone transform is multiplied by its parents transform recursively starting with `parent_bone_name` until a root node is reached.
/// Returns the resulting matrix in row major order or `None` if no matrix could be calculated for the given `parent_bone_name`.
pub fn calculate_single_bind_transform(
    skel: &Skel,
    parent_bone_name: &str,
) -> Option<[[f32; 4]; 4]> {
    // Attempt to find the parent containing the single bind transform.
    let index = skel.bone_entries.elements.iter().position(|b| {
        if let Some(bone_name) = b.name.to_str() {
            bone_name == parent_bone_name
        } else {
            false
        }
    })?;

    // Accumulate transforms of a bone with its parent recursively.
    let mut transform = matrix4x4_to_glam(skel.transforms.elements.get(index)?);
    let mut parent_id = skel.bone_entries.elements[index].parent_index;
    while parent_id != -1 {
        let parent_transform = skel.transforms.elements.get(parent_id as usize)?;
        let parent_transform = matrix4x4_to_glam(parent_transform);

        transform = transform.mul_mat4(&parent_transform);

        parent_id = skel
            .bone_entries
            .elements
            .get(parent_id as usize)?
            .parent_index;
    }

    // Save the result in row-major order.
    Some(transform.transpose().to_cols_array_2d())
}
