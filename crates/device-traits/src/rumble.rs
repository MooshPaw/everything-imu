//! Rumble intensity helpers shared by every driver.
//!
//! The haptic path carries an `f32` intensity in `0.0..=1.0`. Each driver
//! converts that to whatever its hardware expects: a 0-255 motor byte
//! (DualSense, PS Move), a 1-bit motor (Wiimote), or an on/off enable
//! (Joy-Con). These helpers keep that conversion consistent and clamped.

/// Clamp an intensity into the valid `0.0..=1.0` range. NaN maps to `0.0`.
pub fn clamp01(intensity: f32) -> f32 {
    if intensity.is_nan() {
        0.0
    } else {
        intensity.clamp(0.0, 1.0)
    }
}

/// Scale a `0.0..=1.0` intensity to a `0..=255` motor amplitude byte.
pub fn to_u8(intensity: f32) -> u8 {
    (clamp01(intensity) * 255.0).round() as u8
}

/// Whether a driver with only an on/off motor should engage.
///
/// `threshold` is the minimum intensity that counts as "on" — drivers with
/// a single-bit motor (Wiimote) pass `0.5`; enable-only drivers pass `0.0`
/// so any positive intensity engages.
pub fn is_on(intensity: f32, threshold: f32) -> bool {
    clamp01(intensity) > threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp01_bounds_and_nan() {
        assert_eq!(clamp01(-1.0), 0.0);
        assert_eq!(clamp01(0.0), 0.0);
        assert_eq!(clamp01(0.5), 0.5);
        assert_eq!(clamp01(1.0), 1.0);
        assert_eq!(clamp01(2.5), 1.0);
        assert_eq!(clamp01(f32::NAN), 0.0);
    }

    #[test]
    fn to_u8_scales_full_range() {
        assert_eq!(to_u8(0.0), 0);
        assert_eq!(to_u8(1.0), 255);
        assert_eq!(to_u8(0.5), 128);
        assert_eq!(to_u8(-3.0), 0);
        assert_eq!(to_u8(9.0), 255);
    }

    #[test]
    fn is_on_respects_threshold() {
        // Enable-only drivers: any positive intensity engages.
        assert!(!is_on(0.0, 0.0));
        assert!(is_on(0.01, 0.0));
        // Single-bit motor: must clear the halfway mark.
        assert!(!is_on(0.4, 0.5));
        assert!(is_on(0.6, 0.5));
    }
}
