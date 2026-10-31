//! Module for generating the colormaps which will be sent to the vertex shader to choose
//! what color to display each live cell as. As cells age they will move across the HSV Hue range from
//! green (120 degrees) to blue (240 degrees) counterclockwise. The Value (brightness) will also decrease,
//! making the cells fade as they age. The fade follows a power curve (`step^VALUE_DROPOFF`), so cells
//! stay bright for most of their life and then dip in their later years.

// Start and end hue values for the HSV color of the cells
// as they age.
const START_HUE: f32 = 120.0;
const END_HUE: f32 = 240.0;

// start and end values for HSV color of the cells as they age.
const START_VALUE: f32 = 1.0;
const END_VALUE: f32 = 0.4;

// Rate of power dropoff of the value as the cell ages.
const VALUE_DROPOFF: i32 = 2;

/// Holds Hue, Saturation, value values
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    hue: f32,
    saturation: f32,
    value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Srgba {
    red: f32,   // 0.0 -> 1.0
    green: f32, // 0.0 -> 1.0
    blue: f32,  // 0.0 -> 1.0
    alpha: f32, // 0.0 -> 1.0
}

/// Linear RGBA: the colour with the sRGB "gamma" curve undone.
///
/// Used when the surface format is sRGB (`surface_config.format.is_srgb()`, i.e. on native).
/// The GPU then treats shader output as linear and applies the sRGB curve itself when writing
/// to the screen, so we must hand it linear values or colours come out washed out.
/// On the web the surface is never sRGB, so the plain [`Srgba`] values are used as-is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaLinear {
    red: f32,   // 0.0 -> 1.0
    green: f32, // 0.0 -> 1.0
    blue: f32,  // 0.0 -> 1.0
    alpha: f32, // 0.0 -> 1.0
}

/// Which colour values the colormap should contain, depending on the surface format.
#[derive(Debug, Clone, Copy)]
pub enum ColorEncoding {
    /// Plain sRGB values: for non-sRGB surfaces (the web).
    Srgb,
    /// Linear values ([`RgbaLinear`]): for sRGB surfaces (native).
    Linear,
}

impl From<Hsv> for Srgba {
    fn from(value: Hsv) -> Self {
        // h is the hue sector the color is in. the wheel is divided into 6 60 degree sectors
        let h = value.hue.rem_euclid(360.0) / 60.0;
        // c: chroma is difference between smallest and largest RGB component
        let c = value.saturation * value.value;
        // intermediate channel
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        // offset for each channel
        let m = value.value - c;

        let [r, g, b] = match h as u32 {
            0 => [c, x, 0.0],
            1 => [x, c, 0.0],
            2 => [0.0, c, x],
            3 => [0.0, x, c],
            4 => [x, 0.0, c],
            _ => [c, 0.0, x],
        };

        Self {
            red: r + m,
            green: g + m,
            blue: b + m,
            alpha: 1.0,
        }
    }
}

impl From<Srgba> for RgbaLinear {
    fn from(value: Srgba) -> Self {
        Self {
            red: srgba_channel_to_linear(value.red),
            green: srgba_channel_to_linear(value.green),
            blue: srgba_channel_to_linear(value.blue),
            alpha: value.alpha,
        }
    }
}
fn srgba_channel_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn make_colormap_torquise(_color_steps: usize, encoding: ColorEncoding) -> Vec<[f32; 4]> {
    let colors = [0x87D4D2, 0x598988, 0x2F4545, 0x0A0C0B];
    colors
        .iter()
        .map(|hex| hex_to_srgb(*hex as u32))
        .map(|srgba| match encoding {
            ColorEncoding::Srgb => [srgba.red, srgba.green, srgba.blue, srgba.alpha],
            ColorEncoding::Linear => {
                let rgbal = RgbaLinear::from(srgba);
                [rgbal.red, rgbal.green, rgbal.blue, rgbal.alpha]
            }
        })
        .collect()
}

fn hex_to_srgb(hex: u32) -> Srgba {
    Srgba {
        red: ((hex >> 16) & 0xFF) as f32 / 255.0,
        green: ((hex >> 8) & 0xFF) as f32 / 255.0,
        blue: (hex & 0xFF) as f32 / 255.0,
        alpha: 1.0,
    }
}

