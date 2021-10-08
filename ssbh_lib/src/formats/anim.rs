use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use binread::BinRead;
use modular_bitfield::prelude::*;
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimTrackV2 {
    pub name: SsbhString,
    pub flags: TrackFlags,
    pub frame_count: u32,
    pub unk_flags: UnkTrackFlags, // flags?
    pub data_offset: u32,
    pub data_size: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimNode {
    pub name: SsbhString,
    pub tracks: SsbhArray<AnimTrackV2>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimGroup {
    pub group_type: GroupType,
    pub nodes: SsbhArray<AnimNode>,
}

/// Skeletal and material animation.
/// Compatible with file version 1.2, 2.0, and 2.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Anim {
    pub major_version: u16,
    pub minor_version: u16,
    #[br(args(major_version, minor_version))]
    pub header: AnimHeader,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum AnimHeader {
    #[br(pre_assert(major_version == 1 && minor_version == 2))]
    HeaderV1(AnimHeaderV12),

    #[br(pre_assert(major_version == 2 && minor_version == 0))]
    HeaderV20(AnimHeaderV20),

    #[br(pre_assert(major_version == 2 && minor_version == 1))]
    HeaderV21(AnimHeaderV21),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimHeaderV12 {
    pub name: SsbhString,
    pub unk1: u32,
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.   
    pub final_frame_index: f32,
    pub unk2: u64,
    pub tracks: SsbhArray<AnimTrackV1>,
    pub buffers: SsbhArray<SsbhByteBuffer>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimTrackV1 {
    pub name: SsbhString,
    pub track_type: u64, // TODO: Is this an enum?
    pub properties: SsbhArray<Property>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Property {
    pub name: SsbhString,
    /// The index of the corresponding buffer in [buffers](struct.AnimHeaderV12.html#structfield.buffers).
    pub buffer_index: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AnimHeaderV20 {
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    pub final_frame_index: f32,
    // TODO: Is this some other version?
    pub unk1: u16, // always 1?
    pub unk2: u16, // always 3?
    pub name: SsbhString,
    pub groups: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(align_after = 8)]
pub struct AnimHeaderV21 {
    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    pub final_frame_index: f32,
    // TODO: Is this some other version?
    pub unk1: u16, // always 1?
    pub unk2: u16, // always 3?
    pub name: SsbhString,
    pub groups: SsbhArray<AnimGroup>,
    pub buffer: SsbhByteBuffer,
    pub unk_data: UnkData,
}

// TODO: Is this interpolation data?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkData {
    pub unk1: SsbhArray<UnkItem1>,
    pub unk2: SsbhArray<UnkItem2>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem1 {
    pub unk1: u64,                   // TODO: Always 2?
    pub unk2: SsbhArray<UnkSubItem>, // TODO: Always (0, final_frame_index)?
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem2 {
    pub unk1: SsbhString,            // TODO: node name?
    pub unk2: SsbhArray<UnkSubItem>, // TODO: (frame start, frame end)?
}

// TODO: These appear to be start and end frame indices.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkSubItem {
    pub unk1: u32,
    pub unk2: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[ssbhwrite(pad_after = 2)]
pub struct TrackFlags {
    pub track_type: TrackType,
    #[br(pad_after = 2)]
    pub compression_type: CompressionType,
}

#[bitfield(bits = 32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Clone, Copy)]
#[br(map = Self::from_bytes)]
pub struct UnkTrackFlags {
    unk1: bool, // TODO: unk1?
    disable_rotation: bool,
    disable_scale: bool,
    disable_compensate_scale: bool,
    #[skip]
    __: B28,
}

ssbh_write::ssbh_write_modular_bitfield_impl!(UnkTrackFlags, 4);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u8))]
#[ssbhwrite(repr(u8))]
pub enum TrackType {
    Transform = 1,
    UvTransform = 2,
    Float = 3,
    PatternIndex = 5,
    Boolean = 8,
    Vector4 = 9,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u8))]
#[ssbhwrite(repr(u8))]
pub enum CompressionType {
    Direct = 1,

    // TODO: This can be used with non transform tracks for version 2.0 and 2.1.
    // ex: assist/metroid/model/body/c00/model.nuanmb
    ConstTransform = 2,

    Compressed = 4,
    Constant = 5,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u64))]
#[ssbhwrite(repr(u64))]
pub enum GroupType {
    Transform = 1,
    Visibility = 2,
    Material = 4,
    Camera = 5,
}
