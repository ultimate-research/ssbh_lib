#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2018::*;
#[macro_use]
extern crate std;
pub mod mesh_data {
    use binread::{io::Cursor, BinRead};
    use binread::{BinReaderExt, BinResult};
    use half::f16;
    use itertools::Itertools;
    use ssbh_lib::formats::mesh::{MeshAttributeV9, RiggingType};
    use ssbh_lib::{
        formats::mesh::{
            AttributeDataTypeV10, AttributeDataTypeV8, AttributeUsageV8, AttributeUsageV9,
            DrawElementType, Mesh, MeshAttributeV10, MeshAttributeV8, MeshAttributes,
            MeshBoneBuffer, MeshObject, MeshRiggingGroup, RiggingFlags, VertexWeightV8,
            VertexWeights,
        },
        SsbhByteBuffer,
    };
    use ssbh_lib::{Half, Matrix3x3, SsbhArray, Vector3};
    use std::collections::{HashMap, HashSet};
    use std::convert::TryFrom;
    use std::io::{Read, Seek};
    use std::ops::{Add, Div, Sub};
    use std::path::Path;
    use std::{error::Error, io::Write};
    use crate::{read_data, read_vector_data, write_f16, write_f32, write_u8, write_vector_data};
    pub enum DataType {
        Float2,
        Float3,
        Float4,
        HalfFloat2,
        HalfFloat4,
        Byte4,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for DataType {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&DataType::Float2,) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Float2");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&DataType::Float3,) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Float3");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&DataType::Float4,) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Float4");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&DataType::HalfFloat2,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "HalfFloat2");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&DataType::HalfFloat4,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "HalfFloat4");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&DataType::Byte4,) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Byte4");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for DataType {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for DataType {
        #[inline]
        fn eq(&self, other: &DataType) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => true,
                    }
                } else {
                    false
                }
            }
        }
    }
    pub enum AttributeUsage {
        Position,
        Normal,
        Binormal,
        Tangent,
        TextureCoordinate,
        ColorSet,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for AttributeUsage {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&AttributeUsage::Position,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Position");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&AttributeUsage::Normal,) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Normal");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&AttributeUsage::Binormal,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Binormal");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&AttributeUsage::Tangent,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Tangent");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&AttributeUsage::TextureCoordinate,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "TextureCoordinate");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&AttributeUsage::ColorSet,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "ColorSet");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for AttributeUsage {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for AttributeUsage {
        #[inline]
        fn eq(&self, other: &AttributeUsage) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => true,
                    }
                } else {
                    false
                }
            }
        }
    }
    /// Errors while creating a [Mesh] from [MeshObjectData].
    pub enum MeshError {
        /// The attributes for a [MeshObject] would have different number of elements,
        /// so the vertex count cannot be determined.
        AttributeDataLengthMismatch,
        /// Creating a [Mesh] file that for the given version is not supported.
        UnsupportedMeshVersion {
            major_version: u16,
            minor_version: u16,
        },
        /// An error occurred while writing data to a buffer.
        Io(std::io::Error),
    }
    impl std::error::Error for MeshError {}
    impl From<std::io::Error> for MeshError {
        fn from(e: std::io::Error) -> Self {
            Self::Io(e)
        }
    }
    impl std::fmt::Display for MeshError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(self, f)
        }
    }
    impl std::fmt::Debug for MeshError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self { MeshError :: AttributeDataLengthMismatch => f . write_fmt (:: core :: fmt :: Arguments :: new_v1 (& ["Attribute data lengths do not match. Failed to determined vertex count."] , & match () { () => [] , })) , MeshError :: UnsupportedMeshVersion { major_version , minor_version } => f . write_fmt (:: core :: fmt :: Arguments :: new_v1 (& ["Creating a version " , "." , " mesh is not supported."] , & match (& major_version , & minor_version) { (arg0 , arg1) => [:: core :: fmt :: ArgumentV1 :: new (arg0 , :: core :: fmt :: Display :: fmt) , :: core :: fmt :: ArgumentV1 :: new (arg1 , :: core :: fmt :: Display :: fmt)] , })) , MeshError :: Io (err) => f . write_fmt (:: core :: fmt :: Arguments :: new_v1 (& ["IO Error: "] , & match (& err ,) { (arg0 ,) => [:: core :: fmt :: ArgumentV1 :: new (arg0 , :: core :: fmt :: Debug :: fmt)] , })) , }
        }
    }
    /// Errors while reading mesh attribute data.
    pub enum AttributeError {
        /// Attempted to read from a nonexistent buffer.
        InvalidBufferIndex(u64),
        /// Failed to find the offset or stride in bytes for the given buffer index.
        NoOffsetOrStride(u64),
        /// An error occurred while reading the data from the buffer.
        Io(std::io::Error),
        /// An error occurred while reading the data from the buffer.
        BinRead(binread::error::Error),
    }
    impl std::error::Error for AttributeError {}
    impl From<std::io::Error> for AttributeError {
        fn from(e: std::io::Error) -> Self {
            Self::Io(e)
        }
    }
    impl From<binread::error::Error> for AttributeError {
        fn from(e: binread::error::Error) -> Self {
            match e {
                binread::Error::Io(io) => Self::Io(io),
                _ => Self::BinRead(e),
            }
        }
    }
    impl std::fmt::Display for AttributeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(self, f)
        }
    }
    impl std::fmt::Debug for AttributeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AttributeError::InvalidBufferIndex(index) => {
                    f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["No buffer found for index ", "."],
                        &match (&index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ))
                }
                AttributeError::NoOffsetOrStride(index) => {
                    f.write_fmt(::core::fmt::Arguments::new_v1(
                        &[
                            "Found index ",
                            ". Buffer indices higher than 1 are not supported.",
                        ],
                        &match (&index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ))
                }
                AttributeError::Io(err) => f.write_fmt(::core::fmt::Arguments::new_v1(
                    &["IO Error: "],
                    &match (&err,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)],
                    },
                )),
                AttributeError::BinRead(err) => f.write_fmt(::core::fmt::Arguments::new_v1(
                    &["BinRead Error: "],
                    &match (&err,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)],
                    },
                )),
            }
        }
    }
    impl From<AttributeUsageV9> for AttributeUsage {
        fn from(a: AttributeUsageV9) -> Self {
            match a {
                AttributeUsageV9::Position => Self::Position,
                AttributeUsageV9::Normal => Self::Normal,
                AttributeUsageV9::Binormal => Self::Binormal,
                AttributeUsageV9::Tangent => Self::Tangent,
                AttributeUsageV9::TextureCoordinate => Self::TextureCoordinate,
                AttributeUsageV9::ColorSet => Self::ColorSet,
            }
        }
    }
    impl From<AttributeUsageV8> for AttributeUsage {
        fn from(a: AttributeUsageV8) -> Self {
            match a {
                AttributeUsageV8::Position => Self::Position,
                AttributeUsageV8::Normal => Self::Normal,
                AttributeUsageV8::Tangent => Self::Tangent,
                AttributeUsageV8::TextureCoordinate => Self::TextureCoordinate,
                AttributeUsageV8::ColorSet => Self::ColorSet,
            }
        }
    }
    pub struct VertexWeight {
        pub vertex_index: u32,
        pub vertex_weight: f32,
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for VertexWeight {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_vertex_index = ();
                let __binread_generated_options_vertex_index = __binread_generated_var_options;
                let mut vertex_index: u32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_vertex_index,
                    __binread_generated_args_vertex_index.clone(),
                )?;
                let __binread_generated_args_vertex_weight = ();
                let __binread_generated_options_vertex_weight = __binread_generated_var_options;
                let mut vertex_weight: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_vertex_weight,
                    __binread_generated_args_vertex_weight.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut vertex_index,
                    __binread_generated_var_reader,
                    __binread_generated_options_vertex_index,
                    __binread_generated_args_vertex_index.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut vertex_weight,
                    __binread_generated_var_reader,
                    __binread_generated_options_vertex_weight,
                    __binread_generated_args_vertex_weight.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    vertex_index,
                    vertex_weight,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for VertexWeight {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                VertexWeight {
                    vertex_index: ref __self_0_0,
                    vertex_weight: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "VertexWeight");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vertex_index",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vertex_weight",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for VertexWeight {
        #[inline]
        fn clone(&self) -> VertexWeight {
            match *self {
                VertexWeight {
                    vertex_index: ref __self_0_0,
                    vertex_weight: ref __self_0_1,
                } => VertexWeight {
                    vertex_index: ::core::clone::Clone::clone(&(*__self_0_0)),
                    vertex_weight: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    impl From<AttributeDataTypeV10> for DataType {
        fn from(value: AttributeDataTypeV10) -> Self {
            match value {
                AttributeDataTypeV10::Float3 => Self::Float3,
                AttributeDataTypeV10::Byte4 => Self::Byte4,
                AttributeDataTypeV10::HalfFloat4 => Self::HalfFloat4,
                AttributeDataTypeV10::HalfFloat2 => Self::HalfFloat2,
                AttributeDataTypeV10::Float4 => Self::Float4,
                AttributeDataTypeV10::Float2 => Self::Float2,
            }
        }
    }
    impl From<AttributeDataTypeV8> for DataType {
        fn from(value: AttributeDataTypeV8) -> Self {
            match value {
                AttributeDataTypeV8::Float3 => Self::Float3,
                AttributeDataTypeV8::Float2 => Self::Float2,
                AttributeDataTypeV8::Byte4 => Self::Byte4,
                AttributeDataTypeV8::HalfFloat4 => Self::HalfFloat4,
                AttributeDataTypeV8::Float4 => Self::Float4,
            }
        }
    }
    fn read_vertex_indices(
        mesh_index_buffer: &[u8],
        mesh_object: &MeshObject,
    ) -> BinResult<Vec<u32>> {
        let count = mesh_object.vertex_index_count as usize;
        let offset = mesh_object.index_buffer_offset as u64;
        let mut reader = Cursor::new(mesh_index_buffer);
        match mesh_object.draw_element_type {
            DrawElementType::UnsignedShort => read_data::<_, u16, u32>(&mut reader, count, offset),
            DrawElementType::UnsignedInt => read_data::<_, u32, u32>(&mut reader, count, offset),
        }
    }
    fn read_attribute_data<T>(
        mesh: &Mesh,
        mesh_object: &MeshObject,
        attribute: &MeshAttribute,
    ) -> Result<VectorData, AttributeError> {
        let attribute_buffer = mesh
            .vertex_buffers
            .elements
            .get(attribute.index as usize)
            .ok_or(AttributeError::InvalidBufferIndex(attribute.index))?;
        let (offset, stride) = calculate_offset_stride(attribute, mesh_object)?;
        let count = mesh_object.vertex_count as usize;
        let mut reader = Cursor::new(&attribute_buffer.elements);
        let data = match attribute.data_type {
            DataType::Float2 => VectorData::Vector2(read_vector_data::<_, f32, 2>(
                &mut reader,
                count,
                offset,
                stride,
            )?),
            DataType::Float3 => VectorData::Vector3(read_vector_data::<_, f32, 3>(
                &mut reader,
                count,
                offset,
                stride,
            )?),
            DataType::Float4 => VectorData::Vector4(read_vector_data::<_, f32, 4>(
                &mut reader,
                count,
                offset,
                stride,
            )?),
            DataType::HalfFloat2 => VectorData::Vector2(read_vector_data::<_, Half, 2>(
                &mut reader,
                count,
                offset,
                stride,
            )?),
            DataType::HalfFloat4 => VectorData::Vector4(read_vector_data::<_, Half, 4>(
                &mut reader,
                count,
                offset,
                stride,
            )?),
            DataType::Byte4 => {
                let mut elements =
                    read_vector_data::<_, u8, 4>(&mut reader, count, offset, stride)?;
                for [x, y, z, w] in elements.iter_mut() {
                    *x /= 255f32;
                    *y /= 255f32;
                    *z /= 255f32;
                    *w /= 255f32;
                }
                VectorData::Vector4(elements)
            }
        };
        Ok(data)
    }
    fn calculate_offset_stride(
        attribute: &MeshAttribute,
        mesh_object: &MeshObject,
    ) -> Result<(u64, u64), AttributeError> {
        let (offset, stride) = match attribute.index {
            0 => Ok((
                attribute.offset + mesh_object.vertex_buffer0_offset as u64,
                mesh_object.stride0 as u64,
            )),
            1 => Ok((
                attribute.offset + mesh_object.vertex_buffer1_offset as u64,
                mesh_object.stride1 as u64,
            )),
            _ => Err(AttributeError::NoOffsetOrStride(attribute.index)),
        }?;
        Ok((offset, stride))
    }
    /// Read data for all attributes of the given `usage` for `mesh_object`.
    pub fn read_attributes_by_usage(
        mesh: &Mesh,
        mesh_object: &MeshObject,
        usage: AttributeUsage,
    ) -> Result<Vec<AttributeData>, Box<dyn Error>> {
        let mut attributes = Vec::new();
        for attribute in &get_attributes(&mesh_object, usage) {
            let data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
            attributes.push(AttributeData {
                name: attribute.name.to_string(),
                data,
            })
        }
        Ok(attributes)
    }
    /// Returns all the texture coordinate attributes for the specified `mesh_object`.
    /// The v coordinate is transformed to `1.0 - v` if `flip_vertical` is true.
    pub fn read_texture_coordinates(
        mesh: &Mesh,
        mesh_object: &MeshObject,
        flip_vertical: bool,
    ) -> Result<Vec<AttributeData>, Box<dyn Error>> {
        let mut attributes = Vec::new();
        for attribute in &get_attributes(&mesh_object, AttributeUsage::TextureCoordinate) {
            let mut data = read_attribute_data::<f32>(mesh, mesh_object, attribute)?;
            if flip_vertical {
                flip_y(&mut data);
            }
            attributes.push(AttributeData {
                name: attribute.name.to_string(),
                data,
            });
        }
        Ok(attributes)
    }
    fn flip_y(data: &mut VectorData) {
        match data {
            VectorData::Vector2(v) => {
                for [_, y] in v.iter_mut() {
                    *y = 1.0 - *y;
                }
            }
            VectorData::Vector3(v) => {
                for [_, y, _] in v.iter_mut() {
                    *y = 1.0 - *y;
                }
            }
            VectorData::Vector4(v) => {
                for [_, y, _, _] in v.iter_mut() {
                    *y = 1.0 - *y;
                }
            }
        }
    }
    /// Returns all the colorset attributes for the specified `mesh_object`.
    /// [u8] values are converted to [f32] by normalizing to the range 0.0 to 1.0.
    pub fn read_colorsets(
        mesh: &Mesh,
        mesh_object: &MeshObject,
    ) -> Result<Vec<AttributeData>, Box<dyn Error>> {
        read_attributes_by_usage(mesh, mesh_object, AttributeUsage::ColorSet)
    }
    fn read_rigging_data(
        rigging_buffers: &[MeshRiggingGroup],
        mesh_object_name: &str,
        mesh_object_subindex: u64,
    ) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
        let mut bone_influences = Vec::new();
        for rigging_group in rigging_buffers.iter().filter(|r| {
            r.mesh_object_name.to_str() == Some(mesh_object_name)
                && r.mesh_object_sub_index == mesh_object_subindex
        }) {
            bone_influences.extend(read_influences(&rigging_group)?);
        }
        Ok(bone_influences)
    }
    pub struct BoneInfluence {
        pub bone_name: String,
        pub vertex_weights: Vec<VertexWeight>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for BoneInfluence {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                BoneInfluence {
                    bone_name: ref __self_0_0,
                    vertex_weights: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "BoneInfluence");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "bone_name",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vertex_weights",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for BoneInfluence {
        #[inline]
        fn clone(&self) -> BoneInfluence {
            match *self {
                BoneInfluence {
                    bone_name: ref __self_0_0,
                    vertex_weights: ref __self_0_1,
                } => BoneInfluence {
                    bone_name: ::core::clone::Clone::clone(&(*__self_0_0)),
                    vertex_weights: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    /// The data associated with a [Mesh] file.
    /// Supported versions are 1.8 and 1.10.
    pub struct MeshData {
        pub major_version: u16,
        pub minor_version: u16,
        pub objects: Vec<MeshObjectData>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for MeshData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                MeshData {
                    major_version: ref __self_0_0,
                    minor_version: ref __self_0_1,
                    objects: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "MeshData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "major_version",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "minor_version",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "objects",
                        &&(*__self_0_2),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for MeshData {
        #[inline]
        fn clone(&self) -> MeshData {
            match *self {
                MeshData {
                    major_version: ref __self_0_0,
                    minor_version: ref __self_0_1,
                    objects: ref __self_0_2,
                } => MeshData {
                    major_version: ::core::clone::Clone::clone(&(*__self_0_0)),
                    minor_version: ::core::clone::Clone::clone(&(*__self_0_1)),
                    objects: ::core::clone::Clone::clone(&(*__self_0_2)),
                },
            }
        }
    }
    impl MeshData {
        /// Tries to read and convert the MESH from `path`.
        /// The entire file is buffered for performance.
        pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
            let mesh = Mesh::from_file(path)?;
            Ok(MeshData {
                major_version: mesh.major_version,
                minor_version: mesh.minor_version,
                objects: read_mesh_objects(&mesh)?,
            })
        }
        /// Tries to read and convert the MESH from `reader`.
        /// For best performance when opening from a file, use `from_file` instead.
        pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
            let mesh = Mesh::read(reader)?;
            Ok(MeshData {
                major_version: mesh.major_version,
                minor_version: mesh.minor_version,
                objects: read_mesh_objects(&mesh)?,
            })
        }
        /// Converts the data to MESH and writes to the given `writer`.
        /// For best performance when writing to a file, use `write_to_file` instead.
        pub fn write<W: std::io::Write + Seek>(
            &self,
            writer: &mut W,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let mesh = create_mesh(&self)?;
            mesh.write(writer)?;
            Ok(())
        }
        /// Converts the data to MESH and writes to the given `path`.
        /// The entire file is buffered for performance.
        pub fn write_to_file<P: AsRef<Path>>(
            &self,
            path: P,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let mesh = create_mesh(&self)?;
            mesh.write_to_file(path)?;
            Ok(())
        }
    }
    pub struct MeshObjectData {
        pub name: String,
        pub sub_index: u64,
        pub parent_bone_name: String,
        pub vertex_indices: Vec<u32>,
        pub positions: Vec<AttributeData>,
        pub normals: Vec<AttributeData>,
        pub binormals: Vec<AttributeData>,
        pub tangents: Vec<AttributeData>,
        pub texture_coordinates: Vec<AttributeData>,
        pub color_sets: Vec<AttributeData>,
        /// Vertex weights grouped by bone name.
        /// Each vertex will likely be influenced by at most 4 bones, but the format doesn't enforce this.
        /// For single bound objects, [bone_influences](#structfield.bone_influences) should be an empty list.
        pub bone_influences: Vec<BoneInfluence>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for MeshObjectData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                MeshObjectData {
                    name: ref __self_0_0,
                    sub_index: ref __self_0_1,
                    parent_bone_name: ref __self_0_2,
                    vertex_indices: ref __self_0_3,
                    positions: ref __self_0_4,
                    normals: ref __self_0_5,
                    binormals: ref __self_0_6,
                    tangents: ref __self_0_7,
                    texture_coordinates: ref __self_0_8,
                    color_sets: ref __self_0_9,
                    bone_influences: ref __self_0_10,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "MeshObjectData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "name",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "sub_index",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "parent_bone_name",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vertex_indices",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "positions",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "normals",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "binormals",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "tangents",
                        &&(*__self_0_7),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "texture_coordinates",
                        &&(*__self_0_8),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "color_sets",
                        &&(*__self_0_9),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "bone_influences",
                        &&(*__self_0_10),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for MeshObjectData {
        #[inline]
        fn clone(&self) -> MeshObjectData {
            match *self {
                MeshObjectData {
                    name: ref __self_0_0,
                    sub_index: ref __self_0_1,
                    parent_bone_name: ref __self_0_2,
                    vertex_indices: ref __self_0_3,
                    positions: ref __self_0_4,
                    normals: ref __self_0_5,
                    binormals: ref __self_0_6,
                    tangents: ref __self_0_7,
                    texture_coordinates: ref __self_0_8,
                    color_sets: ref __self_0_9,
                    bone_influences: ref __self_0_10,
                } => MeshObjectData {
                    name: ::core::clone::Clone::clone(&(*__self_0_0)),
                    sub_index: ::core::clone::Clone::clone(&(*__self_0_1)),
                    parent_bone_name: ::core::clone::Clone::clone(&(*__self_0_2)),
                    vertex_indices: ::core::clone::Clone::clone(&(*__self_0_3)),
                    positions: ::core::clone::Clone::clone(&(*__self_0_4)),
                    normals: ::core::clone::Clone::clone(&(*__self_0_5)),
                    binormals: ::core::clone::Clone::clone(&(*__self_0_6)),
                    tangents: ::core::clone::Clone::clone(&(*__self_0_7)),
                    texture_coordinates: ::core::clone::Clone::clone(&(*__self_0_8)),
                    color_sets: ::core::clone::Clone::clone(&(*__self_0_9)),
                    bone_influences: ::core::clone::Clone::clone(&(*__self_0_10)),
                },
            }
        }
    }
    pub struct AttributeData {
        pub name: String,
        pub data: VectorData,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for AttributeData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                AttributeData {
                    name: ref __self_0_0,
                    data: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "AttributeData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "name",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "data",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for AttributeData {
        #[inline]
        fn clone(&self) -> AttributeData {
            match *self {
                AttributeData {
                    name: ref __self_0_0,
                    data: ref __self_0_1,
                } => AttributeData {
                    name: ::core::clone::Clone::clone(&(*__self_0_0)),
                    data: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    pub enum VectorData {
        Vector2(Vec<[f32; 2]>),
        Vector3(Vec<[f32; 3]>),
        Vector4(Vec<[f32; 4]>),
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for VectorData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&VectorData::Vector2(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Vector2");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&VectorData::Vector3(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Vector3");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&VectorData::Vector4(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Vector4");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for VectorData {
        #[inline]
        fn clone(&self) -> VectorData {
            match (&*self,) {
                (&VectorData::Vector2(ref __self_0),) => {
                    VectorData::Vector2(::core::clone::Clone::clone(&(*__self_0)))
                }
                (&VectorData::Vector3(ref __self_0),) => {
                    VectorData::Vector3(::core::clone::Clone::clone(&(*__self_0)))
                }
                (&VectorData::Vector4(ref __self_0),) => {
                    VectorData::Vector4(::core::clone::Clone::clone(&(*__self_0)))
                }
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for VectorData {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for VectorData {
        #[inline]
        fn eq(&self, other: &VectorData) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (
                            &VectorData::Vector2(ref __self_0),
                            &VectorData::Vector2(ref __arg_1_0),
                        ) => (*__self_0) == (*__arg_1_0),
                        (
                            &VectorData::Vector3(ref __self_0),
                            &VectorData::Vector3(ref __arg_1_0),
                        ) => (*__self_0) == (*__arg_1_0),
                        (
                            &VectorData::Vector4(ref __self_0),
                            &VectorData::Vector4(ref __arg_1_0),
                        ) => (*__self_0) == (*__arg_1_0),
                        _ => unsafe { ::core::intrinsics::unreachable() },
                    }
                } else {
                    false
                }
            }
        }
        #[inline]
        fn ne(&self, other: &VectorData) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (
                            &VectorData::Vector2(ref __self_0),
                            &VectorData::Vector2(ref __arg_1_0),
                        ) => (*__self_0) != (*__arg_1_0),
                        (
                            &VectorData::Vector3(ref __self_0),
                            &VectorData::Vector3(ref __arg_1_0),
                        ) => (*__self_0) != (*__arg_1_0),
                        (
                            &VectorData::Vector4(ref __self_0),
                            &VectorData::Vector4(ref __arg_1_0),
                        ) => (*__self_0) != (*__arg_1_0),
                        _ => unsafe { ::core::intrinsics::unreachable() },
                    }
                } else {
                    true
                }
            }
        }
    }
    impl VectorData {
        /// The number of vectors.
        ///
        ///    ```rust
        ///    # use ssbh_data::mesh_data::VectorData;
        ///    let data = VectorData::Vector2(vec![[0f32, 1f32], [0f32, 1f32], [0f32, 1f32]]);
        ///    assert_eq!(3, data.len());
        ///    ```
        ///    
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
        fn write<W: Write + Seek, F: Fn(&mut W, &[f32]) -> std::io::Result<()>>(
            &self,
            writer: &mut W,
            offset: u64,
            stride: u64,
            write_t: F,
        ) -> std::io::Result<()> {
            match self {
                VectorData::Vector2(v) => write_vector_data(writer, v, offset, stride, write_t),
                VectorData::Vector3(v) => write_vector_data(writer, v, offset, stride, write_t),
                VectorData::Vector4(v) => write_vector_data(writer, v, offset, stride, write_t),
            }
        }
        fn to_vec3a(&self) -> Vec<geometry_tools::glam::Vec3A> {
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
        fn to_vec4_with_w(&self, w: f32) -> Vec<geometry_tools::glam::Vec4> {
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
    }
    pub fn read_mesh_objects(mesh: &Mesh) -> Result<Vec<MeshObjectData>, Box<dyn Error>> {
        let mut mesh_objects = Vec::new();
        for mesh_object in &mesh.objects.elements {
            let name = mesh_object.name.to_string_lossy();
            let indices = read_vertex_indices(&mesh.index_buffer.elements, &mesh_object)?;
            let positions =
                read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Position)?;
            let normals = read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Normal)?;
            let tangents = read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Tangent)?;
            let binormals =
                read_attributes_by_usage(&mesh, &mesh_object, AttributeUsage::Binormal)?;
            let texture_coordinates = read_texture_coordinates(&mesh, &mesh_object, false)?;
            let color_sets = read_colorsets(&mesh, &mesh_object)?;
            let bone_influences =
                read_rigging_data(&mesh.rigging_buffers.elements, &name, mesh_object.sub_index)?;
            let data = MeshObjectData {
                name,
                sub_index: mesh_object.sub_index,
                parent_bone_name: mesh_object
                    .parent_bone_name
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
                vertex_indices: indices,
                positions,
                normals,
                tangents,
                binormals,
                texture_coordinates,
                color_sets,
                bone_influences,
            };
            mesh_objects.push(data);
        }
        Ok(mesh_objects)
    }
    enum MeshVersion {
        Version110,
        Version108,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for MeshVersion {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&MeshVersion::Version110,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Version110");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&MeshVersion::Version108,) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Version108");
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for MeshVersion {
        #[inline]
        fn clone(&self) -> MeshVersion {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for MeshVersion {}
    pub fn create_mesh(data: &MeshData) -> Result<Mesh, MeshError> {
        let version = match (data.major_version, data.minor_version) {
            (1, 10) => Ok(MeshVersion::Version110),
            (1, 8) => Ok(MeshVersion::Version108),
            _ => Err(MeshError::UnsupportedMeshVersion {
                major_version: data.major_version,
                minor_version: data.minor_version,
            }),
        }?;
        let mesh_vertex_data = create_mesh_objects(version, &data.objects)?;
        let all_positions: Vec<geometry_tools::glam::Vec3A> = data
            .objects
            .iter()
            .map(|o| match o.positions.first() {
                Some(attribute) => attribute.data.to_vec3a(),
                None => Vec::new(),
            })
            .flatten()
            .collect();
        let mesh = Mesh {
            major_version: data.major_version,
            minor_version: data.minor_version,
            model_name: "".into(),
            bounding_info: calculate_bounding_info(&all_positions),
            unk1: 0,
            objects: mesh_vertex_data.mesh_objects.into(),
            buffer_sizes: mesh_vertex_data
                .vertex_buffers
                .iter()
                .map(|b| b.len() as u32)
                .pad_using(4, |_| 0u32)
                .collect::<Vec<u32>>()
                .into(),
            polygon_index_size: mesh_vertex_data.index_buffer.len() as u64,
            vertex_buffers: mesh_vertex_data
                .vertex_buffers
                .into_iter()
                .map(SsbhByteBuffer::new)
                .collect::<Vec<SsbhByteBuffer>>()
                .into(),
            index_buffer: mesh_vertex_data.index_buffer.into(),
            rigging_buffers: create_rigging_buffers(version, &data.objects)?.into(),
        };
        Ok(mesh)
    }
    fn calculate_max_influences(influences: &[BoneInfluence]) -> usize {
        let mut influences_by_vertex = HashMap::new();
        for influence in influences {
            for weight in &influence.vertex_weights {
                let entry = influences_by_vertex
                    .entry(weight.vertex_index)
                    .or_insert_with(HashSet::new);
                entry.insert(&influence.bone_name);
            }
        }
        influences_by_vertex
            .values()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
    }
    fn create_rigging_buffers(
        version: MeshVersion,
        object_data: &[MeshObjectData],
    ) -> std::io::Result<Vec<MeshRiggingGroup>> {
        let mut rigging_buffers = Vec::new();
        for mesh_object in object_data {
            let flags = RiggingFlags {
                max_influences: calculate_max_influences(&mesh_object.bone_influences) as u8,
                unk1: 1,
            };
            let mut buffers = Vec::new();
            for i in &mesh_object.bone_influences {
                let buffer = MeshBoneBuffer {
                    bone_name: i.bone_name.clone().into(),
                    data: create_vertex_weights(version, &i.vertex_weights)?,
                };
                buffers.push(buffer);
            }
            let buffer = MeshRiggingGroup {
                mesh_object_name: mesh_object.name.clone().into(),
                mesh_object_sub_index: mesh_object.sub_index,
                flags,
                buffers: buffers.into(),
            };
            rigging_buffers.push(buffer)
        }
        rigging_buffers.sort_by_key(|k| {
            (
                k.mesh_object_name.to_string_lossy(),
                k.mesh_object_sub_index,
            )
        });
        Ok(rigging_buffers)
    }
    fn create_vertex_weights(
        version: MeshVersion,
        vertex_weights: &[VertexWeight],
    ) -> std::io::Result<VertexWeights> {
        match version {
            MeshVersion::Version108 => {
                let weights: Vec<VertexWeightV8> = vertex_weights
                    .iter()
                    .map(|v| VertexWeightV8 {
                        vertex_index: v.vertex_index,
                        vertex_weight: v.vertex_weight,
                    })
                    .collect();
                Ok(VertexWeights::VertexWeightsV8(weights.into()))
            }
            MeshVersion::Version110 => {
                let mut bytes = Cursor::new(Vec::new());
                for weight in vertex_weights {
                    bytes.write_all(&(weight.vertex_index as u16).to_le_bytes())?;
                    bytes.write_all(&weight.vertex_weight.to_le_bytes())?;
                }
                Ok(VertexWeights::VertexWeightsV10(bytes.into_inner().into()))
            }
        }
    }
    fn get_size_in_bytes_v10(data_type: &AttributeDataTypeV10) -> usize {
        match data_type {
            AttributeDataTypeV10::Float3 => std::mem::size_of::<f32>() * 3,
            AttributeDataTypeV10::Byte4 => std::mem::size_of::<u8>() * 4,
            AttributeDataTypeV10::HalfFloat4 => std::mem::size_of::<f16>() * 4,
            AttributeDataTypeV10::HalfFloat2 => std::mem::size_of::<f16>() * 2,
            AttributeDataTypeV10::Float4 => std::mem::size_of::<f32>() * 4,
            AttributeDataTypeV10::Float2 => std::mem::size_of::<f32>() * 2,
        }
    }
    fn get_size_in_bytes_v8(data_type: &AttributeDataTypeV8) -> usize {
        match data_type {
            AttributeDataTypeV8::Float3 => std::mem::size_of::<f32>() * 3,
            AttributeDataTypeV8::HalfFloat4 => std::mem::size_of::<f16>() * 4,
            AttributeDataTypeV8::Float2 => std::mem::size_of::<f32>() * 2,
            AttributeDataTypeV8::Byte4 => std::mem::size_of::<u8>() * 4,
            AttributeDataTypeV8::Float4 => std::mem::size_of::<f32>() * 4,
        }
    }
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
            attribute_names: SsbhArray::new(<[_]>::into_vec(box [attribute_array_name.into()])),
        };
        *current_stride += get_size_in_bytes_v10(&attribute.data_type) as u32;
        attributes.push(attribute);
    }
    fn create_attributes(
        data: &MeshObjectData,
        version: MeshVersion,
    ) -> (u32, u32, MeshAttributes) {
        match version {
            MeshVersion::Version110 => create_attributes_v10(data),
            MeshVersion::Version108 => create_attributes_v8(data),
        }
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
    fn infer_data_type_v8(
        attribute: &AttributeData,
        usage: AttributeUsageV8,
    ) -> AttributeDataTypeV8 {
        match (usage, &attribute.data) {
            (AttributeUsageV8::ColorSet, VectorData::Vector4(_)) => AttributeDataTypeV8::Byte4,
            (_, VectorData::Vector2(_)) => AttributeDataTypeV8::Float2,
            (_, VectorData::Vector3(_)) => AttributeDataTypeV8::Float3,
            (_, VectorData::Vector4(_)) => AttributeDataTypeV8::HalfFloat4,
        }
    }
    fn create_attributes_v8(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
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
    fn infer_data_type_v10(
        attribute: &AttributeData,
        usage: AttributeUsageV9,
    ) -> AttributeDataTypeV10 {
        match (usage, &attribute.data) {
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
            (AttributeUsageV9::ColorSet, VectorData::Vector2(_)) => {
                AttributeDataTypeV10::HalfFloat2
            }
            (AttributeUsageV9::ColorSet, VectorData::Vector4(_)) => AttributeDataTypeV10::Byte4,
            (_, VectorData::Vector2(_)) => AttributeDataTypeV10::Float2,
            (_, VectorData::Vector3(_)) => AttributeDataTypeV10::Float3,
            (_, VectorData::Vector4(_)) => AttributeDataTypeV10::Float4,
        }
    }
    fn create_attributes_v10(data: &MeshObjectData) -> (u32, u32, MeshAttributes) {
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
    struct MeshVertexData {
        mesh_objects: Vec<MeshObject>,
        vertex_buffers: Vec<Vec<u8>>,
        index_buffer: Vec<u8>,
    }
    enum VertexIndices {
        UnsignedInt(Vec<u32>),
        UnsignedShort(Vec<u16>),
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for VertexIndices {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&VertexIndices::UnsignedInt(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "UnsignedInt");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&VertexIndices::UnsignedShort(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "UnsignedShort");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for VertexIndices {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for VertexIndices {
        #[inline]
        fn eq(&self, other: &VertexIndices) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (
                            &VertexIndices::UnsignedInt(ref __self_0),
                            &VertexIndices::UnsignedInt(ref __arg_1_0),
                        ) => (*__self_0) == (*__arg_1_0),
                        (
                            &VertexIndices::UnsignedShort(ref __self_0),
                            &VertexIndices::UnsignedShort(ref __arg_1_0),
                        ) => (*__self_0) == (*__arg_1_0),
                        _ => unsafe { ::core::intrinsics::unreachable() },
                    }
                } else {
                    false
                }
            }
        }
        #[inline]
        fn ne(&self, other: &VertexIndices) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (
                            &VertexIndices::UnsignedInt(ref __self_0),
                            &VertexIndices::UnsignedInt(ref __arg_1_0),
                        ) => (*__self_0) != (*__arg_1_0),
                        (
                            &VertexIndices::UnsignedShort(ref __self_0),
                            &VertexIndices::UnsignedShort(ref __arg_1_0),
                        ) => (*__self_0) != (*__arg_1_0),
                        _ => unsafe { ::core::intrinsics::unreachable() },
                    }
                } else {
                    true
                }
            }
        }
    }
    fn create_mesh_objects(
        version: MeshVersion,
        mesh_object_data: &[MeshObjectData],
    ) -> Result<MeshVertexData, MeshError> {
        let mut mesh_objects = Vec::new();
        let mut final_buffer_offset = 0;
        let mut index_buffer = Cursor::new(Vec::new());
        let mut buffer0 = Cursor::new(Vec::new());
        let mut buffer1 = Cursor::new(Vec::new());
        for data in mesh_object_data {
            let vertex_count = calculate_vertex_count(data)?;
            let positions = match data.positions.first() {
                Some(attribute) => attribute.data.to_vec3a(),
                None => Vec::new(),
            };
            let vertex_indices = convert_indices(&data.vertex_indices);
            let draw_element_type = match vertex_indices {
                VertexIndices::UnsignedInt(_) => DrawElementType::UnsignedInt,
                VertexIndices::UnsignedShort(_) => DrawElementType::UnsignedShort,
            };
            let vertex_buffer0_offset = buffer0.position();
            let vertex_buffer1_offset = buffer1.position();
            let (stride0, stride1, attributes) = create_attributes(data, version);
            write_attributes(
                data,
                &mut buffer0,
                &mut buffer1,
                &attributes,
                stride0 as u64,
                stride1 as u64,
                vertex_buffer0_offset,
                vertex_buffer1_offset,
            )?;
            let mesh_object = MeshObject {
                name: data.name.clone().into(),
                sub_index: data.sub_index,
                parent_bone_name: data.parent_bone_name.clone().into(),
                vertex_count: vertex_count as u32,
                vertex_index_count: data.vertex_indices.len() as u32,
                unk2: 3,
                vertex_buffer0_offset: vertex_buffer0_offset as u32,
                vertex_buffer1_offset: vertex_buffer1_offset as u32,
                final_buffer_offset,
                buffer_index: 0,
                stride0,
                stride1,
                unk6: match version {
                    MeshVersion::Version110 => 0,
                    MeshVersion::Version108 => 32,
                },
                unk7: 0,
                index_buffer_offset: index_buffer.position() as u32,
                unk8: 4,
                draw_element_type,
                rigging_type: if data.bone_influences.is_empty() {
                    RiggingType::SingleBound
                } else {
                    RiggingType::Weighted
                },
                unk11: 0,
                unk12: 0,
                bounding_info: calculate_bounding_info(&positions),
                attributes,
            };
            write_vertex_indices(&vertex_indices, &mut index_buffer)?;
            final_buffer_offset += 32 * mesh_object.vertex_count;
            mesh_objects.push(mesh_object);
        }
        Ok(MeshVertexData {
            mesh_objects,
            vertex_buffers: <[_]>::into_vec(box [
                buffer0.into_inner(),
                buffer1.into_inner(),
                Vec::new(),
                Vec::new(),
            ]),
            index_buffer: index_buffer.into_inner(),
        })
    }
    fn write_vertex_indices(
        indices: &VertexIndices,
        index_buffer: &mut Cursor<Vec<u8>>,
    ) -> Result<(), std::io::Error> {
        match indices {
            VertexIndices::UnsignedInt(indices) => {
                for index in indices {
                    index_buffer.write_all(&index.to_le_bytes())?;
                }
            }
            VertexIndices::UnsignedShort(indices) => {
                for index in indices {
                    index_buffer.write_all(&index.to_le_bytes())?;
                }
            }
        }
        Ok(())
    }
    fn write_attributes<W: Write + Seek>(
        data: &MeshObjectData,
        buffer0: &mut W,
        buffer1: &mut W,
        attributes: &MeshAttributes,
        stride0: u64,
        stride1: u64,
        offset0: u64,
        offset1: u64,
    ) -> Result<(), std::io::Error> {
        match attributes {
            MeshAttributes::AttributesV8(attributes) => {
                for a in &attributes.elements {
                    let index = a.sub_index as usize;
                    let data = match a.usage {
                        AttributeUsageV8::Position => &data.positions[index].data,
                        AttributeUsageV8::Normal => &data.normals[index].data,
                        AttributeUsageV8::Tangent => &data.tangents[index].data,
                        AttributeUsageV8::TextureCoordinate => {
                            &data.texture_coordinates[index].data
                        }
                        AttributeUsageV8::ColorSet => &data.color_sets[index].data,
                    };
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
                for a in &attributes.elements {
                    let index = a.sub_index as usize;
                    let data = match a.usage {
                        AttributeUsageV9::Position => &data.positions[index].data,
                        AttributeUsageV9::Normal => &data.normals[index].data,
                        AttributeUsageV9::Binormal => &data.binormals[index].data,
                        AttributeUsageV9::Tangent => &data.tangents[index].data,
                        AttributeUsageV9::TextureCoordinate => {
                            &data.texture_coordinates[index].data
                        }
                        AttributeUsageV9::ColorSet => &data.color_sets[index].data,
                    };
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
            MeshAttributes::AttributesV9(_) => ::core::panicking::panic("not yet implemented"),
        }
        Ok(())
    }
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
    fn calculate_vertex_count(data: &MeshObjectData) -> Result<usize, MeshError> {
        let sizes: Vec<_> = data
            .positions
            .iter()
            .map(|a| a.data.len())
            .chain(data.normals.iter().map(|a| a.data.len()))
            .chain(data.binormals.iter().map(|a| a.data.len()))
            .chain(data.tangents.iter().map(|a| a.data.len()))
            .chain(data.texture_coordinates.iter().map(|a| a.data.len()))
            .chain(data.color_sets.iter().map(|a| a.data.len()))
            .collect();
        if sizes.iter().all_equal() {
            match sizes.first() {
                Some(size) => Ok(*size),
                None => Ok(0),
            }
        } else {
            Err(MeshError::AttributeDataLengthMismatch)
        }
    }
    fn convert_indices(indices: &[u32]) -> VertexIndices {
        let u16_indices: Result<Vec<u16>, _> = indices.iter().map(|i| u16::try_from(*i)).collect();
        match u16_indices {
            Ok(indices) => VertexIndices::UnsignedShort(indices),
            Err(_) => VertexIndices::UnsignedInt(indices.into()),
        }
    }
    fn transform_inner(data: &VectorData, transform: &[[f32; 4]; 4], w: f32) -> VectorData {
        let mut points = data.to_vec4_with_w(w);
        let matrix = glam::Mat4::from_cols_array_2d(transform);
        for point in points.iter_mut() {
            *point = matrix.mul_vec4(*point);
        }
        match data {
            VectorData::Vector2(_) => {
                VectorData::Vector2(points.iter().map(|p| [p.x, p.y]).collect())
            }
            VectorData::Vector3(_) => {
                VectorData::Vector3(points.iter().map(|p| [p.x, p.y, p.z]).collect())
            }
            VectorData::Vector4(original) => VectorData::Vector4(
                original
                    .iter()
                    .zip(points)
                    .map(|(old, new)| [new.x, new.y, new.z, old[3]])
                    .collect(),
            ),
        }
    }
    /// Transform the elements in `data` with `transform`.
    /// Transform is assumed to be in row-major order.
    /// The elements are treated as points in homogeneous coordinates by temporarily setting the 4th component to `1.0f32`.
    /// The returned result has the same component count as `data`.
    /// For [VectorData::Vector4], the 4th component is preserved for the returned result.
    ///
    ///```rust
    ///# use ssbh_data::mesh_data::{VectorData, AttributeData, MeshObjectData, transform_points};
    ///# let mesh_object_data = MeshObjectData {
    ///#     name: "abc".into(),
    ///#     sub_index: 0,
    ///#     parent_bone_name: "".into(),
    ///#     vertex_indices: Vec::new(),
    ///#     positions: vec![AttributeData {
    ///#         name: "Position0".into(),
    ///#         data: VectorData::Vector3(Vec::new())
    ///#     }],
    ///#     normals: Vec::new(),
    ///#     binormals: Vec::new(),
    ///#     tangents: Vec::new(),
    ///#     texture_coordinates: Vec::new(),
    ///#     color_sets: Vec::new(),
    ///#     bone_influences: Vec::new(),
    ///# };
    ///// A scaling matrix for x, y, and z.
    ///let transform = [
    ///    [1.0, 0.0, 0.0, 0.0],
    ///    [0.0, 2.0, 0.0, 0.0],
    ///    [0.0, 0.0, 3.0, 0.0],
    ///    [0.0, 0.0, 0.0, 1.0],
    ///];
    ///let transformed_positions = transform_points(&mesh_object_data.positions[0].data, &transform);
    ///```
    pub fn transform_points(data: &VectorData, transform: &[[f32; 4]; 4]) -> VectorData {
        transform_inner(data, transform, 1.0)
    }
    /// Transform the elements in `data` with `transform`.
    /// Transform is assumed to be in row-major order.
    /// The elements are treated as vectors in homogeneous coordinates by temporarily setting the 4th component to `0.0f32`.
    /// The returned result has the same component count as `data`.
    /// For [VectorData::Vector4], the 4th component is preserved for the returned result.
    ///
    ///```rust
    ///# use ssbh_data::mesh_data::{VectorData, AttributeData, MeshObjectData, transform_vectors};
    ///# let mesh_object_data = MeshObjectData {
    ///#     name: "abc".into(),
    ///#     sub_index: 0,
    ///#     parent_bone_name: "".into(),
    ///#     vertex_indices: Vec::new(),
    ///#     positions: Vec::new(),
    ///#     normals: vec![AttributeData {
    ///#         name: "Normal0".into(),
    ///#         data: VectorData::Vector3(Vec::new())
    ///#     }],
    ///#     binormals: Vec::new(),
    ///#     tangents: Vec::new(),
    ///#     texture_coordinates: Vec::new(),
    ///#     color_sets: Vec::new(),
    ///#     bone_influences: Vec::new(),
    ///# };
    ///// A scaling matrix for x, y, and z.
    ///let transform = [
    ///    [1.0, 0.0, 0.0, 0.0],
    ///    [0.0, 2.0, 0.0, 0.0],
    ///    [0.0, 0.0, 3.0, 0.0],
    ///    [0.0, 0.0, 0.0, 1.0],
    ///];
    ///let transformed_normals = transform_vectors(&mesh_object_data.normals[0].data, &transform);
    ///```
    pub fn transform_vectors(data: &VectorData, transform: &[[f32; 4]; 4]) -> VectorData {
        transform_inner(data, transform, 0.0)
    }
    fn calculate_bounding_info(
        positions: &[geometry_tools::glam::Vec3A],
    ) -> ssbh_lib::formats::mesh::BoundingInfo {
        let (sphere_center, sphere_radius) =
            geometry_tools::calculate_bounding_sphere_from_points(&positions);
        let (aabb_min, aabb_max) = geometry_tools::calculate_aabb_from_points(&positions);
        let obb_center = aabb_min.add(aabb_max).div(2f32);
        let obb_size = aabb_max.sub(aabb_min).div(2f32);
        ssbh_lib::formats::mesh::BoundingInfo {
            bounding_sphere: ssbh_lib::formats::mesh::BoundingSphere {
                center: Vector3::new(sphere_center.x, sphere_center.y, sphere_center.z),
                radius: sphere_radius,
            },
            bounding_volume: ssbh_lib::formats::mesh::BoundingVolume {
                min: Vector3::new(aabb_min.x, aabb_min.y, aabb_min.z),
                max: Vector3::new(aabb_max.x, aabb_max.y, aabb_max.z),
            },
            oriented_bounding_box: ssbh_lib::formats::mesh::OrientedBoundingBox {
                center: Vector3::new(obb_center.x, obb_center.y, obb_center.z),
                transform: Matrix3x3::identity(),
                size: Vector3::new(obb_size.x, obb_size.y, obb_size.z),
            },
        }
    }
    fn read_influences(
        rigging_group: &MeshRiggingGroup,
    ) -> Result<Vec<BoneInfluence>, Box<dyn Error>> {
        let mut bone_influences = Vec::new();
        for buffer in &rigging_group.buffers.elements {
            let bone_name = buffer
                .bone_name
                .to_str()
                .ok_or("Failed to read bone name.")?;
            let influences = match &buffer.data {
                VertexWeights::VertexWeightsV8(v) | VertexWeights::VertexWeightsV9(v) => v
                    .elements
                    .iter()
                    .map(|influence| VertexWeight {
                        vertex_index: influence.vertex_index,
                        vertex_weight: influence.vertex_weight,
                    })
                    .collect(),
                VertexWeights::VertexWeightsV10(v) => read_vertex_weights_v9(v),
            };
            let bone_influence = BoneInfluence {
                bone_name: bone_name.to_string(),
                vertex_weights: influences,
            };
            bone_influences.push(bone_influence);
        }
        Ok(bone_influences)
    }
    fn read_vertex_weights_v9(v: &SsbhByteBuffer) -> Vec<VertexWeight> {
        let mut elements = Vec::new();
        let mut reader = Cursor::new(&v.elements);
        while let Ok(influence) = reader.read_le::<ssbh_lib::formats::mesh::VertexWeightV10>() {
            elements.push(VertexWeight {
                vertex_index: influence.vertex_index as u32,
                vertex_weight: influence.vertex_weight,
            });
        }
        elements
    }
    struct MeshAttribute {
        pub name: String,
        pub index: u64,
        pub offset: u64,
        pub data_type: DataType,
    }
    impl From<&MeshAttributeV9> for MeshAttribute {
        fn from(a: &MeshAttributeV9) -> Self {
            MeshAttribute {
                name: get_attribute_name_v9(a).unwrap_or("").to_string(),
                index: a.buffer_index as u64,
                offset: a.buffer_offset as u64,
                data_type: a.data_type.into(),
            }
        }
    }
    impl From<&MeshAttributeV10> for MeshAttribute {
        fn from(a: &MeshAttributeV10) -> Self {
            MeshAttribute {
                name: get_attribute_name_v10(a).unwrap_or("").to_string(),
                index: a.buffer_index as u64,
                offset: a.buffer_offset as u64,
                data_type: a.data_type.into(),
            }
        }
    }
    impl From<&MeshAttributeV8> for MeshAttribute {
        fn from(a: &MeshAttributeV8) -> Self {
            let name = match a.usage {
                AttributeUsageV8::Position => {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Position"],
                        &match (&a.sub_index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                }
                AttributeUsageV8::Normal => {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Normal"],
                        &match (&a.sub_index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                }
                AttributeUsageV8::Tangent => {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Tangent"],
                        &match (&a.sub_index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                }
                AttributeUsageV8::TextureCoordinate => {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["TextureCoordinate"],
                        &match (&a.sub_index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                }
                AttributeUsageV8::ColorSet => {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["colorSet"],
                        &match (&a.sub_index,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                }
            };
            MeshAttribute {
                name,
                index: a.buffer_index as u64,
                offset: a.buffer_offset as u64,
                data_type: a.data_type.into(),
            }
        }
    }
    fn get_attributes(mesh_object: &MeshObject, usage: AttributeUsage) -> Vec<MeshAttribute> {
        match &mesh_object.attributes {
            MeshAttributes::AttributesV8(attributes) => attributes
                .elements
                .iter()
                .filter(|a| AttributeUsage::from(a.usage) == usage)
                .map(|a| a.into())
                .collect(),
            MeshAttributes::AttributesV10(attributes) => attributes
                .elements
                .iter()
                .filter(|a| AttributeUsage::from(a.usage) == usage)
                .map(|a| a.into())
                .collect(),
            MeshAttributes::AttributesV9(attributes) => attributes
                .elements
                .iter()
                .filter(|a| AttributeUsage::from(a.usage) == usage)
                .map(|a| a.into())
                .collect(),
        }
    }
    fn get_attribute_name_v9(attribute: &MeshAttributeV9) -> Option<&str> {
        attribute.attribute_names.elements.get(0)?.to_str()
    }
    fn get_attribute_name_v10(attribute: &MeshAttributeV10) -> Option<&str> {
        attribute.attribute_names.elements.get(0)?.to_str()
    }
}
pub mod modl_data {
    use std::{
        io::{Read, Seek},
        path::Path,
    };
    use ssbh_lib::{formats::modl::*, RelPtr64};
    use crate::create_ssbh_array;
    pub struct ModlEntryData {
        pub mesh_object_name: String,
        pub mesh_object_sub_index: u64,
        pub material_label: String,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ModlEntryData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                ModlEntryData {
                    mesh_object_name: ref __self_0_0,
                    mesh_object_sub_index: ref __self_0_1,
                    material_label: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "ModlEntryData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "mesh_object_name",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "mesh_object_sub_index",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "material_label",
                        &&(*__self_0_2),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    /// The data associated with a [Modl] file.
    /// The supported version is 1.7.
    pub struct ModlData {
        pub major_version: u16,
        pub minor_version: u16,
        pub model_name: String,
        pub skeleton_file_name: String,
        pub material_file_names: Vec<String>,
        pub animation_file_name: Option<String>,
        pub mesh_file_name: String,
        pub entries: Vec<ModlEntryData>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ModlData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                ModlData {
                    major_version: ref __self_0_0,
                    minor_version: ref __self_0_1,
                    model_name: ref __self_0_2,
                    skeleton_file_name: ref __self_0_3,
                    material_file_names: ref __self_0_4,
                    animation_file_name: ref __self_0_5,
                    mesh_file_name: ref __self_0_6,
                    entries: ref __self_0_7,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "ModlData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "major_version",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "minor_version",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "model_name",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "skeleton_file_name",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "material_file_names",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "animation_file_name",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "mesh_file_name",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "entries",
                        &&(*__self_0_7),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl ModlData {
        /// Tries to read and convert the MODL from `path`.
        /// The entire file is buffered for performance.
        pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
            let modl = Modl::from_file(path)?;
            Ok((&modl).into())
        }
        /// Tries to read and convert the MODL from `reader`.
        /// For best performance when opening from a file, use `from_file` instead.
        pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
            let modl = Modl::read(reader)?;
            Ok((&modl).into())
        }
        /// Converts the data to MODL and writes to the given `writer`.
        /// For best performance when writing to a file, use `write_to_file` instead.
        pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
            let modl: Modl = self.into();
            modl.write(writer)?;
            Ok(())
        }
        /// Converts the data to MODL and writes to the given `path`.
        /// The entire file is buffered for performance.
        pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
            let modl: Modl = self.into();
            modl.write_to_file(path)?;
            Ok(())
        }
    }
    impl From<Modl> for ModlData {
        fn from(m: Modl) -> Self {
            Self::from(&m)
        }
    }
    impl From<&Modl> for ModlData {
        fn from(m: &Modl) -> Self {
            Self {
                major_version: m.major_version,
                minor_version: m.minor_version,
                model_name: m.model_name.to_string_lossy(),
                skeleton_file_name: m.skeleton_file_name.to_string_lossy(),
                material_file_names: m
                    .material_file_names
                    .elements
                    .iter()
                    .map(|f| f.to_string_lossy())
                    .collect(),
                animation_file_name: (*m.animation_file_name)
                    .as_ref()
                    .map(|s| s.to_string_lossy()),
                mesh_file_name: m.mesh_file_name.to_string_lossy(),
                entries: m.entries.elements.iter().map(|e| e.into()).collect(),
            }
        }
    }
    impl From<ModlData> for Modl {
        fn from(m: ModlData) -> Self {
            Self::from(&m)
        }
    }
    impl From<&ModlData> for Modl {
        fn from(m: &ModlData) -> Self {
            Self {
                major_version: m.major_version,
                minor_version: m.minor_version,
                model_name: m.model_name.clone().into(),
                skeleton_file_name: m.skeleton_file_name.clone().into(),
                material_file_names: create_ssbh_array(&m.material_file_names, |f| {
                    f.as_str().into()
                }),
                animation_file_name: match &m.animation_file_name {
                    Some(name) => RelPtr64::new(name.as_str().into()),
                    None => RelPtr64::null(),
                },
                mesh_file_name: m.mesh_file_name.as_str().into(),
                entries: create_ssbh_array(&m.entries, |e| e.into()),
            }
        }
    }
    impl From<ModlEntryData> for ModlEntry {
        fn from(m: ModlEntryData) -> Self {
            Self::from(&m)
        }
    }
    impl From<&ModlEntryData> for ModlEntry {
        fn from(m: &ModlEntryData) -> Self {
            Self {
                mesh_object_name: m.mesh_object_name.as_str().into(),
                mesh_object_sub_index: m.mesh_object_sub_index,
                material_label: m.material_label.as_str().into(),
            }
        }
    }
    impl From<&ModlEntry> for ModlEntryData {
        fn from(m: &ModlEntry) -> Self {
            Self {
                mesh_object_name: m.mesh_object_name.to_string_lossy(),
                mesh_object_sub_index: m.mesh_object_sub_index,
                material_label: m.material_label.to_string_lossy(),
            }
        }
    }
    impl From<ModlEntry> for ModlEntryData {
        fn from(m: ModlEntry) -> Self {
            Self::from(&m)
        }
    }
}
pub mod skel_data {
    use std::{
        convert::TryInto,
        io::{Read, Seek},
        path::Path,
    };
    use glam::Mat4;
    use ssbh_lib::{
        formats::skel::{BillboardType, Skel, SkelBoneEntry, SkelEntryFlags},
        Matrix4x4,
    };
    use crate::create_ssbh_array;
    /// The data associated with a [Skel] file.
    /// The supported version is 1.0.
    pub struct SkelData {
        pub major_version: u16,
        pub minor_version: u16,
        pub bones: Vec<BoneData>,
    }
    pub struct BoneData {
        /// The name of the bone.
        pub name: String,
        /// A matrix in row-major order representing the transform of the bone relative to its parent.
        /// For using existing world transformations, see [calculate_relative_transform].
        pub transform: [[f32; 4]; 4],
        /// The index of the parent bone in the bones collection or [None] if this is a root bone with no parents.
        pub parent_index: Option<usize>,
    }
    impl SkelData {
        /// Tries to read and convert the SKEL from `path`.
        /// The entire file is buffered for performance.
        pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
            let skel = Skel::from_file(path)?;
            Ok((&skel).into())
        }
        /// Tries to read and convert the SKEL from `reader`.
        /// For best performance when opening from a file, use `from_file` instead.
        pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
            let skel = Skel::read(reader)?;
            Ok((&skel).into())
        }
        /// Converts the data to SKEL and writes to the given `writer`.
        /// For best performance when writing to a file, use `write_to_file` instead.
        pub fn write<W: std::io::Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
            let skel = create_skel(&self);
            skel.write(writer)?;
            Ok(())
        }
        /// Converts the data to SKEL and writes to the given `path`.
        /// The entire file is buffered for performance.
        pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
            let skel = create_skel(&self);
            skel.write_to_file(path)?;
            Ok(())
        }
    }
    /// Calculates the transform of `world_transform` relative to `parent_world_transform`.
    /// If `parent_world_transform` is [None] or the identity matrix, a copy of `world_transform` is returned.
    /// All matrices are assumed to be in row-major order.
    ///
    ///```rust
    ///# use ssbh_data::skel_data::calculate_relative_transform;
    ///let world_transform = [
    ///    [2.0, 0.0, 0.0, 0.0],
    ///    [0.0, 4.0, 0.0, 0.0],
    ///    [0.0, 0.0, 8.0, 0.0],
    ///    [1.0, 2.0, 3.0, 1.0],
    ///];
    ///let parent_world_transform = [
    ///    [1.0, 0.0, 0.0, 0.0],
    ///    [0.0, 1.0, 0.0, 0.0],
    ///    [0.0, 0.0, 1.0, 0.0],
    ///    [0.0, 0.0, 0.0, 1.0],
    ///];
    ///assert_eq!(
    ///    world_transform,
    ///    calculate_relative_transform(
    ///        &world_transform,
    ///        Some(&parent_world_transform)
    ///    )
    ///);
    ///```
    pub fn calculate_relative_transform(
        world_transform: &[[f32; 4]; 4],
        parent_world_transform: Option<&[[f32; 4]; 4]>,
    ) -> [[f32; 4]; 4] {
        match parent_world_transform {
            Some(parent_world_transform) => {
                let world = mat4_from_row2d(world_transform);
                let parent_world = mat4_from_row2d(parent_world_transform);
                let relative = parent_world.inverse().mul_mat4(&world);
                relative.transpose().to_cols_array_2d()
            }
            None => *world_transform,
        }
    }
    fn inv_transform(m: &[[f32; 4]; 4]) -> Matrix4x4 {
        let m = mat4_from_row2d(m);
        let inv = m.inverse().transpose().to_cols_array_2d();
        Matrix4x4::from_rows_array(&inv)
    }
    pub fn create_skel(data: &SkelData) -> Skel {
        let world_transforms: Vec<_> = data
            .bones
            .iter()
            .map(|b| data.calculate_world_transform(b))
            .collect();
        Skel {
            major_version: data.major_version,
            minor_version: data.minor_version,
            bone_entries: data
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| SkelBoneEntry {
                    name: b.name.clone().into(),
                    index: i as u16,
                    parent_index: match b.parent_index {
                        Some(index) => index as i16,
                        None => -1,
                    },
                    flags: SkelEntryFlags {
                        unk1: 1,
                        billboard_type: BillboardType::None,
                    },
                })
                .collect::<Vec<SkelBoneEntry>>()
                .into(),
            world_transforms: create_ssbh_array(&world_transforms, Matrix4x4::from_rows_array),
            inv_world_transforms: create_ssbh_array(&world_transforms, inv_transform),
            transforms: create_ssbh_array(&data.bones, |b| {
                Matrix4x4::from_rows_array(&b.transform)
            }),
            inv_transforms: create_ssbh_array(&data.bones, |b| inv_transform(&b.transform)),
        }
    }
    impl From<&Skel> for SkelData {
        fn from(skel: &Skel) -> Self {
            Self {
                major_version: skel.major_version,
                minor_version: skel.minor_version,
                bones: skel
                    .bone_entries
                    .elements
                    .iter()
                    .zip(skel.transforms.elements.iter())
                    .map(|(b, t)| create_bone_data(b, t))
                    .collect(),
            }
        }
    }
    fn create_bone_data(b: &SkelBoneEntry, transform: &Matrix4x4) -> BoneData {
        BoneData {
            name: b.name.to_string_lossy(),
            transform: transform.to_rows_array(),
            parent_index: b.parent_index.try_into().ok(),
        }
    }
    fn mat4_from_row2d(elements: &[[f32; 4]; 4]) -> Mat4 {
        Mat4::from_cols_array_2d(&elements).transpose()
    }
    impl SkelData {
        /// Calculates the world transform for `bone` by accumulating the transform with the parents transform recursively.
        /// For single bound objects, the object is transformed by the parent bone's world transform.
        /// Returns the resulting matrix in row-major order.
        ///
        ///    ```rust
        ///    # use ssbh_data::skel_data::{BoneData, SkelData};
        ///    # let data = SkelData {
        ///    #     major_version: 1,
        ///    #     minor_version: 0,
        ///    #     bones: vec![BoneData {
        ///    #         name: "Head".to_string(),
        ///    #         transform: [[0f32; 4]; 4],
        ///    #         parent_index: None,
        ///    #     }],
        ///    # };
        ///    let parent_bone_name = "Head";
        ///    if let Some(parent_bone) = data.bones.iter().find(|b| b.name == parent_bone_name) {
        ///        let world_transform = data.calculate_world_transform(&parent_bone);
        ///    }
        ///    ```
        ///    
        pub fn calculate_world_transform(&self, bone: &BoneData) -> [[f32; 4]; 4] {
            let mut bone = bone;
            let mut transform = mat4_from_row2d(&bone.transform);
            while let Some(parent_index) = bone.parent_index {
                if let Some(parent_bone) = self.bones.get(parent_index) {
                    let parent_transform = mat4_from_row2d(&parent_bone.transform);
                    transform = transform.mul_mat4(&parent_transform);
                    bone = parent_bone;
                } else {
                    break;
                }
            }
            transform.transpose().to_cols_array_2d()
        }
    }
}
mod anim_data {
    use binread::{BinRead, BinResult, ReadOptions};
    use bit_vec::BitVec;
    use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};
    use modular_bitfield::prelude::*;
    use std::{
        io::{Cursor, Read, Seek, Write},
        num::NonZeroU64,
    };
    use ssbh_write::SsbhWrite;
    use binread::BinReaderExt;
    use ssbh_lib::{
        formats::anim::{CompressionType, TrackFlags, TrackType},
        Ptr16, Ptr32, Vector3, Vector4,
    };
    struct CompressedTrackData<T: CompressedData> {
        pub header: CompressedHeader<T>,
        pub compression: T::Compression,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<T: ::core::fmt::Debug + CompressedData> ::core::fmt::Debug for CompressedTrackData<T>
    where
        T::Compression: ::core::fmt::Debug,
    {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                CompressedTrackData {
                    header: ref __self_0_0,
                    compression: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "CompressedTrackData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "header",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "compression",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl<T: CompressedData> binread::BinRead for CompressedTrackData<T> {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_header = ();
                let __binread_generated_options_header = __binread_generated_var_options;
                let mut header: CompressedHeader<T> = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_header,
                    __binread_generated_args_header.clone(),
                )?;
                let __binread_generated_args_compression = ();
                let __binread_generated_options_compression = __binread_generated_var_options;
                let mut compression: T::Compression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_compression,
                    __binread_generated_args_compression.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut header,
                    __binread_generated_var_reader,
                    __binread_generated_options_header,
                    __binread_generated_args_header.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut compression,
                    __binread_generated_var_reader,
                    __binread_generated_options_compression,
                    __binread_generated_args_compression.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    header,
                    compression,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl<T: CompressedData> ssbh_write::SsbhWrite for CompressedTrackData<T> {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.header.ssbh_write(writer, data_ptr)?;
            self.compression.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.header.size_in_bytes();
            size += self.compression.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    struct CompressedHeader<T: CompressedData> {
        pub unk_4: u16,
        pub flags: CompressionFlags,
        pub default_data: Ptr16<T>,
        pub bits_per_entry: u16,
        pub compressed_data: Ptr32<CompressedBuffer>,
        pub frame_count: u32,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<T: ::core::fmt::Debug + CompressedData> ::core::fmt::Debug for CompressedHeader<T> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                CompressedHeader {
                    unk_4: ref __self_0_0,
                    flags: ref __self_0_1,
                    default_data: ref __self_0_2,
                    bits_per_entry: ref __self_0_3,
                    compressed_data: ref __self_0_4,
                    frame_count: ref __self_0_5,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "CompressedHeader");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unk_4",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "flags",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "default_data",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "bits_per_entry",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "compressed_data",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "frame_count",
                        &&(*__self_0_5),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl<T: CompressedData> binread::BinRead for CompressedHeader<T> {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_unk_4 = ();
                let __binread_generated_options_unk_4 = __binread_generated_var_options;
                let mut unk_4: u16 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_unk_4,
                    __binread_generated_args_unk_4.clone(),
                )?;
                let __binread_generated_args_flags = ();
                let __binread_generated_options_flags = __binread_generated_var_options;
                let mut flags: CompressionFlags = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_flags,
                    __binread_generated_args_flags.clone(),
                )?;
                let __binread_generated_args_default_data = ();
                let __binread_generated_options_default_data = __binread_generated_var_options;
                let mut default_data: Ptr16<T> = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_default_data,
                    __binread_generated_args_default_data.clone(),
                )?;
                let __binread_generated_args_bits_per_entry = ();
                let __binread_generated_options_bits_per_entry = __binread_generated_var_options;
                let mut bits_per_entry: u16 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_bits_per_entry,
                    __binread_generated_args_bits_per_entry.clone(),
                )?;
                let __binread_generated_args_compressed_data = ();
                let __binread_generated_options_compressed_data = __binread_generated_var_options;
                let mut compressed_data: Ptr32<CompressedBuffer> = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_compressed_data,
                    __binread_generated_args_compressed_data.clone(),
                )?;
                let __binread_generated_args_frame_count = ();
                let __binread_generated_options_frame_count = __binread_generated_var_options;
                let mut frame_count: u32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_frame_count,
                    __binread_generated_args_frame_count.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut unk_4,
                    __binread_generated_var_reader,
                    __binread_generated_options_unk_4,
                    __binread_generated_args_unk_4.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut flags,
                    __binread_generated_var_reader,
                    __binread_generated_options_flags,
                    __binread_generated_args_flags.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut default_data,
                    __binread_generated_var_reader,
                    __binread_generated_options_default_data,
                    __binread_generated_args_default_data.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut bits_per_entry,
                    __binread_generated_var_reader,
                    __binread_generated_options_bits_per_entry,
                    __binread_generated_args_bits_per_entry.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut compressed_data,
                    __binread_generated_var_reader,
                    __binread_generated_options_compressed_data,
                    __binread_generated_args_compressed_data.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut frame_count,
                    __binread_generated_var_reader,
                    __binread_generated_options_frame_count,
                    __binread_generated_args_frame_count.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    unk_4,
                    flags,
                    default_data,
                    bits_per_entry,
                    compressed_data,
                    frame_count,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl<T: CompressedData> ssbh_write::SsbhWrite for CompressedHeader<T> {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.unk_4.ssbh_write(writer, data_ptr)?;
            self.flags.ssbh_write(writer, data_ptr)?;
            self.default_data.ssbh_write(writer, data_ptr)?;
            self.bits_per_entry.ssbh_write(writer, data_ptr)?;
            self.compressed_data.ssbh_write(writer, data_ptr)?;
            self.frame_count.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.unk_4.size_in_bytes();
            size += self.flags.size_in_bytes();
            size += self.default_data.size_in_bytes();
            size += self.bits_per_entry.size_in_bytes();
            size += self.compressed_data.size_in_bytes();
            size += self.frame_count.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    fn read_to_end<R: Read + Seek>(reader: &mut R, _ro: &ReadOptions, _: ()) -> BinResult<Vec<u8>> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(buf)
    }
    struct CompressedBuffer(# [br (parse_with = read_to_end)] Vec<u8>);
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for CompressedBuffer {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                CompressedBuffer(ref __self_0_0) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "CompressedBuffer");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for CompressedBuffer {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_self_0 = ();
                let __binread_generated_options_self_0 = __binread_generated_var_options;
                let mut self_0: Vec<u8> = read_to_end(
                    __binread_generated_var_reader,
                    __binread_generated_options_self_0,
                    __binread_generated_args_self_0.clone(),
                )?;
                Ok(Self(self_0))
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ssbh_write::SsbhWrite for CompressedBuffer {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.0.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.0.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    # [br (map = Self :: from_bytes)]
    #[allow(clippy::identity_op)]
    struct CompressionFlags {
        bytes: [::core::primitive::u8; { (((16usize - 1) / 8) + 1) * 8 } / 8usize],
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for CompressionFlags {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_var_options,
                    __binread_generated_var_arguments,
                )
                .map(Self::from_bytes)
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(clippy::identity_op)]
    impl ::core::clone::Clone for CompressionFlags {
        #[inline]
        fn clone(&self) -> CompressionFlags {
            {
                let _: ::core::clone::AssertParamIsClone<
                    [::core::primitive::u8; { (((16usize - 1) / 8) + 1) * 8 } / 8usize],
                >;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(clippy::identity_op)]
    impl ::core::marker::Copy for CompressionFlags {}
    #[allow(clippy::identity_op)]
    const _: () = {
        impl ::modular_bitfield::private::checks::CheckFillsUnalignedBits for CompressionFlags {
            type CheckType = [(); (16usize == {
                0usize
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <B12 as ::modular_bitfield::Specifier>::BITS
            }) as usize];
        }
    };
    impl CompressionFlags {
        /// Returns an instance with zero initialized data.
        #[allow(clippy::identity_op)]
        pub const fn new() -> Self {
            Self {
                bytes: [0u8; { (((16usize - 1) / 8) + 1) * 8 } / 8usize],
            }
        }
    }
    impl CompressionFlags {
        /// Returns the underlying bits.
        ///
        /// # Layout
        ///
        /// The returned byte array is layed out in the same way as described
        /// [here](https://docs.rs/modular-bitfield/#generated-structure).
        #[inline]
        #[allow(clippy::identity_op)]
        pub const fn into_bytes(
            self,
        ) -> [::core::primitive::u8; { (((16usize - 1) / 8) + 1) * 8 } / 8usize] {
            self.bytes
        }
        /// Converts the given bytes directly into the bitfield struct.
        #[inline]
        #[allow(clippy::identity_op)]
        pub const fn from_bytes(
            bytes: [::core::primitive::u8; { (((16usize - 1) / 8) + 1) * 8 } / 8usize],
        ) -> Self {
            Self { bytes }
        }
    }
    const _: () = {
        const _: () = {};
        const _: () = {};
        const _: () = {};
        const _: () = {};
        const _: () = {};
    };
    impl CompressionFlags {
        ///Returns the value of has_scale.
        #[inline]
        fn has_scale(&self) -> <bool as ::modular_bitfield::Specifier>::InOut {
            self.has_scale_or_err()
                .expect("value contains invalid bit pattern for field CompressionFlags.has_scale")
        }
        ///Returns the value of has_scale.
        ///
        ///#Errors
        ///
        ///If the returned value contains an invalid bit pattern for has_scale.
        #[inline]
        #[allow(dead_code)]
        fn has_scale_or_err(
            &self,
        ) -> ::core::result::Result<
            <bool as ::modular_bitfield::Specifier>::InOut,
            ::modular_bitfield::error::InvalidBitPattern<
                <bool as ::modular_bitfield::Specifier>::Bytes,
            >,
        > {
            let __bf_read: <bool as ::modular_bitfield::Specifier>::Bytes =
                { ::modular_bitfield::private::read_specifier::<bool>(&self.bytes[..], 0usize) };
            <bool as ::modular_bitfield::Specifier>::from_bytes(__bf_read)
        }
        ///Returns a copy of the bitfield with the value of has_scale set to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_scale.
        #[inline]
        #[allow(dead_code)]
        fn with_has_scale(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> Self {
            self.set_has_scale(new_val);
            self
        }
        ///Returns a copy of the bitfield with the value of has_scale set to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_scale.
        #[inline]
        #[allow(dead_code)]
        fn with_has_scale_checked(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<Self, ::modular_bitfield::error::OutOfBounds> {
            self.set_has_scale_checked(new_val)?;
            ::core::result::Result::Ok(self)
        }
        ///Sets the value of has_scale to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_scale.
        #[inline]
        #[allow(dead_code)]
        fn set_has_scale(&mut self, new_val: <bool as ::modular_bitfield::Specifier>::InOut) {
            self.set_has_scale_checked(new_val)
                .expect("value out of bounds for field CompressionFlags.has_scale")
        }
        ///Sets the value of has_scale to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_scale.
        #[inline]
        fn set_has_scale_checked(
            &mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<(), ::modular_bitfield::error::OutOfBounds> {
            let __bf_base_bits: ::core::primitive::usize =
                8usize * ::core::mem::size_of::<<bool as ::modular_bitfield::Specifier>::Bytes>();
            let __bf_max_value: <bool as ::modular_bitfield::Specifier>::Bytes =
                { !0 >> (__bf_base_bits - <bool as ::modular_bitfield::Specifier>::BITS) };
            let __bf_spec_bits: ::core::primitive::usize =
                <bool as ::modular_bitfield::Specifier>::BITS;
            let __bf_raw_val: <bool as ::modular_bitfield::Specifier>::Bytes =
                { <bool as ::modular_bitfield::Specifier>::into_bytes(new_val) }?;
            if !(__bf_base_bits == __bf_spec_bits || __bf_raw_val <= __bf_max_value) {
                return ::core::result::Result::Err(::modular_bitfield::error::OutOfBounds);
            }
            ::modular_bitfield::private::write_specifier::<bool>(
                &mut self.bytes[..],
                0usize,
                __bf_raw_val,
            );
            ::core::result::Result::Ok(())
        }
        ///Returns the value of has_compensate_scale.
        #[inline]
        fn has_compensate_scale(&self) -> <bool as ::modular_bitfield::Specifier>::InOut {
            self . has_compensate_scale_or_err () . expect ("value contains invalid bit pattern for field CompressionFlags.has_compensate_scale")
        }
        ///Returns the value of has_compensate_scale.
        ///
        ///#Errors
        ///
        ///If the returned value contains an invalid bit pattern for has_compensate_scale.
        #[inline]
        #[allow(dead_code)]
        fn has_compensate_scale_or_err(
            &self,
        ) -> ::core::result::Result<
            <bool as ::modular_bitfield::Specifier>::InOut,
            ::modular_bitfield::error::InvalidBitPattern<
                <bool as ::modular_bitfield::Specifier>::Bytes,
            >,
        > {
            let __bf_read: <bool as ::modular_bitfield::Specifier>::Bytes = {
                ::modular_bitfield::private::read_specifier::<bool>(
                    &self.bytes[..],
                    0usize + <bool as ::modular_bitfield::Specifier>::BITS,
                )
            };
            <bool as ::modular_bitfield::Specifier>::from_bytes(__bf_read)
        }
        ///Returns a copy of the bitfield with the value of has_compensate_scale set to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_compensate_scale.
        #[inline]
        #[allow(dead_code)]
        fn with_has_compensate_scale(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> Self {
            self.set_has_compensate_scale(new_val);
            self
        }
        ///Returns a copy of the bitfield with the value of has_compensate_scale set to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_compensate_scale.
        #[inline]
        #[allow(dead_code)]
        fn with_has_compensate_scale_checked(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<Self, ::modular_bitfield::error::OutOfBounds> {
            self.set_has_compensate_scale_checked(new_val)?;
            ::core::result::Result::Ok(self)
        }
        ///Sets the value of has_compensate_scale to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_compensate_scale.
        #[inline]
        #[allow(dead_code)]
        fn set_has_compensate_scale(
            &mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) {
            self.set_has_compensate_scale_checked(new_val)
                .expect("value out of bounds for field CompressionFlags.has_compensate_scale")
        }
        ///Sets the value of has_compensate_scale to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_compensate_scale.
        #[inline]
        fn set_has_compensate_scale_checked(
            &mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<(), ::modular_bitfield::error::OutOfBounds> {
            let __bf_base_bits: ::core::primitive::usize =
                8usize * ::core::mem::size_of::<<bool as ::modular_bitfield::Specifier>::Bytes>();
            let __bf_max_value: <bool as ::modular_bitfield::Specifier>::Bytes =
                { !0 >> (__bf_base_bits - <bool as ::modular_bitfield::Specifier>::BITS) };
            let __bf_spec_bits: ::core::primitive::usize =
                <bool as ::modular_bitfield::Specifier>::BITS;
            let __bf_raw_val: <bool as ::modular_bitfield::Specifier>::Bytes =
                { <bool as ::modular_bitfield::Specifier>::into_bytes(new_val) }?;
            if !(__bf_base_bits == __bf_spec_bits || __bf_raw_val <= __bf_max_value) {
                return ::core::result::Result::Err(::modular_bitfield::error::OutOfBounds);
            }
            ::modular_bitfield::private::write_specifier::<bool>(
                &mut self.bytes[..],
                0usize + <bool as ::modular_bitfield::Specifier>::BITS,
                __bf_raw_val,
            );
            ::core::result::Result::Ok(())
        }
        ///Returns the value of has_rotation.
        #[inline]
        fn has_rotation(&self) -> <bool as ::modular_bitfield::Specifier>::InOut {
            self.has_rotation_or_err().expect(
                "value contains invalid bit pattern for field CompressionFlags.has_rotation",
            )
        }
        ///Returns the value of has_rotation.
        ///
        ///#Errors
        ///
        ///If the returned value contains an invalid bit pattern for has_rotation.
        #[inline]
        #[allow(dead_code)]
        fn has_rotation_or_err(
            &self,
        ) -> ::core::result::Result<
            <bool as ::modular_bitfield::Specifier>::InOut,
            ::modular_bitfield::error::InvalidBitPattern<
                <bool as ::modular_bitfield::Specifier>::Bytes,
            >,
        > {
            let __bf_read: <bool as ::modular_bitfield::Specifier>::Bytes = {
                ::modular_bitfield::private::read_specifier::<bool>(
                    &self.bytes[..],
                    0usize
                        + <bool as ::modular_bitfield::Specifier>::BITS
                        + <bool as ::modular_bitfield::Specifier>::BITS,
                )
            };
            <bool as ::modular_bitfield::Specifier>::from_bytes(__bf_read)
        }
        ///Returns a copy of the bitfield with the value of has_rotation set to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_rotation.
        #[inline]
        #[allow(dead_code)]
        fn with_has_rotation(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> Self {
            self.set_has_rotation(new_val);
            self
        }
        ///Returns a copy of the bitfield with the value of has_rotation set to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_rotation.
        #[inline]
        #[allow(dead_code)]
        fn with_has_rotation_checked(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<Self, ::modular_bitfield::error::OutOfBounds> {
            self.set_has_rotation_checked(new_val)?;
            ::core::result::Result::Ok(self)
        }
        ///Sets the value of has_rotation to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_rotation.
        #[inline]
        #[allow(dead_code)]
        fn set_has_rotation(&mut self, new_val: <bool as ::modular_bitfield::Specifier>::InOut) {
            self.set_has_rotation_checked(new_val)
                .expect("value out of bounds for field CompressionFlags.has_rotation")
        }
        ///Sets the value of has_rotation to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_rotation.
        #[inline]
        fn set_has_rotation_checked(
            &mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<(), ::modular_bitfield::error::OutOfBounds> {
            let __bf_base_bits: ::core::primitive::usize =
                8usize * ::core::mem::size_of::<<bool as ::modular_bitfield::Specifier>::Bytes>();
            let __bf_max_value: <bool as ::modular_bitfield::Specifier>::Bytes =
                { !0 >> (__bf_base_bits - <bool as ::modular_bitfield::Specifier>::BITS) };
            let __bf_spec_bits: ::core::primitive::usize =
                <bool as ::modular_bitfield::Specifier>::BITS;
            let __bf_raw_val: <bool as ::modular_bitfield::Specifier>::Bytes =
                { <bool as ::modular_bitfield::Specifier>::into_bytes(new_val) }?;
            if !(__bf_base_bits == __bf_spec_bits || __bf_raw_val <= __bf_max_value) {
                return ::core::result::Result::Err(::modular_bitfield::error::OutOfBounds);
            }
            ::modular_bitfield::private::write_specifier::<bool>(
                &mut self.bytes[..],
                0usize
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <bool as ::modular_bitfield::Specifier>::BITS,
                __bf_raw_val,
            );
            ::core::result::Result::Ok(())
        }
        ///Returns the value of has_position.
        #[inline]
        fn has_position(&self) -> <bool as ::modular_bitfield::Specifier>::InOut {
            self.has_position_or_err().expect(
                "value contains invalid bit pattern for field CompressionFlags.has_position",
            )
        }
        ///Returns the value of has_position.
        ///
        ///#Errors
        ///
        ///If the returned value contains an invalid bit pattern for has_position.
        #[inline]
        #[allow(dead_code)]
        fn has_position_or_err(
            &self,
        ) -> ::core::result::Result<
            <bool as ::modular_bitfield::Specifier>::InOut,
            ::modular_bitfield::error::InvalidBitPattern<
                <bool as ::modular_bitfield::Specifier>::Bytes,
            >,
        > {
            let __bf_read: <bool as ::modular_bitfield::Specifier>::Bytes = {
                ::modular_bitfield::private::read_specifier::<bool>(
                    &self.bytes[..],
                    0usize
                        + <bool as ::modular_bitfield::Specifier>::BITS
                        + <bool as ::modular_bitfield::Specifier>::BITS
                        + <bool as ::modular_bitfield::Specifier>::BITS,
                )
            };
            <bool as ::modular_bitfield::Specifier>::from_bytes(__bf_read)
        }
        ///Returns a copy of the bitfield with the value of has_position set to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_position.
        #[inline]
        #[allow(dead_code)]
        fn with_has_position(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> Self {
            self.set_has_position(new_val);
            self
        }
        ///Returns a copy of the bitfield with the value of has_position set to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_position.
        #[inline]
        #[allow(dead_code)]
        fn with_has_position_checked(
            mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<Self, ::modular_bitfield::error::OutOfBounds> {
            self.set_has_position_checked(new_val)?;
            ::core::result::Result::Ok(self)
        }
        ///Sets the value of has_position to the given value.
        ///
        ///#Panics
        ///
        ///If the given value is out of bounds for has_position.
        #[inline]
        #[allow(dead_code)]
        fn set_has_position(&mut self, new_val: <bool as ::modular_bitfield::Specifier>::InOut) {
            self.set_has_position_checked(new_val)
                .expect("value out of bounds for field CompressionFlags.has_position")
        }
        ///Sets the value of has_position to the given value.
        ///
        ///#Errors
        ///
        ///If the given value is out of bounds for has_position.
        #[inline]
        fn set_has_position_checked(
            &mut self,
            new_val: <bool as ::modular_bitfield::Specifier>::InOut,
        ) -> ::core::result::Result<(), ::modular_bitfield::error::OutOfBounds> {
            let __bf_base_bits: ::core::primitive::usize =
                8usize * ::core::mem::size_of::<<bool as ::modular_bitfield::Specifier>::Bytes>();
            let __bf_max_value: <bool as ::modular_bitfield::Specifier>::Bytes =
                { !0 >> (__bf_base_bits - <bool as ::modular_bitfield::Specifier>::BITS) };
            let __bf_spec_bits: ::core::primitive::usize =
                <bool as ::modular_bitfield::Specifier>::BITS;
            let __bf_raw_val: <bool as ::modular_bitfield::Specifier>::Bytes =
                { <bool as ::modular_bitfield::Specifier>::into_bytes(new_val) }?;
            if !(__bf_base_bits == __bf_spec_bits || __bf_raw_val <= __bf_max_value) {
                return ::core::result::Result::Err(::modular_bitfield::error::OutOfBounds);
            }
            ::modular_bitfield::private::write_specifier::<bool>(
                &mut self.bytes[..],
                0usize
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <bool as ::modular_bitfield::Specifier>::BITS
                    + <bool as ::modular_bitfield::Specifier>::BITS,
                __bf_raw_val,
            );
            ::core::result::Result::Ok(())
        }
    }
    impl ::core::fmt::Debug for CompressionFlags {
        fn fmt(&self, __bf_f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            __bf_f
                .debug_struct("CompressionFlags")
                .field(
                    "has_scale",
                    self.has_scale_or_err()
                        .as_ref()
                        .map(|__bf_field| __bf_field as &dyn ::core::fmt::Debug)
                        .unwrap_or_else(|__bf_err| __bf_err as &dyn ::core::fmt::Debug),
                )
                .field(
                    "has_compensate_scale",
                    self.has_compensate_scale_or_err()
                        .as_ref()
                        .map(|__bf_field| __bf_field as &dyn ::core::fmt::Debug)
                        .unwrap_or_else(|__bf_err| __bf_err as &dyn ::core::fmt::Debug),
                )
                .field(
                    "has_rotation",
                    self.has_rotation_or_err()
                        .as_ref()
                        .map(|__bf_field| __bf_field as &dyn ::core::fmt::Debug)
                        .unwrap_or_else(|__bf_err| __bf_err as &dyn ::core::fmt::Debug),
                )
                .field(
                    "has_position",
                    self.has_position_or_err()
                        .as_ref()
                        .map(|__bf_field| __bf_field as &dyn ::core::fmt::Debug)
                        .unwrap_or_else(|__bf_err| __bf_err as &dyn ::core::fmt::Debug),
                )
                .finish()
        }
    }
    impl SsbhWrite for CompressionFlags {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr <= current_pos {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            writer.write_all(&self.into_bytes())?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            self.into_bytes().len() as u64
        }
    }
    pub struct TextureData {
        pub unk1: f32,
        pub unk2: f32,
        pub unk3: f32,
        pub unk4: f32,
        pub unk5: f32,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TextureData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TextureData {
                    unk1: ref __self_0_0,
                    unk2: ref __self_0_1,
                    unk3: ref __self_0_2,
                    unk4: ref __self_0_3,
                    unk5: ref __self_0_4,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "TextureData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unk1",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unk2",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unk3",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unk4",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unk5",
                        &&(*__self_0_4),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for TextureData {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_unk1 = ();
                let __binread_generated_options_unk1 = __binread_generated_var_options;
                let mut unk1: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_unk1,
                    __binread_generated_args_unk1.clone(),
                )?;
                let __binread_generated_args_unk2 = ();
                let __binread_generated_options_unk2 = __binread_generated_var_options;
                let mut unk2: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_unk2,
                    __binread_generated_args_unk2.clone(),
                )?;
                let __binread_generated_args_unk3 = ();
                let __binread_generated_options_unk3 = __binread_generated_var_options;
                let mut unk3: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_unk3,
                    __binread_generated_args_unk3.clone(),
                )?;
                let __binread_generated_args_unk4 = ();
                let __binread_generated_options_unk4 = __binread_generated_var_options;
                let mut unk4: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_unk4,
                    __binread_generated_args_unk4.clone(),
                )?;
                let __binread_generated_args_unk5 = ();
                let __binread_generated_options_unk5 = __binread_generated_var_options;
                let mut unk5: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_unk5,
                    __binread_generated_args_unk5.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut unk1,
                    __binread_generated_var_reader,
                    __binread_generated_options_unk1,
                    __binread_generated_args_unk1.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut unk2,
                    __binread_generated_var_reader,
                    __binread_generated_options_unk2,
                    __binread_generated_args_unk2.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut unk3,
                    __binread_generated_var_reader,
                    __binread_generated_options_unk3,
                    __binread_generated_args_unk3.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut unk4,
                    __binread_generated_var_reader,
                    __binread_generated_options_unk4,
                    __binread_generated_args_unk4.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut unk5,
                    __binread_generated_var_reader,
                    __binread_generated_options_unk5,
                    __binread_generated_args_unk5.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    unk1,
                    unk2,
                    unk3,
                    unk4,
                    unk5,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ssbh_write::SsbhWrite for TextureData {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.unk1.ssbh_write(writer, data_ptr)?;
            self.unk2.ssbh_write(writer, data_ptr)?;
            self.unk3.ssbh_write(writer, data_ptr)?;
            self.unk4.ssbh_write(writer, data_ptr)?;
            self.unk5.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.unk1.size_in_bytes();
            size += self.unk2.size_in_bytes();
            size += self.unk3.size_in_bytes();
            size += self.unk4.size_in_bytes();
            size += self.unk5.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    pub struct Transform {
        pub scale: Vector3,
        pub rotation: Vector4,
        pub translation: Vector3,
        pub compensate_scale: f32,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Transform {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Transform {
                    scale: ref __self_0_0,
                    rotation: ref __self_0_1,
                    translation: ref __self_0_2,
                    compensate_scale: ref __self_0_3,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Transform");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "scale",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "rotation",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "translation",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "compensate_scale",
                        &&(*__self_0_3),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for Transform {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_scale = ();
                let __binread_generated_options_scale = __binread_generated_var_options;
                let mut scale: Vector3 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_scale,
                    __binread_generated_args_scale.clone(),
                )?;
                let __binread_generated_args_rotation = ();
                let __binread_generated_options_rotation = __binread_generated_var_options;
                let mut rotation: Vector4 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_rotation,
                    __binread_generated_args_rotation.clone(),
                )?;
                let __binread_generated_args_translation = ();
                let __binread_generated_options_translation = __binread_generated_var_options;
                let mut translation: Vector3 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_translation,
                    __binread_generated_args_translation.clone(),
                )?;
                let __binread_generated_args_compensate_scale = ();
                let __binread_generated_options_compensate_scale = __binread_generated_var_options;
                let mut compensate_scale: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_compensate_scale,
                    __binread_generated_args_compensate_scale.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut scale,
                    __binread_generated_var_reader,
                    __binread_generated_options_scale,
                    __binread_generated_args_scale.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut rotation,
                    __binread_generated_var_reader,
                    __binread_generated_options_rotation,
                    __binread_generated_args_rotation.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut translation,
                    __binread_generated_var_reader,
                    __binread_generated_options_translation,
                    __binread_generated_args_translation.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut compensate_scale,
                    __binread_generated_var_reader,
                    __binread_generated_options_compensate_scale,
                    __binread_generated_args_compensate_scale.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    scale,
                    rotation,
                    translation,
                    compensate_scale,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ::core::marker::StructuralPartialEq for Transform {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for Transform {
        #[inline]
        fn eq(&self, other: &Transform) -> bool {
            match *other {
                Transform {
                    scale: ref __self_1_0,
                    rotation: ref __self_1_1,
                    translation: ref __self_1_2,
                    compensate_scale: ref __self_1_3,
                } => match *self {
                    Transform {
                        scale: ref __self_0_0,
                        rotation: ref __self_0_1,
                        translation: ref __self_0_2,
                        compensate_scale: ref __self_0_3,
                    } => {
                        (*__self_0_0) == (*__self_1_0)
                            && (*__self_0_1) == (*__self_1_1)
                            && (*__self_0_2) == (*__self_1_2)
                            && (*__self_0_3) == (*__self_1_3)
                    }
                },
            }
        }
        #[inline]
        fn ne(&self, other: &Transform) -> bool {
            match *other {
                Transform {
                    scale: ref __self_1_0,
                    rotation: ref __self_1_1,
                    translation: ref __self_1_2,
                    compensate_scale: ref __self_1_3,
                } => match *self {
                    Transform {
                        scale: ref __self_0_0,
                        rotation: ref __self_0_1,
                        translation: ref __self_0_2,
                        compensate_scale: ref __self_0_3,
                    } => {
                        (*__self_0_0) != (*__self_1_0)
                            || (*__self_0_1) != (*__self_1_1)
                            || (*__self_0_2) != (*__self_1_2)
                            || (*__self_0_3) != (*__self_1_3)
                    }
                },
            }
        }
    }
    impl ssbh_write::SsbhWrite for Transform {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.scale.ssbh_write(writer, data_ptr)?;
            self.rotation.ssbh_write(writer, data_ptr)?;
            self.translation.ssbh_write(writer, data_ptr)?;
            self.compensate_scale.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.scale.size_in_bytes();
            size += self.rotation.size_in_bytes();
            size += self.translation.size_in_bytes();
            size += self.compensate_scale.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    struct ConstantTransform {
        pub scale: Vector3,
        pub rotation: Vector4,
        pub translation: Vector3,
        pub compensate_scale: u32,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ConstantTransform {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                ConstantTransform {
                    scale: ref __self_0_0,
                    rotation: ref __self_0_1,
                    translation: ref __self_0_2,
                    compensate_scale: ref __self_0_3,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "ConstantTransform");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "scale",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "rotation",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "translation",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "compensate_scale",
                        &&(*__self_0_3),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for ConstantTransform {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_scale = ();
                let __binread_generated_options_scale = __binread_generated_var_options;
                let mut scale: Vector3 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_scale,
                    __binread_generated_args_scale.clone(),
                )?;
                let __binread_generated_args_rotation = ();
                let __binread_generated_options_rotation = __binread_generated_var_options;
                let mut rotation: Vector4 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_rotation,
                    __binread_generated_args_rotation.clone(),
                )?;
                let __binread_generated_args_translation = ();
                let __binread_generated_options_translation = __binread_generated_var_options;
                let mut translation: Vector3 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_translation,
                    __binread_generated_args_translation.clone(),
                )?;
                let __binread_generated_args_compensate_scale = ();
                let __binread_generated_options_compensate_scale = __binread_generated_var_options;
                let mut compensate_scale: u32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_compensate_scale,
                    __binread_generated_args_compensate_scale.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut scale,
                    __binread_generated_var_reader,
                    __binread_generated_options_scale,
                    __binread_generated_args_scale.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut rotation,
                    __binread_generated_var_reader,
                    __binread_generated_options_rotation,
                    __binread_generated_args_rotation.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut translation,
                    __binread_generated_var_reader,
                    __binread_generated_options_translation,
                    __binread_generated_args_translation.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut compensate_scale,
                    __binread_generated_var_reader,
                    __binread_generated_options_compensate_scale,
                    __binread_generated_args_compensate_scale.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    scale,
                    rotation,
                    translation,
                    compensate_scale,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ::core::marker::StructuralPartialEq for ConstantTransform {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for ConstantTransform {
        #[inline]
        fn eq(&self, other: &ConstantTransform) -> bool {
            match *other {
                ConstantTransform {
                    scale: ref __self_1_0,
                    rotation: ref __self_1_1,
                    translation: ref __self_1_2,
                    compensate_scale: ref __self_1_3,
                } => match *self {
                    ConstantTransform {
                        scale: ref __self_0_0,
                        rotation: ref __self_0_1,
                        translation: ref __self_0_2,
                        compensate_scale: ref __self_0_3,
                    } => {
                        (*__self_0_0) == (*__self_1_0)
                            && (*__self_0_1) == (*__self_1_1)
                            && (*__self_0_2) == (*__self_1_2)
                            && (*__self_0_3) == (*__self_1_3)
                    }
                },
            }
        }
        #[inline]
        fn ne(&self, other: &ConstantTransform) -> bool {
            match *other {
                ConstantTransform {
                    scale: ref __self_1_0,
                    rotation: ref __self_1_1,
                    translation: ref __self_1_2,
                    compensate_scale: ref __self_1_3,
                } => match *self {
                    ConstantTransform {
                        scale: ref __self_0_0,
                        rotation: ref __self_0_1,
                        translation: ref __self_0_2,
                        compensate_scale: ref __self_0_3,
                    } => {
                        (*__self_0_0) != (*__self_1_0)
                            || (*__self_0_1) != (*__self_1_1)
                            || (*__self_0_2) != (*__self_1_2)
                            || (*__self_0_3) != (*__self_1_3)
                    }
                },
            }
        }
    }
    impl ssbh_write::SsbhWrite for ConstantTransform {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.scale.ssbh_write(writer, data_ptr)?;
            self.rotation.ssbh_write(writer, data_ptr)?;
            self.translation.ssbh_write(writer, data_ptr)?;
            self.compensate_scale.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.scale.size_in_bytes();
            size += self.rotation.size_in_bytes();
            size += self.translation.size_in_bytes();
            size += self.compensate_scale.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    impl From<ConstantTransform> for Transform {
        fn from(value: ConstantTransform) -> Self {
            Self {
                scale: value.scale,
                rotation: value.rotation,
                translation: value.translation,
                compensate_scale: value.compensate_scale as f32,
            }
        }
    }
    struct FloatCompression {
        pub min: f32,
        pub max: f32,
        pub bit_count: u64,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for FloatCompression {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                FloatCompression {
                    min: ref __self_0_0,
                    max: ref __self_0_1,
                    bit_count: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "FloatCompression");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "min",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "max",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "bit_count",
                        &&(*__self_0_2),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for FloatCompression {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_min = ();
                let __binread_generated_options_min = __binread_generated_var_options;
                let mut min: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_min,
                    __binread_generated_args_min.clone(),
                )?;
                let __binread_generated_args_max = ();
                let __binread_generated_options_max = __binread_generated_var_options;
                let mut max: f32 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_max,
                    __binread_generated_args_max.clone(),
                )?;
                let __binread_generated_args_bit_count = ();
                let __binread_generated_options_bit_count = __binread_generated_var_options;
                let mut bit_count: u64 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_bit_count,
                    __binread_generated_args_bit_count.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut min,
                    __binread_generated_var_reader,
                    __binread_generated_options_min,
                    __binread_generated_args_min.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut max,
                    __binread_generated_var_reader,
                    __binread_generated_options_max,
                    __binread_generated_args_max.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut bit_count,
                    __binread_generated_var_reader,
                    __binread_generated_options_bit_count,
                    __binread_generated_args_bit_count.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    min,
                    max,
                    bit_count,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for FloatCompression {
        #[inline]
        fn clone(&self) -> FloatCompression {
            match *self {
                FloatCompression {
                    min: ref __self_0_0,
                    max: ref __self_0_1,
                    bit_count: ref __self_0_2,
                } => FloatCompression {
                    min: ::core::clone::Clone::clone(&(*__self_0_0)),
                    max: ::core::clone::Clone::clone(&(*__self_0_1)),
                    bit_count: ::core::clone::Clone::clone(&(*__self_0_2)),
                },
            }
        }
    }
    impl ssbh_write::SsbhWrite for FloatCompression {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.min.ssbh_write(writer, data_ptr)?;
            self.max.ssbh_write(writer, data_ptr)?;
            self.bit_count.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.min.size_in_bytes();
            size += self.max.size_in_bytes();
            size += self.bit_count.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    struct Vector3Compression {
        pub x: FloatCompression,
        pub y: FloatCompression,
        pub z: FloatCompression,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Vector3Compression {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Vector3Compression {
                    x: ref __self_0_0,
                    y: ref __self_0_1,
                    z: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Vector3Compression");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "x", &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "y", &&(*__self_0_1));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "z", &&(*__self_0_2));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for Vector3Compression {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_x = ();
                let __binread_generated_options_x = __binread_generated_var_options;
                let mut x: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_x,
                    __binread_generated_args_x.clone(),
                )?;
                let __binread_generated_args_y = ();
                let __binread_generated_options_y = __binread_generated_var_options;
                let mut y: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_y,
                    __binread_generated_args_y.clone(),
                )?;
                let __binread_generated_args_z = ();
                let __binread_generated_options_z = __binread_generated_var_options;
                let mut z: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_z,
                    __binread_generated_args_z.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut x,
                    __binread_generated_var_reader,
                    __binread_generated_options_x,
                    __binread_generated_args_x.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut y,
                    __binread_generated_var_reader,
                    __binread_generated_options_y,
                    __binread_generated_args_y.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut z,
                    __binread_generated_var_reader,
                    __binread_generated_options_z,
                    __binread_generated_args_z.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self { x, y, z })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ssbh_write::SsbhWrite for Vector3Compression {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.x.ssbh_write(writer, data_ptr)?;
            self.y.ssbh_write(writer, data_ptr)?;
            self.z.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.x.size_in_bytes();
            size += self.y.size_in_bytes();
            size += self.z.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    struct Vector4Compression {
        pub x: FloatCompression,
        pub y: FloatCompression,
        pub z: FloatCompression,
        pub w: FloatCompression,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Vector4Compression {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Vector4Compression {
                    x: ref __self_0_0,
                    y: ref __self_0_1,
                    z: ref __self_0_2,
                    w: ref __self_0_3,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Vector4Compression");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "x", &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "y", &&(*__self_0_1));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "z", &&(*__self_0_2));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "w", &&(*__self_0_3));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for Vector4Compression {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_x = ();
                let __binread_generated_options_x = __binread_generated_var_options;
                let mut x: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_x,
                    __binread_generated_args_x.clone(),
                )?;
                let __binread_generated_args_y = ();
                let __binread_generated_options_y = __binread_generated_var_options;
                let mut y: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_y,
                    __binread_generated_args_y.clone(),
                )?;
                let __binread_generated_args_z = ();
                let __binread_generated_options_z = __binread_generated_var_options;
                let mut z: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_z,
                    __binread_generated_args_z.clone(),
                )?;
                let __binread_generated_args_w = ();
                let __binread_generated_options_w = __binread_generated_var_options;
                let mut w: FloatCompression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_w,
                    __binread_generated_args_w.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut x,
                    __binread_generated_var_reader,
                    __binread_generated_options_x,
                    __binread_generated_args_x.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut y,
                    __binread_generated_var_reader,
                    __binread_generated_options_y,
                    __binread_generated_args_y.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut z,
                    __binread_generated_var_reader,
                    __binread_generated_options_z,
                    __binread_generated_args_z.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut w,
                    __binread_generated_var_reader,
                    __binread_generated_options_w,
                    __binread_generated_args_w.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self { x, y, z, w })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ssbh_write::SsbhWrite for Vector4Compression {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.x.ssbh_write(writer, data_ptr)?;
            self.y.ssbh_write(writer, data_ptr)?;
            self.z.ssbh_write(writer, data_ptr)?;
            self.w.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.x.size_in_bytes();
            size += self.y.size_in_bytes();
            size += self.z.size_in_bytes();
            size += self.w.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    struct TransformCompression {
        pub scale: Vector3Compression,
        pub rotation: Vector3Compression,
        pub translation: Vector3Compression,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TransformCompression {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TransformCompression {
                    scale: ref __self_0_0,
                    rotation: ref __self_0_1,
                    translation: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "TransformCompression");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "scale",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "rotation",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "translation",
                        &&(*__self_0_2),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for TransformCompression {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_scale = ();
                let __binread_generated_options_scale = __binread_generated_var_options;
                let mut scale: Vector3Compression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_scale,
                    __binread_generated_args_scale.clone(),
                )?;
                let __binread_generated_args_rotation = ();
                let __binread_generated_options_rotation = __binread_generated_var_options;
                let mut rotation: Vector3Compression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_rotation,
                    __binread_generated_args_rotation.clone(),
                )?;
                let __binread_generated_args_translation = ();
                let __binread_generated_options_translation = __binread_generated_var_options;
                let mut translation: Vector3Compression = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_translation,
                    __binread_generated_args_translation.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut scale,
                    __binread_generated_var_reader,
                    __binread_generated_options_scale,
                    __binread_generated_args_scale.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut rotation,
                    __binread_generated_var_reader,
                    __binread_generated_options_rotation,
                    __binread_generated_args_rotation.clone(),
                )?;
                binread::BinRead::after_parse(
                    &mut translation,
                    __binread_generated_var_reader,
                    __binread_generated_options_translation,
                    __binread_generated_args_translation.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self {
                    scale,
                    rotation,
                    translation,
                })
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ssbh_write::SsbhWrite for TransformCompression {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.scale.ssbh_write(writer, data_ptr)?;
            self.rotation.ssbh_write(writer, data_ptr)?;
            self.translation.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.scale.size_in_bytes();
            size += self.rotation.size_in_bytes();
            size += self.translation.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    pub enum TrackData {
        Transform(Vec<Transform>),
        Texture(Vec<TextureData>),
        Float(Vec<f32>),
        PatternIndex(Vec<u32>),
        Boolean(Vec<bool>),
        Vector4(Vec<Vector4>),
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TrackData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&TrackData::Transform(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Transform");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&TrackData::Texture(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Texture");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&TrackData::Float(ref __self_0),) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Float");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&TrackData::PatternIndex(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "PatternIndex");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&TrackData::Boolean(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Boolean");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&TrackData::Vector4(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Vector4");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    trait CompressedData: BinRead<Args = ()> + SsbhWrite {
        type Compression: BinRead<Args = ()> + SsbhWrite;
        fn read_bits(
            header: &CompressedHeader<Self>,
            stream: &mut BitReadStream<LittleEndian>,
            compression: &Self::Compression,
            default: &Self,
        ) -> Self;
    }
    impl CompressedData for Transform {
        type Compression = TransformCompression;
        fn read_bits(
            header: &CompressedHeader<Self>,
            stream: &mut BitReadStream<LittleEndian>,
            compression: &Self::Compression,
            default: &Self,
        ) -> Self {
            read_transform_compressed(header, stream, compression, default)
        }
    }
    impl CompressedData for Vector4 {
        type Compression = Vector4Compression;
        fn read_bits(
            _header: &CompressedHeader<Self>,
            stream: &mut BitReadStream<LittleEndian>,
            compression: &Self::Compression,
            default: &Self,
        ) -> Self {
            read_vector4_compressed(stream, compression, default)
        }
    }
    struct Boolean(u8);
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Boolean {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Boolean(ref __self_0_0) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Boolean");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(non_snake_case)]
    impl binread::BinRead for Boolean {
        type Args = ();
        fn read_options<R: binread::io::Read + binread::io::Seek>(
            __binread_generated_var_reader: &mut R,
            __binread_generated_var_options: &binread::ReadOptions,
            __binread_generated_var_arguments: Self::Args,
        ) -> binread::BinResult<Self> {
            let __binread_generated_position_temp = binread::io::Seek::seek(
                __binread_generated_var_reader,
                binread::io::SeekFrom::Current(0),
            )?;
            (|| {
                let __binread_generated_var_options = __binread_generated_var_options;
                let __binread_generated_args_self_0 = ();
                let __binread_generated_options_self_0 = __binread_generated_var_options;
                let mut self_0: u8 = binread::BinRead::read_options(
                    __binread_generated_var_reader,
                    __binread_generated_options_self_0,
                    __binread_generated_args_self_0.clone(),
                )?;
                let __binread_generated_saved_position = binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Current(0),
                )?;
                binread::BinRead::after_parse(
                    &mut self_0,
                    __binread_generated_var_reader,
                    __binread_generated_options_self_0,
                    __binread_generated_args_self_0.clone(),
                )?;
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_saved_position),
                )?;
                Ok(Self(self_0))
            })()
            .or_else(|error| {
                binread::io::Seek::seek(
                    __binread_generated_var_reader,
                    binread::io::SeekFrom::Start(__binread_generated_position_temp),
                )?;
                Err(error)
            })
        }
    }
    impl ssbh_write::SsbhWrite for Boolean {
        fn ssbh_write<W: std::io::Write + std::io::Seek>(
            &self,
            writer: &mut W,
            data_ptr: &mut u64,
        ) -> std::io::Result<()> {
            let current_pos = writer.stream_position()?;
            if *data_ptr < current_pos + self.size_in_bytes() {
                *data_ptr = current_pos + self.size_in_bytes();
            }
            self.0.ssbh_write(writer, data_ptr)?;
            Ok(())
        }
        fn size_in_bytes(&self) -> u64 {
            let mut size = 0;
            size += self.0.size_in_bytes();
            size
        }
        fn alignment_in_bytes(&self) -> u64 {
            8usize as u64
        }
    }
    impl CompressedData for Boolean {
        type Compression = u128;
        fn read_bits(
            header: &CompressedHeader<Self>,
            stream: &mut BitReadStream<LittleEndian>,
            compression: &Self::Compression,
            default: &Self,
        ) -> Self {
            let value = stream
                .read_int::<u8>(header.bits_per_entry as usize)
                .unwrap();
            Boolean(value)
        }
    }
    fn read_track_data(track_data: &[u8], flags: TrackFlags, frame_count: usize) -> TrackData {
        match (flags.track_type, flags.compression_type) {
            (TrackType::Float, CompressionType::Constant) => {
                let mut reader = Cursor::new(track_data);
                let value: f32 = reader.read_le().unwrap();
                TrackData::Float(<[_]>::into_vec(box [value]))
            }
            (TrackType::Vector4, CompressionType::Compressed) => {
                let mut reader = Cursor::new(track_data);
                let values = read_track_compressed(&mut reader, frame_count);
                TrackData::Vector4(values)
            }
            (TrackType::Vector4, _) => {
                let mut reader = Cursor::new(track_data);
                let mut values = Vec::new();
                for _ in 0..frame_count {
                    let value: Vector4 = reader.read_le().unwrap();
                    values.push(value);
                }
                TrackData::Vector4(values)
            }
            (TrackType::Texture, CompressionType::Constant) => {
                let mut reader = Cursor::new(track_data);
                let value: TextureData = reader.read_le().unwrap();
                TrackData::Texture(<[_]>::into_vec(box [value]))
            }
            (TrackType::PatternIndex, CompressionType::Constant) => {
                let mut reader = Cursor::new(track_data);
                let value: u32 = reader.read_le().unwrap();
                TrackData::PatternIndex(<[_]>::into_vec(box [value]))
            }
            (TrackType::Boolean, CompressionType::Compressed) => {
                let mut reader = Cursor::new(track_data);
                let values: Vec<Boolean> = read_track_compressed(&mut reader, frame_count);
                TrackData::Boolean(values.iter().map(|b| b.0 != 0).collect())
            }
            (TrackType::Boolean, _) => {
                let mut reader = Cursor::new(track_data);
                let mut values = Vec::new();
                for _ in 0..frame_count {
                    let value: Boolean = reader.read_le().unwrap();
                    values.push(value.0 != 0);
                }
                TrackData::Boolean(values)
            }
            (TrackType::Transform, CompressionType::Compressed) => {
                let mut reader = Cursor::new(track_data);
                let values = read_track_compressed(&mut reader, frame_count);
                TrackData::Transform(values)
            }
            (TrackType::Transform, _) => {
                let mut reader = Cursor::new(track_data);
                let mut values = Vec::new();
                for _ in 0..frame_count {
                    let value: ConstantTransform = reader.read_le().unwrap();
                    values.push(value.into());
                }
                TrackData::Transform(values)
            }
            _ => ::std::rt::begin_panic("Unsupported flags"),
        }
    }
    fn write_track_data<W: Write + Seek>(
        writer: &mut W,
        track_data: &TrackData,
        compression: CompressionType,
    ) {
        match compression {
            CompressionType::Direct => ::core::panicking::panic("not yet implemented"),
            CompressionType::ConstTransform => ::core::panicking::panic("not yet implemented"),
            CompressionType::Compressed => match track_data {
                TrackData::Transform(_) => ::core::panicking::panic("not yet implemented"),
                TrackData::Texture(_) => ::core::panicking::panic("not yet implemented"),
                TrackData::Float(_) => ::core::panicking::panic("not yet implemented"),
                TrackData::PatternIndex(_) => ::core::panicking::panic("not yet implemented"),
                TrackData::Boolean(values) => {
                    let mut elements = BitVec::with_capacity(values.len());
                    for value in values {
                        elements.push(*value);
                    }
                    {
                        ::std::io::_print(::core::fmt::Arguments::new_v1(
                            &["", "\n"],
                            &match (&elements,) {
                                (arg0,) => {
                                    [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)]
                                }
                            },
                        ));
                    };
                    let data = CompressedTrackData::<Boolean> {
                        header: CompressedHeader::<Boolean> {
                            unk_4: 4,
                            flags: CompressionFlags::new(),
                            default_data: Ptr16::new(Boolean(0u8)),
                            bits_per_entry: 1,
                            compressed_data: Ptr32::new(CompressedBuffer(elements.to_bytes())),
                            frame_count: values.len() as u32,
                        },
                        compression: 0,
                    };
                    {
                        ::std::io::_print(::core::fmt::Arguments::new_v1(
                            &["", "\n"],
                            &match (&data,) {
                                (arg0,) => {
                                    [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)]
                                }
                            },
                        ));
                    };
                    data.write(writer).unwrap();
                }
                TrackData::Vector4(_) => ::core::panicking::panic("not yet implemented"),
            },
            CompressionType::Constant => ::core::panicking::panic("not yet implemented"),
        }
    }
    fn read_track_compressed<R: Read + Seek, T: CompressedData>(
        reader: &mut R,
        frame_count: usize,
    ) -> Vec<T> {
        let data: CompressedTrackData<T> = reader.read_le().unwrap();
        let bit_buffer =
            BitReadBuffer::new(&data.header.compressed_data.0, bitbuffer::LittleEndian);
        let mut bit_reader = BitReadStream::new(bit_buffer);
        let mut values = Vec::new();
        for _ in 0..frame_count {
            let value = T::read_bits(
                &data.header,
                &mut bit_reader,
                &data.compression,
                &data.header.default_data,
            );
            values.push(value);
        }
        values
    }
    fn read_transform_compressed(
        header: &CompressedHeader<Transform>,
        bit_stream: &mut BitReadStream<LittleEndian>,
        compression: &TransformCompression,
        default: &Transform,
    ) -> Transform {
        let compensate_scale = if header.flags.has_compensate_scale() && header.flags.has_scale() {
            read_compressed_f32(bit_stream, &compression.scale.x).unwrap_or(0.0)
        } else {
            0.0
        };
        let scale = if header.flags.has_scale() {
            read_compressed_vector3(bit_stream, &compression.scale, &default.scale)
        } else {
            default.scale
        };
        let rotation = if header.flags.has_rotation() {
            let default_rotation_xyz =
                Vector3::new(default.rotation.x, default.rotation.y, default.rotation.z);
            let rotation_xyz =
                read_compressed_vector3(bit_stream, &compression.rotation, &default_rotation_xyz);
            Vector4::new(rotation_xyz.x, rotation_xyz.y, rotation_xyz.z, f32::NAN)
        } else {
            default.rotation
        };
        let translation = if header.flags.has_position() {
            read_compressed_vector3(bit_stream, &compression.translation, &default.translation)
        } else {
            default.translation
        };
        let rotation_w = if header.flags.has_rotation() {
            let w_flip = bit_stream.read_bool().unwrap();
            let w = f32::sqrt(
                1.0 - (rotation.x * rotation.x + rotation.y * rotation.y + rotation.z * rotation.z),
            );
            if w_flip {
                -w
            } else {
                w
            }
        } else {
            default.rotation.w
        };
        let rotation = Vector4::new(rotation.x, rotation.y, rotation.z, rotation_w);
        Transform {
            scale,
            rotation,
            translation,
            compensate_scale,
        }
    }
    fn read_vector4_compressed(
        bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
        compression: &Vector4Compression,
        default: &Vector4,
    ) -> Vector4 {
        let x = read_compressed_f32(bit_stream, &compression.x).unwrap_or(default.x);
        let y = read_compressed_f32(bit_stream, &compression.y).unwrap_or(default.y);
        let z = read_compressed_f32(bit_stream, &compression.z).unwrap_or(default.z);
        let w = read_compressed_f32(bit_stream, &compression.w).unwrap_or(default.w);
        Vector4::new(x, y, z, w)
    }
    fn read_compressed_vector3(
        bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
        compression: &Vector3Compression,
        default: &Vector3,
    ) -> Vector3 {
        let x = read_compressed_f32(bit_stream, &compression.x).unwrap_or(default.x);
        let y = read_compressed_f32(bit_stream, &compression.y).unwrap_or(default.y);
        let z = read_compressed_f32(bit_stream, &compression.z).unwrap_or(default.z);
        Vector3::new(x, y, z)
    }
    fn read_compressed_f32(
        bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
        compression: &FloatCompression,
    ) -> Option<f32> {
        let value: u32 = bit_stream.read_int(compression.bit_count as usize).unwrap();
        decompress_f32(
            value,
            compression.min,
            compression.max,
            NonZeroU64::new(compression.bit_count),
        )
    }
    fn bit_mask(bit_count: NonZeroU64) -> u64 {
        (1u64 << bit_count.get()) - 1u64
    }
    fn compress_f32(value: f32, min: f32, max: f32, bit_count: NonZeroU64) -> u32 {
        let scale = bit_mask(bit_count);
        let ratio = (value - min) / (max - min);
        let compressed = ratio * scale as f32;
        compressed as u32
    }
    fn decompress_f32(
        value: u32,
        min: f32,
        max: f32,
        bit_count: Option<NonZeroU64>,
    ) -> Option<f32> {
        let scale = bit_mask(bit_count?);
        let lerp = |a, b, t| a * (1.0 - t) + b * t;
        let value = lerp(min, max, value as f32 / scale as f32);
        Some(value)
    }
}
use std::io::{Read, Write};
use std::ops::Mul;
use binread::io::{Seek, SeekFrom};
use binread::BinReaderExt;
use binread::{BinRead, BinResult};
use half::f16;
use ssbh_lib::SsbhArray;
fn read_data<R: Read + Seek, TIn: BinRead, TOut: From<TIn>>(
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
fn read_vector_data<R: Read + Seek, T: Into<f32> + BinRead, const N: usize>(
    reader: &mut R,
    count: usize,
    offset: u64,
    stride: u64,
) -> BinResult<Vec<[f32; N]>> {
    let mut result = Vec::new();
    for i in 0..count as u64 {
        reader.seek(SeekFrom::Start(offset + i * stride))?;
        let mut element = [0f32; N];
        for e in element.iter_mut() {
            *e = reader.read_le::<T>()?.into();
        }
        result.push(element);
    }
    Ok(result)
}
fn get_u8_clamped(f: f32) -> u8 {
    f.clamp(0.0f32, 1.0f32).mul(255.0f32).round() as u8
}
fn write_f32<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&component.to_le_bytes())?;
    }
    Ok(())
}
fn write_u8<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&[get_u8_clamped(*component)])?;
    }
    Ok(())
}
fn write_f16<W: Write>(writer: &mut W, data: &[f32]) -> std::io::Result<()> {
    for component in data {
        writer.write_all(&f16::from_f32(*component).to_le_bytes())?;
    }
    Ok(())
}
fn write_vector_data<
    W: Write + Seek,
    F: Fn(&mut W, &[f32]) -> std::io::Result<()>,
    const N: usize,
>(
    writer: &mut W,
    elements: &[[f32; N]],
    offset: u64,
    stride: u64,
    write_t: F,
) -> Result<(), std::io::Error> {
    for (i, element) in elements.iter().enumerate() {
        writer.seek(SeekFrom::Start(offset + i as u64 * stride))?;
        write_t(writer, element)?;
    }
    Ok(())
}
fn create_ssbh_array<T, B: BinRead, F: Fn(&T) -> B>(elements: &[T], create_b: F) -> SsbhArray<B> {
    elements.iter().map(create_b).collect::<Vec<B>>().into()
}
