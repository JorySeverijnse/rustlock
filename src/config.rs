use crate::util;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "false", default_missing_value = "true")]
    pub screenshots: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "false", default_missing_value = "true")]
    pub clock: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub indicator: bool,

    #[arg(long, default_value = "100")]
    pub indicator_radius: u32,

    #[arg(long, default_value = "7")]
    pub indicator_thickness: u32,

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

    #[arg(long)]
    #[serde(default)]
    pub effect_melting: Option<f32>,

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

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
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

    #[arg(long, default_value = "2")]
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

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_media: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_battery: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_network: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_bluetooth: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_album_art: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "false", default_missing_value = "true")]
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
    pub media_next_icon: Option<String>,

    /// Apply a pre-defined theme preset
    #[arg(long)]
    #[serde(default)]
    pub theme: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        use clap::CommandFactory;

        let mut config = Config::parse();
        let cmd = Config::command();
        let matches = cmd.get_matches();

        // Helper to check if a value was explicitly set on command line
        let is_cli =
            |key: &str| matches.value_source(key) == Some(clap::parser::ValueSource::CommandLine);

        // 1. Config file layer (overrides defaults and themes)
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

        // 2. Theme presets (applied to fields NOT set on CLI or in File)
        if let Some(theme) = &config.theme {
            match theme.as_str() {
                "modern" => {
                    if config.effect_blur.is_none() && !is_cli("effect_blur") {
                        config.effect_blur = Some((10, 3));
                    }
                    if config.effect_vignette.is_none() && !is_cli("effect_vignette") {
                        config.effect_vignette = Some((0.5, 0.5));
                    }
                    if !is_cli("indicator_radius") {
                        config.indicator_radius = 120;
                    }
                    if !is_cli("ring_color") {
                        config.ring_color = (0.2, 0.6, 0.8, 1.0);
                    }
                }
                "pixel" => {
                    if config.effect_pixelate.is_none() && !is_cli("effect_pixelate") {
                        config.effect_pixelate = Some(10);
                    }
                    if !is_cli("indicator_radius") {
                        config.indicator_radius = 80;
                    }
                    if !is_cli("ring_color") {
                        config.ring_color = (0.8, 0.2, 0.2, 1.0);
                    }
                }
                "glass" => {
                    if config.effect_blur.is_none() && !is_cli("effect_blur") {
                        config.effect_blur = Some((20, 5));
                    }
                    if !is_cli("inside_color") {
                        config.inside_color = (1.0, 1.0, 1.0, 0.1);
                    }
                    if !is_cli("ring_color") {
                        config.ring_color = (1.0, 1.0, 1.0, 0.5);
                    }
                }
                _ => {
                    log::warn!("Unknown theme: {}", theme);
                }
            }
        }

        config
    }
}
