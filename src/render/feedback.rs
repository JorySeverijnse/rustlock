use crate::render::Renderer;
use std::time::Instant;

impl Renderer {
    pub(crate) fn draw_wrong_password_feedback(&self) {
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

    pub(crate) fn draw_key_highlight_feedback(&self) {
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

    pub(crate) fn draw_cleared_feedback(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;
        let intensity = if let Some(start) = self.cleared_feedback_start {
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
                .set_source_rgba(1.0, 0.0, 0.0, intensity * self.fade_alpha * 0.5);
            self.context.arc(
                center_x,
                center_y,
                radius - thickness / 2.0,
                0.0,
                2.0 * std::f64::consts::PI,
            );
            self.context.fill().unwrap();

            self.context.new_path();
            self.context
                .set_source_rgba(1.0, 0.0, 0.0, intensity * self.fade_alpha);
            self.context.set_line_width(thickness + 4.0);
            self.context
                .arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
            self.context.stroke().unwrap();

            self.context.new_path();
            self.context.set_font_size(24.0);
            self.context
                .set_source_rgba(1.0, 1.0, 1.0, intensity * self.fade_alpha);
            let text = "CLEARED";
            let te = self.context.text_extents(text).unwrap();
            self.context
                .move_to(center_x - te.width() / 2.0, center_y - radius - 20.0);
            self.context.show_text(text).unwrap();
        }
    }

    pub(crate) fn update_feedback_timers(&mut self) {
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
        if let Some(start) = self.cleared_feedback_start {
            if start.elapsed() > std::time::Duration::from_millis(500) {
                self.cleared_feedback_shown = false;
                self.cleared_feedback_start = None;
            }
        }
    }

    pub fn show_wrong_password(&mut self) {
        self.wrong_password_shown = true;
        self.wrong_password_start = Some(Instant::now());
    }

    pub fn show_key_highlight(&mut self) {
        self.key_highlight_shown = true;
        self.key_highlight_start = Some(Instant::now());
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let random_val = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.key_highlight_angle = ((random_val % 360) as f64).to_radians();
    }

    pub fn show_cleared_feedback(&mut self) {
        self.cleared_feedback_shown = true;
        self.cleared_feedback_start = Some(Instant::now());
    }
}
