use std::f32::consts::PI;
use std::io::Cursor;
use std::sync::OnceLock;

use binrw::BinRead;
use glam::{Quat, Vec3, quat};

use crate::anim_data::{Vector3, Vector4, error};

pub(super) const G_CURVE_SHORT4_SCALE: f32 = 1.0 / 32767.0;
pub(super) const G_CURVE_INT8_SCALE: f32 = 1.0 / 127.0;
pub(super) const G_CURVE_NIBBLE_SCALE: f32 = 1.0 / 7.0;
pub(super) const G_CURVE_NIBBLE_BIAS: f32 = 8.0;

#[inline]
pub(super) fn read_u16_le(data: &[u8], off: usize) -> Result<u16, error::Error> {
    let mut cursor = Cursor::new(&data[off..]);
    u16::read_le(&mut cursor).map_err(|_| error::Error::InvalidData)
}

#[inline]
pub(super) fn read_u32_le(data: &[u8], off: usize) -> Result<u32, error::Error> {
    let mut cursor = Cursor::new(&data[off..]);
    u32::read_le(&mut cursor).map_err(|_| error::Error::InvalidData)
}

#[inline]
pub(super) fn read_f32_le(data: &[u8], off: usize) -> Result<f32, error::Error> {
    let mut cursor = Cursor::new(&data[off..]);
    f32::read_le(&mut cursor).map_err(|_| error::Error::InvalidData)
}

#[inline]
pub(super) fn align_up(x: usize, align: usize) -> usize {
    if align == 0 {
        return x;
    }
    let m = x % align;
    if m == 0 { x } else { x + (align - m) }
}

pub(super) fn read_vec3_f32_le(data: &[u8], off: usize) -> Result<Vector3, error::Error> {
    Ok(Vector3 {
        x: read_f32_le(data, off)?,
        y: read_f32_le(data, off + 4)?,
        z: read_f32_le(data, off + 8)?,
    })
}

pub(super) fn read_vec4_f32_le(data: &[u8], off: usize) -> Result<Vector4, error::Error> {
    Ok(Vector4 {
        x: read_f32_le(data, off)?,
        y: read_f32_le(data, off + 4)?,
        z: read_f32_le(data, off + 8)?,
        w: read_f32_le(data, off + 12)?,
    })
}

pub(super) struct KernelCache {
    data: Vec<f32>,
    offsets: [usize; 8],
}

impl KernelCache {
    fn generate() -> Self {
        let mut offsets = [0usize; 8];
        let mut accum = 0usize;
        for (i, slot) in offsets.iter_mut().enumerate() {
            *slot = accum;
            let dim = (i + 1) * 4;
            accum += dim * dim;
        }

        let mut data = Vec::with_capacity(3264);
        for v18 in 1..=8 {
            let dim = v18 * 4;
            let scale = (2.0 / dim as f32).sqrt();
            for row in 0..dim {
                for col in 0..dim {
                    let val = ((col as f32 + 0.5) * ((row as f32 + 0.5) * (PI / dim as f32))).cos()
                        * scale;
                    data.push(val);
                }
            }
        }
        KernelCache { data, offsets }
    }

    pub(super) fn row(&self, v18: usize, row_idx: usize) -> &[f32] {
        let dim = v18 * 4;
        let base = self.offsets[v18 - 1];
        let start = base + row_idx * dim;
        let end = start + dim;
        &self.data[start..end]
    }
}

pub(super) fn kernel() -> &'static KernelCache {
    static CACHE: OnceLock<KernelCache> = OnceLock::new();
    CACHE.get_or_init(KernelCache::generate)
}

