use std::io::Cursor;

use binrw::{
    BinRead, BinReaderExt,
    helpers::{count_with, until_eof},
};
use glam::{Quat, Vec3};
use ssbh_lib::{Vector3, Vector4, formats::anim::TrackTypeV1};

use crate::{
    anim_data::{
        TrackValues, Transform, UvTransform,
        error::Error,
        v1::{rotate_4409::*, rotate_basic::*, rotate_inferred::*, translate::*},
    },
    read_vec3,
};

// TODO: Organize this in compression.rs similar to version 2.0+
// TODO: Is the magic multiple fields for const, data type, etc?
#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub enum V12BufferData {
    // scalar
    #[br(magic(0x1003u32))]
    Unk1003(u16),

    #[br(magic(0x1013u32))]
    Unk1013(u16),

    // vector2
    #[br(magic(0x2003u32))]
    Unk2003((f32, f32)),

    // vector3
    #[br(magic(0x3003u32))]
    Unk3003(Vector3),

    Unk3300(Unk3300),

    #[br(magic(0x3308u32))]
    Unk3308(Unk3308),

    #[br(magic(0x3309u32))]
    Unk3309(Unk3309),

    #[br(magic(0x3408u32))]
    Unk3408(Unk3408),

    #[br(magic(0x3409u32))]
    Unk3409(Unk3409),

    // vector4
    #[br(magic(0x4003u32))]
    Unk4003(Vector4),

    #[br(magic(0x4300u32))]
    Unk4300(Unk4300),

    #[br(magic(0x4308u32))]
    Unk4308(Unk4308),

    #[br(magic(0x4309u32))]
    Unk4309(Unk4309),

    #[br(magic(0x4408u32))]
    Unk4408(Unk4408),

    #[br(magic(0x4409u32))]
    Unk4409(Unk4409),
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
#[br(magic(0x3300u32))]
pub struct Unk3300 {
    pub frame_count: u32,
    pub unk1: f32,

    #[br(count = frame_count, align_after = 4)] // align to float boundary
    pub frame_indices: Vec<u8>,

    #[br(parse_with = count_with(frame_count as usize, read_vec3))]
    pub values: Vec<Vec3>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
#[br(magic(0x3308u32))]
pub struct Unk3308 {
    pub frame_count: u32,
    pub unk1: f32,

    #[br(count = frame_count, align_after = 4)] // align to float boundary
    pub frame_indices: Vec<u8>,

    pub base_scale: f32,

    #[br(parse_with = read_vec3)]
    pub endpoint0: Vec3,

