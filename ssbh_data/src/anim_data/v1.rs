use std::collections::HashMap;

use glam::{Quat, Vec3};

use encode::encode_vector3_3409;

use super::{AnimData, GroupType, TrackValues, error};
use ssbh_lib::SsbhByteBuffer;
use ssbh_lib::formats::anim::{Anim, Property, TrackTypeV1, TrackV1};

use bitvec::{order::Lsb0, vec::BitVec};
use ssbh_lib::Vector4;

use crate::anim_data::v1::buffers::read_track_values_v12;
use crate::anim_data::{GroupData, NodeData, TrackData, TransformFlags};
use crate::anim_data::{UvTransform, error::Error};

mod buffers;
mod common;
mod encode;
mod rotate_4409;
mod rotate_basic;
mod rotate_inferred;
mod translate;

fn group_type_v12(track_type: TrackTypeV1) -> GroupType {
    match track_type {
        TrackTypeV1::Transform => GroupType::Transform,
        TrackTypeV1::UvTransform => GroupType::Material,
        TrackTypeV1::Visibility => GroupType::Visibility,
    }
}

pub fn read_groups_v12(
    tracks: &[ssbh_lib::formats::anim::TrackV1],
    buffers: &[ssbh_lib::SsbhByteBuffer],
    animation_frame_count: usize,
) -> Result<Vec<GroupData>, error::Error> {
    // Group by the track type.
    let mut tracks_by_type = HashMap::new();

    // TODO: Avoid unwrap.
    // Node names like bones names are set at the track level for anim 1.2.
    // Save the track name to use for the nodes later.
    // TODO: separate visibility from transform tracks
    for track in tracks {
        let group_type = group_type_v12(track.track_type);
        let track_data = create_track_data_v12(track, buffers, animation_frame_count).unwrap();
        tracks_by_type
            .entry(group_type)
            .or_insert(Vec::new())
            .push((track.name.to_string_lossy(), track_data));
    }

    // Use the grouping conventions for version 2.0+ anims.
    // TODO: Will this preserve data when saving back to 1.2?
    let groups = tracks_by_type
        .into_iter()
        .map(|(group_type, tracks)| GroupData {
            group_type,
            nodes: tracks
                .into_iter()
                .map(|(name, track)| NodeData {
                    name,
                    tracks: vec![track],
                })
                .collect(),
        })
        .collect();

    Ok(groups)
}

// Improved version 1.2 track data creation that properly merges properties
fn create_track_data_v12(
    track: &ssbh_lib::formats::anim::TrackV1,
    buffers: &[ssbh_lib::SsbhByteBuffer],
    animation_frame_count: usize,
) -> Result<TrackData, error::Error> {
    let transform_flags = TransformFlags::default();

    let (values, compensate_scale) = read_track_values_v12(track, buffers, animation_frame_count)?;

    Ok(TrackData {
        name: match track.track_type {
            TrackTypeV1::Transform => "Transform".to_owned(),
            TrackTypeV1::Visibility => "Visibility".to_owned(),
            TrackTypeV1::UvTransform => "UvTransform".to_owned(),
        },
        compensate_scale,
        values,
        transform_flags,
    })
}

