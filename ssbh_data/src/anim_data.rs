//! Types for working with [Anim] data in .nuanmb files.
//!
//! # Examples
//! Animation data is stored in a hierarchy.
//! Values for each frame are stored at the [TrackData] level.
/*!
```rust no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use ssbh_data::prelude::*;

let anim = AnimData::from_file("model.nuanmb")?;

for group in anim.groups {
    for node in group.nodes {
        for track in node.tracks {
            println!("Frame Count: {}", track.values.len());
        }
    }
}
# Ok(()) }
```
 */
//!
//! # Compression
//! Compressed animations use lossy compression for all data types except [TrackValues::Boolean].
//! Float compression encodes values using a configurable number of
//! values between two floating point endpoints.
//! Depending on the endpoints and number of bits, the encoded values
//! between the two endpoints may not be representable by 32 bit floating point.
//! This means that decompression may introduce some error, so compressing an animation
//! again with the same settings may produce slightly different compressed data.
//!
//! # File Differences
//! Unmodified files are not guaranteed to be binary identical after saving.
//! Compressed animations use lossy compression for all data types except [TrackValues::Boolean].
//! When converting to [Anim], compression is enabled for a track if compression would save space.
//! This may produce differences with the original due to compression differences.
//! These errors are small in practice but may cause gameplay differences such as online desyncs.
use binrw::BinRead;
use glam::{Quat, Vec3, Vec4};
use ssbh_lib::{Vector3, Vector4};
use ssbh_lib::{
    Version,
    formats::anim::{Anim, TrackTypeV2, TransformFlags as AnimTransformFlags},
};
use ssbh_write::SsbhWrite;
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
};

pub use ssbh_lib::formats::anim::GroupType;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod bitutils;
pub mod error;
mod v1;
mod v2;

/// Data associated with an [Anim] file.
/// Supported versions are 1.2, 2.0, and 2.1.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct AnimData {
    pub major_version: u16,
    pub minor_version: u16,

    /// The index of the last frame in the animation,
    /// which is calculated as `(frame_count - 1) as f32`.
    ///
    /// Constant animations will last for final_frame_index + 1 many frames.
    ///
    /// Frames use floating point to allow the rendering speed to differ from the animation speed.
    /// For example, some animations in Smash Ultimate interpolate when playing the game at 60fps but 1/4 speed.
    pub final_frame_index: f32,
    pub groups: Vec<GroupData>,
}

impl AnimData {
    // TODO: Test this for small example anims
    /// Encode all animation data to the specified version
    /// with compression chosen by the encoder.
    pub fn to_anim(&self) -> Result<Anim, error::Error> {
        match (self.major_version, self.minor_version) {
            // Use compressed format for v1.2 (EXVS2 compatibility)
            // TODO: select uncompressed for 1.2 if it saves space?
            (1, 2) => Ok(v1::create_anim_v12(self)?),
            (2, 0) => v2::create_anim_v20(self),
            (2, 1) => v2::create_anim_v21(self),
            (major_version, minor_version) => Err(error::Error::UnsupportedVersion {
                major_version,
                minor_version,
            }),
        }
    }

    // TODO: Test this for small example anims
    /// Encode all animation data to the specified version without any compression.
    pub fn to_anim_uncompressed(&self) -> Result<Anim, error::Error> {
        match (self.major_version, self.minor_version) {
            (1, 2) => Ok(v1::create_anim_v12_uncompressed(self)?),
            // TODO: force uncompressed for 2.0 and 2.1
            (2, 0) => todo!(),
            (2, 1) => todo!(),
            (major_version, minor_version) => Err(error::Error::UnsupportedVersion {
                major_version,
                minor_version,
            }),
        }
    }
}

// TODO: Test these conversions.
impl TryFrom<Anim> for AnimData {
    type Error = Box<dyn Error>;

    fn try_from(anim: Anim) -> Result<Self, Self::Error> {
        (&anim).try_into()
    }
}

impl TryFrom<&Anim> for AnimData {
    type Error = Box<dyn Error>;

    fn try_from(anim: &Anim) -> Result<Self, Self::Error> {
        let (major_version, minor_version) = anim.major_minor_version();
        Ok(Self {
            major_version,
            minor_version,
            final_frame_index: match &anim {
                Anim::V12 {
                    final_frame_index, ..
                } => *final_frame_index,
                Anim::V20 {
                    final_frame_index, ..
                } => *final_frame_index,
                Anim::V21 {
                    final_frame_index, ..
                } => *final_frame_index,
            },
            groups: read_anim_groups(anim)?,
        })
    }
}

impl TryFrom<AnimData> for Anim {
    type Error = error::Error;

    fn try_from(data: AnimData) -> Result<Self, Self::Error> {
        data.to_anim()
    }
}

impl TryFrom<&AnimData> for Anim {
    type Error = error::Error;

    fn try_from(data: &AnimData) -> Result<Self, Self::Error> {
        data.to_anim()
    }
}

/// Data associated with a [Group][ssbh_lib::formats::anim::Group].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct GroupData {
    /// The usage type for all the [NodeData] in [nodes](#structfield.nodes)
    pub group_type: GroupType,
    pub nodes: Vec<NodeData>,
}

