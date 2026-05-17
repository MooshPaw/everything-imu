<div align="center">
  <h1>everything-imu</h1>
  <p><strong>Cross-platform SlimeVR IMU bridge for game controllers</strong></p>

  <p>
    <a href="https://github.com/matiaspalmac/everything-imu/actions/workflows/ci.yml"><img src="https://github.com/matiaspalmac/everything-imu/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
    <img src="https://img.shields.io/badge/status-alpha-orange.svg" alt="Status: alpha">
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey.svg" alt="Platform">
  </p>
</div>

`everything-imu` is a native bridge that turns Nintendo and Sony game controllers
into full-body trackers for **SlimeVR-Server**. Plug a controller into your PC,
strap it on, and it shows up in SlimeVR as a regular tracker — no extra hardware,
no firmware flashing.

The bridge reads the controller's built-in IMU at its native sample rate, runs a
sensor fusion filter (VQF or Madgwick) in Rust, and forwards the resulting
quaternion to SlimeVR-Server over UDP using the official protocol.

> **Heads-up:** alpha. The protocol and pipeline are stable; the UI, settings
> persistence, and packaging are still in flux. Expect rough edges and please
> file issues with reproduction steps + the logs from the **Logs** tab.

## Why this exists

SlimeVR-Server already supports full-body tracking with dedicated SlimeVR
hardware. This project plugs the *bring-your-own-IMU* gap: most homes have
controllers with high-quality IMUs sitting in a drawer, and they make great
secondary trackers (waist, knees, feet) when you're trying SlimeVR for the
first time and don't want to commit to buying nodes yet.

We **deliberately do not reimplement SlimeVR-Server features** (skeletal model,
SteamVR driver, calibration UI, etc.). everything-imu is the *bridge layer*
only.

## Supported devices

| Device | Transport | Native rate | Mag | Status |
|--------|-----------|------------:|:---:|--------|
| Joy-Con (L/R) | USB · BT Classic | 200 Hz | ✗ | hardware-validated |
| Switch Pro Controller | USB · BT Classic | 200 Hz | ✗ | hardware-validated |
| Joy-Con 2 / Pro 2 / NSO GC2 | BLE only | 62 Hz | ✓ | hardware-validated |
| DualSense (PS5) | USB · BT | 250 Hz | ✗ | hardware-validated |
| DualSense Edge | USB · BT | 250 Hz | ✗ | needs hardware |
| DualShock 4 (PS4) | USB | 250 Hz | ✗ | hardware-validated |
| PS Move ZCM1 | USB · BT | 175 Hz | ✓ | hardware-validated |
| PS Move ZCM2 | USB · BT | 175 Hz | ✗ | hardware-validated |
| Wii Remote | TCP forwarder (`127.0.0.1:9909`) | 100 Hz | ✗ | hardware-validated |

Deep-dive on each: [DEVICES.md](DEVICES.md).

## Quickstart

1. **Install SlimeVR-Server** and leave it running. The default UDP port is `6969`.
2. **Run everything-imu**. The status bar at the bottom will read `Live` once a
   handshake with the server completes.
3. **Plug or pair a supported controller**. It appears in the *Devices* tab; a
   tracker for it shows up automatically in SlimeVR-Server.
4. **Mount the controller** to your body (waist, knee, ankle, etc.) and assign it
   in SlimeVR-Server's *Body Proportions* flow.
5. **Reset orientation** with the in-app button or via the global hotkey
   `R` (yaw) / `Shift+R` (full). Some devices accept on-device gestures — see
   the per-device sections in [DEVICES.md](DEVICES.md).

### Global hotkeys

| Shortcut | Action |
|----------|--------|
| `Ctrl+K` | Command palette |
| `Ctrl+F` | Global search |
| `Ctrl+Enter` | Cinema mode (immersive overlay) |
| `Ctrl+Shift+B` | Kill-switch the bridge (pause emission) |
| `R` / `Shift+R` | Broadcast yaw / full reset |

## Building from source

### Prerequisites

- **Rust** stable (1.79+ recommended)
- **Node.js** 22+ and **pnpm** 10
- **Windows**: WebView2 (preinstalled on Windows 11)
- **Linux**: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`,
  `librsvg2-dev`, `libayatana-appindicator3-dev`, `libxdo-dev`, `libssl-dev`,
  `libudev-dev`, `patchelf`

### Commands

```bash
git clone https://github.com/matiaspalmac/everything-imu.git
cd everything-imu

pnpm install                  # JS deps
pnpm tauri dev                # dev: live-reload UI + Rust backend
pnpm tauri build              # release bundle (MSI on Windows, AppImage on Linux)

# Backend-only iteration:
cargo test --workspace        # unit + integration tests
cargo run -p headless-cli     # headless bridge for debugging
```

The `everything-imu-app` binary is the full Tauri shell; `headless-cli` is a
no-UI driver useful for tracing protocol-level issues.

## Documentation

| File | Purpose |
|------|---------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Crate graph + responsibilities |
| [DEVICES.md](DEVICES.md) | Per-device IMU, transport, calibration |
| [docs/PLAN.md](docs/PLAN.md) | Sprint plan + delivery state |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Long-term direction |
| [docs/RISKS.md](docs/RISKS.md) | Known hazards |
| [docs/DECISIONS.md](docs/DECISIONS.md) | ADR-style design decisions |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Dev workflow, style, PR rules |

## Stack

- **Core**: Rust, `tokio`, `hidapi`, `btleplug`, `nalgebra`
- **Math & fusion**: VQF (Laidig 2023), Madgwick, BasicVQF
- **Persistence**: SQLite via `rusqlite`
- **Desktop shell**: Tauri 2 + tauri-specta (typed IPC)
- **Frontend**: React 19, TypeScript, Vite, TailwindCSS 4, Zustand 5,
  react-three-fiber

## License

[MIT](LICENSE-MIT). Contributions are accepted under the same license.
