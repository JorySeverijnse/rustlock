use crate::render::ring_shape;
use crate::render::Renderer;

impl Renderer {
    pub(crate) fn draw_indicator(&self) {
        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;
        let radius = self.config.indicator_radius as f64;
        let thickness = self.config.indicator_thickness as f64;
        let shape = self.config.ring_shape;

        // Filled center
        self.context.new_path();
        let (r, g, b, a) = self.config.inside_color;
        self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
        ring_shape::build_fill_path(
            &self.context,
            center_x,
            center_y,
            radius - thickness / 2.0,
            thickness,
            shape,
        );
        render_try!(self.context.fill());

        // Separator line behind the ring
        let (lr, lg, lb, la) = self.config.line_color;
        if la > 0.0 {
            self.context.new_path();
            self.context
                .set_source_rgba(lr, lg, lb, la * self.fade_alpha);
            self.context.set_line_width(1.0);
            ring_shape::build_ring_path(
                &self.context,
                center_x,
                center_y,
                radius - thickness / 2.0,
                shape,
            );
            render_try!(self.context.stroke());
        }

        // Outer ring
        let (r, g, b, a) = if self.caps_lock {
            self.config.caps_lock_color
        } else {
            self.config.ring_color
        };
        self.context.new_path();
        self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
        self.context.set_line_width(thickness);
        self.context.set_line_join(cairo::LineJoin::Round);
        ring_shape::build_ring_path(&self.context, center_x, center_y, radius, shape);
        render_try!(self.context.stroke());

        // Separator line through center
        let (r, g, b, a) = self.config.separator_color;
        if a > 0.0 {
            self.context.new_path();
            self.context.set_source_rgba(r, g, b, a * self.fade_alpha);
            self.context.set_line_width(1.0);
            self.context.move_to(center_x - radius, center_y);
            self.context.line_to(center_x + radius, center_y);
            render_try!(self.context.stroke());
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
        let shape = self.config.ring_shape;

        let max_dots = self.config.max_dots as f64;
        let dot_radius = radius - thickness - 10.0;
        let t_offset = ring_shape::top_centre_offset(shape);

        let count = self.password_display.len();
        if count == 0 {
            return;
        }

        if self.peeking {
            // Draw each character at the same ring-perimeter positions as dots
            self.context.set_font_size(14.0);
            self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);
            for (i, ch) in self.password_display.chars().enumerate() {
                let t = (i as f64 / max_dots) + t_offset;
                let (x, y) = ring_shape::perimeter_point(center_x, center_y, dot_radius, shape, t);

                let mut ch_str = String::new();
                ch_str.push(ch);
                let te = render_try!(self.context.text_extents(&ch_str));
                self.context.new_path();
                self.context.move_to(
                    x - te.x_bearing() - te.width() / 2.0,
                    y - te.y_bearing() - te.height() / 2.0,
                );
                render_try!(self.context.show_text(&ch_str));
            }
        } else {
            self.context.new_path();
            self.context.set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha);

            for i in 0..count {
                let t = (i as f64 / max_dots) + t_offset;
                let (x, y) = ring_shape::perimeter_point(center_x, center_y, dot_radius, shape, t);

                self.context.new_path();
                self.context.arc(x, y, 4.0, 0.0, 2.0 * std::f64::consts::PI);
                render_try!(self.context.fill());
            }
        }

        // Cursor indicator (shared between peek and dot modes).
        if self.fade_alpha > 0.0 {
            // At position 0, place the cursor squarely at the first-dot position
            // (t_offset) instead of subtracting 0.5, which would push t negative
            // and cause perimeter_point to wrap it to the far end of the ring.
            let cursor_t = if self.cursor_position == 0 {
                t_offset
            } else {
                ((self.cursor_position as f64 - 0.5) / max_dots) + t_offset
            };
            let (cx, cy) =
                ring_shape::perimeter_point(center_x, center_y, dot_radius, shape, cursor_t);
            let dx = cx - center_x;
            let dy = cy - center_y;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let nx = dx / len;
            let ny = dy / len;

            let x1 = cx - 8.0 * nx;
            let y1 = cy - 8.0 * ny;
            let x2 = cx + 8.0 * nx;
            let y2 = cy + 8.0 * ny;

            self.context.new_path();
            self.context.set_source_rgba(0.0, 0.8, 1.0, self.fade_alpha);
            self.context.set_line_width(2.0);
            self.context.move_to(x1, y1);
            self.context.line_to(x2, y2);
            render_try!(self.context.stroke());
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
        let te = render_try!(self.context.text_extents(text));
        self.context
            .move_to(center_x - te.width() / 2.0, center_y - radius - 10.0);
        render_try!(self.context.show_text(text));
    }
}
