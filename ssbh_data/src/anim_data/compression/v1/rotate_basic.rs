use glam::{Quat, quat};

use super::common::{
    align_up, compute_block_count_type9, compute_block_len_type9, decode_residual_vector,
    expand_sparse_quat, read_f32_le, read_u16_le, read_u32_le, read_vec3_f32_le, read_vec4_f32_le,
};
use crate::anim_data::{Vector4, bitutils::BitReader, error};

pub fn decode_rotate_4300(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4300 {
        return Err(error::Error::InvalidData);
    }
    let frame_count = read_u32_le(bytes, 4)? as usize;
    if frame_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }

    // Variant A: 16-byte header (unk1 + unk2) + frame_count * vec4<f32>.
    // Variant B: 12-byte header (unk1 only) + frame_count * vec4<f32>.
    // Variant C (observed in game data): 12-byte header + u8 frame_indices[frame_count] + align4 + frame_count * vec4<f32>.
    //
    // The same magic (0x4300) is used for these variants, so we must infer the layout from length.

    // Prefer the indexed-key variant if it matches exactly.
    let indexed_payload_off = align_up(12 + frame_count, 4);
    if indexed_payload_off <= bytes.len() && indexed_payload_off + frame_count * 16 == bytes.len() {
        let frame_indices: Vec<usize> = bytes[12..12 + frame_count]
            .iter()
            .map(|v| *v as usize)
            .collect();
        let mut key_vals = Vec::with_capacity(frame_count);
        let mut pos = indexed_payload_off;
        for _ in 0..frame_count {
            let mut q = Quat::from_array(read_vec4_f32_le(bytes, pos)?.to_array());
            pos += 16;
            let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
            if len2 > 0.0 {
                let inv = 1.0 / len2.sqrt();
                q.x *= inv;
                q.y *= inv;
                q.z *= inv;
                q.w *= inv;
            } else {
                q = Quat::IDENTITY;
            }
            key_vals.push((q.x, q.y, q.z, q.w));
        }
        let total_frames = frame_indices.iter().copied().max().unwrap_or(0) + 1;
        return Ok(expand_sparse_quat(&frame_indices, &key_vals, total_frames));
    }

    // Fallback to raw stream variants.
    let pos = if bytes.len() >= 16 + frame_count * 16 && bytes.len() != 12 + frame_count * 16 {
        16
    } else {
        12
    };
    if pos + frame_count * 16 > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let mut frames = Vec::with_capacity(frame_count);
    let mut pos = pos;
    for _ in 0..frame_count {
        let mut q = Quat::from_array(read_vec4_f32_le(bytes, pos)?.to_array());
        pos += 16;
        let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
        if len2 > 0.0 {
            let inv = 1.0 / len2.sqrt();
            q.x *= inv;
            q.y *= inv;
            q.z *= inv;
            q.w *= inv;
        } else {
            q = Quat::IDENTITY;
        }
        frames.push(q);
    }
    Ok(frames)
}

pub fn decode_rotate_4400(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4400 {
        return Err(error::Error::InvalidData);
    }
    let frame_count = read_u32_le(bytes, 4)? as usize;
    let mut pos = 12;
    let mut frames = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let mut q = if pos + 16 <= bytes.len() {
            Quat::from_array(read_vec4_f32_le(bytes, pos)?.to_array())
        } else if pos + 12 <= bytes.len() && i + 1 == frame_count {
            let v = read_vec3_f32_le(bytes, pos)?;
            // TODO: is this the correct w component?
            quat(v.x, v.y, v.z, 1.0)
        } else {
            return Err(error::Error::InvalidData);
        };
        pos += 16.min(bytes.len().saturating_sub(pos));
        let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
        if len2 > 0.0 {
            let inv = 1.0 / len2.sqrt();
            q.x *= inv;
            q.y *= inv;
            q.z *= inv;
            q.w *= inv;
        } else {
            q = Quat::IDENTITY;
        }
        frames.push(q);
    }
    Ok(frames)
}

pub fn decode_rotate_4200(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4200 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    if key_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }
    let mut frame_indices = Vec::with_capacity(key_count);
    let mut pos = 12;
    for _ in 0..key_count {
        frame_indices.push(read_u16_le(bytes, pos)? as usize);
        pos += 2;
    }
    pos = align_up(pos, 4);
    let mut key_vals = Vec::with_capacity(key_count);
    for i in 0..key_count {
        key_vals.push(read_vec4_f32_le(bytes, pos + i * 16)?);
    }
    let total_frames = frame_indices.iter().copied().max().unwrap_or(0) + 1;
    Ok(expand_sparse_quat(
        &frame_indices,
        &key_vals
            .iter()
            .map(|q| (q.x, q.y, q.z, q.w))
            .collect::<Vec<_>>(),
        total_frames,
    ))
}

