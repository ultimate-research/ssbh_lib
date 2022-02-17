use binread::{BinRead, BinReaderExt, BinResult};
use bitvec::prelude::*;
use itertools::Itertools;
use std::io::{Cursor, Read, Seek, Write};

use ssbh_write::SsbhWrite;

use ssbh_lib::{
    formats::anim::{CompressionType, TrackFlags},
    Ptr16, Ptr32, Vector4,
};

use super::{
    bitutils::{BitReader, BitWriter},
    compression::{
        CompressedBuffer, CompressedHeader, CompressedTrackData, Compression, CompressionFlags,
    },
};
use super::{compression::*, error::Error, ScaleOptions, TrackValues, Transform, UvTransform};

impl TrackValues {
    pub(crate) fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        compression: CompressionType,
        inherit_scale: bool,
        compensate_scale: bool,
    ) -> Result<(), Error> {
        // TODO: Find a way to simplify calculating the default and compression.
        // TODO: Find a way to clean up this code.
        // The default depends on the values.
        // The compression depends on the values and potentially a quality parameter.
        // ex: calculate_default(values), calculate_compression(values)

        match compression {
            CompressionType::Compressed => {
                let flags = CompressionFlags::from_track(self, inherit_scale);

                // TODO: More intelligently choose a bit count
                // For example, if min == max, bit count can be 0, which uses the default.
                // 2^bit_count evenly spaced values can just use bit_count.

                match self {
                    TrackValues::Transform(values) => write_compressed(
                        writer,
                        &values
                            .iter()
                            .map(|t| UncompressedTransform::from_transform(t, compensate_scale))
                            .collect_vec(),
                        flags,
                        compensate_scale,
                    )?,
                    TrackValues::UvTransform(values) => {
                        write_compressed(writer, values, flags, compensate_scale)?
                    }
                    TrackValues::Float(values) => {
                        write_compressed(writer, values, flags, compensate_scale)?
                    }
                    TrackValues::PatternIndex(values) => {
                        write_compressed(writer, values, flags, compensate_scale)?
                    }
                    TrackValues::Boolean(values) => write_compressed(
                        writer,
                        &values.iter().map(Boolean::from).collect_vec(),
                        flags,
                        compensate_scale,
                    )?,
                    TrackValues::Vector4(values) => {
                        write_compressed(writer, values, flags, compensate_scale)?
                    }
                }
            }
            _ => match self {
                TrackValues::Transform(values) => {
                    // Uncompressed transform tracks don't support disabling scale inheritance.
                    if !inherit_scale {
                        return Err(Error::UnsupportedTrackScaleOptions {
                            scale_options: ScaleOptions {
                                inherit_scale,
                                compensate_scale,
                            },
                            compressed: false,
                        });
                    }

                    let values: Vec<_> = values
                        .iter()
                        .map(|t| UncompressedTransform::from_transform(t, compensate_scale))
                        .collect();
                    values.write(writer)?;
                }
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
                <UncompressedTransform as CompressedData>::compressed_overhead_in_bytes()
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
            TrackValues::Transform(_) => UncompressedTransform::default().size_in_bytes(),
            TrackValues::UvTransform(_) => UvTransform::default().size_in_bytes(),
            TrackValues::Float(_) => f32::default().size_in_bytes(),
            TrackValues::PatternIndex(_) => u32::default().size_in_bytes(),
            TrackValues::Boolean(_) => Boolean::default().size_in_bytes(),
            TrackValues::Vector4(_) => Vector4::default().size_in_bytes(),
        }
    }
}

fn write_compressed<W: Write + Seek, T: CompressedData>(
    writer: &mut W,
    values: &[T],
    flags: CompressionFlags,
    compensate_scale: bool,
) -> Result<(), std::io::Error> {
    let (default, compression) = T::get_default_and_compression(values, compensate_scale);

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
    // TODO: Can we reserve size and append instead?
    let mut bits = BitVec::<u8, Lsb0>::new();
    bits.resize(values.len() * compression.bit_count(flags) as usize, false);
    let mut writer = BitWriter::new(bits);

    for v in values {
        v.compress(&mut writer, compression, flags);
    }

    writer.into_bytes()
}

