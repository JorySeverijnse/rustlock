pub fn parse_hex_color(s: &str) -> Result<(f64, f64, f64, f64), String> {
    let s = s.trim_start_matches('#');
    let len = s.len();

    if len != 6 && len != 8 {
        return Err("Color must be 6 (RRGGBB) or 8 (RRGGBBAA)".to_string());
    }

    let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| "Invalid red")? as f64 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| "Invalid green")? as f64 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| "Invalid blue")? as f64 / 255.0;

    let a = if len == 8 {
        u8::from_str_radix(&s[6..8], 16).map_err(|_| "Invalid alpha")? as f64 / 255.0
    } else {
        1.0
    };

    Ok((r, g, b, a))
}

pub fn parse_blur_effect(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err("Blur must be radiusxtimes".to_string());
    }
    let radius = parts[0].parse().map_err(|_| "Invalid radius")?;
    let times = parts[1].parse().map_err(|_| "Invalid times")?;
    Ok((radius, times))
}

pub fn parse_vignette_effect(s: &str) -> Result<(f32, f32), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Vignette must be base:factor".to_string());
    }
    let base = parts[0].parse().map_err(|_| "Invalid base")?;
    let factor = parts[1].parse().map_err(|_| "Invalid factor")?;
    Ok((base, factor))
}
