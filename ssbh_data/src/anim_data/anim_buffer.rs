use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};
use bitbuffer::{BitReadBuffer, BitReadStream, LittleEndian};
use bitvec::{field::BitField, prelude::*};
use itertools::Itertools;
use modular_bitfield::prelude::*;
use std::{
    fmt::Debug,
    io::{Cursor, Read, Seek, Write},
    num::NonZeroU64,
};

use ssbh_write::SsbhWrite;

use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags},
    Ptr16, Ptr32, Vector3, Vector4,
};

use super::{AnimError, TrackValues, Transform, UvTransform};

// The bit_count values for compression types are 64 bits wide.
// This gives a theoretical upper limit of 2^65 - 1 bits for the compressed value.
// The current uncompressed track value types are all 32 bits or smaller.
// Smash Ultimate never uses bit counts above 24, so this gives a sensible representation of u32.
// TODO: It may be helpful to give an error or panic if more than 32 bits are specified for compression.
// TODO: Can we handle arbitrary bit lengths with acceptable performance?
type CompressedBits = u32;

// Use the highest bit count used for Smash Ultimate to avoid quality loss.
const DEFAULT_F32_BIT_COUNT: u64 = 24;

#[derive(Debug, BinRead, SsbhWrite)]
struct CompressedTrackData<T: CompressedData> {
    pub header: CompressedHeader<T>,
    pub compression: T::Compression,
}

#[derive(Debug, BinRead, SsbhWrite)]
struct CompressedHeader<T: CompressedData> {
    pub unk_4: u16,              // TODO: Always 4?
    pub flags: CompressionFlags, // TODO: These are used for texture transforms as well?
    pub default_data: Ptr16<T>,
    pub bits_per_entry: u16,
    pub compressed_data: Ptr32<CompressedBuffer>,
    pub frame_count: u32,
}

// TODO: This could be a shared function/type in lib.rs.
fn read_to_end<R: Read + Seek>(reader: &mut R, _ro: &ReadOptions, _: ()) -> BinResult<Vec<u8>> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}

