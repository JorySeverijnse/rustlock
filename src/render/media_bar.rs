use crate::render::Renderer;
use cairo::{Format, ImageSurface};

impl Renderer {
    pub(crate) fn draw_media(&mut self) {
        if let Some(ref title) = self.system_status.media_title {
            let center_x = self.width as f64 / 2.0;
            let start_y = self.height as f64 - 120.0;
            let art_size = 56.0;
            let spacing = 80.0;

            if self.config.show_album_art && self.system_status.media_art_url != self.last_art_url {
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

            if self.system_status.media_playing {
                if let Some(ref icon) = self.media_pause_icon_surface {
                    let pause_y = start_y + 40.0;
                    let rx = center_x - icon.width() as f64 / 2.0;
                    let ry = pause_y - icon.height() as f64 / 2.0;
                    self.draw_icon_at(rx, ry, icon);
                    self.media_rects.push((
                        "play_pause",
                        rx,
                        ry,
                        icon.width() as f64,
                        icon.height() as f64,
                    ));
                }
            } else if let Some(ref icon) = self.media_play_icon_surface {
                let play_y = start_y + 40.0;
                let rx = center_x - icon.width() as f64 / 2.0;
                let ry = play_y - icon.height() as f64 / 2.0;
                self.draw_icon_at(rx, ry, icon);
                self.media_rects.push((
                    "play_pause",
                    rx,
                    ry,
                    icon.width() as f64,
                    icon.height() as f64,
                ));
            }

            let controls_y = start_y + 65.0;
            let icon_size = 20.0;
            let icon_spacing = 40.0;

            if let Some(ref icon) = self.media_prev_icon_surface {
                let rx = center_x - icon_spacing - icon_size / 2.0;
                let ry = controls_y - icon_size / 2.0;
                self.draw_icon_at(rx, ry, icon);
                self.media_rects
                    .push(("prev", rx, ry, icon_size, icon_size));
            }

            if let Some(ref icon) = self.media_stop_icon_surface {
                let rx = center_x - icon_size / 2.0;
                let ry = controls_y - icon_size / 2.0;
                self.draw_icon_at(rx, ry, icon);
                self.media_rects
                    .push(("stop", rx, ry, icon_size, icon_size));
            }

            if let Some(ref icon) = self.media_next_icon_surface {
                let rx = center_x + icon_spacing - icon_size / 2.0;
                let ry = controls_y - icon_size / 2.0;
                self.draw_icon_at(rx, ry, icon);
                self.media_rects
                    .push(("next", rx, ry, icon_size, icon_size));
            }
        }
    }
}
