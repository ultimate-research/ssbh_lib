//! The [Nlst] format stores a collection of file names to load into the game.
//! These files typically use the ".nulstb" suffix like "main.nulstb".
use crate::{SsbhArray, SsbhString, Version};
use binrw::BinRead;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// A container of file names. Compatible with file version 1.0.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Nlst {
    #[br(pre_assert(major_version == 1 && minor_version == 0))]
    V10 { file_names: SsbhArray<SsbhString> },
}

impl Version for Nlst {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Nlst::V10 { .. } => (1, 0),
        }
    }
}
