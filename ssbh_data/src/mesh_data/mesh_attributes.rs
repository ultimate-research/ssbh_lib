use super::{
    AttributeData, AttributeDataTypeV10Ext, AttributeDataTypeV8Ext, MeshObjectData, VectorData,
};

use super::vector_data::*;
use half::f16;
use itertools::Itertools;
use ssbh_lib::{
    formats::mesh::{
        AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
        AttributeV10, AttributeV8, AttributeV9,
    },
    SsbhArray, SsbhString,
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

    fn from_positions(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::Float4(v.clone()),
        }
    }

    fn from_vectors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::HalfFloat2(get_f16_vectors(v)),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::HalfFloat4(get_f16_vectors(v)),
        }
    }

    fn from_colors(data: &VectorData) -> Self {
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

    fn from_positions(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::Float4(v.clone()),
        }
    }

    fn from_vectors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::HalfFloat4(get_f16_vectors(v)),
        }
    }

    fn from_colors(data: &VectorData) -> Self {
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

fn create_attributes_from_data<
    A: binread::BinRead,
    U,
    V,
    F1: Fn(Vec<(&str, usize, U, V)>, u32) -> Vec<(A, V)>,
    F2: Fn(&A) -> usize + Copy,
    F3: Fn(Vec<V>) -> VersionedVectorData,
>(
    buffer0_data: Vec<(&str, usize, U, V)>,
    buffer1_data: Vec<(&str, usize, U, V)>,
    stride2: u32,
    create_buffer_attributes: F1,
    size_in_bytes: F2,
    versioned_vectors: F3,
) -> ([(u32, VersionedVectorData); 4], SsbhArray<A>) {
    // Calculate attribute offsets and buffer data in the appropriate format.
    let buffer0_attributes = create_buffer_attributes(buffer0_data, 0);
    let buffer1_attributes = create_buffer_attributes(buffer1_data, 1);

    // Separate the mesh attributes from the buffer data.
    let (mut attributes0, vector_data0): (Vec<_>, Vec<_>) = buffer0_attributes.into_iter().unzip();
    let (attributes1, vector_data1): (Vec<_>, Vec<_>) = buffer1_attributes.into_iter().unzip();

    let stride0: usize = attributes0.iter().map(size_in_bytes).sum();
    let stride1: usize = attributes1.iter().map(size_in_bytes).sum();

    attributes0.extend(attributes1);
    (
        [
            (stride0 as u32, versioned_vectors(vector_data0)),
            (stride1 as u32, versioned_vectors(vector_data1)),
            // These last two vertex buffers never seem to contain any attributes.
            (stride2, versioned_vectors(Vec::new())),
            (0, versioned_vectors(Vec::new())),
        ],
        attributes0.into(),
    )
}

// TODO: More efficient to just take ownership of the vector data?
// TODO: Struct for the return type?
pub fn create_attributes_v8(
    data: &MeshObjectData,
) -> ([(u32, VersionedVectorData); 4], SsbhArray<AttributeV8>) {
    // Create a flattened list of attributes grouped by usage.
    // This ensures the attribute order matches existing conventions.
    let buffer0_data = get_positions_v8(&data.positions, AttributeUsageV8::Position)
        .chain(get_vectors_v8(&data.normals, AttributeUsageV8::Normal))
        .chain(get_vectors_v8(&data.tangents, AttributeUsageV8::Tangent))
        .collect_vec();

    let buffer1_data = get_vectors_v8(
        &data.texture_coordinates,
        AttributeUsageV8::TextureCoordinate,
    )
    .chain(get_colors_v8(&data.color_sets, AttributeUsageV8::ColorSet))
    .collect_vec();

    create_attributes_from_data(
        buffer0_data,
        buffer1_data,
        32,
        create_buffer_attributes_v8,
        |a: &AttributeV8| a.data_type.get_size_in_bytes_v8(),
        VersionedVectorData::V8,
    )
}

pub fn create_attributes_v9(
    data: &MeshObjectData,
) -> ([(u32, VersionedVectorData); 4], SsbhArray<AttributeV9>) {
    // Create a flattened list of attributes grouped by usage.
    // This ensures the attribute order matches existing conventions.
    let buffer0_data = get_positions_v9(&data.positions, AttributeUsageV9::Position)
        .chain(get_vectors_v9(&data.normals, AttributeUsageV9::Normal))
        .chain(get_vectors_v9(&data.binormals, AttributeUsageV9::Binormal))
        .chain(get_vectors_v9(&data.tangents, AttributeUsageV9::Tangent))
        .collect_vec();

    let buffer1_data = get_vectors_v9(
        &data.texture_coordinates,
        AttributeUsageV9::TextureCoordinate,
    )
    .chain(get_colors_v9(&data.color_sets, AttributeUsageV9::ColorSet))
    .collect_vec();

    create_attributes_from_data(
        buffer0_data,
        buffer1_data,
        32,
        create_buffer_attributes_v9,
        |a: &AttributeV9| a.data_type.get_size_in_bytes_v8(),
        VersionedVectorData::V8,
    )
}

// TODO: Fix this.
pub fn create_attributes_v10(
    data: &MeshObjectData,
) -> ([(u32, VersionedVectorData); 4], SsbhArray<AttributeV10>) {
    // Create a flattened list of attributes grouped by usage.
    // This ensures the attribute order matches existing conventions.
    let buffer0_data = get_positions_v10(&data.positions, AttributeUsageV9::Position)
        .chain(get_vectors_v10(&data.normals, AttributeUsageV9::Normal))
        .chain(get_vectors_v10(&data.binormals, AttributeUsageV9::Binormal))
        .chain(get_vectors_v10(&data.tangents, AttributeUsageV9::Tangent))
        .collect_vec();

    let buffer1_data = get_vectors_v10(
        &data.texture_coordinates,
        AttributeUsageV9::TextureCoordinate,
    )
    .chain(get_colors_v10(&data.color_sets, AttributeUsageV9::ColorSet))
    .collect_vec();

    create_attributes_from_data(
        buffer0_data,
        buffer1_data,
        0,
        create_buffer_attributes_v10,
        |a: &AttributeV10| a.data_type.get_size_in_bytes_v10(),
        VersionedVectorData::V10,
    )
}

fn get_attributes<U: Copy, V, F: Fn(&VectorData) -> V>(
    attributes: &[AttributeData],
    usage: U,
    f: F,
) -> impl Iterator<Item = (&str, usize, U, V)> {
    // Assign the appropriate name, usage, and subindex.
    // This avoids having to keep attributes grouped by usage.
    attributes
        .iter()
        .enumerate()
        .map(move |(i, a)| (a.name.as_str(), i, usage, f(&a.data)))
}

fn get_positions_v10(
    attributes: &[AttributeData],
    usage: AttributeUsageV9,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV9, VectorDataV10)> {
    get_attributes(attributes, usage, VectorDataV10::from_positions)
}

fn get_vectors_v10(
    attributes: &[AttributeData],
    usage: AttributeUsageV9,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV9, VectorDataV10)> {
    get_attributes(attributes, usage, VectorDataV10::from_vectors)
}

fn get_colors_v10(
    attributes: &[AttributeData],
    usage: AttributeUsageV9,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV9, VectorDataV10)> {
    get_attributes(attributes, usage, VectorDataV10::from_colors)
}

fn get_positions_v9(
    attributes: &[AttributeData],
    usage: AttributeUsageV9,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV9, VectorDataV8)> {
    get_attributes(attributes, usage, VectorDataV8::from_positions)
}

fn get_vectors_v9(
    attributes: &[AttributeData],
    usage: AttributeUsageV9,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV9, VectorDataV8)> {
    get_attributes(attributes, usage, VectorDataV8::from_vectors)
}

fn get_colors_v9(
    attributes: &[AttributeData],
    usage: AttributeUsageV9,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV9, VectorDataV8)> {
    get_attributes(attributes, usage, VectorDataV8::from_colors)
}

fn get_positions_v8(
    attributes: &[AttributeData],
    usage: AttributeUsageV8,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV8, VectorDataV8)> {
    get_attributes(attributes, usage, VectorDataV8::from_positions)
}

fn get_vectors_v8(
    attributes: &[AttributeData],
    usage: AttributeUsageV8,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV8, VectorDataV8)> {
    get_attributes(attributes, usage, VectorDataV8::from_vectors)
}

fn get_colors_v8(
    attributes: &[AttributeData],
    usage: AttributeUsageV8,
) -> impl Iterator<Item = (&str, usize, AttributeUsageV8, VectorDataV8)> {
    get_attributes(attributes, usage, VectorDataV8::from_colors)
}

fn create_buffer_attributes<
    Attribute,
    Usage: Copy,
    VectorData,
    DataType,
    F1: Fn(&str, usize, u32, Usage, DataType, usize) -> Attribute,
    F2: Fn(&VectorData) -> DataType,
    F3: Fn(&Attribute) -> usize,
>(
    buffer_data: Vec<(&str, usize, Usage, VectorData)>,
    buffer_index: u32,
    create_attribute: F1,
    data_type: F2,
    size_in_bytes: F3,
) -> Vec<(Attribute, VectorData)> {
    // For tightly packed data, the offset is a cumulative sum of size.
    let buffer_attributes = buffer_data
        .into_iter()
        .scan(0, |offset, (name, i, usage, data)| {
            let attribute =
                create_attribute(name, i, buffer_index, usage, data_type(&data), *offset);

            *offset += size_in_bytes(&attribute);

            Some((attribute, data))
        });
    buffer_attributes.collect_vec()
}

fn create_buffer_attributes_v8(
    buffer_data: Vec<(&str, usize, AttributeUsageV8, VectorDataV8)>,
    buffer_index: u32,
) -> Vec<(AttributeV8, VectorDataV8)> {
    create_buffer_attributes(
        buffer_data,
        buffer_index,
        create_attribute_v8,
        VectorDataV8::data_type,
        |a: &AttributeV8| a.data_type.get_size_in_bytes_v8(),
    )
}

fn create_buffer_attributes_v9(
    buffer_data: Vec<(&str, usize, AttributeUsageV9, VectorDataV8)>,
    buffer_index: u32,
) -> Vec<(AttributeV9, VectorDataV8)> {
    create_buffer_attributes(
        buffer_data,
        buffer_index,
        create_attribute_v9,
        VectorDataV8::data_type,
        |a: &AttributeV9| a.data_type.get_size_in_bytes_v8(),
    )
}

fn create_buffer_attributes_v10(
    buffer_data: Vec<(&str, usize, AttributeUsageV9, VectorDataV10)>,
    buffer_index: u32,
) -> Vec<(AttributeV10, VectorDataV10)> {
    create_buffer_attributes(
        buffer_data,
        buffer_index,
        create_attribute_v10,
        VectorDataV10::data_type,
        |a: &AttributeV10| a.data_type.get_size_in_bytes_v10(),
    )
}

fn create_attribute_v8(
    _name: &str,
    sub_index: usize,
    buffer_index: u32,
    usage: AttributeUsageV8,
    data_type: AttributeDataTypeV8,
    buffer_offset: usize,
) -> AttributeV8 {
    AttributeV8 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: buffer_offset as u32,
        sub_index: sub_index as u32,
    }
}

