use binrw::io::{Read, Write};
use binrw::io::{Seek, SeekFrom};
use binrw::BinReaderExt;
use binrw::{BinRead, BinResult};
use half::f16;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_lib::formats::mesh::{AttributeDataTypeV10, AttributeDataTypeV8};
use std::ops::Mul;

use super::{DataType, Half};

/// The data for a vertex attribute.
///
/// The precision when saving is inferred based on supported data types for the version specified in the [MeshData].
/// For example, position attributes will prefer the highest available precision, and color sets will prefer the lowest available precision.
/// *The data type selected for saving may change between releases but will always retain the specified component count such as [VectorData::Vector2] vs [VectorData::Vector4].*
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorData {
    Vector2(Vec<[f32; 2]>),
    Vector3(Vec<[f32; 3]>),
    Vector4(Vec<[f32; 4]>),
}

impl VectorData {
    /// The number of vectors.
    /**
    ```rust
    # use ssbh_data::mesh_data::VectorData;
    let data = VectorData::Vector2(vec![[0f32, 1f32], [0f32, 1f32], [0f32, 1f32]]);
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
    let data2 = VectorData::Vector2(vec![[1.0, 2.0]]);
    assert_eq!(vec![[1.0, 2.0, 0.0, 4.0]], data2.to_vec4_with_w(4.0));

    let data3 = VectorData::Vector3(vec![[1.0, 2.0, 3.0]]);
    assert_eq!(vec![[1.0, 2.0, 3.0, 4.0]], data3.to_vec4_with_w(4.0));

    let data4 = VectorData::Vector4(vec![[1.0, 2.0, 3.0, 5.0]]);
    assert_eq!(vec![[1.0, 2.0, 3.0, 4.0]], data4.to_vec4_with_w(4.0));
    ```
     */
    pub fn to_vec4_with_w(&self, w: f32) -> Vec<[f32; 4]> {
        // Allow conversion to homogeneous coordinates by specifying the w component.
        match self {
            VectorData::Vector2(data) => data.iter().map(|[x, y]| [*x, *y, 0f32, w]).collect(),
            VectorData::Vector3(data) => data.iter().map(|[x, y, z]| [*x, *y, *z, w]).collect(),
            VectorData::Vector4(data) => data.iter().map(|[x, y, z, _]| [*x, *y, *z, w]).collect(),
        }
    }

    pub(crate) fn to_glam_vec2(&self) -> Vec<geometry_tools::glam::Vec2> {
        match self {
            VectorData::Vector2(data) => data
                .iter()
                .map(|[x, y]| geometry_tools::glam::Vec2::new(*x, *y))
                .collect(),
            VectorData::Vector3(data) => data
                .iter()
                .map(|[x, y, _]| geometry_tools::glam::Vec2::new(*x, *y))
                .collect(),
            VectorData::Vector4(data) => data
                .iter()
                .map(|[x, y, _, _]| geometry_tools::glam::Vec2::new(*x, *y))
                .collect(),
        }
    }

    pub(crate) fn to_glam_vec3a(&self) -> Vec<geometry_tools::glam::Vec3A> {
        match self {
            VectorData::Vector2(data) => data
                .iter()
                .map(|[x, y]| geometry_tools::glam::Vec3A::new(*x, *y, 0f32))
                .collect(),
            VectorData::Vector3(data) => data
                .iter()
                .map(|[x, y, z]| geometry_tools::glam::Vec3A::new(*x, *y, *z))
                .collect(),
            VectorData::Vector4(data) => data
                .iter()
                .map(|[x, y, z, _]| geometry_tools::glam::Vec3A::new(*x, *y, *z))
                .collect(),
        }
    }

