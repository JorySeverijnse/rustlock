//! Visual effects for lock screen backgrounds.
//!
//! Each effect takes a `&mut ImageSurface` and processes it in-place.
//! Effects are applied in order: blur → vignette → pixelate → swirl.

use anyhow::{Context, Result};
use cairo::ImageSurface;

/// Apply a swirl effect (radial rotation around the image centre).
pub fn apply_swirl(surface: &mut ImageSurface, angle: f32) -> Result<()> {
    let width = surface.width();
    let height = surface.height();
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = center_x.min(center_y);

    let stride = surface.stride() as usize;
    let mut data = vec![0u8; stride * height as usize];
    surface
        .with_data(|src| data.copy_from_slice(src))
        .context("swirl: failed to read surface data")?;
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

    let mut surface_data = surface
        .data()
        .context("swirl: failed to write surface data")?;
    surface_data.copy_from_slice(&data);
    Ok(())
}

/// Pixelate the surface.
pub fn apply_pixelate(surface: &mut ImageSurface, pixel_size: u32) -> Result<()> {
    if pixel_size <= 1 {
        return Ok(());
    }

    let width = surface.width();
    let height = surface.height();
    let stride = surface.stride() as usize;
    let mut data = vec![0u8; stride * height as usize];
    surface
        .with_data(|src| data.copy_from_slice(src))
        .context("pixelate: failed to read surface data")?;

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
                let r = r.checked_div(count).unwrap_or(0) as u8;
                let g = g.checked_div(count).unwrap_or(0) as u8;
                let b = b.checked_div(count).unwrap_or(0) as u8;

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

    let mut surface_data = surface
        .data()
        .context("pixelate: failed to write surface data")?;
    surface_data.copy_from_slice(&data);
    Ok(())
}

/// Apply a box blur effect (fast two-pass sliding-window implementation).
/// Uses multiply-shift to avoid slow integer division in the hot loop.
pub fn apply_blur(surface: &mut ImageSurface, radius: u32, times: u32) -> Result<()> {
    if radius == 0 || times == 0 {
        return Ok(());
    }

    let width = surface.width() as usize;
    let height = surface.height() as usize;
    let stride = surface.stride() as usize;
    let r = radius as usize;

    let mut data = vec![0u8; stride * height];
    surface
        .with_data(|src| data.copy_from_slice(src))
        .context("blur: failed to read surface data")?;

    let mut scratch = vec![0u8; stride * height];

    // Precompute ceil(2^32 / c) for every possible window size c.
    let max_count = (2 * r + 1).min(width.max(height));
    let factor: Vec<u32> = (0..=max_count)
        .map(|c| {
            if c == 0 {
                0
            } else {
                (1u64 << 32).div_ceil(c as u64) as u32
            }
        })
        .collect();

    #[inline(always)]
    fn div_mul(n: u32, factor: u32) -> u8 {
        ((n as u64 * factor as u64) >> 32) as u8
    }

    for _ in 0..times {
        // Horizontal box blur: data -> scratch
        for y in 0..height {
            let row = y * stride;
            let init_end = r.min(width - 1);
            let mut b_acc = 0u32;
            let mut g_acc = 0u32;
            let mut r_acc = 0u32;
            for x in 0..=init_end {
                let px = row + x * 4;
                b_acc += data[px] as u32;
                g_acc += data[px + 1] as u32;
                r_acc += data[px + 2] as u32;
            }
            let mut count = (init_end + 1) as u32;

            for x in 0..width {
                let dst = row + x * 4;
                let f = factor[count as usize];
                scratch[dst] = div_mul(b_acc, f);
                scratch[dst + 1] = div_mul(g_acc, f);
                scratch[dst + 2] = div_mul(r_acc, f);
                scratch[dst + 3] = data[dst + 3];

                if x >= r {
                    let old = row + (x - r) * 4;
                    b_acc -= data[old] as u32;
                    g_acc -= data[old + 1] as u32;
                    r_acc -= data[old + 2] as u32;
                    count -= 1;
                }
                if x + r + 1 < width {
                    let new = row + (x + r + 1) * 4;
                    b_acc += data[new] as u32;
                    g_acc += data[new + 1] as u32;
                    r_acc += data[new + 2] as u32;
                    count += 1;
                }
            }
        }

        // Vertical box blur: scratch -> data
        let init_end = r.min(height - 1);
        let mut b_acc = vec![0u32; width];
        let mut g_acc = vec![0u32; width];
        let mut r_acc = vec![0u32; width];
        for y in 0..=init_end {
            let row = y * stride;
            for x in 0..width {
                let px = row + x * 4;
                b_acc[x] += scratch[px] as u32;
                g_acc[x] += scratch[px + 1] as u32;
                r_acc[x] += scratch[px + 2] as u32;
            }
        }
        let mut count = (init_end + 1) as u32;

        for y in 0..height {
            let f = factor[count as usize];
            let dst_row = y * stride;
            for x in 0..width {
                let dst = dst_row + x * 4;
                data[dst] = div_mul(b_acc[x], f);
                data[dst + 1] = div_mul(g_acc[x], f);
                data[dst + 2] = div_mul(r_acc[x], f);
            }

            if y >= r {
                let old_row = (y - r) * stride;
                for x in 0..width {
                    let px = old_row + x * 4;
                    b_acc[x] -= scratch[px] as u32;
                    g_acc[x] -= scratch[px + 1] as u32;
                    r_acc[x] -= scratch[px + 2] as u32;
                }
                count -= 1;
            }
            if y + r + 1 < height {
                let new_row = (y + r + 1) * stride;
                for x in 0..width {
                    let px = new_row + x * 4;
                    b_acc[x] += scratch[px] as u32;
                    g_acc[x] += scratch[px + 1] as u32;
                    r_acc[x] += scratch[px + 2] as u32;
                }
                count += 1;
            }
        }
    }

    let mut surface_data = surface.data()?;
    surface_data.copy_from_slice(&data);
    Ok(())
}

/// Apply a vignette effect (darken edges).
pub fn apply_vignette(surface: &mut ImageSurface, base: f32, factor: f32) -> Result<()> {
    let width = surface.width();
    let height = surface.height();
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_distance = (center_x * center_x + center_y * center_y).sqrt();

    let stride = surface.stride() as usize;
    let mut data = vec![0u8; stride * height as usize];
    surface
        .with_data(|src| data.copy_from_slice(src))
        .context("vignette: failed to read surface data")?;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            let vignette_factor = base + (1.0 - base) * (distance / max_distance).powf(factor);

            let index = (y as usize * stride) + (x as usize * 4);
            for i in 0..3 {
                let value = data[index + i] as f32 * vignette_factor;
                data[index + i] = value.clamp(0.0, 255.0) as u8;
            }
        }
    }

    let mut surface_data = surface
        .data()
        .context("vignette: failed to write surface data")?;
    surface_data.copy_from_slice(&data);
    Ok(())
}
