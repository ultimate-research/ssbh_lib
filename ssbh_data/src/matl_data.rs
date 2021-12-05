use std::convert::{TryFrom, TryInto};

pub use ssbh_lib::formats::matl::{
    BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MaxAnisotropy, MinFilter, ParamId,
    WrapMode,
};
use ssbh_lib::formats::matl::{
    MatlBlendStateV16, MatlEntries, MatlRasterizerStateV16, MatlSampler, ParamV16,
};
pub use ssbh_lib::{Color4f, Vector4};
use ssbh_lib::{Matl, SsbhString};
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct ParamData<T> {
    // TODO: Is it worth restricting param id by type?
    // This would prevent creating a Vector4 param with CustomFloat0's ID.
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

// TODO: Should data loss from unsupported fields be an error?
impl From<MatlSampler> for SamplerData {
    fn from(v: MatlSampler) -> Self {
        Self {
            wraps: v.wraps,
            wrapt: v.wrapt,
            wrapr: v.wrapr,
            min_filter: v.min_filter,
            mag_filter: v.mag_filter,
            border_color: v.border_color,
            lod_bias: v.lod_bias,
            // TODO: Differentiate between Default and Default2?
            max_anisotropy: match v.texture_filtering_type {
                FilteringType::AnisotropicFiltering => Some(v.max_anisotropy),
                _ => None,
            },
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct BlendStateData {
    pub source_color: BlendFactor,
    pub destination_color: BlendFactor,
    pub alpha_sample_to_coverage: bool,
}

// TODO: Should data loss from unsupported fields be an error?
impl From<MatlBlendStateV16> for BlendStateData {
    fn from(v: MatlBlendStateV16) -> Self {
        Self {
            source_color: v.source_color,
            destination_color: v.destination_color,
            alpha_sample_to_coverage: v.alpha_sample_to_coverage != 0,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub struct RasterizerStateData {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
}

// TODO: Should data loss from unsupported fields be an error?
impl From<MatlRasterizerStateV16> for RasterizerStateData {
    fn from(v: MatlRasterizerStateV16) -> Self {
        Self {
            fill_mode: v.fill_mode,
            cull_mode: v.cull_mode,
            depth_bias: v.depth_bias,
        }
    }
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
    ($attributes:expr, $ty_in:path) => {
        get_attributes!($attributes, $ty_in, |x| x)
    };
    ($attributes:expr, $ty_in:path, $f_convert:expr) => {
        $attributes
            .as_slice()
            .iter()
            .filter_map(|a| {
                a.param.data.as_ref().map(|param| match param {
                    $ty_in(data) => Some(ParamData {
                        param_id: a.param_id,
                        data: $f_convert(data.clone()),
                    }),
                    _ => None,
                })
            })
            .flatten()
            .collect()
    };
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
                            vectors: get_attributes!(e.attributes.elements, ParamV16::Vector4),
                            floats: get_attributes!(e.attributes.elements, ParamV16::Float),
                            booleans: get_attributes!(
                                e.attributes.elements,
                                ParamV16::Boolean,
                                |x| x != 0
                            ),
                            // TODO: Handle and test the remaining types.
                            textures: get_attributes!(
                                e.attributes.elements,
                                ParamV16::MatlString,
                                |x: &SsbhString| x.to_string_lossy()
                            ),
                            samplers: get_attributes!(
                                e.attributes.elements,
                                ParamV16::Sampler,
                                |x: MatlSampler| x.into()
                            ),
                            blend_states: get_attributes!(
                                e.attributes.elements,
                                ParamV16::BlendState,
                                |x: MatlBlendStateV16| x.into()
                            ),
                            rasterizer_states: get_attributes!(
                                e.attributes.elements,
                                ParamV16::RasterizerState,
                                |x: MatlRasterizerStateV16| x.into()
                            ),
                        }
                    })
                    .collect()),
            }?,
        })
    }
}