#[derive(Debug, BinRead, SsbhWrite)]
#[ssbhwrite(alignment = 1)] // TODO: Is 1 byte alignment correct?
struct CompressedBuffer(#[br(parse_with = read_to_end)] Vec<u8>);

// TODO: Investigate these flags more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BitfieldSpecifier)]
#[bits = 2]
enum ScaleType {
    None = 0,
    ScaleNoInheritance = 1,
    Scale = 2,
    UniformScale = 3,
}

// TODO: This is redundant with the compression information.
// TODO: Does this determine transform inheritance instead?
// Determines what values are stored in the compressed bit buffer.
// Missing values are determined based on the compression's default values.
#[bitfield(bits = 16)]
#[derive(Debug, BinRead, Clone, Copy)]
#[br(map = Self::from_bytes)]
struct CompressionFlags {
    #[bits = 2]
    scale_type: ScaleType,
    has_rotation: bool,
    has_translation: bool,
    #[skip]
    __: B12,
}

ssbh_write::ssbh_write_modular_bitfield_impl!(CompressionFlags, 2);

#[derive(Debug, BinRead, Clone, SsbhWrite, Default)]
struct U32Compression {
    pub min: u32,
    pub max: u32,
    pub bit_count: u64,
}

impl Compression for U32Compression {
    fn bit_count(&self, _: CompressionFlags) -> u64 {
        self.bit_count
    }
}

// bools are always 1 bit.
impl Compression for u128 {
    fn bit_count(&self, _: CompressionFlags) -> u64 {
        1
    }
}

#[derive(Debug, BinRead, Clone, SsbhWrite, Default)]
struct F32Compression {
    pub min: f32,
    pub max: f32,
    pub bit_count: u64,
}

impl F32Compression {
    // TODO: Find a better name for this.
    // TODO: Add this to the trait?
    fn from_range(min: f32, max: f32) -> Self {
        let bit_count = if min == max { 0 } else { DEFAULT_F32_BIT_COUNT };

        Self {
            min,
            max,
            bit_count,
        }
    }
}

impl Compression for F32Compression {
    fn bit_count(&self, _: CompressionFlags) -> u64 {
        self.bit_count
    }
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct Vector3Compression {
    pub x: F32Compression,
    pub y: F32Compression,
    pub z: F32Compression,
}

impl Vector3Compression {
    fn from_range(min: Vector3, max: Vector3) -> Self {
        Self {
            x: F32Compression::from_range(min.x, max.x),
            y: F32Compression::from_range(min.y, max.y),
            z: F32Compression::from_range(min.z, max.z),
        }
    }
}

impl Compression for Vector3Compression {
    fn bit_count(&self, _: CompressionFlags) -> u64 {
        self.x.bit_count + self.y.bit_count + self.z.bit_count
    }
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct Vector4Compression {
    pub x: F32Compression,
    pub y: F32Compression,
    pub z: F32Compression,
    pub w: F32Compression,
}

impl Vector4Compression {
    fn from_range(min: Vector4, max: Vector4) -> Self {
        Self {
            x: F32Compression::from_range(min.x, max.x),
            y: F32Compression::from_range(min.y, max.y),
            z: F32Compression::from_range(min.z, max.z),
            w: F32Compression::from_range(min.w, max.w),
        }
    }
}

impl Compression for Vector4Compression {
    fn bit_count(&self, _: CompressionFlags) -> u64 {
        self.x.bit_count + self.y.bit_count + self.z.bit_count + self.w.bit_count
    }
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct TransformCompression {
    // The x component is used for uniform scale.
    pub scale: Vector3Compression,
    // The w component for rotation is handled separately.
    pub rotation: Vector3Compression,
    pub translation: Vector3Compression,
}

impl Compression for TransformCompression {
    fn bit_count(&self, flags: CompressionFlags) -> u64 {
        let mut bit_count = 0;

        bit_count += self.translation.bit_count(flags);

        match flags.scale_type() {
            ScaleType::Scale | ScaleType::ScaleNoInheritance => {
                bit_count += self.scale.bit_count(flags)
            }
            ScaleType::UniformScale => bit_count += self.scale.x.bit_count,
            _ => (),
        }

        // Three compressed floats and a single sign bit.
        bit_count += self.rotation.bit_count(flags);
        if flags.has_rotation() {
            bit_count += 1;
        }

        bit_count
    }
}

#[derive(Debug, BinRead, SsbhWrite, Default)]
struct UvTransformCompression {
    pub unk1: F32Compression,
    pub unk2: F32Compression,
    pub unk3: F32Compression,
    pub unk4: F32Compression,
    pub unk5: F32Compression,
}

impl Compression for UvTransformCompression {
    fn bit_count(&self, _: CompressionFlags) -> u64 {
        self.unk1.bit_count
            + self.unk2.bit_count
            + self.unk3.bit_count
            + self.unk4.bit_count
            + self.unk5.bit_count
    }
}

impl TrackValues {
    // TODO: Pass in scale inheritance and compensate scale?
    pub(crate) fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        compression: CompressionType,
        inherit_scale: bool,
    ) -> std::io::Result<()> {
        // TODO: Return an error on unsupported scale inheritance.
        // This occurs for inherit_scale = false and CompressionType::Uncompressed.
        // TODO: Make a new public error type for this.
        let transform_scale_type = if inherit_scale {
            // TODO: It's possible to optimize the case for uniform scale with inheritance.
            ScaleType::Scale
        } else {
            ScaleType::ScaleNoInheritance
        };

        // Only certain types use flags.
        // TODO: Make a function to test this?
        let flags = match self {
            TrackValues::Transform(_) => CompressionFlags::new()
                .with_scale_type(transform_scale_type)
                .with_has_rotation(true)
                .with_has_translation(true),
            // TODO: Do the flags matter for UV transforms?
            TrackValues::UvTransform(_) => CompressionFlags::new()
                .with_scale_type(ScaleType::ScaleNoInheritance)
                .with_has_rotation(true)
                .with_has_translation(true),
            _ => CompressionFlags::new(),
        };

        // TODO: More intelligently choose a bit count
        // For example, if min == max, bit count can be 0, which uses the default.
        // 2^bit_count evenly spaced values can just use bit_count.
        let bit_count = 24;

        // TODO: Find a way to simplify calculating the default and compression.
        // The default depends on the values.
        // The compression depends on the values and potentially a quality parameter.
        // ex: calculate_default(values), calculate_compression(values)
        match compression {
            CompressionType::Compressed => match self {
                TrackValues::Transform(values) => {
                    let min_scale = find_min_vector3(values.iter().map(|v| &v.scale));
                    let max_scale = find_max_vector3(values.iter().map(|v| &v.scale));

                    // TODO: Does the default quaternion W matter?
                    let min_rotation = find_min_vector4(values.iter().map(|v| &v.rotation));
                    let max_rotation = find_max_vector4(values.iter().map(|v| &v.rotation));

                    let min_translation = find_min_vector3(values.iter().map(|v| &v.translation));
                    let max_translation = find_max_vector3(values.iter().map(|v| &v.translation));

                    write_compressed(
                        writer,
                        values,
                        Transform {
                            scale: min_scale,
                            rotation: min_rotation, // TODO: How to choose a default quaternion?
                            translation: min_translation,
                            // Set to 1 if any of the values are 1.
                            // TODO: Is it possible to preserve per frame compensate scale for compressed transforms?
                            compensate_scale: values
                                .iter()
                                .map(|v| v.compensate_scale)
                                .max()
                                .unwrap_or(0),
                        },
                        TransformCompression {
                            scale: Vector3Compression::from_range(min_scale, max_scale),
                            rotation: Vector3Compression::from_range(
                                min_rotation.xyz(),
                                max_rotation.xyz(),
                            ),
                            translation: Vector3Compression::from_range(
                                min_translation,
                                max_translation,
                            ),
                        },
                        flags,
                    )?;
                }
                TrackValues::UvTransform(values) => {
                    let min_unk1 = find_min_f32(values.iter().map(|v| &v.scale_u));
                    let max_unk1 = find_max_f32(values.iter().map(|v| &v.scale_u));

                    let min_unk2 = find_min_f32(values.iter().map(|v| &v.scale_v));
                    let max_unk2 = find_max_f32(values.iter().map(|v| &v.scale_v));

                    let min_unk3 = find_min_f32(values.iter().map(|v| &v.rotation));
                    let max_unk3 = find_max_f32(values.iter().map(|v| &v.rotation));

                    let min_unk4 = find_min_f32(values.iter().map(|v| &v.translate_u));
                    let max_unk4 = find_max_f32(values.iter().map(|v| &v.translate_u));

                    let min_unk5 = find_min_f32(values.iter().map(|v| &v.translate_v));
                    let max_unk5 = find_max_f32(values.iter().map(|v| &v.translate_v));

                    write_compressed(
                        writer,
                        values,
                        // TODO: How to determine the default?
                        UvTransform {
                            scale_u: min_unk1,
                            scale_v: min_unk2,
                            rotation: min_unk3,
                            translate_u: min_unk4,
                            translate_v: min_unk5,
                        },
                        UvTransformCompression {
                            unk1: F32Compression::from_range(min_unk1, max_unk1),
                            unk2: F32Compression::from_range(min_unk2, max_unk2),
                            unk3: F32Compression::from_range(min_unk3, max_unk3),
                            unk4: F32Compression::from_range(min_unk4, max_unk4),
                            unk5: F32Compression::from_range(min_unk5, max_unk5),
                        },
                        flags,
                    )?;
                }
                TrackValues::Float(values) => {
                    let min = find_min_f32(values.iter());
                    let max = find_max_f32(values.iter());
                    write_compressed(
                        writer,
                        values,
                        min, // TODO: f32 default for compression?
                        F32Compression::from_range(min, max),
                        flags,
                    )?;
                }
                TrackValues::PatternIndex(values) => write_compressed(
                    writer,
                    values,
                    0, // TODO: Better default?
                    U32Compression {
                        min: values.iter().copied().min().unwrap_or(0),
                        max: values.iter().copied().max().unwrap_or(0),
                        bit_count,
                    },
                    flags,
                )?,
                TrackValues::Boolean(values) => write_compressed(
                    writer,
                    &values.iter().map(Boolean::from).collect_vec(),
                    Boolean(0u8),
                    0,
                    flags,
                )?,
                TrackValues::Vector4(values) => {
                    let min = find_min_vector4(values.iter());
                    let max = find_max_vector4(values.iter());

                    write_compressed(
                        writer,
                        values,
                        // TODO: Choose the correct default.
                        min,
                        Vector4Compression::from_range(min, max),
                        flags,
                    )?;
                }
            },
            _ => match self {
                TrackValues::Transform(values) => values.write(writer)?,
                TrackValues::UvTransform(values) => values.write(writer)?,
                TrackValues::Float(values) => values.write(writer)?,
                TrackValues::PatternIndex(values) => values.write(writer)?,
                TrackValues::Boolean(values) => {
                    let values: Vec<Boolean> = values.iter().map(Boolean::from).collect();
                    values.write(writer)?;
                }
                TrackValues::Vector4(values) => values.write(writer)?,
            },
        }

        Ok(())
    }

    // HACK: Use default since SsbhWrite expects self for size in bytes.
    pub(crate) fn compressed_overhead_in_bytes(&self) -> u64 {
        match self {
            TrackValues::Transform(_) => {
                <Transform as CompressedData>::compressed_overhead_in_bytes()
            }
            TrackValues::UvTransform(_) => {
                <UvTransform as CompressedData>::compressed_overhead_in_bytes()
            }
            TrackValues::Float(_) => <f32 as CompressedData>::compressed_overhead_in_bytes(),
            TrackValues::PatternIndex(_) => <u32 as CompressedData>::compressed_overhead_in_bytes(),
            TrackValues::Boolean(_) => <Boolean as CompressedData>::compressed_overhead_in_bytes(),
            TrackValues::Vector4(_) => <Vector4 as CompressedData>::compressed_overhead_in_bytes(),
        }
    }

    pub(crate) fn data_size_in_bytes(&self) -> u64 {
        match self {
            TrackValues::Transform(_) => Transform::default().size_in_bytes(),
            TrackValues::UvTransform(_) => UvTransform::default().size_in_bytes(),
            TrackValues::Float(_) => f32::default().size_in_bytes(),
            TrackValues::PatternIndex(_) => u32::default().size_in_bytes(),
            TrackValues::Boolean(_) => Boolean::default().size_in_bytes(),
            TrackValues::Vector4(_) => Vector4::default().size_in_bytes(),
        }
    }
}

// Return the value that isn't NaN for min and max.
fn find_min_f32<'a, I: Iterator<Item = &'a f32>>(values: I) -> f32 {
    values.copied().reduce(f32::min).unwrap_or(0.0)
}

fn find_max_f32<'a, I: Iterator<Item = &'a f32>>(values: I) -> f32 {
    values.copied().reduce(f32::max).unwrap_or(0.0)
}

fn find_min_vector3<'a, I: Iterator<Item = &'a Vector3>>(values: I) -> Vector3 {
    values
        .copied()
        .reduce(|a, b| Vector3::new(f32::min(a.x, b.x), f32::min(a.y, b.y), f32::min(a.z, b.z)))
        .unwrap_or(Vector3::new(0.0, 0.0, 0.0))
}

fn find_max_vector3<'a, I: Iterator<Item = &'a Vector3>>(values: I) -> Vector3 {
    values
        .copied()
        .reduce(|a, b| Vector3::new(f32::max(a.x, b.x), f32::max(a.y, b.y), f32::max(a.z, b.z)))
        .unwrap_or(Vector3::new(0.0, 0.0, 0.0))
}

fn find_min_vector4<'a, I: Iterator<Item = &'a Vector4>>(values: I) -> Vector4 {
    values
        .copied()
        .reduce(|a, b| {
            Vector4::new(
                f32::min(a.x, b.x),
                f32::min(a.y, b.y),
                f32::min(a.z, b.z),
                f32::min(a.w, b.w),
            )
        })
        .unwrap_or(Vector4::new(0.0, 0.0, 0.0, 0.0))
}

fn find_max_vector4<'a, I: Iterator<Item = &'a Vector4>>(values: I) -> Vector4 {
    values
        .copied()
        .reduce(|a, b| {
            Vector4::new(
                f32::max(a.x, b.x),
                f32::max(a.y, b.y),
                f32::max(a.z, b.z),
                f32::max(a.w, b.w),
            )
        })
        .unwrap_or(Vector4::new(0.0, 0.0, 0.0, 0.0))
}

fn write_compressed<W: Write + Seek, T: CompressedData>(
    writer: &mut W,
    values: &[T],
    default: T,
    compression: T::Compression,
    flags: CompressionFlags,
) -> Result<(), std::io::Error> {
    let compressed_data = create_compressed_buffer(values, &compression, flags);

    let data = CompressedTrackData::<T> {
        header: CompressedHeader::<T> {
            unk_4: 4,
            flags,
            default_data: Ptr16::new(default),
            bits_per_entry: compression.bit_count(flags) as u16, // TODO: This might overflow.
            compressed_data: Ptr32::new(CompressedBuffer(compressed_data)),
            frame_count: values.len() as u32,
        },
        compression,
    };
    data.write(writer)?;
    Ok(())
}

fn create_compressed_buffer<T: CompressedData>(
    values: &[T],
    compression: &T::Compression,
    flags: CompressionFlags,
) -> Vec<u8> {
    // Construct a single buffer and keep incrementing the bit index.
    // This essentially creates a bit writer buffered with u8 elements.
    // We already know the exact size, so there's no need to reallocate.
    let mut bits = BitVec::<Lsb0, u8>::new();
    bits.resize(values.len() * compression.bit_count(flags) as usize, false);

    let mut bit_index = 0;
    for v in values {
        v.compress(&mut bits, &mut bit_index, compression, flags);
    }

    bits.into_vec()
}

// Shared logic for compressing track data to and from bits.
trait CompressedData: BinRead<Args = ()> + SsbhWrite + Default {
    type Compression: Compression + std::fmt::Debug;
    type BitStore: BitStore;

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        flags: CompressionFlags,
    );

    // TODO: Find a way to do this with bitvec to avoid an extra dependency.
    fn decompress(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self>;

    // The size in bytes for the compressed header, default, and a single frame value.
    fn compressed_overhead_in_bytes() -> u64 {
        let header_size = 16;

        // TODO: If SsbhWrite::size_in_bytes didn't take self, we wouldn't need default here.
        // This may cause issues with the Option<T> type.
        header_size + Self::default().size_in_bytes() + Self::Compression::default().size_in_bytes()
    }
}

trait Compression: BinRead<Args = ()> + SsbhWrite + Default {
    fn bit_count(&self, flags: CompressionFlags) -> u64;
}

impl CompressedData for Transform {
    type Compression = TransformCompression;
    type BitStore = u32;

    fn decompress(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_transform_compressed(header, stream, compression, default)
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        flags: CompressionFlags,
    ) {
        match flags.scale_type() {
            // TODO: There's no way to access this value from the public API?
            // TODO: Find a way to expose scale inheritance.
            // TODO: Test different scale types and flags for writing.
            ScaleType::Scale | ScaleType::ScaleNoInheritance => {
                self.scale
                    .compress(bits, bit_index, &compression.scale, flags);
            }
            ScaleType::UniformScale => {
                self.scale
                    .x
                    .compress(bits, bit_index, &compression.scale.x, flags);
            }
            _ => (),
        }

        if flags.has_rotation() {
            self.rotation
                .xyz()
                .compress(bits, bit_index, &compression.rotation, flags);
        }

        if flags.has_translation() {
            self.translation
                .compress(bits, bit_index, &compression.translation, flags);
        }

        if flags.has_rotation() {
            // Add a single sign bit instead of storing w explicitly.
            *bits.get_mut(*bit_index).unwrap() = self.rotation.w.is_sign_negative();
            *bit_index += 1;
        }
    }
}

impl CompressedData for UvTransform {
    type Compression = UvTransformCompression;
    type BitStore = u32;

    fn decompress(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_uv_transform_compressed(header, stream, compression, default)
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        flags: CompressionFlags,
    ) {
        self.scale_u
            .compress(bits, bit_index, &compression.unk1, flags);
        self.scale_v
            .compress(bits, bit_index, &compression.unk2, flags);
        self.rotation
            .compress(bits, bit_index, &compression.unk3, flags);
        self.translate_u
            .compress(bits, bit_index, &compression.unk4, flags);
        self.translate_v
            .compress(bits, bit_index, &compression.unk5, flags);
    }
}

impl CompressedData for Vector3 {
    type Compression = Vector3Compression;
    type BitStore = u32;

    fn decompress(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_vector3_compressed(stream, compression, default)
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        flags: CompressionFlags,
    ) {
        self.x.compress(bits, bit_index, &compression.x, flags);
        self.y.compress(bits, bit_index, &compression.y, flags);
        self.z.compress(bits, bit_index, &compression.z, flags);
    }
}

impl CompressedData for Vector4 {
    type Compression = Vector4Compression;
    type BitStore = u32;

    fn decompress(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_vector4_compressed(stream, compression, default)
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        flags: CompressionFlags,
    ) {
        self.x.compress(bits, bit_index, &compression.x, flags);
        self.y.compress(bits, bit_index, &compression.y, flags);
        self.z.compress(bits, bit_index, &compression.z, flags);
        self.w.compress(bits, bit_index, &compression.w, flags);
    }
}

// TODO: Create a newtype for PatternIndex(u32)?
impl CompressedData for u32 {
    type Compression = U32Compression;
    type BitStore = u32;

    fn decompress(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        read_pattern_index_compressed(stream, compression, default)
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        _flags: CompressionFlags,
    ) {
        // TODO: This is just a guess.
        // TODO: Add a test case?
        let compressed_value = self - compression.min;
        bits[*bit_index..*bit_index + compression.bit_count as usize].store_le(compressed_value);
        *bit_index += compression.bit_count as usize;
    }
}

impl CompressedData for f32 {
    type Compression = F32Compression;
    type BitStore = u32;

    fn decompress(
        _header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        compression: &Self::Compression,
        default: &Self,
    ) -> bitbuffer::Result<Self> {
        Ok(read_compressed_f32(stream, compression)?.unwrap_or(*default))
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        compression: &Self::Compression,
        _flags: CompressionFlags,
    ) {
        if let Some(bit_count) = NonZeroU64::new(compression.bit_count as u64) {
            let compressed_value = compress_f32(*self, compression.min, compression.max, bit_count);
            bits[*bit_index..*bit_index + compression.bit_count as usize]
                .store_le(compressed_value);
            *bit_index += compression.bit_count as usize;
        }
    }
}

#[derive(Debug, BinRead, SsbhWrite, Default, PartialEq, Eq, Clone, Copy)]
pub struct Boolean(u8);

impl From<bool> for Boolean {
    fn from(v: bool) -> Self {
        Self::from(&v)
    }
}

impl From<&bool> for Boolean {
    fn from(v: &bool) -> Self {
        if *v {
            Self(1u8)
        } else {
            Self(0u8)
        }
    }
}

impl From<&Boolean> for bool {
    fn from(v: &Boolean) -> Self {
        v.0 != 0u8
    }
}

impl From<Boolean> for bool {
    fn from(v: Boolean) -> Self {
        Self::from(&v)
    }
}

impl CompressedData for Boolean {
    // There are 16 bytes for determining the compression, but all bytes are set to 0.
    type Compression = u128;
    type BitStore = u8;

    fn decompress(
        header: &CompressedHeader<Self>,
        stream: &mut BitReadStream<LittleEndian>,
        _: &Self::Compression,
        _: &Self,
    ) -> bitbuffer::Result<Self> {
        // Boolean compression is based on bits per entry, which is usually set to 1 bit.
        // TODO: 0 bits uses the default?
        let value = stream.read_int::<u8>(header.bits_per_entry as usize)?;
        Ok(Boolean(value))
    }

    fn compress(
        &self,
        bits: &mut BitSlice<Lsb0, u8>,
        bit_index: &mut usize,
        _: &Self::Compression,
        _: CompressionFlags,
    ) {
        *bits.get_mut(*bit_index).unwrap() = self.into();
        *bit_index += 1;
    }
}

fn read_direct<R: Read + Seek, T: BinRead>(
    reader: &mut R,
    frame_count: usize,
) -> BinResult<Vec<T>> {
    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value: T = reader.read_le()?;
        values.push(value);
    }
    Ok(values)
}

pub fn read_track_values(
    track_data: &[u8],
    flags: TrackFlags,
    count: usize,
) -> Result<(TrackValues, bool), AnimError> {
    // TODO: Are Const, ConstTransform, and Direct all the same?
    // TODO: Can frame count be higher than 1 for Const and ConstTransform?
    use crate::anim_data::TrackType as TrackTy;
    use crate::anim_data::TrackValues as Values;

    let mut reader = Cursor::new(track_data);

    let (values, inherit_scale) = match flags.compression_type {
        CompressionType::Compressed => match flags.track_type {
            TrackTy::Transform => {
                // TODO: Is there a cleaner way to get the scale inheritance information?
                let (values, inherit_scale) = read_compressed_transforms(&mut reader, count)?;
                (Values::Transform(values), inherit_scale)
            }
            TrackTy::UvTransform => (
                Values::UvTransform(read_compressed(&mut reader, count)?),
                false,
            ),
            TrackTy::Float => (Values::Float(read_compressed(&mut reader, count)?), false),
            TrackTy::PatternIndex => (
                Values::PatternIndex(read_compressed(&mut reader, count)?),
                false,
            ),
            TrackTy::Boolean => {
                // TODO: This could be handled by the CompressedData trait.
                let values: Vec<Boolean> = read_compressed(&mut reader, count)?;
                (
                    Values::Boolean(values.iter().map(bool::from).collect()),
                    false,
                )
            }
            TrackTy::Vector4 => (Values::Vector4(read_compressed(&mut reader, count)?), false),
        },
        _ => match flags.track_type {
            // TODO: Is it always true that uncompressed transform tracks inherit scale?
            TrackTy::Transform => (Values::Transform(read_direct(&mut reader, count)?), true),
            TrackTy::UvTransform => (Values::UvTransform(read_direct(&mut reader, count)?), false),
            TrackTy::Float => (Values::Float(read_direct(&mut reader, count)?), false),
            TrackTy::PatternIndex => (
                Values::PatternIndex(read_direct(&mut reader, count)?),
                false,
            ),
            TrackTy::Boolean => {
                let values = read_direct(&mut reader, count)?;
                (
                    Values::Boolean(values.iter().map(bool::from).collect_vec()),
                    false,
                )
            }
            TrackTy::Vector4 => (Values::Vector4(read_direct(&mut reader, count)?), false),
        },
    };

    // TODO: Determine inheritance from the compression flags?
    Ok((values, inherit_scale))
}

// TODO: Avoid duplication of this code.
fn read_compressed<R: Read + Seek, T: CompressedData + std::fmt::Debug>(
    reader: &mut R,
    frame_count: usize,
) -> Result<Vec<T>, AnimError> {
    let data: CompressedTrackData<T> = reader.read_le()?;

    // TODO: Return an error if the header has null pointers.
    // Decompress values.
    let bit_buffer = BitReadBuffer::new(
        &data.header.compressed_data.as_ref().unwrap().0,
        bitbuffer::LittleEndian,
    );
    let mut bit_reader = BitReadStream::new(bit_buffer);

    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value = T::decompress(
            &data.header,
            &mut bit_reader,
            &data.compression,
            data.header.default_data.as_ref().unwrap(),
        )?;
        values.push(value);
    }

    Ok(values)
}

fn read_compressed_transforms<R: Read + Seek>(
    reader: &mut R,
    frame_count: usize,
) -> Result<(Vec<Transform>, bool), AnimError> {
    let data: CompressedTrackData<Transform> = reader.read_le()?;

    // TODO: Is this the best way to handle this?
    let inherit_scale = data.header.flags.scale_type() != ScaleType::ScaleNoInheritance;

    // TODO: Return an error if the header has null pointers.
    // Decompress values.
    let bit_buffer = BitReadBuffer::new(
        &data.header.compressed_data.as_ref().unwrap().0,
        bitbuffer::LittleEndian,
    );
    let mut bit_reader = BitReadStream::new(bit_buffer);

    let mut values = Vec::new();
    for _ in 0..frame_count {
        let value = Transform::decompress(
            &data.header,
            &mut bit_reader,
            &data.compression,
            data.header.default_data.as_ref().unwrap(),
        )?;
        values.push(value);
    }

    Ok((values, inherit_scale))
}

fn read_transform_compressed(
    header: &CompressedHeader<Transform>,
    bit_stream: &mut BitReadStream<LittleEndian>,
    compression: &TransformCompression,
    default: &Transform,
) -> bitbuffer::Result<Transform> {
    let scale = match header.flags.scale_type() {
        ScaleType::UniformScale => {
            let uniform_scale =
                read_compressed_f32(bit_stream, &compression.scale.x)?.unwrap_or(default.scale.x);
            Vector3::new(uniform_scale, uniform_scale, uniform_scale)
        }
        _ => read_vector3_compressed(bit_stream, &compression.scale, &default.scale)?,
    };

    let rotation_xyz =
        read_vector3_compressed(bit_stream, &compression.rotation, &default.rotation.xyz())?;

    let translation =
        read_vector3_compressed(bit_stream, &compression.translation, &default.translation)?;

    let rotation_w = if header.flags.has_rotation() {
        calculate_rotation_w(bit_stream, rotation_xyz)
    } else {
        default.rotation.w
    };

    let rotation = Vector4::new(rotation_xyz.x, rotation_xyz.y, rotation_xyz.z, rotation_w);

    Ok(Transform {
        scale,
        rotation,
        translation,
        // Compressed transforms don't allow specifying compensate scale per frame.
        compensate_scale: default.compensate_scale,
    })
}

fn calculate_rotation_w(bit_stream: &mut BitReadStream<LittleEndian>, rotation: Vector3) -> f32 {
    // Rotations are encoded as xyzw unit quaternions,
    // so x^2 + y^2 + z^2 + w^2 = 1.
    // Solving for the missing w gives two expressions:
    // w = sqrt(1 - x^2 + y^2 + z^2) or -sqrt(1 - x^2 + y^2 + z^2).
    // Thus, we need only need to store the sign bit to uniquely determine w.
    let flip_w = bit_stream.read_bool().unwrap();

    let w2 = 1.0 - (rotation.x * rotation.x + rotation.y * rotation.y + rotation.z * rotation.z);
    // TODO: Is this the right approach to preventing NaN?
    let w = if w2.is_sign_negative() {
        0.0
    } else {
        w2.sqrt()
    };

    if flip_w {
        -w
    } else {
        w
    }
}

fn read_pattern_index_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &U32Compression,
    _default: &u32,
) -> bitbuffer::Result<u32> {
    // TODO: There's only a single track in Smash Ultimate that uses this, so this is just a guess.
    // TODO: How to compress a u32 with min, max, and bitcount?
    let value: u32 = bit_stream.read_int(compression.bit_count as usize)?;
    Ok(value + compression.min)
}

