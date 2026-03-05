pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Screenshot {
    pub fn capture(
        _output: wayland_client::protocol::wl_output::WlOutput,
        width: i32,
        height: i32,
    ) -> Result<Self, String> {
        let width = width as u32;
        let height = height as u32;
        let size = (width * height * 4) as usize;
        let mut data = vec![0u8; size];

        for i in 0..(width * height) as usize {
            let offset = i * 4;
            data[offset] = 40;
            data[offset + 1] = 44;
            data[offset + 2] = 52;
            data[offset + 3] = 255;
        }

        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn apply_blur(&mut self, radius: u32, times: u32) {
        if radius == 0 || times == 0 {
            return;
        }

        let mut img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_raw(self.width, self.height, self.data.clone())
                .expect("Failed to create image buffer");

        for _ in 0..times {
            let mut rgb_data: Vec<[u8; 3]> =
                Vec::with_capacity((self.width * self.height) as usize);
            for pixel in img.pixels() {
                rgb_data.push([pixel[0], pixel[1], pixel[2]]);
            }

            fastblur::gaussian_blur(
                &mut rgb_data,
                self.width as usize,
                self.height as usize,
                radius as f32,
            );

            for (i, pixel) in img.pixels_mut().enumerate() {
                pixel[0] = rgb_data[i][0];
                pixel[1] = rgb_data[i][1];
                pixel[2] = rgb_data[i][2];
            }
        }

        self.data = img.into_raw();
    }

    pub fn apply_vignette(&mut self, base: f32, factor: f32) {
        let center_x = self.width as f32 / 2.0;
        let center_y = self.height as f32 / 2.0;
        let max_distance = (center_x * center_x + center_y * center_y).sqrt();

        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                let vignette_factor = base + (1.0 - base) * (distance / max_distance).powf(factor);

                let index = ((y * self.width + x) * 4) as usize;
                for i in 0..3 {
                    let value = self.data[index + i] as f32 * vignette_factor;
                    self.data[index + i] = value.clamp(0.0, 255.0) as u8;
                }
            }
        }
    }

    pub fn as_image_surface(&self) -> cairo::ImageSurface {
        let surface = cairo::ImageSurface::create(
            cairo::Format::ARgb32,
            self.width as i32,
            self.height as i32,
        )
        .expect("Failed to create image surface");

        // TODO: Properly copy pixel data to surface using cairo API
        // For now, return empty surface
        surface
    }
}