pub fn decode_rotate_4208(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 16 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4208 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }
    if key_count > 33 {
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
    let quat0 = read_vec4_f32_le(bytes, pos + 4)?;
    let quat1 = read_vec4_f32_le(bytes, pos + 20)?;
    let residual_off = pos + 36;
    if residual_off > bytes.len() || residual_off % 4 != 0 {
        return Err(error::Error::InvalidData);
    }
    let block_len = key_count.saturating_sub(1).max(1);
    let mut key_quats = Vec::with_capacity(key_count);
    for local in 0..key_count {
        let t = local as f32 / block_len as f32;
        let k = Vector4 {
            x: quat0.x + (quat1.x - quat0.x) * t,
            y: quat0.y + (quat1.y - quat0.y) * t,
            z: quat0.z + (quat1.z - quat0.z) * t,
            w: quat0.w + (quat1.w - quat0.w) * t,
        };
        let (r_vec, _) =
            decode_residual_vector(bytes, residual_off, base_scale, local, 4, block_len)?;
        let mut q = Vector4 {
            x: k.x + r_vec[0],
            y: k.y + r_vec[1],
            z: k.z + r_vec[2],
            w: k.w + r_vec[3],
        };
        let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
        if len2 > 0.0 {
            let inv = 1.0 / len2.sqrt();
            q.x *= inv;
            q.y *= inv;
            q.z *= inv;
            q.w *= inv;
        }
        key_quats.push((q.x, q.y, q.z, q.w));
    }
    let total_frames = frame_indices.iter().copied().max().unwrap_or(0) + 1;
    Ok(expand_sparse_quat(&frame_indices, &key_quats, total_frames))
}

pub fn decode_rotate_4209(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 16 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4209 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
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
    let endpoints_size = endpoint_count * 16;
    if endpoint_base + endpoints_size > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for i in 0..endpoint_count {
        endpoints.push(read_vec4_f32_le(bytes, endpoint_base + i * 16)?);
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
    let mut key_quats = Vec::with_capacity(key_count);
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let mut block_len = compute_block_len_type9(key_count, block_idx);
        if block_len == 0 {
            block_len = 1;
        }
        let t = local as f32 / block_len as f32;
        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let k = Vector4 {
            x: e0.x + (e1.x - e0.x) * t,
            y: e0.y + (e1.y - e0.y) * t,
            z: e0.z + (e1.z - e0.z) * t,
            w: e0.w + (e1.w - e0.w) * t,
        };
        let rs = residual_starts[block_idx];
        let (r_vec, _) = decode_residual_vector(bytes, rs, base_scale, local, 4, block_len)?;
        let mut q = Vector4 {
            x: k.x + r_vec[0],
            y: k.y + r_vec[1],
            z: k.z + r_vec[2],
            w: k.w + r_vec[3],
        };
        let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
        if len2 > 0.0 {
            let inv = 1.0 / len2.sqrt();
            q.x *= inv;
            q.y *= inv;
            q.z *= inv;
            q.w *= inv;
        }
        key_quats.push((q.x, q.y, q.z, q.w));
    }
    let total_frames = frame_indices.iter().copied().max().unwrap_or(0) + 1;
    Ok(expand_sparse_quat(&frame_indices, &key_quats, total_frames))
}

