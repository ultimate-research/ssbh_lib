use super::{
    get_attribute_name_v9, get_size_in_bytes_v8, AttributeData, MeshObjectData, MeshVersion,
    VectorData,
};
use crate::{
    get_u8_clamped, mesh_data::get_size_in_bytes_v10, write_f16, write_f32, write_u8,
    write_vector_data,
};
use half::f16;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
        MeshAttributeV10, MeshAttributeV8, MeshAttributes,
    },
    Half, SsbhArray,
};
use ssbh_write::SsbhWrite;
use std::io::{Seek, Write};

// data.positions -> vec of AttributeData::Float4 -> calculate stride, data type, etc
// TODO: Find a
#[derive(Debug, PartialEq)]
pub enum AttributeBufferData {
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
    Float4(Vec<[f32; 4]>),
    HalfFloat2(Vec<[f16; 2]>),
    HalfFloat4(Vec<[f16; 4]>),
    Byte4(Vec<[u8; 4]>),
}

impl AttributeBufferData {
    // TODO: Create two enums to avoid panic?
    fn data_type_v8(&self) -> AttributeDataTypeV8 {
        match self {
            AttributeBufferData::Float2(_) => AttributeDataTypeV8::Float2,
            AttributeBufferData::Float3(_) => AttributeDataTypeV8::Float3,
            AttributeBufferData::Float4(_) => AttributeDataTypeV8::Float4,
            AttributeBufferData::HalfFloat4(_) => AttributeDataTypeV8::HalfFloat4,
            AttributeBufferData::Byte4(_) => AttributeDataTypeV8::Byte4,
            AttributeBufferData::HalfFloat2(_) => panic!("Unsupported data type"),
        }
    }

    fn data_type_v10(&self) -> AttributeDataTypeV10 {
        match self {
            AttributeBufferData::Float2(_) => AttributeDataTypeV10::Float2,
            AttributeBufferData::Float3(_) => AttributeDataTypeV10::Float3,
            AttributeBufferData::Float4(_) => AttributeDataTypeV10::Float4,
            AttributeBufferData::HalfFloat4(_) => AttributeDataTypeV10::HalfFloat4,
            AttributeBufferData::Byte4(_) => AttributeDataTypeV10::Byte4,
            AttributeBufferData::HalfFloat2(_) => AttributeDataTypeV10::HalfFloat2,
        }
    }
}

// TODO: Simplify with const generics?
fn get_f16_vector4(v: &[f32; 4]) -> [f16; 4] {
    let [x, y, z, w] = v;
    [
        f16::from_f32(*x),
        f16::from_f32(*y),
        f16::from_f32(*z),
        f16::from_f32(*w),
    ]
}

fn get_f16_vector2(v: &[f32; 2]) -> [f16; 2] {
    let [x, y] = v;
    [f16::from_f32(*x), f16::from_f32(*y)]
}

fn get_clamped_u8_vector4(v: &[f32; 4]) -> [u8; 4] {
    let [x, y, z, w] = v;
    [
        get_u8_clamped(*x),
        get_u8_clamped(*y),
        get_u8_clamped(*z),
        get_u8_clamped(*w),
    ]
}

fn get_f16_vector2s(v: &[[f32; 2]]) -> Vec<[f16; 2]> {
    v.iter().map(get_f16_vector2).collect()
}

fn get_f16_vector4s(v: &[[f32; 4]]) -> Vec<[f16; 4]> {
    v.iter().map(get_f16_vector4).collect()
}

fn get_clamped_u8_vector4s(v: &[[f32; 4]]) -> Vec<[u8; 4]> {
    v.iter().map(get_clamped_u8_vector4).collect()
}

fn get_position_data_v8(data: &[AttributeData]) -> Vec<AttributeBufferData> {
    data.iter()
        .map(|a| match &a.data {
            VectorData::Vector2(v) => AttributeBufferData::Float2(v.clone()),
            VectorData::Vector3(v) => AttributeBufferData::Float3(v.clone()),
            VectorData::Vector4(v) => AttributeBufferData::Float4(v.clone()),
        })
        .collect()
}

fn get_position_data_v10(data: &[AttributeData]) -> Vec<(String, AttributeBufferData)> {
    data.iter()
        .map(|a| match &a.data {
            VectorData::Vector2(v) => (a.name.clone(), AttributeBufferData::Float2(v.clone())),
            VectorData::Vector3(v) => (a.name.clone(), AttributeBufferData::Float3(v.clone())),
            VectorData::Vector4(v) => (a.name.clone(), AttributeBufferData::Float4(v.clone())),
        })
        .collect()
}

