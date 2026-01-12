mod common;
mod encode;
mod rotate_4409;
mod rotate_basic;
mod rotate_inferred;
mod translate;

pub use encode::*;
pub use rotate_4409::decode_rotate_4409;
pub use rotate_basic::{
    decode_rotate_4200, decode_rotate_4208, decode_rotate_4209, decode_rotate_4300,
    decode_rotate_4308, decode_rotate_4400, decode_rotate_4408,
};
pub use rotate_inferred::decode_rotate_4309;
pub use translate::{
    decode_translate_3200, decode_translate_3208, decode_translate_3209, decode_translate_3300,
    decode_translate_3308, decode_translate_3309, decode_translate_3400, decode_translate_3408,
    decode_vector3_3409,
};

#[cfg(test)]
mod tests {
    use super::common::kernel;
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
            common::decode_residual_component(&residual, 0, base_scale, local_idx, block_len)
                .unwrap();

        assert_eq!(next, 16);
        assert!(value.abs() > 0.0);
        assert!(value.abs() < 1.0);
    }
}
