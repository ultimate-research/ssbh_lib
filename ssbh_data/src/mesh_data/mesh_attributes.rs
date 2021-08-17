use super::{get_size_in_bytes_v8, AttributeData, MeshObjectData, MeshVersion, VectorData};
use crate::{mesh_data::get_size_in_bytes_v10, write_f16, write_f32, write_u8};
use ssbh_lib::{
    formats::mesh::{
        AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
        MeshAttributeV10, MeshAttributeV8, MeshAttributes,
    },
    SsbhArray,
};
use std::io::{Seek, Write};

// TODO: Use a trait or find a way to share code with version 1.10.
fn add_attribute_v8(
    attributes: &mut Vec<MeshAttributeV8>,
    current_stride: &mut u32,
    buffer_index: u32,
    sub_index: u32,
    usage: AttributeUsageV8,
    data_type: AttributeDataTypeV8,
) {
    let attribute = MeshAttributeV8 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: *current_stride,
        sub_index,
    };

    *current_stride += get_size_in_bytes_v8(&attribute.data_type) as u32;
    attributes.push(attribute);
}

fn add_attribute_v10(
    attributes: &mut Vec<MeshAttributeV10>,
    current_stride: &mut u32,
    name: &str,
    attribute_array_name: &str,
    buffer_index: u32,
    sub_index: u64,
    usage: AttributeUsageV9,
    data_type: AttributeDataTypeV10,
) {
    let attribute = MeshAttributeV10 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: *current_stride,
        sub_index,
        name: name.into(),
        attribute_names: SsbhArray::new(vec![attribute_array_name.into()]),
    };

    *current_stride += get_size_in_bytes_v10(&attribute.data_type) as u32;
    attributes.push(attribute);
}

fn add_attributes_v8(
    attributes: &mut Vec<MeshAttributeV8>,
    attributes_to_add: &[AttributeData],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV8,
) {
    for (i, attribute) in attributes_to_add.iter().enumerate() {
        let data_type = infer_data_type_v8(attribute, usage);

        add_attribute_v8(
            attributes,
            current_stride,
            buffer_index,
            i as u32,
            usage,
            data_type,
        );
    }
}

fn infer_data_type_v8(attribute: &AttributeData, usage: AttributeUsageV8) -> AttributeDataTypeV8 {
    // TODO: Prefer single precision or allow for custom data types?
    match (usage, &attribute.data) {
        (AttributeUsageV8::ColorSet, VectorData::Vector4(_)) => AttributeDataTypeV8::Byte4,
        (_, VectorData::Vector2(_)) => AttributeDataTypeV8::Float2,
        (_, VectorData::Vector3(_)) => AttributeDataTypeV8::Float3,
        (_, VectorData::Vector4(_)) => AttributeDataTypeV8::HalfFloat4,
    }
}

pub fn create_attributes_v8(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
    let mut attributes = Vec::new();

    let mut stride0 = 0u32;
    add_attributes_v8(
        &mut attributes,
        &data.positions,
        &mut stride0,
        0,
        AttributeUsageV8::Position,
    );
    add_attributes_v8(
        &mut attributes,
        &data.normals,
        &mut stride0,
        0,
        AttributeUsageV8::Normal,
    );

    // TODO: It's unclear what the usage enum for binormal is for version 1.8, so skip it for now.

    add_attributes_v8(
        &mut attributes,
        &data.tangents,
        &mut stride0,
        0,
        AttributeUsageV8::Tangent,
    );

    let mut stride1 = 0;
    add_attributes_v8(
        &mut attributes,
        &data.texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV8::TextureCoordinate,
    );
    add_attributes_v8(
        &mut attributes,
        &data.color_sets,
        &mut stride1,
        1,
        AttributeUsageV8::ColorSet,
    );

    (
        stride0,
        stride1,
        MeshAttributes::AttributesV8(SsbhArray::new(attributes)),
    )
}