    #[br(parse_with = read_vec3)]
    pub endpoint1: Vec3,

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk3309 {
    pub frame_count: u32,
    pub unk1: f32,

    #[br(count = frame_count, align_after = 4)] // align to float boundary
    pub frame_indices: Vec<u8>,

    pub unk3: f32,
    pub unk4: u16,
    pub unk5: u16, // TODO: bits per entry?
    pub unk6: [Vector3; 3],

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk3408 {
    pub frame_count: u32,
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: [Vector3; 2],

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk3409 {
    pub frame_count: u32,
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: u16, // 2, 3
    pub unk4: u16, // TODO: bits per entry?

    #[br(if(unk3 == 3))]
    pub unk5: Option<u32>,

    pub unk6: [Vector3; 3],

    #[br(if(unk3 == 3))]
    pub unk7: Option<Vector3>,

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct V12CompressedBlock {
    pub unk1: u32,
    pub unk2: u32,

    #[br(count = v12_compressed_block_value_count(unk2))]
    pub values: Vec<u32>,
}

fn v12_compressed_block_value_count(unk2: u32) -> usize {
    // TODO: 0x00FFFF00 mask determines the count?
    let value = unk2 & 0xFFFFFF00;
    match value {
        0x10000100 => 1,
        0x11002100 => 5,
        0x11003000 => 6,
        0x11020100 => 1,
        0x12000400 => 4,
        0x12000500 => 5,
        0x12000600 => 6,
        0x12000700 => 7,
        0x12000800 => 8,
        0x12001300 => 5,
        0x12001400 => 6,
        0x12001500 => 7,
        0x12001600 => 8,
        0x12001700 => 9,
        0x12002200 => 6,
        0x12002300 => 7,
        0x12002400 => 8,
        0x12002500 => 9,
        0x12002600 => 10,
        0x12003100 => 7,
        0x12003200 => 8,
        0x12003300 => 9,
        0x12003400 => 10,
        0x12003500 => 11,
        0x12004100 => 9,
        0x12004200 => 10,
        0x12006000 => 12,
        0x12004300 => 11,
        0x12004400 => 12,
        0x12005100 => 11,
        0x12005200 => 12,
        0x12005300 => 13,
        0x12006100 => 13,
        0x12006200 => 14,
        0x12007000 => 14,
        0x12007100 => 15,
        0x12008000 => 16,
        0x12040100 => 1,
        0x12060100 => 1,
        0x12200200 => 3,
        0x12200300 => 4,
        0x12200400 => 5,
        0x12200500 => 6,
        0x12200600 => 7,
        0x12220000 => 1,
        0x12220100 => 2,
        0x12240000 => 1,
        0x12240100 => 2,
        0x12260000 => 1,
        0x12400000 => 2,
        0x12400100 => 3,
        0x12400200 => 4,
        0x12400300 => 5,
        0x12400400 => 6,
        0x12420000 => 2,
        0x12420100 => 3,
        0x12004000 => 8,
        0x12005000 => 10,
        0x12440000 => 2,
        0x12600000 => 3,
        0x12600100 => 4,
        0x12600200 => 5,
        0x12620000 => 3,
        // TODO: Should these all match bytes like above?
        _ => match unk2 {
            0x100010FF => 2,
            0x110002FF => 2,
            0x110003FF => 3,
            0x110011FF => 3,
            0x110012FF => 4,
            0x110020FF => 4,
            0x11020001 => 0,
            0x110200FF => 0,
            0x112000FF => 1,
            0x112001FF => 2,
            0x12002542 => 9,
            0x1200313A => 7,
            0x12003141 => 6,
            0x12003358 => 8,
            0x12003442 => 10,
            0x120043D1 => 11,
            0x12006057 => 12,
            0x12040001 => 0,
            0x12080001 => 0,
            0x12080050 => 0,
            0x12080060 => 0,
            0x1280002E => 4,
            _ => 0,
        },
    }
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk4300 {
    pub frame_count: u32,
    pub unk1: f32,

    #[br(count = frame_count, align_after = 4)] // align to float boundary
    pub frame_indices: Vec<u8>,

    #[br(count = frame_count)]
    pub values: Vec<Vector4>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk4308 {
    pub frame_count: u32,
    pub unk1: f32,

    #[br(count = frame_count, align_after = 4)] // align to float boundary
    pub frame_indices: Vec<u8>,

    pub base_scale: f32,
    pub endpoint0: Vector4,
    pub endpoint1: Vector4,

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk4309 {
    pub frame_count: u32,
    pub unk1: f32,
    #[br(count = frame_count, align_after = 4)] // align to float boundary
    pub unk2: Vec<u8>,

    pub unk3: f32,
    pub unk4: u16, // 2, 3
    pub unk5: u16, // TODO: bits per entry?

    #[br(if(unk4 == 3))]
    pub unk7: Option<u32>,

    pub unk6: [Vector4; 3], // TODO: quaternions?

    #[br(if(unk4 == 3))]
    pub unk8: Option<Vector4>,

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk4408 {
    pub frame_count: u32,
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: [Vector4; 2],

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
pub struct Unk4409 {
    pub frame_count: u32,
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: u16, // 2, 3
    pub unk4: u16, // TODO: bits per entry?

    #[br(if(unk3 == 3))]
    pub unk5: Option<u32>,

    pub unk6: [Vector4; 3],

    #[br(if(unk3 == 3))]
    pub unk7: Option<Vector4>,

    #[br(parse_with = until_eof)]
    pub values: Vec<V12CompressedBlock>,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum V12Values {
    Uint(u16),
    Float(f32),
    Vec2((f32, f32)),
    Vec3(Vec<Vec3>),
    Quat(Vec<Quat>),
}

struct PropertyData {
    scales: Vec<Vec3>,
    rotations: Vec<Quat>,
    translations: Vec<Vec3>,
    visibilities: Vec<bool>,
    uv_transforms: Vec<UvTransform>,
}
pub fn read_track_values_v12(
    track: &ssbh_lib::formats::anim::TrackV1,
    buffers: &[ssbh_lib::SsbhByteBuffer],
    animation_frame_count: usize,
) -> Result<(TrackValues, bool), Error> {
    let mut compensate_scale = false;
    // Collect parsed property data
    let mut property_data = PropertyData {
        scales: Vec::new(),
        rotations: Vec::new(),
        translations: Vec::new(),
        visibilities: Vec::new(),
        uv_transforms: Vec::new(),
    };
    for property in &track.properties.elements {
        let property_name = property.name.to_string_lossy();
        let data =
            buffers
                .get(property.buffer_index as usize)
                .ok_or(Error::BufferIndexOutOfRange {
                    buffer_index: property.buffer_index as usize,
                    buffer_count: buffers.len(),
                })?;

        add_property(
            &mut compensate_scale,
            &mut property_data,
            property_name,
            data,
        )?;
    }

    let values = match track.track_type {
        TrackTypeV1::Transform => {
            let mut transforms = Vec::new();

            for frame_idx in 0..animation_frame_count {
                let scale = property_data
                    .scales
                    .get(frame_idx)
                    .copied()
                    .unwrap_or_else(|| property_data.scales.last().copied().unwrap_or(Vec3::ONE));

                let rotation = property_data
                    .rotations
                    .get(frame_idx)
                    .copied()
                    .unwrap_or_else(|| {
                        property_data
                            .rotations
                            .last()
                            .copied()
                            .unwrap_or(Quat::IDENTITY)
                    });

                let translation = property_data
                    .translations
                    .get(frame_idx)
                    .copied()
                    .unwrap_or_else(|| {
                        property_data
                            .translations
                            .last()
                            .copied()
                            .unwrap_or(Vec3::ZERO)
                    });

                transforms.push(Transform {
                    scale: scale.to_array().into(),
                    rotation: Quat::from_array(rotation.to_array()),
                    translation: translation.to_array().into(),
                });
            }

            // If no frames were generated, create a default identity transform
            if transforms.is_empty() {
                transforms.push(Transform::IDENTITY);
            }

            TrackValues::Transform(transforms)
        }
        TrackTypeV1::Visibility => {
            if property_data.visibilities.is_empty() {
                TrackValues::Boolean(vec![true])
            } else {
                TrackValues::Boolean(property_data.visibilities)
            }
        }
        TrackTypeV1::UvTransform => {
            if property_data.uv_transforms.is_empty() {
                TrackValues::UvTransform(vec![UvTransform {
                    scale_u: 1.0,
                    scale_v: 1.0,
                    rotation: 0.0,
                    translate_u: 0.0,
                    translate_v: 0.0,
                }])
            } else {
                TrackValues::UvTransform(property_data.uv_transforms)
            }
        }
    };
    Ok((values, compensate_scale))
}

fn add_property(
    compensate_scale: &mut bool,
    property_data: &mut PropertyData,
    property_name: String,
    data: &ssbh_lib::SsbhByteBuffer,
) -> Result<(), Error> {
    let value = read_property_value_v12(&data.elements)?;
    match property_name.as_str() {
        "CompensateScale" => match value {
            V12Values::Uint(v) => {
                *compensate_scale = v != 0;
            }
            V12Values::Float(f) => {
                *compensate_scale = f != 0.0;
            }
            _ => {}
        },
        "Scale" => {
            match value {
                V12Values::Vec3(values) => {
                    property_data.scales.extend(values);
                }
                _ => {
                    // For complex scale formats, use default
                    property_data.scales.push(Vec3::ONE);
                }
            }
        }
        "Rotate" => {
            match value {
                V12Values::Vec3(values) => {
                    // Single Vector3 (euler angles, convert to quaternion) - uncompressed
                    property_data
                        .rotations
                        .extend(values.into_iter().map(|v| euler_to_quaternion(v.into())));
                }
                V12Values::Quat(values) => {
                    property_data.rotations.extend(values);
                }
                _ => {
                    // Unknown format, use default identity rotation
                    property_data.rotations.push(Quat::IDENTITY);
                }
            }
        }
        "Translate" => {
            match value {
                V12Values::Vec3(values) => {
                    property_data.translations.extend(values);
                }
                _ => {
                    // Unknown format
                    property_data.translations.push(Vec3::ZERO);
                }
            }
        }
        "Visibility" => match value {
            V12Values::Uint(v) => {
                property_data.visibilities.push(v != 0);
            }
            _ => {
                property_data.visibilities.push(true);
            }
        },
        _ => {
            // Handle other unknown properties
        }
    }
    Ok(())
}

fn read_property_value_v12(bytes: &[u8]) -> Result<V12Values, Error> {
    let mut reader = Cursor::new(bytes);
    let header: u32 = reader.read_le()?;

    // reader.rewind()?;
    // let buffer_data: V12BufferData = reader.read_le()?;
    // TODO: use V12BufferData data
    let value = match header {
        0x1013 => {
            let value: u16 = reader.read_le()?;
            V12Values::Uint(value)
        }
        0x1003 => {
            let value: f32 = reader.read_le()?;
            V12Values::Float(value)
        }
        0x3003 => {
            // Single scale value
            let scale = reader.read_le::<Vector3>()?;
            V12Values::Vec3(vec![scale.into()])
        }
        0x3200 => V12Values::Vec3(decode_translate_3200(bytes)?),
        0x3208 => V12Values::Vec3(decode_translate_3208(bytes)?),
        0x3209 => V12Values::Vec3(decode_translate_3209(bytes)?),
        0x3300 => V12Values::Vec3(decode_translate_3300(bytes)?),
        0x3308 => V12Values::Vec3(decode_translate_3308(bytes)?),
        0x3309 => V12Values::Vec3(decode_translate_3309(bytes)?),
        0x3400 => V12Values::Vec3(decode_translate_3400(bytes)?),
        0x3408 => V12Values::Vec3(decode_translate_3408(bytes)?),
        0x3409 => V12Values::Vec3(decode_vector3_3409(bytes)?),
        0x4003 => {
            // Single Vector4 (quaternion rotation) - uncompressed
            let rotation = reader.read_le::<[f32; 4]>()?;
            V12Values::Quat(vec![Quat::from_array(rotation)])
        }
        0x4200 => V12Values::Quat(decode_rotate_4200(bytes)?),
        0x4208 => V12Values::Quat(decode_rotate_4208(bytes)?),
        0x4209 => V12Values::Quat(decode_rotate_4209(bytes)?),
        0x4300 => V12Values::Quat(decode_rotate_4300(bytes)?),
        0x4308 => V12Values::Quat(decode_rotate_4308(bytes)?),
        0x4309 => V12Values::Quat(decode_rotate_4309(bytes)?),
        0x4400 => V12Values::Quat(decode_rotate_4400(bytes)?),
        0x4408 => V12Values::Quat(decode_rotate_4408(bytes)?),
        0x4409 => V12Values::Quat(decode_rotate_4409(bytes)?),
        _ => {
            // Handle other unknown properties
            todo!()
        }
    };
    Ok(value)
}

// TODO: use glam for this.
fn euler_to_quaternion(euler: Vector3) -> Quat {
    let (sx, cx) = (euler.x * 0.5).sin_cos();
    let (sy, cy) = (euler.y * 0.5).sin_cos();
    let (sz, cz) = (euler.z * 0.5).sin_cos();

    Quat::from_xyzw(
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
        cx * cy * cz + sx * sy * sz,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{quat, vec3};
    use hexlit::hex;
    use pretty_assertions::assert_eq;

    // TODO: tests for entire 1.2 track with properties
    // TODO: One test for each anim 1.2 buffer variant.
    #[test]
    fn read_anim_v12_0330() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_15winloop01_sht_gnd_fr.nuanmb, GBL_RT, Scale
        let data = hex!(03300000 0000803f 0000803f 0000803f);
        assert_eq!(
            V12Values::Vec3(vec![vec3(1.0, 1.0, 1.0)]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0033() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_javelinloop_sht_air_fr.nuanmb, BASE, Translate
        let data = hex!(
            00330000
            02000000
            0000803f 000f0000 46942abf 90d83e3e 00000000 46942abf 4dd83e3e 00000000
        );
        assert_eq!(
            V12Values::Vec3(vec![
                vec3(-0.666325, 0.186373, 0.0),
                vec3(-0.666325, 0.18637294, 0.0),
                vec3(-0.666325, 0.18637286, 0.0),
                vec3(-0.666325, 0.1863728, 0.0),
                vec3(-0.666325, 0.18637273, 0.0),
                vec3(-0.666325, 0.18637267, 0.0),
                vec3(-0.666325, 0.1863726, 0.0),
                vec3(-0.666325, 0.18637253, 0.0),
                vec3(-0.666325, 0.18637246, 0.0),
                vec3(-0.666325, 0.1863724, 0.0),
                vec3(-0.666325, 0.18637232, 0.0),
                vec3(-0.666325, 0.18637227, 0.0),
                vec3(-0.666325, 0.18637219, 0.0),
                vec3(-0.666325, 0.18637213, 0.0),
                vec3(-0.666325, 0.18637206, 0.0),
                vec3(-0.666325, 0.186372, 0.0),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0833() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_15winloop01_sht_gnd_fr.nuanmb, BASE, Translate
        let data = hex!(
            08330000
            0c000000
            0000803f
            0004080c
            1118252b
            3034383b
            baf6a63c
            7bbf313e
            3220bb3f
            4a97bebe
            7bbf313e
            6cecba3f
            4a97bebe
            01000100
            01010211
            00000000
            ffffd615
            ff012011
            137ffcbf
            0x4188ea5c
            01000100
            01010211
            00000000
        );
        assert_eq!(
            V12Values::Vec3(vec![
                vec3(0.173582, 1.46192, -0.372248),
                vec3(0.173582, 1.4631298, -0.372248),
                vec3(0.173582, 1.4643394, -0.372248),
                vec3(0.173582, 1.465549, -0.372248),
                vec3(0.173582, 1.4667587, -0.372248),
                vec3(0.173582, 1.4673619, -0.372248),
                vec3(0.173582, 1.4679651, -0.372248),
                vec3(0.173582, 1.4685682, -0.372248),
                vec3(0.173582, 1.4691714, -0.372248),
                vec3(0.173582, 1.4693661, -0.372248),
                vec3(0.173582, 1.4695609, -0.372248),
                vec3(0.173582, 1.4697555, -0.372248),
                vec3(0.173582, 1.4699502, -0.372248),
                vec3(0.173582, 1.4696864, -0.372248),
                vec3(0.173582, 1.4694226, -0.372248),
                vec3(0.173582, 1.4691588, -0.372248),
                vec3(0.173582, 1.468895, -0.372248),
                vec3(0.173582, 1.4686311, -0.372248),
                vec3(0.173582, 1.4680263, -0.372248),
                vec3(0.173582, 1.4674214, -0.372248),
                vec3(0.173582, 1.4668165, -0.372248),
                vec3(0.173582, 1.4662117, -0.372248),
                vec3(0.173582, 1.4656068, -0.372248),
                vec3(0.173582, 1.465002, -0.372248),
                vec3(0.173582, 1.4643971, -0.372248),
                vec3(0.173582, 1.4636115, -0.372248),
                vec3(0.173582, 1.4628259, -0.372248),
                vec3(0.173582, 1.4620403, -0.372248),
                vec3(0.173582, 1.4612546, -0.372248),
                vec3(0.173582, 1.460469, -0.372248),
                vec3(0.173582, 1.4596834, -0.372248),
                vec3(0.173582, 1.4588978, -0.372248),
                vec3(0.173582, 1.4581122, -0.372248),
                vec3(0.173582, 1.4573267, -0.372248),
                vec3(0.173582, 1.456541, -0.372248),
                vec3(0.173582, 1.4557554, -0.372248),
                vec3(0.173582, 1.4549698, -0.372248),
                vec3(0.173582, 1.4541842, -0.372248),
                vec3(0.173582, 1.4536883, -0.372248),
                vec3(0.173582, 1.4531924, -0.372248),
                vec3(0.173582, 1.4526963, -0.372248),
                vec3(0.173582, 1.4522004, -0.372248),
                vec3(0.173582, 1.4517045, -0.372248),
                vec3(0.173582, 1.4512086, -0.372248),
                vec3(0.173582, 1.4511329, -0.372248),
                vec3(0.173582, 1.4510573, -0.372248),
                vec3(0.173582, 1.4509816, -0.372248),
                vec3(0.173582, 1.450906, -0.372248),
                vec3(0.173582, 1.4508303, -0.372248),
                vec3(0.173582, 1.4512398, -0.372248),
                vec3(0.173582, 1.4516493, -0.372248),
                vec3(0.173582, 1.4520588, -0.372248),
                vec3(0.173582, 1.4524683, -0.372248),
                vec3(0.173582, 1.4533961, -0.372248),
                vec3(0.173582, 1.4543238, -0.372248),
                vec3(0.173582, 1.4552516, -0.372248),
                vec3(0.173582, 1.4561794, -0.372248),
                vec3(0.173582, 1.4575663, -0.372248),
                vec3(0.173582, 1.4589531, -0.372248),
                vec3(0.173582, 1.46034, -0.372248),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0933() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_guardloop_stk_air_fr.nuanmb, BASE, Translate
        let data = hex!(
            09330000
            2a000000
            0000803f
            00010203040507090b0e1417191b1c1d1e1f2021222324252627282a2c2e31373a3c3e3f4041424344450000
            f00f063f
            0200
            0e00
            00000000 45f0bf3f 00000000
            00000000 eacabb3f 00000000
            00000000 0000c03f 00000000
            01000100 01000812 ffffef1d46080012837f030e81cfd50d230e09ebeec2390150ec08c8effd1420fdffde07f226f720
            01000100 01000812
            01000100 01000211
            150a580a ff002011 f17c738c
            01000100 01000211
        );
        assert_eq!(
            V12Values::Vec3(vec![
                vec3(0.0, 1.49952, 0.0),
                vec3(0.0, 1.4976515, 0.0),
                vec3(0.0, 1.4953662, 0.0),
                vec3(0.0, 1.4924352, 0.0),
                vec3(0.0, 1.4888555, 0.0),
                vec3(0.0, 1.4843782, 0.0),
                vec3(0.0, 1.4790397, 0.0),
                vec3(0.0, 1.4737012, 0.0),
                vec3(0.0, 1.4672358, 0.0),
                vec3(0.0, 1.4607704, 0.0),
                vec3(0.0, 1.4534283, 0.0),
                vec3(0.0, 1.446086, 0.0),
                vec3(0.0, 1.4380066, 0.0),
                vec3(0.0, 1.4299271, 0.0),
                vec3(0.0, 1.4218477, 0.0),
                vec3(0.0, 1.4133819, 0.0),
                vec3(0.0, 1.4049162, 0.0),
                vec3(0.0, 1.3964503, 0.0),
                vec3(0.0, 1.3879845, 0.0),
                vec3(0.0, 1.3795187, 0.0),
                vec3(0.0, 1.371053, 0.0),
                vec3(0.0, 1.363187, 0.0),
                vec3(0.0, 1.3553208, 0.0),
                vec3(0.0, 1.3474548, 0.0),
                vec3(0.0, 1.3404669, 0.0),
                vec3(0.0, 1.3334789, 0.0),
                vec3(0.0, 1.327329, 0.0),
                vec3(0.0, 1.3211792, 0.0),
                vec3(0.0, 1.3159728, 0.0),
                vec3(0.0, 1.3113593, 0.0),
                vec3(0.0, 1.3073915, 0.0),
                vec3(0.0, 1.3042191, 0.0),
                vec3(0.0, 1.3016889, 0.0),
                vec3(0.0, 1.3001517, 0.0),
                vec3(0.0, 1.299547, 0.0),
                vec3(0.0, 1.3001189, 0.0),
                vec3(0.0, 1.301717, 0.0),
                vec3(0.0, 1.3040853, 0.0),
                vec3(0.0, 1.30725, 0.0),
                vec3(0.0, 1.3111147, 0.0),
                vec3(0.0, 1.3157693, 0.0),
                vec3(0.0, 1.3212559, 0.0),
                vec3(0.0, 1.3267424, 0.0),
                vec3(0.0, 1.3333225, 0.0),
                vec3(0.0, 1.3399025, 0.0),
                vec3(0.0, 1.3472242, 0.0),
                vec3(0.0, 1.354546, 0.0),
                vec3(0.0, 1.3626473, 0.0),
                vec3(0.0, 1.3707488, 0.0),
                vec3(0.0, 1.3788501, 0.0),
                vec3(0.0, 1.387325, 0.0),
                vec3(0.0, 1.3958, 0.0),
                vec3(0.0, 1.404275, 0.0),
                vec3(0.0, 1.4127499, 0.0),
                vec3(0.0, 1.4212248, 0.0),
                vec3(0.0, 1.4296998, 0.0),
                vec3(0.0, 1.4375246, 0.0),
                vec3(0.0, 1.4453492, 0.0),
                vec3(0.0, 1.453174, 0.0),
                vec3(0.0, 1.4601519, 0.0),
                vec3(0.0, 1.46713, 0.0),
                vec3(0.0, 1.4728534, 0.0),
                vec3(0.0, 1.478577, 0.0),
                vec3(0.0, 1.484428, 0.0),
                vec3(0.0, 1.489633, 0.0),
                vec3(0.0, 1.4934063, 0.0),
                vec3(0.0, 1.4957992, 0.0),
                vec3(0.0, 1.4975187, 0.0),
                vec3(0.0, 1.4988009, 0.0),
                vec3(0.0, 1.5, 0.0),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0834() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_jumpbgn_sht_gnd_fr.nuanmb, BASE, Translate
        let data = hex!(
            08340000
            0a000000
            0000803f
            0x4159883e
            00000000 df4f3d40 00000000
            00000000 0000c03f 00000000
            01000100 01000211
            ffff5b22 ff020011d 77fd311814bae46
            01000100 01000211
        );
        assert_eq!(
            V12Values::Vec3(vec![
                vec3(0.26630595, 0.0, 2.958),
                vec3(0.2367164, 0.0, 2.796),
                vec3(0.20712686, 0.0, 2.6339998),
                vec3(0.17753729, 0.0, 2.472),
                vec3(0.14794776, 0.0, 2.31),
                vec3(0.118358195, 0.0, 2.148),
                vec3(0.088768646, 0.0, 1.986),
                vec3(0.059179097, 0.0, 1.824),
                vec3(0.029589549, 0.0, 1.662),
                vec3(0.0, 0.0, 1.5),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0934() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_kakubb01a_stk_air_fr.nuanmb, GBL_RT, Translate
        let data = hex!(
            09340000
            28000000
            0000803f
            0x51ec9d40
            0200
            0f00
            00000000 00000000 9f8ea33f
            00000000 00000000 f94f4a42
            00000000 00000000 0x6ea37042
            01000100 01000812
            01000100 01000812
            ffff2e0b 52170012 0180ba3692129e0b7f71393f1f29121d234316340d2a0722031dff18fc15fa12f80ff50d
            01000100 01000211
            01000100 01000211
            8a004419 ff002011 fd81578c
        );
        assert_eq!(
            V12Values::Vec3(vec![
                vec3(0.0, 0.0, 1.27779),
                vec3(0.0, 0.0, 2.5748847),
                vec3(0.0, 0.0, 3.8888035),
                vec3(0.0, 0.0, 5.2206244),
                vec3(0.0, 0.0, 6.569663),
                vec3(0.0, 0.0, 7.9355483),
                vec3(0.0, 0.0, 9.316959),
                vec3(0.0, 0.0, 10.713781),
                vec3(0.0, 0.0, 12.125164),
                vec3(0.0, 0.0, 13.550728),
                vec3(0.0, 0.0, 14.990418),
                vec3(0.0, 0.0, 16.442362),
                vec3(0.0, 0.0, 17.907713),
                vec3(0.0, 0.0, 19.384544),
                vec3(0.0, 0.0, 20.872856),
                vec3(0.0, 0.0, 22.372204),
                vec3(0.0, 0.0, 23.881601),
                vec3(0.0, 0.0, 25.40035),
                vec3(0.0, 0.0, 26.928825),
                vec3(0.0, 0.0, 28.465826),
                vec3(0.0, 0.0, 30.010803),
                vec3(0.0, 0.0, 31.563925),
                vec3(0.0, 0.0, 33.12375),
                vec3(0.0, 0.0, 34.68984),
                vec3(0.0, 0.0, 36.26108),
                vec3(0.0, 0.0, 37.838257),
                vec3(0.0, 0.0, 39.419827),
                vec3(0.0, 0.0, 41.0059),
                vec3(0.0, 0.0, 42.59557),
                vec3(0.0, 0.0, 44.188164),
                vec3(0.0, 0.0, 45.78313),
                vec3(0.0, 0.0, 47.380527),
                vec3(0.0, 0.0, 48.97861),
                vec3(0.0, 0.0, 50.5781),
                vec3(0.0, 0.0, 52.17822),
                vec3(0.0, 0.0, 53.776844),
                vec3(0.0, 0.0, 55.374786),
                vec3(0.0, 0.0, 56.972008),
                vec3(0.0, 0.0, 58.567272),
                vec3(0.0, 0.0, 60.1596),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0340() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_kakun31b_stk_air_fr.nuanmb, TE_R, Rotate
        let data = hex!(
            03400000
            00000000 00000000 f70435bf f704353f
        );
        assert_eq!(
            V12Values::Quat(vec![quat(0.0, 0.0, -0.707107, 0.707107)]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0043() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_kakun31b_stk_air_fr.nuanmb, TE_R, Rotate
        let data = hex!(
            00430000
            03000000
            0000803f
            00014000
            352a1c3f 3717b73e 0x9373023e a60e323f
            69c41c3f 0805b53e 4678fb3d e944323f
            69c41c3f 0805b53e 4678fb3d e944323f
        );
        assert_eq!(
            V12Values::Quat(vec![
                quat(0.6100191, 0.35759902, 0.12739402, 0.6955361),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
                quat(0.6123721, 0.3535541, 0.12278803, 0.69636416),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0843() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_bkmaintypebbgn_sht_gnd_bk.nuanmb, TE_R, Rotate
        let data = hex!(
            08430000
            06000000
            0000803f
            0005060708090000
            3945c13d
            e944323f 4678fb3d 4678fbbd e944323f
            eb004c3f 0f0e16be 8e5780bd 662d153f
            b3470100 ff010010 eb81eb07
            ffff0100 ff100010 a917ff7f
            bacb020e 57900100 ff01001032811a07
            483d0100 ff010010 7f57e4fc
        );
        assert_eq!(
            V12Values::Quat(vec![
                quat(0.6963642, 0.12278804, -0.12278804, 0.6963642),
                quat(0.696367, 0.12278788, -0.12279732, 0.6963598),
                quat(0.69636977, 0.122787714, -0.1228066, 0.69635546),
                quat(0.69637257, 0.12278756, -0.12281588, 0.69635105),
                quat(0.6963753, 0.12278739, -0.12282516, 0.6963467),
                quat(0.6963781, 0.12278723, -0.12283444, 0.6963423),
                quat(0.74012196, 0.034967255, -0.08779142, 0.6658),
                quat(0.7735146, -0.09664208, -0.038356192, 0.6251914),
                quat(0.78298694, -0.15716107, -0.04652815, 0.60005593),
                quat(0.7968891, -0.14653802, -0.062667005, 0.58272403)
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0943() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_guardloop_stk_air_fr.nuanmb, HIZA_L, Rotate
        let data = hex!(
            09430000
            2e000000
            0000803f
            00030b0e101214161718191a1b1c1d1e1f2021222324252627292b2e3234363738393a3b3c3d3e3f4041424344450000
            178db83d
            0200
            1700
            00000000 00000000 2e765f3f 0x8ecef93e
            00000000 00000000 c1545f3f 3946fa3e
            00000000 00000000 e3545f3f b345fa3e
            01000100 01000812
            01000100 01000812
            908b1c19 4c062012 7fd0f5fa81f5abecdadeefddcba1c2bac3d2cadfd8dbecd587687878
            ffff5419 4a080012 81300b067f0b54142522102236603f473e2e37212825142c08220a0d0c0008fc
            01000100 01010211 00000000
            01000100 01010211 00000000
            9b089e0e ff010211 813e1005
            4f0f9f0e ff012011 7fc2f0fb158a469a
        );
        assert_eq!(
            V12Values::Quat(vec![
                quat(0.0, 0.872897, 0.487904, 0.00064593536),
                quat(0.0, 0.8688187, 0.49510816, 0.0046872864),
                quat(0.0, 0.8646663, 0.5022709, 0.008728582),
                quat(0.0, 0.86044085, 0.5093904, 0.012768795),
                quat(0.0, 0.8586344, 0.5124061, 0.013677419),
                quat(0.0, 0.85681653, 0.51541525, 0.014585937),
                quat(0.0, 0.85498714, 0.5184178, 0.015494309),
                quat(0.0, 0.8531465, 0.5214136, 0.0164025),
                quat(0.0, 0.8512945, 0.52440244, 0.017310474),
                quat(0.0, 0.8494315, 0.5273844, 0.018218199),
                quat(0.0, 0.8475573, 0.53035915, 0.019125633),
                quat(0.0, 0.84567213, 0.53332675, 0.020032745),
                quat(0.0, 0.8408638, 0.54069966, 0.024330245),
                quat(0.0, 0.83597434, 0.5480215, 0.028625825),
                quat(0.0, 0.8310051, 0.55529, 0.032918245),
                quat(0.0, 0.8232663, 0.5662363, 0.04011462),
                quat(0.0, 0.8153369, 0.5770515, 0.047301713),
                quat(0.0, 0.8070291, 0.58796704, 0.054762125),
                quat(0.0, 0.79852456, 0.59873915, 0.062209185),
                quat(0.0, 0.78956157, 0.60967124, 0.06995359),
                quat(0.0, 0.7803935, 0.6204449, 0.07767981),
                quat(0.0, 0.7708237, 0.6312572, 0.08570428),
                quat(0.0, 0.76104355, 0.6418972, 0.09370536),
                quat(0.0, 0.7404698, 0.66289186, 0.110810235),
                quat(0.0, 0.7185109, 0.68359363, 0.12822568),
                quat(0.0, 0.6953205, 0.7037342, 0.1459028),
                quat(0.0, 0.6706664, 0.7234575, 0.16375558),
                quat(0.0, 0.6448937, 0.7423311, 0.18181539),
                quat(0.0, 0.61778027, 0.76050836, 0.19993664),
                quat(0.0, 0.5895789, 0.77771425, 0.2180762),
                quat(0.0, 0.56032544, 0.7938988, 0.23613596),
                quat(0.0, 0.5300951, 0.80899227, 0.25402915),
                quat(0.0, 0.49907446, 0.8228529, 0.2717314),
                quat(0.0, 0.4673435, 0.8354598, 0.28913134),
                quat(0.0, 0.43504, 0.8467623, 0.30615994),
                quat(0.0, 0.40235916, 0.8566783, 0.32281485),
                quat(0.0, 0.36939037, 0.86526513, 0.33892062),
                quat(0.0, 0.3363241, 0.8724802, 0.35449198),
                quat(0.0, 0.30331138, 0.87834555, 0.3694745),
                quat(0.0, 0.27042302, 0.88295436, 0.38374865),
                quat(0.0, 0.25420725, 0.8845052, 0.3911896),
                quat(0.0, 0.23790997, 0.8857723, 0.39850503),
                quat(0.0, 0.22187188, 0.8867122, 0.4056035),
                quat(0.0, 0.20576538, 0.8873787, 0.4125769),
                quat(0.0, 0.19526592, 0.8874301, 0.4175395),
                quat(0.0, 0.18473867, 0.88736165, 0.42244643),
                quat(0.0, 0.17418794, 0.88717365, 0.42729574),
                quat(0.0, 0.16646233, 0.88681215, 0.43110844),
                quat(0.0, 0.1587232, 0.8863847, 0.4348897),
                quat(0.0, 0.15097228, 0.8858913, 0.43863863),
                quat(0.0, 0.14321129, 0.8853323, 0.4423544),
                quat(0.0, 0.12808481, 0.884686, 0.44824666),
                quat(0.0, 0.11292452, 0.88380617, 0.45402062),
                quat(0.0, 0.09817763, 0.8827847, 0.4594042),
                quat(0.0, 0.08340645, 0.8815449, 0.46467412),
                quat(0.0, 0.054722697, 0.87909, 0.47350422),
                quat(0.0, 0.026908446, 0.8760376, 0.48149166),
                quat(0.0, 0.0, 0.872387, 0.48881587),
                quat(0.042647254, 0.0, 0.8367617, 0.5459039),
                quat(0.088832006, 0.0, 0.79201764, 0.60400087),
                quat(0.13786158, 0.0, 0.7371303, 0.66153854),
                quat(0.18861501, 0.0, 0.671916, 0.7162076),
                quat(0.23956299, 0.0, 0.5970046, 0.7656339),
                quat(0.2889903, 0.0, 0.5141156, 0.8075703),
                quat(0.33521053, 0.0, 0.4257438, 0.84046185),
                quat(0.37687343, 0.0, 0.33490375, 0.8636006),
                quat(0.41314897, 0.0, 0.24463291, 0.87719023),
                quat(0.44372296, 0.0, 0.15753129, 0.8822097),
                quat(0.46877572, 0.0, 0.07553856, 0.8800814),
                quat(0.488817, 0.0, 0.0, 0.87238634),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0844() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_napalm22a_sht_air_fr.nuanmb, KUBI, Rotate
        let data = hex!(
            08440000
            19000000
            0000803f
            442a3b3c
            6f8dfe3c 0x28918ebc d0d90d3a 62d67f3f
            e620683d 49be0ebc 5ca5013a 31947f3f
            ffff9e09 81024012 46810a08547f175687cd86bc66bb66bb
            95386e15 5c002412 8f1dba8b
            5705f70c 29000612
            9208940d 78002412 71f16674
        );
        assert_eq!(
            V12Values::Quat(vec![
                quat(0.031073315, -0.017403208, 0.0005411182, 0.99936545),
                quat(0.03054142, -0.017799487, 0.000539159, 0.9993748),
                quat(0.030624479, -0.01764455, 0.0005372265, 0.9993751),
                quat(0.031447694, -0.017574375, 0.0005352738, 0.9993507),
                quat(0.0328979, -0.017424677, 0.00053331564, 0.99930674),
                quat(0.034844197, -0.017086914, 0.00053136237, 0.9992466),
                quat(0.036971185, -0.016558824, 0.00052942, 0.99917895),
                quat(0.039496582, -0.015917545, 0.00052747846, 0.99909276),
                quat(0.04217451, -0.015247832, 0.0005255359, 0.9989938),
                quat(0.045002557, -0.014583862, 0.00052358897, 0.99888027),
                quat(0.047768462, -0.013906293, 0.0005216427, 0.99876153),
                quat(0.050444707, -0.013183726, 0.0005196983, 0.99863964),
                quat(0.052918985, -0.012415022, 0.00051775714, 0.99852157),
                quat(0.055207033, -0.011634497, 0.0005158149, 0.998407),
                quat(0.057179917, -0.010882226, 0.0005138721, 0.9983044),
                quat(0.058870815, -0.01017324, 0.0005119271, 0.9982137),
                quat(0.060107682, -0.009498919, 0.0005099869, 0.9981466),
                quat(0.060942836, -0.00885883, 0.00050805166, 0.9981018),
                quat(0.06143587, -0.00829116, 0.0005061192, 0.99807644),
                quat(0.061567903, -0.007867997, 0.0005041876, 0.99807173),
                quat(0.061233427, -0.0076538944, 0.00050225813, 0.9980941),
                quat(0.06061798, -0.0076605165, 0.00050032657, 0.9981315),
                quat(0.059618436, -0.007836375, 0.00049840077, 0.9981904),
                quat(0.05829708, -0.00809982, 0.0004964837, 0.9982663),
                quat(0.056671984, -0.008712358, 0.0004945599, 0.99835473),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }

    #[test]
    fn read_anim_v12_0944() {
        // 001gundam_001gundam_001/001hito_001gundam_001gundam_001_guardloop_sht_air_fr.nuanmb, ASHI_L, Rotate
        let data = hex!(
            09440000
            46000000
            0000803f
            dab0cd3b
            0300
            1000
            20000000
            247d5abc e0200cbc 810a37be 27d87b3f
            7a7a1dbc 2278b6bb 69704bbe 7ae17a3f
            07c861bc a11813bc ce0037be f5d77b3f
            c5e25cbc c9580ebc 9ce136be cbd97b3f
            c8719109 63002612 fb219a74
            6769b609 62002612 f9218974
            ffffab22 4b026012 ff7fc40281fdc3ff648975896699679977997799
            9b2bd128 49002612 91f84588
            76607513 50002612 41fa6589
            49541e13 50002612 41fa6589
            bce9df2f 48026012 81b1510a7f1e3c10ab999b88a988a98899879987
            77279d34 49002612 1f2adb99
            79010100 ff010010 81d41f1a
            45010100 ff010010 81d5201a
            25050100 ff010010 7f2ce1e6
            e3000100 ff010010 7f2be0e6
        );
        assert_eq!(
            V12Values::Quat(vec![
                quat(-0.013335498, -0.008552759, -0.17875099, 0.98376685),
                quat(-0.013141736, -0.008489483, -0.17895888, 0.9837322),
                quat(-0.013003081, -0.008366773, -0.17932187, 0.983669),
                quat(-0.012843506, -0.00821765, -0.17969312, 0.9836046),
                quat(-0.012671822, -0.008052318, -0.18016055, 0.98352265),
                quat(-0.012497274, -0.007881654, -0.1806723, 0.9834324),
                quat(-0.01232608, -0.007713619, -0.18124439, 0.9833306),
                quat(-0.012160288, -0.0075517744, -0.18187602, 0.98321736),
                quat(-0.011998486, -0.0073957993, -0.18254046, 0.9830973),
                quat(-0.011837766, -0.007243483, -0.18326552, 0.9829655),
                quat(-0.011676286, -0.0070933, -0.18403819, 0.98282415),
                quat(-0.011514541, -0.0069458894, -0.18480875, 0.9826826),
                quat(-0.011354783, -0.006803781, -0.18559827, 0.9825367),
                quat(-0.011199581, -0.0066699954, -0.18642545, 0.9823827),
                quat(-0.011050063, -0.0065462184, -0.18726455, 0.9822256),
                quat(-0.010904852, -0.0064315945, -0.18811889, 0.9820647),
                quat(-0.010760771, -0.0063231112, -0.18898194, 0.9819013),
                quat(-0.01061474, -0.006217249, -0.18982157, 0.9817416),
                quat(-0.010465713, -0.006111901, -0.19064824, 0.98158365),
                quat(-0.010315883, -0.0060076737, -0.19147277, 0.98142534),
                quat(-0.010170145, -0.0059076785, -0.19228232, 0.98126924),
                quat(-0.01003402, -0.0058158548, -0.19308582, 0.98111343),
                quat(-0.009911364, -0.0057349266, -0.1938592, 0.9809625),
                quat(-0.009802959, -0.0056650243, -0.1945574, 0.9808258),
                quat(-0.009706752, -0.005603813, -0.19521394, 0.9806966),
                quat(-0.009620242, -0.0055484404, -0.1958594, 0.9805691),
                quat(-0.009543462, -0.005498139, -0.19645691, 0.9804507),
                quat(-0.0094806785, -0.0054559065, -0.19699948, 0.98034257),
                quat(-0.009440104, -0.0054284143, -0.19747749, 0.980247),
                quat(-0.0094311, -0.005423721, -0.197851, 0.9801718),
                quat(-0.009459667, -0.0054474683, -0.19816092, 0.9801087),
                quat(-0.009524761, -0.005499602, -0.19841935, 0.9800555),
                quat(-0.0096167065, -0.0055728806, -0.19858481, 0.98002064),
                quat(-0.009611724, -0.005568522, -0.19867107, 0.98000336),
                quat(-0.009697989, -0.005636047, -0.19865991, 0.9800043),
                quat(-0.009816655, -0.0057329736, -0.19853991, 0.9800269),
                quat(-0.00993086, -0.0058259754, -0.19831122, 0.9800715),
                quat(-0.010044327, -0.005918336, -0.19804272, 0.98012406),
                quat(-0.01016217, -0.006014469, -0.19766924, 0.9801977),
                quat(-0.010289464, -0.0061188024, -0.197233, 0.9802837),
                quat(-0.010430468, -0.0062350156, -0.19671172, 0.98038614),
                quat(-0.010587475, -0.0063650864, -0.19609773, 0.9805066),
                quat(-0.010760107, -0.006508735, -0.19547564, 0.98062795),
                quat(-0.010945995, -0.006663885, -0.19479267, 0.9807608),
                quat(-0.011141145, -0.0068271253, -0.1940979, 0.9808952),
                quat(-0.011341673, -0.006995077, -0.19333383, 0.98104256),
                quat(-0.011544539, -0.0071651256, -0.19251099, 0.98120075),
                quat(-0.011748168, -0.007335911, -0.19163743, 0.98136806),
                quat(-0.01195187, -0.0075068623, -0.19079632, 0.9815283),
                quat(-0.012155153, -0.007677518, -0.18998548, 0.9816817),
                quat(-0.012356552, -0.007846527, -0.18912801, 0.9818434),
                quat(-0.0125526255, -0.008010892, -0.1882522, 0.9820079),
                quat(-0.012738715, -0.008166551, -0.18735598, 0.9821756),
                quat(-0.012910123, -0.008309442, -0.18648784, 0.9823373),
                quat(-0.01306412, -0.00843716, -0.18562406, 0.9824977),
                quat(-0.013200727, -0.008549718, -0.18482284, 0.98264605),
                quat(-0.013323061, -0.008649738, -0.18403938, 0.9827905),
                quat(-0.013435416, -0.0087408805, -0.18327324, 0.9829313),
                quat(-0.013540838, -0.0088257585, -0.18254949, 0.9830638),
                quat(-0.013639043, -0.008904099, -0.18184108, 0.9831931),
                quat(-0.01372515, -0.008971735, -0.181187, 0.983312),
                quat(-0.013791077, -0.009021728, -0.18057641, 0.98342294),
                quat(-0.013828024, -0.009046614, -0.18011758, 0.98350626),
                quat(-0.01383129, -0.0090423105, -0.17964143, 0.98359346),
                quat(-0.013801866, -0.009009799, -0.17928039, 0.98366),
                quat(-0.013748859, -0.0089569315, -0.17895198, 0.983721),
                quat(-0.013780596, -0.008978037, -0.17871395, 0.98376364),
                quat(-0.013708919, -0.00890524, -0.17857687, 0.9837902),
                quat(-0.0136099225, -0.008809579, -0.17853504, 0.9838),
                quat(-0.0134818, -0.00868816, -0.178595, 0.983792),
            ]),
            read_property_value_v12(&data).unwrap()
        );
    }
}
