# 🔒 RustLock

[![License](https://img.shields.io/badge/license-GPL-3.0-blue.svg)](https://github.com/yourusername/rustlock/blob/main/LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](https://github.com/yourusername/rustlock/releases)

A high-performance Wayland screen locker written in Rust, inspired by `swaylock-effects`.

<p align="center">
  <img src="assets/rustlock-effects.webp" alt="RustLock effects demo" width="800">
</p>

---

## ✨ Features

- ⚡ **Performance**: Written in safe Rust, optimized binary size (~2.4MB without networking, ~4MB with)
- 🎨 **Visual Effects**:
  - Gaussian blur (configurable radius and passes)
  - Vignette effect (configurable base and factor)
  - Pixelate and Swirl effects
  - Smooth fade-in animation
- 🔐 **Password Indicator**:
  - Circular ring with configurable radius and thickness
  - Dynamic key highlight segments that rotate with each keystroke
  - Full password editing with cursor navigation (arrow keys, Home/End)
  - Visual cursor indicator between dots
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
  - Multiple ring shapes: circle, square, diamond, hexagon, pill
  - Custom icons for WiFi, Bluetooth, Battery
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
    --ring-shape hexagon \
    --effect-blur 7x5 \
    --effect-vignette 0.5:0.5 \
    --ring-color 785412 \
    --key-hl-color 4EAC41 \
    --line-color 00000000 \
    --inside-color 00000088 \
    --separator-color 00000000 \
    --show-network \
    --show-battery \
    --grace 2 \
    --fade-in 0.2
```

### Session Controls

When locked, use function keys to control the system:
- **F1**: Suspend to RAM
- **F2**: Reboot
- **F3**: Power Off

### Password Entry

Use arrow keys to move the cursor while entering your password:
- **Left/Right arrows**: Move cursor one position
- **Home**: Move to start
- **End**: Move to end
- **Delete**: Delete character at cursor
- **Ctrl+U**: Clear entire password

---

## ⚙️ Configuration

Options can be provided via command line or a configuration file at `~/.config/rustlock/config.toml`. CLI arguments take precedence over config file values.

### Options

| Option | Default | Description |
|--------|---------|-------------|
| **General** | | |
| `--screenshots` | — | Capture desktop background before locking |
| `--image <PATH>` | — | Use custom background image instead of screenshot |
| `--clock` | — | Display centered clock and date |
| `--indicator` | `true` | Show password entry ring |
| `--hide-password` | `false` | Hide password dots (dots are shown by default) |
| `--config <PATH>` | — | Path to config file |
| `--debug` | — | Enable debug logging |
| **Ring** | | |
| `--indicator-radius <N>` | `100` | Ring radius in pixels |
| `--indicator-thickness <N>` | `7` | Ring thickness in pixels |
| `--ring-shape <SHAPE>` | `circle` | Ring shape: `circle`, `square`, `diamond`, `hexagon`, `pill` |
| `--max-dots <N>` | `24` | Maximum password dots in the ring |
| **Effects** | | |
| `--effect-blur <R>x<P>` | — | Gaussian blur: radius x passes (e.g., `7x5`) |
| `--effect-vignette <B>:<F>` | — | Vignette: base : factor (e.g., `0.5:0.5`) |
| `--effect-pixelate <S>` | — | Pixelate effect with block size in pixels |
| `--effect-swirl <A>` | — | Swirl distortion with angle |
| **Colors** (hex `RRGGBB[AA]`) | | |
| `--ring-color <HEX>` | `#785412` | Outer ring color |
| `--line-color <HEX>` | `#00000000` | Separator line color |
| `--inside-color <HEX>` | `#00000088` | Inner circle fill color |
| `--separator-color <HEX>` | `#00000000` | Ring segment separator color |
| `--key-hl-color <HEX>` | `#4EAC41` | Key highlight segment color |
| `--caps-lock-key-hl-color <HEX>` | `#4EAC41` | Key highlight color when caps lock is on |
| `--caps-lock-bs-hl-color <HEX>` | `#DB3300` | Backspace highlight color in caps lock |
| `--caps-lock-color <HEX>` | `#E5A445` | Caps lock indicator ring color |
| `--caps-lock-text-color <HEX>` | `#E5A445` | Caps lock text color |
| `--verifying-color <HEX>` | `#0072FF` | Verifying feedback ring color |
| `--show-caps-lock-text` | `false` | Show "CAPS" text when caps lock is active |
| **Display** | | |
| `--show-media` | `false` | Show MPRIS media player information |
| `--show-battery` | `false` | Show battery status |
| `--show-network` | `false` | Show WiFi SSID and signal strength |
| `--show-bluetooth` | `false` | Show Bluetooth status |
| `--show-album-art` | `false` | Show album art for media |
| `--show-keyboard-layout` | `false` | Show keyboard layout indicator |
| **Feedback & Timing** | | |
| `--fade-in <SECONDS>` | `0.2` | Fade-in animation duration |
| `--grace <SECONDS>` | `0` | Grace period — any key press unlocks within N seconds |
| `--auth-timeout <MS>` | `10000` | PAM authentication timeout in milliseconds |
| `--wrong-password-duration <MS>` | `500` | Wrong password feedback animation duration |
| `--key-highlight-duration <MS>` | `300` | Key highlight feedback duration |
| `--cleared-feedback-duration <MS>` | `500` | Cleared password feedback duration |
| `--verifying-timeout <MS>` | `5000` | Verifying feedback fallback timeout |
| `--feedback-window-duration <MS>` | `1000` | Wrong password feedback input window |
| `--key-highlight-window-duration <MS>` | `200` | Key highlight input-side window |
| **System** | | |
| `--pam-service <NAME>` | `rustlock` | PAM service name |
| `--system-poll-interval <S>` | `2` | Polling interval for system status updates |
| `--dbus-reconnect-delay <S>` | `5` | Delay before reconnecting DBus on failure |
| `--command-timeout <S>` | `5` | Timeout for system commands |
| **Custom Icons** (PNG/SVG path) | | |
| `--wifi-icon <PATH>` | — | Custom WiFi icon |
| `--bluetooth-icon <PATH>` | — | Custom Bluetooth icon |
| `--battery-icon <PATH>` | — | Custom battery icon |
| `--media-prev-icon <PATH>` | — | Custom previous track icon |
| `--media-stop-icon <PATH>` | — | Custom stop icon |
| `--media-play-icon <PATH>` | — | Custom play icon |
| `--media-pause-icon <PATH>` | — | Custom pause icon |
| `--media-next-icon <PATH>` | — | Custom next track icon |
| **Logging** | | |
| `--log-file` | — | Write verbose logs to `~/.rustlock.log` |
| `--log-path <PATH>` | — | Path for log file (enables file logging, overrides `--log-file` default path) |

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

## 📄 License

GPL v3