fn add_attributes_v10(
    attributes: &mut Vec<MeshAttributeV10>,
    attributes_to_add: &[AttributeData],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV9,
) {
    for (i, attribute) in attributes_to_add.iter().enumerate() {
        let data_type = infer_data_type_v10(attribute, usage);

        // This is a convention in games such as Smash Ultimate and New Pokemon Snap.
        let name = match (usage, i) {
            (AttributeUsageV9::Tangent, 0) => "map1",
            (AttributeUsageV9::Binormal, 0) => "map1",
            (AttributeUsageV9::Binormal, 1) => "uvSet",
            _ => &attribute.name,
        };

        add_attribute_v10(
            attributes,
            current_stride,
            name,
            &attribute.name,
            buffer_index,
            i as u64,
            usage,
            data_type,
        );
    }
}

fn infer_data_type_v10(attribute: &AttributeData, usage: AttributeUsageV9) -> AttributeDataTypeV10 {
    // TODO: Prefer single precision or allow for custom data types?
    match (usage, &attribute.data) {
        // Some data is less sensitive to the lower precision of f16 or u8.
        (AttributeUsageV9::Normal, VectorData::Vector2(_)) => AttributeDataTypeV10::HalfFloat2,
        (AttributeUsageV9::Normal, VectorData::Vector4(_)) => AttributeDataTypeV10::HalfFloat4,
        (AttributeUsageV9::Tangent, VectorData::Vector2(_)) => AttributeDataTypeV10::HalfFloat2,
        (AttributeUsageV9::Tangent, VectorData::Vector4(_)) => AttributeDataTypeV10::HalfFloat4,
        (AttributeUsageV9::TextureCoordinate, VectorData::Vector2(_)) => {
            AttributeDataTypeV10::HalfFloat2
        }
        (AttributeUsageV9::TextureCoordinate, VectorData::Vector4(_)) => {
            AttributeDataTypeV10::HalfFloat4
        }
        (AttributeUsageV9::ColorSet, VectorData::Vector2(_)) => AttributeDataTypeV10::HalfFloat2,
        (AttributeUsageV9::ColorSet, VectorData::Vector4(_)) => AttributeDataTypeV10::Byte4,
        // Default to using the largest available precision.
        (_, VectorData::Vector2(_)) => AttributeDataTypeV10::Float2,
        (_, VectorData::Vector3(_)) => AttributeDataTypeV10::Float3,
        (_, VectorData::Vector4(_)) => AttributeDataTypeV10::Float4,
    }
}

pub fn create_attributes_v10(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
    let mut attributes = Vec::new();

    let mut stride0 = 0u32;
    add_attributes_v10(
        &mut attributes,
        &data.positions,
        &mut stride0,
        0,
        AttributeUsageV9::Position,
    );
    add_attributes_v10(
        &mut attributes,
        &data.normals,
        &mut stride0,
        0,
        AttributeUsageV9::Normal,
    );
    add_attributes_v10(
        &mut attributes,
        &data.binormals,
        &mut stride0,
        0,
        AttributeUsageV9::Binormal,
    );
    add_attributes_v10(
        &mut attributes,
        &data.tangents,
        &mut stride0,
        0,
        AttributeUsageV9::Tangent,
    );

    let mut stride1 = 0;
    add_attributes_v10(
        &mut attributes,
        &data.texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV9::TextureCoordinate,
    );
    add_attributes_v10(
        &mut attributes,
        &data.color_sets,
        &mut stride1,
        1,
        AttributeUsageV9::ColorSet,
    );

    (
        stride0,
        stride1,
        MeshAttributes::AttributesV10(SsbhArray::new(attributes)),
    )
}

