use crate::{Matrix4x4, SsbhArray, SsbhString};
use binread::BinRead;

use modular_bitfield::bitfield;
#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

use modular_bitfield::prelude::*;

#[bitfield]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Copy, Clone, PartialEq)]
#[br(map = Self::from_bytes)]
pub struct SkelEntryFlags {
    pub unk1: bool,
    #[skip]
    unused: B7,
    pub unk2: bool,
    pub unk3: bool,
    pub unk4: bool,
    pub unk5: bool,
    #[skip]
    unused: B20,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
pub struct SkelBoneEntry {
    pub name: SsbhString,
    pub index: i16,
    pub parent_index: i16,
    pub flags: SkelEntryFlags,
}

/// A heirarchical collection of bones and their associated transforms.
/// The bone entries and transforms are stored in parallel arrays, 
/// so each bone entry has corresponding transforms at the same position in each array.
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