pub(super) fn decode_residual_component(
    residual_stream: &[u8],
    stream_offset: usize,
    base_scale: f32,
    local_idx: usize,
    block_len: usize,
) -> Result<(f32, usize), error::Error> {
    if local_idx == 0 || local_idx >= block_len {
        return Ok((0.0, stream_offset));
    }
    let mut curr = stream_offset;
    let word0 = read_u16_le(residual_stream, curr)? as f32;
    let word1 = read_u16_le(residual_stream, curr + 2)? as f32;
    let byte4 = residual_stream
        .get(curr + 4)
        .copied()
        .ok_or(error::Error::InvalidData)? as f32;
    let byte5 = residual_stream
        .get(curr + 5)
        .copied()
        .ok_or(error::Error::InvalidData)?;
    let byte6 = residual_stream
        .get(curr + 6)
        .copied()
        .ok_or(error::Error::InvalidData)?;
    let byte7 = residual_stream
        .get(curr + 7)
        .copied()
        .ok_or(error::Error::InvalidData)?;
    curr += 8;

    let v14 = (byte5 >> 4) as usize;
    let v15 = (byte5 & 0xF) as usize;
    let v12 = (byte6 & 0xF) as usize;
    let v13 = (byte6 >> 4) as usize;
    let v18 = v14 + v15 + v12 + v13;
    if v18 == 0 || v18 > 8 {
        return Err(error::Error::InvalidData);
    }

    let base_amp = word0 * (1.0 / 65536.0) * base_scale;
    let slope = word1 * (1.0 / 65536.0) * base_amp;
    let amp_u8 = byte4;

    let v16 = (byte7 >> 4) as usize;
    let v17 = (byte7 & 0xF) as usize;
    if v16 + v17 > 8 {
        return Err(error::Error::InvalidData);
    }
    let v19 = 8 - v16 - v17;

    let mut w = [0.0f32; 8];
    let mut wi = 0usize;
    for _ in 0..v16 {
        w[wi] = base_amp;
        wi += 1;
    }
    for _ in 0..v17 {
        w[wi] = slope;
        wi += 1;
    }
    let scaled_slope = slope * (amp_u8 / 255.0);
    for _ in 0..v19 {
        w[wi] = scaled_slope;
        wi += 1;
    }

    let basis_row = kernel().row(v18, local_idx - 1);
    let mut acc = 0.0f32;
    let mut entry_idx = 0usize;
    let mut basis_ptr = 0usize;

    for _ in 0..v14 {
        if curr + 8 > residual_stream.len() {
            return Err(error::Error::InvalidData);
        }
        let shorts = [
            i16::from_le_bytes([residual_stream[curr], residual_stream[curr + 1]]) as f32
                * G_CURVE_SHORT4_SCALE,
            i16::from_le_bytes([residual_stream[curr + 2], residual_stream[curr + 3]]) as f32
                * G_CURVE_SHORT4_SCALE,
            i16::from_le_bytes([residual_stream[curr + 4], residual_stream[curr + 5]]) as f32
                * G_CURVE_SHORT4_SCALE,
            i16::from_le_bytes([residual_stream[curr + 6], residual_stream[curr + 7]]) as f32
                * G_CURVE_SHORT4_SCALE,
        ];
        curr += 8;
        let basis = &basis_row[basis_ptr..basis_ptr + 4];
        basis_ptr += 4;
        let weight = w[entry_idx % 8];
        entry_idx += 1;
        for i in 0..4 {
            acc += basis[i] * shorts[i] * weight;
        }
    }

    for _ in 0..v15 {
        if curr + 4 > residual_stream.len() {
            return Err(error::Error::InvalidData);
        }
        let coeff = [
            residual_stream[curr] as i8 as f32 * G_CURVE_INT8_SCALE,
            residual_stream[curr + 1] as i8 as f32 * G_CURVE_INT8_SCALE,
            residual_stream[curr + 2] as i8 as f32 * G_CURVE_INT8_SCALE,
            residual_stream[curr + 3] as i8 as f32 * G_CURVE_INT8_SCALE,
        ];
        curr += 4;
        let basis = &basis_row[basis_ptr..basis_ptr + 4];
        basis_ptr += 4;
        let weight = w[entry_idx % 8];
        entry_idx += 1;
        for i in 0..4 {
            acc += basis[i] * coeff[i] * weight;
        }
    }

    let pair_count = v13 >> 1;
    for _ in 0..pair_count {
        if curr + 4 > residual_stream.len() {
            return Err(error::Error::InvalidData);
        }
        let b = [
            residual_stream[curr],
            residual_stream[curr + 1],
            residual_stream[curr + 2],
            residual_stream[curr + 3],
        ];
        curr += 4;
        let decode_nibble = |val: u8, hi: bool| -> f32 {
            let n = if hi { val >> 4 } else { val & 0xF };
            (n as f32 - G_CURVE_NIBBLE_BIAS) * G_CURVE_NIBBLE_SCALE
        };
        let hi_vec = [
            decode_nibble(b[0], true),
            decode_nibble(b[1], true),
            decode_nibble(b[2], true),
            decode_nibble(b[3], true),
        ];
        let basis_hi = &basis_row[basis_ptr..basis_ptr + 4];
        basis_ptr += 4;
        let weight_hi = w[entry_idx % 8];
        entry_idx += 1;
        for i in 0..4 {
            acc += basis_hi[i] * hi_vec[i] * weight_hi;
        }
        let lo_vec = [
            decode_nibble(b[0], false),
            decode_nibble(b[1], false),
            decode_nibble(b[2], false),
            decode_nibble(b[3], false),
        ];
        let basis_lo = &basis_row[basis_ptr..basis_ptr + 4];
        basis_ptr += 4;
        let weight_lo = w[entry_idx % 8];
        entry_idx += 1;
        for i in 0..4 {
            acc += basis_lo[i] * lo_vec[i] * weight_lo;
        }
    }

    Ok((acc, curr))
}

