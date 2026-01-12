use glam::{Quat, Vec3, quat};

// Anim v1.2 (nuanmb) encoding helpers for EXVS2 compatibility.
// This module provides encoders for compressed animation formats that match
// the game's expectations for playback.
use crate::anim_data::{Vector3, error};

#[inline]
fn align_up(x: usize, align: usize) -> usize {
    if align == 0 {
        return x;
    }
    let m = x % align;
    if m == 0 { x } else { x + (align - m) }
}

fn compute_block_count(key_count: usize) -> usize {
    if key_count <= 1 {
        1
    } else {
        (key_count - 1) / 33 + 1
    }
}

fn compute_block_len(key_count: usize, block_idx: usize) -> usize {
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

/// Encode Vector3 data to 0x3409 compressed format.
///
/// This encoder produces compressed Vector3 data (translation/scale) compatible with EXVS2.
/// It uses a simplified but effective approach:
/// - 33-key blocks with endpoint + residual encoding
/// - Endpoints fitted using block first/last values
/// - Residuals encoded with minimal DCT coefficients
///
/// # Parameters
/// - `values`: The Vector3 values to encode (one per frame)
///
/// # Returns
/// Compressed buffer with 0x3409 header
///
/// # Encoding Strategy
/// - Block size: 33 keys per block (matches decoder)
/// - Endpoints: Use first and last value of each block
/// - Residuals: DCT-based encoding for accurate reconstruction
/// - comp_bits: Fixed at 3 (X, Y, Z components)
///
/// # Notes
/// This is a simplified encoder that prioritizes game compatibility over
/// optimal compression ratio.
///
/// # Usage
/// Can be used for both translation and scale data, as they share the same format.
pub fn encode_vector3_3409(values: &[Vec3]) -> Result<Vec<u8>, error::Error> {
    if values.is_empty() {
        return Err(error::Error::InvalidData);
    }

    let key_count = values.len();
    let blocks = compute_block_count(key_count);
    let endpoint_count = blocks + 1;

    // Step 1: Fit endpoints (use first/last value of each block)
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for block_idx in 0..blocks {
        let start_key = block_idx * 33;
        endpoints.push(values[start_key]);
    }
    // Add final endpoint
    endpoints.push(values[key_count - 1]);

    // Step 2: Compute residuals for all keys
    let mut residuals = Vec::with_capacity(key_count);
    let mut max_residual = 0.0f32;

    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let block_len = compute_block_len(key_count, block_idx).max(1);
        let t = local as f32 / block_len as f32;

        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let predicted = Vector3 {
            x: e0.x + (e1.x - e0.x) * t,
            y: e0.y + (e1.y - e0.y) * t,
            z: e0.z + (e1.z - e0.z) * t,
        };

        let actual = values[key_idx];
        let residual = Vector3 {
            x: actual.x - predicted.x,
            y: actual.y - predicted.y,
            z: actual.z - predicted.z,
        };

        max_residual = max_residual
            .max(residual.x.abs())
            .max(residual.y.abs())
            .max(residual.z.abs());

        residuals.push(residual);
    }

    // Step 3: Compute base_scale (max absolute residual)
    let base_scale = if max_residual > 1e-6 {
        max_residual
    } else {
        1.0
    };

    // Step 4: Encode residuals using actual residual data
    let residual_stream = encode_residuals(&residuals, key_count, base_scale);

    // Step 5: Pack the buffer
    let mut data = Vec::new();

    // Header
    data.extend_from_slice(&0x3409u32.to_le_bytes());
    data.extend_from_slice(&(key_count as u32).to_le_bytes());
    data.extend_from_slice(&1.0f32.to_le_bytes()); // unk1
    data.extend_from_slice(&base_scale.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes()); // flags
    data.extend_from_slice(&0u16.to_le_bytes()); // bits

    // Endpoints (Vector3 f32 format, 12 bytes each)
    for ep in &endpoints {
        data.extend_from_slice(&ep.x.to_le_bytes());
        data.extend_from_slice(&ep.y.to_le_bytes());
        data.extend_from_slice(&ep.z.to_le_bytes());
    }

    // Residual stream (already 4-byte aligned)
    data.extend_from_slice(&residual_stream);

    Ok(data)
}

