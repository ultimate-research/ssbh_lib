use crate::SsbhArray;
use crate::SsbhString;
use serde::Serialize;

use binread::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, ReadOptions,
};

// Sorted by occurrence count in descending order to improve matching performance.
#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum ParamId {
    #[br(magic = 280u64)]
    BlendState0 = 280,
    #[br(magic = 291u64)]
    RasterizerState0 = 291,
    #[br(magic = 160u64)]
    CustomVector8 = 160,
    #[br(magic = 96u64)]
    Texture4 = 96,
    #[br(magic = 152u64)]
    CustomVector0 = 152,
    #[br(magic = 233u64)]
    CustomBoolean1 = 233,
    #[br(magic = 165u64)]
    CustomVector13 = 165,
    #[br(magic = 235u64)]
    CustomBoolean3 = 235,
    #[br(magic = 236u64)]
    CustomBoolean4 = 236,
    #[br(magic = 99u64)]
    Texture7 = 99,
    #[br(magic = 166u64)]
    CustomVector14 = 166,
    #[br(magic = 200u64)]
    CustomFloat8 = 200,
    #[br(magic = 92u64)]
    Texture0 = 92,
    #[br(magic = 98u64)]
    Texture6 = 98,
    #[br(magic = 155u64)]
    CustomVector3 = 155,
    #[br(magic = 97u64)]
    Texture5 = 97,
    #[br(magic = 325u64)]
    CustomVector30 = 325,
    #[br(magic = 234u64)]
    CustomBoolean2 = 234,
    #[br(magic = 326u64)]
    CustomVector31 = 326,
    #[br(magic = 243u64)]
    CustomBoolean11 = 243,
    #[br(magic = 106u64)]
    Texture14 = 106,
    #[br(magic = 322u64)]
    CustomVector27 = 322,
    #[br(magic = 101u64)]
    Texture9 = 101,
    #[br(magic = 324u64)]
    CustomVector29 = 324,
    #[br(magic = 158u64)]
    CustomVector6 = 158,
    #[br(magic = 163u64)]
    CustomVector11 = 163,
    #[br(magic = 237u64)]
    CustomBoolean5 = 237,
    #[br(magic = 244u64)]
    CustomBoolean12 = 244,
    #[br(magic = 238u64)]
    CustomBoolean6 = 238,
    #[br(magic = 94u64)]
    Texture2 = 94,
    #[br(magic = 93u64)]
    Texture1 = 93,
    #[br(magic = 159u64)]
    CustomVector7 = 159,
    #[br(magic = 193u64)]
    CustomFloat1 = 193,
    #[br(magic = 95u64)]
    Texture3 = 95,
    #[br(magic = 211u64)]
    CustomFloat19 = 211,
    #[br(magic = 170u64)]
    CustomVector18 = 170,
    #[br(magic = 241u64)]
    CustomBoolean9 = 241,
    #[br(magic = 337u64)]
    CustomVector42 = 337,
    #[br(magic = 327u64)]
    CustomVector32 = 327,
    #[br(magic = 239u64)]
    CustomBoolean7 = 239,
    #[br(magic = 196u64)]
    CustomFloat4 = 196,
    #[br(magic = 202u64)]
    CustomFloat10 = 202,
    #[br(magic = 103u64)]
    Texture11 = 103,
    #[br(magic = 307u64)]
    Texture16 = 307,
    #[br(magic = 342u64)]
    CustomVector47 = 342,
    #[br(magic = 102u64)]
    Texture10 = 102,
    #[br(magic = 329u64)]
    CustomVector34 = 329,
    #[br(magic = 203u64)]
    CustomFloat11 = 203,
    #[br(magic = 204u64)]
    CustomFloat12 = 204,
    #[br(magic = 330u64)]
    CustomVector35 = 330,
    #[br(magic = 198u64)]
    CustomFloat6 = 198,
    #[br(magic = 210u64)]
    CustomFloat18 = 210,
    #[br(magic = 332u64)]
    CustomVector37 = 332,
    #[br(magic = 333u64)]
    CustomVector38 = 333,
    #[br(magic = 334u64)]
    CustomVector39 = 334,
    #[br(magic = 171u64)]
    CustomVector19 = 171,
    #[br(magic = 318u64)]
    CustomVector23 = 318,
    #[br(magic = 105u64)]
    Texture13 = 105,
    #[br(magic = 316u64)]
    CustomVector21 = 316,
    #[br(magic = 232u64)]
    CustomBoolean0 = 232,
    #[br(magic = 315u64)]
    CustomVector20 = 315,
    #[br(magic = 242u64)]
    CustomBoolean10 = 242,
    #[br(magic = 335u64)]
    CustomVector40 = 335,
    #[br(magic = 104u64)]
    Texture12 = 104,
    #[br(magic = 317u64)]
    CustomVector22 = 317,
    #[br(magic = 100u64)]
    Texture8 = 100,
    #[br(magic = 341u64)]
    CustomVector46 = 341,
    #[br(magic = 209u64)]
    CustomFloat17 = 209,
    #[br(magic = 319u64)]
    CustomVector24 = 319,
    #[br(magic = 240u64)]
    CustomBoolean8 = 240,
    #[br(magic = 328u64)]
    CustomVector33 = 328,
    #[br(magic = 156u64)]
    CustomVector4 = 156,
    #[br(magic = 192u64)]
    CustomFloat0 = 192,
    #[br(magic = 153u64)]
    CustomVector1 = 153,
    #[br(magic = 154u64)]
    CustomVector2 = 154,
    #[br(magic = 157u64)]
    CustomVector5 = 157,
    #[br(magic = 167u64)]
    CustomVector15 = 167,
    #[br(magic = 168u64)]
    CustomVector16 = 168,
    #[br(magic = 338u64)]
    CustomVector43 = 338,
    #[br(magic = 339u64)]
    CustomVector44 = 339,
    #[br(magic = 340u64)]
    CustomVector45 = 340,
    #[br(magic = 161u64)]
    CustomVector9 = 161,
    #[br(magic = 162u64)]
    CustomVector10 = 162,
    #[br(magic = 0u64)]
    Diffuse = 0,
    #[br(magic = 1u64)]
    Specular = 1,
    #[br(magic = 2u64)]
    Ambient = 2,
    #[br(magic = 3u64)]
    BlendMap = 3,
    #[br(magic = 4u64)]
    Transparency = 4,
    #[br(magic = 5u64)]
    DiffuseMapLayer1 = 5,
    #[br(magic = 6u64)]
    CosinePower = 6,
    #[br(magic = 7u64)]
    SpecularPower = 7,
    #[br(magic = 8u64)]
    Fresnel = 8,
    #[br(magic = 9u64)]
    Roughness = 9,
    #[br(magic = 10u64)]
    EmissiveScale = 10,
    #[br(magic = 11u64)]
    EnableDiffuse = 11,
    #[br(magic = 12u64)]
    EnableSpecular = 12,
    #[br(magic = 13u64)]
    EnableAmbient = 13,
    #[br(magic = 14u64)]
    DiffuseMapLayer2 = 14,
    #[br(magic = 15u64)]
    EnableTransparency = 15,
    #[br(magic = 16u64)]
    EnableOpacity = 16,
    #[br(magic = 17u64)]
    EnableCosinePower = 17,
    #[br(magic = 18u64)]
    EnableSpecularPower = 18,
    #[br(magic = 19u64)]
    EnableFresnel = 19,
    #[br(magic = 20u64)]
    EnableRoughness = 20,
    #[br(magic = 21u64)]
    EnableEmissiveScale = 21,
    #[br(magic = 22u64)]
    WorldMatrix = 22,
    #[br(magic = 23u64)]
    ViewMatrix = 23,
    #[br(magic = 24u64)]
    ProjectionMatrix = 24,
    #[br(magic = 25u64)]
    WorldViewMatrix = 25,
    #[br(magic = 26u64)]
    ViewInverseMatrix = 26,
    #[br(magic = 27u64)]
    ViewProjectionMatrix = 27,
    #[br(magic = 28u64)]
    WorldViewProjectionMatrix = 28,
    #[br(magic = 29u64)]
    WorldInverseTransposeMatrix = 29,
    #[br(magic = 30u64)]
    DiffuseMap = 30,
    #[br(magic = 31u64)]
    SpecularMap = 31,
    #[br(magic = 32u64)]
    AmbientMap = 32,
    #[br(magic = 33u64)]
    EmissiveMap = 33,
    #[br(magic = 34u64)]
    SpecularMapLayer1 = 34,
    #[br(magic = 35u64)]
    TransparencyMap = 35,
    #[br(magic = 36u64)]
    NormalMap = 36,
    #[br(magic = 37u64)]
    DiffuseCubeMap = 37,
    #[br(magic = 38u64)]
    ReflectionMap = 38,
    #[br(magic = 39u64)]
    ReflectionCubeMap = 39,
    #[br(magic = 40u64)]
    RefractionMap = 40,
    #[br(magic = 41u64)]
    AmbientOcclusionMap = 41,
    #[br(magic = 42u64)]
    LightMap = 42,
    #[br(magic = 43u64)]
    AnisotropicMap = 43,
    #[br(magic = 44u64)]
    RoughnessMap = 44,
    #[br(magic = 45u64)]
    ReflectionMask = 45,
    #[br(magic = 46u64)]
    OpacityMask = 46,
    #[br(magic = 47u64)]
    UseDiffuseMap = 47,
    #[br(magic = 48u64)]
    UseSpecularMap = 48,
    #[br(magic = 49u64)]
    UseAmbientMap = 49,
    #[br(magic = 50u64)]
    UseEmissiveMap = 50,
    #[br(magic = 51u64)]
    UseTranslucencyMap = 51,
    #[br(magic = 52u64)]
    UseTransparencyMap = 52,
    #[br(magic = 53u64)]
    UseNormalMap = 53,
    #[br(magic = 54u64)]
    UseDiffuseCubeMap = 54,
    #[br(magic = 55u64)]
    UseReflectionMap = 55,
    #[br(magic = 56u64)]
    UseReflectionCubeMap = 56,
    #[br(magic = 57u64)]
    UseRefractionMap = 57,
    #[br(magic = 58u64)]
    UseAmbientOcclusionMap = 58,
    #[br(magic = 59u64)]
    UseLightMap = 59,
    #[br(magic = 60u64)]
    UseAnisotropicMap = 60,
    #[br(magic = 61u64)]
    UseRoughnessMap = 61,
    #[br(magic = 62u64)]
    UseReflectionMask = 62,
    #[br(magic = 63u64)]
    UseOpacityMask = 63,
    #[br(magic = 64u64)]
    DiffuseSampler = 64,
    #[br(magic = 65u64)]
    SpecularSampler = 65,
    #[br(magic = 66u64)]
    NormalSampler = 66,
    #[br(magic = 67u64)]
    ReflectionSampler = 67,
    #[br(magic = 68u64)]
    SpecularMapLayer2 = 68,
    #[br(magic = 69u64)]
    NormalMapLayer1 = 69,
    #[br(magic = 70u64)]
    NormalMapBc5 = 70,
    #[br(magic = 71u64)]
    NormalMapLayer2 = 71,
    #[br(magic = 72u64)]
    RoughnessMapLayer1 = 72,
    #[br(magic = 73u64)]
    RoughnessMapLayer2 = 73,
    #[br(magic = 74u64)]
    UseDiffuseUvTransform1 = 74,
    #[br(magic = 75u64)]
    UseDiffuseUvTransform2 = 75,
    #[br(magic = 76u64)]
    UseSpecularUvTransform1 = 76,
    #[br(magic = 77u64)]
    UseSpecularUvTransform2 = 77,
    #[br(magic = 78u64)]
    UseNormalUvTransform1 = 78,
    #[br(magic = 79u64)]
    UseNormalUvTransform2 = 79,
    #[br(magic = 80u64)]
    ShadowDepthBias = 80,
    #[br(magic = 81u64)]
    ShadowMap0 = 81,
    #[br(magic = 82u64)]
    ShadowMap1 = 82,
    #[br(magic = 83u64)]
    ShadowMap2 = 83,
    #[br(magic = 84u64)]
    ShadowMap3 = 84,
    #[br(magic = 85u64)]
    ShadowMap4 = 85,
    #[br(magic = 86u64)]
    ShadowMap5 = 86,
    #[br(magic = 87u64)]
    ShadowMap6 = 87,
    #[br(magic = 88u64)]
    ShadowMap7 = 88,
    #[br(magic = 89u64)]
    CastShadow = 89,
    #[br(magic = 90u64)]
    ReceiveShadow = 90,
    #[br(magic = 91u64)]
    ShadowMapSampler = 91,
    #[br(magic = 107u64)]
    Texture15 = 107,
    #[br(magic = 108u64)]
    Sampler0 = 108,
    #[br(magic = 109u64)]
    Sampler1 = 109,
    #[br(magic = 110u64)]
    Sampler2 = 110,
    #[br(magic = 111u64)]
    Sampler3 = 111,
    #[br(magic = 112u64)]
    Sampler4 = 112,
    #[br(magic = 113u64)]
    Sampler5 = 113,
    #[br(magic = 114u64)]
    Sampler6 = 114,
    #[br(magic = 115u64)]
    Sampler7 = 115,
    #[br(magic = 116u64)]
    Sampler8 = 116,
    #[br(magic = 117u64)]
    Sampler9 = 117,
    #[br(magic = 118u64)]
    Sampler10 = 118,
    #[br(magic = 119u64)]
    Sampler11 = 119,
    #[br(magic = 120u64)]
    Sampler12 = 120,
    #[br(magic = 121u64)]
    Sampler13 = 121,
    #[br(magic = 122u64)]
    Sampler14 = 122,
    #[br(magic = 123u64)]
    Sampler15 = 123,
    #[br(magic = 124u64)]
    CustomBuffer0 = 124,
    #[br(magic = 125u64)]
    CustomBuffer1 = 125,
    #[br(magic = 126u64)]
    CustomBuffer2 = 126,
    #[br(magic = 127u64)]
    CustomBuffer3 = 127,
    #[br(magic = 128u64)]
    CustomBuffer4 = 128,
    #[br(magic = 129u64)]
    CustomBuffer5 = 129,
    #[br(magic = 130u64)]
    CustomBuffer6 = 130,
    #[br(magic = 131u64)]
    CustomBuffer7 = 131,
    #[br(magic = 132u64)]
    CustomMatrix0 = 132,
    #[br(magic = 133u64)]
    CustomMatrix1 = 133,
    #[br(magic = 134u64)]
    CustomMatrix2 = 134,
    #[br(magic = 135u64)]
    CustomMatrix3 = 135,
    #[br(magic = 136u64)]
    CustomMatrix4 = 136,
    #[br(magic = 137u64)]
    CustomMatrix5 = 137,
    #[br(magic = 138u64)]
    CustomMatrix6 = 138,
    #[br(magic = 139u64)]
    CustomMatrix7 = 139,
    #[br(magic = 140u64)]
    CustomMatrix8 = 140,
    #[br(magic = 141u64)]
    CustomMatrix9 = 141,
    #[br(magic = 142u64)]
    CustomMatrix10 = 142,
    #[br(magic = 143u64)]
    CustomMatrix11 = 143,
    #[br(magic = 144u64)]
    CustomMatrix12 = 144,
    #[br(magic = 145u64)]
    CustomMatrix13 = 145,
    #[br(magic = 146u64)]
    CustomMatrix14 = 146,
    #[br(magic = 147u64)]
    CustomMatrix15 = 147,
    #[br(magic = 148u64)]
    CustomMatrix16 = 148,
    #[br(magic = 149u64)]
    CustomMatrix17 = 149,
    #[br(magic = 150u64)]
    CustomMatrix18 = 150,
    #[br(magic = 151u64)]
    CustomMatrix19 = 151,
    #[br(magic = 164u64)]
    CustomVector12 = 164,
    #[br(magic = 169u64)]
    CustomVector17 = 169,
    #[br(magic = 172u64)]
    CustomColor0 = 172,
    #[br(magic = 173u64)]
    CustomColor1 = 173,
    #[br(magic = 174u64)]
    CustomColor2 = 174,
    #[br(magic = 175u64)]
    CustomColor3 = 175,
    #[br(magic = 176u64)]
    CustomColor4 = 176,
    #[br(magic = 177u64)]
    CustomColor5 = 177,
    #[br(magic = 178u64)]
    CustomColor6 = 178,
    #[br(magic = 179u64)]
    CustomColor7 = 179,
    #[br(magic = 180u64)]
    CustomColor8 = 180,
    #[br(magic = 181u64)]
    CustomColor9 = 181,
    #[br(magic = 182u64)]
    CustomColor10 = 182,
    #[br(magic = 183u64)]
    CustomColor11 = 183,
    #[br(magic = 184u64)]
    CustomColor12 = 184,
    #[br(magic = 185u64)]
    CustomColor13 = 185,
    #[br(magic = 186u64)]
    CustomColor14 = 186,
    #[br(magic = 187u64)]
    CustomColor15 = 187,
    #[br(magic = 188u64)]
    CustomColor16 = 188,
    #[br(magic = 189u64)]
    CustomColor17 = 189,
    #[br(magic = 190u64)]
    CustomColor18 = 190,
    #[br(magic = 191u64)]
    CustomColor19 = 191,
    #[br(magic = 194u64)]
    CustomFloat2 = 194,
    #[br(magic = 195u64)]
    CustomFloat3 = 195,
    #[br(magic = 197u64)]
    CustomFloat5 = 197,
    #[br(magic = 199u64)]
    CustomFloat7 = 199,
    #[br(magic = 201u64)]
    CustomFloat9 = 201,
    #[br(magic = 205u64)]
    CustomFloat13 = 205,
    #[br(magic = 206u64)]
    CustomFloat14 = 206,
    #[br(magic = 207u64)]
    CustomFloat15 = 207,
    #[br(magic = 208u64)]
    CustomFloat16 = 208,
    #[br(magic = 212u64)]
    CustomInteger0 = 212,
    #[br(magic = 213u64)]
    CustomInteger1 = 213,
    #[br(magic = 214u64)]
    CustomInteger2 = 214,
    #[br(magic = 215u64)]
    CustomInteger3 = 215,
    #[br(magic = 216u64)]
    CustomInteger4 = 216,
    #[br(magic = 217u64)]
    CustomInteger5 = 217,
    #[br(magic = 218u64)]
    CustomInteger6 = 218,
    #[br(magic = 219u64)]
    CustomInteger7 = 219,
    #[br(magic = 220u64)]
    CustomInteger8 = 220,
    #[br(magic = 221u64)]
    CustomInteger9 = 221,
    #[br(magic = 222u64)]
    CustomInteger10 = 222,
    #[br(magic = 223u64)]
    CustomInteger11 = 223,
    #[br(magic = 224u64)]
    CustomInteger12 = 224,
    #[br(magic = 225u64)]
    CustomInteger13 = 225,
    #[br(magic = 226u64)]
    CustomInteger14 = 226,
    #[br(magic = 227u64)]
    CustomInteger15 = 227,
    #[br(magic = 228u64)]
    CustomInteger16 = 228,
    #[br(magic = 229u64)]
    CustomInteger17 = 229,
    #[br(magic = 230u64)]
    CustomInteger18 = 230,
    #[br(magic = 231u64)]
    CustomInteger19 = 231,
    #[br(magic = 245u64)]
    CustomBoolean13 = 245,
    #[br(magic = 246u64)]
    CustomBoolean14 = 246,
    #[br(magic = 247u64)]
    CustomBoolean15 = 247,
    #[br(magic = 248u64)]
    CustomBoolean16 = 248,
    #[br(magic = 249u64)]
    CustomBoolean17 = 249,
    #[br(magic = 250u64)]
    CustomBoolean18 = 250,
    #[br(magic = 251u64)]
    CustomBoolean19 = 251,
    #[br(magic = 252u64)]
    UvTransform0 = 252,
    #[br(magic = 253u64)]
    UvTransform1 = 253,
    #[br(magic = 254u64)]
    UvTransform2 = 254,
    #[br(magic = 255u64)]
    UvTransform3 = 255,
    #[br(magic = 256u64)]
    UvTransform4 = 256,
    #[br(magic = 257u64)]
    UvTransform5 = 257,
    #[br(magic = 258u64)]
    UvTransform6 = 258,
    #[br(magic = 259u64)]
    UvTransform7 = 259,
    #[br(magic = 260u64)]
    UvTransform8 = 260,
    #[br(magic = 261u64)]
    UvTransform9 = 261,
    #[br(magic = 262u64)]
    UvTransform10 = 262,
    #[br(magic = 263u64)]
    UvTransform11 = 263,
    #[br(magic = 264u64)]
    UvTransform12 = 264,
    #[br(magic = 265u64)]
    UvTransform13 = 265,
    #[br(magic = 266u64)]
    UvTransform14 = 266,
    #[br(magic = 267u64)]
    UvTransform15 = 267,
    #[br(magic = 268u64)]
    DiffuseUvTransform1 = 268,
    #[br(magic = 269u64)]
    DiffuseUvTransform2 = 269,
    #[br(magic = 270u64)]
    SpecularUvTransform1 = 270,
    #[br(magic = 271u64)]
    SpecularUvTransform2 = 271,
    #[br(magic = 272u64)]
    NormalUvTransform1 = 272,
    #[br(magic = 273u64)]
    NormalUvTransform2 = 273,
    #[br(magic = 274u64)]
    DiffuseUvTransform = 274,
    #[br(magic = 275u64)]
    SpecularUvTransform = 275,
    #[br(magic = 276u64)]
    NormalUvTransform = 276,
    #[br(magic = 277u64)]
    UseDiffuseUvTransform = 277,
    #[br(magic = 278u64)]
    UseSpecularUvTransform = 278,
    #[br(magic = 279u64)]
    UseNormalUvTransform = 279,
    #[br(magic = 281u64)]
    BlendState1 = 281,
    #[br(magic = 282u64)]
    BlendState2 = 282,
    #[br(magic = 283u64)]
    BlendState3 = 283,
    #[br(magic = 284u64)]
    BlendState4 = 284,
    #[br(magic = 285u64)]
    BlendState5 = 285,
    #[br(magic = 286u64)]
    BlendState6 = 286,
    #[br(magic = 287u64)]
    BlendState7 = 287,
    #[br(magic = 288u64)]
    BlendState8 = 288,
    #[br(magic = 289u64)]
    BlendState9 = 289,
    #[br(magic = 290u64)]
    BlendState10 = 290,
    #[br(magic = 292u64)]
    RasterizerState1 = 292,
    #[br(magic = 293u64)]
    RasterizerState2 = 293,
    #[br(magic = 294u64)]
    RasterizerState3 = 294,
    #[br(magic = 295u64)]
    RasterizerState4 = 295,
    #[br(magic = 296u64)]
    RasterizerState5 = 296,
    #[br(magic = 297u64)]
    RasterizerState6 = 297,
    #[br(magic = 298u64)]
    RasterizerState7 = 298,
    #[br(magic = 299u64)]
    RasterizerState8 = 299,
    #[br(magic = 300u64)]
    RasterizerState9 = 300,
    #[br(magic = 301u64)]
    RasterizerState10 = 301,
    #[br(magic = 302u64)]
    ShadowColor = 302,
    #[br(magic = 303u64)]
    EmissiveMapLayer1 = 303,
    #[br(magic = 304u64)]
    EmissiveMapLayer2 = 304,
    #[br(magic = 305u64)]
    AlphaTestFunc = 305,
    #[br(magic = 306u64)]
    AlphaTestRef = 306,
    #[br(magic = 308u64)]
    Texture17 = 308,
    #[br(magic = 309u64)]
    Texture18 = 309,
    #[br(magic = 310u64)]
    Texture19 = 310,
    #[br(magic = 311u64)]
    Sampler16 = 311,
    #[br(magic = 312u64)]
    Sampler17 = 312,
    #[br(magic = 313u64)]
    Sampler18 = 313,
    #[br(magic = 314u64)]
    Sampler19 = 314,
    #[br(magic = 320u64)]
    CustomVector25 = 320,
    #[br(magic = 321u64)]
    CustomVector26 = 321,
    #[br(magic = 323u64)]
    CustomVector28 = 323,
    #[br(magic = 331u64)]
    CustomVector36 = 331,
    #[br(magic = 336u64)]
    CustomVector41 = 336,
    #[br(magic = 343u64)]
    CustomVector48 = 343,
    #[br(magic = 344u64)]
    CustomVector49 = 344,
    #[br(magic = 345u64)]
    CustomVector50 = 345,
    #[br(magic = 346u64)]
    CustomVector51 = 346,
    #[br(magic = 347u64)]
    CustomVector52 = 347,
    #[br(magic = 348u64)]
    CustomVector53 = 348,
    #[br(magic = 349u64)]
    CustomVector54 = 349,
    #[br(magic = 350u64)]
    CustomVector55 = 350,
    #[br(magic = 351u64)]
    CustomVector56 = 351,
    #[br(magic = 352u64)]
    CustomVector57 = 352,
    #[br(magic = 353u64)]
    CustomVector58 = 353,
    #[br(magic = 354u64)]
    CustomVector59 = 354,
    #[br(magic = 355u64)]
    CustomVector60 = 355,
    #[br(magic = 356u64)]
    CustomVector61 = 356,
    #[br(magic = 357u64)]
    CustomVector62 = 357,
    #[br(magic = 358u64)]
    CustomVector63 = 358,
    #[br(magic = 359u64)]
    UseBaseColorMap = 359,
    #[br(magic = 360u64)]
    UseMetallicMap = 360,
    #[br(magic = 361u64)]
    BaseColorMap = 361,
    #[br(magic = 362u64)]
    BaseColorMapLayer1 = 362,
    #[br(magic = 363u64)]
    MetallicMap = 363,
    #[br(magic = 364u64)]
    MetallicMapLayer1 = 364,
    #[br(magic = 365u64)]
    DiffuseLightingAoOffset = 365,
}