pub(super) fn decode_residual_vector(
    residual_stream: &[u8],
    start_offset: usize,
    base_scale: f32,
    local_idx: usize,
    comp_bits: usize,
    block_len: usize,
) -> Result<(Vec<f32>, usize), error::Error> {
    let mut result = vec![0.0f32; comp_bits];
    let mut curr = start_offset;
    for c in 0..comp_bits {
        let (val, next) =
            decode_residual_component(residual_stream, curr, base_scale, local_idx, block_len)?;
        result[c] = val;
        curr = next;
    }
    Ok((result, curr))
}

pub(super) fn compute_block_len(key_count: usize, block_idx: usize) -> usize {
    if key_count <= 1 {
        return 1;
    }
    let last_block = (key_count - 1) / 33;
    if block_idx == last_block {
        key_count - 33 * block_idx - 1
    } else {
        33
    }
}

pub(super) fn compute_block_count(key_count: usize) -> usize {
    if key_count <= 1 {
        1
    } else {
        (key_count - 1) / 33 + 1
    }
}

pub(super) fn compute_block_qcounts(
    payload: &[u8],
    residual_start0: usize,
    base_scale: f32,
    comp_bits: usize,
    key_count: usize,
) -> Result<Vec<usize>, error::Error> {
    if key_count <= 1 {
        return Ok(vec![0]);
    }
    let blocks = compute_block_count(key_count);
    let mut q_counts = Vec::with_capacity(blocks);
    let mut cursor = residual_start0;
    for block_idx in 0..blocks {
        let block_len = compute_block_len(key_count, block_idx);
        if block_len <= 1 {
            q_counts.push(0);
            continue;
        }
        let (_, end_off) =
            decode_residual_vector(payload, cursor, base_scale, 1, comp_bits, block_len)?;
        let delta = end_off - cursor;
        if delta == 0 || !delta.is_multiple_of(4) {
            return Err(error::Error::InvalidData);
        }
        q_counts.push(delta / 4);
        cursor = end_off;
    }
    Ok(q_counts)
}

pub(super) fn expand_sparse_vec3(
    frame_indices: &[usize],
    values: &[Vec3],
    total_frames: usize,
) -> Vec<Vec3> {
    if values.is_empty() || frame_indices.is_empty() {
        return vec![Vec3::ZERO];
    }
    let mut frames = vec![values[0]; total_frames];

    let first_f = frame_indices[0];
    for f in 0..first_f.min(total_frames) {
        frames[f] = values[0];
    }

    for i in 0..frame_indices.len().saturating_sub(1) {
        let f0 = frame_indices[i];
        let mut f1 = frame_indices[i + 1];
        if f1 <= f0 {
            continue;
        }
        if f1 >= total_frames {
            f1 = total_frames - 1;
        }
        let span = f1.saturating_sub(f0);
        if span == 0 {
            frames[f0] = values[i];
            continue;
        }
        let [x0, y0, z0] = values[i].to_array();
        let [x1, y1, z1] = values[i + 1].to_array();
        for f in f0..=f1 {
            let t = (f - f0) as f32 / span as f32;
            frames[f] = Vec3 {
                x: x0 + (x1 - x0) * t,
                y: y0 + (y1 - y0) * t,
                z: z0 + (z1 - z0) * t,
            };
        }
    }

    if let Some(&last_f) = frame_indices.last() {
        for f in last_f.min(total_frames)..total_frames {
            let [x, y, z] = values.last().unwrap().to_array();
            frames[f] = Vec3 { x, y, z };
        }
    }
    frames
}