/// Encode residuals for 0x3409 format.
///
/// This encodes actual residual data for each block using DCT-like coefficients.
/// The residuals represent the difference between actual values and linear interpolation
/// between endpoints.
///
/// # Parameters
/// - `residuals`: Pre-computed residual values (actual - predicted)
/// - `key_count`: Total number of keys in the animation
/// - `base_scale`: Scale factor for residual quantization
///
/// # Returns
/// Encoded residual stream (4-byte aligned)
fn encode_residuals(residuals: &[Vector3], key_count: usize, base_scale: f32) -> Vec<u8> {
    let blocks = compute_block_count(key_count);
    let mut stream = Vec::new();

    // For each block, encode residuals for X, Y, Z components
    for block_idx in 0..blocks {
        let block_len = compute_block_len(key_count, block_idx);
        if block_len <= 1 {
            continue; // No residuals needed for single-key blocks
        }

        let start_key = block_idx * 33 + 1; // Skip first key (endpoint)
        let end_key = (block_idx * 33 + block_len + 1).min(key_count);

        // Extract residuals for this block
        let mut block_residuals_x = Vec::new();
        let mut block_residuals_y = Vec::new();
        let mut block_residuals_z = Vec::new();

        for key_idx in start_key..end_key {
            block_residuals_x.push(residuals[key_idx].x);
            block_residuals_y.push(residuals[key_idx].y);
            block_residuals_z.push(residuals[key_idx].z);
        }

        // Encode each component
        encode_residual_component(&mut stream, &block_residuals_x, base_scale);
        encode_residual_component(&mut stream, &block_residuals_y, base_scale);
        encode_residual_component(&mut stream, &block_residuals_z, base_scale);
    }

    // Ensure at least 4 bytes in the stream for decoder inference to work
    // The decoder's inference logic requires residual_off < bytes.len()
    if stream.is_empty() {
        stream.extend_from_slice(&[0u8; 4]);
    }

    // Ensure 4-byte alignment
    while stream.len() % 4 != 0 {
        stream.push(0);
    }

    stream
}

/// Encode a residual component using simplified DCT-like coefficients.
///
/// This creates a residual encoding that matches the game's decoder expectations.
/// Uses a minimal coefficient representation that balances compression and accuracy.
///
/// # Strategy
/// - Compute DCT-like transform coefficients from residual samples
/// - Quantize coefficients based on base_scale
/// - Pack into the expected binary format with proper headers
///
/// # Layout
/// - word0 (u16): base amplitude (max absolute residual, scaled)
/// - word1 (u16): slope (linear trend, scaled)
/// - byte4 (u8): amplitude multiplier
/// - byte5 (u8): v14 (16-bit coeff count), v15 (8-bit coeff count)
/// - byte6 (u8): v12, v13 (additional coefficient flags)
/// - byte7 (u8): v16, v17 (weight distribution flags)
/// - coefficient data: quantized DCT coefficients
fn encode_residual_component(stream: &mut Vec<u8>, residuals: &[f32], base_scale: f32) {
    if residuals.is_empty() {
        return;
    }

    // Compute statistics for this component
    let mut max_abs = 0.0f32;
    let mut sum = 0.0f32;
    for &r in residuals {
        max_abs = max_abs.max(r.abs());
        sum += r;
    }
    let _mean = sum / residuals.len() as f32;

    // Compute linear trend (slope)
    let mut slope = 0.0f32;
    if residuals.len() > 1 {
        let first = residuals[0];
        let last = residuals[residuals.len() - 1];
        slope = (last - first) / (residuals.len() - 1) as f32;
    }

    // Compute DCT-like coefficients (simplified: use first few harmonics)
    let coeff_count = residuals.len().min(8);
    let mut coefficients = vec![0.0f32; coeff_count];

    for k in 0..coeff_count {
        let mut sum_cos = 0.0f32;
        for (n, &r) in residuals.iter().enumerate() {
            let angle = std::f32::consts::PI * k as f32 * (n as f32 + 0.5) / residuals.len() as f32;
            sum_cos += r * angle.cos();
        }
        coefficients[k] = sum_cos * 2.0 / residuals.len() as f32;
    }

    // Quantization parameters
    let epsilon = 1e-6;
    let amplitude_scale = if max_abs > epsilon { max_abs } else { epsilon };

    // word0: base amplitude (normalized to base_scale)
    let word0_f = (amplitude_scale / base_scale.max(epsilon)) * 65535.0;
    let word0 = word0_f.clamp(0.0, 65535.0) as u16;
    stream.extend_from_slice(&word0.to_le_bytes());

    // word1: slope (normalized)
    let slope_scale = if base_scale > epsilon {
        base_scale
    } else {
        epsilon
    };
    let word1_f = ((slope / slope_scale) * 32767.0 + 32768.0).clamp(0.0, 65535.0);
    let word1 = word1_f as u16;
    stream.extend_from_slice(&word1.to_le_bytes());

    // byte4: amplitude multiplier (fixed at 0 for simplicity)
    stream.push(0u8);

    // byte5: v14 (16-bit coeff count), v15 (8-bit coeff count)
    // Use conservative counts: v14=1 (one 16-bit group = 4 coeffs)
    let v14 = 1u8;
    let v15 = 0u8;
    stream.push((v14 << 4) | v15);

    // byte6: v12=0, v13=0 (no additional flags)
    stream.push(0x00u8);

    // byte7: v16=0, v17=0 (standard weight distribution)
    stream.push(0x00u8);

    // Encode coefficients: quantize and pack
    // For v14=1, we have 4 coefficients (16-bit each = 8 bytes total)
    for i in 0..4 {
        let coeff = if i < coefficients.len() {
            coefficients[i]
        } else {
            0.0
        };

        // Quantize to 16-bit signed
        let quantized =
            ((coeff / amplitude_scale.max(epsilon)) * 32767.0).clamp(-32768.0, 32767.0) as i16;
        stream.extend_from_slice(&quantized.to_le_bytes());
    }
}

