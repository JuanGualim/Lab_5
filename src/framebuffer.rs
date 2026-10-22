//! Framebuffer en memoria (CPU). La ventana solo copia este buffer a una textura.

use crate::color::Color;

/// Tamaño de la ventana y del framebuffer. La rasterización recorta contra estos límites.
pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    buffer: Vec<Color>,
    background: Color,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![Color::BACKGROUND; width * height],
            background: Color::BACKGROUND,
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(self.background);
    }

    /// Escribe un píxel; ignora las posiciones fuera del framebuffer.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.buffer[y as usize * self.width + x as usize] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        self.buffer[y * self.width + x]
    }

    /// Bytes RGBA8 en orden de filas, listos para `UpdateTexture` o para exportar.
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        self.buffer
            .iter()
            .flat_map(|c| [c.r, c.g, c.b, c.a])
            .collect()
    }
}
