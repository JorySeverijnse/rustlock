# wayrustlock

A premium, high-performance Wayland screen locker written in Rust, designed to replicate and exceed the capabilities of `swaylock-effects` while ensuring stability on modern compositors like Niri and Sway.

## Features

- **Blazing Fast Performance:** Written in 100% safe Rust for maximum efficiency and security.
- **Advanced Visual Effects:**
  - **Gaussian Blur:** Highly optimized blur effect for your desktop background.
  - **Vignette:** Add a professional vignette effect to darken the edges of your screen.
  - **Fade-in Animation:** Smooth transition from your desktop to the lock screen.
- **Iconic swaylock-effects UI:**
  - **Dynamic Rotating Highlights:** Premium visual feedback as you type, with segments that appear at random angles and rotate dynamically.
  - **Internal Information Hub:** A perfectly centered clock, date, and system uptime indicator inside the ring.
  - **Custom Colors:** Full control over ring, inside, separator, and highlight colors.
- **Robust Logic:**
  - **Niri Compatibility:** Fixed protocol violations that cause red screens on Niri.
  - **Pre-lock Screenshot:** Captures your desktop *before* locking to ensure visual effects work perfectly even if the compositor hides surfaces immediately.
  - **Grace Period:** Configurable unlock grace period for convenience.
  - **Safe Exit:** Always waits for the compositor's confirmation before exiting, preventing session corruption.
- **Permanent Logging:** Verbose debug logs are always captured in `~/.wayrustlock.log` for immediate troubleshooting.

## Usage

### Basic Command
```bash
wayrustlock --screenshots --effect-blur 7x5 --effect-vignette 0.5:0.5
```

### Full Configuration Example
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

## Configuration File

You can also provide settings via a configuration file located at `~/.config/wayrustlock/config.toml`. Note that CLI arguments always take precedence over the config file.

## Options

| Option | Description |
|--------|-------------|
| `--screenshots` | Enable desktop background capture. |
| `--clock` | Show the centered clock and uptime. |
| `--indicator` | Show the password indicator ring. |
| `--indicator-radius` | Radius of the indicator ring (default: 100). |
| `--indicator-thickness` | Thickness of the indicator ring (default: 7). |
| `--effect-blur` | Gaussian blur settings (e.g., `7x5` for radius 7, 5 passes). |
| `--effect-vignette` | Vignette settings (e.g., `0.5:0.5` for base:factor). |
| `--ring-color` | Color of the outer ring (RRGGBB[AA]). |
| `--inside-color` | Color of the inner circle (RRGGBB[AA]). |
| `--key-hl-color` | Color of the key highlight segments (RRGGBB[AA]). |
| `--grace` | Unlock grace period in seconds (default: 2). |
| `--fade-in` | Fade-in animation duration in seconds (default: 0.2). |

## Installation

Ensure you have a Rust toolchain and the necessary Wayland development libraries installed, then run:

```bash
cargo build --release
```

The binary will be available at `target/release/wayrustlock`.

## License

MIT / Apache 2.0
