use crate::render::Renderer;
use cairo::{Format, ImageSurface};

impl Renderer {
    pub(crate) fn load_icons(&mut self) {
        log::info!("Attempting to load status icons...");
        let wifi_names = [
            "network-wireless-signal-excellent-symbolic",
            "network-wireless-signal-excellent",
            "network-wireless-symbolic",
            "network-wireless",
        ];
        let wifi_path = self
            .config
            .wifi_icon
            .clone()
            .or_else(|| {
                for name in &wifi_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();

        if !wifi_path.is_empty() {
            log::info!("Resolved WiFi icon path: {}", wifi_path);
            self.wifi_icon_surface = self.load_icon(&wifi_path);
        }

        let bt_names = [
            "bluetooth-active-symbolic",
            "bluetooth-symbolic",
            "bluetooth-active",
            "bluetooth",
        ];
        let bt_path = self
            .config
            .bluetooth_icon
            .clone()
            .or_else(|| {
                for name in &bt_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();

        if !bt_path.is_empty() {
            log::info!("Resolved Bluetooth icon path: {}", bt_path);
            self.bluetooth_icon_surface = self.load_icon(&bt_path);
        }

        let batt_names = [
            "battery-level-100-symbolic",
            "battery-full-symbolic",
            "battery-full",
            "battery-level-100",
            "battery",
            "battery-symbolic",
        ];
        let batt_path = self
            .config
            .battery_icon
            .clone()
            .or_else(|| {
                for name in &batt_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();

        if !batt_path.is_empty() {
            log::info!("Resolved Battery icon path: {}", batt_path);
            self.battery_icon_surface = self.load_icon(&batt_path);
        }

        let prev_names = [
            "media-skip-backward-symbolic",
            "media-skip-backward",
            "media-playlist-repeat-symbolic",
        ];
        let prev_path = self
            .config
            .media_prev_icon
            .clone()
            .or_else(|| {
                for name in &prev_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();
        if !prev_path.is_empty() {
            self.media_prev_icon_surface = self.load_icon(&prev_path);
        }

        let stop_names = ["media-playback-stop-symbolic", "media-playback-stop"];
        let stop_path = self
            .config
            .media_stop_icon
            .clone()
            .or_else(|| {
                for name in &stop_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();
        if !stop_path.is_empty() {
            self.media_stop_icon_surface = self.load_icon(&stop_path);
        }

        let play_names = ["media-playback-start-symbolic", "media-playback-start"];
        let play_path = self
            .config
            .media_play_icon
            .clone()
            .or_else(|| {
                for name in &play_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();
        if !play_path.is_empty() {
            self.media_play_icon_surface = self.load_icon(&play_path);
        }

        let pause_names = ["media-playback-pause-symbolic", "media-playback-pause"];
        let pause_path = self
            .config
            .media_pause_icon
            .clone()
            .or_else(|| {
                for name in &pause_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();
        if !pause_path.is_empty() {
            self.media_pause_icon_surface = self.load_icon(&pause_path);
        }

        let next_names = ["media-skip-forward-symbolic", "media-skip-forward"];
        let next_path = self
            .config
            .media_next_icon
            .clone()
            .or_else(|| {
                for name in &next_names {
                    if let Some(path) = self.find_system_icon(name) {
                        return Some(path);
                    }
                }
                None
            })
            .unwrap_or_default();
        if !next_path.is_empty() {
            self.media_next_icon_surface = self.load_icon(&next_path);
        }
    }

    pub(crate) fn find_system_icon(&self, name: &str) -> Option<String> {
        let data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
        let mut search_paths = Vec::new();

        for dir in data_dirs.split(':') {
            let p = std::path::PathBuf::from(dir).join("icons");
            if p.exists() {
                search_paths.push(p);
            }
        }
        let sys_path = std::path::PathBuf::from("/run/current-system/sw/share/icons");
        if sys_path.exists() {
            search_paths.push(sys_path);
        }
        let usr_path = std::path::PathBuf::from("/usr/share/icons");
        if usr_path.exists() {
            search_paths.push(usr_path);
        }

        let themes = [
            "WhiteSur",
            "WhiteSur-dark",
            "WhiteSur-light",
            "Adwaita",
            "hicolor",
            "breeze",
            "Papirus",
        ];
        let categories = [
            "status/symbolic",
            "actions/symbolic",
            "devices/symbolic",
            "status/24",
            "status/22",
            "status/16",
            "status",
            "actions",
            "devices",
            "symbolic/status",
            "symbolic/actions",
            "symbolic/devices",
            "24x24/status",
            "22x22/status",
            "16x16/status",
            "48x48/status",
        ];

        for base in &search_paths {
            for theme in &themes {
                for cat in &categories {
                    for ext in [".svg", ".png"] {
                        let icon_path = base.join(theme).join(cat).join(format!("{}{}", name, ext));
                        if icon_path.exists() {
                            return Some(icon_path.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }

        for base in &search_paths {
            for theme in &themes {
                let theme_root = base.join(theme);
                if !theme_root.exists() {
                    continue;
                }

                if let Ok(entries) = std::fs::read_dir(&theme_root) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            for ext in [".svg", ".png"] {
                                let icon_path = entry.path().join(format!("{}{}", name, ext));
                                if icon_path.exists() {
                                    return Some(icon_path.to_string_lossy().into_owned());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub(crate) fn load_icon(&self, identifier: &str) -> Option<ImageSurface> {
        let path = if identifier.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(identifier.replacen('~', &home, 1))
        } else {
            std::path::PathBuf::from(identifier)
        };

        if !path.exists() {
            return None;
        }

        if path.extension().and_then(|s| s.to_str()) == Some("svg") {
            if let Some(surface) = self.load_svg_with_resvg(&path) {
                return Some(surface);
            }
        }

        match gdk_pixbuf::Pixbuf::from_file(&path) {
            Ok(pixbuf) => {
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
            }
            Err(_) => None,
        }
    }

    pub(crate) fn load_svg_with_resvg(&self, path: &std::path::Path) -> Option<ImageSurface> {
        use resvg::usvg;
        let opt = usvg::Options::default();
        let svg_data = std::fs::read(path).ok()?;
        let tree = usvg::Tree::from_data(&svg_data, &opt).ok()?;
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::default(),
            &mut pixmap.as_mut(),
        );
        let mut surface =
            ImageSurface::create(Format::ARgb32, size.width() as i32, size.height() as i32).ok()?;
        {
            let mut surface_data = surface.data().ok()?;
            let pix_data = pixmap.data();
            for i in (0..pix_data.len()).step_by(4) {
                surface_data[i] = pix_data[i + 2];
                surface_data[i + 1] = pix_data[i + 1];
                surface_data[i + 2] = pix_data[i];
                surface_data[i + 3] = pix_data[i + 3];
            }
        }
        Some(surface)
    }

    pub(crate) fn draw_clock(&self) {
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

    pub(crate) fn draw_network(&self) {
        if !self.config.show_network {
            return;
        }
        let margin = 20.0;
        let x = margin;
        let y = margin + 20.0;

        if let Some(ref ssid) = self.system_status.wifi_ssid {
            if let Some(ref icon) = self.wifi_icon_surface {
                self.draw_icon_at(x, y - 15.0, icon);
                let text_x = x + 24.0 + 10.0;
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
        }
    }

    pub(crate) fn draw_status(&self) {
        if let Some(percent) = self.system_status.battery_percent {
            let margin = 20.0;
            let icon_width = 30.0;
            let x = self.width as f64 - margin - icon_width - 50.0;
            let y = margin + 20.0;

            if let Some(ref icon) = self.battery_icon_surface {
                self.draw_icon_at(x, y - 15.0, icon);
                let text_x = x + 24.0 + 10.0;
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

    pub(crate) fn draw_bluetooth(&self) {
        if !self.config.show_bluetooth {
            return;
        }
        let margin = 20.0;
        let x = margin;
        let y = margin + 50.0;

        let (status_text, is_off) = if self.system_status.bluetooth_connected {
            (self.system_status.bluetooth_devices.join(", "), false)
        } else {
            ("Bluetooth off".to_string(), true)
        };

        let alpha_mult = if is_off { 0.5 } else { 1.0 };

        if let Some(ref icon) = self.bluetooth_icon_surface {
            self.draw_icon_at_with_alpha(x, y - 12.0, icon, alpha_mult);
            let text_x = x + 24.0 + 10.0;
            self.context.new_path();
            self.context
                .set_source_rgba(1.0, 1.0, 1.0, self.fade_alpha * alpha_mult);
            self.context.set_font_size(14.0);
            self.context.move_to(text_x, y);
            self.context.show_text(&status_text).unwrap();
        }
    }

    pub(crate) fn draw_keyboard_layout(&self) {
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

    pub(crate) fn draw_icon_at(&self, x: f64, y: f64, surface: &ImageSurface) {
        self.context.save().unwrap();
        let target_size = 24.0;
        let scale =
            (target_size / surface.width() as f64).min(target_size / surface.height() as f64);
        self.context.translate(x, y);
        self.context.scale(scale, scale);
        self.context.set_source_surface(surface, 0.0, 0.0).unwrap();
        self.context.paint_with_alpha(self.fade_alpha).unwrap();
        self.context.restore().unwrap();
    }

    pub(crate) fn draw_icon_at_with_alpha(
        &self,
        x: f64,
        y: f64,
        surface: &ImageSurface,
        alpha: f64,
    ) {
        self.context.save().unwrap();
        let target_size = 24.0;
        let scale =
            (target_size / surface.width() as f64).min(target_size / surface.height() as f64);
        self.context.translate(x, y);
        self.context.scale(scale, scale);
        self.context.set_source_surface(surface, 0.0, 0.0).unwrap();
        self.context
            .paint_with_alpha(self.fade_alpha * alpha)
            .unwrap();
        self.context.restore().unwrap();
    }

    pub(crate) fn draw_battery_icon_at(
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
