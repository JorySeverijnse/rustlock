use crate::util;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long)]
    pub screenshots: bool,

    #[arg(long)]
    pub clock: bool,

    #[arg(long)]
    pub indicator: bool,

    #[arg(long, default_value = "100")]
    pub indicator_radius: u32,

    #[arg(long, default_value = "7")]
    pub indicator_thickness: u32,

    #[arg(long, value_parser = util::parse_blur_effect)]
    pub effect_blur: Option<(u32, u32)>,

    #[arg(long, value_parser = util::parse_vignette_effect)]
    pub effect_vignette: Option<(f32, f32)>,

    #[arg(long, default_value = "785412", value_parser = util::parse_hex_color)]
    pub ring_color: (f64, f64, f64, f64),

    #[arg(long, default_value = "4EAC41", value_parser = util::parse_hex_color)]
    pub key_hl_color: (f64, f64, f64, f64),

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

    /// Show screen temporarily when a key is pressed (like swaylock-effects peek)
    #[arg(long)]
    pub temp_screenshot: bool,
}

impl Config {
    pub fn load() -> Self {
        let mut config = Config::parse();

        if let Some(config_path) = &config.config {
            if let Ok(file_content) = std::fs::read_to_string(config_path) {
                if let Ok(file_config) = toml::from_str::<Config>(&file_content) {
                    config = file_config;
                } else {
                    eprintln!(
                        "Warning: Failed to parse config file {}",
                        config_path.display()
                    );
                }
            } else {
                eprintln!("Warning: Config file {} not found", config_path.display());
            }
        }

        config
    }
}
