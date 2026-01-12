use std::io::Cursor;

use binrw::BinReaderExt;
use glam::{Vec3, vec3};

use super::common::{
    G_CURVE_SHORT4_SCALE, align_up, compute_block_count, compute_block_count_type9,
    compute_block_len, compute_block_len_type9, compute_block_qcounts, decode_residual_vector,
    expand_sparse_vec3, read_f32_le, read_u16_le, read_u32_le, read_vec3_f32_le,
};
use crate::anim_data::{error, v1::buffers::Unk3300};

pub fn decode_translate_3200(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3200 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    let mut frame_indices = Vec::with_capacity(key_count);
    let mut pos = 12;
    for _ in 0..key_count {
        frame_indices.push((read_u16_le(bytes, pos)? as f32 * unk1).round() as usize);
        pos += 2;
    }
    pos = align_up(pos, 4);
    let mut values = Vec::with_capacity(key_count);
    for i in 0..key_count {
        values.push(read_vec3_f32_le(bytes, pos + i * 12)?.into());
    }
    let last_frame = *frame_indices.iter().max().unwrap_or(&0);
    let total_frames = last_frame + 1;
    Ok(expand_sparse_vec3(&frame_indices, &values, total_frames))
}

pub fn decode_translate_3208(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 16 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3208 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    if key_count > 34 {
        return Err(error::Error::InvalidData);
    }
    let mut frame_indices = Vec::with_capacity(key_count);
    let mut pos = 12;
    for _ in 0..key_count {
        frame_indices.push((read_u16_le(bytes, pos)? as f32 * unk1).round() as usize);
        pos += 2;
    }
    pos = align_up(pos, 4);
    let base_scale = read_f32_le(bytes, pos)?;
    let endpoint0 = read_vec3_f32_le(bytes, pos + 4)?;
    let endpoint1 = read_vec3_f32_le(bytes, pos + 16)?;
    let residual_off = pos + 28;
    if residual_off > bytes.len() || residual_off % 4 != 0 {
        return Err(error::Error::InvalidData);
    }
    let block_len = key_count.saturating_sub(1).max(1);
    let mut key_values = Vec::with_capacity(key_count);
    for local in 0..key_count {
        let t = local as f32 / block_len as f32;
        let kx = endpoint0.x + (endpoint1.x - endpoint0.x) * t;
        let ky = endpoint0.y + (endpoint1.y - endpoint0.y) * t;
        let kz = endpoint0.z + (endpoint1.z - endpoint0.z) * t;
        let (r_vec, _) =
            decode_residual_vector(bytes, residual_off, base_scale, local, 3, block_len)?;
        key_values.push(vec3(kx + r_vec[0], ky + r_vec[1], kz + r_vec[2]));
    }
    let frames = expand_sparse_vec3(
        &frame_indices,
        &key_values,
        frame_indices.iter().copied().max().unwrap_or(0) + 1,
    );
    Ok(frames)
}

pub fn decode_translate_3300(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    let mut reader = Cursor::new(bytes);
    let header: Unk3300 = reader.read_le()?;

    let key_count = header.frame_count as usize;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    let frame_indices: Vec<_> = header
        .frame_indices
        .into_iter()
        .map(|i| (i as f32 * header.unk1).round() as usize)
        .collect();

    let frames = expand_sparse_vec3(
        &frame_indices,
        &header.values,
        frame_indices.iter().copied().max().unwrap_or(0) + 1,
    );
    Ok(frames)
}