pub(super) fn expand_sparse_quat(
    frame_indices: &[usize],
    values: &[(f32, f32, f32, f32)],
    total_frames: usize,
) -> Vec<Quat> {
    if values.is_empty() || frame_indices.is_empty() {
        return vec![Quat::IDENTITY];
    }
    let mut frames = vec![quat(values[0].0, values[0].1, values[0].2, values[0].3,); total_frames];

    let first_f = frame_indices[0];
    for f in 0..first_f.min(total_frames) {
        frames[f] = quat(values[0].0, values[0].1, values[0].2, values[0].3);
    }

    for i in 0..frame_indices.len().saturating_sub(1) {
        let f0 = frame_indices[i];
        let mut f1 = frame_indices[i + 1];
        if f1 <= f0 {
            continue;
        }
        if f1 >= total_frames {
            f1 = total_frames - 1;
        }
        let span = f1.saturating_sub(f0);
        if span == 0 {
            frames[f0] = quat(values[i].0, values[i].1, values[i].2, values[i].3);
            continue;
        }
        let (x0, y0, z0, w0) = values[i];
        let (x1, y1, z1, w1) = values[i + 1];
        for f in f0..=f1 {
            let t = (f - f0) as f32 / span as f32;
            frames[f] = quat(
                x0 + (x1 - x0) * t,
                y0 + (y1 - y0) * t,
                z0 + (z1 - z0) * t,
                w0 + (w1 - w0) * t,
            );
        }
    }

    if let Some(&last_f) = frame_indices.last() {
        for f in last_f.min(total_frames)..total_frames {
            let (x, y, z, w) = *values.last().unwrap();
            frames[f] = quat(x, y, z, w);
        }
    }
    frames
}

pub(super) fn compute_block_count_type9(key_count: usize) -> usize {
    if key_count <= 1 {
        1
    } else {
        (key_count - 2) / 33 + 1
    }
}

pub(super) fn compute_block_len_type9(key_count: usize, block_idx: usize) -> usize {
    if key_count <= 1 {
        1
    } else {
        let last_block = compute_block_count_type9(key_count) - 1;
        if block_idx == last_block {
            key_count - 33 * block_idx - 1
        } else {
            33
        }
    }
}

pub(super) fn compute_block_count_4309(key_count: usize) -> usize {
    if key_count <= 1 {
        1
    } else {
        (key_count - 1) / 33 + 1
    }
}

pub(super) fn compute_block_len_4309(key_count: usize, block_idx: usize) -> usize {
    if key_count <= 1 {
        return 1;
    }
    let last_block = (key_count - 1) / 33;
    if block_idx == last_block {
        key_count - 33 * block_idx - 1
    } else {
        33
    }
}

pub(super) fn compute_block_qcounts_4309(
    payload: &[u8],
    residual_start0: usize,
    base_scale: f32,
    comp_bits: usize,
    key_count: usize,
) -> Result<Vec<usize>, error::Error> {
    if key_count <= 1 {
        return Ok(vec![0]);
    }

    let last_block = (key_count - 1) / 33;
    let blocks = last_block + 1;

    let mut q_counts = Vec::with_capacity(blocks);
    let mut cursor = residual_start0;
    for block_idx in 0..blocks {
        let block_len = compute_block_len_4309(key_count, block_idx);
        if block_len <= 1 {
            q_counts.push(0);
            continue;
        }

        let (_, end_off) =
            decode_residual_vector(payload, cursor, base_scale, 1, comp_bits, block_len)?;
        let delta = end_off - cursor;
        if delta == 0 || !delta.is_multiple_of(4) {
            return Err(error::Error::InvalidData);
        }
        q_counts.push(delta / 4);
        cursor = end_off;
    }
    Ok(q_counts)
}

pub(super) fn try_pick_comp_bits_4309(
    payload: &[u8],
    residual_start0: usize,
    base_scale: f32,
    key_count: usize,
) -> Result<(usize, Vec<usize>, usize), error::Error> {
    let mut best: Option<(usize, Vec<usize>, usize)> = None;
    let mut best_key: Option<(usize, i32)> = None;

    for comp_bits in [4, 3, 2, 1] {
        let q_counts = match compute_block_qcounts_4309(
            payload,
            residual_start0,
            base_scale,
            comp_bits,
            key_count,
        ) {
            Ok(q) => q,
            Err(_) => continue,
        };

        let sum: usize = q_counts.iter().sum();
        let end_off = residual_start0 + 4 * sum;
        if end_off > payload.len() {
            continue;
        }
        let slack = payload.len() - end_off;
        let cand_key = (slack, -(comp_bits as i32));
        if best_key.is_none() || cand_key < best_key.unwrap() {
            best = Some((comp_bits, q_counts, end_off));
            best_key = Some(cand_key);
        }
    }

    best.ok_or(error::Error::InvalidData)
}

