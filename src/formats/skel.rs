use crate::{Matrix4x4, SsbhArray, SsbhString};
use binread::BinRead;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct SkelBoneEntry {
    pub name: SsbhString,
    pub id: i16,
    pub parent_id: i16,
    pub unk_type: u32, // TODO: Can this be an enum?
}

// A heirarchical collection of bones and their associated transforms.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct Skel {
    pub major_version: u16,
    pub minor_version: u16,
    pub bone_entries: SsbhArray<SkelBoneEntry>,
    pub world_transforms: SsbhArray<Matrix4x4>,
    pub inv_world_transforms: SsbhArray<Matrix4x4>,
    pub transforms: SsbhArray<Matrix4x4>,
    pub inv_transforms: SsbhArray<Matrix4x4>,
}