pub fn decode_translate_3308(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 16 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3308 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    if key_count > 33 {
        return Err(error::Error::InvalidData);
    }
    let mut frame_indices = Vec::with_capacity(key_count);
    let mut pos = 12;
    for _ in 0..key_count {
        frame_indices.push(
            (bytes.get(pos).copied().ok_or(error::Error::InvalidData)? as f32 * unk1).round()
                as usize,
        );
        pos += 1;
    }
    pos = align_up(pos, 4);
    let base_scale = read_f32_le(bytes, pos)?;
    let endpoint0 = read_vec3_f32_le(bytes, pos + 4)?;
    let endpoint1 = read_vec3_f32_le(bytes, pos + 16)?;
    let residual_off = pos + 28;
    if residual_off > bytes.len() || residual_off % 4 != 0 {
        return Err(error::Error::InvalidData);
    }
    let block_len = key_count.saturating_sub(1).max(1);
    let mut key_values = Vec::with_capacity(key_count);
    for local in 0..key_count {
        let t = local as f32 / block_len as f32;
        let kx = endpoint0.x + (endpoint1.x - endpoint0.x) * t;
        let ky = endpoint0.y + (endpoint1.y - endpoint0.y) * t;
        let kz = endpoint0.z + (endpoint1.z - endpoint0.z) * t;
        let (r_vec, _) =
            decode_residual_vector(bytes, residual_off, base_scale, local, 3, block_len)?;
        key_values.push(vec3(kx + r_vec[0], ky + r_vec[1], kz + r_vec[2]));
    }
    let frames = expand_sparse_vec3(
        &frame_indices,
        &key_values,
        frame_indices.iter().copied().max().unwrap_or(0) + 1,
    );
    Ok(frames)
}

pub fn decode_translate_3209(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 16 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3209 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    let mut frame_indices = Vec::with_capacity(key_count);
    let mut pos = 12;
    for _ in 0..key_count {
        frame_indices.push((read_u16_le(bytes, pos)? as f32 * unk1).round() as usize);
        pos += 2;
    }
    pos = align_up(pos, 4);
    if pos + 8 > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let base_scale = read_f32_le(bytes, pos)?;
    let block_count = read_u16_le(bytes, pos + 4)? as usize;
    let expected_blocks = compute_block_count_type9(key_count);
    if block_count != expected_blocks || block_count == 0 {
        return Err(error::Error::InvalidData);
    }
    let mut block_words = Vec::with_capacity(block_count.saturating_sub(1));
    let mut bw_off = pos + 6;
    for _ in 0..block_count.saturating_sub(1) {
        block_words.push(read_u16_le(bytes, bw_off)? as usize);
        bw_off += 2;
    }
    let endpoint_base = align_up(pos + 4 + 2 * block_count, 4);
    let endpoint_count = block_count + 1;
    let endpoints_size = endpoint_count * 12;
    if endpoint_base + endpoints_size > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for i in 0..endpoint_count {
        endpoints.push(read_vec3_f32_le(bytes, endpoint_base + i * 12)?);
    }
    let residual_off = endpoint_base + endpoints_size;
    if residual_off > bytes.len() || !residual_off.is_multiple_of(4) {
        return Err(error::Error::InvalidData);
    }
    let mut residual_starts = Vec::with_capacity(block_count);
    residual_starts.push(residual_off);
    for w in &block_words {
        residual_starts.push(residual_off + 4 * *w);
    }

    let mut key_values = Vec::with_capacity(key_count);
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let mut block_len = compute_block_len_type9(key_count, block_idx);
        if block_len == 0 {
            block_len = 1;
        }
        let t_block = local as f32 / block_len as f32;
        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let kx = e0.x + (e1.x - e0.x) * t_block;
        let ky = e0.y + (e1.y - e0.y) * t_block;
        let kz = e0.z + (e1.z - e0.z) * t_block;
        let rs = residual_starts[block_idx];
        let (r_vec, _) = decode_residual_vector(bytes, rs, base_scale, local, 3, block_len)?;
        key_values.push(vec3(kx + r_vec[0], ky + r_vec[1], kz + r_vec[2]));
    }
    let frames = expand_sparse_vec3(
        &frame_indices,
        &key_values,
        frame_indices.iter().copied().max().unwrap_or(0) + 1,
    );
    Ok(frames)
}

