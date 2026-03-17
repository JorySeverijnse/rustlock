use cairo::{Context, Format, ImageSurface};
use std::time::Instant;

use crate::config::Config;
use crate::system::SystemStatus;

/// Cairo-based renderer for the lock screen
pub struct Renderer {
    width: i32,
    height: i32,
    config: Config,
    surface: ImageSurface,
    context: Context,
    fade_alpha: f64,
    wrong_password_shown: bool,
    key_highlight_shown: bool,
    wrong_password_start: Option<Instant>,
    key_highlight_start: Option<Instant>,
    key_highlight_angle: f64,
    background: Option<ImageSurface>,
    password_display: String,
    uptime_cache: String,
    last_uptime_update: Option<Instant>,
    pub caps_lock: bool,
    pub system_status: SystemStatus,
    media_art_surface: Option<ImageSurface>,
    last_art_url: Option<String>,
    wifi_icon_surface: Option<ImageSurface>,
    bluetooth_icon_surface: Option<ImageSurface>,
    battery_icon_surface: Option<ImageSurface>,
    media_prev_icon_surface: Option<ImageSurface>,
    media_stop_icon_surface: Option<ImageSurface>,
    media_play_icon_surface: Option<ImageSurface>,
    media_next_icon_surface: Option<ImageSurface>,
}

impl Renderer {
    /// Create a new renderer with the given dimensions and configuration
    pub fn new(width: i32, height: i32, config: Config) -> Self {
        log::debug!("Renderer::new({}, {}, ...) called", width, height);
        let surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Failed to create Cairo surface");
        let context = Context::new(&surface).expect("Failed to create Cairo context");

        let mut renderer = Self {
            width,
            height,
            config: config.clone(),
            surface,
            context,
            fade_alpha: 0.0,
            wrong_password_shown: false,
            key_highlight_shown: false,
            wrong_password_start: None,
            key_highlight_start: None,
            key_highlight_angle: 0.0,
            background: None,
            password_display: String::new(),
            uptime_cache: String::new(),
            last_uptime_update: None,
            caps_lock: false,
            system_status: SystemStatus::default(),
            media_art_surface: None,
            last_art_url: None,
            wifi_icon_surface: None,
            bluetooth_icon_surface: None,
            battery_icon_surface: None,
            media_prev_icon_surface: None,
            media_stop_icon_surface: None,
            media_play_icon_surface: None,
            media_next_icon_surface: None,
        };

        renderer.load_icons();
        renderer
    }

    fn load_icons(&mut self) {
        if let Some(ref path) = self.config.wifi_icon {
            self.wifi_icon_surface = self.load_icon(path);
        }
        if let Some(ref path) = self.config.bluetooth_icon {
            self.bluetooth_icon_surface = self.load_icon(path);
        }
        if let Some(ref path) = self.config.battery_icon {
            self.battery_icon_surface = self.load_icon(path);
        }
        if let Some(ref path) = self.config.media_prev_icon {
            self.media_prev_icon_surface = self.load_icon(path);
        }
        if let Some(ref path) = self.config.media_stop_icon {
            self.media_stop_icon_surface = self.load_icon(path);
        }
        if let Some(ref path) = self.config.media_play_icon {
            self.media_play_icon_surface = self.load_icon(path);
        }
        if let Some(ref path) = self.config.media_next_icon {
            self.media_next_icon_surface = self.load_icon(path);
        }
    }

