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
    pub effect_blur: Option<(u32, u32)>,

    #[arg(long, value_parser = util::parse_vignette_effect)]
    pub effect_vignette: Option<(f32, f32)>,

    #[arg(long)]
    pub effect_pixelate: Option<u32>,

    #[arg(long)]
    pub effect_swirl: Option<f32>,

    #[arg(long)]
    pub effect_melting: Option<f32>,

    #[arg(long, default_value = "785412", value_parser = util::parse_hex_color)]
    pub ring_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "4EAC41", value_parser = util::parse_hex_color)]
    pub key_hl_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "4EAC41", value_parser = util::parse_hex_color)]
    pub caps_lock_key_hl_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "DB3300", value_parser = util::parse_hex_color)]
    pub caps_lock_bs_hl_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "E5A445", value_parser = util::parse_hex_color)]
    pub caps_lock_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "E5A445", value_parser = util::parse_hex_color)]
    pub caps_lock_text_color: (f64, f64, f64, f64),

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_caps_lock_text: bool,

    #[arg(long, action = clap::ArgAction::Set, num_args = 0..=1, default_value = "true", default_missing_value = "true")]
    pub show_masked_password: bool,

    #[arg(long, default_value = "00000000", value_parser = util::parse_hex_color)]
    pub line_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "00000088", value_parser = util::parse_hex_color)]
    pub inside_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "00000000", value_parser = util::parse_hex_color)]
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
    pub image: Option<PathBuf>,

    #[arg(long)]
    pub wifi_icon: Option<String>,

    #[arg(long)]
    pub bluetooth_icon: Option<String>,

    #[arg(long)]
    pub battery_icon: Option<String>,

    #[arg(long)]
    pub media_prev_icon: Option<String>,

    #[arg(long)]
    pub media_stop_icon: Option<String>,

    #[arg(long)]
    pub media_play_icon: Option<String>,

    #[arg(long)]
    pub media_next_icon: Option<String>,

    /// Apply a pre-defined theme preset
    #[arg(long)]
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
                if let Ok(table) = toml::from_str::<toml::Table>(&file_content) {
                    log::debug!("Loaded configuration from {:?}", config_path);

                    let merge_bool = |val: &mut bool, key: &str| {
                        if !is_cli(key) {
                            if let Some(toml::Value::Boolean(b)) = table.get(key) {
                                *val = *b;
                            }
                        }
                    };

                    let merge_u32 = |val: &mut u32, key: &str| {
                        if !is_cli(key) {
                            if let Some(toml::Value::Integer(i)) = table.get(key) {
                                *val = *i as u32;
                            }
                        }
                    };

                    let merge_f32 = |val: &mut f32, key: &str| {
                        if !is_cli(key) {
                            if let Some(toml::Value::Float(f)) = table.get(key) {
                                *val = *f as f32;
                            } else if let Some(toml::Value::Integer(i)) = table.get(key) {
                                *val = *i as f32;
                            }
                        }
                    };

                    let merge_string = |val: &mut String, key: &str| {
                        if !is_cli(key) {
                            if let Some(toml::Value::String(s)) = table.get(key) {
                                *val = s.clone();
                            }
                        }
                    };

                    merge_bool(&mut config.screenshots, "screenshots");
                    merge_bool(&mut config.clock, "clock");
                    merge_bool(&mut config.indicator, "indicator");
                    merge_u32(&mut config.indicator_radius, "indicator_radius");
                    merge_u32(&mut config.indicator_thickness, "indicator_thickness");
                    merge_f32(&mut config.grace, "grace");
                    merge_f32(&mut config.fade_in, "fade_in");
                    merge_string(&mut config.pam_service, "pam_service");
                    merge_bool(&mut config.show_media, "show_media");
                    merge_bool(&mut config.show_battery, "show_battery");
                    merge_bool(&mut config.show_network, "show_network");
                    merge_bool(&mut config.show_bluetooth, "show_bluetooth");
                    merge_bool(&mut config.show_album_art, "show_album_art");
                    merge_bool(&mut config.show_keyboard_layout, "show_keyboard_layout");
                    merge_bool(&mut config.show_masked_password, "show_masked_password");

                    if !is_cli("image") {
                        if let Some(toml::Value::String(s)) = table.get("image") {
                            config.image = Some(std::path::PathBuf::from(s));
                        }
                    }

                    if !is_cli("wifi_icon") {
                        if let Some(toml::Value::String(s)) = table.get("wifi_icon") {
                            config.wifi_icon = Some(s.clone());
                        }
                    }

                    if !is_cli("bluetooth_icon") {
                        if let Some(toml::Value::String(s)) = table.get("bluetooth_icon") {
                            config.bluetooth_icon = Some(s.clone());
                        }
                    }

                    if !is_cli("battery_icon") {
                        if let Some(toml::Value::String(s)) = table.get("battery_icon") {
                            config.battery_icon = Some(s.clone());
                        }
                    }

                    if !is_cli("media_prev_icon") {
                        if let Some(toml::Value::String(s)) = table.get("media_prev_icon") {
                            config.media_prev_icon = Some(s.clone());
                        }
                    }

                    if !is_cli("media_stop_icon") {
                        if let Some(toml::Value::String(s)) = table.get("media_stop_icon") {
                            config.media_stop_icon = Some(s.clone());
                        }
                    }

                    if !is_cli("media_play_icon") {
                        if let Some(toml::Value::String(s)) = table.get("media_play_icon") {
                            config.media_play_icon = Some(s.clone());
                        }
                    }

                    if !is_cli("media_next_icon") {
                        if let Some(toml::Value::String(s)) = table.get("media_next_icon") {
                            config.media_next_icon = Some(s.clone());
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