pub fn decode_translate_3309(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 16 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3309 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    let mut frame_indices = Vec::with_capacity(key_count);
    let mut pos = 12;
    for _ in 0..key_count {
        frame_indices.push(
            (bytes.get(pos).copied().ok_or(error::Error::InvalidData)? as f32 * unk1).round()
                as usize,
        );
        pos += 1;
    }
    pos = align_up(pos, 4);
    if pos + 8 > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let base_scale = read_f32_le(bytes, pos)?;
    let block_count = read_u16_le(bytes, pos + 4)? as usize;
    let expected_blocks = compute_block_count_type9(key_count);
    if block_count != expected_blocks || block_count == 0 {
        return Err(error::Error::InvalidData);
    }
    let mut block_words = Vec::with_capacity(block_count.saturating_sub(1));
    let mut bw_off = pos + 6;
    for _ in 0..block_count.saturating_sub(1) {
        block_words.push(read_u16_le(bytes, bw_off)? as usize);
        bw_off += 2;
    }
    let endpoint_base = align_up(pos + 2 * block_count + 4, 4);
    let endpoint_count = block_count + 1;
    let endpoints_size = endpoint_count * 12;
    if endpoint_base + endpoints_size > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for i in 0..endpoint_count {
        endpoints.push(read_vec3_f32_le(bytes, endpoint_base + i * 12)?);
    }
    let residual_off = endpoint_base + endpoints_size;
    if residual_off > bytes.len() || !residual_off.is_multiple_of(4) {
        return Err(error::Error::InvalidData);
    }
    let mut residual_starts = Vec::with_capacity(block_count);
    residual_starts.push(residual_off);
    for w in &block_words {
        residual_starts.push(residual_off + 4 * *w);
    }

    let mut key_values = Vec::with_capacity(key_count);
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let mut block_len = compute_block_len_type9(key_count, block_idx);
        if block_len == 0 {
            block_len = 1;
        }
        let t_block = local as f32 / block_len as f32;
        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let kx = e0.x + (e1.x - e0.x) * t_block;
        let ky = e0.y + (e1.y - e0.y) * t_block;
        let kz = e0.z + (e1.z - e0.z) * t_block;
        let rs = residual_starts[block_idx];
        let (r_vec, _) = decode_residual_vector(bytes, rs, base_scale, local, 3, block_len)?;
        key_values.push(vec3(kx + r_vec[0], ky + r_vec[1], kz + r_vec[2]));
    }
    let frames = expand_sparse_vec3(
        &frame_indices,
        &key_values,
        frame_indices.iter().copied().max().unwrap_or(0) + 1,
    );
    Ok(frames)
}

pub fn decode_translate_3400(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3400 {
        return Err(error::Error::InvalidData);
    }
    let frame_count = read_u32_le(bytes, 4)? as usize;
    let pos = 12;
    let mut frames = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        frames.push(read_vec3_f32_le(bytes, pos + i * 12)?.into());
    }
    Ok(frames)
}

