use cairo::Context;

use crate::config::RingShape;

/// Number of linear segments used to approximate curved portions of a shape.
/// Higher = smoother, lower = faster.
const SEGMENTS: usize = 120;

/// Return the (x, y) point on the shape's perimeter at normalized position `t` ∈ [0, 1].
/// `t = 0` is a reference point (rightmost for most shapes); `t` increases clockwise.
/// `r` is the shape's characteristic radius (distance from center to side/vertex).
pub(crate) fn perimeter_point(cx: f64, cy: f64, r: f64, shape: RingShape, t: f64) -> (f64, f64) {
    // Normalize t to [0, 1). Rust's % preserves sign, and the shape-specific
    // functions use floor/truncation that break on negative values.
    let t = t - t.floor();

    match shape {
        RingShape::Circle => {
            let angle = t * 2.0 * std::f64::consts::PI;
            (cx + r * angle.cos(), cy + r * angle.sin())
        }
        RingShape::Square => square_perimeter_point(cx, cy, r, t),
        RingShape::Diamond => diamond_perimeter_point(cx, cy, r, t),
        RingShape::Hexagon => hexagon_perimeter_point(cx, cy, r, t),
        RingShape::Pill => pill_perimeter_point(cx, cy, r, t),
    }
}

/// Right-top-right-bottom-left-bottom-left-top order (clockwise from right).
fn square_perimeter_point(cx: f64, cy: f64, r: f64, t: f64) -> (f64, f64) {
    let t = t % 1.0;
    let side = (t * 4.0).floor() as u32;
    let local = t * 4.0 - side as f64;

    match side {
        0 => {
            // Right side: top → bottom
            (cx + r, cy - r + local * 2.0 * r)
        }
        1 => {
            // Bottom side: right → left
            (cx + r - local * 2.0 * r, cy + r)
        }
        2 => {
            // Left side: bottom → top
            (cx - r, cy + r - local * 2.0 * r)
        }
        _ => {
            // Top side: left → right
            (cx - r + local * 2.0 * r, cy - r)
        }
    }
}

/// Right-bottom-left-top order (clockwise from right).
fn diamond_perimeter_point(cx: f64, cy: f64, r: f64, t: f64) -> (f64, f64) {
    let t = t % 1.0;
    let side = (t * 4.0).floor() as u32;
    let local = t * 4.0 - side as f64;

    match side {
        0 => {
            // Right to bottom
            (cx + r - local * r, cy + local * r)
        }
        1 => {
            // Bottom to left
            (cx - local * r, cy + r - local * r)
        }
        2 => {
            // Left to top
            (cx - r + local * r, cy - local * r)
        }
        _ => {
            // Top to right
            (cx + local * r, cy - r + local * r)
        }
    }
}
///   0: right → bottom-right   (vertex to vertex)
///   1: bottom edge            (right → left)
///   2: bottom-left → left
///   3: left → top-left
///   4: top edge               (left → right)
///   5: top-right → right
fn hexagon_perimeter_point(cx: f64, cy: f64, r: f64, t: f64) -> (f64, f64) {
    let t = t % 1.0;
    let side = (t * 6.0).floor() as u32;
    let local = t * 6.0 - side as f64;

    // Shared helper for edges between two vertices
    let vert =
        |angle_rad: f64| -> (f64, f64) { (cx + r * angle_rad.cos(), cy + r * angle_rad.sin()) };

    // Vertices clockwise from right (angle = 0)
    let v = [
        vert(0.0),                                // V0: right
        vert(std::f64::consts::PI * (1.0 / 3.0)), // V1: bottom-right
        vert(std::f64::consts::PI * (2.0 / 3.0)), // V2: bottom-left
        vert(std::f64::consts::PI),               // V3: left
        vert(std::f64::consts::PI * (4.0 / 3.0)), // V4: top-left
        vert(std::f64::consts::PI * (5.0 / 3.0)), // V5: top-right
    ];

    let (x0, y0) = v[side as usize];
    let (x1, y1) = v[((side + 1) % 6) as usize];
    (x0 + local * (x1 - x0), y0 + local * (y1 - y0))
}

/// Clockwise from top-right corner: right cap (downward) → bottom straight
/// (leftward) → left cap (upward) → top straight (rightward).
///
/// The pill is a stadium / capsule: cap radius = r, straight-section length = 2r,
/// total width = 4r, total height = 2r.
fn pill_perimeter_point(cx: f64, cy: f64, r: f64, t: f64) -> (f64, f64) {
    let t = t % 1.0;
    let total_p = 4.0 + 2.0 * std::f64::consts::PI; // 4r + 2πr, normalised by r
    let straights = 2.0 / total_p; // each straight segment's t fraction
    let caps = std::f64::consts::PI / total_p; // each cap's t fraction

    if t < caps {
        // Right cap: semicircle, top → bottom, centred at (r, 0)
        let local = t / caps;
        let angle = -std::f64::consts::PI / 2.0 + local * std::f64::consts::PI;
        (cx + r + r * angle.cos(), cy + r * angle.sin())
    } else if t < caps + straights {
        // Bottom straight: right → left
        let local = (t - caps) / straights;
        (cx + r - local * 2.0 * r, cy + r)
    } else if t < caps + straights + caps {
        // Left cap: semicircle, bottom → top, centred at (-r, 0)
        let local = (t - caps - straights) / caps;
        let angle = std::f64::consts::PI / 2.0 + local * std::f64::consts::PI;
        (cx - r + r * angle.cos(), cy + r * angle.sin())
    } else {
        // Top straight: left → right
        let local = (t - 2.0 * caps - straights) / straights;
        (cx - r + local * 2.0 * r, cy - r)
    }
}

