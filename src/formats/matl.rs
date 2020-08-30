use crate::SsbhArray;
use crate::SsbhString;
use serde::Serialize;

use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};

#[derive(BinRead, Debug, PartialEq, Serialize)]
enum ParamId {
    #[br(magic = 0u64)]
    Diffuse,
    #[br(magic = 1u64)]
    Specular,
    #[br(magic = 2u64)]
    Ambient,
    #[br(magic = 3u64)]
    BlendMap,
    #[br(magic = 4u64)]
    Transparency,
    #[br(magic = 5u64)]
    DiffuseMapLayer1,
    #[br(magic = 6u64)]
    CosinePower,
    #[br(magic = 7u64)]
    SpecularPower,
    #[br(magic = 8u64)]
    Fresnel,
    #[br(magic = 9u64)]
    Roughness,
    #[br(magic = 10u64)]
    EmissiveScale,
    #[br(magic = 11u64)]
    EnableDiffuse,
    #[br(magic = 12u64)]
    EnableSpecular,
    #[br(magic = 13u64)]
    EnableAmbient,
    #[br(magic = 14u64)]
    DiffuseMapLayer2,
    #[br(magic = 15u64)]
    EnableTransparency,
    #[br(magic = 16u64)]
    EnableOpacity,
    #[br(magic = 17u64)]
    EnableCosinePower,
    #[br(magic = 18u64)]
    EnableSpecularPower,
    #[br(magic = 19u64)]
    EnableFresnel,
    #[br(magic = 20u64)]
    EnableRoughness,
    #[br(magic = 21u64)]
    EnableEmissiveScale,
    #[br(magic = 22u64)]
    WorldMatrix,
    #[br(magic = 23u64)]
    ViewMatrix,
    #[br(magic = 24u64)]
    ProjectionMatrix,
    #[br(magic = 25u64)]
    WorldViewMatrix,
    #[br(magic = 26u64)]
    ViewInverseMatrix,
    #[br(magic = 27u64)]
    ViewProjectionMatrix,
    #[br(magic = 28u64)]
    WorldViewProjectionMatrix,
    #[br(magic = 29u64)]
    WorldInverseTransposeMatrix,
    #[br(magic = 30u64)]
    DiffuseMap,
    #[br(magic = 31u64)]
    SpecularMap,
    #[br(magic = 32u64)]
    AmbientMap,
    #[br(magic = 33u64)]
    EmissiveMap,
    #[br(magic = 34u64)]
    SpecularMapLayer1,
    #[br(magic = 35u64)]
    TransparencyMap,
    #[br(magic = 36u64)]
    NormalMap,
    #[br(magic = 37u64)]
    DiffuseCubeMap,
    #[br(magic = 38u64)]
    ReflectionMap,
    #[br(magic = 39u64)]
    ReflectionCubeMap,
    #[br(magic = 40u64)]
    RefractionMap,
    #[br(magic = 41u64)]
    AmbientOcclusionMap,
    #[br(magic = 42u64)]
    LightMap,
    #[br(magic = 43u64)]
    AnisotropicMap,
    #[br(magic = 44u64)]
    RoughnessMap,
    #[br(magic = 45u64)]
    ReflectionMask,
    #[br(magic = 46u64)]
    OpacityMask,
    #[br(magic = 47u64)]
    UseDiffuseMap,
    #[br(magic = 48u64)]
    UseSpecularMap,
    #[br(magic = 49u64)]
    UseAmbientMap,
    #[br(magic = 50u64)]
    UseEmissiveMap,
    #[br(magic = 51u64)]
    UseTranslucencyMap,
    #[br(magic = 52u64)]
    UseTransparencyMap,
    #[br(magic = 53u64)]
    UseNormalMap,
    #[br(magic = 54u64)]
    UseDiffuseCubeMap,
    #[br(magic = 55u64)]
    UseReflectionMap,
    #[br(magic = 56u64)]
    UseReflectionCubeMap,
    #[br(magic = 57u64)]
    UseRefractionMap,
    #[br(magic = 58u64)]
    UseAmbientOcclusionMap,
    #[br(magic = 59u64)]
    UseLightMap,
    #[br(magic = 60u64)]
    UseAnisotropicMap,
    #[br(magic = 61u64)]
    UseRoughnessMap,
    #[br(magic = 62u64)]
    UseReflectionMask,
    #[br(magic = 63u64)]
    UseOpacityMask,
    #[br(magic = 64u64)]
    DiffuseSampler,
    #[br(magic = 65u64)]
    SpecularSampler,
    #[br(magic = 66u64)]
    NormalSampler,
    #[br(magic = 67u64)]
    ReflectionSampler,
    #[br(magic = 68u64)]
    SpecularMapLayer2,
    #[br(magic = 69u64)]
    NormalMapLayer1,
    #[br(magic = 70u64)]
    NormalMapBc5,
    #[br(magic = 71u64)]
    NormalMapLayer2,
    #[br(magic = 72u64)]
    RoughnessMapLayer1,
    #[br(magic = 73u64)]
    RoughnessMapLayer2,
    #[br(magic = 74u64)]
    UseDiffuseUvTransform1,
    #[br(magic = 75u64)]
    UseDiffuseUvTransform2,
    #[br(magic = 76u64)]
    UseSpecularUvTransform1,
    #[br(magic = 77u64)]
    UseSpecularUvTransform2,
    #[br(magic = 78u64)]
    UseNormalUvTransform1,
    #[br(magic = 79u64)]
    UseNormalUvTransform2,
    #[br(magic = 80u64)]
    ShadowDepthBias,
    #[br(magic = 81u64)]
    ShadowMap0,
    #[br(magic = 82u64)]
    ShadowMap1,
    #[br(magic = 83u64)]
    ShadowMap2,
    #[br(magic = 84u64)]
    ShadowMap3,
    #[br(magic = 85u64)]
    ShadowMap4,
    #[br(magic = 86u64)]
    ShadowMap5,
    #[br(magic = 87u64)]
    ShadowMap6,
    #[br(magic = 88u64)]
    ShadowMap7,
    #[br(magic = 89u64)]
    CastShadow,
    #[br(magic = 90u64)]
    ReceiveShadow,
    #[br(magic = 91u64)]
    ShadowMapSampler,
    #[br(magic = 92u64)]
    Texture0,
    #[br(magic = 93u64)]
    Texture1,
    #[br(magic = 94u64)]
    Texture2,
    #[br(magic = 95u64)]
    Texture3,
    #[br(magic = 96u64)]
    Texture4,
    #[br(magic = 97u64)]
    Texture5,
    #[br(magic = 98u64)]
    Texture6,
    #[br(magic = 99u64)]
    Texture7,
    #[br(magic = 100u64)]
    Texture8,
    #[br(magic = 101u64)]
    Texture9,
    #[br(magic = 102u64)]
    Texture10,
    #[br(magic = 103u64)]
    Texture11,
    #[br(magic = 104u64)]
    Texture12,
    #[br(magic = 105u64)]
    Texture13,
    #[br(magic = 106u64)]
    Texture14,
    #[br(magic = 107u64)]
    Texture15,
    #[br(magic = 108u64)]
    Sampler0,
    #[br(magic = 109u64)]
    Sampler1,
    #[br(magic = 110u64)]
    Sampler2,
    #[br(magic = 111u64)]
    Sampler3,
    #[br(magic = 112u64)]
    Sampler4,
    #[br(magic = 113u64)]
    Sampler5,
    #[br(magic = 114u64)]
    Sampler6,
    #[br(magic = 115u64)]
    Sampler7,
    #[br(magic = 116u64)]
    Sampler8,
    #[br(magic = 117u64)]
    Sampler9,
    #[br(magic = 118u64)]
    Sampler10,
    #[br(magic = 119u64)]
    Sampler11,
    #[br(magic = 120u64)]
    Sampler12,
    #[br(magic = 121u64)]
    Sampler13,
    #[br(magic = 122u64)]
    Sampler14,
    #[br(magic = 123u64)]
    Sampler15,
    #[br(magic = 124u64)]
    CustomBuffer0,
    #[br(magic = 125u64)]
    CustomBuffer1,
    #[br(magic = 126u64)]
    CustomBuffer2,
    #[br(magic = 127u64)]
    CustomBuffer3,
    #[br(magic = 128u64)]
    CustomBuffer4,
    #[br(magic = 129u64)]
    CustomBuffer5,
    #[br(magic = 130u64)]
    CustomBuffer6,
    #[br(magic = 131u64)]
    CustomBuffer7,
    #[br(magic = 132u64)]
    CustomMatrix0,
    #[br(magic = 133u64)]
    CustomMatrix1,
    #[br(magic = 134u64)]
    CustomMatrix2,
    #[br(magic = 135u64)]
    CustomMatrix3,
    #[br(magic = 136u64)]
    CustomMatrix4,
    #[br(magic = 137u64)]
    CustomMatrix5,
    #[br(magic = 138u64)]
    CustomMatrix6,
    #[br(magic = 139u64)]
    CustomMatrix7,
    #[br(magic = 140u64)]
    CustomMatrix8,
    #[br(magic = 141u64)]
    CustomMatrix9,
    #[br(magic = 142u64)]
    CustomMatrix10,
    #[br(magic = 143u64)]
    CustomMatrix11,
    #[br(magic = 144u64)]
    CustomMatrix12,
    #[br(magic = 145u64)]
    CustomMatrix13,
    #[br(magic = 146u64)]
    CustomMatrix14,
    #[br(magic = 147u64)]
    CustomMatrix15,
    #[br(magic = 148u64)]
    CustomMatrix16,
    #[br(magic = 149u64)]
    CustomMatrix17,
    #[br(magic = 150u64)]
    CustomMatrix18,
    #[br(magic = 151u64)]
    CustomMatrix19,
    #[br(magic = 152u64)]
    CustomVector0,
    #[br(magic = 153u64)]
    CustomVector1,
    #[br(magic = 154u64)]
    CustomVector2,
    #[br(magic = 155u64)]
    CustomVector3,
    #[br(magic = 156u64)]
    CustomVector4,
    #[br(magic = 157u64)]
    CustomVector5,
    #[br(magic = 158u64)]
    CustomVector6,
    #[br(magic = 159u64)]
    CustomVector7,
    #[br(magic = 160u64)]
    CustomVector8,
    #[br(magic = 161u64)]
    CustomVector9,
    #[br(magic = 162u64)]
    CustomVector10,
    #[br(magic = 163u64)]
    CustomVector11,
    #[br(magic = 164u64)]
    CustomVector12,
    #[br(magic = 165u64)]
    CustomVector13,
    #[br(magic = 166u64)]
    CustomVector14,
    #[br(magic = 167u64)]
    CustomVector15,
    #[br(magic = 168u64)]
    CustomVector16,
    #[br(magic = 169u64)]
    CustomVector17,
    #[br(magic = 170u64)]
    CustomVector18,
    #[br(magic = 171u64)]
    CustomVector19,
    #[br(magic = 172u64)]
    CustomColor0,
    #[br(magic = 173u64)]
    CustomColor1,
    #[br(magic = 174u64)]
    CustomColor2,
    #[br(magic = 175u64)]
    CustomColor3,
    #[br(magic = 176u64)]
    CustomColor4,
    #[br(magic = 177u64)]
    CustomColor5,
    #[br(magic = 178u64)]
    CustomColor6,
    #[br(magic = 179u64)]
    CustomColor7,
    #[br(magic = 180u64)]
    CustomColor8,
    #[br(magic = 181u64)]
    CustomColor9,
    #[br(magic = 182u64)]
    CustomColor10,
    #[br(magic = 183u64)]
    CustomColor11,
    #[br(magic = 184u64)]
    CustomColor12,
    #[br(magic = 185u64)]
    CustomColor13,
    #[br(magic = 186u64)]
    CustomColor14,
    #[br(magic = 187u64)]
    CustomColor15,
    #[br(magic = 188u64)]
    CustomColor16,
    #[br(magic = 189u64)]
    CustomColor17,
    #[br(magic = 190u64)]
    CustomColor18,
    #[br(magic = 191u64)]
    CustomColor19,
    #[br(magic = 192u64)]
    CustomFloat0,
    #[br(magic = 193u64)]
    CustomFloat1,
    #[br(magic = 194u64)]
    CustomFloat2,
    #[br(magic = 195u64)]
    CustomFloat3,
    #[br(magic = 196u64)]
    CustomFloat4,
    #[br(magic = 197u64)]
    CustomFloat5,
    #[br(magic = 198u64)]
    CustomFloat6,
    #[br(magic = 199u64)]
    CustomFloat7,
    #[br(magic = 200u64)]
    CustomFloat8,
    #[br(magic = 201u64)]
    CustomFloat9,
    #[br(magic = 202u64)]
    CustomFloat10,
    #[br(magic = 203u64)]
    CustomFloat11,
    #[br(magic = 204u64)]
    CustomFloat12,
    #[br(magic = 205u64)]
    CustomFloat13,
    #[br(magic = 206u64)]
    CustomFloat14,
    #[br(magic = 207u64)]
    CustomFloat15,
    #[br(magic = 208u64)]
    CustomFloat16,
    #[br(magic = 209u64)]
    CustomFloat17,
    #[br(magic = 210u64)]
    CustomFloat18,
    #[br(magic = 211u64)]
    CustomFloat19,
    #[br(magic = 212u64)]
    CustomInteger0,
    #[br(magic = 213u64)]
    CustomInteger1,
    #[br(magic = 214u64)]
    CustomInteger2,
    #[br(magic = 215u64)]
    CustomInteger3,
    #[br(magic = 216u64)]
    CustomInteger4,
    #[br(magic = 217u64)]
    CustomInteger5,
    #[br(magic = 218u64)]
    CustomInteger6,
    #[br(magic = 219u64)]
    CustomInteger7,
    #[br(magic = 220u64)]
    CustomInteger8,
    #[br(magic = 221u64)]
    CustomInteger9,
    #[br(magic = 222u64)]
    CustomInteger10,
    #[br(magic = 223u64)]
    CustomInteger11,
    #[br(magic = 224u64)]
    CustomInteger12,
    #[br(magic = 225u64)]
    CustomInteger13,
    #[br(magic = 226u64)]
    CustomInteger14,
    #[br(magic = 227u64)]
    CustomInteger15,
    #[br(magic = 228u64)]
    CustomInteger16,
    #[br(magic = 229u64)]
    CustomInteger17,
    #[br(magic = 230u64)]
    CustomInteger18,
    #[br(magic = 231u64)]
    CustomInteger19,
    #[br(magic = 232u64)]
    CustomBoolean0,
    #[br(magic = 233u64)]
    CustomBoolean1,
    #[br(magic = 234u64)]
    CustomBoolean2,
    #[br(magic = 235u64)]
    CustomBoolean3,
    #[br(magic = 236u64)]
    CustomBoolean4,
    #[br(magic = 237u64)]
    CustomBoolean5,
    #[br(magic = 238u64)]
    CustomBoolean6,
    #[br(magic = 239u64)]
    CustomBoolean7,
    #[br(magic = 240u64)]
    CustomBoolean8,
    #[br(magic = 241u64)]
    CustomBoolean9,
    #[br(magic = 242u64)]
    CustomBoolean10,
    #[br(magic = 243u64)]
    CustomBoolean11,
    #[br(magic = 244u64)]
    CustomBoolean12,
    #[br(magic = 245u64)]
    CustomBoolean13,
    #[br(magic = 246u64)]
    CustomBoolean14,
    #[br(magic = 247u64)]
    CustomBoolean15,
    #[br(magic = 248u64)]
    CustomBoolean16,
    #[br(magic = 249u64)]
    CustomBoolean17,
    #[br(magic = 250u64)]
    CustomBoolean18,
    #[br(magic = 251u64)]
    CustomBoolean19,
    #[br(magic = 252u64)]
    UvTransform0,
    #[br(magic = 253u64)]
    UvTransform1,
    #[br(magic = 254u64)]
    UvTransform2,
    #[br(magic = 255u64)]
    UvTransform3,
    #[br(magic = 256u64)]
    UvTransform4,
    #[br(magic = 257u64)]
    UvTransform5,
    #[br(magic = 258u64)]
    UvTransform6,
    #[br(magic = 259u64)]
    UvTransform7,
    #[br(magic = 260u64)]
    UvTransform8,
    #[br(magic = 261u64)]
    UvTransform9,
    #[br(magic = 262u64)]
    UvTransform10,
    #[br(magic = 263u64)]
    UvTransform11,
    #[br(magic = 264u64)]
    UvTransform12,
    #[br(magic = 265u64)]
    UvTransform13,
    #[br(magic = 266u64)]
    UvTransform14,
    #[br(magic = 267u64)]
    UvTransform15,
    #[br(magic = 268u64)]
    DiffuseUvTransform1,
    #[br(magic = 269u64)]
    DiffuseUvTransform2,
    #[br(magic = 270u64)]
    SpecularUvTransform1,
    #[br(magic = 271u64)]
    SpecularUvTransform2,
    #[br(magic = 272u64)]
    NormalUvTransform1,
    #[br(magic = 273u64)]
    NormalUvTransform2,
    #[br(magic = 274u64)]
    DiffuseUvTransform,
    #[br(magic = 275u64)]
    SpecularUvTransform,
    #[br(magic = 276u64)]
    NormalUvTransform,
    #[br(magic = 277u64)]
    UseDiffuseUvTransform,
    #[br(magic = 278u64)]
    UseSpecularUvTransform,
    #[br(magic = 279u64)]
    UseNormalUvTransform,
    #[br(magic = 280u64)]
    BlendState0,
    #[br(magic = 281u64)]
    BlendState1,
    #[br(magic = 282u64)]
    BlendState2,
    #[br(magic = 283u64)]
    BlendState3,
    #[br(magic = 284u64)]
    BlendState4,
    #[br(magic = 285u64)]
    BlendState5,
    #[br(magic = 286u64)]
    BlendState6,
    #[br(magic = 287u64)]
    BlendState7,
    #[br(magic = 288u64)]
    BlendState8,
    #[br(magic = 289u64)]
    BlendState9,
    #[br(magic = 290u64)]
    BlendState10,
    #[br(magic = 291u64)]
    RasterizerState0,
    #[br(magic = 292u64)]
    RasterizerState1,
    #[br(magic = 293u64)]
    RasterizerState2,
    #[br(magic = 294u64)]
    RasterizerState3,
    #[br(magic = 295u64)]
    RasterizerState4,
    #[br(magic = 296u64)]
    RasterizerState5,
    #[br(magic = 297u64)]
    RasterizerState6,
    #[br(magic = 298u64)]
    RasterizerState7,
    #[br(magic = 299u64)]
    RasterizerState8,
    #[br(magic = 300u64)]
    RasterizerState9,
    #[br(magic = 301u64)]
    RasterizerState10,
    #[br(magic = 302u64)]
    ShadowColor,
    #[br(magic = 303u64)]
    EmissiveMapLayer1,
    #[br(magic = 304u64)]
    EmissiveMapLayer2,
    #[br(magic = 305u64)]
    AlphaTestFunc,
    #[br(magic = 306u64)]
    AlphaTestRef,
    #[br(magic = 307u64)]
    Texture16,
    #[br(magic = 308u64)]
    Texture17,
    #[br(magic = 309u64)]
    Texture18,
    #[br(magic = 310u64)]
    Texture19,
    #[br(magic = 311u64)]
    Sampler16,
    #[br(magic = 312u64)]
    Sampler17,
    #[br(magic = 313u64)]
    Sampler18,
    #[br(magic = 314u64)]
    Sampler19,
    #[br(magic = 315u64)]
    CustomVector20,
    #[br(magic = 316u64)]
    CustomVector21,
    #[br(magic = 317u64)]
    CustomVector22,
    #[br(magic = 318u64)]
    CustomVector23,
    #[br(magic = 319u64)]
    CustomVector24,
    #[br(magic = 320u64)]
    CustomVector25,
    #[br(magic = 321u64)]
    CustomVector26,
    #[br(magic = 322u64)]
    CustomVector27,
    #[br(magic = 323u64)]
    CustomVector28,
    #[br(magic = 324u64)]
    CustomVector29,
    #[br(magic = 325u64)]
    CustomVector30,
    #[br(magic = 326u64)]
    CustomVector31,
    #[br(magic = 327u64)]
    CustomVector32,
    #[br(magic = 328u64)]
    CustomVector33,
    #[br(magic = 329u64)]
    CustomVector34,
    #[br(magic = 330u64)]
    CustomVector35,
    #[br(magic = 331u64)]
    CustomVector36,
    #[br(magic = 332u64)]
    CustomVector37,
    #[br(magic = 333u64)]
    CustomVector38,
    #[br(magic = 334u64)]
    CustomVector39,
    #[br(magic = 335u64)]
    CustomVector40,
    #[br(magic = 336u64)]
    CustomVector41,
    #[br(magic = 337u64)]
    CustomVector42,
    #[br(magic = 338u64)]
    CustomVector43,
    #[br(magic = 339u64)]
    CustomVector44,
    #[br(magic = 340u64)]
    CustomVector45,
    #[br(magic = 341u64)]
    CustomVector46,
    #[br(magic = 342u64)]
    CustomVector47,
    #[br(magic = 343u64)]
    CustomVector48,
    #[br(magic = 344u64)]
    CustomVector49,
    #[br(magic = 345u64)]
    CustomVector50,
    #[br(magic = 346u64)]
    CustomVector51,
    #[br(magic = 347u64)]
    CustomVector52,
    #[br(magic = 348u64)]
    CustomVector53,
    #[br(magic = 349u64)]
    CustomVector54,
    #[br(magic = 350u64)]
    CustomVector55,
    #[br(magic = 351u64)]
    CustomVector56,
    #[br(magic = 352u64)]
    CustomVector57,
    #[br(magic = 353u64)]
    CustomVector58,
    #[br(magic = 354u64)]
    CustomVector59,
    #[br(magic = 355u64)]
    CustomVector60,
    #[br(magic = 356u64)]
    CustomVector61,
    #[br(magic = 357u64)]
    CustomVector62,
    #[br(magic = 358u64)]
    CustomVector63,
    #[br(magic = 359u64)]
    UseBaseColorMap,
    #[br(magic = 360u64)]
    UseMetallicMap,
    #[br(magic = 361u64)]
    BaseColorMap,
    #[br(magic = 362u64)]
    BaseColorMapLayer1,
    #[br(magic = 363u64)]
    MetallicMap,
    #[br(magic = 364u64)]
    MetallicMapLayer1,
    #[br(magic = 365u64)]
    DiffuseLightingAoOffset,
}

