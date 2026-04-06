use serde::{Deserialize, Deserializer, Serializer};

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

pub fn deserialize_hex_color<'de, D>(deserializer: D) -> Result<(f64, f64, f64, f64), D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_hex_color(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_hex_color<S>(
    color: &(f64, f64, f64, f64),
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let (r, g, b, a) = color;
    let r = (r * 255.0) as u8;
    let g = (g * 255.0) as u8;
    let b = (b * 255.0) as u8;
    let a = (a * 255.0) as u8;

    if a == 255 {
        serializer.serialize_str(&format!("{:02x}{:02x}{:02x}", r, g, b))
    } else {
        serializer.serialize_str(&format!("{:02x}{:02x}{:02x}{:02x}", r, g, b, a))
    }
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

pub fn deserialize_blur_effect<'de, D>(deserializer: D) -> Result<Option<(u32, u32)>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => parse_blur_effect(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

pub fn serialize_blur_effect<S>(
    val: &Option<(u32, u32)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Some((radius, times)) => serializer.serialize_str(&format!("{}x{}", radius, times)),
        None => serializer.serialize_none(),
    }
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

pub fn deserialize_vignette_effect<'de, D>(deserializer: D) -> Result<Option<(f32, f32)>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => parse_vignette_effect(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

pub fn serialize_vignette_effect<S>(
    val: &Option<(f32, f32)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Some((base, factor)) => serializer.serialize_str(&format!("{}:{}", base, factor)),
        None => serializer.serialize_none(),
    }
}
