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

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct SkelBoneEntry {
    pub name: SsbhString,
    pub index: i16,
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
