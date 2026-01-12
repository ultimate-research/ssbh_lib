use glam::Quat;

use super::common::{
    align_up, compute_block_count_4309, compute_block_len_4309, decode_residual_vector, quat_nlerp,
    quat_normalize, read_f32_le, read_u32_le, try_parse_endpoints_4309, try_pick_comp_bits_4309,
};
use crate::anim_data::{Vector4, error};

pub fn decode_rotate_4309(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    let header = read_u32_le(bytes, 0)?;
    if header != 0x4309 {
        return Err(error::Error::InvalidData);
    }

    let key_count = read_u32_le(bytes, 4)? as usize;

    if key_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }

    let mut pos = 12;
    let end = pos + key_count;
    if end > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let frame_indices_u8: Vec<u8> = bytes[pos..end].to_vec();
    pos = align_up(end, 4);

    if pos + 12 > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let base_scale = read_f32_le(bytes, pos)?;
    pos += 12;

    let endpoint_count = compute_block_count_4309(key_count) + 1;
    let (endpoints, residual_payload, comp_bits, q_counts) =
        infer_4309_layout(bytes, pos, endpoint_count, base_scale, key_count)?;

    let mut prefix_words = vec![0usize];
    for &q in &q_counts {
        prefix_words.push(prefix_words.last().unwrap() + q);
    }

    let mut key_quats = Vec::with_capacity(key_count);
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let block_len = compute_block_len_4309(key_count, block_idx).max(1);

        let t_block = local as f32 / block_len as f32;
        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let k = Vector4 {
            x: e0.x + t_block * (e1.x - e0.x),
            y: e0.y + t_block * (e1.y - e0.y),
            z: e0.z + t_block * (e1.z - e0.z),
            w: e0.w + t_block * (e1.w - e0.w),
        };

        let rs = 4 * prefix_words[block_idx];
        let (r_vec, _) = decode_residual_vector(
            &residual_payload,
            rs,
            base_scale,
            local,
            comp_bits,
            block_len,
        )?;
        let qx = k.x + r_vec[0];
        let qy = k.y + r_vec[1];
        let qz = k.z + r_vec[2];
        let qw = k.w + r_vec[3];
        key_quats.push(quat_normalize(qx, qy, qz, qw));
    }

    let last_frame = *frame_indices_u8.iter().max().unwrap_or(&0) as usize;
    let total_frames = last_frame + 1;
    let mut frames_q = vec![key_quats[0]; total_frames];

    let first_f = frame_indices_u8[0] as usize;
    for f in 0..first_f.min(total_frames) {
        frames_q[f] = key_quats[0];
    }

    for i in 0..frame_indices_u8.len() - 1 {
        let f0 = frame_indices_u8[i] as usize;
        let f1 = frame_indices_u8[i + 1] as usize;
        if f1 <= f0 {
            continue;
        }
        let span = f1 - f0;
        if span == 0 {
            if f0 < total_frames {
                frames_q[f0] = key_quats[i];
            }
            continue;
        }
        for f in f0..=(f1.min(total_frames - 1)) {
            let t = (f - f0) as f32 / span as f32;
            frames_q[f] = quat_nlerp(key_quats[i], key_quats[i + 1], t);
        }
    }

    let last_f = frame_indices_u8[frame_indices_u8.len() - 1] as usize;
    for f in last_f..total_frames {
        frames_q[f] = key_quats[key_quats.len() - 1];
    }

    Ok(frames_q)
}

fn infer_4309_layout(
    curve_bytes: &[u8],
    base_offset: usize,
    endpoint_count: usize,
    base_scale: f32,
    key_count: usize,
) -> Result<(Vec<Quat>, Vec<u8>, usize, Vec<usize>), error::Error> {
    let mut best: Option<(Vec<Quat>, Vec<u8>, usize, Vec<usize>)> = None;
    let mut best_key: Option<(usize, i32, usize, usize)> = None;

    for elem_size in [16, 12] {
        let endpoints =
            match try_parse_endpoints_4309(curve_bytes, base_offset, endpoint_count, elem_size) {
                Ok(e) => e,
                Err(_) => continue,
            };

        let endpoints_end = base_offset + endpoint_count * elem_size;
        for pad in (0..=0x20).step_by(4) {
            let residual_off = endpoints_end + pad;
            if residual_off >= curve_bytes.len() {
                continue;
            }
            let residual_payload = curve_bytes[residual_off..].to_vec();
            if !residual_payload.len().is_multiple_of(4) {
                continue;
            }

            let (comp_bits, q_counts, end_off) =
                match try_pick_comp_bits_4309(&residual_payload, 0, base_scale, key_count) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

            let slack = curve_bytes.len() - (residual_off + end_off);
            let elem_rank = if elem_size == 16 { 0 } else { 1 };
            let cand_key = (slack, -(comp_bits as i32), elem_rank, residual_off);
            if best_key.is_none() || cand_key < best_key.unwrap() {
                best = Some((
                    endpoints.clone(),
                    residual_payload.clone(),
                    comp_bits,
                    q_counts.clone(),
                ));
                best_key = Some(cand_key);
            }
            if slack <= 16 {
                break;
            }
        }
    }

    best.ok_or(error::Error::InvalidData)
}