pub fn decode_translate_3408(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3408 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let _unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }
    let blocks = compute_block_count(key_count);
    let endpoint_count = blocks + 1;
    let endpoints_size = endpoint_count * 12;
    let scan_start = 12;
    let scan_end = (scan_start + endpoints_size + 0x40).min(bytes.len());

    let mut best: Option<(Vec<Vec3>, f32, usize, usize, Vec<usize>)> = None;
    let mut best_key: Option<(usize, isize, usize)> = None;

    for endpoint_base in (scan_start..=scan_end).step_by(4) {
        if endpoint_base + endpoints_size > bytes.len() {
            continue;
        }
        let mut endpoints = Vec::with_capacity(endpoint_count);
        for i in 0..endpoint_count {
            endpoints.push(read_vec3_f32_le(bytes, endpoint_base + i * 12)?.into());
        }
        let end_ep = endpoint_base + endpoints_size;
        for base_scale_off in
            (end_ep..=(end_ep + 0x10).min(bytes.len().saturating_sub(4))).step_by(4)
        {
            let base_scale = read_f32_le(bytes, base_scale_off)?;
            for residual_off in
                (base_scale_off + 4..=(base_scale_off + 0x80).min(bytes.len())).step_by(4)
            {
                if residual_off >= bytes.len() {
                    continue;
                }
                for comp_bits in [3usize, 2, 1, 4] {
                    let q_counts = match compute_block_qcounts(
                        bytes,
                        residual_off,
                        base_scale,
                        comp_bits,
                        key_count,
                    ) {
                        Ok(q) => q,
                        Err(_) => continue,
                    };
                    let end_off = residual_off + 4 * q_counts.iter().sum::<usize>();
                    if end_off > bytes.len() {
                        continue;
                    }
                    let slack = bytes.len() - end_off;
                    let cand_key = (slack, -(comp_bits as isize), residual_off);
                    if best.is_none() || cand_key < best_key.unwrap() {
                        best = Some((
                            endpoints.clone(),
                            base_scale,
                            comp_bits,
                            residual_off,
                            q_counts,
                        ));
                        best_key = Some(cand_key);
                    }
                    break;
                }
            }
        }
    }

    let (endpoints, base_scale, comp_bits, residual_off, q_counts) =
        best.ok_or(error::Error::InvalidData)?;
    let mut prefix_words = Vec::with_capacity(q_counts.len() + 1);
    prefix_words.push(0usize);
    for q in &q_counts {
        prefix_words.push(prefix_words.last().copied().unwrap_or(0) + *q);
    }

    let mut out = Vec::with_capacity(key_count);
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let mut block_len = compute_block_len(key_count, block_idx);
        if block_len == 0 {
            block_len = 1;
        }
        let t_block = local as f32 / block_len as f32;
        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let kx = e0.x + (e1.x - e0.x) * t_block;
        let ky = e0.y + (e1.y - e0.y) * t_block;
        let kz = e0.z + (e1.z - e0.z) * t_block;
        let rs = residual_off + 4 * prefix_words[block_idx];
        let (r_vec, _) =
            decode_residual_vector(bytes, rs, base_scale, local, comp_bits, block_len)?;
        out.push(Vec3 {
            x: kx + r_vec.first().copied().unwrap_or(0.0),
            y: ky + r_vec.get(1).copied().unwrap_or(0.0),
            z: kz + r_vec.get(2).copied().unwrap_or(0.0),
        });
    }
    Ok(out)
}

pub fn decode_vector3_3409(bytes: &[u8]) -> Result<Vec<Vec3>, error::Error> {
    if bytes.len() < 20 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x3409 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let _unk1 = read_f32_le(bytes, 8)?;
    let base_scale = read_f32_le(bytes, 12)?;
    let _flags = read_u16_le(bytes, 16)?;
    let _bits = read_u16_le(bytes, 18)?;

    if key_count == 0 {
        return Ok(vec![Vec3::ZERO]);
    }

    let blocks = compute_block_count(key_count);
    let endpoint_count = blocks + 1;

    let mut best: Option<(Vec<Vec3>, usize, usize, Vec<usize>)> = None;
    let mut best_key: Option<(usize, i32, usize)> = None;

    for scan_start in [0x14usize, 0x18usize] {
        if scan_start > bytes.len() {
            continue;
        }
        let (endpoints, comp_bits, residual_off, q_counts) =
            match infer_3409_layout(bytes, scan_start, endpoint_count, base_scale, key_count) {
                Ok(v) => v,
                Err(_) => continue,
            };

        let end_off = residual_off + 4 * q_counts.iter().sum::<usize>();
        if end_off > bytes.len() {
            continue;
        }
        let slack = bytes.len() - end_off;
        let cand_key = (slack, -(comp_bits as i32), residual_off);
        if best_key.is_none() || cand_key < best_key.unwrap() {
            best = Some((endpoints, comp_bits, residual_off, q_counts));
            best_key = Some(cand_key);
        }
    }

    let (endpoints, comp_bits, residual_off, q_counts) = best.ok_or(error::Error::InvalidData)?;

    let mut prefix_words = vec![0usize];
    for q in &q_counts {
        prefix_words.push(prefix_words.last().copied().unwrap_or(0) + *q);
    }

    let mut out = Vec::with_capacity(key_count);
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let mut block_len = compute_block_len(key_count, block_idx);
        if block_len == 0 {
            block_len = 1;
        }
        let t_block = local as f32 / block_len as f32;
        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let kx = e0.x + (e1.x - e0.x) * t_block;
        let ky = e0.y + (e1.y - e0.y) * t_block;
        let kz = e0.z + (e1.z - e0.z) * t_block;
        let rs = residual_off + 4 * prefix_words[block_idx];
        let (r_vec, _) =
            decode_residual_vector(bytes, rs, base_scale, local, comp_bits, block_len)?;
        out.push(Vec3 {
            x: kx + r_vec.first().copied().unwrap_or(0.0),
            y: ky + r_vec.get(1).copied().unwrap_or(0.0),
            z: kz + r_vec.get(2).copied().unwrap_or(0.0),
        });
    }
    Ok(out)
}