fn create_attribute_v9(
    name: &str,
    sub_index: usize,
    buffer_index: u32,
    usage: AttributeUsageV9,
    data_type: AttributeDataTypeV8,
    buffer_offset: usize,
) -> AttributeV9 {
    AttributeV9 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: buffer_offset as u32,
        sub_index: sub_index as u64,
        name: calculate_attribute_name(usage, sub_index, name),
        attribute_names: SsbhArray::new(vec![name.into()]),
    }
}

fn create_attribute_v10(
    name: &str,
    sub_index: usize,
    buffer_index: u32,
    usage: AttributeUsageV9,
    data_type: AttributeDataTypeV10,
    buffer_offset: usize,
) -> AttributeV10 {
    AttributeV10 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: buffer_offset as u32,
        sub_index: sub_index as u64,
        name: calculate_attribute_name(usage, sub_index, name),
        attribute_names: SsbhArray::new(vec![name.into()]),
    }
}

fn calculate_attribute_name(usage: AttributeUsageV9, sub_index: usize, name: &str) -> SsbhString {
    match (usage, sub_index) {
        // This is likely due to which UVs were used to generate the tangents/binormals.
        (AttributeUsageV9::Tangent, 0) => "map1".into(),
        (AttributeUsageV9::Binormal, 0) => "map1".into(),
        (AttributeUsageV9::Binormal, 1) => "uvSet".into(),
        _ => name.into(),
    }
}

