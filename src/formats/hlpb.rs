use crate::{Matrix4x4, SsbhArray, SsbhString, Vector3, Vector4};
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct HlpbRotateAim {
    name: SsbhString,
    aim_bone_name1: SsbhString,
    aim_bone_name2: SsbhString,
    aim_type1: SsbhString,
    aim_type2: SsbhString,
    target_bone_name1: SsbhString,
    target_bone_name2: SsbhString,
    unk1: i32,
    unk2: i32,
    unk3: f32,
    unk4: f32,
    unk5: f32,
    unk6: f32,
    unk7: f32,
    unk8: f32,
    unk9: f32,
    unk10: f32,
    unk11: f32,
    unk12: f32,
    unk13: f32,
    unk14: f32,
    unk15: f32,
    unk16: f32,
    unk17: f32,
    unk18: f32,
    unk19: f32,
    unk20: f32,
    unk21: f32,
    unk22: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct HlpbRotateInterpolation {
    name: SsbhString,
    bone_name: SsbhString,
    root_bone_name: SsbhString,
    parent_bone_name: SsbhString,
    driver_bone_name: SsbhString,
    // TODO: Could this be an enum?
    unk_type: u32,
    aoi: Vector3,
    quat1: Vector4,
    quat2: Vector4,
    range_min: Vector3,
    range_max: Vector3,
}

#[derive(Serialize, BinRead, Debug)]
pub struct Hlpb {
    major_version: u16,
    minor_version: u16,
    aim_entries: SsbhArray<HlpbRotateAim>,
    interpolation_entries: SsbhArray<HlpbRotateInterpolation>,
    list1: SsbhArray<i32>,
    list2: SsbhArray<i32>,
}
