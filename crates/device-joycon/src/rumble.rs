//! Joy-Con 1 HD Rumble frame encoder.
//!
//! Joy-Con 1 vibration is driven by an 8-byte packed frame (4 bytes per
//! side) in output report 0x10 / 0x01. Each 4-byte side encodes a
//! high-band and a low-band frequency + amplitude. Sending the documented
//! neutral frame produces no buzz — to actually vibrate, the amplitude
//! must be encoded with the non-linear curve Nintendo's firmware expects.
//!
//! Encoder ported from the well-validated `joycon-python` implementation
//! (itself derived from dekuNukem's reverse-engineering notes). We drive a
//! fixed pleasant buzz frequency and vary only amplitude with intensity.

/// Documented silent frame — produces no vibration.
pub const NEUTRAL_RUMBLE_FRAME: [u8; 4] = [0x00, 0x01, 0x40, 0x40];

/// Fixed low-band buzz frequency (Hz). Useful range is 40–626 for the
/// low band; ~160 Hz is a firm, controller-typical rumble.
const LOW_FREQ_HZ: f64 = 160.0;
/// Fixed high-band buzz frequency (Hz).
const HIGH_FREQ_HZ: f64 = 320.0;

/// Encoded amplitude is capped here. The hardware thermal-throttles above
/// ~0.8 intensity; 0xC8 corresponds to full encoded amplitude and also
/// keeps `data[1]` within a byte.
const MAX_HF_AMP: i32 = 0xC8;

/// Encode one side's 4-byte HD Rumble frame for `intensity` in `0.0..=1.0`.
///
/// `0.0` returns [`NEUTRAL_RUMBLE_FRAME`] (silent). Both Joy-Con sides take
/// the same frame for a single-motor buzz.
pub fn encode_rumble_frame(intensity: f32) -> [u8; 4] {
    let a = intensity.clamp(0.0, 1.0) as f64;
    if a == 0.0 {
        return NEUTRAL_RUMBLE_FRAME;
    }

    // Frequency encoding (constant for our fixed buzz).
    let hf = ((32.0 * (HIGH_FREQ_HZ * 0.1).log2()).round() - 0x60 as f64) * 4.0;
    let hf = hf as i32;
    let lf = ((32.0 * (LOW_FREQ_HZ * 0.1).log2()).round() - 0x40 as f64) as i32;

    // Non-linear amplitude curve — three ranges, matching firmware.
    let scaled = (a * 1000.0).log2() * 32.0 - 96.0;
    let hf_amp_f = if a < 0.117 {
        scaled / (5.0 - a * a) - 1.0
    } else if a < 0.23 {
        scaled - 92.0 // 0x5c
    } else {
        scaled * 2.0 - 246.0 // 0xf6
    };
    let hf_amp = (hf_amp_f as i32).clamp(0, MAX_HF_AMP);

    // Low-band amplitude is derived from the high-band value; an odd value
    // sets a parity bit in the 16-bit lf_amp word.
    let mut lf_amp = (hf_amp as f64 * 0.5) as i32;
    let parity = lf_amp % 2;
    if parity != 0 {
        lf_amp -= 1;
    }
    lf_amp >>= 1;
    lf_amp += 0x40;
    let lf_amp: u32 = if parity != 0 {
        lf_amp as u32 | 0x8000
    } else {
        lf_amp as u32
    };

    [
        (hf & 0xFF) as u8,
        (((hf >> 8) & 0xFF) + hf_amp) as u8,
        ((lf & 0xFF) as u32 + ((lf_amp >> 8) & 0xFF)) as u8,
        (lf_amp & 0xFF) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_intensity_is_neutral() {
        assert_eq!(encode_rumble_frame(0.0), NEUTRAL_RUMBLE_FRAME);
    }

    #[test]
    fn negative_intensity_clamps_to_neutral() {
        assert_eq!(encode_rumble_frame(-1.0), NEUTRAL_RUMBLE_FRAME);
    }

    #[test]
    fn full_intensity_matches_reference_encoding() {
        // Computed from the joycon-python algorithm at 160/320 Hz.
        assert_eq!(encode_rumble_frame(1.0), [0x00, 0xC8, 0xC0, 0x71]);
    }

    #[test]
    fn half_intensity_matches_reference_encoding() {
        assert_eq!(encode_rumble_frame(0.5), [0x00, 0x88, 0xC0, 0x61]);
    }

    #[test]
    fn over_one_clamps_to_full() {
        assert_eq!(encode_rumble_frame(5.0), encode_rumble_frame(1.0));
    }

    #[test]
    fn higher_intensity_raises_encoded_amplitude() {
        // data[1] carries the high-band amplitude — must be monotonic.
        let low = encode_rumble_frame(0.3)[1];
        let mid = encode_rumble_frame(0.6)[1];
        let high = encode_rumble_frame(1.0)[1];
        assert!(low < mid, "{low} !< {mid}");
        assert!(mid < high, "{mid} !< {high}");
    }

    #[test]
    fn any_positive_intensity_is_not_neutral() {
        for pct in 1..=100 {
            let f = encode_rumble_frame(pct as f32 / 100.0);
            assert_ne!(
                f, NEUTRAL_RUMBLE_FRAME,
                "intensity {pct}% encoded as neutral"
            );
        }
    }
}
