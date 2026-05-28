//! Steam Deck integrated controller IMU bridge.
//!
//! Hardware: BMI260 IMU (accel + gyro, no magnetometer) on a Valve Jupiter
//! (LCD) / Galileo (OLED) chassis. Exposed via the same Valve HID interface as
//! the original Steam Controller — VID `0x28DE`, PID `0x1205`.
//!
//! ## Lizard mode
//! By default the in-kernel `hid-steam` driver puts the Deck's gamepad surface
//! into "lizard mode" — buttons emulate a keyboard, the right trackpad
//! emulates a mouse. To get raw HID reports with IMU data we must:
//! 1. send `ID_CLEAR_DIGITAL_MAPPINGS`
//! 2. send `ID_LOAD_DEFAULT_SETTINGS`
//! 3. send `ID_SET_DIGITAL_MAPPINGS` (empty payload)
//! 4. repeat every <800 ms or the kernel re-enables lizard
//!
//! ## Scaling
//! Per Valve firmware: gyro full-scale ±2000 dps, accel full-scale ±2 g, both
//! reported as `int16`. Sample rate ~250 Hz (4 ms interval).

pub mod factory;
pub mod ids;
pub mod lizard;
pub mod report;
pub mod scale;

#[cfg(feature = "synthetic-source")]
pub mod synthetic;

pub mod device;

pub use device::SteamDeckDevice;
pub use device_traits::{Device, DeviceFactory};
pub use factory::SteamDeckFactory;
pub use ids::{STEAM_DECK_PID, VALVE_VID};
