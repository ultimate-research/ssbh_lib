use super::{get_size_in_bytes_v8, AttributeData, MeshObjectData, VectorData};
use crate::{
    get_u8_clamped, mesh_data::get_size_in_bytes_v10, write_f16, write_f32, write_u8,
    write_vector_data,
};
use half::f16;
use itertools::Itertools;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
        MeshAttributeV10, MeshAttributeV8, MeshAttributeV9, MeshAttributes,
    },
    SsbhArray,
};
use std::io::{Seek, Write};

#[derive(Debug, PartialEq)]
pub enum VersionedVectorData {
    V8(Vec<VectorDataV8>),
    V10(Vec<VectorDataV10>),
}

#[derive(Debug, PartialEq)]
pub enum VectorDataV10 {
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
    Float4(Vec<[f32; 4]>),
    HalfFloat2(Vec<[f16; 2]>),
    HalfFloat4(Vec<[f16; 4]>),
    Byte4(Vec<[u8; 4]>),
}

#[derive(Debug, PartialEq)]
pub enum VectorDataV8 {
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
    Float4(Vec<[f32; 4]>),
    HalfFloat4(Vec<[f16; 4]>),
    Byte4(Vec<[u8; 4]>),
}

impl VectorDataV10 {
    fn data_type(&self) -> AttributeDataTypeV10 {
        match self {
            VectorDataV10::Float2(_) => AttributeDataTypeV10::Float2,
            VectorDataV10::Float3(_) => AttributeDataTypeV10::Float3,
            VectorDataV10::Float4(_) => AttributeDataTypeV10::Float4,
            VectorDataV10::HalfFloat4(_) => AttributeDataTypeV10::HalfFloat4,
            VectorDataV10::Byte4(_) => AttributeDataTypeV10::Byte4,
            VectorDataV10::HalfFloat2(_) => AttributeDataTypeV10::HalfFloat2,
        }
    }

    fn write<W: Write + Seek>(
        &self,
        buffer: &mut W,
        offset: u64,
        stride: u64,
    ) -> std::io::Result<()> {
        match self {
            VectorDataV10::Float2(v) => write_vector_data(buffer, v, offset, stride, write_f32)?,
            VectorDataV10::Float3(v) => write_vector_data(buffer, v, offset, stride, write_f32)?,
            VectorDataV10::Float4(v) => write_vector_data(buffer, v, offset, stride, write_f32)?,
            VectorDataV10::HalfFloat2(v) => {
                write_vector_data(buffer, v, offset, stride, write_f16)?
            }
            VectorDataV10::HalfFloat4(v) => {
                write_vector_data(buffer, v, offset, stride, write_f16)?
            }
            VectorDataV10::Byte4(v) => write_vector_data(buffer, v, offset, stride, write_u8)?,
        }
        Ok(())
    }

    fn from_position_data(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::Float4(v.clone()),
        }
    }

    fn from_vector_data(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::HalfFloat2(get_f16_vectors(v)),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::HalfFloat4(get_f16_vectors(v)),
        }
    }

    fn from_color_data(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::HalfFloat2(get_f16_vectors(v)),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::Byte4(get_clamped_u8_vectors(v)),
        }
    }
}

impl VectorDataV8 {
    fn data_type(&self) -> AttributeDataTypeV8 {
        match self {
            VectorDataV8::Float2(_) => AttributeDataTypeV8::Float2,
            VectorDataV8::Float3(_) => AttributeDataTypeV8::Float3,
            VectorDataV8::Float4(_) => AttributeDataTypeV8::Float4,
            VectorDataV8::HalfFloat4(_) => AttributeDataTypeV8::HalfFloat4,
            VectorDataV8::Byte4(_) => AttributeDataTypeV8::Byte4,
        }
    }

