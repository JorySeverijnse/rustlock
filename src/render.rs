use cairo::{Context, Format, ImageSurface};
use std::time::Instant;

use crate::config::Config;
use crate::util::Color;

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
    background: Option<ImageSurface>,
}

impl Renderer {
    /// Convert color tuple to Color struct
    fn tuple_to_color(&self, color: (f64, f64, f64, f64)) -> Color {
        Color {
            r: (color.0 * 255.0) as u8,
            g: (color.1 * 255.0) as u8,
            b: (color.2 * 255.0) as u8,
            a: (color.3 * 255.0) as u8,
        }
    }

    /// Create a new renderer with the given dimensions and configuration
    pub fn new(width: i32, height: i32, config: Config) -> Self {
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
            background: None,
        }
    }

    /// Resize the renderer to new dimensions
    pub fn resize(&mut self, width: i32, height: i32) {
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
    }

    /// Render the current frame
    pub fn render(&mut self) {
        // Clear the surface - draw a VISIBLE color (dark gray) instead of black
        self.context.set_source_rgba(0.15, 0.15, 0.15, 1.0);
        self.context.paint().expect("Failed to clear surface");

        // Draw background if available
        if let Some(ref background) = self.background {
            self.context
                .set_source_surface(background, 0.0, 0.0)
                .expect("Failed to set background source");
            self.context
                .paint_with_alpha(self.fade_alpha)
                .expect("Failed to draw background");
        } else {
            // Draw solid color background (dark gray visible color)
            self.context.set_source_rgba(0.15, 0.15, 0.15, 1.0);
            self.context
                .paint()
                .expect("Failed to draw solid background");
        }

        // Draw clock if enabled
        if self.config.clock {
            self.draw_clock();
        }

        // Draw indicator if enabled
        if self.config.indicator {
            self.draw_indicator();
        }

        // Draw wrong password feedback if active
        if self.wrong_password_shown {
            self.draw_wrong_password_feedback();
        }

        // Draw key highlight feedback if active
        if self.key_highlight_shown {
            self.draw_key_highlight_feedback();
        }

        self.update_feedback_timers();
    }

    /// Get the rendered image surface
    pub fn as_image_surface(&self) -> &ImageSurface {
        &self.surface
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

    /// Get surface dimensions and stride
    pub fn surface_info(&self) -> (i32, i32, i32) {
        (self.width, self.height, self.surface.stride())
    }

    /// Draw the clock in the center of the screen
    fn draw_clock(&self) {
        use chrono::Local;

        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        let date_str = now.format("%A, %B %d").to_string();

        self.context.set_font_size(72.0);
        self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);

        // Center the text
        let extents = self
            .context
            .text_extents(&time_str)
            .expect("Failed to get text extents");
        let x = (self.width as f64 - extents.width()) / 2.0;
        let y = (self.height as f64 / 2.0) - extents.height() / 2.0;

        self.context.move_to(x, y);
        self.context
            .show_text(&time_str)
            .expect("Failed to draw time");

        // Draw date below time
        self.context.set_font_size(24.0);
        let date_extents = self
            .context
            .text_extents(&date_str)
            .expect("Failed to get date extents");
        let date_x = (self.width as f64 - date_extents.width()) / 2.0;
        let date_y = y + extents.height() + 20.0;

        self.context.move_to(date_x, date_y);
        self.context
            .show_text(&date_str)
            .expect("Failed to draw date");
    }

    /// Draw the password indicator ring
    fn draw_indicator(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;

        // Draw outer ring
        let ring_color = self.tuple_to_color(self.config.ring_color);
        self.context.set_source_rgba(
            ring_color.r as f64 / 255.0,
            ring_color.g as f64 / 255.0,
            ring_color.b as f64 / 255.0,
            ring_color.a as f64 / 255.0 * self.fade_alpha,
        );
        self.context.set_line_width(thickness);
        self.context
            .arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
        self.context.stroke().expect("Failed to draw ring");

        // Draw inside fill
        let inside_color = self.tuple_to_color(self.config.inside_color);
        self.context.set_source_rgba(
            inside_color.r as f64 / 255.0,
            inside_color.g as f64 / 255.0,
            inside_color.b as f64 / 255.0,
            inside_color.a as f64 / 255.0 * self.fade_alpha,
        );
        self.context.arc(
            center_x,
            center_y,
            radius - thickness / 2.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        self.context.fill().expect("Failed to fill inside");

        // Draw separator line
        let separator_color = self.tuple_to_color(self.config.separator_color);
        if separator_color.a > 0 {
            self.context.set_source_rgba(
                separator_color.r as f64 / 255.0,
                separator_color.g as f64 / 255.0,
                separator_color.b as f64 / 255.0,
                separator_color.a as f64 / 255.0 * self.fade_alpha,
            );
            self.context.set_line_width(1.0);
            self.context.move_to(center_x - radius, center_y);
            self.context.line_to(center_x + radius, center_y);
            self.context.stroke().expect("Failed to draw separator");
        }
    }

    /// Draw wrong password feedback (red flash)
    fn draw_wrong_password_feedback(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;

        // Calculate flash intensity based on time
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
            self.context
                .set_source_rgba(1.0, 0.0, 0.0, intensity * self.fade_alpha);
            self.context
                .arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
            self.context
                .fill()
                .expect("Failed to draw wrong password feedback");
        }
    }

    /// Draw key highlight feedback (green flash)
    fn draw_key_highlight_feedback(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;

        // Calculate flash intensity based on time
        let intensity = if let Some(start) = self.key_highlight_start {
            let elapsed = start.elapsed();
            let duration = std::time::Duration::from_millis(200);
            if elapsed < duration {
                1.0 - (elapsed.as_secs_f64() / duration.as_secs_f64())
            } else {
                0.0
            }
        } else {
            0.0
        };

        if intensity > 0.0 {
            let key_hl_color = self.tuple_to_color(self.config.key_hl_color);
            self.context.set_source_rgba(
                key_hl_color.r as f64 / 255.0,
                key_hl_color.g as f64 / 255.0,
                key_hl_color.b as f64 / 255.0,
                key_hl_color.a as f64 / 255.0 * intensity * self.fade_alpha,
            );
            self.context
                .arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
            self.context
                .fill()
                .expect("Failed to draw key highlight feedback");
        }
    }

    /// Update feedback timers and reset expired feedback
    fn update_feedback_timers(&mut self) {
        // Check wrong password feedback timeout
        if let Some(start) = self.wrong_password_start {
            if start.elapsed() > std::time::Duration::from_millis(500) {
                self.wrong_password_shown = false;
                self.wrong_password_start = None;
            }
        }

        // Check key highlight feedback timeout
        if let Some(start) = self.key_highlight_start {
            if start.elapsed() > std::time::Duration::from_millis(200) {
                self.key_highlight_shown = false;
                self.key_highlight_start = None;
            }
        }
    }
}
