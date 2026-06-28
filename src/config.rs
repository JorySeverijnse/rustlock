use crate::util;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum RingShape {
    #[default]
    Circle,
    Square,
    Diamond,
    Hexagon,
    Pill,
}

impl FromStr for RingShape {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "circle" => Ok(RingShape::Circle),
            "square" => Ok(RingShape::Square),
            "diamond" => Ok(RingShape::Diamond),
            "hexagon" => Ok(RingShape::Hexagon),
            "pill" => Ok(RingShape::Pill),
            _ => Err(format!(
                "Unknown ring shape '{}'. Options: circle, square, diamond, hexagon, pill",
                s
            )),
        }
    }
}

impl fmt::Display for RingShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RingShape::Circle => write!(f, "circle"),
            RingShape::Square => write!(f, "square"),
            RingShape::Diamond => write!(f, "diamond"),
            RingShape::Hexagon => write!(f, "hexagon"),
            RingShape::Pill => write!(f, "pill"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum CompletionShell {
    Bash,
    Fish,
    Zsh,
}

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub screenshots: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub clock: bool,

    #[arg(long, action = clap::ArgAction::SetTrue, default_value_t = true)]
    pub indicator: bool,

    #[arg(long, default_value = "100")]
    pub indicator_radius: u32,

    #[arg(long, default_value = "7")]
    pub indicator_thickness: u32,

    #[arg(long, default_value = "circle", value_parser = clap::value_parser!(RingShape))]
    #[serde(default)]
    pub ring_shape: RingShape,

    #[arg(long, value_parser = util::parse_blur_effect)]
    #[serde(
        deserialize_with = "util::deserialize_blur_effect",
        serialize_with = "util::serialize_blur_effect",
        default
    )]
    pub effect_blur: Option<(u32, u32)>,

    #[arg(long, value_parser = util::parse_vignette_effect)]
    #[serde(
        deserialize_with = "util::deserialize_vignette_effect",
        serialize_with = "util::serialize_vignette_effect",
        default
    )]
    pub effect_vignette: Option<(f32, f32)>,

    #[arg(long)]
    #[serde(default)]
    pub effect_pixelate: Option<u32>,

    #[arg(long)]
    #[serde(default)]
    pub effect_swirl: Option<f32>,

    #[arg(long, default_value = "785412", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub ring_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "4EAC41", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub key_hl_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "4EAC41", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub caps_lock_key_hl_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "DB3300", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub caps_lock_bs_hl_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "E5A445", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub caps_lock_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "E5A445", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub caps_lock_text_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "0072FF", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub verifying_color: (f64, f64, f64, f64),

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_caps_lock_text: bool,

    #[arg(long, default_value = "00000000", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub line_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "00000088", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub inside_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "00000000", value_parser = util::parse_hex_color)]
    #[serde(
        deserialize_with = "util::deserialize_hex_color",
        serialize_with = "util::serialize_hex_color"
    )]
    pub separator_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "0")]
    pub grace: f32,

    #[arg(long, default_value = "0.2")]
    pub fade_in: f32,

    #[arg(long, default_value = "login")]
    pub pam_service: String,

    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub debug: bool,

    /// Write verbose logs to ~/.rustlock.log
    #[arg(long)]
    pub log_file: bool,

    /// Path for log file (enables file logging, overrides --log-file default path)
    #[arg(long)]
    #[serde(default)]
    pub log_path: Option<PathBuf>,

    /// Timeout (ms) for PAM authentication before showing failure
    #[arg(long, default_value = "10000")]
    pub auth_timeout: u64,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_media: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_battery: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_network: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_bluetooth: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_album_art: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub hide_password: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub show_keyboard_layout: bool,

    #[arg(long)]
    #[serde(default)]
    pub image: Option<PathBuf>,

    #[arg(long)]
    #[serde(default)]
    pub wifi_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub bluetooth_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub battery_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub media_prev_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub media_stop_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub media_play_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub media_pause_icon: Option<String>,

    #[arg(long)]
    #[serde(default)]
    pub media_next_icon: Option<String>,

    /// Maximum number of password dots in the indicator ring
    #[arg(long, default_value = "24")]
    pub max_dots: u32,

    /// Duration (ms) for wrong password feedback animation
    #[arg(long, default_value = "500")]
    pub wrong_password_duration: u64,

    /// Duration (ms) for key highlight feedback animation
    #[arg(long, default_value = "300")]
    pub key_highlight_duration: u64,

    /// Duration (ms) for cleared password feedback animation
    #[arg(long, default_value = "500")]
    pub cleared_feedback_duration: u64,

    /// Duration (ms) for verifying feedback fallback timeout
    #[arg(long, default_value = "5000")]
    pub verifying_timeout: u64,

    /// Duration (ms) that wrong password feedback is shown input-side
    #[arg(long, default_value = "1000")]
    pub feedback_window_duration: u64,

    /// Duration (ms) for key highlight feedback input-side window
    #[arg(long, default_value = "200")]
    pub key_highlight_window_duration: u64,

    /// Polling interval (seconds) for system status updates
    #[arg(long, default_value = "2")]
    pub system_poll_interval: u64,

    /// Delay (seconds) before reconnecting DBus on failure
    #[arg(long, default_value = "5")]
    pub dbus_reconnect_delay: u64,

    /// Timeout (seconds) for system commands (poweroff, reboot, suspend)
    #[arg(long, default_value = "5")]
    pub command_timeout: u64,

    /// Generate shell completions for the given shell
    #[arg(long, value_enum)]
    #[serde(skip)]
    pub completions: Option<CompletionShell>,
}

