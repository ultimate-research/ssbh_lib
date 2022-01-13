//! The [Matl] format stores a collection of materials used for model rendering.
//! These files typically use the ".numatb" suffix like "model.numatb".
//!
//! The materials define some of the inputs for the specified shader and provide additional configuration over the rendering pipeline such as alpha blending settings.
//! The materials in the [Matl] file are assigned to objects in the [Mesh](crate::formats::mesh::Mesh) file by the [Modl](crate::formats::modl::Modl) file.

use crate::DataType;
use crate::{Color4f, SsbhString, Vector4, Version};
use crate::{SsbhArray, SsbhEnum64};
use binread::BinRead;
use ssbh_write::SsbhWrite;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "strum")]
use strum::{Display, EnumString, EnumVariantNames, FromRepr};

// TODO: Rename these to Parameters to be consistent?
/// A named material value.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq)]
pub struct AttributeV15 {
    /// Determines how the value in [param](#structfield.param) will be used by the shader.
    pub param_id: ParamId,
    /// The value and data type.
    pub param: SsbhEnum64<ParamV15>,
}

/// A named material value.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq)]
pub struct AttributeV16 {
    /// Determines how the value in [param](#structfield.param) will be used by the shader.
    pub param_id: ParamId,
    /// The value and data type.
    pub param: SsbhEnum64<ParamV16>,
}

/// A named collection of material values for a specified shader.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq)]
pub struct MatlEntryV15 {
    /// The name of this material.
    /// Material names should be unique.
    pub material_label: SsbhString,

    /// The collection of named material values.
    pub attributes: SsbhArray<AttributeV15>,

    /// The ID of the shader to associate with this material.
    /// For Smash Ultimate, the format is `<shader ID>_<render pass>`.
    /// For example, the [shader_label](#structfield.shader_label) for shader `SFX_PBS_010002000800824f` and the `nu::opaque` render pass is "SFX_PBS_010002000800824f_opaque".
    pub shader_label: SsbhString,
}

/// A named collection of material values for a specified shader.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq)]
pub struct MatlEntryV16 {
    /// The name of this material.
    /// Material names should be unique.
    pub material_label: SsbhString,

    /// The collection of named material values.
    pub attributes: SsbhArray<AttributeV16>,

    /// The ID of the shader to associate with this material.
    /// For Smash Ultimate, the format is `<shader ID>_<render pass>`.
    /// For example, the [shader_label](#structfield.shader_label) for shader `SFX_PBS_010002000800824f` and the `nu::opaque` render pass is "SFX_PBS_010002000800824f_opaque".
    pub shader_label: SsbhString,
}

/// A container of materials.
/// Compatible with file version 1.5 and 1.6.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[br(import(major_version: u16, minor_version: u16))]
pub enum Matl {
    // TODO: Would it be clearer to have "V15 { entries: SsbhArray<...> }"?
    // It seems redundant to type Matl::V15(MatlV15 { ... }).
    // TODO: Add support for named enum fields to SsbhWrite.
    #[br(pre_assert(major_version == 1 &&  minor_version == 5))]
    V15(MatlV15),
    #[br(pre_assert(major_version == 1 &&  minor_version == 6))]
    V16(MatlV16),
}

