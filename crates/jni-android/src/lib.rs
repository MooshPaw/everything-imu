//! JNI bindings exposing the `imu-fusion` workspace crate to the Android `mobile/core` module.
//!
//! Sprint 0: cdylib scaffold only. Real `extern "system"` Java_* fns land in sprint 2.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

pub fn fusion_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_non_empty() {
        assert!(!fusion_version().is_empty());
    }
}