fn read_uv_transform_compressed(
    header: &CompressedHeader<UvTransform>,
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &UvTransformCompression,
    default: &UvTransform,
) -> bitbuffer::Result<UvTransform> {
    // UvTransforms use similar logic to Transforms.
    let (scale_u, scale_v) = match header.flags.scale_type() {
        ScaleType::UniformScale => {
            let uniform_scale =
                read_compressed_f32(bit_stream, &compression.unk1)?.unwrap_or(default.scale_u);
            (uniform_scale, uniform_scale)
        }
        _ => {
            let scale_u =
                read_compressed_f32(bit_stream, &compression.unk1)?.unwrap_or(default.scale_u);
            let scale_v =
                read_compressed_f32(bit_stream, &compression.unk2)?.unwrap_or(default.scale_v);
            (scale_u, scale_v)
        }
    };

    // TODO: How do flags affect these values?
    let rotation = read_compressed_f32(bit_stream, &compression.unk3)?.unwrap_or(default.rotation);
    let translate_u =
        read_compressed_f32(bit_stream, &compression.unk4)?.unwrap_or(default.translate_u);
    let translate_v =
        read_compressed_f32(bit_stream, &compression.unk5)?.unwrap_or(default.translate_v);

    Ok(UvTransform {
        scale_u,
        scale_v,
        rotation,
        translate_u,
        translate_v,
    })
}