fn infer_3409_layout(
    bytes: &[u8],
    scan_start: usize,
    endpoint_count: usize,
    base_scale: f32,
    key_count: usize,
) -> Result<(Vec<Vec3>, usize, usize, Vec<usize>), error::Error> {
    let mut best: Option<(Vec<Vec3>, usize, usize, Vec<usize>)> = None;
    let mut best_key: Option<(usize, i32, usize)> = None;

    for elem_size in [12, 8] {
        let endpoint_base = scan_start;
        let endpoints_end = endpoint_base + endpoint_count * elem_size;
        if endpoints_end > bytes.len() {
            continue;
        }

        let endpoints =
            match try_parse_endpoints_3409(bytes, endpoint_base, endpoint_count, elem_size) {
                Ok(e) => e,
                Err(_) => continue,
            };

        for pad in (0..=32).step_by(4) {
            let residual_off = endpoints_end + pad;
            if residual_off >= bytes.len() {
                continue;
            }

            for comp_bits in [3, 2, 1, 4] {
                let q_counts = match compute_block_qcounts(
                    bytes,
                    residual_off,
                    base_scale,
                    comp_bits,
                    key_count,
                ) {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let end_off = residual_off + 4 * q_counts.iter().sum::<usize>();
                if end_off > bytes.len() {
                    continue;
                }
                let slack = bytes.len() - end_off;
                let cand_key = (slack, -(comp_bits as i32), residual_off);
                if best.is_none() || cand_key < best_key.unwrap() {
                    best = Some((endpoints.clone(), comp_bits, residual_off, q_counts));
                    best_key = Some(cand_key);
                }
                break;
            }
        }
    }

    best.ok_or(error::Error::InvalidData)
}

fn try_parse_endpoints_3409(
    bytes: &[u8],
    endpoint_base: usize,
    endpoint_count: usize,
    elem_size: usize,
) -> Result<Vec<Vec3>, error::Error> {
    let end = endpoint_base + endpoint_count * elem_size;
    if end > bytes.len() {
        return Err(error::Error::InvalidData);
    }

    let mut out = Vec::with_capacity(endpoint_count);
    let mut max_abs = 0.0f32;

    if elem_size == 12 {
        for i in 0..endpoint_count {
            let v: Vec3 = read_vec3_f32_le(bytes, endpoint_base + i * 12)?.into();
            max_abs = max_abs.max(v.x.abs()).max(v.y.abs()).max(v.z.abs());
            out.push(v);
        }
    } else if elem_size == 8 {
        for i in 0..endpoint_count {
            let o = endpoint_base + i * 8;
            let x = i16::from_le_bytes([bytes[o], bytes[o + 1]]) as f32 * G_CURVE_SHORT4_SCALE;
            let y = i16::from_le_bytes([bytes[o + 2], bytes[o + 3]]) as f32 * G_CURVE_SHORT4_SCALE;
            let z = i16::from_le_bytes([bytes[o + 4], bytes[o + 5]]) as f32 * G_CURVE_SHORT4_SCALE;
            max_abs = max_abs.max(x.abs()).max(y.abs()).max(z.abs());
            out.push(Vec3 { x, y, z });
        }
    } else {
        return Err(error::Error::InvalidData);
    }

    if !max_abs.is_finite() || max_abs > 1.0e6 {
        return Err(error::Error::InvalidData);
    }

    Ok(out)
}
