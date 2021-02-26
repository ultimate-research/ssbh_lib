use ndarray::{arr2, Array2};
use ssbh_lib::{
    formats::{mesh::MeshObject, skel::Skel},
    Matrix4x4,
};

fn matrix4x4_to_array2(matrix: &Matrix4x4) -> Array2<f32> {
    arr2(&[
        [matrix.row1.x, matrix.row1.y, matrix.row1.z, matrix.row1.w],
        [matrix.row2.x, matrix.row2.y, matrix.row2.z, matrix.row2.w],
        [matrix.row3.x, matrix.row3.y, matrix.row3.z, matrix.row3.w],
        [matrix.row4.x, matrix.row4.y, matrix.row4.z, matrix.row4.w],
    ])
}

/// Calculates the combined transform matrix for `mesh_object` in `skel` based on the parent bone name for `mesh_object`.
/// Each bone transform is multiplied by its parents transform until the root is reached.
/// The result is returned in row major order with each row being `(f32,f32,f32,f32)`.
pub fn get_single_bind_transform<'a>(
    skel: &'a Skel,
    mesh_object: &MeshObject,
) -> Option<[(f32, f32, f32, f32); 4]> {
    // TODO: This whole thing is kind of messy.
    let index = skel.bone_entries.elements.iter().position(|b| {
        match (
            b.name.get_string(),
            mesh_object.parent_bone_name.get_string(),
        ) {
            (Some(bone_name), Some(mesh_bone_name)) => bone_name == mesh_bone_name,
            _ => false,
        }
    })?;

    // Accumulate transforms of a bone with its parent recursively.
    let mut transform = matrix4x4_to_array2(skel.transforms.elements.get(index)?);
    let mut parent_id = skel.bone_entries.elements[index].parent_index;
    while parent_id != -1 {
        let parent_transform =
            matrix4x4_to_array2(skel.transforms.elements.get(parent_id as usize)?);
        transform = transform.dot(&parent_transform);
        parent_id = skel
            .bone_entries
            .elements
            .get(parent_id as usize)?
            .parent_index;
    }

    Some([
        (
            transform[[0, 0]],
            transform[[0, 1]],
            transform[[0, 2]],
            transform[[0, 3]],
        ),
        (
            transform[[1, 0]],
            transform[[1, 1]],
            transform[[1, 2]],
            transform[[1, 3]],
        ),
        (
            transform[[2, 0]],
            transform[[2, 1]],
            transform[[2, 2]],
            transform[[2, 3]],
        ),
        (
            transform[[3, 0]],
            transform[[3, 1]],
            transform[[3, 2]],
            transform[[3, 3]],
        ),
    ])
}
