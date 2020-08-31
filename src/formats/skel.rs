use crate::{Matrix4x4, SsbhArray, SsbhString};
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct SkelBoneEntry {
    name: SsbhString,
    id: u16,
    parent_id: u16,
    // TODO: Can this be an enum?
    unk_type: u32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct Skel {
    major_version: u16,
    minor_version: u16,
    bone_entries: SsbhArray<SkelBoneEntry>,
    world_transforms: SsbhArray<Matrix4x4>,
    inv_world_transforms: SsbhArray<Matrix4x4>,
    transforms: SsbhArray<Matrix4x4>,
    inv_transforms: SsbhArray<Matrix4x4>,
}