#[cfg(test)]
mod tests {
    use ssbh_lib::{
        formats::matl::{MatlAttributeV16, MatlEntryV16},
        RelPtr64, SsbhEnum64,
    };

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
                    attributes: vec![
                        MatlAttributeV16 {
                            param_id: ParamId::CustomVector13,
                            // TODO: Add convenience methods to param to avoid specifying datatype manually?
                            // Specifying the data type like this is error prone.
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::Vector4(Vector4::new(
                                    1.0, 2.0, 3.0, 4.0,
                                ))),
                                data_type: 5,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::CustomFloat5,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::Float(0.5)),
                                data_type: 1,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::CustomBoolean0,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::Boolean(1)),
                                data_type: 2,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::CustomBoolean1,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::Boolean(0)),
                                data_type: 2,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::Texture1,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::MatlString("abc".into())),
                                data_type: 11,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::Sampler0,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::Sampler(MatlSampler {
                                    wraps: WrapMode::ClampToBorder,
                                    wrapt: WrapMode::ClampToEdge,
                                    wrapr: WrapMode::MirroredRepeat,
                                    min_filter: MinFilter::LinearMipmapLinear,
                                    mag_filter: MagFilter::Nearest,
                                    texture_filtering_type: FilteringType::AnisotropicFiltering,
                                    border_color: Color4f {
                                        r: 1.0,
                                        g: 1.0,
                                        b: 3.0,
                                        a: 4.0,
                                    },
                                    unk11: 0,
                                    unk12: 0,
                                    lod_bias: -1.0,
                                    max_anisotropy: MaxAnisotropy::Four,
                                })),
                                data_type: 14,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::BlendState0,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::BlendState(MatlBlendStateV16 {
                                    source_color: BlendFactor::DestinationColor,
                                    unk2: 0,
                                    destination_color: BlendFactor::One,
                                    unk4: 0,
                                    unk5: 0,
                                    unk6: 0,
                                    alpha_sample_to_coverage: 1,
                                    unk8: 0,
                                    unk9: 0,
                                    unk10: 0,
                                })),
                                data_type: 17,
                            },
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::RasterizerState0,
                            param: SsbhEnum64 {
                                data: RelPtr64::new(ParamV16::RasterizerState(
                                    MatlRasterizerStateV16 {
                                        fill_mode: FillMode::Solid,
                                        cull_mode: CullMode::Front,
                                        depth_bias: -5.0,
                                        unk4: 0.0,
                                        unk5: 0.0,
                                        unk6: 0,
                                    },
                                )),
                                data_type: 18,
                            },
                        },
                    ]
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
                material_label: "a".into(),
                shader_label: "b".into(),
                vectors: vec![ParamData {
                    param_id: ParamId::CustomVector13,
                    data: Vector4::new(1.0, 2.0, 3.0, 4.0,)
                }],
                floats: vec![ParamData {
                    param_id: ParamId::CustomFloat5,
                    data: 0.5
                }],
                booleans: vec![
                    ParamData {
                        param_id: ParamId::CustomBoolean0,
                        data: true
                    },
                    ParamData {
                        param_id: ParamId::CustomBoolean1,
                        data: false
                    }
                ],
                textures: vec![ParamData {
                    param_id: ParamId::Texture1,
                    data: "abc".into()
                }],
                samplers: vec![ParamData {
                    param_id: ParamId::Sampler0,
                    data: SamplerData {
                        wraps: WrapMode::ClampToBorder,
                        wrapt: WrapMode::ClampToEdge,
                        wrapr: WrapMode::MirroredRepeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Nearest,
                        border_color: Color4f {
                            r: 1.0,
                            g: 1.0,
                            b: 3.0,
                            a: 4.0
                        },
                        lod_bias: -1.0,
                        max_anisotropy: Some(MaxAnisotropy::Four),
                    }
                }],
                blend_states: vec![ParamData {
                    param_id: ParamId::BlendState0,
                    data: BlendStateData {
                        source_color: BlendFactor::DestinationColor,
                        destination_color: BlendFactor::One,
                        alpha_sample_to_coverage: true,
                    }
                }],
                rasterizer_states: vec![ParamData {
                    param_id: ParamId::RasterizerState0,
                    data: RasterizerStateData {
                        fill_mode: FillMode::Solid,
                        cull_mode: CullMode::Front,
                        depth_bias: -5.0,
                    }
                }]
            }],
            data.entries
        );
    }
}
