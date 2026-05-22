//! Haptic bridge configuration — the address→device mapping.

use serde::{Deserialize, Serialize};

/// VRChat's default outgoing OSC port. The bridge binds here to receive
/// avatar parameter updates.
pub const DEFAULT_OSC_PORT: u16 = 9001;

/// How a single OSC parameter drives a device's rumble motor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HapticMode {
    /// Use a proximity float (0..1) directly as rumble intensity.
    ///
    /// `gain` scales the raw value; `min_threshold` suppresses values below
    /// it (sends 0.0) to ignore contact-receiver noise near the edge.
    Proximity { gain: f32, min_threshold: f32 },
    /// Treat the parameter as a trigger: when it rises past 0.5 fire a
    /// fixed-length pulse at full intensity, then auto-off.
    Pulse { pulse_ms: u32 },
}

impl Default for HapticMode {
    fn default() -> Self {
        HapticMode::Proximity {
            gain: 1.0,
            min_threshold: 0.05,
        }
    }
}

/// One OSC-address → device binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HapticRule {
    /// Full OSC address, e.g. `/avatar/parameters/bHaptics_Vest_Front`.
    pub osc_address: String,
    /// Target device MAC (the same identity SlimeVR uses).
    pub device_mac: [u8; 6],
    pub mode: HapticMode,
}

/// Full haptic bridge configuration, persisted as JSON in the settings store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HapticConfig {
    pub enabled: bool,
    pub listen_port: u16,
    pub rules: Vec<HapticRule>,
}

impl Default for HapticConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            listen_port: DEFAULT_OSC_PORT,
            rules: Vec::new(),
        }
    }
}
