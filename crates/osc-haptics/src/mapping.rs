//! Pure mapping logic — OSC value → rumble intensity. No I/O.

use crate::config::{HapticMode, HapticRule};
use rosc::OscType;

/// A resolved rumble command for one device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HapticAction {
    pub device_mac: [u8; 6],
    /// Target intensity in `0.0..=1.0`.
    pub intensity: f32,
    /// When set, the intensity should be held for this long then forced to 0.
    pub pulse_ms: Option<u32>,
}

fn clamp01(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

/// Extract a numeric value from an OSC argument.
///
/// VRChat sends avatar parameters as `Bool`, `Int`, or `Float`. Bools map to
/// 0.0/1.0; ints and floats pass through. Anything else yields `None`.
pub fn osc_value_to_f32(arg: &OscType) -> Option<f32> {
    match arg {
        OscType::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        OscType::Int(i) => Some(*i as f32),
        OscType::Float(f) => Some(*f),
        OscType::Double(d) => Some(*d as f32),
        _ => None,
    }
}

/// Resolve every rule that matches `address` into a rumble command.
///
/// `value` is the numeric OSC value already extracted via [`osc_value_to_f32`].
pub fn resolve(rules: &[HapticRule], address: &str, value: f32) -> Vec<HapticAction> {
    rules
        .iter()
        .filter(|r| r.osc_address == address)
        .map(|r| apply_mode(r.device_mac, r.mode, value))
        .collect()
}

fn apply_mode(device_mac: [u8; 6], mode: HapticMode, value: f32) -> HapticAction {
    match mode {
        HapticMode::Proximity {
            gain,
            min_threshold,
        } => {
            let intensity = if value < min_threshold {
                0.0
            } else {
                clamp01(value * gain)
            };
            HapticAction {
                device_mac,
                intensity,
                pulse_ms: None,
            }
        }
        HapticMode::Pulse { pulse_ms } => {
            if value > 0.5 {
                HapticAction {
                    device_mac,
                    intensity: 1.0,
                    pulse_ms: Some(pulse_ms),
                }
            } else {
                HapticAction {
                    device_mac,
                    intensity: 0.0,
                    pulse_ms: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC_A: [u8; 6] = [0x02, 0, 0, 0, 0, 0x0A];
    const MAC_B: [u8; 6] = [0x02, 0, 0, 0, 0, 0x0B];

    fn proximity_rule(addr: &str, mac: [u8; 6]) -> HapticRule {
        HapticRule {
            osc_address: addr.into(),
            device_mac: mac,
            mode: HapticMode::Proximity {
                gain: 1.0,
                min_threshold: 0.05,
            },
        }
    }

    #[test]
    fn osc_bool_int_float_extract() {
        assert_eq!(osc_value_to_f32(&OscType::Bool(true)), Some(1.0));
        assert_eq!(osc_value_to_f32(&OscType::Bool(false)), Some(0.0));
        assert_eq!(osc_value_to_f32(&OscType::Int(1)), Some(1.0));
        assert_eq!(osc_value_to_f32(&OscType::Float(0.42)), Some(0.42));
        assert_eq!(osc_value_to_f32(&OscType::String("x".into())), None);
    }

    #[test]
    fn resolve_ignores_non_matching_address() {
        let rules = [proximity_rule("/avatar/parameters/Touch", MAC_A)];
        assert!(resolve(&rules, "/avatar/parameters/Other", 1.0).is_empty());
    }

    #[test]
    fn proximity_passes_value_through_with_gain() {
        let rules = [HapticRule {
            osc_address: "/p".into(),
            device_mac: MAC_A,
            mode: HapticMode::Proximity {
                gain: 2.0,
                min_threshold: 0.05,
            },
        }];
        let out = resolve(&rules, "/p", 0.3);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].intensity, 0.6);
        assert_eq!(out[0].pulse_ms, None);
    }

    #[test]
    fn proximity_clamps_and_gates_below_threshold() {
        let rules = [proximity_rule("/p", MAC_A)];
        // Below min_threshold → silenced.
        assert_eq!(resolve(&rules, "/p", 0.01)[0].intensity, 0.0);
        // Above 1.0 after gain → clamped.
        let hot = [HapticRule {
            osc_address: "/p".into(),
            device_mac: MAC_A,
            mode: HapticMode::Proximity {
                gain: 5.0,
                min_threshold: 0.05,
            },
        }];
        assert_eq!(resolve(&hot, "/p", 0.9)[0].intensity, 1.0);
    }

    #[test]
    fn pulse_fires_on_rising_value_only() {
        let rules = [HapticRule {
            osc_address: "/t".into(),
            device_mac: MAC_A,
            mode: HapticMode::Pulse { pulse_ms: 150 },
        }];
        let on = resolve(&rules, "/t", 1.0);
        assert_eq!(on[0].intensity, 1.0);
        assert_eq!(on[0].pulse_ms, Some(150));
        let off = resolve(&rules, "/t", 0.0);
        assert_eq!(off[0].intensity, 0.0);
        assert_eq!(off[0].pulse_ms, None);
    }

    #[test]
    fn one_address_can_drive_multiple_devices() {
        let rules = [
            proximity_rule("/shared", MAC_A),
            proximity_rule("/shared", MAC_B),
        ];
        let out = resolve(&rules, "/shared", 0.5);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].device_mac, MAC_A);
        assert_eq!(out[1].device_mac, MAC_B);
    }
}