fn read_uncompressed<R: Read + Seek, T: BinRead>(
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
) -> Result<(TrackValues, bool, bool), Error> {
    // TODO: Are Const, ConstTransform, and Direct all the same?
    // TODO: Can frame count be higher than 1 for Const and ConstTransform?
    use crate::anim_data::TrackTypeV2 as TrackTy;
    use crate::anim_data::TrackValues as Values;

    let mut reader = Cursor::new(track_data);

    let (values, inherit_scale, compensate_scale) = match flags.compression_type {
        CompressionType::Compressed => match flags.track_type {
            TrackTy::Transform => {
                // TODO: Is there a cleaner way to get the scale inheritance information?
                let (values, inherit_scale, compensate_scale) =
                    read_compressed_transforms(&mut reader, count)?;
                let values = values.iter().map(Transform::from).collect();
                (Values::Transform(values), inherit_scale, compensate_scale)
            }
            TrackTy::UvTransform => (
                Values::UvTransform(read_compressed(&mut reader, count)?),
                false,
                false,
            ),
            TrackTy::Float => (
                Values::Float(read_compressed(&mut reader, count)?),
                false,
                false,
            ),
            TrackTy::PatternIndex => (
                Values::PatternIndex(read_compressed(&mut reader, count)?),
                false,
                false,
            ),
            TrackTy::Boolean => {
                // TODO: This could be handled by the CompressedData trait.
                // TODO: Create a separate UncompressedData trait?
                // i.e. CompressedData: UncompressedData
                // This may be able to simplify the conversion logic for bool and Transform.
                let values: Vec<Boolean> = read_compressed(&mut reader, count)?;
                (
                    Values::Boolean(values.iter().map(bool::from).collect()),
                    false,
                    false,
                )
            }
            TrackTy::Vector4 => (
                Values::Vector4(read_compressed(&mut reader, count)?),
                false,
                false,
            ),
        },
        _ => match flags.track_type {
            TrackTy::Transform => {
                let values: Vec<UncompressedTransform> = read_uncompressed(&mut reader, count)?;
                // TODO: This should be an error if the values aren't all the same.
                let compensate_scale = values.iter().map(|t| t.compensate_scale).max().unwrap_or(0);
                (
                    Values::Transform(values.iter().map(Transform::from).collect()),
                    true, // TODO: Do uncompressed transform tracks always inherit scale?
                    compensate_scale != 0,
                )
            }
            TrackTy::UvTransform => (
                Values::UvTransform(read_uncompressed(&mut reader, count)?),
                false,
                false,
            ),
            TrackTy::Float => (
                Values::Float(read_uncompressed(&mut reader, count)?),
                false,
                false,
            ),
            TrackTy::PatternIndex => (
                Values::PatternIndex(read_uncompressed(&mut reader, count)?),
                false,
                false,
            ),
            TrackTy::Boolean => {
                let values = read_uncompressed(&mut reader, count)?;
                (
                    Values::Boolean(values.iter().map(bool::from).collect_vec()),
                    false,
                    false,
                )
            }
            TrackTy::Vector4 => (
                Values::Vector4(read_uncompressed(&mut reader, count)?),
                false,
                false,
            ),
        },
    };

    // TODO: Find a cleaner way to handle inheritance?
    Ok((values, inherit_scale, compensate_scale))
}

fn read_compressed<R: Read + Seek, T: CompressedData>(
    reader: &mut R,
    frame_count: usize,
) -> Result<Vec<T>, Error> {
    let data: CompressedTrackData<T> = reader.read_le()?;
    let values = read_compressed_inner(data, frame_count)?;
    Ok(values)
}

fn read_compressed_inner<T: CompressedData>(
    data: CompressedTrackData<T>,
    frame_count: usize,
) -> Result<Vec<T>, Error> {
    // Check for unexpected compression flags.
    // This is either an unresearched flag or an improperly compressed file.
    let expected_bit_count = data.compression.bit_count(data.header.flags) as usize;
    if data.header.bits_per_entry as usize != expected_bit_count {
        return Err(Error::UnexpectedBitCount {
            expected: expected_bit_count,
            actual: data.header.bits_per_entry as usize,
        });
    }

    let buffer = &data
        .header
        .compressed_data
        .as_ref()
        .ok_or(Error::MalformedCompressionHeader)?
        .0;

    // Decompress values.
    let mut reader = BitReader::from_slice(buffer);

    // Encode a repeated value as a single "frame".
    // TODO: Investigate the side effects of forcing uncompressed on save.
    // This prevents a potential out of memory or lengthy loop.
    // This case doesn't occur in any of Smash Ultimate's game files.
    let actual_count = if expected_bit_count == 0 && frame_count > 0 {
        1
    } else {
        frame_count
    };

    let mut values = Vec::new();
    for _ in 0..actual_count {
        let value = T::decompress(
            &mut reader,
            &data.compression,
            data.header
                .default_data
                .as_ref()
                .ok_or(Error::MalformedCompressionHeader)?,
            T::get_args(&data.header),
        )?;

        values.push(value);
    }

    Ok(values)
}