pub(crate) fn write_attributes<W: Write + Seek>(
    buffer_info: &[(u32, VersionedVectorData)],
    buffers: &mut [W],
    offsets: &[u64],
) -> Result<(), std::io::Error> {
    for (buffer_index, (stride, attribute_data)) in buffer_info.iter().enumerate() {
        // TODO: Avoid array indexing here?
        let base_offset = offsets[buffer_index];
        let buffer = &mut buffers[buffer_index];

        // Accumulate the data size since data is tightly packed.
        match attribute_data {
            VersionedVectorData::V8(attribute_data) => {
                attribute_data
                    .iter()
                    .try_fold::<_, _, std::io::Result<u64>>(base_offset, |acc_offset, data| {
                        data.write(buffer, acc_offset, *stride as u64)?;
                        Ok(acc_offset + data.data_type().get_size_in_bytes_v8() as u64)
                    })?;
            }
            VersionedVectorData::V10(attribute_data) => {
                attribute_data
                    .iter()
                    .try_fold::<_, _, std::io::Result<u64>>(base_offset, |total_offset, data| {
                        data.write(buffer, total_offset, *stride as u64).unwrap();
                        Ok(total_offset + data.data_type().get_size_in_bytes_v10() as u64)
                    })?;
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

    #[test]
    fn position_data_type_v10() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            VectorDataV10::Float2(vec![[0.0, 1.0]]),
            VectorDataV10::from_positions(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV10::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV10::from_positions(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV10::Float4(vec![[0.0, 1.0, 2.0, 3.0]]),
            VectorDataV10::from_positions(&VectorData::Vector4(vec![[0.0, 1.0, 2.0, 3.0]]))
        );
    }

    #[test]
    fn vector_data_type_v10() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            VectorDataV10::HalfFloat2(vec![[f16::from_f32(0.0), f16::from_f32(1.0),]]),
            VectorDataV10::from_vectors(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV10::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV10::from_vectors(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV10::HalfFloat4(vec![[
                f16::from_f32(0.0),
                f16::from_f32(1.0),
                f16::from_f32(2.0),
                f16::from_f32(3.0)
            ]]),
            VectorDataV10::from_vectors(&VectorData::Vector4(vec![[0.0, 1.0, 2.0, 3.0]]))
        );
    }

    #[test]
    fn color_data_type_v10() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            VectorDataV10::HalfFloat2(vec![[f16::from_f32(0.0), f16::from_f32(1.0)]]),
            VectorDataV10::from_colors(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV10::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV10::from_colors(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV10::Byte4(vec![[0u8, 128u8, 255u8, 255u8]]),
            VectorDataV10::from_colors(&VectorData::Vector4(vec![[0.0, 0.5, 1.0, 2.0]]))
        );
    }

    #[test]
    fn position_data_type_v8_v9() {
        // Check that positions use the largest available floating point type.
        assert_eq!(
            VectorDataV8::Float2(vec![[0.0, 1.0]]),
            VectorDataV8::from_positions(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV8::from_positions(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV8::Float4(vec![[0.0, 1.0, 2.0, 3.0]]),
            VectorDataV8::from_positions(&VectorData::Vector4(vec![[0.0, 1.0, 2.0, 3.0]]))
        );
    }

    #[test]
    fn vector_data_type_v8_v9() {
        // Check that vectors use the smallest available floating point type.
        assert_eq!(
            VectorDataV8::Float2(vec![[0.0, 1.0]]),
            VectorDataV8::from_vectors(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV8::from_vectors(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV8::HalfFloat4(vec![[
                f16::from_f32(0.0),
                f16::from_f32(1.0),
                f16::from_f32(2.0),
                f16::from_f32(3.0)
            ]]),
            VectorDataV8::from_vectors(&VectorData::Vector4(vec![[0.0, 1.0, 2.0, 3.0]]))
        );
    }

    #[test]
    fn color_data_type_v8_v9() {
        // Check that color sets use the smallest available type.
        assert_eq!(
            VectorDataV8::Float2(vec![[0.0, 1.0]]),
            VectorDataV8::from_colors(&VectorData::Vector2(vec![[0.0, 1.0]]))
        );

        assert_eq!(
            VectorDataV8::Float3(vec![[0.0, 1.0, 2.0]]),
            VectorDataV8::from_colors(&VectorData::Vector3(vec![[0.0, 1.0, 2.0]]))
        );

        assert_eq!(
            VectorDataV8::Byte4(vec![[0u8, 128u8, 255u8, 255u8]]),
            VectorDataV8::from_colors(&VectorData::Vector4(vec![[0.0, 0.5, 1.0, 2.0]]))
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

        let mut attributes = attributes.elements.iter();

        // Check buffer 0.
        assert_eq!(
            &AttributeV8 {
                usage: AttributeUsageV8::Position,
                data_type: AttributeDataTypeV8::Float3,
                buffer_index: 0,
                buffer_offset: 0,
                sub_index: 0,
            },
            attributes.next().unwrap()
        );

        assert_eq!(
            &AttributeV8 {
                usage: AttributeUsageV8::Normal,
                data_type: AttributeDataTypeV8::Float3,
                buffer_index: 0,
                buffer_offset: 12,
                sub_index: 0,
            },
            attributes.next().unwrap()
        );

        assert_eq!(
            &AttributeV8 {
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
            &AttributeV8 {
                usage: AttributeUsageV8::TextureCoordinate,
                data_type: AttributeDataTypeV8::Float2,
                buffer_index: 1,
                buffer_offset: 0,
                sub_index: 0,
            },
            attributes.next().unwrap()
        );

        assert_eq!(
            &AttributeV8 {
                usage: AttributeUsageV8::TextureCoordinate,
                data_type: AttributeDataTypeV8::Float2,
                buffer_index: 1,
                buffer_offset: 8,
                sub_index: 1,
            },
            attributes.next().unwrap()
        );

        assert_eq!(
            &AttributeV8 {
                usage: AttributeUsageV8::ColorSet,
                data_type: AttributeDataTypeV8::Byte4,
                buffer_index: 1,
                buffer_offset: 16,
                sub_index: 0,
            },
            attributes.next().unwrap()
        );

        assert_eq!(
            &AttributeV8 {
                usage: AttributeUsageV8::ColorSet,
                data_type: AttributeDataTypeV8::Byte4,
                buffer_index: 1,
                buffer_offset: 20,
                sub_index: 1,
            },
            attributes.next().unwrap()
        );
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

        let mut attributes = attributes.elements.iter();
        // Check buffer 0.
        assert_eq!(
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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
            &AttributeV9 {
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

        let mut attributes = attributes.elements.iter();
        // Check buffer 0.
        assert_eq!(
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
            &AttributeV10 {
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
