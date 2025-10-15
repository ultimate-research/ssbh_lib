use binrw::BinReaderExt;
use binrw::io::{Read, Write};
use binrw::io::{Seek, SeekFrom};
use binrw::{BinRead, BinResult};
use glam::{Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles, vec4};
use half::f16;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_lib::formats::mesh::{AttributeDataTypeV8, AttributeDataTypeV10};
use std::ops::Mul;

use super::{DataType, Half};

/// The data for a vertex attribute.
///
/// The precision when saving is inferred based on supported data types for the version specified in the [MeshData](super::MeshData).
/// For example, position attributes will prefer the highest available precision ([f32]), and color sets will prefer the lowest available precision ([u8]).
/// *The data type selected for saving may change between releases but will always retain the specified component count such as [VectorData::Vector2] vs [VectorData::Vector4].*
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorData {
    Vector2(Vec<Vec2>),
    Vector3(Vec<Vec3>),
    Vector4(Vec<Vec4>),
}

impl VectorData {
    /// The number of vectors.
    ///
    /// # Examples
    /**
    ```rust
    # use ssbh_data::mesh_data::VectorData;
    let data = VectorData::Vector2(vec![glam::vec2(0.0, 1.0); 3]);
    assert_eq!(3, data.len());
    ```
    */
    pub fn len(&self) -> usize {
        match self {
            VectorData::Vector2(v) => v.len(),
            VectorData::Vector3(v) => v.len(),
            VectorData::Vector4(v) => v.len(),
        }
    }

    /// Returns `true` if there are no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pads the data to 4 components per vector with a specified w component.
    /// This includes replacing the w component for [VectorData::Vector4].
    /**
    ```rust
    # use ssbh_data::mesh_data::VectorData;
    let data2 = VectorData::Vector2(vec![glam::vec2(1.0, 2.0)]);
    assert_eq!(vec![glam::vec4(1.0, 2.0, 0.0, 4.0)], data2.to_vec4_with_w(4.0));

    let data3 = VectorData::Vector3(vec![glam::vec3(1.0, 2.0, 3.0)]);
    assert_eq!(vec![glam::vec4(1.0, 2.0, 3.0, 4.0)], data3.to_vec4_with_w(4.0));

    let data4 = VectorData::Vector4(vec![glam::vec4(1.0, 2.0, 3.0, 5.0)]);
    assert_eq!(vec![glam::vec4(1.0, 2.0, 3.0, 4.0)], data4.to_vec4_with_w(4.0));
    ```
     */
    pub fn to_vec4_with_w(&self, w: f32) -> Vec<Vec4> {
        // Allow conversion to homogeneous coordinates by specifying the w component.
        match self {
            VectorData::Vector2(data) => data.iter().map(|v| v.extend(0.0).extend(w)).collect(),
            VectorData::Vector3(data) => data.iter().map(|v| v.extend(w)).collect(),
            VectorData::Vector4(data) => data.iter().map(|v| vec4(v.x, v.y, v.z, w)).collect(),
        }
    }

    pub(crate) fn to_glam_vec2(&self) -> Vec<Vec2> {
        match self {
            VectorData::Vector2(data) => data.clone(),
            VectorData::Vector3(data) => data.iter().map(|v| v.xy()).collect(),
            VectorData::Vector4(data) => data.iter().map(|v| v.xy()).collect(),
        }
    }

    pub(crate) fn to_vec3(&self) -> Vec<Vec3> {
        match self {
            VectorData::Vector2(data) => data.iter().map(|v| v.extend(0.0)).collect(),
            VectorData::Vector3(data) => data.clone(),
            VectorData::Vector4(data) => data.iter().map(|v| v.xyz()).collect(),
        }
    }

