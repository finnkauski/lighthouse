/// # Color module (UNDERDEVELOPED)
///
/// This module (gated under the `color` feature) contains helpers in converting
/// colors to the required representations for the HUE API.
///
/// **NOTE:** Currently untested and work in progress. If you want to please submit
/// a PR with improvements.
use palette::{rgb::Srgb, Hsl};

/// Convert from 'rgb' to the 'xy' values that can be sent to the
/// hue lights. Does not internally use color gamut.
///
/// **NOTE:** Currently no gamma correction is used. This was implemented based on the
/// gist found [here](https://gist.github.com/popcorn245/30afa0f98eea1c2fd34d).
pub fn rgb_to_xy(rgb: Vec<u8>) -> [f32; 2] {
    // NOTE: more information https://gist.github.com/popcorn245/30afa0f98eea1c2fd34d
    let standardise = |c: u8| {
        let val = (c as f32) / 255.0;
        if val > 0.04045 {
            ((val + 0.055) / (1.0 + 0.055)).powf(2.4)
        } else {
            val / 12.92
        }
    };

    let cnv: Vec<f32> = rgb.into_iter().map(standardise).collect();
    let (red, green, blue) = (cnv[0], cnv[1], cnv[2]);

    let x = red * 0.664_511 + green * 0.154_324 + blue * 0.162_028;
    let y = red * 0.283_881 + green * 0.668_433 + blue * 0.047_685;
    let z = red * 0.000_088 + green * 0.072_310 + blue * 0.986_039;
    let denominator = x + y + z;

    // TODO: if the z is truly the brightness we need to return it
    [x / denominator, y / denominator]
}

/// Convert from 'rgb' to the 'hsl' values that can be sent to the
/// hue lights.
pub fn rgb_to_hsl(rgb: Vec<u8>) -> (u16, u8, u8) {
    let standard: Vec<f32> = rgb
        .into_iter()
        .map(|val: u8| (val as f32) / 255.0)
        .collect();
    let (red, green, blue) = (standard[0], standard[1], standard[2]);
    let hsl: Hsl = Srgb::new(red, green, blue).into();
    let (h, s, l) = hsl.into_components();
    (
        (h.to_positive_degrees() / 360.0 * 65535.0) as u16,
        (s * 254.0) as u8,
        (l * 254.0) as u8,
    )
}

/// Convert hex color to `hsl`
pub fn hex_to_hsl(s: &str) -> Result<(u16, u8, u8), std::num::ParseIntError> {
    let rgb = hex_to_rgb(s)?;
    Ok(rgb_to_hsl(rgb))
}

/// Convert hex color string to `rgb`
pub fn hex_to_rgb(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