fn _make_colormap_fading(color_steps: usize, _encoding: ColorEncoding) -> Vec<[f32; 4]> {
    let color_steps = color_steps.max(1);
    // at least 1, so a single colour doesn't divide 0 by 0
    let last = (color_steps - 1).max(1) as f32;
    (0..color_steps)
        .map(|i| {
            let step = i as f32 / last; // 0.0 → 1.0
            let value = START_VALUE + step.powi(VALUE_DROPOFF) * (END_VALUE - START_VALUE);
            [value, value, value, 1.0]
        })
        .collect()
}
pub fn make_colormap(color_steps: usize, encoding: ColorEncoding) -> Vec<[f32; 4]> {
    // make_colormap_hsv_rotation(color_steps, encoding)
    // make_colormap_fading(color_steps, encoding)
    make_colormap_torquise(color_steps, encoding)
}
/// Age → colour table for the render shader: `color_steps` colours (at least 1), green → faded blue.
pub fn make_colormap_hsv_rotation(color_steps: usize, encoding: ColorEncoding) -> Vec<[f32; 4]> {
    let color_steps = color_steps.max(1);
    // at least 1, so a single colour doesn't divide 0 by 0
    let last = (color_steps - 1).max(1) as f32;
    (0..color_steps)
        .map(|i| {
            let step = i as f32 / last; // 0.0 → 1.0
            // use rem_euclid() to wrap around 0-360.
            let hue_travel_distance = (START_HUE - END_HUE).rem_euclid(360.0);
            let hue = (START_HUE - step * hue_travel_distance).rem_euclid(360.0);
            let value = START_VALUE + step.powi(VALUE_DROPOFF) * (END_VALUE - START_VALUE);
            Hsv {
                hue,
                saturation: 0.7,
                value,
            }
        })
        // Fully opaque (alpha = 1): the fade comes from Value alone.
        .map(Srgba::from)
        .map(|srgba| match encoding {
            ColorEncoding::Srgb => [srgba.red, srgba.green, srgba.blue, srgba.alpha],
            ColorEncoding::Linear => {
                let rgbal = RgbaLinear::from(srgba);
                [rgbal.red, rgbal.green, rgbal.blue, rgbal.alpha]
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::colormap::{ColorEncoding, Hsv, Srgba, make_colormap, srgba_channel_to_linear};

    /// Compare colours with a small tolerance for float rounding.
    fn assert_close(actual: [f32; 4], expected: [f32; 4]) {
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-5, "{actual:?} != {expected:?}");
        }
    }

    // #[test]
    // fn colormap_endpoints_and_length() {
    //     let map = make_colormap(5, ColorEncoding::Srgb);
    //     assert_eq!(map.len(), 5);
    //     assert_close(map[0], [0.0, 1.0, 0.0, 1.0]); // green
    //     assert_close(map[2], [0.95625, 0.0, 0.0, 1.0]); // red, value 1 - 0.5^4 * 0.7
    //     assert_close(map[4], [0.0, 0.0, 0.3, 1.0]); // blue at END_VALUE
    // }

    // #[test]
    // fn colormap_small_sizes_have_no_nan() {
    //     for steps in [0, 1] {
    //         let map = make_colormap(steps, ColorEncoding::Srgb);
    //         assert_eq!(map.len(), 1);
    //         assert_close(map[0], [0.0, 1.0, 0.0, 1.0]);
    //     }
    // }

    #[test]
    fn linear_encoding() {
        assert_eq!(srgba_channel_to_linear(0.0), 0.0);
        assert!((srgba_channel_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert!((srgba_channel_to_linear(0.5) - 0.214).abs() < 1e-3);
        // each colour channel converted, alpha untouched
        let srgb = make_colormap(5, ColorEncoding::Srgb);
        let linear = make_colormap(5, ColorEncoding::Linear);
        for (s, l) in srgb.iter().zip(&linear) {
            let expected = [
                srgba_channel_to_linear(s[0]),
                srgba_channel_to_linear(s[1]),
                srgba_channel_to_linear(s[2]),
                s[3],
            ];
            assert_close(*l, expected);
        }
    }

    impl Hsv {
        fn from_arr(arr: [f32; 3]) -> Result<Self, String> {
            if arr[0] < 0.0 || arr[0] > 360.0 {
                Err("hue out of range 0 - 360".to_string())
            } else if arr[1] < 0.0 || arr[1] > 1.0 {
                Err("saturation out of range 0 - 1".to_string())
            } else if arr[2] < 0.0 || arr[2] > 1.0 {
                Err("value out of range 0 - 1".to_string())
            } else {
                Ok(Self {
                    hue: arr[0],
                    saturation: arr[1],
                    value: arr[2],
                })
            }
        }
    }
    impl Srgba {
        fn from_arr(arr: [f32; 4]) -> Result<Self, String> {
            if arr[0] < 0.0 || arr[0] > 1.0 {
                Err("Red out of range 0 - 1".to_string())
            } else if arr[1] < 0.0 || arr[1] > 1.0 {
                Err("Green out of range 0 - 1".to_string())
            } else if arr[2] < 0.0 || arr[2] > 1.0 {
                Err("blue out of range 0 - 1".to_string())
            } else if arr[3] < 0.0 || arr[3] > 1.0 {
                Err("alpha out of range 0 - 1".to_string())
            } else {
                Ok(Self {
                    red: arr[0],
                    green: arr[1],
                    blue: arr[2],
                    alpha: arr[3],
                })
            }
        }
    }
    #[test]
    fn hsv_to_rgb_conversion() {
        assert_eq!(
            Srgba::from(Hsv::from_arr([120.0, 1.0, 1.0]).unwrap()),
            Srgba::from_arr([0.0, 1.0, 0.0, 1.0]).unwrap()
        );
        assert_eq!(
            Srgba::from(Hsv::from_arr([180.0, 1.0, 1.0]).unwrap()),
            Srgba::from_arr([0.0, 1.0, 1.0, 1.0]).unwrap()
        );
        assert_eq!(
            Srgba::from(Hsv::from_arr([120.0, 1.0, 1.0]).unwrap()),
            Srgba::from_arr([0.0, 1.0, 0.0, 1.0]).unwrap()
        );
        assert_eq!(
            Srgba::from(Hsv::from_arr([240.0, 1.0, 1.0]).unwrap()),
            Srgba::from_arr([0.0, 0.0, 1.0, 1.0]).unwrap()
        );
    }
}
