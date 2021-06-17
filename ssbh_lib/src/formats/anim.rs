use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use ssbh_write_derive::SsbhWrite;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

use binread::BinRead;

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimTrack {
    pub name: SsbhString,
    pub flags: TrackFlags,
    pub frame_count: u32,
    pub unk3: u32,
    pub data_offset: u32,
    pub data_size: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimNode {
    pub name: SsbhString,
    pub tracks: SsbhArray<AnimTrack>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimGroup {
    pub anim_type: AnimType,
    pub nodes: SsbhArray<AnimNode>,
}

/// Skeletal and material animation.
/// Compatible with file version 2.0 and 2.1.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Anim {
    pub major_version: u16,
    pub minor_version: u16,
    #[br(args(major_version, minor_version))]
    pub header: AnimHeader,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum AnimHeader {
    #[br(pre_assert(major_version == 1 && minor_version == 2))]
    HeaderV1(AnimHeaderV12),

    #[br(pre_assert(major_version == 2 && minor_version == 0))]
    HeaderV20(AnimHeaderV20),

    #[br(pre_assert(major_version == 2 && minor_version == 1))]
    HeaderV21(AnimHeaderV21)
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimHeaderV12 {
    pub name: SsbhString,
    pub unk1: u32,
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.   
    pub final_frame_index: f32,
    pub unk2: u64,
    pub unk3: SsbhArray<UnkStruct1>,
    pub buffers: SsbhArray<SsbhByteBuffer>
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkStruct1 {
    pub unk1: SsbhString,
    pub unk2: u64,
    pub unk3: SsbhArray<UnkStruct2>
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkStruct2 {
    // TODO: property name?
    pub unk1: SsbhString,
    // TODO: the index into the buffers?
    pub unk2: u64,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimHeaderV20 {
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    pub final_frame_index: f32,
    // TODO: Is this some other version?
    pub unk1: u16, // always 1?
    pub unk2: u16, // always 3?
    pub name: SsbhString,
    pub animations: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimHeaderV21 {
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    pub final_frame_index: f32,
    // TODO: Is this some other version?
    pub unk1: u16, // always 1?
    pub unk2: u16, // always 3?
    pub name: SsbhString,
    pub animations: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
    pub unk_data: UnkData,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkData {
    pub unk1: SsbhArray<UnkItem1>,
    pub unk2: SsbhArray<UnkItem2>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem1 {
    pub unk1: u64,
    pub unk2: SsbhArray<UnkSubItem>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem2 {
    pub unk1: SsbhString,
    pub unk2: SsbhArray<UnkSubItem>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkSubItem {
    pub unk1: u32,
    pub unk2: u32,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 2)]
pub struct TrackFlags {
    pub track_type: TrackType,
    #[br(pad_after = 2)]
    pub compression_type: CompressionType,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u8))]
pub enum TrackType {
    Transform = 1,
    Texture = 2,
    Float = 3,
    PatternIndex = 5,
    Boolean = 8,
    Vector4 = 9,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u8))]
pub enum CompressionType {
    Direct = 1,
    ConstTransform = 2,
    Compressed = 4,
    Constant = 5,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr(u64))]
pub enum AnimType {
    Transform = 1,
    Visibility = 2,
    Material = 4,
    Camera = 5,
}
