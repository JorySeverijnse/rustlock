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

    #[arg(long, default_value = "true")]
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

    /// Write verbose logs to ~/.wayrustlock.log
    #[arg(long)]
    pub log_file: bool,

    /// Show screen temporarily when a key is pressed (like swaylock-effects peek)
    #[arg(long)]
    pub temp_screenshot: bool,
}

impl Config {
    pub fn load() -> Self {
        let cli_config = Config::parse();
        
        // Use path from CLI if provided, otherwise default
        let config_path = cli_config.config.clone().unwrap_or_else(|| {
            let mut path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
            path.push(".config/wayrustlock/config.toml");
            path
        });

        if config_path.exists() {
            if let Ok(file_content) = std::fs::read_to_string(&config_path) {
                if let Ok(_file_config) = toml::from_str::<Config>(&file_content) {
                    return cli_config; 
                }
            }
        }

        cli_config
    }
}