/// Encode quaternion data to 0x4409 compressed format.
///
/// This encoder targets EXVS2 Anim v1.2 compatibility. The format matches the
/// validated decode model in `nuanmb_v12/rotate_4409.rs`:
/// - 33-key blocks
/// - vec4<f32> endpoints (block_count + 1)
/// - residual stream encoded using the same residual/kernel family as 0x3409/0x4409 decoders
///
/// Note: This is a lossy encoder.
pub fn encode_rotate_4409(values: &[Quat]) -> Result<Vec<u8>, error::Error> {
    if values.is_empty() {
        return Err(error::Error::InvalidData);
    }

    let key_count = values.len();
    let blocks = compute_block_count(key_count);
    let endpoint_count = blocks + 1;

    // Step 1: Fit endpoints (use first key of each block, plus final key).
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for block_idx in 0..blocks {
        let start_key = block_idx * 33;
        endpoints.push(values[start_key]);
    }
    endpoints.push(values[key_count - 1]);

    // Step 2: Compute residuals.
    let mut residuals = Vec::with_capacity(key_count);
    let mut max_residual = 0.0f32;
    for key_idx in 0..key_count {
        let block_idx = key_idx / 33;
        let local = key_idx - 33 * block_idx;
        let block_len = compute_block_len(key_count, block_idx).max(1);
        let t = local as f32 / block_len as f32;

        let e0 = endpoints[block_idx];
        let e1 = endpoints[block_idx + 1];
        let predicted = quat(
            e0.x + (e1.x - e0.x) * t,
            e0.y + (e1.y - e0.y) * t,
            e0.z + (e1.z - e0.z) * t,
            e0.w + (e1.w - e0.w) * t,
        );

        let actual = values[key_idx];
        let r = quat(
            actual.x - predicted.x,
            actual.y - predicted.y,
            actual.z - predicted.z,
            actual.w - predicted.w,
        );

        max_residual = max_residual
            .max(r.x.abs())
            .max(r.y.abs())
            .max(r.z.abs())
            .max(r.w.abs());
        residuals.push(r);
    }

    // Step 3: Compute base_scale (max absolute residual).
    let base_scale = if max_residual > 1e-6 {
        max_residual
    } else {
        1.0
    };

    // Step 4: Encode residual stream.
    let residual_stream = encode_residuals_vec4(&residuals, key_count, base_scale);

    // Step 5: Pack buffer.
    let mut data = Vec::new();
    data.extend_from_slice(&0x4409u32.to_le_bytes());
    data.extend_from_slice(&(key_count as u32).to_le_bytes());
    data.extend_from_slice(&1.0f32.to_le_bytes()); // unk1
    data.extend_from_slice(&base_scale.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes()); // flags
    data.extend_from_slice(&0u16.to_le_bytes()); // bits

    // Endpoints as vec4<f32>.
    for ep in &endpoints {
        data.extend_from_slice(&ep.x.to_le_bytes());
        data.extend_from_slice(&ep.y.to_le_bytes());
        data.extend_from_slice(&ep.z.to_le_bytes());
        data.extend_from_slice(&ep.w.to_le_bytes());
    }

    // Residual stream (already 4-byte aligned).
    data.extend_from_slice(&residual_stream);
    Ok(data)
}