    fn write<W: Write + Seek>(
        &self,
        buffer: &mut W,
        offset: u64,
        stride: u64,
    ) -> std::io::Result<()> {
        match self {
            VectorDataV8::Float2(v) => write_vector_data(buffer, v, offset, stride, write_f32)?,
            VectorDataV8::Float3(v) => write_vector_data(buffer, v, offset, stride, write_f32)?,
            VectorDataV8::Float4(v) => write_vector_data(buffer, v, offset, stride, write_f32)?,
            VectorDataV8::HalfFloat4(v) => write_vector_data(buffer, v, offset, stride, write_f16)?,
            VectorDataV8::Byte4(v) => write_vector_data(buffer, v, offset, stride, write_u8)?,
        }
        Ok(())
    }

    fn from_position_data(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::Float4(v.clone()),
        }
    }

    fn from_vector_data(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::HalfFloat4(get_f16_vectors(v)),
        }
    }

    fn from_color_data(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::Byte4(get_clamped_u8_vectors(v)),
        }
    }
}

fn get_f16_vector<const N: usize>(vector: &[f32; N]) -> [f16; N] {
    let mut output = [f16::ZERO; N];
    for i in 0..N {
        output[i] = f16::from_f32(vector[i]);
    }
    output
}

fn get_clamped_u8_vector<const N: usize>(vector: &[f32; N]) -> [u8; N] {
    let mut output = [0u8; N];
    for i in 0..N {
        output[i] = get_u8_clamped(vector[i]);
    }
    output
}

fn get_f16_vectors<const N: usize>(vector: &[[f32; N]]) -> Vec<[f16; N]> {
    vector.iter().map(get_f16_vector).collect()
}

fn get_clamped_u8_vectors<const N: usize>(vector: &[[f32; N]]) -> Vec<[u8; N]> {
    vector.iter().map(get_clamped_u8_vector).collect()
}

fn get_position_data_v8(data: &[AttributeData]) -> Vec<VectorDataV8> {
    data.iter()
        .map(|a| VectorDataV8::from_position_data(&a.data))
        .collect()
}

fn get_position_data_v9(data: &[AttributeData]) -> Vec<(String, VectorDataV8)> {
    data.iter()
        .map(|a| (a.name.clone(), VectorDataV8::from_position_data(&a.data)))
        .collect()
}

fn get_vector_data_v8(data: &[AttributeData]) -> Vec<VectorDataV8> {
    data.iter()
        .map(|a| VectorDataV8::from_vector_data(&a.data))
        .collect()
}

fn get_vector_data_v9(data: &[AttributeData]) -> Vec<(String, VectorDataV8)> {
    data.iter()
        .map(|a| (a.name.clone(), VectorDataV8::from_vector_data(&a.data)))
        .collect()
}

fn get_color_data_v8(data: &[AttributeData]) -> Vec<VectorDataV8> {
    data.iter()
        .map(|a| VectorDataV8::from_color_data(&a.data))
        .collect()
}

fn get_color_data_v9(data: &[AttributeData]) -> Vec<(String, VectorDataV8)> {
    data.iter()
        .map(|a| (a.name.clone(), VectorDataV8::from_color_data(&a.data)))
        .collect()
}

// TODO: More efficient to just take ownership of the vector data?
// TODO: create a struct for returning (strides, data, attributes)
pub fn create_attributes_v8(
    data: &MeshObjectData,
) -> ([(u32, VersionedVectorData); 4], MeshAttributes) {
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
                VersionedVectorData::V8(
                    positions
                        .into_iter()
                        .chain(normals.into_iter())
                        .chain(tangents.into_iter())
                        .collect(),
                ),
            ),
            (
                stride1,
                VersionedVectorData::V8(
                    texture_coordinates
                        .into_iter()
                        .chain(color_sets.into_iter())
                        .collect(),
                ),
            ),
            // These last two vertex buffers never seem to contain any attributes.
            (32, VersionedVectorData::V8(Vec::new())),
            (0, VersionedVectorData::V8(Vec::new())),
        ],
        MeshAttributes::AttributesV8(mesh_attributes.into()),
    )
}