#[derive(Serialize, BinRead, Debug)]
#[br(import(param_type: ParamDataType))]
enum Param {
    #[br(pre_assert(param_type == ParamDataType::Float))]
    Float(f32),

    #[br(pre_assert(param_type == ParamDataType::Boolean))]
    Boolean(u32),

    #[br(pre_assert(param_type == ParamDataType::Vector4))]
    Vector4(MatlVec4),

    #[br(pre_assert(param_type == ParamDataType::MatlString))]
    MatlString(SsbhString),

    #[br(pre_assert(param_type == ParamDataType::Sampler))]
    Sampler(MatlSampler),

    #[br(pre_assert(param_type == ParamDataType::UvTransform))]
    UvTransform(MatlUvTransform),

    #[br(pre_assert(param_type == ParamDataType::BlendState))]
    BlendState(MatlBlendState),

    #[br(pre_assert(param_type == ParamDataType::RasterizerState))]
    RasterizerState(MatlRasterizerState),
}

#[derive(Serialize, BinRead, Debug, Copy, Clone, PartialEq)]
enum ParamDataType {
    #[br(magic = 0x1u64)]
    Float,

    #[br(magic = 0x2u64)]
    Boolean,

    #[br(magic = 0x5u64)]
    Vector4,

    #[br(magic = 0xBu64)]
    MatlString,

