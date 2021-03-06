use crate::{SsbhArray, SsbhString, Vector3, Vector4};
use binread::BinRead;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct HlpbRotateAim {
    pub name: SsbhString,
    pub aim_bone_name1: SsbhString,
    pub aim_bone_name2: SsbhString,
    pub aim_type1: SsbhString,
    pub aim_type2: SsbhString,
    pub target_bone_name1: SsbhString,
    pub target_bone_name2: SsbhString,
    pub unk1: i32,
    pub unk2: i32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: f32,
    pub unk7: f32,
    pub unk8: f32,
    pub unk9: f32,
    pub unk10: f32,
    pub unk11: f32,
    pub unk12: f32,
    pub unk13: f32,
    pub unk14: f32,
    pub unk15: f32,
    pub unk16: f32,
    pub unk17: f32,
    pub unk18: f32,
    pub unk19: f32,
    pub unk20: f32,
    pub unk21: f32,
    pub unk22: f32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct HlpbRotateInterpolation {
    pub name: SsbhString,
    pub bone_name: SsbhString,
    pub root_bone_name: SsbhString,
    pub parent_bone_name: SsbhString,
    pub driver_bone_name: SsbhString,
    // TODO: Could this be an enum?
    pub unk_type: u32,
    pub aoi: Vector3,
    pub quat1: Vector4,
    pub quat2: Vector4,
    pub range_min: Vector3,
    pub range_max: Vector3,
}

/// Helper bones.
/// Compatible with file version 1.1.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Hlpb {
    pub major_version: u16,
    pub minor_version: u16,
    pub aim_entries: SsbhArray<HlpbRotateAim>,
    pub interpolation_entries: SsbhArray<HlpbRotateInterpolation>,
    pub list1: SsbhArray<i32>,
    pub list2: SsbhArray<i32>,
}