fn get_vector_data_v8(data: &[AttributeData]) -> Vec<AttributeBufferData> {
    data.iter()
        .map(|a| match &a.data {
            VectorData::Vector2(v) => AttributeBufferData::Float2(v.clone()),
            VectorData::Vector3(v) => AttributeBufferData::Float3(v.clone()),
            VectorData::Vector4(v) => AttributeBufferData::HalfFloat4(get_f16_vector4s(v)),
        })
        .collect()
}

fn get_vector_data_v10(data: &[AttributeData]) -> Vec<(String, AttributeBufferData)> {
    data.iter()
        .map(|a| match &a.data {
            VectorData::Vector2(v) => (
                a.name.clone(),
                AttributeBufferData::HalfFloat2(get_f16_vector2s(v)),
            ),
            VectorData::Vector3(v) => (a.name.clone(), AttributeBufferData::Float3(v.clone())),
            VectorData::Vector4(v) => (
                a.name.clone(),
                AttributeBufferData::HalfFloat4(get_f16_vector4s(v)),
            ),
        })
        .collect()
}

fn get_color_data_v8(data: &[AttributeData]) -> Vec<AttributeBufferData> {
    data.iter()
        .map(|a| match &a.data {
            VectorData::Vector2(v) => AttributeBufferData::Float2(v.clone()),
            VectorData::Vector3(v) => AttributeBufferData::Float3(v.clone()),
            VectorData::Vector4(v) => AttributeBufferData::Byte4(get_clamped_u8_vector4s(v)),
        })
        .collect()
}

fn get_color_data_v10(data: &[AttributeData]) -> Vec<(String, AttributeBufferData)> {
    data.iter()
        .map(|a| match &a.data {
            VectorData::Vector2(v) => (
                a.name.clone(),
                AttributeBufferData::HalfFloat2(get_f16_vector2s(v)),
            ),
            VectorData::Vector3(v) => (a.name.clone(), AttributeBufferData::Float3(v.clone())),
            VectorData::Vector4(v) => (
                a.name.clone(),
                AttributeBufferData::Byte4(get_clamped_u8_vector4s(v)),
            ),
        })
        .collect()
}