    pub(crate) fn to_glam_vec4_with_w(&self, w: f32) -> Vec<geometry_tools::glam::Vec4> {
        // Allow conversion to homogeneous coordinates by specifying the w component.
        match self {
            VectorData::Vector2(data) => data
                .iter()
                .map(|[x, y]| geometry_tools::glam::Vec4::new(*x, *y, 0f32, w))
                .collect(),
            VectorData::Vector3(data) => data
                .iter()
                .map(|[x, y, z]| geometry_tools::glam::Vec4::new(*x, *y, *z, w))
                .collect(),
            VectorData::Vector4(data) => data
                .iter()
                .map(|[x, y, z, _]| geometry_tools::glam::Vec4::new(*x, *y, *z, w))
                .collect(),
        }
    }

    pub(crate) fn read<R: Read + Seek>(
        reader: &mut R,
        count: usize,
        offset: u64,
        stride: u64,
        data_type: &DataType,
    ) -> BinResult<Self> {
        match data_type {
            DataType::Float2 => Ok(VectorData::Vector2(read_vector_data::<_, f32, 2>(
                reader, count, offset, stride,
            )?)),
            DataType::Float3 => Ok(VectorData::Vector3(read_vector_data::<_, f32, 3>(
                reader, count, offset, stride,
            )?)),
            DataType::Float4 => Ok(VectorData::Vector4(read_vector_data::<_, f32, 4>(
                reader, count, offset, stride,
            )?)),
            DataType::HalfFloat2 => Ok(VectorData::Vector2(read_vector_data::<_, Half, 2>(
                reader, count, offset, stride,
            )?)),
            DataType::HalfFloat4 => Ok(VectorData::Vector4(read_vector_data::<_, Half, 4>(
                reader, count, offset, stride,
            )?)),
            DataType::Byte4 => {
                let mut elements = read_vector_data::<_, u8, 4>(reader, count, offset, stride)?;
                // Normalize the values by converting from the range [0u8, 255u8] to [0.0f32, 1.0f32].
                for [x, y, z, w] in elements.iter_mut() {
                    *x /= 255f32;
                    *y /= 255f32;
                    *z /= 255f32;
                    *w /= 255f32;
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
            VectorData::Vector2(v) => VectorDataV10::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::Float4(v.clone()),
        }
    }

    pub fn from_vectors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::HalfFloat2(get_f16_vectors(v)),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::HalfFloat4(get_f16_vectors(v)),
        }
    }

    pub fn from_colors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV10::HalfFloat2(get_f16_vectors(v)),
            VectorData::Vector3(v) => VectorDataV10::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV10::Byte4(get_clamped_u8_vectors(v)),
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
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::Float4(v.clone()),
        }
    }

    pub fn from_vectors(data: &VectorData) -> Self {
        match data {
            VectorData::Vector2(v) => VectorDataV8::Float2(v.clone()),
            VectorData::Vector3(v) => VectorDataV8::Float3(v.clone()),
            VectorData::Vector4(v) => VectorDataV8::HalfFloat4(get_f16_vectors(v)),
        }
    }

    pub fn from_colors(data: &VectorData) -> Self {
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

// TODO: Move this to just be for reading indices?
// TODO: Just use binrw for this?
pub fn read_data<R: Read + Seek, TIn: BinRead<Args = ()>, TOut: From<TIn>>(
    reader: &mut R,
    count: usize,
    offset: u64,
) -> BinResult<Vec<TOut>> {
    let mut result = Vec::new();
    reader.seek(SeekFrom::Start(offset))?;
    for _ in 0..count as u64 {
        result.push(reader.read_le::<TIn>()?.into());
    }
    Ok(result)
}

// TODO: Make these private
pub fn read_vector_data<R: Read + Seek, T: Into<f32> + BinRead<Args = ()>, const N: usize>(
    reader: &mut R,
    count: usize,
    offset: u64,
    stride: u64, // TODO: NonZero<u64>
) -> BinResult<Vec<[f32; N]>> {
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

        let mut element = [0f32; N];
        for e in element.iter_mut() {
            *e = reader.read_le::<T>()?.into();
        }
        result.push(element);
    }
    Ok(result)
}

