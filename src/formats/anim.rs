use crate::SsbhArray;
use crate::SsbhString;
use serde::Serialize;

use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};
// TODO: Have an Ssbh<T> struct with an enum for the actual data
// TODO: Dump all distinct magics/extensions for SSBH
#[derive(Serialize, BinRead, Debug)]
pub struct Anim {
    major_version: u16,
    minor_version: u16,
}
