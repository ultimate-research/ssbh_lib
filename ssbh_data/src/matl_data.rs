use std::{
    convert::{TryFrom, TryInto},
    io::{Read, Seek, Write},
};

use itertools::Itertools;
pub use ssbh_lib::formats::matl::{
    BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MaxAnisotropy, MinFilter, ParamId,
    WrapMode,
};
use ssbh_lib::{
    formats::matl::{
        MatlAttributeV16, MatlBlendStateV16, MatlEntries, MatlEntryV16, MatlRasterizerStateV16,
        MatlSampler, ParamV16,
    },
    RelPtr64, SsbhEnum64,
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

    /// An error occurred while writing data.
    #[error(transparent)]
    Io(#[from] std::io::Error),
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
    pub blend_states: Vec<ParamData<BlendStateData>>,
    pub floats: Vec<ParamData<f32>>,
    pub booleans: Vec<ParamData<bool>>,
    pub vectors: Vec<ParamData<Vector4>>,
    pub rasterizer_states: Vec<ParamData<RasterizerStateData>>,
    pub samplers: Vec<ParamData<SamplerData>>,
    pub textures: Vec<ParamData<String>>,
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

impl From<SamplerData> for MatlSampler {
    fn from(v: SamplerData) -> Self {
        Self::from(&v)
    }
}

impl From<&SamplerData> for MatlSampler {
    fn from(v: &SamplerData) -> Self {
        Self {
            wraps: v.wraps,
            wrapt: v.wrapt,
            wrapr: v.wrapr,
            min_filter: v.min_filter,
            mag_filter: v.mag_filter,
            border_color: v.border_color,
            lod_bias: v.lod_bias,
            max_anisotropy: v.max_anisotropy.unwrap_or(MaxAnisotropy::One),
            // TODO: Differentiate between Default and Default2?
            texture_filtering_type: match v.max_anisotropy {
                Some(_) => FilteringType::AnisotropicFiltering,
                None => FilteringType::Default,
            },
            unk11: 0,
            unk12: 0,
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
        Self::from(&v)
    }
}

impl From<&MatlBlendStateV16> for BlendStateData {
    fn from(v: &MatlBlendStateV16) -> Self {
        Self {
            source_color: v.source_color,
            destination_color: v.destination_color,
            alpha_sample_to_coverage: v.alpha_sample_to_coverage != 0,
        }
    }
}

impl From<BlendStateData> for MatlBlendStateV16 {
    fn from(v: BlendStateData) -> Self {
        Self::from(&v)
    }
}

impl From<&BlendStateData> for MatlBlendStateV16 {
    fn from(v: &BlendStateData) -> Self {
        Self {
            source_color: v.source_color,
            unk2: 0,
            destination_color: v.destination_color,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            alpha_sample_to_coverage: if v.alpha_sample_to_coverage { 1 } else { 0 },
            unk8: 0,
            unk9: 0,
            unk10: 0,
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
        Self::from(&v)
    }
}

impl From<&MatlRasterizerStateV16> for RasterizerStateData {
    fn from(v: &MatlRasterizerStateV16) -> Self {
        Self {
            fill_mode: v.fill_mode,
            cull_mode: v.cull_mode,
            depth_bias: v.depth_bias,
        }
    }
}

impl From<RasterizerStateData> for MatlRasterizerStateV16 {
    fn from(v: RasterizerStateData) -> Self {
        Self::from(&v)
    }
}

impl From<&RasterizerStateData> for MatlRasterizerStateV16 {
    fn from(v: &RasterizerStateData) -> Self {
        Self {
            fill_mode: v.fill_mode,
            cull_mode: v.cull_mode,
            depth_bias: v.depth_bias,
            unk4: 0.0,
            unk5: 0.0,
            unk6: 0,
        }
    }
}

impl SsbhData for MatlData {
    type WriteError = MatlError;

    fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Matl::from_file(path)?.try_into().map_err(Into::into)
    }

    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        Matl::read(reader)?.try_into().map_err(Into::into)
    }

    fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Self::WriteError> {
        Matl::try_from(self)?.write(writer).map_err(Into::into)
    }

    fn write_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Self::WriteError> {
        Matl::try_from(self)?
            .write_to_file(path)
            .map_err(Into::into)
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
        Self::try_from(&value)
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
                MatlEntries::EntriesV16(entries) => {
                    Ok(entries.elements.iter().map(Into::into).collect())
                }
            }?,
        })
    }
}

