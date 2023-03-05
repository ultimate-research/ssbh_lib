//! The [Skel] format stores the model's skeleton used for skeletal animations.
//! These files typically use the ".nusktb" suffix like "model.nusktb".
//! Animations are often stored in [Anim](crate::formats::anim::Anim) files that override the [Skel] file's bone transforms.
//! [Skel] files are linked with [Mesh](crate::formats::mesh::Mesh) and [Matl](crate::formats::matl::Matl) files using a [Modl](crate::formats::modl::Modl) file.

use crate::{Matrix4x4, SsbhArray, SsbhString, Version};
use binrw::BinRead;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "strum")]
use strum::{Display, EnumString, EnumIter, FromRepr};

use ssbh_write::SsbhWrite;

/// An ordered, hierarchical collection of bones and their associated transforms.
/// Each bone entry has transformation matrices stored at the corresponding locations in the transform arrays.
/// The [transforms](#structfield.transforms) array can be used to calculate the remaining arrays.
/// Compatible with file version 1.0.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Clone)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Skel {
    #[br(pre_assert(major_version == 1 && minor_version == 0))]
    V10 {
        /// A skeleton consisting of an ordered hierarchy of bones.
        bone_entries: SsbhArray<SkelBoneEntry>,
        /// The transformation in world space for each bone in
        /// [bone_entries](#structfield.bone_entries).
        /// The world space transform for a bone is calculated by accumulating the transformations in [transforms](#structfield.transforms)
        /// with the transformation of the bone's parent recursively.
        world_transforms: SsbhArray<Matrix4x4>,
        /// The inverses of the matrices in [world_transforms](#structfield.world_transforms).
        inv_world_transforms: SsbhArray<Matrix4x4>,
        /// The associated transformation for each of the bones in [bone_entries](#structfield.bone_entries) relative to its parent's world transform.
        /// If the bone has no parent, this is equivalent to the corresponding value in [world_transforms](#structfield.world_transforms).
        transforms: SsbhArray<Matrix4x4>,
        /// The inverses of the matrices in [transforms](#structfield.transforms).
        inv_transforms: SsbhArray<Matrix4x4>,
    },
}

impl Version for Skel {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Skel::V10 { .. } => (1, 0),
        }
    }
}

/// A named bone.
/// [index](#structfield.index) and [parent_index](#structfield.parent_index) determine the skeleton's bone hierarchy.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq, Eq)]
pub struct SkelBoneEntry {
    /// The name of the bone.
    pub name: SsbhString,
    /// The index of this [SkelBoneEntry] in [bone_entries](struct.Skel.html.#structfield.bone_entries).
    pub index: u16,
    /// The index of the parent [SkelBoneEntry] in [bone_entries](struct.Skel.html.#structfield.bone_entries) or `-1` if there is no parent.
    pub parent_index: i16,
    pub flags: SkelEntryFlags,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq, Eq)]
#[ssbhwrite(pad_after = 2)]
pub struct SkelEntryFlags {
    pub unk1: u8, // TODO: usually 1?
    #[br(pad_after = 2)]
    pub billboard_type: BillboardType,
}

/// Billboarding reorients a [MeshObject](crate::formats::mesh::MeshObject) parented to a bone based on the camera.
/// This effect is commonly used for 2D sprites for particles and distant objects like trees.
///
/// View plane alignment does not take into account the object's position.
/// This means a rectangular sprite will not appear distorted regardless of its position on screen.
/// For aligning a rectangular sprite to the screen with no distortion, use [BillboardType::XYAxisViewPlaneAligned].
///
/// Viewpoint oriented billboards do take into account the object's position.
/// This means a rectangular sprite positioned on the edges of the screen will appear
/// slightly warped due to perspective projection.
/// For example, [BillboardType::XYAxisViewPointAligned] will appear to point towards the
/// camera axes at the center of the screen in pause mode in Smash Ultimate.
// https://www.flipcode.com/archives/Billboarding-Excerpt_From_iReal-Time_Renderingi_2E.shtml
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumIter, EnumString)
)]
#[derive(Debug, BinRead, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u8))]
#[ssbhwrite(repr(u8))]
pub enum BillboardType {
    /// Disable billboarding for this bone.
    Disabled = 0,
    /// The bone rotates along the X-axis to face the camera.
    XAxisViewPointAligned = 1, // effected by cam pos + rot
    /// The bone rotates along the Y-axis to face the camera.
    YAxisViewPointAligned = 2, // effected by cam pos + rot
    /// Likely identical to [BillboardType::Disabled]
    Unk3 = 3,
    /// The bone rotates along the X and Y axes to face the camera.
    XYAxisViewPointAligned = 4, // effected by cam pos + rot
    /// The bone rotates along the Y-axis to face the camera.
    YAxisViewPlaneAligned = 6, // effected by cam rot
    /// The bone rotates along the X and Y axes to face the camera.
    XYAxisViewPlaneAligned = 8, // effected by cam rot
}
