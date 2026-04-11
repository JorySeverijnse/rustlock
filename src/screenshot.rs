//!
//! This module provides functionality to capture the current screen contents
//! and apply visual effects like blur and vignette, similar to swaylock-effects.

use anyhow::{Context, Result};
use cairo::ImageSurface;
use log::debug;
use smithay_client_toolkit::shm::{slot::Buffer, slot::SlotPool};
use std::sync::Mutex;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_output, wl_shm};
use wayland_client::{Dispatch, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    Flags, ZwlrScreencopyFrameV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::config::Config;

/// A captured screenshot with optional visual effects applied.
pub struct Screenshot {
    surface: ImageSurface,
}

impl Screenshot {
    /// Create a new screenshot from a Cairo surface.
    pub fn new(surface: ImageSurface) -> Self {
        Self { surface }
    }

    /// Consume the screenshot and return the underlying Cairo surface.
    pub fn into_inner(self) -> ImageSurface {
        self.surface
    }

    /// Apply configured visual effects to the screenshot.
    pub fn apply_effects(&mut self, config: &Config) -> Result<()> {
        if let Some((radius, times)) = config.effect_blur {
            self.apply_blur(radius, times)?;
        }
        if let Some((base, factor)) = config.effect_vignette {
            self.apply_vignette(base, factor);
        }
        if let Some(pixel_size) = config.effect_pixelate {
            self.apply_pixelate(pixel_size);
        }
        if let Some(angle) = config.effect_swirl {
            self.apply_swirl(angle);
        }
        if let Some(factor) = config.effect_melting {
            self.apply_melting(factor);
        }
        Ok(())
    }

    /// Apply a swirl effect.
    pub fn apply_swirl(&mut self, angle: f32) {
        let width = self.surface.width();
        let height = self.surface.height();
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let radius = center_x.min(center_y);

        let stride = self.surface.stride() as usize;
        let mut data = vec![0u8; stride * height as usize];
        self.surface
            .with_data(|src| data.copy_from_slice(src))
            .unwrap();
        let original = data.clone();

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let d = (dx * dx + dy * dy).sqrt();

                if d < radius {
                    let percent = (radius - d) / radius;
                    let theta = percent * percent * angle;
                    let s = theta.sin();
                    let c = theta.cos();

                    let nx = (c * dx - s * dy + center_x) as i32;
                    let ny = (s * dx + c * dy + center_y) as i32;

                    if nx >= 0 && nx < width && ny >= 0 && ny < height {
                        let src_idx = (ny as usize * stride) + (nx as usize * 4);
                        let dst_idx = (y as usize * stride) + (x as usize * 4);
                        data[dst_idx..dst_idx + 4].copy_from_slice(&original[src_idx..src_idx + 4]);
                    }
                }
            }
        }

        let mut surface_data = self.surface.data().unwrap();
        surface_data.copy_from_slice(&data);
    }

    /// Apply a melting effect (vertical smear).
    pub fn apply_melting(&mut self, factor: f32) {
        let width = self.surface.width();
        let height = self.surface.height();

        let stride = self.surface.stride() as usize;
        let mut data = vec![0u8; stride * height as usize];
        self.surface
            .with_data(|src| data.copy_from_slice(src))
            .unwrap();

        use rand::RngExt;
        let mut rng = rand::rng();

        for x in 0..width {
            let mut melt_amount = 0.0;
            for y in 0..height {
                melt_amount += rng.random_range(0.0..factor);
                let src_y = (y as f32 - melt_amount).max(0.0) as i32;

                let src_idx = (src_y as usize * stride) + (x as usize * 4);
                let dst_idx = (y as usize * stride) + (x as usize * 4);

                // Copy the pixel from above to create a smear
                let pixel = [
                    data[src_idx],
                    data[src_idx + 1],
                    data[src_idx + 2],
                    data[src_idx + 3],
                ];
                data[dst_idx..dst_idx + 4].copy_from_slice(&pixel);
            }
        }

        let mut surface_data = self.surface.data().unwrap();
        surface_data.copy_from_slice(&data);
    }

    /// Pixelate the surface.
    pub fn apply_pixelate(&mut self, pixel_size: u32) {
        if pixel_size <= 1 {
            return;
        }

        let width = self.surface.width();
        let height = self.surface.height();
        let stride = self.surface.stride() as usize;
        let mut data = vec![0u8; stride * height as usize];
        self.surface
            .with_data(|src| data.copy_from_slice(src))
            .unwrap();

        for y in (0..height).step_by(pixel_size as usize) {
            for x in (0..width).step_by(pixel_size as usize) {
                let mut r = 0u32;
                let mut g = 0u32;
                let mut b = 0u32;
                let mut count = 0u32;

                // Average pixels in the block
                for py in 0..pixel_size {
                    for px in 0..pixel_size {
                        let cur_x = x + px as i32;
                        let cur_y = y + py as i32;
                        if cur_x < width && cur_y < height {
                            let index = (cur_y as usize * stride) + (cur_x as usize * 4);
                            r += data[index] as u32;
                            g += data[index + 1] as u32;
                            b += data[index + 2] as u32;
                            count += 1;
                        }
                    }
                }

                if count > 0 {
                    let r = (r / count) as u8;
                    let g = (g / count) as u8;
                    let b = (b / count) as u8;

                    // Fill the block
                    for py in 0..pixel_size {
                        for px in 0..pixel_size {
                            let cur_x = x + px as i32;
                            let cur_y = y + py as i32;
                            if cur_x < width && cur_y < height {
                                let index = (cur_y as usize * stride) + (cur_x as usize * 4);
                                data[index] = r;
                                data[index + 1] = g;
                                data[index + 2] = b;
                            }
                        }
                    }
                }
            }
        }

        let mut surface_data = self.surface.data().unwrap();
        surface_data.copy_from_slice(&data);
    }

    /// Apply a Gaussian blur effect.
    pub fn apply_blur(&mut self, radius: u32, times: u32) -> Result<()> {
        if radius == 0 || times == 0 {
            return Ok(());
        }

        let width = self.surface.width();
        let height = self.surface.height();
        let stride = self.surface.stride() as usize;
        let mut data = vec![0u8; stride * height as usize];

        self.surface
            .with_data(|src| data.copy_from_slice(src))
            .context("Failed to get surface data")?;

        // Convert to image::RgbaImage for processing
        let mut img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_raw(width as u32, height as u32, data)
                .context("Failed to create image buffer")?;

        for _ in 0..times {
            let mut rgb_data: Vec<[u8; 3]> =
                Vec::with_capacity((width as usize) * (height as usize));
            for pixel in img.pixels() {
                rgb_data.push([pixel[0], pixel[1], pixel[2]]);
            }

            fastblur::gaussian_blur(
                &mut rgb_data,
                width as usize,
                height as usize,
                radius as f32,
            );

            for (i, pixel) in img.pixels_mut().enumerate() {
                pixel[0] = rgb_data[i][0];
                pixel[1] = rgb_data[i][1];
                pixel[2] = rgb_data[i][2];
            }
        }

        // Copy back to surface
        let new_data = img.into_raw();
        let mut surface_data = self.surface.data()?;
        surface_data.copy_from_slice(&new_data);
        Ok(())
    }

    /// Apply a vignette effect (darken edges).
    pub fn apply_vignette(&mut self, base: f32, factor: f32) {
        let width = self.surface.width();
        let height = self.surface.height();
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let max_distance = (center_x * center_x + center_y * center_y).sqrt();

        let stride = self.surface.stride() as usize;
        let mut data = vec![0u8; stride * height as usize];
        self.surface
            .with_data(|src| data.copy_from_slice(src))
            .unwrap();

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                let vignette_factor = base + (1.0 - base) * (distance / max_distance).powf(factor);

                let index = ((y * width + x) * 4) as usize;
                for i in 0..3 {
                    let value = data[index + i] as f32 * vignette_factor;
                    data[index + i] = value.clamp(0.0, 255.0) as u8;
                }
            }
        }

        let mut surface_data = self.surface.data().unwrap();
        surface_data.copy_from_slice(&data);
    }
}

