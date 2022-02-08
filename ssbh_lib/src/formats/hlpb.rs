use crate::{SsbhArray, SsbhString, Vector3, Vector4, Version};
use binread::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RotateAim {
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct RotateInterpolation {
    pub name: SsbhString, // formatted "nuHelperBoneRotateInterp{i}" for some i
    pub bone_name: SsbhString,
    pub root_bone_name: SsbhString,
    pub parent_bone_name: SsbhString,
    pub driver_bone_name: SsbhString,
    // TODO: Could this be an enum?
    pub unk_type: u32, // 0, 1, 2 (usually 1 or 2)
    pub aoi: Vector3,  // components 0.0 to 1.0 or slightly above
    pub quat1: Vector4,
    pub quat2: Vector4,
    pub range_min: Vector3, // always -180.0, -180.0, -180.0
    pub range_max: Vector3, // always 180.0, 180.0, 180.0
}

/// Helper bones.
/// Compatible with file version 1.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Hlpb {
    #[br(pre_assert(major_version == 1 && minor_version == 1))]
    V11 {
        aim_entries: SsbhArray<RotateAim>,
        interpolation_entries: SsbhArray<RotateInterpolation>,
        // TODO: Why are these usually empty?
        // TODO: Why is aim_entries.len() + interpolation_entries.len() == list1.len() == list2.len()?
        list1: SsbhArray<u32>, // indices of some kind?
        list2: SsbhArray<u32>, // elements always 0 or 1
    },
}

impl Version for Hlpb {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Hlpb::V11 { .. } => (1, 1),
        }
    }
}