    #[br(magic = 0xEu64)]
    Sampler,

    #[br(magic = 0x10u64)]
    UvTransform,

    #[br(magic = 0x11u64)]
    BlendState,

    #[br(magic = 0x12u64)]
    RasterizerState,
}

#[derive(Serialize, Debug)]
struct ParamData {
    data: Param,
}

impl BinRead for ParamData {
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let pos_before_read = reader.seek(SeekFrom::Current(0))?;
        let ptr = u64::read_options(reader, options, ())?;
        let data_type = ParamDataType::read_options(reader, options, ())?;
        let saved_pos = reader.seek(SeekFrom::Current(0))?;

        reader.seek(SeekFrom::Start(pos_before_read + ptr))?;
        let value = Param::read_options(reader, options, (data_type,))?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(ParamData { data: value })
    }
}

#[derive(Serialize, BinRead, PartialEq, Debug, Clone)]
enum FillMode {
    #[br(magic = 0u32)]
    Line,
    #[br(magic = 1u32)]
    Solid,
}

#[derive(Serialize, BinRead, PartialEq, Debug, Clone)]
enum CullMode {
    #[br(magic = 0u32)]
    Back,
    #[br(magic = 1u32)]
    Front,
    #[br(magic = 2u32)]
    FrontAndBack,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
struct MatlRasterizerState {
    fill_mode: FillMode,
    cull_mode: CullMode,
    depth_bias: f32,
    unk4: f32,
    unk5: f32,
    unk6: u32,
    unk7: u32,
    unk8: f32,
}

#[derive(Serialize, BinRead, PartialEq, Debug, Clone)]
enum WrapMode {
    #[br(magic = 0u32)]
    Repeat,
    #[br(magic = 1u32)]
    ClampToEdge,
    #[br(magic = 2u32)]
    MirroredRepeat,
    #[br(magic = 3u32)]
    ClampToBorder,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
struct MatlSampler {
    wraps: WrapMode,
    wrapt: WrapMode,
    wrapr: WrapMode,
    min_filter: u32,
    mag_filter: u32,
    unk6: u32,
    unk7: u32,
    unk8: u32,
    unk9: u32,
    unk10: u32,
    unk11: u32,
    unk12: u32,
    lod_bias: f32,
    max_anisotropy: u32,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
struct MatlUvTransform {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    v: f32,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
struct MatlVec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[derive(Serialize, BinRead, Debug)]
struct MatlAttribute {
    param_id: ParamId,
    param: ParamData,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
struct MatlBlendState {
    unk1: u32,
    unk2: u32,
    blend_factor1: u32,
    unk4: u32,
    unk5: u32,
    blend_factor2: u32,
    unk7: u32,
    unk8: u32,
    unk9: u32,
    unk10: u32,
    unk11: u32,
    unk12: u32,
}

#[derive(Serialize, BinRead, Debug)]
struct MatlEntry {
    material_label: SsbhString,
    attributes: SsbhArray<MatlAttribute>,
    shader_label: SsbhString,
}

#[derive(Serialize, BinRead, Debug)]
pub struct Matl {
    major_version: u16,
    minor_version: u16,
    entries: SsbhArray<MatlEntry>,
}
