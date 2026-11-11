use femtovg::{LineCap, Paint};

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub color: Color,
    pub size: Size,
    pub fill: bool,
    pub round_caps: bool,
    pub annotation_size_factor: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum Size {
    #[default]
    Small = 0,
    Medium = 1,
    Large = 2,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: Color::default(),
            size: Size::default(),
            fill: false,
            round_caps: true,
            annotation_size_factor: 1.0,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::orange()
    }
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn orange() -> Self {
        Self::new(240, 147, 43, 255)
    }
    pub fn red() -> Self {
        Self::new(235, 77, 75, 255)
    }
    pub fn green() -> Self {
        Self::new(106, 176, 76, 255)
    }
    pub fn blue() -> Self {
        Self::new(34, 166, 179, 255)
    }

    /// Returns the color with its RGB channels inverted, preserving alpha.

    /// Returns the color with its alpha channel replaced.

    /// Returns black or white, whichever contrasts better with this color,
    /// preserving alpha. Uses WCAG relative luminance (channels gamma-decoded to
    /// linear light, then weighted); black and white are equal at L = 0.179.
    pub fn contrast(self) -> Self {
        fn linearize(channel: u8) -> f32 {
            let c = channel as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        let luminance =
            0.2126 * linearize(self.r) + 0.7152 * linearize(self.g) + 0.0722 * linearize(self.b);
        if luminance > 0.179 {
            Self::new(0, 0, 0, self.a)
        } else {
            Self::new(255, 255, 255, self.a)
        }
    }
}

impl From<Color> for femtovg::Color {
    fn from(value: Color) -> Self {
        femtovg::Color {
            r: value.r as f32 / 255.0,
            g: value.g as f32 / 255.0,
            b: value.b as f32 / 255.0,
            a: value.a as f32 / 255.0,
        }
    }
}

impl From<Style> for Paint {
    fn from(value: Style) -> Self {
        Paint::default()
            .with_anti_alias(true)
            .with_font_size(value.size.to_text_size(value.annotation_size_factor) as f32)
            .with_color(value.color.into())
            .with_line_cap(if value.round_caps {
                LineCap::Round
            } else {
                LineCap::Butt
            })
            .with_line_width(value.size.to_line_width(value.annotation_size_factor))
    }
}

impl Size {
    pub fn to_text_size(self, size_factor: f32) -> i32 {
        match self {
            Size::Small => (36.0 * size_factor) as i32,
            Size::Medium => (54.0 * size_factor) as i32,
            Size::Large => (96.0 * size_factor) as i32,
        }
    }

    pub fn to_line_width(self, size_factor: f32) -> f32 {
        match self {
            Size::Small => 3.0 * size_factor,
            Size::Medium => 5.0 * size_factor,
            Size::Large => 7.0 * size_factor,
        }
    }

    pub fn to_arrow_tail_width(self, size_factor: f32) -> f32 {
        match self {
            Size::Small => 3.0 * size_factor,
            Size::Medium => 10.0 * size_factor,
            Size::Large => 25.0 * size_factor,
        }
    }

    pub fn to_arrow_head_length(self, size_factor: f32) -> f32 {
        match self {
            Size::Small => 15.0 * size_factor,
            Size::Medium => 30.0 * size_factor,
            Size::Large => 60.0 * size_factor,
        }
    }
}
