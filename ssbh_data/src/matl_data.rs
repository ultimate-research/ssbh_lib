use std::convert::{TryFrom, TryInto};

pub use ssbh_lib::formats::matl::{
    BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MaxAnisotropy, MinFilter, ParamId,
    WrapMode,
};
use ssbh_lib::formats::matl::{MatlAttributeV16, MatlEntries, MatlEntryV16, ParamV16};
use ssbh_lib::Matl;
pub use ssbh_lib::{Color4f, Vector4};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::SsbhData;

/// Errors while creating a [Matl] from [MatlData].
#[derive(Error, Debug)]

pub enum MatlError {
    /// Creating a [Matl] file for the given version is not supported.
    #[error(
        "Creating a version {}.{} matl is not supported.",
        major_version,
        minor_version
    )]
    UnsupportedVersion {
        major_version: u16,
        minor_version: u16,
    },
}

/// The data associated with a [Matl] file.
/// The supported version is 1.6.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct MatlData {
    pub major_version: u16,
    pub minor_version: u16,
    pub entries: Vec<MatlEntryData>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct MatlEntryData {
    pub material_label: String,
    pub shader_label: String,
    pub vectors: Vec<ParamData<Vector4>>,
    pub floats: Vec<ParamData<f32>>,
    pub booleans: Vec<ParamData<bool>>,
    pub textures: Vec<ParamData<String>>,
    pub samplers: Vec<ParamData<SamplerData>>,
    pub blend_states: Vec<ParamData<BlendStateData>>,
    pub rasterizer_states: Vec<ParamData<RasterizerStateData>>,
    // TODO: UV Transform?
}

// TODO: Type aliases to make this easier to type like Vector4Param instead of ParamData::<Vector4>?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ParamData<T> {
    param_id: ParamId,
    data: T,
}

// TODO: Derive default for these types to make them easier to use.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct SamplerData {
    pub wraps: WrapMode,
    pub wrapt: WrapMode,
    pub wrapr: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    /// The color when sampling texture coordinates outside the 0 to 1 range.
    /// This only applies to [WrapMode::ClampToBorder].
    pub border_color: Color4f,
    pub lod_bias: f32,
    /// The amount of anisotropic filtering to used.
    /// A value of [None] disables anisotropic filtering.
    pub max_anisotropy: Option<MaxAnisotropy>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct BlendStateData {
    pub source_color: BlendFactor,
    pub destination_color: BlendFactor,
    pub alpha_sample_to_coverage: bool,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct RasterizerStateData {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
}

impl SsbhData for MatlData {
    type WriteError = MatlError;

    fn from_file<P: AsRef<std::path::Path>>(_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }

    fn read<R: std::io::Read + std::io::Seek>(
        _reader: &mut R,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
    ) -> Result<(), Self::WriteError> {
        todo!()
    }

    fn write_to_file<P: AsRef<std::path::Path>>(&self, _path: P) -> Result<(), Self::WriteError> {
        todo!()
    }
}

// It may be possible to filter a specified enum variant without a macro in the future.
macro_rules! get_attributes {
    ($iter:expr, $ty_in:path, $ty_out:ty) => {
        $iter
            .filter_map(|a| {
                a.param.data.as_ref().map(|param| match param {
                    $ty_in(data) => Some(ParamData::<$ty_out> {
                        param_id: a.param_id,
                        data: *data,
                    }),
                    _ => None,
                })
            })
            .flatten()
            .collect()
    };
}

fn get_vectors(attributes: &[MatlAttributeV16]) -> Vec<ParamData<Vector4>> {
    get_attributes!(attributes.iter(), ParamV16::Vector4, Vector4)
}

impl TryFrom<Matl> for MatlData {
    type Error = MatlError;

    fn try_from(value: Matl) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl TryFrom<&Matl> for MatlData {
    type Error = MatlError;

    // TODO: This should fail for version 1.5?
    fn try_from(data: &Matl) -> Result<Self, Self::Error> {
        Ok(Self {
            major_version: data.major_version,
            minor_version: data.minor_version,
            entries: match &data.entries {
                MatlEntries::EntriesV15(_) => Err(MatlError::UnsupportedVersion {
                    major_version: 1,
                    minor_version: 5,
                }),
                MatlEntries::EntriesV16(entries) => Ok(entries
                    .elements
                    .iter()
                    .map(|e| {
                        MatlEntryData {
                            material_label: e.material_label.to_string_lossy(),
                            shader_label: e.shader_label.to_string_lossy(),
                            vectors: get_vectors(&e.attributes.elements),
                            // TODO: Handle and test the remaining types.
                            floats: Vec::new(),
                            booleans: Vec::new(),
                            textures: Vec::new(),
                            samplers: Vec::new(),
                            blend_states: Vec::new(),
                            rasterizer_states: Vec::new(),
                        }
                    })
                    .collect()),
            }?,
        })
    }
}

#[cfg(test)]
mod tests {
    use ssbh_lib::{formats::matl::MatlAttributeV16, RelPtr64, SsbhEnum64};

    use super::*;

    // TODO: Test both directions for conversions.
    // TODO: Test the supported versions
    // TODO: Test the order for saved parameters.
    #[test]
    fn create_empty_matl_data_1_5() {
        let result = MatlData::try_from(&Matl {
            major_version: 1,
            minor_version: 5,
            entries: MatlEntries::EntriesV15(Vec::new().into()),
        });

        assert!(matches!(
            result,
            Err(MatlError::UnsupportedVersion {
                major_version: 1,
                minor_version: 5
            })
        ));
    }

    #[test]
    fn create_empty_matl_data_1_6() {
        let data = MatlData::try_from(&Matl {
            major_version: 1,
            minor_version: 6,
            entries: MatlEntries::EntriesV16(Vec::new().into()),
        })
        .unwrap();

        assert_eq!(1, data.major_version);
        assert_eq!(6, data.minor_version);
        assert!(data.entries.is_empty());
    }

    #[test]
    fn create_matl_data_single_entry() {
        let data = MatlData::try_from(&Matl {
            major_version: 1,
            minor_version: 6,
            entries: MatlEntries::EntriesV16(
                vec![MatlEntryV16 {
                    material_label: "a".into(),
                    attributes: vec![MatlAttributeV16 {
                        param_id: ParamId::CustomVector13,
                        // TODO: Add convenience methods to param to avoid specifying datatype manually?
                        // Specifying the data type like this is error prone.
                        param: SsbhEnum64 {
                            data: RelPtr64::new(ParamV16::Vector4(Vector4::new(
                                1.0, 2.0, 3.0, 4.0,
                            ))),
                            data_type: 5,
                        },
                    }]
                    .into(),

                    shader_label: "b".into(),
                }]
                .into(),
            ),
        })
        .unwrap();

        assert_eq!(1, data.major_version);
        assert_eq!(6, data.minor_version);
        assert_eq!(
            vec![MatlEntryData {
                // TODO: Test conversions for all parameter types.
                material_label: "a".into(),
                shader_label: "b".into(),
                vectors: vec![ParamData::<Vector4> {
                    param_id: ParamId::CustomVector13,
                    data: Vector4::new(1.0, 2.0, 3.0, 4.0,)
                }],
                floats: Vec::new(),
                booleans: Vec::new(),
                textures: Vec::new(),
                samplers: Vec::new(),
                blend_states: Vec::new(),
                rasterizer_states: Vec::new()
            }],
            data.entries
        );
    }
}