#[derive(Serialize, BinRead, Debug)]
#[br(import(param_type: ParamDataType))]
pub enum Param {
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

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum ParamDataType {
    #[br(magic = 0x1u64)]
    Float = 0x1,

    #[br(magic = 0x2u64)]
    Boolean = 0x2,

    #[br(magic = 0x5u64)]
    Vector4 = 0x5,

    #[br(magic = 0xBu64)]
    MatlString = 0xB,

    #[br(magic = 0xEu64)]
    Sampler = 0xE,

    #[br(magic = 0x10u64)]
    UvTransform = 0x10,

    #[br(magic = 0x11u64)]
    BlendState = 0x11,

    #[br(magic = 0x12u64)]
    RasterizerState = 0x12,
}

#[derive(Serialize, Debug)]
pub struct ParamData {
    pub data: Param,
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

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum FillMode {
    #[br(magic = 0u32)]
    Line = 0,
    #[br(magic = 1u32)]
    Solid = 1,
}

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum CullMode {
    #[br(magic = 0u32)]
    Back = 0,
    #[br(magic = 1u32)]
    Front = 1,
    #[br(magic = 2u32)]
    FrontAndBack = 2,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
pub struct MatlRasterizerState {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: u32
}

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum WrapMode {
    #[br(magic = 0u32)]
    Repeat = 0,
    #[br(magic = 1u32)]
    ClampToEdge = 1,
    #[br(magic = 2u32)]
    MirroredRepeat = 2,
    #[br(magic = 3u32)]
    ClampToBorder = 3,
}

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum MinFilter {
    #[br(magic = 0u32)]
    Nearest = 0,
    #[br(magic = 1u32)]
    LinearMipmapLinear = 1,
    #[br(magic = 2u32)]
    LinearMipmapLinear2 = 2,
}

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum MagFilter {
    #[br(magic = 0u32)]
    Nearest = 0,
    #[br(magic = 1u32)]
    Linear = 1,
    #[br(magic = 2u32)]
    Linear2 = 2,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
pub struct MatlSampler {
    pub wraps: WrapMode,
    pub wrapt: WrapMode,
    pub wrapr: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub lod_bias: f32,
    pub max_anisotropy: u32,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
pub struct MatlUvTransform {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub v: f32,
}

#[derive(Serialize, BinRead, Debug, Clone, PartialEq)]
pub struct MatlVec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Serialize, BinRead, Debug)]
pub struct MatlAttribute {
    pub param_id: ParamId,
    pub param: ParamData,
}

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub enum BlendFactor {
    #[br(magic = 0u32)]
    Zero = 0,

    #[br(magic = 1u32)]
    One = 1,

    #[br(magic = 2u32)]
    SourceAlpha = 2,

    #[br(magic = 3u32)]
    DestinationAlpha = 3,

    #[br(magic = 4u32)]
    SourceColor = 4,

    #[br(magic = 5u32)]
    DestinationColor = 5,

    #[br(magic = 6u32)]
    OneMinusSourceAlpha = 6,

    #[br(magic = 7u32)]
    OneMinusDestinationAlpha = 7,

    #[br(magic = 8u32)]
    OneMinusSourceColor = 8,

    #[br(magic = 9u32)]
    OneMinusDestinationColor = 9,

    #[br(magic = 10u32)]
    SourceAlphaSaturate = 10,
}

#[derive(Serialize, BinRead, Debug, Clone, Copy, PartialEq)]
pub struct MatlBlendState {
    pub source_color: BlendFactor,
    pub unk2: u32,
    pub destination_color: BlendFactor,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32
}

#[derive(Serialize, BinRead, Debug)]
pub struct MatlEntry {
    pub material_label: SsbhString,
    pub attributes: SsbhArray<MatlAttribute>,
    pub shader_label: SsbhString,
}

/// A container of materials.
#[derive(Serialize, BinRead, Debug)]
pub struct Matl {
    pub major_version: u16,
    pub minor_version: u16,
    pub entries: SsbhArray<MatlEntry>,
}
