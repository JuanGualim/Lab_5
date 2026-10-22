//! Fragmentos: candidatos a píxel generados por la rasterización.

use crate::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fragment {
    /// Columna del píxel (su centro está en `x + 0.5`).
    pub x: i32,
    /// Fila del píxel (su centro está en `y + 0.5`).
    pub y: i32,
    pub color: Color,
}

impl Fragment {
    pub const fn new(x: i32, y: i32, color: Color) -> Self {
        Fragment { x, y, color }
    }
}
