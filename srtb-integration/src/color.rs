use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct HslColor {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn from_hex_str(hex: &str) -> Result<Self, ColorError> {
        let hex = if let Some(h) = hex.strip_prefix('#') {
            h
        } else {
            hex
        };
        if hex.len() != 6 {
            return Err(ColorError::InvalidSize(hex.len()));
        }
        let i = u32::from_str_radix(hex, 16).map_err(|_| ColorError::InvalidInteger)?;
        Ok(Self::from_hex(i))
    }

    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as u8;
        let g = ((hex >> 8) & 0xFF) as u8;
        let b = (hex & 0xFF) as u8;
        Self { r, g, b }
    }
}

impl From<HslColor> for RgbColor {
    fn from(value: HslColor) -> Self {
        let HslColor { h, s, l } = value;
        let (h, s, l) = (h as f64, s as f64, l as f64);
        let c = (1. - (2. * l - 1.).abs()) * s;
        let h = h * 6.;
        let x = c * (1. - (h % 2. - 1.).abs());
        let (r, g, b) = if (0. ..1.).contains(&h) {
            (c, x, 0.)
        } else if (1. ..2.).contains(&h) {
            (x, c, 0.)
        } else if (2. ..3.).contains(&h) {
            (0., c, x)
        } else if (3. ..4.).contains(&h) {
            (0., x, c)
        } else if (4. ..5.).contains(&h) {
            (x, 0., c)
        } else if (5. ..=6.).contains(&h) {
            (c, 0., x)
        } else {
            (0., 0., 0.)
        };
        let m = l - (c / 2.);
        let r = ((r + m) * 255.).round() as u8;
        let g = ((g + m) * 255.).round() as u8;
        let b = ((b + m) * 255.).round() as u8;
        Self { r, g, b }
    }
}

impl From<RgbColor> for HslColor {
    fn from(value: RgbColor) -> Self {
        let RgbColor { r, g, b } = value;
        let (r, g, b) = (r as f64 / 255., g as f64 / 255., b as f64 / 255.);

        let x_max = r.max(g.max(b));
        let x_min = r.min(g.min(b));
        let d = x_max - x_min;

        let h = if d == 0. {
            0.
        } else if x_max == r {
            60. * (((g - b) / d) % 6.)
        } else if x_max == g {
            60. * (((b - r) / d) + 2.)
        } else if x_max == b {
            60. * (((r - g) / d) + 4.)
        } else {
            0.
        };

        let l = (x_max + x_min) / 2.;

        let s = if d == 0. {
            0.
        } else {
            d / (1. - (2. * l - 1.).abs())
        };

        let h = h / 360.;
        let h = if h >= 1. {
            h - 1.
        } else if h < 0. {
            h + 1.
        } else {
            h
        };

        let (h, s, l) = (h as f32, s as f32, l as f32);
        Self { h, s, l }
    }
}

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("invalid color length: expected 6, found {0}")]
    InvalidSize(usize),

    #[error("not a valid 32-bit integer")]
    InvalidInteger,
}

#[cfg(test)]
mod test {
    use super::{ColorError, HslColor, RgbColor};

    #[test]
    fn hsl_to_rgb() {
        let col = HslColor {
            h: 0.,
            s: 1.,
            l: 0.5,
        };
        let expected_col = RgbColor { r: 255, g: 0, b: 0 };
        assert_eq!(expected_col, col.into());

        let col = HslColor {
            h: 0.94623655,
            s: 0.25619835,
            l: 0.2372549,
        };
        let expected_col = RgbColor {
            r: 76,
            g: 45,
            b: 55,
        };
        assert_eq!(expected_col, col.into());

        let col = HslColor {
            h: 0.429972,
            s: 0.5804878,
            l: 0.5980392,
        };
        let expected_col = RgbColor {
            r: 93,
            g: 212,
            b: 162,
        };
        assert_eq!(expected_col, col.into());
    }

    #[test]
    fn rgb_to_hsl() {
        let col = RgbColor { r: 255, g: 0, b: 0 };
        let expected_col = HslColor {
            h: 0.,
            s: 1.,
            l: 0.5,
        };
        assert_eq!(expected_col, col.into());

        let col = RgbColor {
            r: 76,
            g: 45,
            b: 55,
        };
        let expected_col = HslColor {
            h: 0.94623655,
            s: 0.25619835,
            l: 0.2372549,
        };
        assert_eq!(expected_col, col.into());

        let col = RgbColor {
            r: 93,
            g: 212,
            b: 162,
        };
        let expected_col = HslColor {
            h: 0.429972,
            s: 0.5804878,
            l: 0.5980392,
        };
        assert_eq!(expected_col, col.into());
    }

    #[test]
    fn hex_to_rgb() -> Result<(), ColorError> {
        let col = RgbColor::from_hex_str("#1f1e33")?;
        let expected_col = RgbColor::from_hex(0x1f1e33);
        assert_eq!(col, expected_col);

        Ok(())
    }
}
