use glam::Quat;

use super::common::{
    compute_block_len, compute_block_qcounts, decode_residual_vector, quat_normalize, read_f32_le,
    read_u16_le, read_u32_le, read_vec4_f32_le,
};
use crate::anim_data::error;

pub fn decode_rotate_4409(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 0x14 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4409 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let _unk1 = read_f32_le(bytes, 8)?;
    let base_scale = read_f32_le(bytes, 12)?;
    let _bits = read_u16_le(bytes, 18)?;

    if key_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }

    let block_count = if key_count <= 1 {
        0usize
    } else {
        (key_count - 1) / 33 + 1
    };
    let endpoint_count = block_count + 1;

    let mut best: Option<(Vec<Quat>, usize, usize)> = None;
    let mut best_key: Option<(usize, i32, usize)> = None;

    for endpoint_base in [0x14usize, 0x18usize] {
        let endpoints_size = endpoint_count * 16;
        let residual_off = endpoint_base + endpoints_size;
        if residual_off > bytes.len() {
            continue;
        }
        if (endpoint_base % 4) != 0 || (residual_off % 4) != 0 {
            continue;
        }

        let mut endpoints = Vec::with_capacity(endpoint_count);
        let mut max_abs = 0.0f32;
        let mut ok = true;
        for i in 0..endpoint_count {
            let q = match read_vec4_f32_le(bytes, endpoint_base + i * 16) {
                Ok(v) => v,
                Err(_) => {
                    ok = false;
                    break;
                }
            };
            max_abs = max_abs
                .max(q.x.abs())
                .max(q.y.abs())
                .max(q.z.abs())
                .max(q.w.abs());
            endpoints.push(Quat::from_array(q.to_array()));
        }
        if !ok || !max_abs.is_finite() || max_abs > 1.0e6 {
            continue;
        }

        for comp_bits in [4usize, 3, 2, 1] {
            let end_off = match walk_residual_4409(
                bytes,
                residual_off,
                base_scale,
                comp_bits,
                key_count,
                block_count,
            ) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if end_off > bytes.len() {
                continue;
            }
            let slack = bytes.len() - end_off;
            let cand_key = (slack, -(comp_bits as i32), residual_off);
            if best_key.is_none() || cand_key < best_key.unwrap() {
                best = Some((endpoints.clone(), residual_off, comp_bits));
                best_key = Some(cand_key);
            }
            break;
        }
    }

    let (endpoints, residual_off, comp_bits) = best.ok_or(error::Error::InvalidData)?;

    let q_counts = compute_block_qcounts(bytes, residual_off, base_scale, comp_bits, key_count)?;
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
        let k = Quat::from_xyzw(
            e0.x + (e1.x - e0.x) * t_block,
            e0.y + (e1.y - e0.y) * t_block,
            e0.z + (e1.z - e0.z) * t_block,
            e0.w + (e1.w - e0.w) * t_block,
        );
        let rs = residual_off + 4 * prefix_words[block_idx];
        let (r_vec, _) =
            decode_residual_vector(bytes, rs, base_scale, local, comp_bits, block_len)?;
        let q = quat_normalize(
            k.x + r_vec.first().copied().unwrap_or(0.0),
            k.y + r_vec.get(1).copied().unwrap_or(0.0),
            k.z + r_vec.get(2).copied().unwrap_or(0.0),
            k.w + r_vec.get(3).copied().unwrap_or(0.0),
        );
        out.push(q);
    }
    Ok(out)
}

fn walk_residual_4409(
    payload: &[u8],
    residual_off: usize,
    base_scale: f32,
    comp_bits: usize,
    key_count: usize,
    block_count: usize,
) -> Result<usize, error::Error> {
    let mut cursor = residual_off;
    for block_idx in 0..block_count {
        let block_len = compute_block_len(key_count, block_idx);
        if block_len <= 1 {
            continue;
        }
        let (_, end_off) =
            decode_residual_vector(payload, cursor, base_scale, 1, comp_bits, block_len)?;
        let delta = end_off.saturating_sub(cursor);
        if delta == 0 || (delta % 4) != 0 {
            return Err(error::Error::InvalidData);
        }
        cursor = end_off;
    }
    Ok(cursor)
}