impl TryFrom<MatlData> for Matl {
    type Error = MatlError;

    fn try_from(value: MatlData) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&MatlData> for Matl {
    type Error = MatlError;

    fn try_from(value: &MatlData) -> Result<Self, Self::Error> {
        // TODO: Errors on unsupported versions?
        Ok(Self {
            major_version: value.major_version,
            minor_version: value.minor_version,
            entries: MatlEntries::EntriesV16(
                value.entries.iter().map(Into::into).collect_vec().into(),
            ),
        })
    }
}

impl From<&MatlEntryV16> for MatlEntryData {
    fn from(e: &MatlEntryV16) -> Self {
        Self {
            material_label: e.material_label.to_string_lossy(),
            shader_label: e.shader_label.to_string_lossy(),
            vectors: get_attributes!(e.attributes.elements, ParamV16::Vector4),
            floats: get_attributes!(e.attributes.elements, ParamV16::Float),
            booleans: get_attributes!(e.attributes.elements, ParamV16::Boolean, |x| x != 0),
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
    }
}

/*
blendstate
floats
boolean
vector
rasterizer
sampler
texture
 */

impl From<&MatlEntryData> for MatlEntryV16 {
    fn from(e: &MatlEntryData) -> Self {
        Self {
            // TODO: Add tests for parameter ordering from Smash Ultimate materials?
            material_label: e.material_label.as_str().into(), // TODO: add From<&String> for SsbhString?
            // TODO: Is there a standard ordering for attributes?
            attributes: e
                .blend_states
                .iter()
                .map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                })
                .chain(e.floats.iter().map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                }))
                .chain(e.booleans.iter().map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                }))
                .chain(e.vectors.iter().map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                }))
                .chain(e.rasterizer_states.iter().map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                }))
                .chain(e.samplers.iter().map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                }))
                .chain(e.textures.iter().map(|a| MatlAttributeV16 {
                    param_id: a.param_id,
                    param: a.data.to_param(),
                }))
                .collect_vec()
                .into(),
            shader_label: e.shader_label.as_str().into(),
        }
    }
}

// TODO: Automatically generate this code somehow?
trait ToParam {
    fn to_param(&self) -> SsbhEnum64<ParamV16>;
}

impl ToParam for Vector4 {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            data: RelPtr64::new(ParamV16::Vector4(*self)),
            data_type: 5,
        }
    }
}

impl ToParam for f32 {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            data: RelPtr64::new(ParamV16::Float(*self)),
            data_type: 1,
        }
    }
}

impl ToParam for bool {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            data: RelPtr64::new(ParamV16::Boolean(if *self { 1 } else { 0 })),
            data_type: 2,
        }
    }
}

impl ToParam for String {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        self.as_str().to_param()
    }
}

impl ToParam for &str {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            // TODO: Avoid the as_str call here?
            data: RelPtr64::new(ParamV16::MatlString((*self).into())),
            data_type: 11,
        }
    }
}

impl ToParam for MatlSampler {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            data: RelPtr64::new(ParamV16::Sampler(self.clone())),
            data_type: 14,
        }
    }
}

impl ToParam for SamplerData {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        MatlSampler::from(self).to_param()
    }
}

impl ToParam for MatlBlendStateV16 {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            data: RelPtr64::new(ParamV16::BlendState(self.clone())),
            data_type: 17,
        }
    }
}

impl ToParam for BlendStateData {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        MatlBlendStateV16::from(self).to_param()
    }
}

impl ToParam for MatlRasterizerStateV16 {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        SsbhEnum64 {
            data: RelPtr64::new(ParamV16::RasterizerState(self.clone())),
            data_type: 18,
        }
    }
}