fn read_compressed_transforms<R: Read + Seek>(
    reader: &mut R,
    frame_count: usize,
) -> Result<(Vec<UncompressedTransform>, bool, bool), Error> {
    let data: CompressedTrackData<UncompressedTransform> = reader.read_le()?;

    // TODO: Is this the best way to handle scale settings?
    let inherit_scale = data.header.flags.scale_type() != ScaleType::ScaleNoInheritance;

    let compensate_scale = data
        .header
        .default_data
        .as_ref()
        .ok_or(Error::MalformedCompressionHeader)?
        .compensate_scale
        != 0;

    let values = read_compressed_inner(data, frame_count)?;

    Ok((values, inherit_scale, compensate_scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{anim_data::Transform, assert_hex_eq};
    use hexlit::hex;
    use ssbh_lib::{formats::anim::TrackTypeV2, Vector3};

    #[test]
    fn read_constant_vector4_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, CustomVector30
        let data = hex!(cdcccc3e 0000c03f 0000803f 0000803f);
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Vector4,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!(cdcccc3e 0000c03f 0000803f 0000803f));
    }

    #[test]
    fn read_constant_texture_single_frame() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeL, nfTexture1[0]
        let data = hex!(0000803f 0000803f 00000000 00000000 00000000);
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::UvTransform,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
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

        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::UvTransform,
                compression_type: CompressionType::Compressed,
            },
            4,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::UvTransform,
                compression_type: CompressionType::Compressed,
            },
            37,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::PatternIndex,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::PatternIndex,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

        // TODO: This is just a guess for min: 1, max: 2, bit_count: 1.
        assert!(matches!(
            values,
            TrackValues::PatternIndex(values)
            if values == vec![1, 2, 2, 2, 2, 2, 2, 2]
        ));
    }

    #[test]
    fn read_compressed_pattern_index_zero_bit_count() {
        let data = hex!(
            00000000 22000000 00000004 00000000      // header
            00000000 00000000 00000000 00000000      // compression
            00000004 00000000                        // default value
            000000000080000010000000000000ffffff     // compressed values
        );
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::PatternIndex,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();
    }

    #[test]
    fn read_constant_float_single_frame() {
        // assist/shovelknight/model/body/c00/model.nuanmb, asf_shovelknight_mat, CustomFloat8
        let data = hex!(cdcccc3e);
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Float,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!(cdcccc3e));
    }

    #[test]
    fn read_compressed_float_all_equal() {
        // This is an edge case that doesn't appear in game.
        // It's possible to have a high frame count with 0 bits per entry.
        // The default value is used for all entries.
        // A naive implementation will likely crash.
        let data = hex!(
            04000000 20000000 24000000 FFFFFFFF // header
            cdcccc3e cdcccc3e 10000000 00000000 // compression
            cdcccc3e                            // default value
                                                // compressed values
        );
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Float,
                compression_type: CompressionType::Compressed,
            },
            0xFFFFFFFF,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

        assert!(matches!(values, TrackValues::Float(values) if values == vec![0.4]));
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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Float,
                compression_type: CompressionType::Compressed,
            },
            5,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
        )
        .unwrap();

        assert_eq!(*writer.get_ref(), hex!("01"));
    }

    #[test]
    fn read_constant_boolean_single_frame_false() {
        // fighter/mario/motion/body/c00/a00wait1.nuanmb, EyeR, CustomBoolean11
        let data = hex!("00");
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Boolean,
                compression_type: CompressionType::Constant,
            },
            1,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Boolean,
                compression_type: CompressionType::Compressed,
            },
            3,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
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
            false,
            false,
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
            false,
            false,
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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Vector4,
                compression_type: CompressionType::Compressed,
            },
            8,
        )
        .unwrap();

        assert_eq!(false, inherit_scale);
        assert_eq!(false, compensate_scale);

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
            false,
            false,
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
            false,
            false,
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

        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Transform,
                compression_type: CompressionType::ConstTransform,
            },
            1,
        )
        .unwrap();

        assert_eq!(true, inherit_scale);
        assert_eq!(true, compensate_scale);

        assert!(matches!(values,
            TrackValues::Transform(values)
            if values == vec![
                Transform {
                    translation: Vector3::new(1.51284, -0.232973, -0.371597),
                    rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    scale: Vector3::new(1.0, 1.0, 1.0),
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
            }]),
            &mut writer,
            CompressionType::Constant,
            true,
            true,
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
        let default = UncompressedTransform {
            scale: Vector3::new(4.0, 4.0, 4.0),
            rotation: Vector4::new(2.0, 2.0, 2.0, 2.0),
            translation: Vector3::new(3.0, 3.0, 3.0),
            compensate_scale: 0,
        };

        let header = CompressedHeader::<UncompressedTransform> {
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

        let mut reader = BitReader::from_slice(&data_hex);

        let default = UncompressedTransform {
            scale: Vector3::new(4.0, 4.0, 4.0),
            rotation: Vector4::new(2.0, 2.0, 2.0, 2.0),
            translation: Vector3::new(3.0, 3.0, 3.0),
            compensate_scale: 0,
        };
        reader
            .decompress(&compression, &default, header.flags)
            .unwrap();
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
    fn read_compressed_transform_multiple_frames_null_default() {
        // assist/shovelknight/model/body/c00/model.nuanmb, ArmL, Transform
        // Default pointer set to 0.
        let data = hex!(
            // header
            04000600 00002b00 cc000000 02000000
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
            16a41d40 00000000 00000000
            00000000
            // compressed values
            00e0ff03 00f8ff00 e0ff1f
        );

        let result = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Transform,
                compression_type: CompressionType::Compressed,
            },
            2,
        );
        assert!(matches!(result, Err(Error::MalformedCompressionHeader)));
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
            16a41d40 00000000 00000000
            00000000
            // compressed values
            00e0ff03 00f8ff00 e0ff1f
        );

        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Transform,
                compression_type: CompressionType::Compressed,
            },
            2,
        )
        .unwrap();

        assert_eq!(true, inherit_scale);
        assert_eq!(false, compensate_scale);

        assert!(matches!(values,
            TrackValues::Transform(values)
            if values == vec![
                Transform {
                    translation: Vector3::new(2.46314, 0.0, 0.0),
                    rotation: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                },
                Transform {
                    translation: Vector3::new(2.46314, 0.0, 0.0),
                    rotation: Vector4::new(0.0477874, -0.0656469, 0.654826, 0.7514052),
                    scale: Vector3::new(1.0, 1.0, 1.0),
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
            },
            Transform {
                translation: Vector3::new(1.0, 2.0, 3.0),
                rotation: Vector4::new(4.0, 5.0, 6.0, 0.0),
                scale: Vector3::new(8.0, 9.0, 10.0),
            },
        ];

        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Transform(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false,
            false,
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
            read_compressed(&mut Cursor::new(writer.get_ref()), 2)
                .unwrap()
                .iter()
                .map(Transform::from)
                .collect::<Vec<Transform>>()
        );
    }

    #[test]
    fn write_compressed_transform_multiple_frames_uniform_scale() {
        let values = vec![
            Transform {
                translation: Vector3::new(-1.0, -2.0, -3.0),
                rotation: Vector4::new(-4.0, -5.0, -6.0, 0.0),
                scale: Vector3::new(-8.0, -8.0, -8.0),
            },
            Transform {
                translation: Vector3::new(1.0, 2.0, 3.0),
                rotation: Vector4::new(4.0, 5.0, 6.0, 0.0),
                scale: Vector3::new(9.0, 9.0, 9.0),
            },
        ];

        let mut writer = Cursor::new(Vec::new());
        TrackValues::write(
            &TrackValues::Transform(values.clone()),
            &mut writer,
            CompressionType::Compressed,
            false,
            false,
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
            read_compressed(&mut Cursor::new(writer.get_ref()), 2)
                .unwrap()
                .iter()
                .map(Transform::from)
                .collect::<Vec<Transform>>()
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
        let (values, inherit_scale, compensate_scale) = read_track_values(
            &data,
            TrackFlags {
                track_type: TrackTypeV2::Transform,
                compression_type: CompressionType::Direct,
            },
            2,
        )
        .unwrap();

        assert_eq!(true, inherit_scale);
        assert_eq!(false, compensate_scale);

        assert!(matches!(values,
            TrackValues::Transform(values)
            if values == vec![
                Transform {
                    translation: Vector3::new(-28.6956, 5.01271, 7.83398),
                    rotation: Vector4::new(0.157021, -0.587681, -0.0991261, 0.787496),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                },
                Transform {
                    translation: Vector3::new(-32.9135, 5.27391, 8.69207),
                    rotation: Vector4::new(0.134616, -0.599292, -0.111365, 0.781233),
                    scale: Vector3::new(1.0, 1.0, 1.0),
                },
            ]
        ));
    }
}
