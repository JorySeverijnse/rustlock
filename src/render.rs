use cairo::{Context, Format, ImageSurface};
use std::time::Instant;

use crate::config::Config;

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
    caps_lock: bool,
}

impl Renderer {
    /// Create a new renderer with the given dimensions and configuration
    pub fn new(width: i32, height: i32, config: Config) -> Self {
        log::debug!("Renderer::new({}, {}, ...) called", width, height);
        let surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Failed to create Cairo surface");
        let context = Context::new(&surface).expect("Failed to create Cairo context");

        Self {
            width,
            height,
            config,
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

        if !self.password_display.is_empty() {
            self.draw_password_display();
        }

        if self.caps_lock {
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

        self.context.new_path();
        let (r, g, b, a) = self.config.ring_color;
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
        self.context.set_font_size(12.0);
        self.context.set_source_rgba(1.0, 0.5, 0.0, self.fade_alpha);
        let text = "CAPS LOCK";
        let te = self.context.text_extents(text).unwrap();
        self.context
            .move_to(center_x - te.width() / 2.0, center_y + radius / 1.1 + 20.0);
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
            let (r, g, b, a) = self.config.key_hl_color;
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
}