    fn load_icon(&self, identifier: &str) -> Option<ImageSurface> {
        let path = if identifier.starts_with('/') {
            std::path::PathBuf::from(identifier)
        } else {
            return None;
        };

        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&path) {
            let w = pixbuf.width();
            let h = pixbuf.height();
            let mut surface = ImageSurface::create(Format::ARgb32, w, h).ok()?;
            {
                let mut surface_data = surface.data().ok()?;
                let pix_data = unsafe { pixbuf.pixels() };
                let n_channels = pixbuf.n_channels();
                let rowstride = pixbuf.rowstride() as usize;

                for y in 0..h as usize {
                    for x in 0..w as usize {
                        let pix_idx = y * rowstride + x * n_channels as usize;
                        let surf_idx = (y * w as usize + x) * 4;

                        if n_channels == 4 {
                            surface_data[surf_idx] = pix_data[pix_idx + 2];
                            surface_data[surf_idx + 1] = pix_data[pix_idx + 1];
                            surface_data[surf_idx + 2] = pix_data[pix_idx];
                            surface_data[surf_idx + 3] = pix_data[pix_idx + 3];
                        } else if n_channels == 3 {
                            surface_data[surf_idx] = pix_data[pix_idx + 2];
                            surface_data[surf_idx + 1] = pix_data[pix_idx + 1];
                            surface_data[surf_idx + 2] = pix_data[pix_idx];
                            surface_data[surf_idx + 3] = 255;
                        }
                    }
                }
            }
            Some(surface)
        } else {
            None
        }
    }

    /// Resize the renderer to new dimensions
    pub fn resize(&mut self, width: i32, height: i32) {
        log::debug!("Renderer::resize({}, {}) called", width, height);
        self.width = width;
        self.height = height;

        self.surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Failed to create Cairo surface");
        self.context = Context::new(&self.surface).expect("Failed to create Cairo context");
    }

    /// Set the background image (screenshot)
    pub fn set_background(&mut self, background: ImageSurface) {
        self.background = Some(background);
    }

    /// Set the fade-in alpha value (0.0 to 1.0)
    pub fn set_fade_alpha(&mut self, alpha: f64) {
        self.fade_alpha = alpha.clamp(0.0, 1.0);
    }

    /// Show wrong password feedback
    pub fn show_wrong_password(&mut self) {
        self.wrong_password_shown = true;
        self.wrong_password_start = Some(Instant::now());
    }

    /// Show key highlight feedback
    pub fn show_key_highlight(&mut self) {
        self.key_highlight_shown = true;
        self.key_highlight_start = Some(Instant::now());

        // Generate ONE random angle for this highlight
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let random_val = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.key_highlight_angle = ((random_val % 360) as f64).to_radians();
    }

    /// Set the password display string (masked)
    pub fn set_password_display(&mut self, password: String) {
        self.password_display = password;
    }

    /// Render the current frame
    pub fn render(&mut self) {
        // Clear the surface
        self.context.new_path();
        self.context.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        self.context.paint().expect("Failed to clear surface");

        // Draw background
        if let Some(ref background) = self.background {
            self.context.new_path();
            self.context
                .set_source_surface(background, 0.0, 0.0)
                .expect("Failed to set source");
            self.context
                .paint_with_alpha(self.fade_alpha)
                .expect("Failed to paint");
        }

        if self.config.indicator {
            self.draw_indicator();
        }

        if self.config.clock {
            self.draw_clock();
        }

        if self.config.show_media {
            self.draw_media();
        }

        if self.config.show_network {
            self.draw_network();
        }

        if self.config.show_battery {
            self.draw_status();
        }

        if self.config.show_bluetooth {
            self.draw_bluetooth();
        }

        if self.config.show_keyboard_layout {
            self.draw_keyboard_layout();
        }

        if !self.password_display.is_empty() {
            self.draw_password_display();
        }

        if self.caps_lock && self.config.show_caps_lock_text {
            self.draw_caps_lock_indicator();
        }

        if self.wrong_password_shown {
            self.draw_wrong_password_feedback();
        }

        if self.key_highlight_shown {
            self.draw_key_highlight_feedback();
        }

        self.update_feedback_timers();
    }

    /// Get raw pixel data from the surface (ARGB32 format)
    pub fn get_pixel_data(&self) -> Result<Vec<u8>, cairo::BorrowError> {
        let stride = self.surface.stride() as usize;
        let height = self.height as usize;
        let mut data = vec![0u8; stride * height];
        self.surface.with_data(|src| {
            data.copy_from_slice(src);
        })?;
        Ok(data)
    }

    pub fn surface_info(&self) -> (i32, i32, i32) {
        (self.width, self.height, self.surface.stride())
    }

    fn update_uptime(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_uptime_update {
            if now.duration_since(last).as_secs() < 10 {
                return;
            }
        }
        let uptime_secs = std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0) as u64;
        self.uptime_cache = format!("up {}h {}m", uptime_secs / 3600, (uptime_secs % 3600) / 60);
        self.last_uptime_update = Some(now);
    }

    fn draw_clock(&self) {
        use chrono::Local;
        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        let date_str = now.format("%A, %B %d").to_string();
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;

        self.context.new_path();
        self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
        self.context.set_font_size(48.0);
        let te = self.context.text_extents(&time_str).unwrap();
        self.context
            .move_to(center_x - te.width() / 2.0, center_y + te.height() / 4.0);
        self.context.show_text(&time_str).unwrap();

        self.context.new_path();
        self.context.set_font_size(14.0);
        let de = self.context.text_extents(&date_str).unwrap();
        self.context.move_to(
            center_x - de.width() / 2.0,
            center_y + te.height() / 4.0 + 25.0,
        );
        self.context.show_text(&date_str).unwrap();

        self.context.new_path();
        let ue = self.context.text_extents(&self.uptime_cache).unwrap();
        self.context.move_to(
            center_x - ue.width() / 2.0,
            center_y + te.height() / 4.0 + 43.0,
        );
        self.context.show_text(&self.uptime_cache).unwrap();
    }

    fn draw_indicator(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;

        self.context.new_path();
        let (r, g, b, a) = self.config.inside_color;
        self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
        self.context.arc(
            center_x,
            center_y,
            radius - thickness / 2.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        self.context.fill().unwrap();

        let (lr, lg, lb, la) = self.config.line_color;
        if la > 0.0 {
            self.context.new_path();
            self.context
                .set_source_rgba(lr, lg, lb, la * self.fade_alpha);
            self.context.set_line_width(1.0);
            self.context.arc(
                center_x,
                center_y,
                radius - thickness / 2.0,
                0.0,
                2.0 * std::f64::consts::PI,
            );
            self.context.stroke().unwrap();
        }

        // Use caps lock color when caps lock is on and indicator is enabled, otherwise ring color
        let (r, g, b, a) = if self.caps_lock {
            self.config.caps_lock_color
        } else {
            self.config.ring_color
        };
        self.context.new_path();
        self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
        self.context.set_line_width(thickness);
        self.context
            .arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
        self.context.stroke().unwrap();

        let (r, g, b, a) = self.config.separator_color;
        if a > 0.0 {
            self.context.new_path();
            self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
            self.context.set_line_width(1.0);
            self.context.move_to(center_x - radius, center_y);
            self.context.line_to(center_x + radius, center_y);
            self.context.stroke().unwrap();
        }
    }

    fn draw_password_display(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        self.context.new_path();
        self.context.set_font_size(32.0);
        self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
        let te = self.context.text_extents(&self.password_display).unwrap();
        self.context
            .move_to(center_x - te.width() / 2.0, center_y + radius / 1.1);
        self.context.show_text(&self.password_display).unwrap();
    }

    fn draw_caps_lock_indicator(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        self.context.new_path();
        // Use configurable caps lock text color
        let (r, g, b, a) = self.config.caps_lock_text_color;
        self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
        // Increase font size for bigger letters
        self.context.set_font_size(24.0);
        let text = "Caps Lock";
        let te = self.context.text_extents(text).unwrap();
        // Position above the ring
        self.context
            .move_to(center_x - te.width() / 2.0, center_y - radius - 10.0);
        self.context.show_text(text).unwrap();
    }

    fn draw_wrong_password_feedback(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;
        let intensity = if let Some(start) = self.wrong_password_start {
            let elapsed = start.elapsed();
            let duration = std::time::Duration::from_millis(500);
            if elapsed < duration {
                1.0 - (elapsed.as_secs_f64() / duration.as_secs_f64())
            } else {
                0.0
            }
        } else {
            0.0
        };

        if intensity > 0.0 {
            self.context.new_path();
            self.context
                .set_source_rgba(1.0, 0.0, 0.0, intensity * self.fade_alpha);
            self.context.set_line_width(thickness + 2.0);
            self.context
                .arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
            self.context.stroke().unwrap();
        }
    }

    fn draw_key_highlight_feedback(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;
        let intensity = if let Some(start) = self.key_highlight_start {
            let elapsed = start.elapsed();
            let duration = std::time::Duration::from_millis(300);
            if elapsed < duration {
                1.0 - (elapsed.as_secs_f64() / duration.as_secs_f64())
            } else {
                0.0
            }
        } else {
            0.0
        };

        if intensity > 0.0 {
            let (r, g, b, a) = if self.caps_lock {
                self.config.caps_lock_key_hl_color
            } else {
                self.config.key_hl_color
            };
            self.context
                .set_source_rgba(r, g, b, a * intensity * self.fade_alpha);
            self.context.set_line_width(thickness + 1.5);

            // Draw ONLY ONE segment that rotates based on password length
            let global_offset = (self.password_display.len() as f64 * 45.0).to_radians();
            self.context.new_path();
            let actual_start = global_offset + self.key_highlight_angle;
            self.context.arc(
                center_x,
                center_y,
                radius,
                actual_start,
                actual_start + (40.0_f64).to_radians(),
            );
            self.context.stroke().unwrap();
        }
    }

    fn update_feedback_timers(&mut self) {
        self.update_uptime();
        if let Some(start) = self.wrong_password_start {
            if start.elapsed() > std::time::Duration::from_millis(500) {
                self.wrong_password_shown = false;
                self.wrong_password_start = None;
            }
        }
        if let Some(start) = self.key_highlight_start {
            if start.elapsed() > std::time::Duration::from_millis(300) {
                self.key_highlight_shown = false;
                self.key_highlight_start = None;
            }
        }
    }

    fn draw_media(&mut self) {
        if let Some(ref title) = self.system_status.media_title {
            let center_x = self.width as f64 / 2.0;
            let start_y = self.height as f64 - 120.0;
            let art_size = 56.0;
            let spacing = 80.0;

            if self.config.show_album_art
                && self.system_status.media_art_url != self.last_art_url {
                    self.last_art_url = self.system_status.media_art_url.clone();
                    self.media_art_surface = None;
                    if let Some(ref data) = self.system_status.media_art_data {
                        if let Ok(img) = image::load_from_memory(data) {
                            let img = img.to_rgba8();
                            let (w, h) = img.dimensions();
                            let mut surface =
                                ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();
                            {
                                let mut surface_data = surface.data().unwrap();
                                for y in 0..h {
                                    for x in 0..w {
                                        let pixel = img.get_pixel(x, y);
                                        let idx = ((y * w + x) * 4) as usize;
                                        surface_data[idx] = pixel[2];
                                        surface_data[idx + 1] = pixel[1];
                                        surface_data[idx + 2] = pixel[0];
                                        surface_data[idx + 3] = pixel[3];
                                    }
                                }
                            }
                            self.media_art_surface = Some(surface);
                        }
                    }
                }

            let has_art = self.config.show_album_art && self.media_art_surface.is_some();
            let text_x = if has_art {
                center_x - spacing / 2.0
            } else {
                center_x
            };
            let art_x = center_x - spacing - art_size / 2.0;

            if has_art {
                if let Some(ref art) = self.media_art_surface {
                    self.context.save().unwrap();
                    let scale = art_size / art.width() as f64;
                    self.context.translate(art_x, start_y);
                    self.context.scale(scale, scale);
                    self.context.set_source_surface(art, 0.0, 0.0).unwrap();
                    self.context.paint_with_alpha(self.fade_alpha).unwrap();
                    self.context.restore().unwrap();
                }
            }

            self.context.new_path();
            self.context
                .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * 0.9);
            self.context.set_font_size(16.0);

            let display_text = if let Some(ref artist) = self.system_status.media_artist {
                format!("{} - {}", artist, title)
            } else {
                title.clone()
            };

            let te = self.context.text_extents(&display_text).unwrap();
            self.context
                .move_to(text_x - te.width() / 2.0, start_y + 20.0);
            self.context.show_text(&display_text).unwrap();

            let status_text = if self.system_status.media_playing {
                if let Some(ref icon) = self.media_play_icon_surface {
                    // Draw play icon instead of text
                    let play_y = start_y + 40.0;
                    self.draw_icon_at(
                        center_x - icon.width() as f64 / 2.0,
                        play_y - icon.height() as f64 / 2.0,
                        icon,
                    );
                    ""
                } else {
                    "▶ Playing"
                }
            } else {
                "⏸ Paused"
            };

            if !status_text.is_empty() {
                self.context.set_font_size(12.0);
                let se = self.context.text_extents(status_text).unwrap();
                self.context
                    .move_to(text_x - se.width() / 2.0, start_y + 40.0);
                self.context.show_text(status_text).unwrap();
            }

            let controls_y = start_y + 65.0;

            // Draw media control icons
            let icon_size = 20.0;
            let icon_spacing = 40.0;
            let controls_center_x = center_x;

            // Previous icon
            if let Some(ref icon) = self.media_prev_icon_surface {
                let ix = controls_center_x - icon_spacing;
                self.draw_icon_at(ix - icon_size / 2.0, controls_y - icon_size / 2.0, icon);
            } else {
                self.context
                    .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * 0.7);
                self.context.set_font_size(16.0);
                self.context
                    .move_to(controls_center_x - icon_spacing - 8.0, controls_y);
                self.context.show_text("⏮").unwrap();
            }

            // Stop icon
            if let Some(ref icon) = self.media_stop_icon_surface {
                let ix = controls_center_x;
                self.draw_icon_at(ix - icon_size / 2.0, controls_y - icon_size / 2.0, icon);
            } else {
                self.context
                    .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * 0.7);
                self.context.set_font_size(16.0);
                self.context.move_to(controls_center_x - 8.0, controls_y);
                self.context.show_text("⏹").unwrap();
            }

            // Next icon
            if let Some(ref icon) = self.media_next_icon_surface {
                let ix = controls_center_x + icon_spacing;
                self.draw_icon_at(ix - icon_size / 2.0, controls_y - icon_size / 2.0, icon);
            } else {
                self.context
                    .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * 0.7);
                self.context.set_font_size(16.0);
                self.context
                    .move_to(controls_center_x + icon_spacing - 8.0, controls_y);
                self.context.show_text("⏭").unwrap();
            }
        }
    }

    fn draw_network(&self) {
        if !self.config.show_network {
            return;
        }
        let margin = 20.0;
        let x = margin;
        let y = margin + 20.0;

        if let Some(ref ssid) = self.system_status.wifi_ssid {
            if let Some(ref icon) = self.wifi_icon_surface {
                self.draw_icon_at(x, y - 15.0, icon);
                let text_x = x + icon.width() as f64 + 10.0;
                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(16.0);
                self.context.move_to(text_x, y);
                self.context.show_text(ssid).unwrap();
            } else {
                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(16.0);
                self.context.move_to(x, y);
                self.context.show_text(ssid).unwrap();
            }
        } else {
            let text = "No WiFi";
            self.context.new_path();
            self.context
                .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * 0.5);
            self.context.set_font_size(16.0);
            self.context.move_to(x, y);
            self.context.show_text(text).unwrap();
        }
    }

    fn draw_status(&self) {
        if let Some(percent) = self.system_status.battery_percent {
            let margin = 20.0;
            let icon_width = 30.0;
            let x = self.width as f64 - margin - icon_width - 50.0;
            let y = margin + 20.0;

            if let Some(ref icon) = self.battery_icon_surface {
                self.draw_icon_at(x, y - 15.0, icon);
                let text_x = x + icon.width() as f64 + 10.0;
                let battery_text = format!("{:.0}%", percent);
                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(16.0);
                self.context.move_to(text_x, y);
                self.context.show_text(&battery_text).unwrap();
            } else {
                self.draw_battery_icon_at(
                    x,
                    y - 12.0,
                    icon_width,
                    15.0,
                    percent,
                    self.system_status.is_charging,
                );
                let battery_text = format!("{:.0}%", percent);
                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(16.0);
                self.context.move_to(x + icon_width + 10.0, y);
                self.context.show_text(&battery_text).unwrap();
            }
        }
    }

    fn draw_bluetooth(&self) {
        if !self.config.show_bluetooth {
            return;
        }
        let margin = 20.0;
        let x = margin;
        let y = margin + 50.0;

        if self.system_status.bluetooth_connected {
            if let Some(ref icon) = self.bluetooth_icon_surface {
                self.draw_icon_at(x, y - 12.0, icon);
                let text_x = x + icon.width() as f64 + 10.0;
                let devices = self.system_status.bluetooth_devices.join(", ");
                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(14.0);
                self.context.move_to(text_x, y);
                self.context.show_text(&devices).unwrap();
            } else {
                let text = format!(
                    "🔵 {} device(s)",
                    self.system_status.bluetooth_devices.len()
                );
                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(14.0);
                self.context.move_to(x, y);
                self.context.show_text(&text).unwrap();
            }
        } else {
            let text = "🔴 Bluetooth off";
            self.context.new_path();
            self.context
                .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * 0.5);
            self.context.set_font_size(14.0);
            self.context.move_to(x, y);
            self.context.show_text(text).unwrap();
        }
    }

    fn draw_keyboard_layout(&self) {
        if self.config.show_keyboard_layout {
            if let Some(ref layout) = self.system_status.keyboard_layout {
                let margin = 20.0;
                let x = margin;
                let y = margin + 80.0;

                self.context.new_path();
                self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
                self.context.set_font_size(16.0);
                let text = format!("Layout: {}", layout);
                self.context.move_to(x, y);
                self.context.show_text(&text).unwrap();
            }
        }
    }

    fn draw_icon_at(&self, x: f64, y: f64, surface: &ImageSurface) {
        self.context.save().unwrap();
        let target_size = 24.0;
        let scale_x = target_size / surface.width() as f64;
        let scale_y = target_size / surface.height() as f64;
        let scale = scale_x.min(scale_y);
        self.context.translate(x, y);
        self.context.scale(scale, scale);
        self.context.set_source_surface(surface, 0.0, 0.0).unwrap();
        self.context.paint_with_alpha(self.fade_alpha).unwrap();
        self.context.restore().unwrap();
    }

    fn draw_battery_icon_at(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        percent: f64,
        charging: bool,
    ) {
        let alpha = self.fade_alpha;
        self.context.new_path();
        self.context.set_source_rgba(1.0, 1.0, 1.0, alpha * 0.5);
        self.context.set_line_width(2.0);
        self.context.rectangle(x, y, width, height);
        self.context.stroke().unwrap();
        self.context.new_path();
        self.context
            .rectangle(x + width, y + height / 4.0, 3.0, height / 2.0);
        self.context.fill().unwrap();
        let fill_width = (width - 4.0) * (percent / 100.0);
        self.context.new_path();
        if percent < 20.0 {
            self.context.set_source_rgba(1.0, 0.2, 0.2, alpha);
        } else {
            self.context.set_source_rgba(0.2, 1.0, 0.2, alpha * 0.8);
        }
        self.context
            .rectangle(x + 2.0, y + 2.0, fill_width, height - 4.0);
        self.context.fill().unwrap();
        if charging {
            self.context.new_path();
            self.context.set_source_rgba(1.0, 1.0, 0.0, alpha);
            let bx = x + width / 2.0;
            let by = y + height / 2.0;
            self.context.move_to(bx - 3.0, by + 2.0);
            self.context.line_to(bx + 1.0, by - 1.0);
            self.context.line_to(bx - 1.0, by - 1.0);
            self.context.line_to(bx + 3.0, by - 6.0);
            self.context.line_to(bx - 1.0, by - 3.0);
            self.context.line_to(bx + 1.0, by - 3.0);
            self.context.close_path();
            self.context.fill().unwrap();
        }
    }
}
