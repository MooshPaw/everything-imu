//! Synthetic OSC dispatch helper — drives the same routing path the listener
//! runs at packet-arrival, but without a real socket.
//!
//! Used by the UI's "Test rule" button and by integration tests to confirm
//! a given (address, value) pair would actually match a rule and produce a
//! [`HapticAction`].
//!
//! Pure function — no I/O, no async. Safe to call from any thread.

use crate::config::{normalize_address, HapticConfig, HapticRule};
use crate::mapping::{resolve, HapticAction};

/// Outcome of a simulated dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct TestFireOutcome {
    /// Address that was actually used for matching (after normalisation).
    pub normalised_address: String,
    /// Actions every matching rule produced.
    pub actions: Vec<HapticAction>,
}

impl TestFireOutcome {
    pub fn matched(&self) -> bool {
        !self.actions.is_empty()
    }
}

/// Simulate one OSC packet with the given address + value against the
/// supplied rule set. The input `address` is normalised first so users can
/// pass either a bare param name or a full OSC path.
pub fn simulate_dispatch(rules: &[HapticRule], address: &str, value: f32) -> TestFireOutcome {
    let normalised = normalize_address(address);
    let actions = if normalised.is_empty() {
        Vec::new()
    } else {
        resolve(rules, &normalised, value)
    };
    TestFireOutcome {
        normalised_address: normalised,
        actions,
    }
}

/// Convenience wrapper that takes a full [`HapticConfig`] instead of a raw
/// slice — UI code holds the config struct so this matches its shape.
pub fn simulate_against_config(
    config: &HapticConfig,
    address: &str,
    value: f32,
) -> TestFireOutcome {
    simulate_dispatch(&config.rules, address, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HapticMode;

    fn rule(addr: &str, mac: [u8; 6]) -> HapticRule {
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
    fn bare_param_resolves_via_auto_prefix() {
        let rules = [rule("/avatar/parameters/Touch", [1; 6])];
        let outcome = simulate_dispatch(&rules, "Touch", 0.8);
        assert_eq!(outcome.normalised_address, "/avatar/parameters/Touch");
        assert!(outcome.matched(), "auto-prefix must let bare names match");
        assert_eq!(outcome.actions.len(), 1);
        assert!((outcome.actions[0].intensity - 0.8).abs() < 1e-6);
    }

    #[test]
    fn full_path_passthrough_matches() {
        let rules = [rule("/avatar/parameters/Touch", [1; 6])];
        let outcome = simulate_dispatch(&rules, "/avatar/parameters/Touch", 0.5);
        assert!(outcome.matched());
    }

    #[test]
    fn no_match_returns_empty_actions() {
        let rules = [rule("/avatar/parameters/Touch", [1; 6])];
        let outcome = simulate_dispatch(&rules, "Other", 1.0);
        assert!(!outcome.matched());
        assert_eq!(outcome.normalised_address, "/avatar/parameters/Other");
    }

    #[test]
    fn empty_address_returns_no_actions() {
        let rules = [rule("/avatar/parameters/Touch", [1; 6])];
        let outcome = simulate_dispatch(&rules, "", 1.0);
        assert!(outcome.normalised_address.is_empty());
        assert!(!outcome.matched());
    }

    #[test]
    fn simulate_against_config_wraps_rules() {
        let cfg = HapticConfig {
            enabled: true,
            listen_port: 9001,
            rules: vec![rule("/avatar/parameters/Touch", [1; 6])],
        };
        let outcome = simulate_against_config(&cfg, "Touch", 0.5);
        assert!(outcome.matched());
    }
}