pub fn create_attributes_v9(
    data: &MeshObjectData,
) -> ([(u32, VersionedVectorData); 4], MeshAttributes) {
    // 1. Convert the data into the appropriate format based on usage and component count.
    // TODO: Avoid collecting until the end?
    let positions = get_position_data_v9(&data.positions);
    let normals = get_vector_data_v9(&data.normals);
    let binormals = get_vector_data_v9(&data.binormals);
    let tangents = get_vector_data_v9(&data.tangents);
    let texture_coordinates = get_vector_data_v9(&data.texture_coordinates);
    let color_sets = get_color_data_v9(&data.color_sets);

    // 2. Compute the strides + offsets + attributes
    let mut stride0 = 0;
    let mut stride1 = 0;
    let mut mesh_attributes = Vec::new();

    add_attributes_v9(
        &mut mesh_attributes,
        &positions,
        &mut stride0,
        0,
        AttributeUsageV9::Position,
    );
    add_attributes_v9(
        &mut mesh_attributes,
        &normals,
        &mut stride0,
        0,
        AttributeUsageV9::Normal,
    );
    add_attributes_v9(
        &mut mesh_attributes,
        &binormals,
        &mut stride0,
        0,
        AttributeUsageV9::Binormal,
    );
    add_attributes_v9(
        &mut mesh_attributes,
        &tangents,
        &mut stride0,
        0,
        AttributeUsageV9::Tangent,
    );

    add_attributes_v9(
        &mut mesh_attributes,
        &texture_coordinates,
        &mut stride1,
        1,
        AttributeUsageV9::TextureCoordinate,
    );
    add_attributes_v9(
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
                VersionedVectorData::V8(
                    positions
                        .into_iter()
                        .map(|(_, a)| a)
                        .chain(normals.into_iter().map(|(_, a)| a))
                        .chain(binormals.into_iter().map(|(_, a)| a))
                        .chain(tangents.into_iter().map(|(_, a)| a))
                        .collect(),
                ),
            ),
            (
                stride1,
                VersionedVectorData::V8(
                    texture_coordinates
                        .into_iter()
                        .map(|(_, a)| a)
                        .chain(color_sets.into_iter().map(|(_, a)| a))
                        .collect(),
                ),
            ),
            // These last two vertex buffers never seem to contain any attributes.
            (32, VersionedVectorData::V8(Vec::new())),
            (0, VersionedVectorData::V8(Vec::new())),
        ],
        MeshAttributes::AttributesV9(mesh_attributes.into()),
    )
}

fn add_attributes_v8(
    attributes: &mut Vec<MeshAttributeV8>,
    attributes_to_add: &[VectorDataV8],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV8,
) {
    for (i, attribute) in attributes_to_add.iter().enumerate() {
        let data_type = attribute.data_type();

        let attribute = MeshAttributeV8 {
            usage,
            data_type,
            buffer_index,
            buffer_offset: *current_stride,
            sub_index: i as u32,
        };

        *current_stride += get_size_in_bytes_v8(&attribute.data_type) as u32;
        attributes.push(attribute);
    }
}

fn add_attributes_v9(
    attributes: &mut Vec<MeshAttributeV9>,
    attributes_to_add: &[(String, VectorDataV8)],
    current_stride: &mut u32,
    buffer_index: u32,
    usage: AttributeUsageV9,
) {
    // TODO: Preserve name as well as array name?
    for (i, (attribute_name, attribute)) in attributes_to_add.iter().enumerate() {
        let data_type = attribute.data_type();

        // This is likely due to which UVs were used to generate the tangents/binormals.x
        let name = match (usage, i) {
            (AttributeUsageV9::Tangent, 0) => "map1",
            (AttributeUsageV9::Binormal, 0) => "map1",
            (AttributeUsageV9::Binormal, 1) => "uvSet",
            _ => attribute_name,
        };

        let attribute = MeshAttributeV9 {
            usage,
            data_type,
            buffer_index,
            buffer_offset: *current_stride,
            sub_index: i as u64,
            name: name.into(),
            attribute_names: SsbhArray::new(vec![attribute_name.as_str().into()]),
        };

        *current_stride += get_size_in_bytes_v8(&attribute.data_type) as u32;
        attributes.push(attribute);
    }
}

