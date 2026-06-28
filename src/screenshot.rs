//!
//! This module provides functionality to capture the current screen contents
//! and apply visual effects like blur and vignette, similar to swaylock-effects.

use anyhow::{Context, Result};
use cairo::ImageSurface;
use log::warn;
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
            crate::effects::apply_blur(&mut self.surface, radius, times)?;
        }
        if let Some((base, factor)) = config.effect_vignette {
            crate::effects::apply_vignette(&mut self.surface, base, factor)?;
        }
        if let Some(pixel_size) = config.effect_pixelate {
            crate::effects::apply_pixelate(&mut self.surface, pixel_size)?;
        }
        if let Some(angle) = config.effect_swirl {
            crate::effects::apply_swirl(&mut self.surface, angle)?;
        }
        Ok(())
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
            warn!("zwlr_screencopy_manager_v1 not available — backgrounds will not be captured");
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
            let canvas_end = canvas.len();
            for row in 0..height {
                let src_offset = row * stride;
                let dst_offset = row * pixel_width;
                let copy_end = (src_offset + pixel_width).min(canvas_end);
                if copy_end > src_offset {
                    data[dst_offset..dst_offset + pixel_width]
                        .copy_from_slice(&canvas[src_offset..copy_end]);
                }
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
