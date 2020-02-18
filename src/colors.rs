use palette::{rgb::Srgb, Hsl};

// TODO: gamma correction?
// TODO: Z value is the brightness
// TODO: The values in the gist above are not the same as mine the standardise aligns
/// Convert from 'rgb' to the 'xy' values that can be sent to the
/// hue lights. Does not internally use color gamut.
/// Needs a rewrite
pub fn rgb_to_xy(r0: u8, g0: u8, b0: u8) -> [f32; 2] {
    // NOTE: more information https://gist.github.com/popcorn245/30afa0f98eea1c2fd34d
    let standardise = |c: f32| {
        if c > 0.04045 {
            ((c + 0.055) / (1.0 + 0.055)).powf(2.4)
        } else {
            c / 12.92
        }
    };

    let red = standardise((r0 as f32) / 255.0);
    let green = standardise((g0 as f32) / 255.0);
    let blue = standardise((b0 as f32) / 255.0);

    let x = red * 0.664_511 + green * 0.154_324 + blue * 0.162_028;
    let y = red * 0.283_881 + green * 0.668_433 + blue * 0.047_685;
    let z = red * 0.000_088 + green * 0.072_310 + blue * 0.986_039;
    let denominator = x + y + z;

    // TODO: if the z is truly the brightness we need to return it
    [x / denominator, y / denominator]
}

/// Convert from 'rgb' to the 'hsl' values that can be sent to the
/// hue lights.
pub fn rgb_to_hsl(red: u8, green: u8, blue: u8) -> (u16, u8, u8) {
    let red = (red as f32) / 255.0;
    let green = (green as f32) / 255.0;
    let blue = (blue as f32) / 255.0;

    let hsl: Hsl = Srgb::new(red, green, blue).into();
    let (h, s, l) = hsl.into_components();
    (
        (h.to_positive_degrees() / 360.0 * 65535.0) as u16,
        (s * 254.0) as u8,
        (l * 254.0) as u8,
    )
}

/// Convert hex color to `hsl`
pub fn hex_to_hsl(s: String) -> Result<(u16, u8, u8), std::num::ParseIntError> {
    let rgb = hex_to_rgb(s)?;
    Ok(rgb_to_hsl(rgb[0], rgb[1], rgb[2]))
}

/// Convert hex color string to `rgb`
pub fn hex_to_rgb(s: String) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
