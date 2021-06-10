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
#[ssbhwrite(pad_after = 2)]
pub struct SkelEntryFlags {
    pub unk1: u8,
    #[br(pad_after = 2)]
    pub billboard_type: BillboardType,
}

/// A named bone.
/// [index](#structfield.index) and [parent_index](#structfield.parent_index) determine the skeleton's bone heirarchy.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SkelBoneEntry {
    /// The name of the bone.
    pub name: SsbhString,
    // TODO: Should this be a u16 instead?
    /// The index of this [SkelBoneEntry] in [bone_entries](struct.Skel.html.#structfield.bone_entries).
    pub index: u16,
    /// The index of the parent [SkelBoneEntry] in [bone_entries](struct.Skel.html.#structfield.bone_entries) or `-1` if there is no parent.
    pub parent_index: i16,
    pub flags: SkelEntryFlags,
}

/// An ordered, hierarchical collection of bones and their associated transforms.
/// Each bone entry has transformation matrices stored at the corresponding locations in the transform arrays.
/// The [transforms](#structfield.transforms) array can be used to calculate the remaining arrays.
/// Compatible with file version 1.0.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Skel {
    pub major_version: u16,
    pub minor_version: u16,
    /// A skeleton consisting of a heirarchy of bones.
    pub bone_entries: SsbhArray<SkelBoneEntry>,
    /// The resulting of accumulating the transformation in [transforms](#structfield.transforms) for each bone in 
    /// [bone_entries](#structfield.bone_entries) with its parents transformation recursively.
    /// This defines each bone's transformation in world space.
    pub world_transforms: SsbhArray<Matrix4x4>,
    /// The inverses of the matrices in [world_transforms](#structfield.world_transforms).
    pub inv_world_transforms: SsbhArray<Matrix4x4>,
    /// The associated transformation for each of the bones in [bone_entries](#structfield.bone_entries).
    pub transforms: SsbhArray<Matrix4x4>,
    /// The inverses of the matrices in [transforms](#structfield.transforms).
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