impl Config {
    pub fn load() -> Self {
        use clap::CommandFactory;

        let mut config = Config::parse();

        // Handle --completions early (before config file merge so file can't inject it)
        if let Some(shell) = config.completions {
            use clap_complete::Shell;
            let mut cmd = Config::command();
            let name = "rustlock";
            match shell {
                CompletionShell::Bash => {
                    clap_complete::generate(Shell::Bash, &mut cmd, name, &mut std::io::stdout())
                }
                CompletionShell::Fish => {
                    clap_complete::generate(Shell::Fish, &mut cmd, name, &mut std::io::stdout())
                }
                CompletionShell::Zsh => {
                    clap_complete::generate(Shell::Zsh, &mut cmd, name, &mut std::io::stdout())
                }
            }
            std::process::exit(0);
        }

        let cmd = Config::command();
        let matches = cmd.get_matches();

        // Helper to check if a value was explicitly set on command line
        let is_cli =
            |key: &str| matches.value_source(key) == Some(clap::parser::ValueSource::CommandLine);

        // Config file layer (overrides defaults, CLI args take precedence)
        let config_path = config.config.clone().unwrap_or_else(|| {
            let mut path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
            path.push(".config/rustlock/config.toml");
            path
        });

        if config_path.exists() {
            if let Ok(file_content) = std::fs::read_to_string(&config_path) {
                if let Ok(file_table) = toml::from_str::<toml::Table>(&file_content) {
                    log::debug!("Loaded configuration from {:?}", config_path);

                    // Convert current config to a TOML table to facilitate merging
                    if let Ok(mut config_table) = toml::Value::try_from(config.clone()) {
                        if let Some(config_table) = config_table.as_table_mut() {
                            for (key, value) in file_table {
                                if !is_cli(&key) {
                                    config_table.insert(key, value);
                                }
                            }

                            // Convert back to Config struct
                            if let Ok(new_config) =
                                toml::Value::Table(config_table.clone()).try_into::<Config>()
                            {
                                config = new_config;
                            }
                        }
                    }
                }
            }
        }

        config.auth_timeout = config.auth_timeout.max(100);
        config.max_dots = config.max_dots.max(1);
        config.fade_in = config.fade_in.max(0.0);
        config.grace = config.grace.max(0.0);

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_dots_default() {
        let config = Config::parse_from(["test"]);
        assert_eq!(config.max_dots, 24);
    }

    #[test]
    fn test_auth_timeout_default() {
        let config = Config::parse_from(["test"]);
        assert_eq!(config.auth_timeout, 10000);
    }

    #[test]
    fn test_auth_timeout_min_clamp() {
        let mut config = Config::parse_from(["test", "--auth-timeout", "0"]);
        config.auth_timeout = config.auth_timeout.max(100);
        assert_eq!(config.auth_timeout, 100);
    }

    #[test]
    fn test_max_dots_min_clamp() {
        let mut config = Config::parse_from(["test", "--max-dots", "0"]);
        config.max_dots = config.max_dots.max(1);
        assert_eq!(config.max_dots, 1);
    }

    #[test]
    fn test_fade_in_negative_clamp() {
        let mut config = Config::parse_from(["test", "--fade-in=-1"]);
        config.fade_in = config.fade_in.max(0.0);
        assert_eq!(config.fade_in, 0.0);
    }

    #[test]
    fn test_grace_negative_clamp() {
        let mut config = Config::parse_from(["test", "--grace=-1"]);
        config.grace = config.grace.max(0.0);
        assert_eq!(config.grace, 0.0);
    }

    #[test]
    fn test_log_path_default_none() {
        let config = Config::parse_from(["test"]);
        assert!(config.log_path.is_none());
    }
}
