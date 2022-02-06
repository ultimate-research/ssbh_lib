//! The [Modl] format describes the files associated with a model.
//! These files typically use the ".numdlb" or "nusrcmdlb" suffix like "model.numdlb" or "model.nusrcmdlb".
use crate::{RelPtr64, SsbhArray, SsbhString8};
use crate::{SsbhString, Version};
use binread::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// Associates a [MatlEntry](crate::formats::matl::MatlEntryV16) with a [MeshObject](crate::formats::mesh::MeshObject).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct ModlEntry {
    /// The `name` of the [MeshObject](crate::formats::mesh::MeshObject).
    pub mesh_object_name: SsbhString,

    /// The `sub_index` of the [MeshObject](crate::formats::mesh::MeshObject).
    pub mesh_object_sub_index: u64,

    /// The `material_label` of the [MatlEntry](crate::formats::matl::MatlEntryV16).
    pub material_label: SsbhString,
}

/// Defines the mesh, materials, and skeleton used to render a model.
/// Compatible with file version 1.7.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Modl {
    #[br(pre_assert(major_version == 1 && minor_version == 7))]
    V17 {
        /// The name of the model such as "model".
        model_name: SsbhString, // TODO: this might be the source file used to generate the .numdlb
        /// The name of the associated [Skel](crate::formats::skel::Skel) file such as "model.nusktb".
        skeleton_file_name: SsbhString,
        /// The names of the associated [Matl](crate::formats::matl::Matl) files.
        /// Smash ultimate uses a single file such as "model.numatb".
        material_file_names: SsbhArray<SsbhString>,
        /// The name of the optional associated [Anim](crate::formats::anim::Anim) file such as "model.nuanmb".
        animation_file_name: RelPtr64<SsbhString>,
        /// The name of the associated [Mesh](crate::formats::mesh::Mesh) file such as "model.numshb".
        mesh_file_name: SsbhString8,
        /// A collection of material assignments to the [MeshObject](crate::formats::mesh::MeshObject)
        /// in the [Mesh](crate::formats::mesh::Mesh) determined by [mesh_file_name](#structfield.mesh_file_name).
        entries: SsbhArray<ModlEntry>,
    },
}

impl Version for Modl {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Modl::V17 { .. } => (1, 7),
        }
    }
}