/// Data associated with a [Node][ssbh_lib::formats::anim::Node].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NodeData {
    pub name: String,
    pub tracks: Vec<TrackData>,
}

/// The data associated with a [TrackV2](ssbh_lib::formats::anim::TrackV2).
///
/// # Examples
/// The scale settings and transform flags should usually use their default value.
/**
```rust
use ssbh_data::anim_data::{TrackData, TrackValues, Transform, TransformFlags};

let track = TrackData {
    name: "Transform".to_string(),
    values: TrackValues::Transform(vec![Transform::IDENTITY]),
    compensate_scale: false,
    transform_flags: TransformFlags::default()
};
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct TrackData {
    /// The name of the property to animate.
    ///
    /// For tracks in a group of type [GroupType::Material], this is the name of the material parameter like "CustomVector31".
    /// Other group types tend to use the name of the group type like "Transform" or "Visibility".
    pub name: String,

    /// Revert the scaling of the immediate parent when `true`.
    /// Only applies to [TrackValues::Transform].
    ///
    /// The final scale relative to the parent is `current_scale * (1 / parent_scale)`.
    /// For Smash Ultimate, this is not applied recursively on the parent,
    /// so only the immediate parent's scaling is taken into account.
    /// This matches the behavior of scale compensation in Autodesk Maya.
    pub compensate_scale: bool,

    pub transform_flags: TransformFlags,

    /// The frame values for the property specified by [name](#structfield.name).
    ///
    /// Each element in the [TrackValues] provides the value for a single frame.
    /// If the [TrackValues] contains a single element, this track will be considered constant
    /// and repeat that element for each frame in the animation
    /// up to and including [final_frame_index](struct.AnimData.html#structfield.final_frame_index).
    pub values: TrackValues,
}

/// See [ssbh_lib::formats::anim::TransformFlags].
// Including compensate scale would be redundant with ScaleOptions.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct TransformFlags {
    pub override_translation: bool,
    pub override_rotation: bool,
    pub override_scale: bool,
    pub override_compensate_scale: bool,
}

impl From<TransformFlags> for AnimTransformFlags {
    fn from(f: TransformFlags) -> Self {
        Self::new(
            f.override_translation,
            f.override_rotation,
            f.override_scale,
            f.override_compensate_scale,
        )
    }
}

impl From<AnimTransformFlags> for TransformFlags {
    fn from(f: AnimTransformFlags) -> Self {
        Self {
            override_translation: f.override_translation(),
            override_rotation: f.override_rotation(),
            override_scale: f.override_scale(),
            override_compensate_scale: f.override_compensate_scale(),
        }
    }
}

// TODO: Investigate if the names based on the Anim 1.2 property names are accurate.
/// A decomposed 2D transformation for texture coordinates.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, SsbhWrite, Default, Clone, Copy)]
pub struct UvTransform {
    pub scale_u: f32,
    pub scale_v: f32,
    pub rotation: f32,
    pub translate_u: f32,
    pub translate_v: f32,
}

/// A decomposed 3D transformation consisting of a scale, rotation, and translation.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Transform {
    pub scale: Vec3,
    pub rotation: Quat,
    pub translation: Vec3,
}

impl Transform {
    /// An identity transformation representing no scale, rotation, or translation.
    pub const IDENTITY: Transform = Transform {
        scale: Vec3::ONE,
        rotation: Quat::IDENTITY,
        translation: Vec3::ZERO,
    };
}

// TODO: Add version 1.2 types.
// TODO: Create runtime errors when saving tracks with incompatible data?
/// A value collection with an element for each frame of the animation.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub enum TrackValues {
    /// Transformations used for camera or skeletal animations.
    Transform(Vec<Transform>),
    /// Transformations applied to UV coordinates for texture animations.
    UvTransform(Vec<UvTransform>),
    /// Animated scalar parameter values.
    Float(Vec<f32>),
    // TODO: rename to u32?
    PatternIndex(Vec<u32>),
    /// Visibility animations or animated boolean parameters.
    Boolean(Vec<bool>),
    /// Material animations or animated vector parameters.
    Vector4(Vec<Vec4>),
}

impl TrackValues {
    /// Returns the number of elements, which is equivalent to the number of frames.
    /// # Examples
    /**
    ```rust
    # use ssbh_data::anim_data::TrackValues;
    assert_eq!(3, TrackValues::Boolean(vec![true, false, true]).len());
    ```
     */
    pub fn len(&self) -> usize {
        match self {
            TrackValues::Transform(v) => v.len(),
            TrackValues::UvTransform(v) => v.len(),
            TrackValues::Float(v) => v.len(),
            TrackValues::PatternIndex(v) => v.len(),
            TrackValues::Boolean(v) => v.len(),
            TrackValues::Vector4(v) => v.len(),
        }
    }

    /// Returns `true` there are no elements.
    /**
    ```rust
    # use ssbh_data::anim_data::TrackValues;
    assert!(TrackValues::Transform(Vec::new()).is_empty());
    ```
     */
    pub fn is_empty(&self) -> bool {
        match self {
            TrackValues::Transform(v) => v.is_empty(),
            TrackValues::UvTransform(v) => v.is_empty(),
            TrackValues::Float(v) => v.is_empty(),
            TrackValues::PatternIndex(v) => v.is_empty(),
            TrackValues::Boolean(v) => v.is_empty(),
            TrackValues::Vector4(v) => v.is_empty(),
        }
    }

    fn track_type(&self) -> TrackTypeV2 {
        match self {
            TrackValues::Transform(_) => TrackTypeV2::Transform,
            TrackValues::UvTransform(_) => TrackTypeV2::UvTransform,
            TrackValues::Float(_) => TrackTypeV2::Float,
            TrackValues::PatternIndex(_) => TrackTypeV2::PatternIndex,
            TrackValues::Boolean(_) => TrackTypeV2::Boolean,
            TrackValues::Vector4(_) => TrackTypeV2::Vector4,
        }
    }
}

