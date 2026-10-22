//! Algoritmo de Bresenham: genera los fragmentos de un segmento.

use crate::color::Color;
use crate::fragment::Fragment;
use crate::framebuffer::{HEIGHT, WIDTH};
use crate::vertex::Vertex;

/// Fragmentos del segmento `a`-`b`. Cada vértice cae en el píxel que lo contiene
/// (`floor`), y solo se conservan las posiciones dentro del framebuffer.
pub fn line(a: &Vertex, b: &Vertex, color: Color) -> Vec<Fragment> {
    let mut fragments = Vec::new();

    let mut x0 = a.position.x.floor() as i64;
    let mut y0 = a.position.y.floor() as i64;
    let x1 = b.position.x.floor() as i64;
    let y1 = b.position.y.floor() as i64;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && y0 >= 0 && x0 < WIDTH as i64 && y0 < HEIGHT as i64 {
            fragments.push(Fragment::new(x0 as i32, y0 as i32, color));
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    fragments
}
