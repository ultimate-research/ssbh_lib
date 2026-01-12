use glam::{Quat, Vec3};

use crate::anim_data::buffers::v1::*;

use super::{AnimData, GroupType, TrackValues, error};
use ssbh_lib::SsbhByteBuffer;
use ssbh_lib::formats::anim::{Anim, Property, TrackTypeV1, TrackV1};

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
