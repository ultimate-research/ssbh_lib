//! The [Hlpb] format stores bone constraints for helper bones.
//! These files typically use the ".nuhlpb" suffix like "model.nuhlpb".
//!
//! Constraints determine the transformations of a bone programatically rather than through explicit key frames.
//! This simplifies the number of bones to manually animate and can improve deformation quality in difficult areas
//! such as elbows, knees, etc.
use crate::{SsbhArray, SsbhString, Vector3, Vector4, Version};
use binrw::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// Helper bone constraints.
/// Compatible with file version 1.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Clone)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Hlpb {
    #[br(pre_assert(major_version == 1 && minor_version == 1))]
    V11 {
        aim_constraints: SsbhArray<AimConstraint>,
        orient_constraints: SsbhArray<OrientConstraint>,

        /// The index of each constraint in [aim_entries](enum.Hlpb.html#variant.V11.field.aim_entries)
        /// and the index of each constraint in [orient_constraints](enum.Hlpb.html#variant.V11.field.orient_constraints).
        ///
        /// Two aim constraints and three orient constraints would have indices `[0, 1, 0, 1, 2]`.
        constraint_indices: SsbhArray<u32>,

        /// The type of each constraint using the same ordering as [constraint_indices](enum.Hlpb.html#variant.V11.field.constraint_indices).
        ///
        /// Two aim constraints and three orient constraints would have indices `[0, 0, 1, 1, 1]`.
        constraint_types: SsbhArray<ConstraintType>,
    },
}

impl Version for Hlpb {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Hlpb::V11 { .. } => (1, 1),
        }
    }
}

// TODO: Fix these field names to use standard conventions.
// TODO: Clarify which bone is the source and target.
/// Constrains a bone's local axis to point towards another bone.
///
/// This is similar to the aim constraint in Autodesk Maya.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Clone)]
pub struct AimConstraint {
    /// The name of the constraint like `"nuHelperBoneRotateAim1"`.
    pub name: SsbhString,
    pub aim_bone_name1: SsbhString,
    pub aim_bone_name2: SsbhString,
    pub aim_type1: SsbhString, // always "DEFAULT"
    pub aim_type2: SsbhString, // always "DEFAULT"
    pub target_bone_name1: SsbhString,
    pub target_bone_name2: SsbhString,
    pub unk1: u32, // always 0
    pub unk2: u32, // always 1
    /// The axes in the bone's local space to constrain. This is usally X+ `(1, 0, 0)`.
    pub aim: Vector3,
    pub up: Vector3,
    // Applies some sort of additional rotation?
    pub quat1: Vector4,
    pub quat2: Vector4,
    pub unk17: f32, // always 0
    pub unk18: f32, // always 0
    pub unk19: f32, // always 0
    pub unk20: f32, // always 0
    pub unk21: f32, // always 0
    pub unk22: f32, // always 0
}

/// Constrains the orientation of a target bone to a source bone by interpolating XYZ rotations.
///
/// This is similar to the orient constraint in Autodesk Maya.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Clone)]
pub struct OrientConstraint {
    /// The name of the constraint like `"nuHelperBoneRotateInterp1"`.
    pub name: SsbhString,
    pub parent_bone1_name: SsbhString,
    pub parent_bone2_name: SsbhString,
    /// The name of the bone to interpolate rotation with the target bone.
    pub source_bone_name: SsbhString,
    /// The name of the constrained bone.
    pub target_bone_name: SsbhString,
    // TODO: Could this be an enum?
    // type 1 needs the root/bone name set properly?
    pub unk_type: u32, // 0, 1, 2 (usually 1 or 2)

    /// Interpolation factors for the XYZ rotation axes.
    ///
    /// Quaternion rotations are decomposed to euler angles before interpolating.
    /// Rotation axes are interpolated individually and applied in the order X -> Y -> Z.
    ///
    /// A value of `1.0` for an axis uses the source bone rotation.
    /// A value of `0.0` for an axis uses the target bone rotation.
    pub constraint_axes: Vector3,

    // Applies some sort of additional rotation?
    pub quat1: Vector4,
    pub quat2: Vector4,
    pub range_min: Vector3, // always -180.0, -180.0, -180.0
    pub range_max: Vector3, // always 180.0, 180.0, 180.0
}

/// The type of bone constraint for entries in [constraint_types](enum.Hlpb.html#variant.V11.field.constraint_types).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum ConstraintType {
    /// [AimConstraint]
    Aim = 0,
    /// [OrientConstraint]
    Orient = 1,
}
