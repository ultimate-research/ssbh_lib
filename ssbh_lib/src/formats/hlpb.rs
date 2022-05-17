//! The [Hlpb] format stores bone constraints for helper bones.
//! These files typically use the ".nuhlpb" suffix like "model.nuhlpb".
//!
//! Constraints determine the transformations of a bone programatically rather than through explicit key frames.
//! This simplifies the number of bones to manually animate and can improve deformation quality in difficult areas
//! such as elbows, knees, etc.
use crate::{SsbhArray, SsbhString, Vector3, Vector4, Version};
use binread::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

// TODO: What does this constraint do?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Clone)]
pub struct AimConstraint {
    pub name: SsbhString, // formatted as "nuHelperBoneRotateAim{i}" for some i
    pub aim_bone_name1: SsbhString,
    pub aim_bone_name2: SsbhString,
    pub aim_type1: SsbhString, // always "DEFAULT"
    pub aim_type2: SsbhString, // always "DEFAULT"
    pub target_bone_name1: SsbhString,
    pub target_bone_name2: SsbhString,
    pub unk1: i32,  // always 0
    pub unk2: i32,  // always 1
    pub unk3: f32,  // always 1
    pub unk4: f32,  // always 0
    pub unk5: f32,  // always 0
    pub unk6: f32,  // always 0
    pub unk7: f32,  // always 1
    pub unk8: f32,  // always 0
    pub unk9: f32,  // always 0
    pub unk10: f32, // always 0
    pub unk11: f32, // always 0
    pub unk12: f32, // always 0
    pub unk13: f32, // usually -0.5
    pub unk14: f32, // usually -0.5
    pub unk15: f32, // usually -0.5
    pub unk16: f32, // usually 0.5
    pub unk17: f32, // always 0
    pub unk18: f32, // always 0
    pub unk19: f32, // always 0
    pub unk20: f32, // always 0
    pub unk21: f32, // always 0
    pub unk22: f32, // always 0
}

// TODO: Why are there duplicate entries with identical fields?
// TODO: Rename and document these fields.
// TODO: Is this the orient constraint in Maya?
/// Constrains the orientation of a bone to match another bone.
///
/// This is similar to the orient constraint in Autodesk Maya.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, PartialEq, Clone)]
pub struct OrientConstraint {
    pub name: SsbhString, // formatted "nuHelperBoneRotateInterp{i}" for some i
    pub bone_name: SsbhString,
    pub root_bone_name: SsbhString,
    pub parent_bone_name: SsbhString,
    pub driver_bone_name: SsbhString,
    // TODO: Could this be an enum?
    pub unk_type: u32, // 0, 1, 2 (usually 1 or 2)

    /// Controls the effect of the constraint on the XYZ axes.
    ///
    /// A value of `1.0, 1.0, 1.0` fully affects all axes.
    /// A value of `0.0, 0.5, 0.5` affects the Y and Z axes with half intensity.
    pub constraint_axes: Vector3,

    // Applies some sort of additional rotation?
    pub quat1: Vector4,
    pub quat2: Vector4,
    pub range_min: Vector3, // always -180.0, -180.0, -180.0
    pub range_max: Vector3, // always 180.0, 180.0, 180.0
}

/// Helper bones.
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

/// The type of bone constraint.
///
/// These constraint types are similar to the constraints available in Autodesk Maya.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum ConstraintType {
    Aim = 0,
    Orient = 1,
}