#[derive(Clone)]
/// Information about a buffer from the screencopy protocol.
pub struct BufferInfo {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: wl_shm::Format,
}

/// Handle to a captured buffer that can be converted to a Cairo surface.
pub struct ScreencopyBufferHandle {
    pub buffer: Buffer,
    pub info: BufferInfo,
    pub y_invert: bool,
}

/// Manager for the wlr-screencopy protocol.
pub struct ScreenshotManager {
    manager: Option<ZwlrScreencopyManagerV1>,
}

impl ScreenshotManager {
    /// Bind to the wlr-screencopy global and create a new manager.
    ///
    /// Returns `Ok(Self)` if the protocol is available, otherwise `Err`.
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Result<Self>
    where
        D: Dispatch<ZwlrScreencopyManagerV1, ()> + 'static,
    {
        let manager = globals
            .bind::<ZwlrScreencopyManagerV1, _, _>(qh, 1..=3, ())
            .ok();

        if manager.is_none() {
            debug!("zwlr_screencopy_manager_v1 not available");
        }

        Ok(Self { manager })
    }

    /// Initiate a screencopy operation for the given output.
    ///
    /// This method sends a screencopy request and returns the frame object.
    /// The frame events will be dispatched to the provided queue's dispatcher
    /// with the given user data.
    pub fn capture_output<D>(
        &self,
        output: &wl_output::WlOutput,
        qh: &QueueHandle<D>,
        user_data: CaptureData,
    ) -> Result<ZwlrScreencopyFrameV1>
    where
        D: Dispatch<ZwlrScreencopyFrameV1, CaptureData> + 'static,
    {
        let manager = self.manager.as_ref().context("Screencopy not available")?;
        let frame = manager.capture_output(0, output, qh, user_data);
        Ok(frame)
    }