pub fn get_u8_clamped(f: f32) -> u8 {
    f.clamp(0.0f32, 1.0f32).mul(255.0f32).round() as u8
}

pub fn write_f32<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

pub fn write_u8<W: Write>(writer: &mut W, data: &[u8]) -> std::io::Result<()> {
    writer.write_all(data)
}

pub fn write_f16<W: Write>(writer: &mut W, data: &[f16]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}

pub fn write_vector_data<
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
    use hexlit::hex;

    #[test]
    fn read_data_count0() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, u16>(&mut reader, 0, 0).unwrap();
        assert_eq!(Vec::<u16>::new(), values);
    }

    #[test]
    fn read_data_count4() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, u32>(&mut reader, 4, 0).unwrap();
        assert_eq!(vec![1u32, 2u32, 3u32, 4u32], values);
    }

    #[test]
    fn read_data_offset() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_data::<_, u8, f32>(&mut reader, 2, 1).unwrap();
        assert_eq!(vec![2f32, 3f32], values);
    }

    #[test]
    fn read_vector_data_count0() {
        let mut reader = Cursor::new(hex!("01020304"));
        let values = read_vector_data::<_, u8, 4>(&mut reader, 0, 0, 0).unwrap();
        assert_eq!(Vec::<[f32; 4]>::new(), values);
    }

    #[test]
    fn read_vector_data_count1() {
        let mut reader = Cursor::new(hex!("00010203"));
        let values = read_vector_data::<_, u8, 4>(&mut reader, 1, 0, 4).unwrap();
        assert_eq!(vec![[0.0f32, 1.0f32, 2.0f32, 3.0f32]], values);
    }

    #[test]
    fn read_vector_data_zero_stride() {
        // This should return an error and not attempt to read the specified number of elements.
        // This prevents a potential panic from a failed allocation.
        let mut reader = Cursor::new(hex!("01020304"));
        let result = read_vector_data::<_, u8, 2>(&mut reader, usize::MAX, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn read_vector_data_count_exceeds_buffer() {
        // This should return an error and not attempt to read the specified number of elements.
        // This prevents a potential panic from a failed allocation.
        let mut reader = Cursor::new(hex!("01020304"));
        let result = read_vector_data::<_, u8, 2>(&mut reader, usize::MAX, 0, 1);
        assert!(result.is_err());
    }

    #[test]
    fn read_vector_data_stride_equals_size() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 3, 0, 2).unwrap();
        assert_eq!(
            vec![[0.0f32, 1.0f32], [2.0f32, 3.0f32], [4.0f32, 5.0f32]],
            values
        );
    }

    #[test]
    fn read_vector_data_stride_equals_size_offset() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 3, 2, 2).unwrap();
        assert_eq!(
            vec![[2.0f32, 3.0f32], [4.0f32, 5.0f32], [6.0f32, 7.0f32],],
            values
        );
    }

    #[test]
    fn read_vector_data_stride_exceeds_size() {
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 2, 0, 4).unwrap();
        assert_eq!(vec![[0.0f32, 1.0f32], [4.0f32, 5.0f32]], values);
    }

    #[test]
    fn read_vector_data_stride_exceeds_size_offset() {
        // offset + (stride * count) points past the buffer,
        // but we only read 2 bytes from the last block of size stride = 4
        let mut reader = Cursor::new(hex!("00010203 04050607"));
        let values = read_vector_data::<_, u8, 2>(&mut reader, 2, 2, 4).unwrap();
        assert_eq!(vec![[2.0f32, 3.0f32], [6.0f32, 7.0f32]], values);
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
    fn u8_clamped() {
        assert_eq!(0u8, get_u8_clamped(-1.0f32));

        for u in 0..=255u8 {
            assert_eq!(u, get_u8_clamped(u as f32 / 255.0f32));
        }

        assert_eq!(255u8, get_u8_clamped(2.0f32));
    }
}