pub fn create_attributes_v10(
    data: &MeshObjectData,
) -> ([(u32, VersionedVectorData); 4], MeshAttributes) {
    // Create a flattened list of attributes grouped by usage.
    // This ensures the attribute order matches existing conventions.
    let buffer0_data = data
        .positions
        .iter()
        .enumerate()
        .map(|(i, a)| {
            (
                &a.name,
                i,
                AttributeUsageV9::Position,
                VectorDataV10::from_position_data(&a.data),
            )
        })
        .chain(data.normals.iter().enumerate().map(|(i, a)| {
            (
                &a.name,
                i,
                AttributeUsageV9::Normal,
                VectorDataV10::from_vector_data(&a.data),
            )
        }))
        .chain(data.binormals.iter().enumerate().map(|(i, a)| {
            (
                &a.name,
                i,
                AttributeUsageV9::Binormal,
                VectorDataV10::from_vector_data(&a.data),
            )
        }))
        .chain(data.tangents.iter().enumerate().map(|(i, a)| {
            (
                &a.name,
                i,
                AttributeUsageV9::Tangent,
                VectorDataV10::from_vector_data(&a.data),
            )
        }));
    // TODO: Do we want to collect here?

    let buffer0_attributes = create_buffer_attributes(buffer0_data);

    // TODO: How to avoid repetition when doing buffer1?
    let buffer1_data = data
        .texture_coordinates
        .iter()
        .enumerate()
        .map(|(i, a)| {
            (
                &a.name,
                i,
                AttributeUsageV9::TextureCoordinate,
                VectorDataV10::from_vector_data(&a.data),
            )
        })
        .chain(data.color_sets.iter().enumerate().map(|(i, a)| {
            (
                &a.name,
                i,
                AttributeUsageV9::ColorSet,
                VectorDataV10::from_color_data(&a.data),
            )
        }));

    let buffer1_attributes = create_buffer_attributes(buffer1_data);

    // Separate the vector data and attributes.
    let (mut attributes0, vector_data0): (Vec<_>, Vec<_>) = buffer0_attributes.into_iter().unzip();
    let (attributes1, vector_data1): (Vec<_>, Vec<_>) = buffer1_attributes.into_iter().unzip();

    // TODO: Is there a way to calculate stride in the previous function?
    let stride0: usize = attributes0
        .iter()
        .map(|a| get_size_in_bytes_v10(&a.data_type))
        .sum();

    let stride1: usize = attributes1
        .iter()
        .map(|a| get_size_in_bytes_v10(&a.data_type))
        .sum();

    // TODO: Chaining the attributes like this is confusing.
    attributes0.extend(attributes1);
    (
        [
            (stride0 as u32, VersionedVectorData::V10(vector_data0)),
            (stride1 as u32, VersionedVectorData::V10(vector_data1)),
            // These last two vertex buffers never seem to contain any attributes.
            (0, VersionedVectorData::V10(Vec::new())),
            (0, VersionedVectorData::V10(Vec::new())),
        ],
        MeshAttributes::AttributesV10(attributes0.into()),
    )
}

fn create_buffer_attributes<
    'a,
    I: Iterator<Item = (&'a String, usize, AttributeUsageV9, VectorDataV10)>,
>(
    buffer0_data: I,
) -> Vec<(MeshAttributeV10, VectorDataV10)> {
    // For tightly packed data, the offset is a cumulative sum of size.
    let buffer0_attributes = buffer0_data.scan(0, |offset, (name, i, usage, data)| {
        let attribute = create_attribute_v10(name, i, 0, usage, data.data_type(), *offset);
        *offset += get_size_in_bytes_v10(&attribute.data_type);
        Some((attribute, data))
    });
    buffer0_attributes.collect_vec()
}

fn create_attribute_v10(
    name: &str,
    i: usize,
    buffer_index: u32,
    usage: AttributeUsageV9,
    data_type: AttributeDataTypeV10,
    buffer_offset: usize,
) -> MeshAttributeV10 {
    let attribute = MeshAttributeV10 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: buffer_offset as u32,
        sub_index: i as u64,
        name: match (usage, i) {
            // This is likely due to which UVs were used to generate the tangents/binormals.
            (AttributeUsageV9::Tangent, 0) => "map1".into(),
            (AttributeUsageV9::Binormal, 0) => "map1".into(),
            (AttributeUsageV9::Binormal, 1) => "uvSet".into(),
            _ => name.into(),
        },
        attribute_names: SsbhArray::new(vec![name.into()]),
    };
    attribute
}

