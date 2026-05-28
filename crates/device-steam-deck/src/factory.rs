//! `SteamDeckFactory` — `DeviceFactory` impl. Real path uses hidapi; synthetic
//! path emits a deterministic sample stream.

use crate::device::SteamDeckDevice;
use crate::ids::is_gamepad_interface;
use device_traits::{Device, DeviceError, DeviceFactory, DeviceMetadata};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct SteamDeckFactory {
    mode: FactoryMode,
}

enum FactoryMode {
    Real,
    #[cfg(feature = "synthetic-source")]
    Synthetic {
        count: u8,
    },
}

impl SteamDeckFactory {
    pub fn new() -> Self {
        Self {
            mode: FactoryMode::Real,
        }
    }

    pub fn real() -> Self {
        Self::new()
    }

    #[cfg(feature = "synthetic-source")]
    pub fn synthetic(count: u8) -> Self {
        Self {
            mode: FactoryMode::Synthetic { count },
        }
    }
}

impl Default for SteamDeckFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DeviceFactory for SteamDeckFactory {
    async fn enumerate_loop(
        &self,
        out: mpsc::Sender<(DeviceMetadata, Box<dyn Device>)>,
    ) -> Result<(), DeviceError> {
        match &self.mode {
            FactoryMode::Real => real_enumerate_loop(out).await,
            #[cfg(feature = "synthetic-source")]
            FactoryMode::Synthetic { count } => synthetic_enumerate_loop(*count, out).await,
        }
    }
}

fn hid_api_singleton() -> Result<&'static Mutex<hidapi::HidApi>, hidapi::HidError> {
    static API: OnceLock<Mutex<hidapi::HidApi>> = OnceLock::new();
    if let Some(api) = API.get() {
        return Ok(api);
    }
    let api = hidapi::HidApi::new()?;
    Ok(API.get_or_init(|| Mutex::new(api)))
}

async fn real_enumerate_loop(
    out: mpsc::Sender<(DeviceMetadata, Box<dyn Device>)>,
) -> Result<(), DeviceError> {
    let api = hid_api_singleton().map_err(|e| DeviceError::Hid(e.to_string()))?;
    let mut known: HashSet<String> = HashSet::new();

    loop {
        {
            let mut guard = api.lock().unwrap();
            guard
                .refresh_devices()
                .map_err(|e| DeviceError::Hid(e.to_string()))?;
        }
        let infos: Vec<_> = {
            let guard = api.lock().unwrap();
            guard
                .device_list()
                .filter(|i| {
                    is_gamepad_interface(i.vendor_id(), i.product_id(), i.usage_page(), i.usage())
                })
                .map(|i| {
                    (
                        i.path().to_owned(),
                        i.serial_number().unwrap_or("").to_string(),
                        i.interface_number(),
                    )
                })
                .collect()
        };

        let present: HashSet<String> = infos
            .iter()
            .map(|(path, _, iface)| format!("{path:?}#{iface}"))
            .collect();
        known.retain(|k| present.contains(k));

        for (path, serial, iface) in infos {
            let key = format!("{path:?}#{iface}");
            if !known.insert(key.clone()) {
                continue;
            }
            let device = {
                let guard = api.lock().unwrap();
                match guard.open_path(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::warn!(?path, error = %e, "failed to open steam deck hid");
                        known.remove(&key);
                        continue;
                    }
                }
            };
            let mac = mac_from_serial(&serial);
            let dev = SteamDeckDevice::new(device, serial, mac);
            let meta = dev.metadata().clone();
            if out
                .send((meta, Box::new(dev) as Box<dyn Device>))
                .await
                .is_err()
            {
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}

#[cfg(feature = "synthetic-source")]
async fn synthetic_enumerate_loop(
    count: u8,
    out: mpsc::Sender<(DeviceMetadata, Box<dyn Device>)>,
) -> Result<(), DeviceError> {
    for i in 0..count {
        let dev = crate::synthetic::SyntheticSteamDeck::new(i as u64);
        let meta = dev.metadata().clone();
        if out
            .send((meta, Box::new(dev) as Box<dyn Device>))
            .await
            .is_err()
        {
            return Ok(());
        }
    }
    std::future::pending().await
}

#[derive(Debug, Clone)]
pub struct PairedSteamDeck {
    pub serial: String,
    pub path: String,
    pub mac: [u8; 6],
}

impl SteamDeckFactory {
    pub fn list_paired() -> Result<Vec<PairedSteamDeck>, DeviceError> {
        let api = hid_api_singleton().map_err(|e| DeviceError::Hid(e.to_string()))?;
        let mut guard = api.lock().unwrap();
        guard
            .refresh_devices()
            .map_err(|e| DeviceError::Hid(e.to_string()))?;
        let mut out = Vec::new();
        for i in guard.device_list() {
            if !is_gamepad_interface(i.vendor_id(), i.product_id(), i.usage_page(), i.usage()) {
                continue;
            }
            let serial = i.serial_number().unwrap_or("").to_string();
            let mac = mac_from_serial(&serial);
            out.push(PairedSteamDeck {
                serial,
                path: format!("{:?}", i.path()),
                mac,
            });
        }
        Ok(out)
    }
}

fn mac_from_serial(serial: &str) -> [u8; 6] {
    let h = fnv1a_64(serial.as_bytes()).to_le_bytes();
    [0x02, 0x28, 0xDE, h[0], h[1], h[2]]
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001b3;

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_is_deterministic() {
        assert_eq!(mac_from_serial("DECK01"), mac_from_serial("DECK01"));
    }

    #[test]
    fn mac_is_locally_administered() {
        let m = mac_from_serial("any");
        assert_eq!(m[0] & 0x02, 0x02);
    }

    #[test]
    fn mac_uses_valve_oui_marker() {
        // bytes [1..3] = 0x28 0xDE so traces are easily tagged as Valve devices.
        let m = mac_from_serial("DECK");
        assert_eq!(m[1], 0x28);
        assert_eq!(m[2], 0xDE);
    }

    #[test]
    fn distinct_serials_yield_distinct_macs() {
        assert_ne!(mac_from_serial("A"), mac_from_serial("B"));
    }

    #[test]
    fn ids_constants_in_scope() {
        use crate::ids::{STEAM_DECK_PID, VALVE_VID};
        assert_eq!(STEAM_DECK_PID, 0x1205);
        assert_eq!(VALVE_VID, 0x28DE);
        assert!(is_gamepad_interface(VALVE_VID, STEAM_DECK_PID, 0, 0));
    }
}