fn read_vector4_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &Vector4Compression,
    default: &Vector4,
) -> bitbuffer::Result<Vector4> {
    let x = read_compressed_f32(bit_stream, &compression.x)?.unwrap_or(default.x);
    let y = read_compressed_f32(bit_stream, &compression.y)?.unwrap_or(default.y);
    let z = read_compressed_f32(bit_stream, &compression.z)?.unwrap_or(default.z);
    let w = read_compressed_f32(bit_stream, &compression.w)?.unwrap_or(default.w);
    Ok(Vector4::new(x, y, z, w))
}

fn read_vector3_compressed(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &Vector3Compression,
    default: &Vector3,
) -> bitbuffer::Result<Vector3> {
    let x = read_compressed_f32(bit_stream, &compression.x)?.unwrap_or(default.x);
    let y = read_compressed_f32(bit_stream, &compression.y)?.unwrap_or(default.y);
    let z = read_compressed_f32(bit_stream, &compression.z)?.unwrap_or(default.z);
    Ok(Vector3::new(x, y, z))
}

fn read_compressed_f32(
    bit_stream: &mut BitReadStream<bitbuffer::LittleEndian>,
    compression: &F32Compression,
) -> bitbuffer::Result<Option<f32>> {
    match NonZeroU64::new(compression.bit_count as u64) {
        Some(bit_count) => {
            // TODO: Scale type 2 seems to allow actual scaling?
            if compression.min == compression.max {
                Ok(None)
            } else {
                let value = bit_stream.read_int(bit_count.get() as usize)?;
                Ok(decompress_f32(
                    value,
                    compression.min,
                    compression.max,
                    bit_count,
                ))
            }
        }
        None => Ok(None),
    }
}