pub fn decode_rotate_4308(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4308 {
        return Err(error::Error::InvalidData);
    }
    let key_count = read_u32_le(bytes, 4)? as usize;
    let _unk1 = read_f32_le(bytes, 8)?;
    if key_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }
    let mut pos = 12;
    if pos + key_count > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let frame_indices: Vec<usize> = bytes[pos..pos + key_count]
        .iter()
        .map(|v| *v as usize)
        .collect();
    pos = align_up(pos + key_count, 4);
    if pos + 36 > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let base_scale = read_f32_le(bytes, pos)?;
    let quat0 = read_vec4_f32_le(bytes, pos + 4)?;
    let quat1 = read_vec4_f32_le(bytes, pos + 20)?;
    pos += 36;
    let payload = &bytes[pos..];
    let nibble_size = (key_count * 4).div_ceil(2);
    if payload.len() == nibble_size {
        let mut key_quats = Vec::with_capacity(key_count);
        for i in 0..key_count {
            let byte = payload[i / 2];
            let raw = if i % 2 == 0 { byte & 0xF } else { byte >> 4 };
            let t = 1.0 - (raw as f32 / 15.0);
            let qx = quat0.x + (quat1.x - quat0.x) * t;
            let qy = quat0.y + (quat1.y - quat0.y) * t;
            let qz = quat0.z + (quat1.z - quat0.z) * t;
            let qw = quat0.w + (quat1.w - quat0.w) * t;
            let len2 = qx * qx + qy * qy + qz * qz + qw * qw;
            let inv = if len2 > 0.0 { 1.0 / len2.sqrt() } else { 1.0 };
            key_quats.push((qx * inv, qy * inv, qz * inv, qw * inv));
        }
        let total_frames = frame_indices.iter().copied().max().unwrap_or(0) + 1;
        return Ok(expand_sparse_quat(&frame_indices, &key_quats, total_frames));
    }

    let block_len = key_count.saturating_sub(1).max(1);
    let mut key_quats = Vec::with_capacity(key_count);
    for local in 0..key_count {
        let t = local as f32 / block_len as f32;
        let k = Vector4 {
            x: quat0.x + (quat1.x - quat0.x) * t,
            y: quat0.y + (quat1.y - quat0.y) * t,
            z: quat0.z + (quat1.z - quat0.z) * t,
            w: quat0.w + (quat1.w - quat0.w) * t,
        };
        let (r_vec, _) = decode_residual_vector(bytes, pos, base_scale, local, 4, block_len)?;
        let mut q = Vector4 {
            x: k.x + r_vec[0],
            y: k.y + r_vec[1],
            z: k.z + r_vec[2],
            w: k.w + r_vec[3],
        };
        let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
        if len2 > 0.0 {
            let inv = 1.0 / len2.sqrt();
            q.x *= inv;
            q.y *= inv;
            q.z *= inv;
            q.w *= inv;
        }
        key_quats.push((q.x, q.y, q.z, q.w));
    }
    let total_frames = frame_indices.iter().copied().max().unwrap_or(0) + 1;
    Ok(expand_sparse_quat(&frame_indices, &key_quats, total_frames))
}

pub fn decode_rotate_4408(bytes: &[u8]) -> Result<Vec<Quat>, error::Error> {
    if bytes.len() < 12 {
        return Err(error::Error::InvalidData);
    }
    if read_u32_le(bytes, 0)? != 0x4408 {
        return Err(error::Error::InvalidData);
    }
    let frame_count = read_u32_le(bytes, 4)? as usize;
    let _unk1 = read_f32_le(bytes, 8)?;
    let _unk2 = read_f32_le(bytes, 12)?;
    if frame_count == 0 {
        return Ok(vec![Quat::IDENTITY]);
    }
    let mut pos = 16;
    if pos + 32 > bytes.len() {
        return Err(error::Error::InvalidData);
    }
    let defaults = [
        read_vec4_f32_le(bytes, pos)?,
        read_vec4_f32_le(bytes, pos + 16)?,
    ];
    pos += 32;
    let payload = &bytes[pos..];

    if payload.len() == frame_count * 5 {
        let mut br = BitReader::from_slice(payload);
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let ix = br.read_u32(10)? as f32 / 1023.0;
            let iy = br.read_u32(10)? as f32 / 1023.0;
            let iz = br.read_u32(10)? as f32 / 1023.0;
            let iw = br.read_u32(10)? as f32 / 1023.0;
            let x = defaults[0].x + (defaults[1].x - defaults[0].x) * ix;
            let y = defaults[0].y + (defaults[1].y - defaults[0].y) * iy;
            let z = defaults[0].z + (defaults[1].z - defaults[0].z) * iz;
            let w = defaults[0].w + (defaults[1].w - defaults[0].w) * iw;
            let len2 = x * x + y * y + z * z + w * w;
            let inv = if len2 > 0.0 { 1.0 / len2.sqrt() } else { 1.0 };
            frames.push(quat(x * inv, y * inv, z * inv, w * inv));
        }
        return Ok(frames);
    }

    if frame_count > 34 {
        return Err(error::Error::InvalidData);
    }
    let base_scale = defaults[0].x;
    let block_len = frame_count.saturating_sub(1).max(1);
    let mut frames = Vec::with_capacity(frame_count);
    for local in 0..frame_count {
        let t = local as f32 / block_len as f32;
        let k = quat(
            defaults[0].x + (defaults[1].x - defaults[0].x) * t,
            defaults[0].y + (defaults[1].y - defaults[0].y) * t,
            defaults[0].z + (defaults[1].z - defaults[0].z) * t,
            defaults[0].w + (defaults[1].w - defaults[0].w) * t,
        );
        let (r_vec, _) = decode_residual_vector(bytes, pos, base_scale, local, 4, block_len)?;
        let mut q = quat(
            k.x + r_vec[0],
            k.y + r_vec[1],
            k.z + r_vec[2],
            k.w + r_vec[3],
        );
        let len2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
        if len2 > 0.0 {
            let inv = 1.0 / len2.sqrt();
            q.x *= inv;
            q.y *= inv;
            q.z *= inv;
            q.w *= inv;
        }
        frames.push(q);
    }
    Ok(frames)
}