/// Build the full closed path of the shape outline (at radius `r`).
/// Call `stroke()` after this to draw the ring.
pub(crate) fn build_ring_path(ctx: &Context, cx: f64, cy: f64, r: f64, shape: RingShape) {
    match shape {
        RingShape::Circle => {
            ctx.arc(cx, cy, r, 0.0, 2.0 * std::f64::consts::PI);
        }
        RingShape::Square | RingShape::Diamond | RingShape::Hexagon | RingShape::Pill => {
            let (x0, y0) = perimeter_point(cx, cy, r, shape, 0.0);
            ctx.move_to(x0, y0);
            // Subdivide perimeter into enough segments for smooth rendering
            let n = 80;
            for i in 1..=n {
                let pt = i as f64 / n as f64;
                let (x, y) = perimeter_point(cx, cy, r, shape, pt);
                ctx.line_to(x, y);
            }
            ctx.close_path();
        }
    }
}

/// Build a partial path along the shape perimeter from normalized position
/// `t_start` to `t_end`. Call `stroke()` after this to draw a sector.
pub(crate) fn build_sector_path(
    ctx: &Context,
    cx: f64,
    cy: f64,
    r: f64,
    shape: RingShape,
    t_start: f64,
    t_end: f64,
) {
    match shape {
        RingShape::Circle => {
            let a_start = t_start * 2.0 * std::f64::consts::PI;
            let a_end = t_end * 2.0 * std::f64::consts::PI;
            ctx.arc(cx, cy, r, a_start, a_end);
        }
        RingShape::Square | RingShape::Diamond | RingShape::Hexagon | RingShape::Pill => {
            let (x0, y0) = perimeter_point(cx, cy, r, shape, t_start);
            ctx.move_to(x0, y0);
            for i in 1..=SEGMENTS {
                let t = t_start + (t_end - t_start) * (i as f64 / SEGMENTS as f64);
                let (x, y) = perimeter_point(cx, cy, r, shape, t);
                ctx.line_to(x, y);
            }
        }
    }
}

/// Build the filled interior path (inset from outer ring by `thickness / 2`).
/// Call `fill()` after this.
pub(crate) fn build_fill_path(
    ctx: &Context,
    cx: f64,
    cy: f64,
    radius: f64,
    thickness: f64,
    shape: RingShape,
) {
    let inner_r = (radius - thickness / 2.0).max(0.0);
    if inner_r <= 0.0 {
        return;
    }
    build_ring_path(ctx, cx, cy, inner_r, shape);
}

/// Check whether a point `(px, py)` lies inside the shape's fill area.
/// This is used for hit‑testing (e.g. click & hold to peek) so the clickable
/// region matches what the user sees on screen.
pub(crate) fn point_in_shape(cx: f64, cy: f64, r: f64, shape: RingShape, px: f64, py: f64) -> bool {
    let dx = (px - cx).abs();
    let dy = (py - cy).abs();
    match shape {
        RingShape::Circle => dx * dx + dy * dy <= r * r,
        RingShape::Square => dx <= r && dy <= r,
        RingShape::Diamond => dx + dy <= r,
        RingShape::Hexagon => {
            // Regular hexagon, vertex at (r, 0), edge slopes at ±60°.
            // For |dx| ≥ r/2 the sloping edge bounds: |dy| ≤ √3 (r − |dx|).
            // For |dx| ≤ r/2 the flat top bounds:    |dy| ≤ √3 r / 2.
            let sqrt3 = 3.0_f64.sqrt();
            if dx >= r / 2.0 {
                dy <= sqrt3 * (r - dx)
            } else {
                dy <= r * sqrt3 / 2.0
            }
        }
        RingShape::Pill => {
            // Pill = centre rectangle (2r × 2r) + semicircular caps of
            // radius r at each end.  Total width 4r, height 2r.
            if dx <= r {
                dy <= r
            } else {
                let ex = dx - r;
                ex * ex + dy * dy <= r * r
            }
        }
    }
}

/// Return the normalized `t` offset that places the first password dot at the
/// visual top-centre of the shape. May be negative; callers should NOT wrap.
pub(crate) fn top_centre_offset(shape: RingShape) -> f64 {
    match shape {
        // Circle/Diamond: top at t=0.75 → offset -(1-0.75) = -0.25
        RingShape::Circle | RingShape::Diamond => -0.25,
        // Square: top edge centre at t=0.875 → offset -(1-0.875) = -0.125
        RingShape::Square => -0.125,
        // Hexagon: top edge centre at t=0.75 → offset -0.25
        RingShape::Hexagon => -0.25,
        // Pill: top straight centre at t = 1 - 1/(4+2π) ≈ 0.9027
        RingShape::Pill => -1.0 / (4.0 + 2.0 * std::f64::consts::PI),
    }
}