fn bit_mask(bit_count: NonZeroU64) -> u64 {
    // Get a mask of bit_count many bits set to 1.
    // Don't allow zero to avoid overflow.
    // TODO: handle the case where bit_count is extremely large?
    (1u64 << bit_count.get()) - 1u64
}

fn compress_f32(value: f32, min: f32, max: f32, bit_count: NonZeroU64) -> CompressedBits {
    // The inverse operation of decompression.
    // We don't allow bit_count to be zero.
    // This prevents divide by zero.
    let scale = bit_mask(bit_count);

    // There could be large errors due to cancellations when the absolute difference of max and min is small.
    // This is likely rare in practice.
    let ratio = (value - min) / (max - min);
    let compressed = ratio * scale as f32;
    compressed as CompressedBits
}

// TODO: It should be possible to test the edge cases by debugging Smash running in an emulator.
// Ex: Create a vector4 animation with all frames set to the same compressed value and inspect the uniform buffer.
fn decompress_f32(value: CompressedBits, min: f32, max: f32, bit_count: NonZeroU64) -> Option<f32> {
    // Anim supports custom ranges and non standard bit counts for fine tuning compression.
    // Unsigned normalized u8 would use min: 0.0, max: 1.0, and bit_count: 8.
    // This produces 2 ^ 8 evenly spaced floating point values between 0.0 and 1.0,
    // so 0b00000000 corresponds to 0.0 and 0b11111111 corresponds to 1.0.

    // We don't allow zero to prevent divide by zero.
    let scale = bit_mask(bit_count);

    // TODO: There may be some edge cases with this implementation of linear interpolation.
    // TODO: What happens when value > scale?
    let lerp = |a, b, t| a * (1.0 - t) + b * t;
    let value = lerp(min, max, value as f32 / scale as f32);

    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_hex_eq;
    use hexlit::hex;
    use ssbh_lib::formats::anim::TrackType;

    #[test]
    fn bit_masks() {
        assert_eq!(0b1u64, bit_mask(NonZeroU64::new(1).unwrap()));
        assert_eq!(0b11u64, bit_mask(NonZeroU64::new(2).unwrap()));
        assert_eq!(0b111111111u64, bit_mask(NonZeroU64::new(9).unwrap()));
    }

    #[test]
    fn compress_float_8bit() {
        let bit_count = NonZeroU64::new(8).unwrap();
        for i in 0..=255u8 {
            assert_eq!(
                i as CompressedBits,
                compress_f32(i as f32 / u8::MAX as f32, 0.0, 1.0, bit_count)
            );
        }
    }

    #[test]
    fn decompress_float_8bit() {
        let bit_count = NonZeroU64::new(8).unwrap();
        for i in 0..=255u8 {
            assert_eq!(
                Some(i as f32 / u8::MAX as f32),
                decompress_f32(i as CompressedBits, 0.0, 1.0, bit_count)
            );
        }
    }

    #[test]
    fn decompress_float_14bit() {
        // stage/poke_unova/battle/motion/s13_a, D_lightning_B, CustomVector3
        assert_eq!(
            Some(1.254_003_3),
            decompress_f32(2350, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            Some(1.185_819_5),
            decompress_f32(2654, 0.0, 7.32, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            Some(2.964_048_1),
            decompress_f32(2428, 0.0, 20.0, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            Some(1.218_784_5),
            decompress_f32(2284, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
    }

    #[test]
    fn compress_float_14bit() {
        // stage/poke_unova/battle/motion/s13_a, D_lightning_B, CustomVector3
        assert_eq!(
            2350,
            compress_f32(1.254_003_3, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2654,
            compress_f32(1.185_819_5, 0.0, 7.32, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2428,
            compress_f32(2.964_048_1, 0.0, 20.0, NonZeroU64::new(14).unwrap())
        );
        assert_eq!(
            2284,
            compress_f32(1.218_784_5, 0.0, 8.74227, NonZeroU64::new(14).unwrap())
        );
    }

    #[test]
    fn compress_decompress_float_24bit() {
        assert_eq!(
            bit_mask(NonZeroU64::new(24).unwrap()) as CompressedBits,
            compress_f32(1.0, -1.0, 1.0, NonZeroU64::new(24).unwrap())
        );

        assert_eq!(
            1.0,
            decompress_f32(
                bit_mask(NonZeroU64::new(24).unwrap()) as CompressedBits,
                -1.0,
                1.0,
                NonZeroU64::new(24).unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn read_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let data = hex!(cdcccc3e 0000c03f 0000803f 0000803f);
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert!(matches!(
            values,
            TrackValues::Vector4(values)
            if values== vec![Vector4::new(0.4, 1.5, 1.0, 1.0)]
        ));
    }

    #[test]
    fn write_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Vector4(vec![Vector4::new(0.4, 1.5, 1.0, 1.0)]),
            &mut writer,
            CompressionType::Constant,
            false
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!(cdcccc3e 0000c03f 0000803f 0000803f));
    }

    #[test]
    fn read_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let data = hex!(0000803f 0000803f 00000000 00000000 00000000);
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert!(matches!(
            values,
            TrackValues::UvTransform(values)
            if values == vec![
                UvTransform {
                    scale_u: 1.0,
                    scale_v: 1.0,
                    rotation: 0.0,
                    translate_u: 0.0,
                    translate_v: 0.0
                }
            ]
        ));
    }

    #[test]
    fn write_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::UvTransform(vec![UvTransform {
                scale_u: 1.0,
                scale_v: 1.0,
                rotation: 0.0,
                translate_u: 0.0,
                translate_v: 0.0,
            }]),
            &mut writer,
            CompressionType::Constant,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(0000803f 0000803f 00000000 00000000 00000000)
        );
    }

    #[test]
    fn read_compressed_uv_transform_multiple_frames() {
        // stage/kirby_greens/normal/motion/whispy_set/whispy_set_turnblowl3.nuanmb, _sfx_GrdGreensGrassAM1, nfTexture0[0]
        let data = hex!(
            // header
            04000900 60002600 74000000 14000000
            // scale compression
            2a8e633e 34a13d3f 0a000000 00000000
            cdcc4c3e 7a8c623f 0a000000 00000000
            // rotation compression
            00000000 00000000 10000000 00000000
            // translation compression
            ec51b8be bc7413bd 09000000 00000000
            a24536be e17a943e 09000000 00000000
            // default value
            34a13d3f 7a8c623f 00000000 bc7413bd a24536be
            // compressed values
            ffffff1f 80b4931a cfc12071 8de500e6 535555
        );

        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Compressed,
            },
            4,
        )
        .unwrap();

        // TODO: This is just a guess based on the flags.
        assert!(matches!(
            values,
            TrackValues::UvTransform(values)
            if values == vec![
                UvTransform {
                    scale_u: 0.740741,
                    scale_v: 0.884956,
                    rotation: 0.0,
                    translate_u: -0.036,
                    translate_v: -0.178
                },
                UvTransform {
                    scale_u: 0.5881758,
                    scale_v: 0.6412375,
                    rotation: 0.0,
                    translate_u: -0.0721409,
                    translate_v: -0.12579648
                },
                UvTransform {
                    scale_u: 0.4878173,
                    scale_v: 0.5026394,
                    rotation: 0.0,
                    translate_u: -0.1082818,
                    translate_v: -0.07359296
                },
                UvTransform {
                    scale_u: 0.4168567,
                    scale_v: 0.41291887,
                    rotation: 0.0,
                    translate_u: -0.14378865,
                    translate_v: -0.02230528
                }
            ]
        ));
    }

    #[test]
    fn read_compressed_uv_transform_multiple_frames_uniform_scale() {
        // fighter/mario/motion/body/c00/f01damageflymeteor.nuanmb, EyeL0_phong15__S_CUS_0x9ae11165_____NORMEXP16___VTC_, DiffuseUVTransform
        let data = hex!(
            // header
            04000B00 60001600 74000000 25000000
            // scale compression
            3333333F 9A99593F 08000000 00000000
            3333333F 9A99593F 10000000 00000000
            // rotation compression
            00000000 00000000 10000000 00000000
            // translation compression
            9A9919BE 9A9999BD 07000000 00000000
            9A99993D 9A99193E 07000000 00000000
            // default value
            9A99593F 9A99593F 00000000 9A9999BD 9A99993D
            // compressed values
            FF7FC0FF 1FF0FF07 FCFF01FF 7FC0FF1F
            F0FF07FC FF01FF7F C0FF1FF0 FF07FCFF
            01FF7FC0 FF1F108F 3F309B33 9B4D1999
            AC399331 3B1CF000 803F00E0 0F00F803
            00FE0080 3F00E00F 00F80300 FE00803F
            00E00F00 F80300FE 00803F00 E00F00F8 0300FE00 803F
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::UvTransform,
                compression_type: CompressionType::Compressed,
            },
            37,
        )
        .unwrap();

        // Just check for reading the correct count for now.
        // TODO: Check the scale values.
        assert!(matches!(values, TrackValues::UvTransform(v) if v.len() == 37));
    }

    #[test]
    fn write_compressed_uv_transform_multiple_frames() {
        let values = vec![
            UvTransform {
                scale_u: -1.0,
                scale_v: -2.0,
                rotation: -3.0,
                translate_u: -4.0,
                translate_v: -5.0,
            },
            UvTransform {
                scale_u: 1.0,
                scale_v: 2.0,
                rotation: 3.0,
                translate_u: 4.0,
                translate_v: 5.0,
            },
        ];
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::UvTransform(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        // TODO: How to determine a good default value?
        // TODO: Check more examples to see if default is just the min.
        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                // header
                04000d00 60007800 74000000 02000000
                // scale compression
                000080BF 0000803F 18000000 00000000
                000000C0 00000040 18000000 00000000
                // rotation compression
                000040C0 00004040 18000000 00000000
                // translation compression
                000080C0 00008040 18000000 00000000
                0000A0C0 0000A040 18000000 00000000
                // default value
                000080BF 000000C0 000040C0 000080C0 0000A0C0
                // compressed values
                000000 000000 000000 000000 000000
                FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF
            )
        );

        assert_eq!(
            values,
            read_compressed(&mut Cursor::new(writer.get_ref()), 2).unwrap()
        );
    }

    #[test]
    fn read_constant_pattern_index_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture0[0].PatternIndex
        let data = hex!("01000000");
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert!(matches!(values, TrackValues::PatternIndex(values) if values == vec![1]));
    }

    #[test]
    fn write_constant_pattern_index_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture0[0].PatternIndex
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::PatternIndex(vec![1]),
            &mut writer,
            CompressionType::Constant,
            false
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!(01000000));
    }

    #[test]
    fn read_compressed_pattern_index_multiple_frames() {
        // stage/fzero_mutecity3ds/normal/motion/s05_course/s05_course__l00b.nuanmb, phong32__S_CUS_0xa3c00501___NORMEXP16_, DiffuseUVTransform.PatternIndex.
        // Shortened from 650 to 8 frames.
        let data = hex!(
            04000000 20000100 24000000 8a020000 // header
            01000000 02000000 01000000 00000000 // compression
            01000000                            // default value
            fe                                  // compressed values
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::PatternIndex,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();

        // TODO: This is just a guess for min: 1, max: 2, bit_count: 1.
        assert!(matches!(
            values,
            TrackValues::PatternIndex(values)
            if values == vec![1, 2, 2, 2, 2, 2, 2, 2]
        ));
    }

    #[test]
    fn read_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let data = hex!(cdcccc3e);
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert!(matches!(values, TrackValues::Float(values) if values == vec![0.4]));
    }

    #[test]
    fn write_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Float(vec![0.4]),
            &mut writer,
            CompressionType::Constant,
            false,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!(cdcccc3e));
    }

    #[test]
    fn read_compressed_float_multiple_frames() {
        // pacman/model/body/c00/model.nuanmb, phong3__phong0__S_CUS_0xa2001001___7__AT_GREATER128___VTC__NORMEXP16___CULLNONE_A_AB_SORT, CustomFloat2
        let data = hex!(
            04000000 20000200 24000000 05000000 // header
            00000000 00004040 02000000 00000000 // compression
            00000000                            // default value
            e403                                // compressed values
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Float,
                compression_type: CompressionType::Compressed,
            },
            5,
        )
        .unwrap();

        assert!(
            matches!(values, TrackValues::Float(values) if values == vec![0.0, 1.0, 2.0, 3.0, 3.0])
        );
    }

    #[test]
    fn write_compressed_floats_multiple_frame() {
        // Test that the min/max and bit counts are used properly
        let values = vec![0.5, 2.0];
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Float(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                04000000 20001800 24000000 02000000 // header
                0000003F 00000040 18000000 00000000 // compression
                0000003F                            // default value
                000000 FFFFFF                       // compressed values
            )
        );

        assert_eq!(
            values,
            read_compressed(&mut Cursor::new(writer.get_ref()), 2).unwrap()
        );
    }

    #[test]
    fn read_constant_boolean_single_frame_true() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean1
        let data = hex!("01");
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert!(matches!(values, TrackValues::Boolean(values) if values == vec![true]));
    }

    #[test]
    fn write_constant_boolean_single_frame_true() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean1
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![true]),
            &mut writer,
            CompressionType::Constant,
            false
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!("01"));
    }

    #[test]
    fn read_constant_boolean_single_frame_false() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean11
        let data = hex!("00");
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert!(matches!(values, TrackValues::Boolean(values) if values == vec![false]));
    }

    #[test]
    fn read_compressed_boolean_multiple_frames() {
        // assist/ashley/motion/body/c00/vis.nuanmb, magic, Visibility
        let data = hex!(
            04000000 20000100 21000000 03000000 // header
            00000000 00000000 00000000 00000000 // bool compression (always 0's)
            0006                                // compressed values (bits)
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Boolean,
                compression_type: CompressionType::Compressed,
            },
            3,
        )
        .unwrap();

        assert!(matches!(
            values,
            TrackValues::Boolean(values)
            if values == vec![false, true, true]
        ));
    }

    #[test]
    fn write_compressed_boolean_single_frame() {
        // Test writing a single bit.
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![true]),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                04000000 20000100 21000000 01000000 // header
                00000000 00000000 00000000 00000000 // bool compression (always 0's)
                0001                                // compressed values (bits)
            )
        );

        assert_eq!(
            vec![Boolean(1)],
            read_compressed(&mut Cursor::new(writer.get_ref()), 1).unwrap()
        );
    }

    #[test]
    fn write_compressed_boolean_three_frames() {
        // assist/ashley/motion/body/c00/vis.nuanmb, magic, Visibility
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![false, true, true]),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                04000000 20000100 21000000 03000000 // header
                00000000 00000000 00000000 00000000 // bool compression (always 0's)
                0006                                // compressed values (bits)
            )
        );

        assert_eq!(
            vec![Boolean(0), Boolean(1), Boolean(1)],
            read_compressed(&mut Cursor::new(writer.get_ref()), 3).unwrap()
        );
    }

    #[test]
    fn write_compressed_boolean_multiple_frames() {
        // fighter/mario/motion/body/c00/a00wait3.nuanmb, MarioFaceN, Visibility
        // Shortened from 96 to 11 frames.
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Boolean(vec![true; 11]),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                04000000 20000100 21000000 0B000000 // header
                00000000 00000000 00000000 00000000 // bool compression (always 0's)
                00FF07                              // compressed values (bits)
            )
        );

        assert_eq!(
            vec![Boolean(1); 11],
            read_compressed(&mut Cursor::new(writer.get_ref()), 11).unwrap()
        );
    }

    #[test]
    fn read_compressed_vector4_multiple_frames() {
        // The default data (00000000 00000000 3108ac3d bc74133e)
        // uses the 0 bit count of one compression entry and the min/max of the next.
        // TODO: Is it worth adding code complexity to support this optimization?

        // fighter/cloud/motion/body/c00/b00guardon.nuanmb, EyeL, CustomVector31
        let data = hex!(
            // header
            04000000 50000300 60000000 08000000
            // xyzw compression
            0000803f 0000803f 00000000 00000000
            0000803f 0000803f 00000000 00000000
            3108ac3d bc74133e 03000000 00000000
            00000000 00000000 00000000 00000000
            // default value
            0000803f 0000803f 3108ac3d 00000000
            // compressed values
            88c6fa
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Vector4,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();

        assert!(matches!(values,
            TrackValues::Vector4(values)
            if values == vec![
                Vector4::new(1.0, 1.0, 0.084, 0.0),
                Vector4::new(1.0, 1.0, 0.09257142, 0.0),
                Vector4::new(1.0, 1.0, 0.10114286, 0.0),
                Vector4::new(1.0, 1.0, 0.109714285, 0.0),
                Vector4::new(1.0, 1.0, 0.118285716, 0.0),
                Vector4::new(1.0, 1.0, 0.12685715, 0.0),
                Vector4::new(1.0, 1.0, 0.13542856, 0.0),
                Vector4::new(1.0, 1.0, 0.144, 0.0)
            ]
        ));
    }

    #[test]
    fn write_compressed_vector4_multiple_frames() {
        let values = vec![
            Vector4::new(-1.0, -2.0, -3.0, -4.0),
            Vector4::new(1.0, 2.0, 3.0, 4.0),
        ];
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Vector4(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                // header
                04000000 50006000 60000000 02000000
                // xyzw compression
                000080BF 0000803F 18000000 00000000
                000000C0 00000040 18000000 00000000
                000040C0 00004040 18000000 00000000
                000080C0 00008040 18000000 00000000
                // default value
                000080BF 000000C0 000040C0 000080C0
                // compressed values
                000000 000000 000000 000000 FFFFFF FFFFFF FFFFFF FFFFFF
            )
        );

        assert_eq!(
            values,
            read_compressed(&mut Cursor::new(writer.get_ref()), 2).unwrap()
        );
    }

    #[test]
    fn write_compressed_vector4_multiple_frames_defaults() {
        let values = vec![
            Vector4::new(1.0, 2.0, 3.0, -4.0),
            Vector4::new(1.0, 2.0, 3.0, 4.0),
        ];
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Vector4(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                // header
                04000000 50001800 60000000 02000000
                // xyzw compression
                0000803F 0000803F 00000000 00000000
                00000040 00000040 00000000 00000000
                00004040 00004040 00000000 00000000
                000080C0 00008040 18000000 00000000
                // default value
                0000803F 00000040 00004040 000080C0
                // compressed values
                000000 FFFFFF
            )
        );

        assert_eq!(
            values,
            read_compressed(&mut Cursor::new(writer.get_ref()), 2).unwrap()
        );
    }

    #[test]
    fn read_constant_transform_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, FingerL11, Transform
        let data = hex!(
            0000803f 0000803f 0000803f          // scale
            00000000 00000000 00000000          // translation
            0000803f bea4c13f_79906ebe f641bebe // rotation
            01000000                            // compensate scale
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::ConstTransform,
            },
            1,
        )
        .unwrap();

        assert!(matches!(values,
            TrackValues::Transform(values)
            if values == vec![
                Transform {
                    translation: Vector3::new(1.51284, -0.232973, -0.371597),
                    rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                    compensate_scale: 1
                }
            ]
        ));
    }

    #[test]
    fn write_constant_transform_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, FingerL11, Transform
        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Transform(vec![Transform {
                translation: Vector3::new(1.51284, -0.232973, -0.371597),
                rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                scale: Vector3::new(1.0, 1.0, 1.0),
                compensate_scale: 1,
            }]),
            &mut writer,
            CompressionType::Constant,
            false
        )
        .unwrap();

        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                0000803f 0000803f 0000803f          // scale
                00000000 00000000 00000000          // translation
                0000803f bea4c13f_79906ebe f641bebe // rotation
                01000000                            // compensate scale
            )
        );
    }

    fn read_compressed_transform_scale_with_flags(flags: CompressionFlags, data_hex: Vec<u8>) {
        let default = Transform {
            scale: Vector3::new(4.0, 4.0, 4.0),
            rotation: Vector4::new(2.0, 2.0, 2.0, 2.0),
            translation: Vector3::new(3.0, 3.0, 3.0),
            compensate_scale: 0,
        };

        let header = CompressedHeader::<Transform> {
            unk_4: 4,
            flags,
            default_data: Ptr16::new(default),
            // TODO: Bits per entry shouldn't matter?
            bits_per_entry: 16,
            compressed_data: Ptr32::new(CompressedBuffer(data_hex.clone())),
            frame_count: 1,
        };
        let float_compression = F32Compression {
            min: 0.0,
            max: 0.0,
            bit_count: 0,
        };

        // Disable everything except scale.
        let compression = TransformCompression {
            scale: Vector3Compression {
                x: F32Compression {
                    min: 0.0,
                    max: 0.0,
                    bit_count: 0,
                },
                y: F32Compression {
                    min: 0.0,
                    max: 0.0,
                    bit_count: 0,
                },
                z: F32Compression {
                    min: 0.0,
                    max: 0.0,
                    bit_count: 0,
                },
            },
            rotation: Vector3Compression {
                x: float_compression.clone(),
                y: float_compression.clone(),
                z: float_compression.clone(),
            },
            translation: Vector3Compression {
                x: float_compression.clone(),
                y: float_compression.clone(),
                z: float_compression,
            },
        };
        let bit_buffer = BitReadBuffer::new(&data_hex, bitbuffer::LittleEndian);
        let mut bit_reader = BitReadStream::new(bit_buffer);

        let default = Transform {
            scale: Vector3::new(4.0, 4.0, 4.0),
            rotation: Vector4::new(2.0, 2.0, 2.0, 2.0),
            translation: Vector3::new(3.0, 3.0, 3.0),
            compensate_scale: 0,
        };
        read_transform_compressed(&header, &mut bit_reader, &compression, &default).unwrap();
    }

    #[test]
    fn read_scale_data_flags() {
        read_compressed_transform_scale_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::None),
            hex!("").to_vec(),
        );
        read_compressed_transform_scale_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::UniformScale),
            hex!("FF").to_vec(),
        );
        read_compressed_transform_scale_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::Scale),
            hex!("FFFFFF").to_vec(),
        );
        read_compressed_transform_scale_with_flags(
            CompressionFlags::new().with_scale_type(ScaleType::ScaleNoInheritance),
            hex!("FFFFFF").to_vec(),
        );
    }

    #[test]
    fn read_compressed_transform_multiple_frames() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        let data = hex!(
            // header
            04000600 a0002b00 cc000000 02000000
            // scale compression
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            0000803f 0000803f 10000000 00000000
            // rotation compression
            00000000 b9bc433d 0d000000 00000000
            e27186bd 00000000 0d000000 00000000
            00000000 ada2273f 10000000 00000000
            // translation compression
            16a41d40 16a41d40 10000000 00000000
            00000000 00000000 10000000 00000000
            00000000 00000000 10000000 00000000
            // default value
            0000803f 0000803f 0000803f
            00000000 00000000 00000000 0000803f
            16a41d40 00000000 00000000 00000000 00e0ff03
            // compressed values
            00f8ff00 e0ff1f
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::Compressed,
            },
            2,
        )
        .unwrap();

        assert!(matches!(values,
            TrackValues::Transform(values)
            if values == vec![
                Transform {
                    translation: Vector3::new(2.46314, 0.0, 0.0),
                    rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                    compensate_scale: 0
                },
                Transform {
                    translation: Vector3::new(2.46314, 0.0, 0.0),
                    rotation: Vector4::new(0.0477874, -0.0656469, 0.654826, 0.7514052),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                    compensate_scale: 0
                }
            ]
        ));
    }

    #[test]
    fn write_compressed_transform_multiple_frames() {
        let values = vec![
            Transform {
                translation: Vector3::new(-1.0, -2.0, -3.0),
                rotation: Vector4::new(-4.0, -5.0, -6.0, 0.0),
                scale: Vector3::new(-8.0, -9.0, -10.0),
                compensate_scale: 0,
            },
            Transform {
                translation: Vector3::new(1.0, 2.0, 3.0),
                rotation: Vector4::new(4.0, 5.0, 6.0, 0.0),
                scale: Vector3::new(8.0, 9.0, 10.0),
                compensate_scale: 0,
            },
        ];

        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Transform(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        // TODO: How to determine a good default value?
        // TODO: Check more examples to see if default is just the min.
        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                // header
                04000d00 a000d900 cc000000 02000000
                // scale compression
                000000C1 00000041 18000000 00000000
                000010C1 00001041 18000000 00000000
                000020C1 00002041 18000000 00000000
                // rotation compression
                000080C0 00008040 18000000 00000000
                0000A0C0 0000A040 18000000 00000000
                0000C0C0 0000C040 18000000 00000000
                // translation compression
                000080BF 0000803F 18000000 00000000
                000000C0 00000040 18000000 00000000
                000040C0 00004040 18000000 00000000
                // default value
                000000C1 000010C1 000020C1
                000080C0 0000A0C0 0000C0C0 00000000
                000080BF 000000C0 000040C0 00000000
                // compressed values
                000000 000000 000000 000000 000000 000000 000000 000000 000000
                FEFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF 01
            )
        );

        assert_eq!(
            values,
            read_compressed(&mut Cursor::new(writer.get_ref()), 2).unwrap()
        );
    }

    #[test]
    fn write_compressed_transform_multiple_frames_uniform_scale() {
        let values = vec![
            Transform {
                translation: Vector3::new(-1.0, -2.0, -3.0),
                rotation: Vector4::new(-4.0, -5.0, -6.0, 0.0),
                scale: Vector3::new(-8.0, -8.0, -8.0),
                compensate_scale: 0,
            },
            Transform {
                translation: Vector3::new(1.0, 2.0, 3.0),
                rotation: Vector4::new(4.0, 5.0, 6.0, 0.0),
                scale: Vector3::new(9.0, 9.0, 9.0),
                compensate_scale: 0,
            },
        ];

        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Transform(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false
        )
        .unwrap();

        // TODO: Check for optimizing for uniform scale with header 04000f00?
        assert_hex_eq!(
            writer.get_ref(),
            &hex!(
                // header
                04000d00 a000d900 cc000000 02000000
                // scale compression
                000000C1 00001041 18000000 00000000
                000000C1 00001041 18000000 00000000
                000000C1 00001041 18000000 00000000
                // rotation compression
                000080C0 00008040 18000000 00000000
                0000A0C0 0000A040 18000000 00000000
                0000C0C0 0000C040 18000000 00000000
                // translation compression
                000080BF 0000803F 18000000 00000000
                000000C0 00000040 18000000 00000000
                000040C0 00004040 18000000 00000000
                // default value
                000000C1 000000C1 000000C1
                000080C0 0000A0C0 0000C0C0 00000000
                000080BF 000000C0 000040C0 00000000
                // compressed values
                000000 000000 000000 000000 000000 000000 000000 000000 000000
                FEFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF FFFFFF 01
            )
        );

        assert_eq!(
            values,
            read_compressed(&mut Cursor::new(writer.get_ref()), 2).unwrap()
        );
    }

    #[test]
    fn read_direct_transform_multiple_frames() {
        // camera/fighter/ike/c00/d02finalstart.nuanmb, gya_camera, Transform
        // Shortened from 8 to 2 frames.
        let data = hex!(
            0000803f 0000803f 0000803f
            1dca203e 437216bf a002cbbd 5699493f
            9790e5c1 1f68a040 f7affa40 00000000 0000803f
            0000803f 0000803f c7d8093e
            336b19bf 5513e4bd e3fe473f
            6da703c2 dfc3a840 b8120b41 00000000
        );
        // TODO: Test inherit scale?
        let (values, _) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackType::Transform,
                compression_type: CompressionType::Direct,
            },
            2,
        )
        .unwrap();

        assert!(matches!(values,
            TrackValues::Transform(values)
            if values== vec![
                Transform {
                    translation: Vector3::new(-28.6956, 5.01271, 7.83398),
                    rotation: Vector4::new(0.157021, -0.587681, -0.0991261, 0.787496),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                    compensate_scale: 0
                },
                Transform {
                    translation: Vector3::new(-32.9135, 5.27391, 8.69207),
                    rotation: Vector4::new(0.134616, -0.599292, -0.111365, 0.781233),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                    compensate_scale: 0
                },
            ]
        ));
    }

    #[test]
    fn calculate_rotation_w_unit_quaternion() {
        let bit_buffer = BitReadBuffer::new(&[1u8], bitbuffer::LittleEndian);
        let mut bit_reader = BitReadStream::new(bit_buffer);
        assert_eq!(
            0.0,
            calculate_rotation_w(&mut bit_reader, Vector3::new(1.0, 0.0, 0.0))
        );
    }

    #[test]
    fn calculate_rotation_w_non_unit_quaternion() {
        let bit_buffer = BitReadBuffer::new(&[1u8], bitbuffer::LittleEndian);
        let mut bit_reader = BitReadStream::new(bit_buffer);

        // W isn't well defined in this case.
        // Just assume W is 0.0 when the square root would be negative.
        // TODO: There may be a better approach with better animation quality.
        assert_eq!(
            0.0,
            calculate_rotation_w(&mut bit_reader, Vector3::new(1.0, 1.0, 1.0))
        );
    }
}