    pub(crate) fn read<R: Read + Seek>(
        reader: &mut R,
        count: usize,
        offset: u64,
        stride: u64,
        data_type: DataType,
    ) -> BinResult<Self> {
        match data_type {
            DataType::Float2 => Ok(VectorData::Vector2(read_vector_data::<_, f32, Vec2, 2>(
                reader, count, offset, stride,
            )?)),
            DataType::Float3 => Ok(VectorData::Vector3(read_vector_data::<_, f32, Vec3, 3>(
                reader, count, offset, stride,
            )?)),
            DataType::Float4 => Ok(VectorData::Vector4(read_vector_data::<_, f32, Vec4, 4>(
                reader, count, offset, stride,
            )?)),
            DataType::HalfFloat2 => Ok(VectorData::Vector2(read_vector_data::<_, Half, Vec2, 2>(
                reader, count, offset, stride,
            )?)),
            DataType::HalfFloat4 => Ok(VectorData::Vector4(read_vector_data::<_, Half, Vec4, 4>(
                reader, count, offset, stride,
            )?)),
            DataType::Byte4 => {
                let mut elements =
                    read_vector_data::<_, u8, Vec4, 4>(reader, count, offset, stride)?;
                // Normalize the values by converting from the range [0u8, 255u8] to [0.0f32, 1.0f32].
                for v in elements.iter_mut() {
                    *v /= 255f32;
                }
                Ok(VectorData::Vector4(elements))
            }
        }
    }
}

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
    pub fn data_type(&self) -> AttributeDataTypeV10 {
        match self {
            VectorDataV10::Float2(_) => AttributeDataTypeV10::Float2,
            VectorDataV10::Float3(_) => AttributeDataTypeV10::Float3,
            VectorDataV10::Float4(_) => AttributeDataTypeV10::Float4,
            VectorDataV10::HalfFloat4(_) => AttributeDataTypeV10::HalfFloat4,
            VectorDataV10::Byte4(_) => AttributeDataTypeV10::Byte4,
            VectorDataV10::HalfFloat2(_) => AttributeDataTypeV10::HalfFloat2,
        }
    }

    pub fn write<W: Write + Seek>(
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

    pub fn from_positions(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => {
                VectorDataV10::Float2(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector3(v) => {
                VectorDataV10::Float3(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector4(v) => {
                VectorDataV10::Float4(v.iter().map(|v| v.to_array()).collect())
            }
        }
    }

    pub fn from_vectors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => {
                VectorDataV10::HalfFloat2(v.iter().map(|v| f16_vector(v.to_array())).collect())
            }
            VectorData::Vector3(v) => {
                VectorDataV10::Float3(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector4(v) => {
                VectorDataV10::HalfFloat4(v.iter().map(|v| f16_vector(v.to_array())).collect())
            }
        }
    }

    pub fn from_colors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => {
                VectorDataV10::HalfFloat2(v.iter().map(|v| f16_vector(v.to_array())).collect())
            }
            VectorData::Vector3(v) => {
                VectorDataV10::Float3(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector4(v) => {
                VectorDataV10::Byte4(v.iter().map(|v| clamped_u8_vector(v.to_array())).collect())
            }
        }
    }
}

impl VectorDataV8 {
    pub fn data_type(&self) -> AttributeDataTypeV8 {
        match self {
            VectorDataV8::Float2(_) => AttributeDataTypeV8::Float2,
            VectorDataV8::Float3(_) => AttributeDataTypeV8::Float3,
            VectorDataV8::Float4(_) => AttributeDataTypeV8::Float4,
            VectorDataV8::HalfFloat4(_) => AttributeDataTypeV8::HalfFloat4,
            VectorDataV8::Byte4(_) => AttributeDataTypeV8::Byte4,
        }
    }

    pub fn write<W: Write + Seek>(
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

    pub fn from_positions(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => {
                VectorDataV8::Float2(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector3(v) => {
                VectorDataV8::Float3(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector4(v) => {
                VectorDataV8::Float4(v.iter().map(|v| v.to_array()).collect())
            }
        }
    }

    pub fn from_vectors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => {
                VectorDataV8::Float2(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector3(v) => {
                VectorDataV8::Float3(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector4(v) => {
                VectorDataV8::HalfFloat4(v.iter().map(|v| f16_vector(v.to_array())).collect())
            }
        }
    }

    pub fn from_colors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => {
                VectorDataV8::Float2(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector3(v) => {
                VectorDataV8::Float3(v.iter().map(|v| v.to_array()).collect())
            }
            VectorData::Vector4(v) => {
                VectorDataV8::Byte4(v.iter().map(|v| clamped_u8_vector(v.to_array())).collect())
            }
        }
    }
}

fn f16_vector<const N: usize>(vector: [f32; N]) -> [f16; N] {
    vector.map(f16::from_f32)
}

fn clamped_u8_vector<const N: usize>(vector: [f32; N]) -> [u8; N] {
    vector.map(u8_clamped)
}

fn read_vector_data<R, T, U, const N: usize>(
    reader: &mut R,
    count: usize,
    offset: u64,
    stride: u64, // TODO: NonZero<u64>
) -> BinResult<Vec<U>>
where
    R: Read + Seek,
    T: Into<f32> + for<'a> BinRead<Args<'a> = ()>,
    U: From<[f32; N]>,
{
    // It's possible that both count and stride are 0 to specify no data.
    // Return an error in the case where stride is 0 and count is arbitrarily large.
    // This prevents reading the same element repeatedly and likely crashing.
    if count > 0 && stride == 0 {
        // TODO: Create a better error type?
        return BinResult::Err(binrw::error::Error::Custom {
            pos: offset,
            err: Box::new("Invalid zero stride detected."),
        });
    }

    let mut result = Vec::new();
    for i in 0..count as u64 {
        // The data type may be smaller than stride to allow interleaving different attributes.
        reader.seek(SeekFrom::Start(offset + i * stride))?;

        let array: [f32; N] = reader.read_le::<[T; N]>()?.map(Into::into);
        result.push(array.into());
    }
    Ok(result)
}

fn u8_clamped(f: f32) -> u8 {
    f.clamp(0.0f32, 1.0f32).mul(255.0f32).round() as u8
}

fn write_f32<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

fn write_u8<W: Write>(writer: &mut W, data: &[u8]) -> std::io::Result<()> {
    writer.write_all(data)
}

fn write_f16<W: Write>(writer: &mut W, data: &[f16]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

fn write_vector_data<
    T,
    W: Write + Seek,
    F: Fn(&mut W, &[T]) -> std::io::Result<()>,
    const N: usize,
>(
    writer: &mut W,
    elements: &[[T; N]],
    offset: u64,
    stride: u64,
    write_t: F,
) -> Result<(), std::io::Error> {
    // TODO: Support a stride of 0?
    // Don't zero pad the last element to stride.
    for (i, element) in elements.iter().enumerate() {
        writer.seek(SeekFrom::Start(offset + i as u64 * stride))?;
        write_t(writer, element)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::io::Cursor;
    use glam::vec2;
    use hexlit::hex;

    // TODO: Test conversions for versioned vector data.

    #[test]
    fn read_vector_data_count0() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = VectorData::read(&mut reader, 0, 0, 0, DataType::Byte4).unwrap();
        assert_eq!(VectorData::Vector4(Vec::new()), values);
    }

    #[test]
    fn read_vector_data_count1() {
        let mut reader = Cursor::new(hex!("004080FF"));
        let values = VectorData::read(&mut reader, 1, 0, 4, DataType::Byte4).unwrap();
        // https://registry.khronos.org/vulkan/specs/1.3/html/chap3.html#fundamentals-fixedfpconv
        assert_eq!(
            VectorData::Vector4(vec![vec4(
                0.0 / 255.0,
                64.0 / 255.0,
                128.0 / 255.0,
                255.0 / 255.0
            )]),
            values
        );
    }

    #[test]
    fn read_vector_data_zero_stride() {
        // This should return an error and not attempt to read the specified number of elements.
        // This prevents a potential panic from a failed allocation.
        let mut reader = Cursor::new(hex!("01020304"));
        let result = VectorData::read(&mut reader, usize::MAX, 0, 0, DataType::Byte4);
        assert!(result.is_err());
    }

    #[test]
    fn read_vector_data_count_exceeds_buffer() {
        // This should return an error and not attempt to read the specified number of elements.
        // This prevents a potential panic from a failed allocation.
        let mut reader = Cursor::new(hex!("01020304"));
        let result = VectorData::read(&mut reader, usize::MAX, 0, 1, DataType::Byte4);
        assert!(result.is_err());
    }

    #[test]
    fn read_vector_data_stride_equals_size() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, Vec2, 2>(&mut reader, 3, 0, 2).unwrap();
        assert_eq!(vec![vec2(0.0, 1.0), vec2(2.0, 3.0), vec2(4.0, 5.0)], values);
    }

    #[test]
    fn read_vector_data_stride_equals_size_offset() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, Vec2, 2>(&mut reader, 3, 2, 2).unwrap();
        assert_eq!(vec![vec2(2.0, 3.0), vec2(4.0, 5.0), vec2(6.0, 7.0)], values);
    }

    #[test]
    fn read_vector_data_stride_exceeds_size() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, Vec2, 2>(&mut reader, 2, 0, 4).unwrap();
        assert_eq!(vec![vec2(0.0, 1.0), vec2(4.0, 5.0)], values);
    }

    #[test]
    fn read_vector_data_stride_exceeds_size_offset() {
        // offset + (stride * count) points past the buffer,
        // but we only read 2 bytes from the last block of size stride = 4
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, Vec2, 2>(&mut reader, 2, 2, 4).unwrap();
        assert_eq!(vec![vec2(2.0, 3.0), vec2(6.0, 7.0)], values);
    }

    #[test]
    fn write_vector_data_count0() {
        let mut writer = Cursor::new(Vec::new());
        write_vector_data::<f32, _, _, 1>(&mut writer, &[], 0, 4, write_f32).unwrap();
        assert!(writer.get_ref().is_empty());
    }

    #[test]
    fn write_vector_data_count1() {
        let mut writer = Cursor::new(Vec::new());
        write_vector_data(&mut writer, &[[1f32, 2f32]], 0, 8, write_f32).unwrap();
        assert_eq!(*writer.get_ref(), hex!("0000803F 00000040"),);
    }

    #[test]
    fn write_vector_stride_offset() {
        let mut writer = Cursor::new(Vec::new());
        write_vector_data(
            &mut writer,
            &[[1f32, 2f32, 3f32], [1f32, 0f32, 0f32]],
            4,
            16,
            write_f32,
        )
        .unwrap();

        // The last 4 bytes of padding from stride should be missing.
        // This matches the behavior of read_vector_data.
        assert_eq!(
            *writer.get_ref(),
            hex!(
                "00000000 
                 0000803F 00000040 00004040 00000000 
                 0000803F 00000000 00000000"
            )
        );
    }

    #[test]
    fn f32_to_u8_clamped() {
        assert_eq!(0u8, u8_clamped(-1.0f32));

        for u in 0..=255u8 {
            assert_eq!(u, u8_clamped(u as f32 / 255.0f32));
        }

        assert_eq!(255u8, u8_clamped(2.0f32));
    }
}
