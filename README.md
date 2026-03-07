# rustlock

A high-performance Wayland screen locker written in Rust, inspired by `swaylock-effects`.

## Features

- **Performance**: Written in safe Rust with minimal dependencies
- **Visual Effects**:
  - Gaussian blur (configurable radius and passes)
  - Vignette effect (configurable base and factor)
  - Smooth fade-in animation
- **Password Indicator**:
  - Circular ring with configurable radius and thickness
  - Dynamic key highlight segments that rotate with each keystroke
  - Customizable colors (ring, inside, separator, highlight)
- **Information Display**:
  - Centered clock (HH:MM format)
  - Full date
  - System uptime
- **Screenshot Support**:
  - Captures desktop background before locking
  - Applies visual effects to background
 - **Authentication**:
   - PAM-based authentication
   - Configurable grace period (any key press within N seconds unlocks without password)
- **Logging**: Verbose debug logs written to `~/.rustlock.log`

## Usage

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

## Configuration

Options can be provided via command line or a configuration file at `~/.config/rustlock/config.toml`. CLI arguments take precedence.

### Options

| Option | Description |
|--------|-------------|
| `--screenshots` | Capture desktop background before locking |
| `--clock` | Display centered clock and date |
| `--indicator` | Show password entry ring (default: true) |
| `--indicator-radius <N>` | Ring radius in pixels (default: 100) |
| `--indicator-thickness <N>` | Ring thickness in pixels (default: 7) |
| `--effect-blur <R>x<P>` | Gaussian blur: radius x passes (e.g., `7x5`) |
| `--effect-vignette <B>:<F>` | Vignette: base:factor (e.g., `0.5:0.5`) |
| `--ring-color <RRGGBB[AA]>` | Outer ring color (hex, optional alpha) |
| `--key-hl-color <RRGGBB[AA]>` | Key highlight segment color |
| `--line-color <RRGGBB[AA]>` | Separator line color |
| `--inside-color <RRGGBB[AA]>` | Inner circle color |
| `--separator-color <RRGGBB[AA]>` | Ring separator color |
| `--grace <SECONDS>` | Grace period in seconds (default: 2) |
| `--fade-in <SECONDS>` | Fade-in animation duration (default: 0.2) |
| `--pam-service <NAME>` | PAM service name (default: "rustlock") |
| `--config <PATH>` | Path to config file |
| `--debug` | Enable debug logging |
| `--log-file` | Write logs to `~/.rustlock.log` |
| `--temp-screenshot` | Enable peek feature (press 'p' to temporarily show background) |

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/rustlock`.

## License

MIT / Apache 2.0