// Function to create version 1.2 animation from AnimData
pub(super) fn create_anim_v12(data: &AnimData) -> Result<Anim, error::Error> {
    let mut tracks = Vec::new();
    let mut buffers = Vec::new();

    // Convert each group back to tracks for version 1.2
    for group in &data.groups {
        for node in &group.nodes {
            for track in &node.tracks {
                // Determine track type based on group type and track name
                let track_type = match group.group_type {
                    GroupType::Transform => TrackTypeV1::Transform,
                    GroupType::Visibility => TrackTypeV1::Visibility,
                    GroupType::Material => TrackTypeV1::UvTransform,
                    _ => TrackTypeV1::Transform, // Default fallback
                };

                // Create properties based on track type and values
                let mut properties = Vec::new();

                match (&track.values, track_type) {
                    (TrackValues::Transform(transforms), TrackTypeV1::Transform) => {
                        if !transforms.is_empty() {
                            // Extract scale, rotation, and translation data
                            let scales: Vec<Vec3> = transforms.iter().map(|t| t.scale).collect();
                            let rotations: Vec<Quat> =
                                transforms.iter().map(|t| t.rotation).collect();
                            let translations: Vec<Vec3> =
                                transforms.iter().map(|t| t.translation).collect();

                            // Create Scale property
                            // Preserve property presence semantics as much as possible.
                            // The high level JSON does not retain the original property list for Anim v1.2,
                            // so we avoid writing properties that look like unanimated defaults.
                            let write_scale = scales.iter().any(|s| {
                                (s.x - 1.0).abs() > 1e-6
                                    || (s.y - 1.0).abs() > 1e-6
                                    || (s.z - 1.0).abs() > 1e-6
                            });
                            if write_scale {
                                if scales.len() == 1 {
                                    // Single frame scale
                                    let mut scale_data = Vec::new();
                                    scale_data.extend_from_slice(&0x3003u32.to_le_bytes());
                                    scale_data.extend_from_slice(&scales[0].x.to_le_bytes());
                                    scale_data.extend_from_slice(&scales[0].y.to_le_bytes());
                                    scale_data.extend_from_slice(&scales[0].z.to_le_bytes());

                                    buffers.push(SsbhByteBuffer {
                                        elements: scale_data,
                                    });
                                    properties.push(Property {
                                        name: "Scale".into(),
                                        buffer_index: (buffers.len() - 1) as u64,
                                    });
                                } else {
                                    // Multi-frame scale data - use an uncompressed indexed format.
                                    // This avoids generating malformed 0x3409 buffers until the encoder is fully validated.
                                    let scale_data =
                                        create_v12_uncompressed_vector3_data(&scales, "Scale")?;
                                    buffers.push(SsbhByteBuffer {
                                        elements: scale_data,
                                    });
                                    properties.push(Property {
                                        name: "Scale".into(),
                                        buffer_index: (buffers.len() - 1) as u64,
                                    });
                                }
                            }

                            // Create Rotation property
                            let write_rotate = rotations.iter().any(|q| {
                                q.x.abs() > 1e-6
                                    || q.y.abs() > 1e-6
                                    || q.z.abs() > 1e-6
                                    || (q.w - 1.0).abs() > 1e-6
                            });
                            if write_rotate {
                                if rotations.len() == 1 {
                                    // Single frame rotation
                                    let mut rotation_data = Vec::new();
                                    rotation_data.extend_from_slice(&0x4003u32.to_le_bytes());
                                    rotation_data.extend_from_slice(&rotations[0].x.to_le_bytes());
                                    rotation_data.extend_from_slice(&rotations[0].y.to_le_bytes());
                                    rotation_data.extend_from_slice(&rotations[0].z.to_le_bytes());
                                    rotation_data.extend_from_slice(&rotations[0].w.to_le_bytes());

                                    buffers.push(SsbhByteBuffer {
                                        elements: rotation_data,
                                    });
                                    properties.push(Property {
                                        name: "Rotate".into(),
                                        buffer_index: (buffers.len() - 1) as u64,
                                    });
                                } else {
                                    // Multi-frame rotation data - write an uncompressed format.
                                    // This avoids generating a malformed 0x4409 buffer.
                                    let rotation_data =
                                        create_v12_uncompressed_vector4_data(&rotations)?;
                                    buffers.push(SsbhByteBuffer {
                                        elements: rotation_data,
                                    });
                                    properties.push(Property {
                                        name: "Rotate".into(),
                                        buffer_index: (buffers.len() - 1) as u64,
                                    });
                                }
                            }

                            // Create Translation property
                            let write_translate = translations
                                .iter()
                                .any(|t| t.x.abs() > 1e-6 || t.y.abs() > 1e-6 || t.z.abs() > 1e-6);
                            if write_translate {
                                if translations.len() == 1 {
                                    // Single frame translation
                                    let mut translation_data = Vec::new();
                                    translation_data.extend_from_slice(&0x3003u32.to_le_bytes());
                                    translation_data
                                        .extend_from_slice(&translations[0].x.to_le_bytes());
                                    translation_data
                                        .extend_from_slice(&translations[0].y.to_le_bytes());
                                    translation_data
                                        .extend_from_slice(&translations[0].z.to_le_bytes());

                                    buffers.push(SsbhByteBuffer {
                                        elements: translation_data,
                                    });
                                    properties.push(Property {
                                        name: "Translate".into(),
                                        buffer_index: (buffers.len() - 1) as u64,
                                    });
                                } else {
                                    // Multi-frame translation data - use an uncompressed indexed format.
                                    // This avoids generating malformed 0x3409 buffers until the encoder is fully validated.
                                    let translation_data = create_v12_uncompressed_vector3_data(
                                        &translations,
                                        "Translate",
                                    )?;
                                    buffers.push(SsbhByteBuffer {
                                        elements: translation_data,
                                    });
                                    properties.push(Property {
                                        name: "Translate".into(),
                                        buffer_index: (buffers.len() - 1) as u64,
                                    });
                                }
                            }

                            // CompensateScale property if needed
                            // CompensateScale is represented as a boolean-like 0x1013 u16 in Anim v1.2.
                            // Preserve property presence by always writing it, even when false.
                            {
                                let mut compensate_data = Vec::new();
                                compensate_data.extend_from_slice(&0x1013u32.to_le_bytes());
                                let v: u16 = if track.compensate_scale {
                                    0x7FFF
                                } else {
                                    0x0000
                                };
                                compensate_data.extend_from_slice(&v.to_le_bytes());

                                buffers.push(SsbhByteBuffer {
                                    elements: compensate_data,
                                });
                                properties.push(Property {
                                    name: "CompensateScale".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            }

                            // Visibility is commonly present as a 0x1013 u16 on Transform tracks.
                            // The high level representation does not currently preserve it, so default to true.
                            {
                                let mut visibility_data = Vec::new();
                                visibility_data.extend_from_slice(&0x1013u32.to_le_bytes());
                                visibility_data.extend_from_slice(&0x7FFFu16.to_le_bytes());

                                buffers.push(SsbhByteBuffer {
                                    elements: visibility_data,
                                });
                                properties.push(Property {
                                    name: "Visibility".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            }
                        }
                    }
                    (TrackValues::Boolean(bools), TrackTypeV1::Visibility) => {
                        if !bools.is_empty() {
                            if bools.len() == 1 {
                                // Single frame visibility
                                let mut visibility_data = Vec::new();
                                visibility_data.extend_from_slice(&0x1013u32.to_le_bytes());
                                visibility_data.extend_from_slice(
                                    &(if bools[0] { 1u16 } else { 0u16 }).to_le_bytes(),
                                );

                                buffers.push(SsbhByteBuffer {
                                    elements: visibility_data,
                                });
                                properties.push(Property {
                                    name: "Visibility".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            } else {
                                // Multi-frame visibility data - create appropriate compressed format
                                let visibility_data = create_v12_compressed_bool_data(bools)?;
                                buffers.push(SsbhByteBuffer {
                                    elements: visibility_data,
                                });
                                properties.push(Property {
                                    name: "Visibility".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            }
                        }
                    }
                    (TrackValues::UvTransform(uv_transforms), TrackTypeV1::UvTransform) => {
                        if !uv_transforms.is_empty() {
                            if uv_transforms.len() == 1 {
                                // Single frame UV transform
                                let uv = &uv_transforms[0];
                                let mut uv_data = Vec::new();
                                uv_data.extend_from_slice(&0x5014u32.to_le_bytes());
                                uv_data.extend_from_slice(&uv.scale_u.to_le_bytes());
                                uv_data.extend_from_slice(&uv.scale_v.to_le_bytes());
                                uv_data.extend_from_slice(&uv.rotation.to_le_bytes());
                                uv_data.extend_from_slice(&uv.translate_u.to_le_bytes());
                                uv_data.extend_from_slice(&uv.translate_v.to_le_bytes());

                                buffers.push(SsbhByteBuffer { elements: uv_data });
                                properties.push(Property {
                                    name: "UvTransform".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            } else {
                                // Multi-frame UV transform data - create appropriate compressed format
                                let uv_data = create_v12_compressed_uv_data(uv_transforms)?;
                                buffers.push(SsbhByteBuffer { elements: uv_data });
                                properties.push(Property {
                                    name: "UvTransform".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            }
                        }
                    }
                    _ => {
                        // Unsupported combinations should not generate invalid placeholder buffers.
                        // Writing an unknown property with a 0x0000 header can crash consumers.
                    }
                }

                // Create the track
                if !properties.is_empty() {
                    tracks.push(TrackV1 {
                        name: node.name.as_str().into(),
                        track_type,
                        properties: properties.into(),
                    });
                }
            }
        }
    }

    // Validate final_frame_index is non-negative
    let final_frame_index = if data.final_frame_index >= 0.0 {
        Ok(data.final_frame_index)
    } else {
        Err(error::Error::InvalidFinalFrameIndex {
            final_frame_index: data.final_frame_index,
        })
    }?;

    Ok(Anim::V12 {
        name: "".into(), // Default empty name
        // NOTE (empirically observed in-game):
        // `unk1` behaves like a playback range multiplier for Anim v1.2.
        // For an animation with N frames (0..N-1), setting `unk1 = 0.5` results in only the first
        // half of the timeline being played (approximately 0..(N/2 - 1)).
        // Setting `unk1 = 0.0` causes the animation to appear non-advancing (only the first frame).
        unk1: 1.0,
        final_frame_index,
        unk2: 0.0, // Default unknown value
        unk3: 0.0, // Default unknown value
        tracks: tracks.into(),
        buffers: buffers.into(),
    })
}

// Create uncompressed v1.2 animation from AnimData
// This version uses only constant and raw stream formats, never compressed formats
pub(super) fn create_anim_v12_uncompressed(data: &AnimData) -> Result<Anim, error::Error> {
    let mut tracks = Vec::new();
    let mut buffers = Vec::new();

    let _frame_count = (data.final_frame_index as usize) + 1;

    // Convert each group back to tracks for version 1.2
    for group in &data.groups {
        for node in &group.nodes {
            for track in &node.tracks {
                // Determine track type based on group type and track name
                let track_type = match group.group_type {
                    GroupType::Transform => TrackTypeV1::Transform,
                    GroupType::Visibility => TrackTypeV1::Visibility,
                    GroupType::Material => TrackTypeV1::UvTransform,
                    _ => TrackTypeV1::Transform, // Default fallback
                };

                // Create properties based on track type and values
                let mut properties = Vec::new();

                match (&track.values, track_type) {
                    (TrackValues::Transform(transforms), TrackTypeV1::Transform) => {
                        if !transforms.is_empty() {
                            // Extract scale, rotation, and translation data
                            let scales: Vec<Vec3> = transforms.iter().map(|t| t.scale).collect();
                            let rotations: Vec<Quat> =
                                transforms.iter().map(|t| t.rotation).collect();
                            let translations: Vec<Vec3> =
                                transforms.iter().map(|t| t.translation).collect();

                            // Create Scale property
                            let scale_data =
                                create_v12_uncompressed_vector3_data(&scales, "Scale")?;
                            buffers.push(SsbhByteBuffer {
                                elements: scale_data,
                            });
                            properties.push(Property {
                                name: "Scale".into(),
                                buffer_index: (buffers.len() - 1) as u64,
                            });

                            // Create Rotation property
                            let rotation_data = create_v12_uncompressed_vector4_data(&rotations)?;
                            buffers.push(SsbhByteBuffer {
                                elements: rotation_data,
                            });
                            properties.push(Property {
                                name: "Rotate".into(),
                                buffer_index: (buffers.len() - 1) as u64,
                            });

                            // Create Translation property
                            let translation_data =
                                create_v12_uncompressed_vector3_data(&translations, "Translate")?;
                            buffers.push(SsbhByteBuffer {
                                elements: translation_data,
                            });
                            properties.push(Property {
                                name: "Translate".into(),
                                buffer_index: (buffers.len() - 1) as u64,
                            });

                            // CompensateScale property if needed
                            if track.compensate_scale {
                                let mut compensate_data = Vec::new();
                                compensate_data.extend_from_slice(&0x1003u32.to_le_bytes());
                                compensate_data.extend_from_slice(&1.0f32.to_le_bytes()); // true = 1.0

                                buffers.push(SsbhByteBuffer {
                                    elements: compensate_data,
                                });
                                properties.push(Property {
                                    name: "CompensateScale".into(),
                                    buffer_index: (buffers.len() - 1) as u64,
                                });
                            }
                        }
                    }
                    (TrackValues::Boolean(bools), TrackTypeV1::Visibility) => {
                        if !bools.is_empty() {
                            let visibility_data = create_v12_uncompressed_bool_data(bools)?;
                            buffers.push(SsbhByteBuffer {
                                elements: visibility_data,
                            });
                            properties.push(Property {
                                name: "Visibility".into(),
                                buffer_index: (buffers.len() - 1) as u64,
                            });
                        }
                    }
                    (TrackValues::UvTransform(uv_transforms), TrackTypeV1::UvTransform) => {
                        if !uv_transforms.is_empty() {
                            let uv_data = create_v12_uncompressed_uv_data(uv_transforms)?;
                            buffers.push(SsbhByteBuffer { elements: uv_data });
                            properties.push(Property {
                                name: "UvTransform".into(),
                                buffer_index: (buffers.len() - 1) as u64,
                            });
                        }
                    }
                    _ => {
                        // Create a default empty property for unsupported combinations
                        let mut default_data = Vec::new();
                        default_data.extend_from_slice(&0x0000u32.to_le_bytes());

                        buffers.push(SsbhByteBuffer {
                            elements: default_data,
                        });
                        properties.push(Property {
                            name: track.name.as_str().into(),
                            buffer_index: (buffers.len() - 1) as u64,
                        });
                    }
                }

                // Create the track
                tracks.push(TrackV1 {
                    name: node.name.as_str().into(),
                    track_type,
                    properties: properties.into(),
                });
            }
        }
    }

    // Validate final_frame_index is non-negative
    let final_frame_index = if data.final_frame_index >= 0.0 {
        Ok(data.final_frame_index)
    } else {
        Err(error::Error::InvalidFinalFrameIndex {
            final_frame_index: data.final_frame_index,
        })
    }?;

    Ok(Anim::V12 {
        name: "".into(), // Default empty name
        unk1: 1.0,
        final_frame_index,
        unk2: 0.0, // Default unknown value
        unk3: 0.0, // Default unknown value
        tracks: tracks.into(),
        buffers: buffers.into(),
    })
}

// Helper functions for version 1.2 creation

/// Create compressed Vector3 data for version 1.2 using format 0x3409
/// Uses the proper EXVS2-compatible encoder with 33-key blocks and residual encoding.
/// This is used for both Scale and Translate properties as they share the same format.
#[allow(dead_code)]
pub fn create_v12_compressed_vector3_data(values: &[Vec3]) -> Result<Vec<u8>, Error> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    // Use the proper 0x3409 encoder.
    // This encoder is compatible with both Scale and Translate data.
    encode_vector3_3409(values)
}

/// Calculate interpolation index for a value within the range [first, middle, last]
pub fn calculate_interpolation_index(
    value: f32,
    first: f32,
    middle: f32,
    last: f32,
    bits: usize,
) -> u32 {
    let max_index = (1u32 << bits) - 1;

    // Determine which segment the value falls into
    let t = if (last - first).abs() < 1e-6 {
        // Values are all the same
        0.0
    } else if value <= middle {
        // First half: interpolate between first and middle
        if (middle - first).abs() < 1e-6 {
            0.0
        } else {
            (value - first) / (last - first) * 0.5
        }
    } else {
        // Second half: interpolate between middle and last
        if (last - middle).abs() < 1e-6 {
            0.5
        } else {
            0.5 + (value - middle) / (last - first) * 0.5
        }
    };

    (t * max_index as f32).clamp(0.0, max_index as f32) as u32
}

/// Write bits to a BitVec
#[allow(dead_code)]
fn write_bits(bits: &mut BitVec<u8, Lsb0>, value: u32, count: usize) {
    for i in 0..count {
        bits.push((value >> i) & 1 == 1);
    }
}

/// Create compressed Vector4 data for version 1.2 using format 0x4409
/// Used for quaternion rotations - based on GitHub discussion format 0x0944/0x4409
#[allow(dead_code)]
pub fn create_v12_compressed_vector4_data(values: &[Vector4]) -> Result<Vec<u8>, Error> {
    let frame_count = values.len() as u32;
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Use format 0x4409 for compressed Vector4 data (quaternions)
    data.extend_from_slice(&0x4409u32.to_le_bytes());
    data.extend_from_slice(&frame_count.to_le_bytes());
    data.extend_from_slice(&1.0f32.to_le_bytes()); // unk1 - typically 1.0
    data.extend_from_slice(&0.0f32.to_le_bytes()); // unk2 - varies
    data.extend_from_slice(&2u16.to_le_bytes()); // flags - typically 2

    // For quaternions, we compress XYZ with indices and W with a sign bit
    // Use fewer bits per component for quaternions (8 bits per component = 24 bits + 1 sign bit = 25 bits total)
    let bits_per_entry = 25u16;
    data.extend_from_slice(&bits_per_entry.to_le_bytes());

    // Three default Vector4 quaternion values: first frame, middle frame, last frame
    let first_value = values[0];
    let middle_value = if values.len() > 1 {
        values[values.len() / 2]
    } else {
        first_value
    };
    let last_value = if values.len() > 1 {
        values[values.len() - 1]
    } else {
        first_value
    };

    // Write the three key frame quaternion values
    data.extend_from_slice(&first_value.x.to_le_bytes());
    data.extend_from_slice(&first_value.y.to_le_bytes());
    data.extend_from_slice(&first_value.z.to_le_bytes());
    data.extend_from_slice(&first_value.w.to_le_bytes());

    data.extend_from_slice(&middle_value.x.to_le_bytes());
    data.extend_from_slice(&middle_value.y.to_le_bytes());
    data.extend_from_slice(&middle_value.z.to_le_bytes());
    data.extend_from_slice(&middle_value.w.to_le_bytes());

    data.extend_from_slice(&last_value.x.to_le_bytes());
    data.extend_from_slice(&last_value.y.to_le_bytes());
    data.extend_from_slice(&last_value.z.to_le_bytes());
    data.extend_from_slice(&last_value.w.to_le_bytes());

    // For small frame counts, write raw data
    if frame_count <= 3 {
        for value in values {
            data.extend_from_slice(&value.x.to_le_bytes());
            data.extend_from_slice(&value.y.to_le_bytes());
            data.extend_from_slice(&value.z.to_le_bytes());
            data.extend_from_slice(&value.w.to_le_bytes());
        }
    } else {
        // Compress quaternion frames: XYZ as interpolation indices + W sign bit
        let mut compressed_bits = BitVec::<u8, Lsb0>::new();
        let bits_per_component = 8usize;

        for value in values {
            // Encode X component
            let x_index = calculate_interpolation_index(
                value.x,
                first_value.x,
                middle_value.x,
                last_value.x,
                bits_per_component,
            );
            write_bits(&mut compressed_bits, x_index, bits_per_component);

            // Encode Y component
            let y_index = calculate_interpolation_index(
                value.y,
                first_value.y,
                middle_value.y,
                last_value.y,
                bits_per_component,
            );
            write_bits(&mut compressed_bits, y_index, bits_per_component);

            // Encode Z component
            let z_index = calculate_interpolation_index(
                value.z,
                first_value.z,
                middle_value.z,
                last_value.z,
                bits_per_component,
            );
            write_bits(&mut compressed_bits, z_index, bits_per_component);

            // Encode W sign bit (similar to version 2.0+ quaternion compression)
            compressed_bits.push(value.w.is_sign_negative());
        }

        data.extend_from_slice(&compressed_bits.into_vec());
    }

    Ok(data)
}

/// Create compressed boolean data for version 1.2
/// Uses format 0x1013 for single values or raw boolean data for multiple frames
pub fn create_v12_compressed_bool_data(values: &[bool]) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Check if all values are the same
    let all_same = values.iter().all(|&v| v == values[0]);

    if all_same || values.len() == 1 {
        // Use simple format 0x1013 for constant boolean
        data.extend_from_slice(&0x1013u32.to_le_bytes());
        data.extend_from_slice(&(if values[0] { 1u16 } else { 0u16 }).to_le_bytes());
    } else {
        // For varying boolean values, we could use a compressed bit format
        // For now, write each frame as a u16 (similar to reading logic)
        // In a real implementation, you might want to pack these as bits
        data.extend_from_slice(&0x1013u32.to_le_bytes());
        for &value in values {
            data.extend_from_slice(&(if value { 1u16 } else { 0u16 }).to_le_bytes());
        }
    }

    Ok(data)
}

/// Create uncompressed Vector3 data for version 1.2
/// Uses constant format 0x3003 for single values or raw stream format 0x3400 for multi-frame data
pub fn create_v12_uncompressed_vector3_data(
    values: &[Vec3],
    _property_name: &str,
) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Check if all values are identical (constant track)
    let all_same = values.iter().all(|v| {
        (v.x - values[0].x).abs() < 1e-6
            && (v.y - values[0].y).abs() < 1e-6
            && (v.z - values[0].z).abs() < 1e-6
    });

    if all_same || values.len() == 1 {
        // Use constant format 0x3003
        data.extend_from_slice(&0x3003u32.to_le_bytes());
        data.extend_from_slice(&values[0].x.to_le_bytes());
        data.extend_from_slice(&values[0].y.to_le_bytes());
        data.extend_from_slice(&values[0].z.to_le_bytes());
    } else {
        // Use indexed keyframe format 0x3300 for compatibility.
        // This stores uncompressed f32 Vector3 values with explicit frame indices.
        let key_count = values.len();
        if key_count > u8::MAX as usize + 1 {
            return Err(Error::InvalidFinalFrameIndex {
                final_frame_index: key_count as f32 - 1.0,
            });
        }

        data.extend_from_slice(&0x3300u32.to_le_bytes());
        data.extend_from_slice(&(key_count as u32).to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes()); // unk1 - typically 1.0

        // Frame indices (one byte per key).
        for i in 0..key_count {
            data.push(i as u8);
        }
        // Align to 4 bytes before key values.
        while (data.len() % 4) != 0 {
            data.push(0);
        }

        // Key values as raw f32 data.
        for value in values {
            data.extend_from_slice(&value.x.to_le_bytes());
            data.extend_from_slice(&value.y.to_le_bytes());
            data.extend_from_slice(&value.z.to_le_bytes());
        }
    }

    Ok(data)
}

/// Create uncompressed Vector4 data for version 1.2 (quaternions).
///
/// Uses constant format 0x4003 for single values.
/// For multi-frame data, uses the 0x4300 raw quaternion stream format.
pub fn create_v12_uncompressed_vector4_data(values: &[Quat]) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Normalize all quaternions.
    // Also enforce sign continuity (q and -q represent the same rotation).
    // Many runtimes interpolate quaternions directly, so sign flips can cause visible pops.
    let mut normalized_values: Vec<Vector4> = values
        .iter()
        .map(|q| {
            let magnitude = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
            if magnitude > 1e-6 {
                Vector4 {
                    x: q.x / magnitude,
                    y: q.y / magnitude,
                    z: q.z / magnitude,
                    w: q.w / magnitude,
                }
            } else {
                // Identity quaternion if magnitude is too small
                Vector4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 1.0,
                }
            }
        })
        .collect();

    // Enforce hemisphere continuity relative to the previous frame.
    for i in 1..normalized_values.len() {
        let prev = normalized_values[i - 1];
        let curr = normalized_values[i];
        let dot = prev.x * curr.x + prev.y * curr.y + prev.z * curr.z + prev.w * curr.w;
        if dot < 0.0 {
            normalized_values[i] = Vector4 {
                x: -curr.x,
                y: -curr.y,
                z: -curr.z,
                w: -curr.w,
            };
        }
    }

    // Check if all values are identical (constant track)
    let all_same = normalized_values.iter().all(|v| {
        (v.x - normalized_values[0].x).abs() < 1e-6
            && (v.y - normalized_values[0].y).abs() < 1e-6
            && (v.z - normalized_values[0].z).abs() < 1e-6
            && (v.w - normalized_values[0].w).abs() < 1e-6
    });

    if all_same || normalized_values.len() == 1 {
        // Use constant format 0x4003
        data.extend_from_slice(&0x4003u32.to_le_bytes());
        data.extend_from_slice(&normalized_values[0].x.to_le_bytes());
        data.extend_from_slice(&normalized_values[0].y.to_le_bytes());
        data.extend_from_slice(&normalized_values[0].z.to_le_bytes());
        data.extend_from_slice(&normalized_values[0].w.to_le_bytes());
    } else {
        // Use 0x4300 raw quaternion stream.
        // Layout (validated against tooling):
        // - u32 header (0x4300)
        // - u32 frame_count
        // - f32 unk1 (often 1.0)
        // - f32 unk2 (often 0.0)
        // - frame_count * Vector4<f32> quaternions
        let frame_count = normalized_values.len();

        data.extend_from_slice(&0x4300u32.to_le_bytes());
        data.extend_from_slice(&(frame_count as u32).to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes()); // unk1
        data.extend_from_slice(&0.0f32.to_le_bytes()); // unk2

        for value in &normalized_values {
            data.extend_from_slice(&value.x.to_le_bytes());
            data.extend_from_slice(&value.y.to_le_bytes());
            data.extend_from_slice(&value.z.to_le_bytes());
            data.extend_from_slice(&value.w.to_le_bytes());
        }
    }

    Ok(data)
}

/// Create uncompressed boolean data for version 1.2
/// Uses constant format 0x1013 for single values or raw stream format for multi-frame data
pub fn create_v12_uncompressed_bool_data(values: &[bool]) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Check if all values are identical (constant track)
    let all_same = values.iter().all(|&v| v == values[0]);

    if all_same || values.len() == 1 {
        // Use constant format 0x1013
        data.extend_from_slice(&0x1013u32.to_le_bytes());
        data.extend_from_slice(&(if values[0] { 1u16 } else { 0u16 }).to_le_bytes());
    } else {
        // Use raw stream format - write header and all values
        // Format: 0x1019 header + frame_count + values as u16
        data.extend_from_slice(&0x1019u32.to_le_bytes());
        let frame_count = values.len() as u32;
        data.extend_from_slice(&frame_count.to_le_bytes());

        for &value in values {
            data.extend_from_slice(&(if value { 1u16 } else { 0u16 }).to_le_bytes());
        }
    }

    Ok(data)
}

/// Create uncompressed UV transform data for version 1.2
/// Uses format 0x5014 for constant or raw stream
pub fn create_v12_uncompressed_uv_data(values: &[UvTransform]) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Check if all values are the same
    let all_same = values.iter().all(|v| {
        (v.scale_u - values[0].scale_u).abs() < 1e-6
            && (v.scale_v - values[0].scale_v).abs() < 1e-6
            && (v.rotation - values[0].rotation).abs() < 1e-6
            && (v.translate_u - values[0].translate_u).abs() < 1e-6
            && (v.translate_v - values[0].translate_v).abs() < 1e-6
    });

    if all_same || values.len() == 1 {
        // Use constant format 0x5014
        data.extend_from_slice(&0x5014u32.to_le_bytes());
        let uv = &values[0];
        data.extend_from_slice(&uv.scale_u.to_le_bytes());
        data.extend_from_slice(&uv.scale_v.to_le_bytes());
        data.extend_from_slice(&uv.rotation.to_le_bytes());
        data.extend_from_slice(&uv.translate_u.to_le_bytes());
        data.extend_from_slice(&uv.translate_v.to_le_bytes());
    } else {
        // Use raw stream - write header with frame count, then all values
        // Format: 0x5019 header + frame_count + all UV transforms
        data.extend_from_slice(&0x5019u32.to_le_bytes());
        let frame_count = values.len() as u32;
        data.extend_from_slice(&frame_count.to_le_bytes());

        for uv in values {
            data.extend_from_slice(&uv.scale_u.to_le_bytes());
            data.extend_from_slice(&uv.scale_v.to_le_bytes());
            data.extend_from_slice(&uv.rotation.to_le_bytes());
            data.extend_from_slice(&uv.translate_u.to_le_bytes());
            data.extend_from_slice(&uv.translate_v.to_le_bytes());
        }
    }

    Ok(data)
}

/// Create UV transform data for version 1.2
/// Uses format 0x5014 for UV transformations
pub fn create_v12_compressed_uv_data(values: &[UvTransform]) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    if values.is_empty() {
        return Ok(data);
    }

    // Check if all values are the same
    let all_same = values.iter().all(|v| {
        v.scale_u == values[0].scale_u
            && v.scale_v == values[0].scale_v
            && v.rotation == values[0].rotation
            && v.translate_u == values[0].translate_u
            && v.translate_v == values[0].translate_v
    });

    if all_same || values.len() == 1 {
        // Use simple format 0x5014 for constant UV transform
        data.extend_from_slice(&0x5014u32.to_le_bytes());
        let uv = &values[0];
        data.extend_from_slice(&uv.scale_u.to_le_bytes());
        data.extend_from_slice(&uv.scale_v.to_le_bytes());
        data.extend_from_slice(&uv.rotation.to_le_bytes());
        data.extend_from_slice(&uv.translate_u.to_le_bytes());
        data.extend_from_slice(&uv.translate_v.to_le_bytes());
    } else {
        // For varying UV transforms, write each frame
        // Could potentially use a compressed format similar to Vector3/Vector4
        // For now, write as raw data (format 0x5014 followed by all frames)
        data.extend_from_slice(&0x5014u32.to_le_bytes());
        for uv in values {
            data.extend_from_slice(&uv.scale_u.to_le_bytes());
            data.extend_from_slice(&uv.scale_v.to_le_bytes());
            data.extend_from_slice(&uv.rotation.to_le_bytes());
            data.extend_from_slice(&uv.translate_u.to_le_bytes());
            data.extend_from_slice(&uv.translate_v.to_le_bytes());
        }
    }

    Ok(data)
}
