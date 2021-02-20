use ssbh_lib::{
    formats::{mesh::MeshObject, skel::Skel},
    Matrix4x4,
};

/// Finds the corresponding transform matrix for `mesh_object` in `skel` based on the parent bone name for `mesh_object`. 
pub fn get_single_bind_transform<'a>(skel: &'a Skel, mesh_object: &MeshObject) -> Option<&'a Matrix4x4> {
    let index = skel.bone_entries.elements.iter().position(|b| {
        match (b.name.get_string(), mesh_object.parent_bone_name.get_string()) {
            (Some(bone_name), Some(mesh_bone_name)) => bone_name == mesh_bone_name,
            _ => false,
        }
    })?;
    skel.transforms.elements.get(index)
}