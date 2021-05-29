//! The [Skel] format stores the model's skeleton used for skeletal animations.
//! These files typically use the ".nusktb" suffix like "model.nusktb".
//! Animations are often stored in [Anim](crate::formats::anim::Anim) files that override the [Skel] file's bone transforms.
//! [Skel] files are linked with [Mesh](crate::formats::mesh::Mesh) and [Matl](crate::formats::matl::Matl) files using a [Modl](crate::formats::modl::Modl) file.

use crate::{Matrix4x4, SsbhArray, SsbhString};
use binread::BinRead;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};
use ssbh_write_derive::SsbhWrite;

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SkelEntryFlags {
    pub unk1: u8,
    pub billboard_type: BillboardType,
    #[cfg_attr(feature = "derive_serde", serde(skip))]
    pub padding: u16,
}

/// A named bone.
/// [index](#structfield.index) and [parent_index](#structfield.parent_index) determine the skeleton's bone heirarchy.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SkelBoneEntry {
    /// The name of the bone.
    pub name: SsbhString,
    /// The index of this [SkelBoneEntry] in [bone_entries](struct.Skel.html.#structfield.bone_entries).
    pub index: i16,
    /// The index of the parent [SkelBoneEntry] in [bone_entries](struct.Skel.html.#structfield.bone_entries) or `-1` if there is no parent.
    pub parent_index: i16,
    pub flags: SkelEntryFlags,
}

/// A hierarchical collection of bones and their associated transforms.
/// The bone entries and transforms are stored in parallel arrays,
/// so each bone entry has corresponding transforms at the same position in each array.
/// Compatible with file version 1.0.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Skel {
    pub major_version: u16,
    pub minor_version: u16,
    /// A skeleton consisting of a heirarchy of bones.
    pub bone_entries: SsbhArray<SkelBoneEntry>,
    pub world_transforms: SsbhArray<Matrix4x4>,
    pub inv_world_transforms: SsbhArray<Matrix4x4>,
    pub transforms: SsbhArray<Matrix4x4>,
    pub inv_transforms: SsbhArray<Matrix4x4>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u8))]
pub enum BillboardType {
    None = 0,
    XAxialViewpoint = 1,
    YAxialViewpoint = 2,
    Unused = 3,
    XYAxialViewpoint = 4,
    YAxial = 6,
    XYAxial = 8,
}
