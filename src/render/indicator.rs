use crate::render::Renderer;

impl Renderer {
    pub(crate) fn draw_indicator(&self) {
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

    pub(crate) fn draw_password_display(&self) {
        if self.config.hide_password {
            return;
        }
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;

        self.context.new_path();
        self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);

        let count = self.password_display.len();
        if count == 0 {
            return;
        }

        let dot_radius = radius - thickness - 10.0;
        let angle_step = (360.0 / 24.0_f64).to_radians();

        for i in 0..count {
            let angle = (i as f64 * angle_step) - std::f64::consts::FRAC_PI_2;
            let x = center_x + dot_radius * angle.cos();
            let y = center_y + dot_radius * angle.sin();

            self.context.new_path();
            self.context.arc(x, y, 4.0, 0.0, 2.0 * std::f64::consts::PI);
            self.context.fill().unwrap();
        }

        if self.fade_alpha > 0.0 && self.cursor_position > 0 {
            let cursor_angle =
                ((self.cursor_position as f64 - 0.5) * angle_step) - std::f64::consts::FRAC_PI_2;
            let x1 = center_x + (dot_radius - 8.0) * cursor_angle.cos();
            let y1 = center_y + (dot_radius - 8.0) * cursor_angle.sin();
            let x2 = center_x + (dot_radius + 8.0) * cursor_angle.cos();
            let y2 = center_y + (dot_radius + 8.0) * cursor_angle.sin();

            self.context.new_path();
            self.context.set_source_rgba(0.0, 0.8, 1.0, self.fade_alpha);
            self.context.set_line_width(2.0);
            self.context.move_to(x1, y1);
            self.context.line_to(x2, y2);
            self.context.stroke().unwrap();
        }
    }

    pub(crate) fn draw_caps_lock_indicator(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        self.context.new_path();
        let (r, g, b, a) = self.config.caps_lock_text_color;
        self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
        self.context.set_font_size(24.0);
        let text = "Caps Lock";
        let te = self.context.text_extents(text).unwrap();
        self.context
            .move_to(center_x - te.width() / 2.0, center_y - radius - 10.0);
        self.context.show_text(text).unwrap();
    }
}
