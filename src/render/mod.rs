use cairo::{Context, Format, ImageSurface};
use std::time::Instant;

use crate::config::Config;
use crate::system::SystemStatus;

mod feedback;
mod indicator;
mod media_bar;
mod status_bar;

pub struct Renderer {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) config: Config,
    pub(crate) surface: ImageSurface,
    pub(crate) context: Context,
    pub(crate) fade_alpha: f64,
    pub(crate) wrong_password_shown: bool,
    pub(crate) key_highlight_shown: bool,
    pub(crate) cleared_feedback_shown: bool,
    pub(crate) wrong_password_start: Option<Instant>,
    pub(crate) key_highlight_start: Option<Instant>,
    pub(crate) cleared_feedback_start: Option<Instant>,
    pub(crate) key_highlight_angle: f64,
    pub(crate) background: Option<ImageSurface>,
    pub(crate) password_display: String,
    pub(crate) cursor_position: usize,
    pub(crate) uptime_cache: String,
    pub(crate) last_uptime_update: Option<Instant>,
    pub caps_lock: bool,
    pub system_status: SystemStatus,
    pub(crate) media_art_surface: Option<ImageSurface>,
    pub(crate) last_art_url: Option<String>,
    pub(crate) wifi_icon_surface: Option<ImageSurface>,
    pub(crate) bluetooth_icon_surface: Option<ImageSurface>,
    pub(crate) battery_icon_surface: Option<ImageSurface>,
    pub(crate) media_prev_icon_surface: Option<ImageSurface>,
    pub(crate) media_stop_icon_surface: Option<ImageSurface>,
    pub(crate) media_play_icon_surface: Option<ImageSurface>,
    pub(crate) media_pause_icon_surface: Option<ImageSurface>,
    pub(crate) media_next_icon_surface: Option<ImageSurface>,
    pub media_rects: Vec<(&'static str, f64, f64, f64, f64)>,
}

impl Renderer {
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
            cleared_feedback_shown: false,
            wrong_password_start: None,
            key_highlight_start: None,
            cleared_feedback_start: None,
            key_highlight_angle: 0.0,
            background: None,
            password_display: String::new(),
            cursor_position: 0,
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
            media_pause_icon_surface: None,
            media_next_icon_surface: None,
            media_rects: Vec::new(),
        };

        renderer.load_icons();
        renderer
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        log::debug!("Renderer::resize({}, {}) called", width, height);
        self.width = width;
        self.height = height;

        self.surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Failed to create Cairo surface");
        self.context = Context::new(&self.surface).expect("Failed to create Cairo context");
    }

    pub fn set_background(&mut self, background: ImageSurface) {
        self.background = Some(background);
    }

    pub fn set_fade_alpha(&mut self, alpha: f64) {
        self.fade_alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn set_password_display(&mut self, length: usize) {
        self.password_display = ".".repeat(length);
    }

    pub fn set_cursor_position(&mut self, position: usize) {
        self.cursor_position = position;
    }

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

    pub fn render(&mut self) {
        self.media_rects.clear();
        self.context.new_path();
        self.context.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        self.context.paint().expect("Failed to clear surface");

        if let Some(ref background) = self.background {
            self.context.save().expect("Failed to save context");
            let bg_width = background.width() as f64;
            let bg_height = background.height() as f64;
            let scale_x = self.width as f64 / bg_width;
            let scale_y = self.height as f64 / bg_height;
            let scale = scale_x.max(scale_y);

            let offset_x = (self.width as f64 - bg_width * scale) / 2.0;
            let offset_y = (self.height as f64 - bg_height * scale) / 2.0;

            self.context.translate(offset_x, offset_y);
            self.context.scale(scale, scale);
            self.context.new_path();
            self.context
                .set_source_surface(background, 0.0, 0.0)
                .expect("Failed to set source");
            self.context
                .paint_with_alpha(self.fade_alpha)
                .expect("Failed to paint");
            self.context.restore().expect("Failed to restore context");
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

        if self.cleared_feedback_shown {
            self.draw_cleared_feedback();
        }

        self.update_feedback_timers();
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
}