pub fn write_attributes<W: Write + Seek>(
    data: &MeshObjectData,
    buffer0: &mut W,
    buffer1: &mut W,
    attributes: &MeshAttributes,
    stride0: u64,
    stride1: u64,
    offset0: u64,
    offset1: u64,
) -> Result<(), std::io::Error> {
    // TODO: Is there a nicer way to write this without so many matches?
    match attributes {
        MeshAttributes::AttributesV8(attributes) => {
            // TODO: It seems redundant to index by sub_index since we created the attributes from the MeshObjectData already.
            for a in &attributes.elements {
                // TODO: These accesses may panic.
                let index = a.sub_index as usize;
                let data = match a.usage {
                    AttributeUsageV8::Position => &data.positions[index].data,
                    AttributeUsageV8::Normal => &data.normals[index].data,
                    AttributeUsageV8::Tangent => &data.tangents[index].data,
                    AttributeUsageV8::TextureCoordinate => &data.texture_coordinates[index].data,
                    AttributeUsageV8::ColorSet => &data.color_sets[index].data,
                };

                // TODO: Don't assume two buffers?
                if a.buffer_index == 0 {
                    write_attributes_v8(
                        buffer0,
                        data,
                        &a.data_type,
                        offset0 + a.buffer_offset as u64,
                        stride0,
                    )?;
                } else {
                    write_attributes_v8(
                        buffer1,
                        data,
                        &a.data_type,
                        offset1 + a.buffer_offset as u64,
                        stride1,
                    )?;
                }
            }
        }
        MeshAttributes::AttributesV10(attributes) => {
            // TODO: It seems redundant to index by sub_index since we created the attributes from the MeshObjectData already.
            for a in &attributes.elements {
                // TODO: These accesses may panic.
                let index = a.sub_index as usize;
                let data = match a.usage {
                    AttributeUsageV9::Position => &data.positions[index].data,
                    AttributeUsageV9::Normal => &data.normals[index].data,
                    AttributeUsageV9::Binormal => &data.binormals[index].data,
                    AttributeUsageV9::Tangent => &data.tangents[index].data,
                    AttributeUsageV9::TextureCoordinate => &data.texture_coordinates[index].data,
                    AttributeUsageV9::ColorSet => &data.color_sets[index].data,
                };

                // TODO: Don't assume two buffers?
                if a.buffer_index == 0 {
                    write_attributes_v10(
                        buffer0,
                        data,
                        &a.data_type,
                        offset0 + a.buffer_offset as u64,
                        stride0,
                    )?;
                } else {
                    write_attributes_v10(
                        buffer1,
                        data,
                        &a.data_type,
                        offset1 + a.buffer_offset as u64,
                        stride1,
                    )?;
                }
            }
        }
        // TODO: Support writing mesh version 1.9
        MeshAttributes::AttributesV9(_) => todo!(),
    }
    Ok(())
}

// TODO: Check for this case where the component count on data_type and data don't match.
fn write_attributes_v8<W: Write + Seek>(
    writer: &mut W,
    data: &VectorData,
    data_type: &AttributeDataTypeV8,
    offset: u64,
    stride: u64,
) -> Result<(), std::io::Error> {
    match data_type {
        AttributeDataTypeV8::Float3 => data.write(writer, offset, stride, write_f32),
        AttributeDataTypeV8::HalfFloat4 => data.write(writer, offset, stride, write_f16),
        AttributeDataTypeV8::Float2 => data.write(writer, offset, stride, write_f32),
        AttributeDataTypeV8::Byte4 => data.write(writer, offset, stride, write_u8),
        AttributeDataTypeV8::Float4 => data.write(writer, offset, stride, write_f32),
    }
}

