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

/// Default VRChat avatar-parameter address prefix.
///
/// Users frequently type just the parameter name (`Haptics_Chest`) into the
/// UI; without normalisation the listener would never match VRChat's outgoing
/// `/avatar/parameters/Haptics_Chest`. [`normalize_address`] applies this
/// prefix when the input lacks any slash.
pub const VRCHAT_PARAM_PREFIX: &str = "/avatar/parameters/";

/// Normalise a user-entered OSC address.
///
/// - Empty / whitespace-only → empty string (caller may reject the rule).
/// - Bare parameter name without `/` → prefixed with [`VRCHAT_PARAM_PREFIX`].
/// - Starts with `/` → passed through verbatim (advanced users targeting
///   non-VRChat senders like Resonite or VirtualMotionCapture stay untouched).
pub fn normalize_address(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        return trimmed.to_string();
    }
    format!("{VRCHAT_PARAM_PREFIX}{trimmed}")
}

impl HapticRule {
    /// Construct a rule, applying [`normalize_address`] to the address.
    pub fn new(osc_address: impl Into<String>, device_mac: [u8; 6], mode: HapticMode) -> Self {
        Self {
            osc_address: normalize_address(&osc_address.into()),
            device_mac,
            mode,
        }
    }

    /// Re-normalise the address in place. Call after the user edits the
    /// rule via the UI; idempotent.
    pub fn normalize(&mut self) {
        let next = normalize_address(&self.osc_address);
        self.osc_address = next;
    }
}

impl HapticConfig {
    /// Normalise every rule's address in place. Use after loading config
    /// from disk or after a UI edit batch to guarantee the listener's
    /// runtime snapshot only ever holds fully-qualified addresses.
    pub fn normalize_rules(&mut self) {
        for r in &mut self.rules {
            r.normalize();
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_bare_param_gets_vrchat_prefix() {
        assert_eq!(
            normalize_address("Haptics_Chest"),
            "/avatar/parameters/Haptics_Chest"
        );
    }

    #[test]
    fn normalize_full_path_unchanged() {
        assert_eq!(
            normalize_address("/avatar/parameters/Touch"),
            "/avatar/parameters/Touch"
        );
        // Non-VRChat senders (Resonite, VMC) target their own roots — keep
        // verbatim when the user already wrote a slash.
        assert_eq!(
            normalize_address("/tracking/eye/CenterPitchYaw"),
            "/tracking/eye/CenterPitchYaw"
        );
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(normalize_address("  Foo  "), "/avatar/parameters/Foo");
    }

    #[test]
    fn normalize_empty_yields_empty() {
        assert_eq!(normalize_address(""), "");
        assert_eq!(normalize_address("   "), "");
    }

    #[test]
    fn rule_new_normalises() {
        let r = HapticRule::new("Foo", [0; 6], HapticMode::default());
        assert_eq!(r.osc_address, "/avatar/parameters/Foo");
    }

    #[test]
    fn normalize_rules_is_idempotent() {
        let mut cfg = HapticConfig {
            enabled: true,
            listen_port: 9001,
            rules: vec![
                HapticRule {
                    osc_address: "Bar".into(),
                    device_mac: [0; 6],
                    mode: HapticMode::default(),
                },
                HapticRule {
                    osc_address: "/avatar/parameters/Already".into(),
                    device_mac: [0; 6],
                    mode: HapticMode::default(),
                },
            ],
        };
        cfg.normalize_rules();
        assert_eq!(cfg.rules[0].osc_address, "/avatar/parameters/Bar");
        assert_eq!(cfg.rules[1].osc_address, "/avatar/parameters/Already");
        // Run twice; result stable.
        cfg.normalize_rules();
        assert_eq!(cfg.rules[0].osc_address, "/avatar/parameters/Bar");
        assert_eq!(cfg.rules[1].osc_address, "/avatar/parameters/Already");
    }
}