// TODO: More efficient to just take ownership of the vector data?
// TODO: create a struct for returning (strides, data, attributes)
pub fn create_attributes_v8(
    data: &MeshObjectData,
) -> ([(u32, Vec<AttributeBufferData>); 4], MeshAttributes) {
    // 1. Convert the data into the appropriate format based on usage and component count.
    // TODO: Avoid collecting until the end?
    let positions = get_position_data_v8(&data.positions);
    let normals = get_vector_data_v8(&data.normals);
    let tangents = get_vector_data_v8(&data.tangents);
    let texture_coordinates = get_vector_data_v8(&data.texture_coordinates);
    let color_sets = get_color_data_v8(&data.color_sets);

    // 2. Compute the strides + offsets + attributes
    let mut stride0 = 0;
    let mut stride1 = 0;
    let mut mesh_attributes = Vec::new();

    add_attributes_v8(
        &mut mesh_attributes,
        &positions,
        &mut stride0,
        0,
        AttributeUsageV8::Position,
    );
    add_attributes_v8(
        &mut mesh_attributes,
        &normals,
        &mut stride0,
        0,
        AttributeUsageV8::Normal,
    );
    add_attributes_v8(
        &mut mesh_attributes,
        &tangents,
        &mut stride0,
        0,
        AttributeUsageV8::Tangent,
    );

    add_attributes_v8(
        &mut mesh_attributes,
        &texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV8::TextureCoordinate,
    );
    add_attributes_v8(
        &mut mesh_attributes,
        &color_sets,
        &mut stride1,
        1,
        AttributeUsageV8::ColorSet,
    );

    // 3. Chain the attributes and positions together.
    // TODO: Just return vector instead of SsbhArray?
    (
        [
            (
                stride0,
                positions
                    .into_iter()
                    .chain(normals.into_iter())
                    .chain(tangents.into_iter())
                    .collect(),
            ),
            (
                stride1,
                texture_coordinates
                    .into_iter()
                    .chain(color_sets.into_iter())
                    .collect(),
            ),
            // These last two vertex buffers never seem to contain any attributes.
            (0, Vec::new()),
            (0, Vec::new()),
        ],
        MeshAttributes::AttributesV8(mesh_attributes.into()),
    )
}

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
    attributes_to_add: &[AttributeBufferData],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV8,
) {
    for (i, attribute) in attributes_to_add.iter().enumerate() {
        let data_type = attribute.data_type_v8();

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

fn add_attributes_v10(
    attributes: &mut Vec<MeshAttributeV10>,
    attributes_to_add: &[(String, AttributeBufferData)],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV9,
) {
    for (i, (attribute_name, attribute)) in attributes_to_add.iter().enumerate() {
        let data_type = attribute.data_type_v10();

        // This is likely due to which UVs were used to generate the tangents/binormals.x
        let name = match (usage, i) {
            (AttributeUsageV9::Tangent, 0) => "map1",
            (AttributeUsageV9::Binormal, 0) => "map1",
            (AttributeUsageV9::Binormal, 1) => "uvSet",
            _ => &attribute_name,
        };

        add_attribute_v10(
            attributes,
            current_stride,
            name,
            &attribute_name,
            buffer_index,
            i as u64,
            usage,
            data_type,
        );
    }
}

pub fn create_attributes_v10(
    data: &MeshObjectData,
) -> ([(u32, Vec<AttributeBufferData>); 4], MeshAttributes) {
    // 1. Convert the data into the appropriate format based on usage and component count.
    // TODO: Avoid collecting until the end?
    // TODO: This really should use two AttributeBufferData enums to avoid incompatible types.
    let positions = get_position_data_v10(&data.positions);
    let normals = get_vector_data_v10(&data.normals);
    let binormals = get_vector_data_v10(&data.binormals);
    let tangents = get_vector_data_v10(&data.tangents);
    let texture_coordinates = get_vector_data_v10(&data.texture_coordinates);
    let color_sets = get_color_data_v10(&data.color_sets);

    // 2. Compute the strides + offsets + attributes
    let mut stride0 = 0;
    let mut stride1 = 0;
    let mut mesh_attributes = Vec::new();

    add_attributes_v10(
        &mut mesh_attributes,
        &positions,
        &mut stride0,
        0,
        AttributeUsageV9::Position,
    );
    add_attributes_v10(
        &mut mesh_attributes,
        &normals,
        &mut stride0,
        0,
        AttributeUsageV9::Normal,
    );
    add_attributes_v10(
        &mut mesh_attributes,
        &binormals,
        &mut stride0,
        0,
        AttributeUsageV9::Binormal,
    );
    add_attributes_v10(
        &mut mesh_attributes,
        &tangents,
        &mut stride0,
        0,
        AttributeUsageV9::Tangent,
    );

    add_attributes_v10(
        &mut mesh_attributes,
        &texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV9::TextureCoordinate,
    );
    add_attributes_v10(
        &mut mesh_attributes,
        &color_sets,
        &mut stride1,
        1,
        AttributeUsageV9::ColorSet,
    );

    // 3. Chain the attributes and positions together.
    // TODO: Just return vector instead of SsbhArray?
    (
        [
            (
                stride0,
                positions
                    .into_iter()
                    .map(|(_, a)| a)
                    .chain(normals.into_iter().map(|(_, a)| a))
                    .chain(binormals.into_iter().map(|(_, a)| a))
                    .chain(tangents.into_iter().map(|(_, a)| a))
                    .collect(),
            ),
            (
                stride1,
                texture_coordinates
                    .into_iter()
                    .map(|(_, a)| a)
                    .chain(color_sets.into_iter().map(|(_, a)| a))
                    .collect(),
            ),
            // These last two vertex buffers never seem to contain any attributes.
            (0, Vec::new()),
            (0, Vec::new()),
        ],
        MeshAttributes::AttributesV10(mesh_attributes.into()),
    )
}

// TODO: Test cases for this function.
pub(crate) fn write_attributes<W: Write + Seek>(
    buffer_info: &[(u32, Vec<AttributeBufferData>)],
    buffers: &mut [W],
    offsets: &[u64],
    version: MeshVersion,
) -> Result<(), std::io::Error> {
    // TODO: Avoid array indexing here?
    for (buffer_index, (stride, attribute_data)) in buffer_info.iter().enumerate() {
        let offset = offsets[buffer_index];
        let mut buffer = &mut buffers[buffer_index];

        let mut attribute_offset = 0;
        for data in attribute_data {
            let total_offset = offset + attribute_offset;

            match data {
                AttributeBufferData::Float2(v) => {
                    write_vector_data(&mut buffer, v, total_offset, *stride as u64, write_f32)?
                }
                AttributeBufferData::Float3(v) => {
                    write_vector_data(&mut buffer, v, total_offset, *stride as u64, write_f32)?
                }
                AttributeBufferData::Float4(v) => {
                    write_vector_data(&mut buffer, v, total_offset, *stride as u64, write_f32)?
                }
                AttributeBufferData::HalfFloat2(v) => {
                    write_vector_data(&mut buffer, v, total_offset, *stride as u64, write_f16)?
                }
                AttributeBufferData::HalfFloat4(v) => {
                    write_vector_data(&mut buffer, v, total_offset, *stride as u64, write_f16)?
                }
                AttributeBufferData::Byte4(v) => {
                    write_vector_data(&mut buffer, v, total_offset, *stride as u64, write_u8)?
                }
            }

            attribute_offset += match version {
                MeshVersion::Version110 => get_size_in_bytes_v10(&data.data_type_v10()) as u64,
                MeshVersion::Version108 => get_size_in_bytes_v8(&data.data_type_v8()) as u64,
            };
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::hex_bytes;

    use super::*;

    // TODO: Modify the functions to just take &[VectorData] instead of &[AttributeData]?
    fn create_attribute_data(data: &[VectorData]) -> Vec<AttributeData> {
        data.iter()
            .map(|data| AttributeData {
                name: String::new(),
                data: data.clone(),
            })
            .collect()
    }

    #[test]
    fn position_data_type_v10() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            vec![(String::new(), AttributeBufferData::Float2(vec![[0.0, 1.0]]))],
            get_position_data_v10(&create_attribute_data(&vec![VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::Float3(vec![[0.0, 1.0, 2.0]])
            )],
            get_position_data_v10(&create_attribute_data(&vec![VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::Float4(vec![[0.0, 1.0, 2.0, 3.0]])
            )],
            get_position_data_v10(&create_attribute_data(&vec![VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn vector_data_type_v10() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::HalfFloat2(vec![[f16::from_f32(0.0), f16::from_f32(1.0),]])
            )],
            get_vector_data_v10(&create_attribute_data(&vec![VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::Float3(vec![[0.0, 1.0, 2.0]])
            )],
            get_vector_data_v10(&create_attribute_data(&vec![VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::HalfFloat4(vec![[
                    f16::from_f32(0.0),
                    f16::from_f32(1.0),
                    f16::from_f32(2.0),
                    f16::from_f32(3.0)
                ]])
            )],
            get_vector_data_v10(&create_attribute_data(&vec![VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn color_data_type_v10() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::HalfFloat2(vec![[f16::from_f32(0.0), f16::from_f32(1.0)]])
            )],
            get_color_data_v10(&create_attribute_data(&vec![VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::Float3(vec![[0.0, 1.0, 2.0]])
            )],
            get_color_data_v10(&create_attribute_data(&vec![VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                AttributeBufferData::Byte4(vec![[0u8, 128u8, 255u8, 255u8]])
            )],
            get_color_data_v10(&create_attribute_data(&vec![VectorData::Vector4(vec![[
                0.0, 0.5, 1.0, 2.0
            ]])]))
        );
    }

    #[test]
    fn position_data_type_v8() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            vec![AttributeBufferData::Float2(vec![[0.0, 1.0]])],
            get_position_data_v8(&create_attribute_data(&vec![VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![AttributeBufferData::Float3(vec![[0.0, 1.0, 2.0]])],
            get_position_data_v8(&create_attribute_data(&vec![VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![AttributeBufferData::Float4(vec![[0.0, 1.0, 2.0, 3.0]])],
            get_position_data_v8(&create_attribute_data(&vec![VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn vector_data_type_v8() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            vec![AttributeBufferData::Float2(vec![[0.0, 1.0]])],
            get_vector_data_v8(&create_attribute_data(&vec![VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![AttributeBufferData::Float3(vec![[0.0, 1.0, 2.0]])],
            get_vector_data_v8(&create_attribute_data(&vec![VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![AttributeBufferData::HalfFloat4(vec![[
                f16::from_f32(0.0),
                f16::from_f32(1.0),
                f16::from_f32(2.0),
                f16::from_f32(3.0)
            ]])],
            get_vector_data_v8(&create_attribute_data(&vec![VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn color_data_type_v8() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            vec![AttributeBufferData::Float2(vec![[0.0, 1.0]])],
            get_color_data_v8(&create_attribute_data(&vec![VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![AttributeBufferData::Float3(vec![[0.0, 1.0, 2.0]])],
            get_color_data_v8(&create_attribute_data(&vec![VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![AttributeBufferData::Byte4(vec![[0u8, 128u8, 255u8, 255u8]])],
            get_color_data_v8(&create_attribute_data(&vec![VectorData::Vector4(vec![[
                0.0, 0.5, 1.0, 2.0
            ]])]))
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

        let ([(stride0, _), (stride1, _), _, _], attributes) = create_attributes_v8(&data);
        assert_eq!(32, stride0);
        assert_eq!(24, stride1);

        match attributes {
            MeshAttributes::AttributesV8(a) => {
                let mut attributes = a.elements.iter();

                // TODO: Use partial eq here.
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

        let ([(stride0, _), (stride1, _), _, _], attributes) = create_attributes_v10(&data);
        assert_eq!(56, stride0);
        assert_eq!(16, stride1);

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

    #[test]
    fn write_attributes_empty() {
        write_attributes::<std::io::Cursor<Vec<u8>>>(&[], &mut [], &[], MeshVersion::Version108)
            .unwrap();
        write_attributes::<std::io::Cursor<Vec<u8>>>(&[], &mut [], &[], MeshVersion::Version110)
            .unwrap();
    }

    #[test]
    fn write_attributes_single_buffer_single_attribute() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let buffer_info = vec![(12, vec![AttributeBufferData::Float3(vec![[1.0, 2.0, 3.0]])])];
        write_attributes(
            &buffer_info,
            &mut [&mut buffer0],
            &[0],
            MeshVersion::Version108,
        )
        .unwrap();
        write_attributes(
            &buffer_info,
            &mut [&mut buffer0],
            &[0],
            MeshVersion::Version110,
        )
        .unwrap();

        assert_eq!(&hex_bytes("0000803F 00000040 00004040"), buffer0.get_ref());
    }

    #[test]
    fn write_multiple_buffers_multiple_attributes_v10() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let mut buffer1 = Cursor::new(Vec::<u8>::new());

        let buffer_info = vec![(
            32,
            vec![
                AttributeBufferData::Float3(vec![[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]]),
                AttributeBufferData::HalfFloat2(vec![
                    [f16::from_f32(2.0), f16::from_f32(2.0)],
                    [f16::from_f32(2.0), f16::from_f32(2.0)],
                ]),
                AttributeBufferData::Float4(vec![[3.0, 3.0, 3.0, 3.0], [3.0, 3.0, 3.0, 3.0]]),
            ],
        )];
        write_attributes(
            &buffer_info,
            &mut [&mut buffer0, &mut buffer1],
            &[4, 8],
            MeshVersion::Version110,
        )
        .unwrap();

        assert_eq!(
            &hex_bytes(
                "00000000 0000803F 0000803F 0000803F 00400040 00004040 00004040 00004040 00004040
                          00000000 00000000 00000000 00400040 00004040 00004040 00004040 00004040"
            ),
            buffer0.get_ref()
        );
        assert_eq!(&hex_bytes(""), buffer1.get_ref());
    }

    #[test]
    fn write_multiple_buffers_multiple_attributes_v8() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let mut buffer1 = Cursor::new(Vec::<u8>::new());

        let buffer_info = vec![(
            32,
            vec![
                AttributeBufferData::Float3(vec![[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]]),
                AttributeBufferData::Float2(vec![[2.0, 2.0], [2.0, 2.0]]),
                AttributeBufferData::Float4(vec![[3.0, 3.0, 3.0, 3.0], [3.0, 3.0, 3.0, 3.0]]),
            ],
        )];
        write_attributes(
            &buffer_info,
            &mut [&mut buffer0, &mut buffer1],
            &[4, 8],
            MeshVersion::Version108,
        )
        .unwrap();

        assert_eq!(
            &hex_bytes(
                "00000000 0000803F 0000803F 0000803F 00000040 00000040 00004040 00004040 00004040
                          00000000 00000000 00000000 00000040 00000040 00004040 00004040 00004040"
            ),
            buffer0.get_ref()
        );
        assert_eq!(&hex_bytes(""), buffer1.get_ref());
    }
}
