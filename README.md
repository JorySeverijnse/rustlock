# 🔒 RustLock

[![License](https://img.shields.io/badge/license-AGPL--3.0%2B-blue.svg)](https://github.com/yourusername/rustlock/blob/main/LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](https://github.com/yourusername/rustlock/releases)

A high-performance Wayland screen locker written in Rust, inspired by `swaylock-effects`.

---

## ✨ Features

- ⚡ **Performance**: Written in safe Rust, optimized binary size (~2.4MB without networking, ~4MB with)
- 🎨 **Visual Effects**:
  - Gaussian blur (configurable radius and passes)
  - Vignette effect (configurable base and factor)
  - Pixelate, Swirl, and Melting effects
  - Smooth fade-in animation
- 🔐 **Password Indicator**:
  - Circular ring with configurable radius and thickness
  - Dynamic key highlight segments that rotate with each keystroke
  - Caps lock indicator
- 🕐 **Information Display**:
  - Centered clock (HH:MM format)
  - Full date
  - System uptime
- 📻 **Media & System Status** (optional):
  - MPRIS media player integration with album art
  - Battery percentage and charging status
  - WiFi SSID and signal strength
  - Bluetooth connected devices
  - Keyboard layout indicator
- 🔑 **Session Management**:
  - F1: Suspend
  - F2: Reboot  
  - F3: Power Off
- 📸 **Screenshot Support**:
  - Captures desktop background before locking
  - Custom background image support
- 🔐 **Authentication**:
  - PAM-based authentication
  - Configurable grace period (any key press within N seconds unlocks without password)
- 🎯 **Customization**:
  - Custom icons for WiFi, Bluetooth, Battery
  - Theme presets (dark, light, nord, dracula)
  - Configuration via config file or CLI

---

## 🚀 Usage

### Basic Example

```bash
rustlock --screenshots --effect-blur 7x5 --effect-vignette 0.5:0.5
```

### Full Configuration

```bash
rustlock \
    --screenshots \
    --clock \
    --indicator \
    --indicator-radius 100 \
    --indicator-thickness 7 \
    --effect-blur 7x5 \
    --effect-vignette 0.5:0.5 \
    --ring-color 785412 \
    --key-hl-color 4EAC41 \
    --line-color 00000000 \
    --inside-color 00000088 \
    --separator-color 00000000 \
    --grace 2 \
    --fade-in 0.2
```

### Session Controls

When locked, use function keys to control the system:
- **F1**: Suspend to RAM
- **F2**: Reboot
- **F3**: Power Off

---

## ⚙️ Configuration

Options can be provided via command line or a configuration file at `~/.config/rustlock/config.toml`. CLI arguments take precedence over config file, which takes precedence over theme defaults.

### Options

| Option | Description |
|--------|-------------|
| **General** | |
| `--screenshots` | Capture desktop background before locking |
| `--image <PATH>` | Use custom background image instead of screenshot |
| `--clock` | Display centered clock and date |
| `--indicator` | Show password entry ring (default: true) |
| `--indicator-radius <N>` | Ring radius in pixels (default: 100) |
| `--indicator-thickness <N>` | Ring thickness in pixels (default: 7) |
| **Effects** | |
| `--effect-blur <R>x<P>` | Gaussian blur: radius x passes (e.g., `7x5`) |
| `--effect-pixelate` | Pixelate effect |
| `--effect-swirl` | Swirl distortion effect |
| `--effect-melting` | Melting distortion effect |
| `--effect-vignette <B>:<F>` | Vignette: base:factor (e.g., `0.5:0.5`) |
| **Colors** | |
| `--ring-color <RRGGBB[AA]>` | Outer ring color (hex, optional alpha) |
| `--key-hl-color <RRGGBB[AA]>` | Key highlight segment color |
| `--line-color <RRGGBB[AA]>` | Separator line color |
| `--inside-color <RRGGBB[AA]>` | Inner circle color |
| `--separator-color <RRGGBB[AA]>` | Ring separator color |
| **Display Options** | |
| `--show-media` | Show MPRIS media info (default: true) |
| `--show-battery` | Show battery status (default: true) |
| `--show-network` | Show WiFi status (default: true) |
| `--show-bluetooth` | Show Bluetooth status (default: true) |
| `--show-keyboard-layout` | Show keyboard layout indicator (default: true) |
| `--show-album-art` | Show album art (default: true) |
| **Custom Icons** | |
| `--wifi-icon <PATH>` | Custom WiFi icon (PNG/SVG) |
| `--bluetooth-icon <PATH>` | Custom Bluetooth icon (PNG/SVG) |
| `--battery-icon <PATH>` | Custom battery icon (PNG/SVG) |
| **Other** | |
| `--grace <SECONDS>` | Grace period in seconds (default: 2) |
| `--fade-in <SECONDS>` | Fade-in animation duration (default: 0.2) |
| `--pam-service <NAME>` | PAM service name (default: "rustlock") |
| `--config <PATH>` | Path to config file |
| `--theme <NAME>` | Theme preset: dark, light, nord, dracula |
| `--debug` | Enable debug logging |
| `--log-file` | Write logs to `~/.rustlock.log` |

---

## 📦 Installation

### Using Nix (Recommended)

```bash
nix-shell -p rustlock
```

Or with flakes:
```bash
nix run github:yourusername/rustlock
```

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/rustlock`.

### Build Options

- **With networking** (default): Includes reqwest for album art fetching
  ```bash
  cargo build --release --features networking
  ```

- **Without networking**: Smaller binary (~2.4MB)
  ```bash
  cargo build --release --no-default-features
  ```

---

## ✅ Completed

- [x] PAM-based authentication
- [x] Grace period (any key unlocks within N seconds)
- [x] Screenshot capture with blur/vignette/pixelate/swirl/melting effects
- [x] Configuration file support (`~/.config/rustlock/config.toml`) with schema validation
- [x] Debug logging to `~/.rustlock.log`
- [x] Clock and date display
- [x] Password indicator ring with rotating highlights
- [x] Dynamic screen resolution detection
- [x] Full multi-monitor support with different resolutions
- [x] Theme/profile support with presets (dark, light, nord, dracula)
- [x] Wayland protocol stability fixes
- [x] Media control integration (MPRIS support with Album Art)
- [x] Battery, WiFi, and Bluetooth status indicators
- [x] Custom background image support
- [x] Custom icons for status indicators
- [x] Keyboard layout indicator
- [x] Session management (F1-F3 keys)
- [x] Caps lock indicator

---

## 📄 License

GPL v3