fn write_attributes_v10<W: Write + Seek>(
    writer: &mut W,
    data: &VectorData,
    data_type: &AttributeDataTypeV10,
    offset: u64,
    stride: u64,
) -> Result<(), std::io::Error> {
    match data_type {
        AttributeDataTypeV10::Float3 => data.write(writer, offset, stride, write_f32),
        AttributeDataTypeV10::HalfFloat4 => data.write(writer, offset, stride, write_f16),
        AttributeDataTypeV10::Float2 => data.write(writer, offset, stride, write_f32),
        AttributeDataTypeV10::Byte4 => data.write(writer, offset, stride, write_u8),
        AttributeDataTypeV10::Float4 => data.write(writer, offset, stride, write_f32),
        AttributeDataTypeV10::HalfFloat2 => data.write(writer, offset, stride, write_f16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_data_type_v10(data: VectorData, usage: AttributeUsageV9) -> AttributeDataTypeV10 {
        let a = AttributeData {
            name: "".to_string(),
            data,
        };
        infer_data_type_v10(&a, usage)
    }

    #[test]
    fn infer_position_type_v10() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            AttributeDataTypeV10::Float2,
            get_data_type_v10(VectorData::Vector2(Vec::new()), AttributeUsageV9::Position)
        );
        assert_eq!(
            AttributeDataTypeV10::Float3,
            get_data_type_v10(VectorData::Vector3(Vec::new()), AttributeUsageV9::Position)
        );
        assert_eq!(
            AttributeDataTypeV10::Float4,
            get_data_type_v10(VectorData::Vector4(Vec::new()), AttributeUsageV9::Position)
        );
    }

    #[test]
    fn infer_normal_type_v10() {
        // Check that normals use the smallest available floating point type.
        assert_eq!(
            AttributeDataTypeV10::HalfFloat2,
            get_data_type_v10(VectorData::Vector2(Vec::new()), AttributeUsageV9::Normal)
        );
        assert_eq!(
            AttributeDataTypeV10::Float3,
            get_data_type_v10(VectorData::Vector3(Vec::new()), AttributeUsageV9::Normal)
        );
        assert_eq!(
            AttributeDataTypeV10::HalfFloat4,
            get_data_type_v10(VectorData::Vector4(Vec::new()), AttributeUsageV9::Normal)
        );
    }

    #[test]
    fn infer_texcoord_type_v10() {
        // Check that texture coordinates use the smallest available floating point type.
        assert_eq!(
            AttributeDataTypeV10::HalfFloat2,
            get_data_type_v10(
                VectorData::Vector2(Vec::new()),
                AttributeUsageV9::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataTypeV10::Float3,
            get_data_type_v10(
                VectorData::Vector3(Vec::new()),
                AttributeUsageV9::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataTypeV10::HalfFloat4,
            get_data_type_v10(
                VectorData::Vector4(Vec::new()),
                AttributeUsageV9::TextureCoordinate
            )
        );
    }

    #[test]
    fn infer_colorset_type_v10() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            AttributeDataTypeV10::HalfFloat2,
            get_data_type_v10(VectorData::Vector2(Vec::new()), AttributeUsageV9::ColorSet)
        );
        assert_eq!(
            AttributeDataTypeV10::Float3,
            get_data_type_v10(VectorData::Vector3(Vec::new()), AttributeUsageV9::ColorSet)
        );
        assert_eq!(
            AttributeDataTypeV10::Byte4,
            get_data_type_v10(VectorData::Vector4(Vec::new()), AttributeUsageV9::ColorSet)
        );
    }

    fn get_data_type_v8(data: VectorData, usage: AttributeUsageV8) -> AttributeDataTypeV8 {
        let a = AttributeData {
            name: "".to_string(),
            data,
        };
        infer_data_type_v8(&a, usage)
    }

    #[test]
    fn infer_position_type_v8() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(VectorData::Vector2(Vec::new()), AttributeUsageV8::Position)
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(VectorData::Vector3(Vec::new()), AttributeUsageV8::Position)
        );
        assert_eq!(
            AttributeDataTypeV8::HalfFloat4,
            get_data_type_v8(VectorData::Vector4(Vec::new()), AttributeUsageV8::Position)
        );
    }

    #[test]
    fn infer_normal_type_v8() {
        // Check that normals use the smallest available floating point type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(VectorData::Vector2(Vec::new()), AttributeUsageV8::Normal)
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(VectorData::Vector3(Vec::new()), AttributeUsageV8::Normal)
        );
        assert_eq!(
            AttributeDataTypeV8::HalfFloat4,
            get_data_type_v8(VectorData::Vector4(Vec::new()), AttributeUsageV8::Normal)
        );
    }

    #[test]
    fn infer_texcoord_type_v8() {
        // Check that texture coordinates use the smallest available floating point type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(
                VectorData::Vector2(Vec::new()),
                AttributeUsageV8::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(
                VectorData::Vector3(Vec::new()),
                AttributeUsageV8::TextureCoordinate
            )
        );
        assert_eq!(
            AttributeDataTypeV8::HalfFloat4,
            get_data_type_v8(
                VectorData::Vector4(Vec::new()),
                AttributeUsageV8::TextureCoordinate
            )
        );
    }

    #[test]
    fn infer_colorset_type_v8() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            AttributeDataTypeV8::Float2,
            get_data_type_v8(VectorData::Vector2(Vec::new()), AttributeUsageV8::ColorSet)
        );
        assert_eq!(
            AttributeDataTypeV8::Float3,
            get_data_type_v8(VectorData::Vector3(Vec::new()), AttributeUsageV8::ColorSet)
        );
        assert_eq!(
            AttributeDataTypeV8::Byte4,
            get_data_type_v8(VectorData::Vector4(Vec::new()), AttributeUsageV8::ColorSet)
        );
    }

    #[test]
    fn create_attributes_mesh_v1_8() {
        let data = MeshObjectData {
            name: "name".into(),
            sub_index: 0,
            parent_bone_name: "".into(),
            vertex_indices: Vec::new(),
            positions: vec![AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            normals: vec![AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            binormals: Vec::new(),
            tangents: vec![AttributeData {
                name: "t0".into(),
                data: VectorData::Vector4(Vec::new()),
            }],
            texture_coordinates: vec![
                AttributeData {
                    name: "firstUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
                AttributeData {
                    name: "secondUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
            ],
            color_sets: vec![
                AttributeData {
                    name: "color1".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
                AttributeData {
                    name: "color2".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
            ],
            bone_influences: Vec::new(),
        };

        let (stride0, stride1, attributes) = create_attributes_v8(&data);
        assert_eq!(32, stride0);
        assert_eq!(24, stride1);

        match attributes {
            MeshAttributes::AttributesV8(a) => {
                let mut attributes = a.elements.iter();

                // Check buffer 0.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Position, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float3, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Normal, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(12, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float3, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::Tangent, a.usage);
                assert_eq!(0, a.buffer_index);
                assert_eq!(24, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::HalfFloat4, a.data_type);

                // Check buffer 1.
                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(0, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float2, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::TextureCoordinate, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(8, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Float2, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(16, a.buffer_offset);
                assert_eq!(0, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Byte4, a.data_type);

                let a = attributes.next().unwrap();
                assert_eq!(AttributeUsageV8::ColorSet, a.usage);
                assert_eq!(1, a.buffer_index);
                assert_eq!(20, a.buffer_offset);
                assert_eq!(1, a.sub_index);
                assert_eq!(AttributeDataTypeV8::Byte4, a.data_type);
            }
            _ => panic!("invalid version"),
        };
    }

    // TODO: create_attributes_mesh_v1_9

    #[test]
    fn create_attributes_mesh_v1_10() {
        let data = MeshObjectData {
            name: "name".into(),
            sub_index: 0,
            parent_bone_name: "".into(),
            vertex_indices: Vec::new(),
            positions: vec![AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            normals: vec![AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            binormals: vec![
                AttributeData {
                    name: "b1".into(),
                    data: VectorData::Vector3(Vec::new()),
                },
                AttributeData {
                    name: "b2".into(),
                    data: VectorData::Vector3(Vec::new()),
                },
            ],
            tangents: vec![AttributeData {
                name: "t0".into(),
                data: VectorData::Vector4(Vec::new()),
            }],
            texture_coordinates: vec![
                AttributeData {
                    name: "firstUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
                AttributeData {
                    name: "secondUv".into(),
                    data: VectorData::Vector2(Vec::new()),
                },
            ],
            color_sets: vec![
                AttributeData {
                    name: "color1".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
                AttributeData {
                    name: "color2".into(),
                    data: VectorData::Vector4(Vec::new()),
                },
            ],
            bone_influences: Vec::new(),
        };

        let (stride0, stride1, attributes) = create_attributes_v10(&data);
        assert_eq!(56, stride0);
        assert_eq!(16, stride1);

        // TODO: Just use partial equal?
        match attributes {
            MeshAttributes::AttributesV10(a) => {
                let mut attributes = a.elements.iter();
                // Check buffer 0.
                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::Position,
                        data_type: AttributeDataTypeV10::Float3,
                        buffer_index: 0,
                        buffer_offset: 0,
                        sub_index: 0,
                        name: "p0".into(),
                        attribute_names: SsbhArray::new(vec!["p0".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::Normal,
                        data_type: AttributeDataTypeV10::Float3,
                        buffer_index: 0,
                        buffer_offset: 12,
                        sub_index: 0,
                        name: "n0".into(),
                        attribute_names: SsbhArray::new(vec!["n0".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::Binormal,
                        data_type: AttributeDataTypeV10::Float3,
                        buffer_index: 0,
                        buffer_offset: 24,
                        sub_index: 0,
                        // Using "map1" is a convention likely due to generating binormals from this attribute.
                        name: "map1".into(),
                        attribute_names: SsbhArray::new(vec!["b1".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::Binormal,
                        data_type: AttributeDataTypeV10::Float3,
                        buffer_index: 0,
                        buffer_offset: 36,
                        sub_index: 1,
                        // Using "uvSet" is a convention likely due to generating binormals from this attribute.
                        name: "uvSet".into(),
                        attribute_names: SsbhArray::new(vec!["b2".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::Tangent,
                        data_type: AttributeDataTypeV10::HalfFloat4,
                        buffer_index: 0,
                        buffer_offset: 48,
                        sub_index: 0,
                        // Using "map1" is a convention likely due to generating tangents from this attribute.
                        name: "map1".into(),
                        attribute_names: SsbhArray::new(vec!["t0".into()]),
                    },
                    attributes.next().unwrap()
                );

                // Check buffer 1.
                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::TextureCoordinate,
                        data_type: AttributeDataTypeV10::HalfFloat2,
                        buffer_index: 1,
                        buffer_offset: 0,
                        sub_index: 0,
                        name: "firstUv".into(),
                        attribute_names: SsbhArray::new(vec!["firstUv".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::TextureCoordinate,
                        data_type: AttributeDataTypeV10::HalfFloat2,
                        buffer_index: 1,
                        buffer_offset: 4,
                        sub_index: 1,
                        name: "secondUv".into(),
                        attribute_names: SsbhArray::new(vec!["secondUv".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::ColorSet,
                        data_type: AttributeDataTypeV10::Byte4,
                        buffer_index: 1,
                        buffer_offset: 8,
                        sub_index: 0,
                        name: "color1".into(),
                        attribute_names: SsbhArray::new(vec!["color1".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV10 {
                        usage: AttributeUsageV9::ColorSet,
                        data_type: AttributeDataTypeV10::Byte4,
                        buffer_index: 1,
                        buffer_offset: 12,
                        sub_index: 1,
                        name: "color2".into(),
                        attribute_names: SsbhArray::new(vec!["color2".into()]),
                    },
                    attributes.next().unwrap()
                );
            }
            _ => panic!("invalid version"),
        };
    }
}
