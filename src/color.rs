//! Color RGBA de 8 bits por canal.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const BLACK: Color = Color::new(0, 0, 0);
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const BACKGROUND: Color = Color::new(24, 26, 33);
    /// Color uniforme de relleno: el mismo para ambos algoritmos.
    pub const FILL: Color = Color::new(255, 170, 60);
    pub const OUTLINE: Color = Color::new(110, 210, 255);
}
