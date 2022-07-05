//! The [Anim] format stores per frame animation data.
//! These files typically use the ".nuanmb" suffix like "model.nuanmb".
//!
//! Format version 2.0 and later uses the heirarchy of
//! [Group] -> [Node] -> [TrackV2] to organize animations.
//! The data for each frame is stored in a buffer that is usually compressed.
//! For a higher level API that handles compression and decompression, see [ssbh_data](https://crates.io/crates/ssbh_data).
use crate::SsbhArray;
use crate::SsbhByteBuffer;
use crate::SsbhString;
use crate::Version;
use binrw::BinRead;
use modular_bitfield::prelude::*;
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "strum")]
use strum::{Display, EnumString, EnumVariantNames, FromRepr};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct TrackV2 {
    pub name: SsbhString,
    pub flags: TrackFlags,
    pub frame_count: u32,
    pub transform_flags: TransformFlags,
    pub data_offset: u32,
    pub data_size: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Node {
    pub name: SsbhString,
    pub tracks: SsbhArray<TrackV2>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Group {
    pub group_type: GroupType,
    pub nodes: SsbhArray<Node>,
}

/// Skeletal and material animation.
/// Compatible with file version 1.2, 2.0, and 2.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Anim {
    #[br(pre_assert(major_version == 1 && minor_version == 2))]
    V12 {
        name: SsbhString,
        unk1: u32,
        /// The index of the last frame in the animation,
        /// which is calculated as `(frame_count - 1) as f32`.
        ///
        /// Frames use floating point to allow the rendering speed to differ from the animation speed.
        /// For example, some animations in Smash Ultimate interpolate when playing the game at 60fps but 1/4 speed.
        final_frame_index: f32,
        unk2: u64,
        tracks: SsbhArray<TrackV1>,
        buffers: SsbhArray<SsbhByteBuffer>,
    },

    #[br(pre_assert(major_version == 2 && minor_version == 0))]
    V20 {
        /// The index of the last frame in the animation,
        /// which is calculated as `(frame_count - 1) as f32`.
        final_frame_index: f32,
        // TODO: Is this some other version?
        unk1: u16, // always 1?
        unk2: u16, // always 3?
        name: SsbhString,
        groups: SsbhArray<Group>,
        buffer: SsbhByteBuffer,
    },

    #[br(pre_assert(major_version == 2 && minor_version == 1))]
    #[ssbhwrite(align_after = 8)]
    V21 {
        /// The index of the last frame in the animation,
        /// which is calculated as `(frame_count - 1) as f32`.
        final_frame_index: f32,
        // TODO: Is this some other version?
        unk1: u16, // always 1?
        unk2: u16, // always 3?
        name: SsbhString,
        groups: SsbhArray<Group>,
        buffer: SsbhByteBuffer,
        unk_data: UnkData,
    },
}

impl Version for Anim {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Anim::V12 { .. } => (1, 2),
            Anim::V20 { .. } => (2, 0),
            Anim::V21 { .. } => (2, 1),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct TrackV1 {
    pub name: SsbhString,
    pub track_type: TrackTypeV1,
    pub properties: SsbhArray<Property>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct Property {
    pub name: SsbhString,
    /// The index of the corresponding buffer in [buffers](enum.Anim.html#variant.V12.field.buffers).
    pub buffer_index: u64,
}

// TODO: Is this interpolation data?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkData {
    pub unk1: SsbhArray<UnkItem1>,
    pub unk2: SsbhArray<UnkItem2>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem1 {
    pub unk1: u64,                   // TODO: Always 2?
    pub unk2: SsbhArray<UnkSubItem>, // TODO: Always (0, final_frame_index)?
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkItem2 {
    pub unk1: SsbhString,            // TODO: node name?
    pub unk2: SsbhArray<UnkSubItem>, // TODO: (frame start, frame end)?
}

// TODO: These appear to be start and end frame indices.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct UnkSubItem {
    pub unk1: u32,
    pub unk2: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[ssbhwrite(pad_after = 2)]
pub struct TrackFlags {
    pub track_type: TrackTypeV2,
    #[br(pad_after = 2)]
    pub compression_type: CompressionType,
}

/// Flags for disabling the effects of values for [TrackTypeV2::Transform].
/// This overrides any values set for the transform values themselves.
///
/// # Examples
/// Disabling the translation, rotation, and scale completely relaces
/// the animation transforms with the transforms from the default pose in the model's skeleton.
/// For most models, disabling transforms for all bones in an animation will create a "T-pose".
/**
```rust
use ssbh_lib::formats::anim::TransformFlags;

let flags = TransformFlags::new()
    .with_override_translation(true)
    .with_override_rotation(true)
    .with_override_translation(true);
```
*/
#[bitfield(bits = 32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Clone, Copy)]
#[br(map = Self::from_bytes)]
pub struct TransformFlags {
    /// Overrides the translation values with the default resting pose from the skeleton.
    pub override_translation: bool,
    /// Overrides the rotation values with the default resting pose from the skeleton.
    pub override_rotation: bool,
    /// Overrides the scale values with the default resting pose from the skeleton.
    pub override_scale: bool,
    /// Sets scale compensation to `false` for all transforms in this track.
    pub override_compensate_scale: bool,
    #[skip]
    __: B28,
}

ssbh_write::ssbh_write_modular_bitfield_impl!(TransformFlags, 4);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u64))]
#[ssbhwrite(repr(u64))]
pub enum TrackTypeV1 {
    Transform = 0,
    UvTransform = 2,
    Boolean = 5,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u8))]