pub(crate) fn write_attributes<W: Write + Seek>(
    buffer_info: &[(u32, VersionedVectorData)],
    buffers: &mut [W],
    offsets: &[u64],
) -> Result<(), std::io::Error> {
    for (buffer_index, (stride, attribute_data)) in buffer_info.iter().enumerate() {
        // TODO: Avoid array indexing here?
        let offset = offsets[buffer_index];
        let buffer = &mut buffers[buffer_index];

        match attribute_data {
            VersionedVectorData::V8(attribute_data) => {
                let mut attribute_offset = 0;
                for data in attribute_data {
                    let total_offset = offset + attribute_offset;
                    data.write(buffer, total_offset, *stride as u64)?;

                    attribute_offset += get_size_in_bytes_v8(&data.data_type()) as u64;
                }
            }
            VersionedVectorData::V10(attribute_data) => {
                let mut attribute_offset = 0;

                for data in attribute_data {
                    let total_offset = offset + attribute_offset;
                    data.write(buffer, total_offset, *stride as u64)?;

                    attribute_offset += get_size_in_bytes_v10(&data.data_type()) as u64;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;
    use std::io::Cursor;

    fn create_attribute_data(data: &[VectorData]) -> Vec<AttributeData> {
        data.iter()
            .map(|data| AttributeData {
                name: String::new(),
                data: data.clone(),
            })
            .collect()
    }

    #[test]
    fn position_data_type_v9() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            vec![(String::new(), VectorDataV8::Float2(vec![[0.0, 1.0]]))],
            get_position_data_v9(&create_attribute_data(&[VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![(String::new(), VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]]))],
            get_position_data_v9(&create_attribute_data(&[VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                VectorDataV8::Float4(vec![[0.0, 1.0, 2.0, 3.0]])
            )],
            get_position_data_v9(&create_attribute_data(&[VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn position_data_type_v10() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            VectorDataV10::Float2(vec![[0.0, 1.0]]),
            VectorDataV10::from_position_data(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV10::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV10::from_position_data(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV10::Float4(vec![[0.0, 1.0, 2.0, 3.0]]),
            VectorDataV10::from_position_data(&VectorData::Vector4(vec![[0.0, 1.0, 2.0, 3.0]]))
        );
    }

    #[test]
    fn vector_data_type_v9() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            vec![(String::new(), VectorDataV8::Float2(vec![[0.0, 1.0]]))],
            get_vector_data_v9(&create_attribute_data(&[VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![(String::new(), VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]]))],
            get_vector_data_v9(&create_attribute_data(&[VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                VectorDataV8::HalfFloat4(vec![[
                    f16::from_f32(0.0),
                    f16::from_f32(1.0),
                    f16::from_f32(2.0),
                    f16::from_f32(3.0)
                ]])
            )],
            get_vector_data_v9(&create_attribute_data(&[VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn vector_data_type_v10() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            VectorDataV10::HalfFloat2(vec![[f16::from_f32(0.0), f16::from_f32(1.0),]]),
            VectorDataV10::from_vector_data(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV10::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV10::from_vector_data(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV10::HalfFloat4(vec![[
                f16::from_f32(0.0),
                f16::from_f32(1.0),
                f16::from_f32(2.0),
                f16::from_f32(3.0)
            ]]),
            VectorDataV10::from_vector_data(&VectorData::Vector4(vec![[0.0, 1.0, 2.0, 3.0]]))
        );
    }

    #[test]
    fn color_data_type_v9() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            vec![(String::new(), VectorDataV8::Float2(vec![[0.0, 1.0]]))],
            get_color_data_v9(&create_attribute_data(&[VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![(String::new(), VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]]))],
            get_color_data_v9(&create_attribute_data(&[VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![(
                String::new(),
                VectorDataV8::Byte4(vec![[0u8, 128u8, 255u8, 255u8]])
            )],
            get_color_data_v9(&create_attribute_data(&[VectorData::Vector4(vec![[
                0.0, 0.5, 1.0, 2.0
            ]])]))
        );
    }

    #[test]
    fn color_data_type_v10() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            VectorDataV10::HalfFloat2(vec![[f16::from_f32(0.0), f16::from_f32(1.0)]]),
            VectorDataV10::from_color_data(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV10::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV10::from_color_data(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV10::Byte4(vec![[0u8, 128u8, 255u8, 255u8]]),
            VectorDataV10::from_color_data(&VectorData::Vector4(vec![[0.0, 0.5, 1.0, 2.0]]))
        );
    }

    #[test]
    fn position_data_type_v8() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            vec![VectorDataV8::Float2(vec![[0.0, 1.0]])],
            get_position_data_v8(&create_attribute_data(&[VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]])],
            get_position_data_v8(&create_attribute_data(&[VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![VectorDataV8::Float4(vec![[0.0, 1.0, 2.0, 3.0]])],
            get_position_data_v8(&create_attribute_data(&[VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn vector_data_type_v8() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            vec![VectorDataV8::Float2(vec![[0.0, 1.0]])],
            get_vector_data_v8(&create_attribute_data(&[VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]])],
            get_vector_data_v8(&create_attribute_data(&[VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![VectorDataV8::HalfFloat4(vec![[
                f16::from_f32(0.0),
                f16::from_f32(1.0),
                f16::from_f32(2.0),
                f16::from_f32(3.0)
            ]])],
            get_vector_data_v8(&create_attribute_data(&[VectorData::Vector4(vec![[
                0.0, 1.0, 2.0, 3.0
            ]])]))
        );
    }

    #[test]
    fn color_data_type_v8() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            vec![VectorDataV8::Float2(vec![[0.0, 1.0]])],
            get_color_data_v8(&create_attribute_data(&[VectorData::Vector2(vec![[
                0.0, 1.0
            ]])]))
        );

        assert_eq!(
            vec![VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]])],
            get_color_data_v8(&create_attribute_data(&[VectorData::Vector3(vec![[
                0.0, 1.0, 2.0
            ]])]))
        );

        assert_eq!(
            vec![VectorDataV8::Byte4(vec![[0u8, 128u8, 255u8, 255u8]])],
            get_color_data_v8(&create_attribute_data(&[VectorData::Vector4(vec![[
                0.0, 0.5, 1.0, 2.0
            ]])]))
        );
    }

    #[test]
    fn create_attributes_mesh_v1_8() {
        let data = MeshObjectData {
            name: "name".into(),
            positions: vec![AttributeData {
                name: "p0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
            normals: vec![AttributeData {
                name: "n0".into(),
                data: VectorData::Vector3(Vec::new()),
            }],
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
            ..MeshObjectData::default()
        };

        let ([(stride0, _), (stride1, _), (stride2, _), (stride3, _)], attributes) =
            create_attributes_v8(&data);
        assert_eq!(32, stride0);
        assert_eq!(24, stride1);
        assert_eq!(32, stride2);
        assert_eq!(0, stride3);

        match attributes {
            MeshAttributes::AttributesV8(a) => {
                let mut attributes = a.elements.iter();

                // Check buffer 0.
                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::Position,
                        data_type: AttributeDataTypeV8::Float3,
                        buffer_index: 0,
                        buffer_offset: 0,
                        sub_index: 0,
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::Normal,
                        data_type: AttributeDataTypeV8::Float3,
                        buffer_index: 0,
                        buffer_offset: 12,
                        sub_index: 0,
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::Tangent,
                        data_type: AttributeDataTypeV8::HalfFloat4,
                        buffer_index: 0,
                        buffer_offset: 24,
                        sub_index: 0,
                    },
                    attributes.next().unwrap()
                );

                // Check buffer 1.
                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::TextureCoordinate,
                        data_type: AttributeDataTypeV8::Float2,
                        buffer_index: 1,
                        buffer_offset: 0,
                        sub_index: 0,
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::TextureCoordinate,
                        data_type: AttributeDataTypeV8::Float2,
                        buffer_index: 1,
                        buffer_offset: 8,
                        sub_index: 1,
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::ColorSet,
                        data_type: AttributeDataTypeV8::Byte4,
                        buffer_index: 1,
                        buffer_offset: 16,
                        sub_index: 0,
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV8 {
                        usage: AttributeUsageV8::ColorSet,
                        data_type: AttributeDataTypeV8::Byte4,
                        buffer_index: 1,
                        buffer_offset: 20,
                        sub_index: 1,
                    },
                    attributes.next().unwrap()
                );
            }
            _ => panic!("invalid version"),
        };
    }

    #[test]
    fn create_attributes_mesh_v1_9() {
        let data = MeshObjectData {
            name: "name".into(),
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
            ..MeshObjectData::default()
        };

        let ([(stride0, _), (stride1, _), (stride2, _), (stride3, _)], attributes) =
            create_attributes_v9(&data);
        assert_eq!(56, stride0);
        assert_eq!(24, stride1);
        assert_eq!(32, stride2);
        assert_eq!(0, stride3);

        match attributes {
            MeshAttributes::AttributesV9(a) => {
                let mut attributes = a.elements.iter();
                // Check buffer 0.
                assert_eq!(
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::Position,
                        data_type: AttributeDataTypeV8::Float3,
                        buffer_index: 0,
                        buffer_offset: 0,
                        sub_index: 0,
                        name: "p0".into(),
                        attribute_names: SsbhArray::new(vec!["p0".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::Normal,
                        data_type: AttributeDataTypeV8::Float3,
                        buffer_index: 0,
                        buffer_offset: 12,
                        sub_index: 0,
                        name: "n0".into(),
                        attribute_names: SsbhArray::new(vec!["n0".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::Binormal,
                        data_type: AttributeDataTypeV8::Float3,
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
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::Binormal,
                        data_type: AttributeDataTypeV8::Float3,
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
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::Tangent,
                        data_type: AttributeDataTypeV8::HalfFloat4,
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
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::TextureCoordinate,
                        data_type: AttributeDataTypeV8::Float2,
                        buffer_index: 1,
                        buffer_offset: 0,
                        sub_index: 0,
                        name: "firstUv".into(),
                        attribute_names: SsbhArray::new(vec!["firstUv".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::TextureCoordinate,
                        data_type: AttributeDataTypeV8::Float2,
                        buffer_index: 1,
                        buffer_offset: 8,
                        sub_index: 1,
                        name: "secondUv".into(),
                        attribute_names: SsbhArray::new(vec!["secondUv".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::ColorSet,
                        data_type: AttributeDataTypeV8::Byte4,
                        buffer_index: 1,
                        buffer_offset: 16,
                        sub_index: 0,
                        name: "color1".into(),
                        attribute_names: SsbhArray::new(vec!["color1".into()]),
                    },
                    attributes.next().unwrap()
                );

                assert_eq!(
                    &MeshAttributeV9 {
                        usage: AttributeUsageV9::ColorSet,
                        data_type: AttributeDataTypeV8::Byte4,
                        buffer_index: 1,
                        buffer_offset: 20,
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
            sort_bias: 0,
            disable_depth_test: false,
            disable_depth_write: false,
        };

        let ([(stride0, _), (stride1, _), (stride2, _), (stride3, _)], attributes) =
            create_attributes_v10(&data);
        assert_eq!(56, stride0);
        assert_eq!(16, stride1);
        assert_eq!(0, stride2);
        assert_eq!(0, stride3);

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
        write_attributes::<std::io::Cursor<Vec<u8>>>(&[], &mut [], &[]).unwrap();
    }

    #[test]
    fn write_attributes_single_buffer_single_attribute_v8() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let buffer_info = vec![(
            12,
            VersionedVectorData::V8(vec![VectorDataV8::Float3(vec![[1.0, 2.0, 3.0]])]),
        )];
        write_attributes(&buffer_info, &mut [&mut buffer0], &[0]).unwrap();

        assert_eq!(*buffer0.get_ref(), hex!(0000803F 00000040 00004040),);
    }

    #[test]
    fn write_attributes_single_buffer_single_attribute_v10() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let buffer_info = vec![(
            12,
            VersionedVectorData::V10(vec![VectorDataV10::Float3(vec![[1.0, 2.0, 3.0]])]),
        )];
        write_attributes(&buffer_info, &mut [&mut buffer0], &[0]).unwrap();

        assert_eq!(*buffer0.get_ref(), hex!(0000803F 00000040 00004040));
    }

    #[test]
    fn write_multiple_buffers_multiple_attributes_v8() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let mut buffer1 = Cursor::new(Vec::<u8>::new());

        let buffer_info = vec![
            (
                36,
                VersionedVectorData::V8(vec![
                    VectorDataV8::Float3(vec![[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]]),
                    VectorDataV8::Float2(vec![[2.0, 2.0], [2.0, 2.0]]),
                    VectorDataV8::Float4(vec![[3.0, 3.0, 3.0, 3.0], [3.0, 3.0, 3.0, 3.0]]),
                ]),
            ),
            (
                16,
                VersionedVectorData::V8(vec![
                    VectorDataV8::Float2(vec![[1.0, 1.0], [0.0, 0.0]]),
                    VectorDataV8::Float2(vec![[2.0, 2.0], [2.0, 2.0]]),
                ]),
            ),
        ];
        write_attributes(&buffer_info, &mut [&mut buffer0, &mut buffer1], &[4, 8]).unwrap();

        assert_eq!(
            *buffer0.get_ref(),
            hex!(
                // Offset
                00000000
                // Vertex 0
                0000803F 0000803F 0000803F 00000040 00000040 00004040 00004040 00004040 00004040
                // Vertex 1
                00000000 00000000 00000000 00000040 00000040 00004040 00004040 00004040 00004040
            )
        );
        assert_eq!(
            *buffer1.get_ref(),
            hex!(
                // Offset
                00000000 00000000
                // Vertex 0
                0000803F 0000803F 00000040 00000040
                // Vertex 1
                00000000 00000000 00000040 00000040
            )
        );
    }

    #[test]
    fn write_multiple_buffers_multiple_attributes_v10() {
        let mut buffer0 = Cursor::new(Vec::<u8>::new());
        let mut buffer1 = Cursor::new(Vec::<u8>::new());

        let buffer_info = vec![
            (
                32,
                VersionedVectorData::V10(vec![
                    VectorDataV10::Float3(vec![[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]]),
                    VectorDataV10::HalfFloat2(vec![
                        [f16::from_f32(2.0), f16::from_f32(2.0)],
                        [f16::from_f32(2.0), f16::from_f32(2.0)],
                    ]),
                    VectorDataV10::Float4(vec![[3.0, 3.0, 3.0, 3.0], [3.0, 3.0, 3.0, 3.0]]),
                ]),
            ),
            (
                16,
                VersionedVectorData::V8(vec![
                    VectorDataV8::Float2(vec![[1.0, 1.0], [0.0, 0.0]]),
                    VectorDataV8::Float2(vec![[2.0, 2.0], [2.0, 2.0]]),
                ]),
            ),
        ];
        write_attributes(&buffer_info, &mut [&mut buffer0, &mut buffer1], &[4, 8]).unwrap();

        assert_eq!(
            *buffer0.get_ref(),
            hex!(
                // Offset
                00000000
                // Vertex 0
                0000803F 0000803F 0000803F 00400040 00004040 00004040 00004040 00004040
                // Vertex 1
                00000000 00000000 00000000 00400040 00004040 00004040 00004040 00004040
            ),
        );
        assert_eq!(
            *buffer1.get_ref(),
            hex!(
                // Offset
                00000000 00000000
                // Vertex 0
                0000803F 0000803F 00000040 00000040
                // Vertex 1
                00000000 00000000 00000040 00000040
            ),
        );
    }
}