pub(super) fn try_parse_endpoints_4309(
    curve_bytes: &[u8],
    endpoints_off: usize,
    endpoint_count: usize,
    elem_size: usize,
) -> Result<Vec<Quat>, error::Error> {
    let end = endpoints_off + endpoint_count * elem_size;
    if endpoints_off >= curve_bytes.len() || end > curve_bytes.len() {
        return Err(error::Error::InvalidData);
    }
    if !endpoints_off.is_multiple_of(4) {
        return Err(error::Error::InvalidData);
    }

    let mut out = Vec::with_capacity(endpoint_count);
    let mut max_abs = 0.0f32;

    if elem_size == 16 {
        for i in 0..endpoint_count {
            let q = read_vec4_f32_le(curve_bytes, endpoints_off + i * 16)?;
            max_abs = max_abs
                .max(q.x.abs())
                .max(q.y.abs())
                .max(q.z.abs())
                .max(q.w.abs());
            out.push(quat_normalize(q.x, q.y, q.z, q.w));
        }
    } else if elem_size == 12 {
        for i in 0..endpoint_count {
            let v = read_vec3_f32_le(curve_bytes, endpoints_off + i * 12)?;
            max_abs = max_abs.max(v.x.abs()).max(v.y.abs()).max(v.z.abs());
            out.push(reconstruct_quat_from_xyz(v.x, v.y, v.z, 1.0));
        }
    } else {
        return Err(error::Error::InvalidData);
    }

    if !max_abs.is_finite() || max_abs > 1.0e6 {
        return Err(error::Error::InvalidData);
    }

    for i in 1..out.len() {
        if out[i - 1].dot(out[i]) < 0.0 {
            out[i] = -out[i];
        }
    }

    Ok(out)
}

pub(super) fn quat_normalize(x: f32, y: f32, z: f32, w: f32) -> Quat {
    let n2 = x * x + y * y + z * z + w * w;
    if n2 <= 0.0 {
        return Quat::IDENTITY;
    }
    let inv = 1.0 / n2.sqrt();
    quat(x * inv, y * inv, z * inv, w * inv)
}

pub(super) fn quat_nlerp(q0: Quat, q1: Quat, t: f32) -> Quat {
    let x = q0.x + (q1.x - q0.x) * t;
    let y = q0.y + (q1.y - q0.y) * t;
    let z = q0.z + (q1.z - q0.z) * t;
    let w = q0.w + (q1.w - q0.w) * t;
    quat_normalize(x, y, z, w)
}

pub(super) fn reconstruct_quat_from_xyz(x: f32, y: f32, z: f32, sign: f32) -> Quat {
    let s = x * x + y * y + z * z;
    let w = sign * (1.0 - s).max(0.0).sqrt();
    quat_normalize(x, y, z, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f32::consts::PI;

    #[test]
    fn kernel_row_matches_formula() {
        let cache = kernel();
        let first = cache.row(1, 0)[0];
        let expected = ((0.5_f32) * (0.5_f32 * (PI / 4.0))).cos() * (2.0_f32 / 4.0).sqrt();
        assert!(
            (first - expected).abs() < 1e-6,
            "first={}, expected={}",
            first,
            expected
        );

        let row1_col2 = cache.row(1, 1)[2];
        let expected_row1_col2 =
            ((2.5_f32) * (1.5_f32 * (PI / 4.0))).cos() * (2.0_f32 / 4.0).sqrt();
        assert!(
            (row1_col2 - expected_row1_col2).abs() < 1e-6,
            "row1_col2={}, expected={}",
            row1_col2,
            expected_row1_col2
        );
    }

    #[test]
    fn residual_component_advances_cursor_and_decodes() {
        let mut residual = Vec::new();
        residual.extend_from_slice(&0x4000u16.to_le_bytes());
        residual.extend_from_slice(&0x8000u16.to_le_bytes());
        residual.push(255);
        residual.push(0x10);
        residual.push(0x00);
        residual.push(0x00);
        residual.extend_from_slice(&[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let base_scale = 1.0;
        let local_idx = 1;
        let block_len = 4;
        let (value, next) =
            decode_residual_component(&residual, 0, base_scale, local_idx, block_len).unwrap();

        assert_eq!(next, 16);
        assert!(value.abs() > 0.0);
        assert!(value.abs() < 1.0);
    }
}