    /// Convert a captured buffer to a Cairo ImageSurface.
    pub fn buffer_to_surface(
        &self,
        handle: ScreencopyBufferHandle,
        pool: &mut SlotPool,
    ) -> Result<ImageSurface> {
        let info = handle.info;
        let y_invert = handle.y_invert;

        let canvas = handle
            .buffer
            .canvas(pool)
            .context("Failed to get buffer canvas")?;

        let pixel_width = (info.width * 4) as usize;
        let stride = info.stride as usize;
        let height = info.height as usize;

        if stride < pixel_width {
            anyhow::bail!("Stride smaller than pixel width");
        }

        let raw_data = {
            let mut data = vec![0u8; (info.width * info.height * 4) as usize];
            for row in 0..height {
                let src_offset = row * stride;
                let dst_offset = row * pixel_width;
                data[dst_offset..dst_offset + pixel_width]
                    .copy_from_slice(&canvas[src_offset..src_offset + pixel_width]);
            }
            data
        };

        let converted_data = match info.format {
            wayland_client::protocol::wl_shm::Format::Argb8888 => raw_data,
            wayland_client::protocol::wl_shm::Format::Xbgr8888 => {
                convert_xbgr8888_to_argb32(&raw_data, info.width as usize, info.height as usize)
            }
            wayland_client::protocol::wl_shm::Format::Xrgb8888 => {
                convert_xrgb8888_to_argb32(&raw_data, info.width as usize, info.height as usize)
            }
            _ => {
                log::warn!("Unsupported format {:?}, using raw data as-is", info.format);
                raw_data
            }
        };

        if y_invert {
            let mut flipped = vec![0u8; (info.width * info.height * 4) as usize];
            let src_stride = (info.width * 4) as usize;
            for row in 0..height {
                let src_row = height - 1 - row;
                let src_offset = src_row * src_stride;
                let dst_offset = row * src_stride;
                flipped[dst_offset..dst_offset + src_stride]
                    .copy_from_slice(&converted_data[src_offset..src_offset + src_stride]);
            }
            return ImageSurface::create_for_data(
                flipped,
                cairo::Format::ARgb32,
                info.width as i32,
                info.height as i32,
                src_stride as i32,
            )
            .context("Failed to create flipped Cairo surface");
        }

        ImageSurface::create_for_data(
            converted_data,
            cairo::Format::ARgb32,
            info.width as i32,
            info.height as i32,
            pixel_width as i32,
        )
        .context("Failed to create Cairo surface")
    }
}

/// Convert Xbgr8888 buffer data to ARGB32 format (little-endian byte order).
/// Xbgr8888: 32-bit word 0xXXBBGGRR, memory layout: [R, G, B, X]
/// ARGB32: 32-bit word 0xAARRGGBB, memory layout: [B, G, R, A]
fn convert_xbgr8888_to_argb32(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(width * height * 4);
    for i in 0..width * height {
        let src = i * 4;
        // Source: [R, G, B, X] -> Destination: [B, G, R, A=255]
        result.push(data[src + 2]); // B
        result.push(data[src + 1]); // G
        result.push(data[src]); // R
        result.push(255); // A
    }
    result
}

/// Convert Xrgb8888 buffer data to ARGB32 format (little-endian byte order).
/// Xrgb8888: 32-bit word 0xXXRRGGBB, memory layout: [B, G, R, X]
/// ARGB32: 32-bit word 0xAARRGGBB, memory layout: [B, G, R, A]
fn convert_xrgb8888_to_argb32(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(width * height * 4);
    for i in 0..width * height {
        let src = i * 4;
        // Source: [B, G, R, X] -> Destination: [B, G, R, A=255]
        result.push(data[src]); // B
        result.push(data[src + 1]); // G
        result.push(data[src + 2]); // R
        result.push(255); // A
    }
    result
}

/// User data associated with a screencopy frame request.
///
/// Stores intermediate data needed to assemble the final screenshot once
/// all frame events are received.
pub struct CaptureData {
    pub output_idx: usize,
    pub info: Mutex<Option<BufferInfo>>,
    pub flags: Mutex<Option<Flags>>,
    pub buffer: Mutex<Option<Buffer>>,
    pub pool: Mutex<Option<SlotPool>>,
}

impl CaptureData {
    /// Create new capture data for the given output index.
    pub fn new(output_idx: usize) -> Self {
        Self {
            output_idx,
            info: Mutex::new(None),
            flags: Mutex::new(None),
            buffer: Mutex::new(None),
            pool: Mutex::new(None),
        }
    }
}