fn encode_residuals_vec4(residuals: &[Quat], key_count: usize, base_scale: f32) -> Vec<u8> {
    let blocks = compute_block_count(key_count);
    let mut stream = Vec::new();

    for block_idx in 0..blocks {
        let block_len = compute_block_len(key_count, block_idx);
        if block_len <= 1 {
            continue;
        }

        let start_key = block_idx * 33 + 1; // Skip first key (endpoint).
        let end_key = (block_idx * 33 + block_len + 1).min(key_count);

        let mut bx = Vec::new();
        let mut by = Vec::new();
        let mut bz = Vec::new();
        let mut bw = Vec::new();
        for key_idx in start_key..end_key {
            bx.push(residuals[key_idx].x);
            by.push(residuals[key_idx].y);
            bz.push(residuals[key_idx].z);
            bw.push(residuals[key_idx].w);
        }

        encode_residual_component(&mut stream, &bx, base_scale);
        encode_residual_component(&mut stream, &by, base_scale);
        encode_residual_component(&mut stream, &bz, base_scale);
        encode_residual_component(&mut stream, &bw, base_scale);
    }

    if stream.is_empty() {
        stream.extend_from_slice(&[0u8; 4]);
    }
    while (stream.len() % 4) != 0 {
        stream.push(0);
    }
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_abs_diff_eq;
    use glam::vec3;

    use crate::anim_data::compression::v1::decode_vector3_3409;

    #[test]
    fn encode_decode_vector3_3409_simple() {
        // Test that encoding and decoding produce similar results
        let original = vec![
            vec3(0.0, 1.0, 0.0),
            vec3(0.1, 1.1, 0.1),
            vec3(0.2, 1.2, 0.2),
            vec3(0.3, 1.3, 0.3),
        ];

        let encoded = encode_vector3_3409(&original).unwrap();

        // Verify header
        assert_eq!(&encoded[0..4], &0x3409u32.to_le_bytes());
        assert_eq!(&encoded[4..8], &4u32.to_le_bytes()); // key_count

        let decoded = decode_vector3_3409(&encoded).unwrap();

        // Should have same frame count
        assert_eq!(decoded.len(), original.len());

        // Check approximate equality (lossy compression)
        for (i, (orig, dec)) in original.iter().zip(decoded.iter()).enumerate() {
            let err_x = (orig.x - dec.x).abs();
            let err_y = (orig.y - dec.y).abs();
            let err_z = (orig.z - dec.z).abs();
            assert!(
                err_x < 0.1 && err_y < 0.1 && err_z < 0.1,
                "Frame {} error too large: ({}, {}, {})",
                i,
                err_x,
                err_y,
                err_z
            );
        }
    }

    #[test]
    fn encode_decode_vector3_3409_multi_block() {
        // Test with more than 33 keys (multiple blocks)
        let mut original = Vec::new();
        for i in 0..100 {
            let t = i as f32 / 99.0;
            original.push(vec3(t * 10.0, t.sin() * 5.0, t.cos() * 5.0));
        }

        let encoded = encode_vector3_3409(&original).unwrap();
        let decoded = decode_vector3_3409(&encoded).unwrap();

        assert_eq!(decoded.len(), original.len());

        // Check endpoints are preserved.
        assert_abs_diff_eq!(original[0], decoded[0], epsilon = 0.01);
        assert_abs_diff_eq!(original[99], decoded[99], epsilon = 0.01);
    }

    #[test]
    fn encode_vector3_3409_single_frame() {
        let original = vec![vec3(1.0, 2.0, 3.0)];
        let encoded = encode_vector3_3409(&original).unwrap();
        let decoded = decode_vector3_3409(&encoded).unwrap();

        assert_eq!(decoded.len(), 1);
        assert_abs_diff_eq!(decoded[0], vec3(1.0, 2.0, 3.0), epsilon = 0.01);
    }

    #[test]
    fn encode_vector3_3409_empty_fails() {
        let result = encode_vector3_3409(&[]);
        assert!(result.is_err());
    }

    // TODO: test 4409

    #[test]
    fn encode_rotate_4409_empty_fails() {
        let result = encode_rotate_4409(&[]);
        assert!(result.is_err());
    }
}