#[ssbhwrite(repr(u8))]
pub enum TrackTypeV2 {
    Transform = 1,
    UvTransform = 2,
    Float = 3,
    PatternIndex = 5,
    Boolean = 8,
    Vector4 = 9,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u8))]
#[ssbhwrite(repr(u8))]
pub enum CompressionType {
    /// Uncompressed
    Direct = 1,

    // TODO: This can be used with non transform tracks for version 2.0 and 2.1.
    // ex: assist/metroid/model/body/c00/model.nuanmb
    /// Uncompressed
    ConstTransform = 2,

    /// Values are compressed to use fewer bits.
    /// This compression is only lossless for [TrackTypeV2::Boolean].
    Compressed = 4,

    /// Uncompressed
    Constant = 5,
}

/// Determines the usage for a [Group].
///
/// This often corresponds with [TrackTypeV2] like [GroupType::Transform] and [TrackTypeV2::Transform].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u64))]
#[ssbhwrite(repr(u64))]
pub enum GroupType {
    Transform = 1,
    Visibility = 2,
    Material = 4,
    Camera = 5,
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;

    use super::*;

    #[test]
    fn align_v20() {
        let mut buffer = Cursor::new(Vec::new());
        let anim = Anim::V20 {
            final_frame_index: 0.0,
            unk1: 0,
            unk2: 0,
            name: "a".into(),
            groups: SsbhArray::new(),
            buffer: SsbhByteBuffer::new(),
        };
        anim.write(&mut buffer).unwrap();

        assert_eq!(2, buffer.into_inner().len() % 8);
    }

    #[test]
    fn align_v21() {
        let mut buffer = Cursor::new(Vec::new());
        let anim = Anim::V21 {
            final_frame_index: 0.0,
            unk1: 0,
            unk2: 0,
            name: "a".into(),
            groups: SsbhArray::new(),
            buffer: SsbhByteBuffer::new(),
            unk_data: UnkData {
                unk1: SsbhArray::new(),
                unk2: SsbhArray::new(),
            },
        };
        anim.write(&mut buffer).unwrap();

        // Version 2.10 is aligned to 8 bytes.
        assert_eq!(0, buffer.into_inner().len() % 8);
    }
}