impl ToParam for RasterizerStateData {
    fn to_param(&self) -> SsbhEnum64<ParamV16> {
        MatlRasterizerStateV16::from(self).to_param()
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
                            param: Vector4::new(1.0, 2.0, 3.0, 4.0).to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::CustomFloat5,
                            param: 0.5.to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::CustomBoolean0,
                            param: true.to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::CustomBoolean1,
                            param: false.to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::Texture1,
                            param: "abc".to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::Sampler0,
                            param: MatlSampler {
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
                            }
                            .to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::BlendState0,
                            param: MatlBlendStateV16 {
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
                            }
                            .to_param(),
                        },
                        MatlAttributeV16 {
                            param_id: ParamId::RasterizerState0,
                            param: MatlRasterizerStateV16 {
                                fill_mode: FillMode::Solid,
                                cull_mode: CullMode::Front,
                                depth_bias: -5.0,
                                unk4: 0.0,
                                unk5: 0.0,
                                unk6: 0,
                            }
                            .to_param(),
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

    #[test]
    fn ultimate_matl_entry_conversions() {
        // fighter/mario/model/body/c00/model.numatb "alp_mario_002"
        let entry = MatlEntryV16 {
            material_label: "alp_mario_002".into(),
            attributes: vec![
                MatlAttributeV16 {
                    param_id: ParamId::BlendState0,
                    param: MatlBlendStateV16 {
                        source_color: BlendFactor::One,
                        unk2: 0,
                        destination_color: BlendFactor::Zero,
                        unk4: 1,
                        unk5: 0,
                        unk6: 0,
                        alpha_sample_to_coverage: 0,
                        unk8: 0,
                        unk9: 0,
                        unk10: 5,
                    }
                    .to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomBoolean1,
                    param: true.to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomBoolean3,
                    param: true.to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomBoolean4,
                    param: true.to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomFloat8,
                    param: 0.7.to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomVector0,
                    param: Vector4::new(1.0, 0.0, 0.0, 0.0).to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomVector13,
                    param: Vector4::new(1.0, 1.0, 1.0, 1.0).to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomVector14,
                    param: Vector4::new(1.0, 1.0, 1.0, 1.0).to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::CustomVector8,
                    param: Vector4::new(1.0, 1.0, 1.0, 1.0).to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::RasterizerState0,
                    param: MatlRasterizerStateV16 {
                        fill_mode: FillMode::Solid,
                        cull_mode: CullMode::Back,
                        depth_bias: 0.0,
                        unk4: 0.0,
                        unk5: 0.0,
                        unk6: 16777217,
                    }
                    .to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Sampler0,
                    param: MatlSampler {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::Repeat,
                        wrapr: WrapMode::Repeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        texture_filtering_type: FilteringType::Default2,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        unk11: 0,
                        unk12: 2139095022,
                        lod_bias: 0.0,
                        max_anisotropy: MaxAnisotropy::Two,
                    }
                    .to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Sampler4,
                    param: MatlSampler {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::Repeat,
                        wrapr: WrapMode::Repeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        texture_filtering_type: FilteringType::Default2,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        unk11: 0,
                        unk12: 2139095022,
                        lod_bias: 0.0,
                        max_anisotropy: MaxAnisotropy::Two,
                    }
                    .to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Sampler6,
                    param: MatlSampler {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::Repeat,
                        wrapr: WrapMode::Repeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        texture_filtering_type: FilteringType::Default2,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        unk11: 0,
                        unk12: 2139095022,
                        lod_bias: 0.0,
                        max_anisotropy: MaxAnisotropy::Two,
                    }
                    .to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Sampler7,
                    param: MatlSampler {
                        wraps: WrapMode::ClampToEdge,
                        wrapt: WrapMode::ClampToEdge,
                        wrapr: WrapMode::ClampToEdge,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        texture_filtering_type: FilteringType::Default2,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        unk11: 0,
                        unk12: 2139095022,
                        lod_bias: 0.0,
                        max_anisotropy: MaxAnisotropy::Two,
                    }
                    .to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Texture0,
                    param: "alp_mario_002_col".to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Texture4,
                    param: "alp_mario_002_nor".to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Texture6,
                    param: "alp_mario_002_prm".to_param(),
                },
                MatlAttributeV16 {
                    param_id: ParamId::Texture7,
                    param: "#replace_cubemap".to_param(),
                },
            ]
            .into(),
            shader_label: "SFX_PBS_0100000008008269_opaque".into(),
        };

        let data = MatlEntryData {
            material_label: "alp_mario_002".into(),
            shader_label: "SFX_PBS_0100000008008269_opaque".into(),
            blend_states: vec![ParamData {
                param_id: ParamId::BlendState0,
                data: BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::Zero,
                    alpha_sample_to_coverage: false,
                },
            }],
            floats: vec![ParamData {
                param_id: ParamId::CustomFloat8,
                data: 0.7,
            }],
            booleans: vec![
                ParamData {
                    param_id: ParamId::CustomBoolean1,
                    data: true,
                },
                ParamData {
                    param_id: ParamId::CustomBoolean3,
                    data: true,
                },
                ParamData {
                    param_id: ParamId::CustomBoolean4,
                    data: true,
                },
            ],
            vectors: vec![
                ParamData {
                    param_id: ParamId::CustomVector0,
                    data: Vector4::new(1.0, 0.0, 0.0, 0.0),
                },
                ParamData {
                    param_id: ParamId::CustomVector13,
                    data: Vector4::new(1.0, 1.0, 1.0, 1.0),
                },
                ParamData {
                    param_id: ParamId::CustomVector14,
                    data: Vector4::new(1.0, 1.0, 1.0, 1.0),
                },
                ParamData {
                    param_id: ParamId::CustomVector8,
                    data: Vector4::new(1.0, 1.0, 1.0, 1.0),
                },
            ],
            rasterizer_states: vec![ParamData {
                param_id: ParamId::RasterizerState0,
                data: RasterizerStateData {
                    fill_mode: FillMode::Solid,
                    cull_mode: CullMode::Back,
                    depth_bias: 0.0,
                },
            }],
            samplers: vec![
                ParamData {
                    param_id: ParamId::Sampler0,
                    data: SamplerData {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::Repeat,
                        wrapr: WrapMode::Repeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        lod_bias: 0.0,
                        max_anisotropy: None,
                    },
                },
                ParamData {
                    param_id: ParamId::Sampler4,
                    data: SamplerData {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::Repeat,
                        wrapr: WrapMode::Repeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        lod_bias: 0.0,
                        max_anisotropy: None,
                    },
                },
                ParamData {
                    param_id: ParamId::Sampler6,
                    data: SamplerData {
                        wraps: WrapMode::Repeat,
                        wrapt: WrapMode::Repeat,
                        wrapr: WrapMode::Repeat,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        lod_bias: 0.0,
                        max_anisotropy: None,
                    },
                },
                ParamData {
                    param_id: ParamId::Sampler7,
                    data: SamplerData {
                        wraps: WrapMode::ClampToEdge,
                        wrapt: WrapMode::ClampToEdge,
                        wrapr: WrapMode::ClampToEdge,
                        min_filter: MinFilter::LinearMipmapLinear,
                        mag_filter: MagFilter::Linear,
                        border_color: Color4f {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        },
                        lod_bias: 0.0,
                        max_anisotropy: None,
                    },
                },
            ],
            textures: vec![
                ParamData {
                    param_id: ParamId::Texture0,
                    data: "alp_mario_002_col".into(),
                },
                ParamData {
                    param_id: ParamId::Texture4,
                    data: "alp_mario_002_nor".into(),
                },
                ParamData {
                    param_id: ParamId::Texture6,
                    data: "alp_mario_002_prm".into(),
                },
                ParamData {
                    param_id: ParamId::Texture7,
                    data: "#replace_cubemap".into(),
                },
            ],
        };

        assert_eq!(data, MatlEntryData::from(&entry));
        // TODO: How to test this?
        // TODO: Can we guarantee this preserves all information?
        // assert_eq!(entry, MatlEntryV16::from(&data));
    }
}
