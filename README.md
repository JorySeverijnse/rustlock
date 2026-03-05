# wayrustlock

A production-ready Wayland screen locker inspired by swaylock-effects.

## Features

- Session locking via ext-session-lock-v1 protocol
- Screenshot capture using wlr-screencopy-unstable-v1
- Gaussian blur and vignette effects
- Clock display with customizable formatting
- Indicator ring with customizable colors and dimensions
- PAM authentication
- Grace period and fade-in animations
- Multi-monitor support

## Installation

```bash
cargo build --release
```

## Usage

Basic usage:
```bash
wayrustlock
```

With all options from swaylock-effects compatibility:
```bash
wayrustlock \
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

## Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--screenshots` | Take screenshots of each output as background | false |
| `--clock` | Show clock in center of screen | false |
| `--indicator` | Show password indicator ring | false |
| `--indicator-radius` | Radius of indicator ring in pixels | 100 |
| `--indicator-thickness` | Thickness of indicator ring in pixels | 7 |
| `--effect-blur` | Blur radius and iterations (e.g., 7x5) | none |
| `--effect-vignette` | Vignette base:factor (e.g., 0.5:0.5) | none |
| `--ring-color` | Ring color (hex RRGGBB) | 785412 |
| `--key-hl-color` | Key press highlight color | 4EAC41 |
| `--line-color` | Line color | 00000000 |
| `--inside-color` | Inside fill color | 00000088 |
| `--separator-color` | Separator color | 00000000 |
| `--grace` | Grace period in seconds before password required | 2 |
| `--fade-in` | Fade-in duration in seconds | 0.2 |
| `--pam-service` | PAM service name | login |
| `--config` | Path to TOML config file | none |
| `--debug` | Enable debug logging | false |

## Configuration File

You can also use a TOML configuration file:

```toml
screenshots = true
clock = true
indicator = true
indicator_radius = 100
indicator_thickness = 7
effect_blur = "7x5"
effect_vignette = "0.5:0.5"
ring_color = "785412"
key_hl_color = "4EAC41"
line_color = "00000000"
inside_color = "00000088"
separator_color = "00000000"
grace = 2
fade_in = 0.2
pam_service = "login"
```

## Dependencies

- Wayland compositor (sway, labwc, etc.)
- PAM (linux-pam)
- Required Wayland protocols:
  - ext-session-lock-v1
  - wlr-screencopy-unstable-v1

## Building

This project requires Rust 2021 edition and the following dependencies:

- wayland development libraries
- cairo development libraries
- pam development libraries

On Debian/Ubuntu:
```bash
sudo apt install libwayland-dev libcairo2-dev libpam0g-dev
```

On Fedora:
```bash
sudo dnf install wayland-devel cairo-devel pam-devel
```

## License

MIT
