# wayrustlock

A production-ready Wayland screen locker inspired by swaylock-effects.

## ⚠️ SAFETY WARNING - READ BEFORE USE

**This tool is under active development.** Screen lockers can cause system lockups if they malfunction. 

**If the screen locker gets stuck:**
- Type password and press **Enter** to unlock (demo mode - any password works)
- Switch to another TTY: Press `Ctrl+Alt+F2`, login, then run `pkill -9 wayrustlock`
- From another terminal: `pkill -9 wayrustlock` or `killall wayrustlock`
- If screen is black/red: hard restart may be required

**Debug logging:** Check `~/.wayrustlock.log` to see what's happening

**Test with timeout first:**
```bash
timeout 15 ./target/release/wayrustlock --indicator --clock
```

Then check the log file:
```bash
cat ~/.wayrustlock.log
```

## Features (Implemented vs Planned)

### ✅ Implemented
- Session locking via ext-session-lock-v1 protocol (tested on sway)
- Buffer creation from Cairo surfaces (wl_shm)
- CLI argument parsing with all swaylock-effects options
- PAM authentication infrastructure (using pam-client crate)
- Keyboard handler with proper KeyEvent processing
- Module architecture (auth, input, lock, render, screenshot, timer, util)

### 🔄 In Progress
- Screenshot capture (wlr-screencopy protocol not yet integrated)
- Full PAM integration with auth loop

### ❌ Not Yet Implemented
- Real screenshot capture (currently shows solid color background)
- Grace period and fade-in animations

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