// TODO: Test conversions from anim?
fn read_anim_groups(anim: &Anim) -> Result<Vec<GroupData>, error::Error> {
    match anim {
        ssbh_lib::prelude::Anim::V12 {
            tracks,
            buffers,
            final_frame_index,
            ..
        } => {
            // For version 1.2, use the animation's final_frame_index to determine frame count
            let frame_count = (*final_frame_index as usize).saturating_add(1);
            v1::read_groups_v12(&tracks.elements, &buffers.elements, frame_count)
        }
        ssbh_lib::formats::anim::Anim::V20 { groups, buffer, .. } => {
            v2::read_groups_v20(&groups.elements, &buffer.elements)
        }
        ssbh_lib::formats::anim::Anim::V21 { groups, buffer, .. } => {
            v2::read_groups_v20(&groups.elements, &buffer.elements)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Test the conversions more thoroughly.

    #[test]
    fn create_empty_anim_v_1_2() {
        let anim = AnimData {
            major_version: 1,
            minor_version: 2,
            final_frame_index: 1.5,
            groups: Vec::new(),
        }
        .to_anim()
        .unwrap();

        assert!(matches!(
            anim,
            Anim::V12 {
                final_frame_index,
                ..
            } if final_frame_index == 1.5
        ));
    }

    #[test]
    fn create_empty_anim_v_1_2_uncompressed() {
        let anim = AnimData {
            major_version: 1,
            minor_version: 2,
            final_frame_index: 1.5,
            groups: Vec::new(),
        }
        .to_anim_uncompressed()
        .unwrap();

        assert!(matches!(
            anim,
            Anim::V12 {
                final_frame_index,
                ..
            } if final_frame_index == 1.5
        ));
    }

    #[test]
    fn create_empty_anim_v_2_0() {
        let anim = AnimData {
            major_version: 2,
            minor_version: 0,
            final_frame_index: 1.5,
            groups: Vec::new(),
        }
        .to_anim()
        .unwrap();

        assert!(matches!(
            anim,
            Anim::V20 {
                final_frame_index,
                ..
            } if final_frame_index == 1.5
        ));
    }

    #[test]
    fn create_empty_anim_v_2_1() {
        let anim = AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: 2.5,
            groups: Vec::new(),
        }
        .to_anim()
        .unwrap();

        assert!(matches!(anim, Anim::V21 {
            final_frame_index,
            ..
        } if final_frame_index == 2.5));
    }

    #[test]
    fn create_anim_negative_frame_index() {
        let result = AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: -1.0,
            groups: Vec::new(),
        }
        .to_anim();

        assert!(matches!(
            result,
            Err(error::Error::InvalidFinalFrameIndex {
                final_frame_index
            }) if final_frame_index == -1.0
        ));
    }

    #[test]
    fn create_anim_insufficient_frame_index() {
        let result = AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: 2.0,
            groups: vec![GroupData {
                group_type: GroupType::Visibility,
                nodes: vec![NodeData {
                    name: String::new(),
                    tracks: vec![TrackData {
                        name: String::new(),
                        values: TrackValues::Boolean(vec![true; 4]),
                        compensate_scale: false,
                        transform_flags: TransformFlags::default(),
                    }],
                }],
            }],
        }
        .to_anim();

        // A value of at least 3.0 is expected.
        assert!(matches!(
            result,
            Err(error::Error::InvalidFinalFrameIndex {
                final_frame_index
            }) if final_frame_index == 2.0
        ));
    }

    #[test]
    fn create_anim_zero_frame_index() {
        let anim = AnimData {
            major_version: 2,
            minor_version: 1,
            final_frame_index: 0.0,
            groups: Vec::new(),
        }
        .to_anim()
        .unwrap();

        assert!(matches!(anim, Anim::V21 {
            final_frame_index,
            ..
        } if final_frame_index == 0.0));
    }

    #[test]
    fn create_empty_anim_invalid_version() {
        let result = AnimData {
            major_version: 1,
            minor_version: 1,
            final_frame_index: 0.0,
            groups: Vec::new(),
        }
        .to_anim();

        assert!(matches!(
            result,
            Err(error::Error::UnsupportedVersion {
                major_version: 1,
                minor_version: 1
            })
        ));
    }
}