impl Version for Matl {
    fn major_minor_version(&self) -> (u16, u16) {
        match self {
            Matl::V15(_) => (1, 5),
            Matl::V16(_) => (1, 6),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MatlV15 {
    pub entries: SsbhArray<MatlEntryV15>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MatlV16 {
    pub entries: SsbhArray<MatlEntryV16>,
}

/// A material parameter value.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq)]
#[br(import(data_type: u64))]
pub enum ParamV15 {
    #[br(pre_assert(data_type == 1u64))]
    Float(f32),

    #[br(pre_assert(data_type == 2u64))]
    Boolean(u32),

    /// A vector for storing RGBA colors, XYZW values, or up to four [f32] parameters.
    #[br(pre_assert(data_type == 5u64))]
    Vector4(Vector4),

    /// A vector for storing RGBA colors.
    #[br(pre_assert(data_type == 7u64))]
    Unk7(Color4f),

    /// A string value used to store texture file names.
    /// Examples: `"../../textures/cos_149000_02"`, `"/common/shader/sfxPBS/default_Params"`, `"#replace_cubemap"`, `"asf_ashley_col"`.
    #[br(pre_assert(data_type == 11u64))]
    String(SsbhString),

    #[br(pre_assert(data_type == 14u64))]
    Sampler(Sampler),

    #[br(pre_assert(data_type == 16u64))]
    UvTransform(UvTransform),

    #[br(pre_assert(data_type == 17u64))]
    BlendState(BlendStateV15),

    #[br(pre_assert(data_type == 18u64))]
    RasterizerState(RasterizerStateV15),
}

impl DataType for ParamV15 {
    fn data_type(&self) -> u64 {
        match self {
            ParamV15::Float(_) => 1,
            ParamV15::Boolean(_) => 2,
            ParamV15::Vector4(_) => 5,
            ParamV15::Unk7(_) => 7,
            ParamV15::String(_) => 11,
            ParamV15::Sampler(_) => 14,
            ParamV15::UvTransform(_) => 16,
            ParamV15::BlendState(_) => 17,
            ParamV15::RasterizerState(_) => 18,
        }
    }
}

/// A material parameter value.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, PartialEq)]
#[br(import(data_type: u64))]
pub enum ParamV16 {
    #[br(pre_assert(data_type == 1u64))]
    Float(f32),

    #[br(pre_assert(data_type == 2u64))]
    Boolean(u32),

    /// A vector for storing RGBA colors, XYZW values, or up to four [f32] parameters.
    #[br(pre_assert(data_type == 5u64))]
    Vector4(Vector4),

    /// A vector for storing RGBA colors.
    #[br(pre_assert(data_type == 7u64))]
    Unk7(Color4f),

    /// A string value used to store texture file names.
    /// Examples: `"../../textures/cos_149000_02"`, `"/common/shader/sfxPBS/default_Params"`, `"#replace_cubemap"`, `"asf_ashley_col"`.    
    #[br(pre_assert(data_type == 11u64))]
    String(SsbhString),

    #[br(pre_assert(data_type == 14u64))]
    Sampler(Sampler),

    #[br(pre_assert(data_type == 16u64))]
    UvTransform(UvTransform),

    #[br(pre_assert(data_type == 17u64))]
    BlendState(BlendStateV16),

    #[br(pre_assert(data_type == 18u64))]
    RasterizerState(RasterizerStateV16),
}

impl DataType for ParamV16 {
    fn data_type(&self) -> u64 {
        match self {
            ParamV16::Float(_) => 1,
            ParamV16::Boolean(_) => 2,
            ParamV16::Vector4(_) => 5,
            ParamV16::Unk7(_) => 7,
            ParamV16::String(_) => 11,
            ParamV16::Sampler(_) => 14,
            ParamV16::UvTransform(_) => 16,
            ParamV16::BlendState(_) => 17,
            ParamV16::RasterizerState(_) => 18,
        }
    }
}

/// An enumeration of all possible material parameters.
/// Not all values are used by Smash Ultimate's shaders.
/// For up to date documentation, see the [Material Parameters](https://github.com/ScanMountGoat/Smush-Material-Research/blob/master/Material%20Parameters.md) page on Github.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u64))]
#[ssbhwrite(repr(u64))]
pub enum ParamId {
    // Sorted by Smash Ultimate's occurrence count in descending order to improve matching performance.
    BlendState0 = 280,
    RasterizerState0 = 291,
    CustomVector8 = 160,
    Texture4 = 96,
    CustomVector0 = 152,
    CustomBoolean1 = 233,
    CustomVector13 = 165,
    CustomBoolean3 = 235,
    CustomBoolean4 = 236,
    Texture7 = 99,
    CustomVector14 = 166,
    CustomFloat8 = 200,
    Texture0 = 92,
    Texture6 = 98,
    CustomVector3 = 155,
    Texture5 = 97,
    CustomVector30 = 325,
    CustomBoolean2 = 234,
    CustomVector31 = 326,
    CustomBoolean11 = 243,
    Texture14 = 106,
    CustomVector27 = 322,
    Texture9 = 101,
    CustomVector29 = 324,
    CustomVector6 = 158,
    CustomVector11 = 163,
    CustomBoolean5 = 237,
    CustomBoolean12 = 244,
    CustomBoolean6 = 238,
    Texture2 = 94,
    Texture1 = 93,
    CustomVector7 = 159,
    CustomFloat1 = 193,
    Texture3 = 95,
    CustomFloat19 = 211,
    CustomVector18 = 170,
    CustomBoolean9 = 241,
    CustomVector42 = 337,
    CustomVector32 = 327,
    CustomBoolean7 = 239,
    CustomFloat4 = 196,
    CustomFloat10 = 202,
    Texture11 = 103,
    Texture16 = 307,
    CustomVector47 = 342,
    Texture10 = 102,
    CustomVector34 = 329,
    CustomFloat11 = 203,
    CustomFloat12 = 204,
    CustomVector35 = 330,
    CustomFloat6 = 198,
    CustomFloat18 = 210,
    CustomVector37 = 332,
    CustomVector38 = 333,
    CustomVector39 = 334,
    CustomVector19 = 171,
    CustomVector23 = 318,
    Texture13 = 105,
    CustomVector21 = 316,
    CustomBoolean0 = 232,
    CustomVector20 = 315,
    CustomBoolean10 = 242,
    CustomVector40 = 335,
    Texture12 = 104,
    CustomVector22 = 317,
    Texture8 = 100,
    CustomVector46 = 341,
    CustomFloat17 = 209,
    CustomVector24 = 319,
    CustomBoolean8 = 240,
    CustomVector33 = 328,
    CustomVector4 = 156,
    CustomFloat0 = 192,
    CustomVector1 = 153,
    CustomVector2 = 154,
    CustomVector5 = 157,
    CustomVector15 = 167,
    CustomVector16 = 168,
    CustomVector43 = 338,
    CustomVector44 = 339,
    CustomVector45 = 340,
    CustomVector9 = 161,
    CustomVector10 = 162,
    Diffuse = 0,
    Specular = 1,
    Ambient = 2,
    BlendMap = 3,
    Transparency = 4,
    DiffuseMapLayer1 = 5,
    CosinePower = 6,
    SpecularPower = 7,
    Fresnel = 8,
    Roughness = 9,
    EmissiveScale = 10,
    EnableDiffuse = 11,
    EnableSpecular = 12,
    EnableAmbient = 13,
    DiffuseMapLayer2 = 14,
    EnableTransparency = 15,
    EnableOpacity = 16,
    EnableCosinePower = 17,
    EnableSpecularPower = 18,
    EnableFresnel = 19,
    EnableRoughness = 20,
    EnableEmissiveScale = 21,
    WorldMatrix = 22,
    ViewMatrix = 23,
    ProjectionMatrix = 24,
    WorldViewMatrix = 25,
    ViewInverseMatrix = 26,
    ViewProjectionMatrix = 27,
    WorldViewProjectionMatrix = 28,
    WorldInverseTransposeMatrix = 29,
    DiffuseMap = 30,
    SpecularMap = 31,
    AmbientMap = 32,
    EmissiveMap = 33,
    SpecularMapLayer1 = 34,
    TransparencyMap = 35,
    NormalMap = 36,
    DiffuseCubeMap = 37,
    ReflectionMap = 38,
    ReflectionCubeMap = 39,
    RefractionMap = 40,
    AmbientOcclusionMap = 41,
    LightMap = 42,
    AnisotropicMap = 43,
    RoughnessMap = 44,
    ReflectionMask = 45,
    OpacityMask = 46,
    UseDiffuseMap = 47,
    UseSpecularMap = 48,
    UseAmbientMap = 49,
    UseEmissiveMap = 50,
    UseTranslucencyMap = 51,
    UseTransparencyMap = 52,
    UseNormalMap = 53,
    UseDiffuseCubeMap = 54,
    UseReflectionMap = 55,
    UseReflectionCubeMap = 56,
    UseRefractionMap = 57,
    UseAmbientOcclusionMap = 58,
    UseLightMap = 59,
    UseAnisotropicMap = 60,
    UseRoughnessMap = 61,
    UseReflectionMask = 62,
    UseOpacityMask = 63,
    DiffuseSampler = 64,
    SpecularSampler = 65,
    NormalSampler = 66,
    ReflectionSampler = 67,
    SpecularMapLayer2 = 68,
    NormalMapLayer1 = 69,
    NormalMapBc5 = 70,
    NormalMapLayer2 = 71,
    RoughnessMapLayer1 = 72,
    RoughnessMapLayer2 = 73,
    UseDiffuseUvTransform1 = 74,
    UseDiffuseUvTransform2 = 75,
    UseSpecularUvTransform1 = 76,
    UseSpecularUvTransform2 = 77,
    UseNormalUvTransform1 = 78,
    UseNormalUvTransform2 = 79,
    ShadowDepthBias = 80,
    ShadowMap0 = 81,
    ShadowMap1 = 82,
    ShadowMap2 = 83,
    ShadowMap3 = 84,
    ShadowMap4 = 85,
    ShadowMap5 = 86,
    ShadowMap6 = 87,
    ShadowMap7 = 88,
    CastShadow = 89,
    ReceiveShadow = 90,
    ShadowMapSampler = 91,
    Texture15 = 107,
    Sampler0 = 108,
    Sampler1 = 109,
    Sampler2 = 110,
    Sampler3 = 111,
    Sampler4 = 112,
    Sampler5 = 113,
    Sampler6 = 114,
    Sampler7 = 115,
    Sampler8 = 116,
    Sampler9 = 117,
    Sampler10 = 118,
    Sampler11 = 119,
    Sampler12 = 120,
    Sampler13 = 121,
    Sampler14 = 122,
    Sampler15 = 123,
    CustomBuffer0 = 124,
    CustomBuffer1 = 125,
    CustomBuffer2 = 126,
    CustomBuffer3 = 127,
    CustomBuffer4 = 128,
    CustomBuffer5 = 129,
    CustomBuffer6 = 130,
    CustomBuffer7 = 131,
    CustomMatrix0 = 132,
    CustomMatrix1 = 133,
    CustomMatrix2 = 134,
    CustomMatrix3 = 135,
    CustomMatrix4 = 136,
    CustomMatrix5 = 137,
    CustomMatrix6 = 138,
    CustomMatrix7 = 139,
    CustomMatrix8 = 140,
    CustomMatrix9 = 141,
    CustomMatrix10 = 142,
    CustomMatrix11 = 143,
    CustomMatrix12 = 144,
    CustomMatrix13 = 145,
    CustomMatrix14 = 146,
    CustomMatrix15 = 147,
    CustomMatrix16 = 148,
    CustomMatrix17 = 149,
    CustomMatrix18 = 150,
    CustomMatrix19 = 151,
    CustomVector12 = 164,
    CustomVector17 = 169,
    CustomColor0 = 172,
    CustomColor1 = 173,
    CustomColor2 = 174,
    CustomColor3 = 175,
    CustomColor4 = 176,
    CustomColor5 = 177,
    CustomColor6 = 178,
    CustomColor7 = 179,
    CustomColor8 = 180,
    CustomColor9 = 181,
    CustomColor10 = 182,
    CustomColor11 = 183,
    CustomColor12 = 184,
    CustomColor13 = 185,
    CustomColor14 = 186,
    CustomColor15 = 187,
    CustomColor16 = 188,
    CustomColor17 = 189,
    CustomColor18 = 190,
    CustomColor19 = 191,
    CustomFloat2 = 194,
    CustomFloat3 = 195,
    CustomFloat5 = 197,
    CustomFloat7 = 199,
    CustomFloat9 = 201,
    CustomFloat13 = 205,
    CustomFloat14 = 206,
    CustomFloat15 = 207,
    CustomFloat16 = 208,
    // The following values are unused for Smash Ultimate.
    CustomInteger0 = 212,
    CustomInteger1 = 213,
    CustomInteger2 = 214,
    CustomInteger3 = 215,
    CustomInteger4 = 216,
    CustomInteger5 = 217,
    CustomInteger6 = 218,
    CustomInteger7 = 219,
    CustomInteger8 = 220,
    CustomInteger9 = 221,
    CustomInteger10 = 222,
    CustomInteger11 = 223,
    CustomInteger12 = 224,
    CustomInteger13 = 225,
    CustomInteger14 = 226,
    CustomInteger15 = 227,
    CustomInteger16 = 228,
    CustomInteger17 = 229,
    CustomInteger18 = 230,
    CustomInteger19 = 231,
    CustomBoolean13 = 245,
    CustomBoolean14 = 246,
    CustomBoolean15 = 247,
    CustomBoolean16 = 248,
    CustomBoolean17 = 249,
    CustomBoolean18 = 250,
    CustomBoolean19 = 251,
    UvTransform0 = 252,
    UvTransform1 = 253,
    UvTransform2 = 254,
    UvTransform3 = 255,
    UvTransform4 = 256,
    UvTransform5 = 257,
    UvTransform6 = 258,
    UvTransform7 = 259,
    UvTransform8 = 260,
    UvTransform9 = 261,
    UvTransform10 = 262,
    UvTransform11 = 263,
    UvTransform12 = 264,
    UvTransform13 = 265,
    UvTransform14 = 266,
    UvTransform15 = 267,
    DiffuseUvTransform1 = 268,
    DiffuseUvTransform2 = 269,
    SpecularUvTransform1 = 270,
    SpecularUvTransform2 = 271,
    NormalUvTransform1 = 272,
    NormalUvTransform2 = 273,
    DiffuseUvTransform = 274,
    SpecularUvTransform = 275,
    NormalUvTransform = 276,
    UseDiffuseUvTransform = 277,
    UseSpecularUvTransform = 278,
    UseNormalUvTransform = 279,
    BlendState1 = 281,
    BlendState2 = 282,
    BlendState3 = 283,
    BlendState4 = 284,
    BlendState5 = 285,
    BlendState6 = 286,
    BlendState7 = 287,
    BlendState8 = 288,
    BlendState9 = 289,
    BlendState10 = 290,
    RasterizerState1 = 292,
    RasterizerState2 = 293,
    RasterizerState3 = 294,
    RasterizerState4 = 295,
    RasterizerState5 = 296,
    RasterizerState6 = 297,
    RasterizerState7 = 298,
    RasterizerState8 = 299,
    RasterizerState9 = 300,
    RasterizerState10 = 301,
    ShadowColor = 302,
    EmissiveMapLayer1 = 303,
    EmissiveMapLayer2 = 304,
    AlphaTestFunc = 305,
    AlphaTestRef = 306,
    Texture17 = 308,
    Texture18 = 309,
    Texture19 = 310,
    Sampler16 = 311,
    Sampler17 = 312,
    Sampler18 = 313,
    Sampler19 = 314,
    CustomVector25 = 320,
    CustomVector26 = 321,
    CustomVector28 = 323,
    CustomVector36 = 331,
    CustomVector41 = 336,
    CustomVector48 = 343,
    CustomVector49 = 344,
    CustomVector50 = 345,
    CustomVector51 = 346,
    CustomVector52 = 347,
    CustomVector53 = 348,
    CustomVector54 = 349,
    CustomVector55 = 350,
    CustomVector56 = 351,
    CustomVector57 = 352,
    CustomVector58 = 353,
    CustomVector59 = 354,
    CustomVector60 = 355,
    CustomVector61 = 356,
    CustomVector62 = 357,
    CustomVector63 = 358,
    UseBaseColorMap = 359,
    UseMetallicMap = 360,
    BaseColorMap = 361,
    BaseColorMapLayer1 = 362,
    MetallicMap = 363,
    MetallicMapLayer1 = 364,
    DiffuseLightingAoOffset = 365,
}

/// Determines how polygons are shaded.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum FillMode {
    Line = 0,
    Solid = 1,
}

/// Determines the criteria for when to cull a face.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum CullMode {
    Back = 0,
    Front = 1,
    None = 2,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
pub struct RasterizerStateV15 {
    pub unk1: u32,
    pub unk2: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
#[ssbhwrite(pad_after = 4)]
pub struct RasterizerStateV16 {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: u32,
}

/// Determines how texture coordinates outside the 0 to 1 range
/// are handled when sampling from the texture.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum WrapMode {
    Repeat = 0,
    ClampToEdge = 1,
    MirroredRepeat = 2,
    ClampToBorder = 3,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum MinFilter {
    Nearest = 0,
    LinearMipmapLinear = 1,
    LinearMipmapLinear2 = 2,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum MagFilter {
    Nearest = 0,
    Linear = 1,
    Linear2 = 2,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum FilteringType {
    Default = 0,
    Default2 = 1, // TODO: Does this change anything?
    AnisotropicFiltering = 2,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
pub struct Sampler {
    pub wraps: WrapMode,
    pub wrapt: WrapMode,
    pub wrapr: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub texture_filtering_type: FilteringType,
    pub border_color: Color4f,
    pub unk11: u32,
    pub unk12: u32,
    pub lod_bias: f32,
    pub max_anisotropy: MaxAnisotropy,
}

/// Available anistropy levels for anisotropic texture filtering.
///
/// Higher values produce higher quality filtering at the cost of performance.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum MaxAnisotropy {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
pub struct UvTransform {
    pub x: f32, // TODO: this is probably the same as the anim data type
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub v: f32,
}

/// Available blending modes for the source and destination color for alpha blending.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "strum",
    derive(FromRepr, Display, EnumVariantNames, EnumString)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite, Clone, Copy, PartialEq, Eq)]
#[br(repr(u32))]
#[ssbhwrite(repr(u32))]
pub enum BlendFactor {
    Zero = 0,
    One = 1,
    SourceAlpha = 2,
    DestinationAlpha = 3,
    SourceColor = 4,
    DestinationColor = 5,
    OneMinusSourceAlpha = 6,
    OneMinusDestinationAlpha = 7,
    OneMinusSourceColor = 8,
    OneMinusDestinationColor = 9,
    SourceAlphaSaturate = 10,
}

/// Determines the alpha blending settings to use when rendering.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
pub struct BlendStateV15 {
    pub unk1: u64,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u64,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
}

/// Determines the alpha blending settings to use when rendering.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, PartialEq, SsbhWrite)]
#[ssbhwrite(pad_after = 8)]
pub struct BlendStateV16 {
    pub source_color: BlendFactor,
    pub unk2: u32,
    pub destination_color: BlendFactor,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    /// 1 = enabled, 0 = disabled
    pub alpha_sample_to_coverage: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
}
