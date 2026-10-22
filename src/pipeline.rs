//! Etapas del pipeline: ensamblaje de primitivas → rasterización → procesamiento de fragmentos.
//!
//! Solo la rasterización depende de `TriangleMode`; las otras dos etapas no cambian al cambiar
//! de algoritmo.

use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::triangle::{Triangle, TriangleMode, triangle};
use crate::vertex::Vertex;

/// Agrupa los vértices de tres en tres.
pub fn primitive_assembly(vertices: &[Vertex]) -> Vec<Triangle> {
    vertices
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]])
        .collect()
}

/// Genera los fragmentos de todos los triángulos con el modo indicado.
pub fn rasterize(triangles: &[Triangle], mode: TriangleMode) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    for [v1, v2, v3] in triangles {
        fragments.extend(triangle(v1, v2, v3, mode));
    }
    fragments
}

/// Escribe cada fragmento en el framebuffer (por ahora, sin pruebas de profundidad).
pub fn fragment_processing(framebuffer: &mut Framebuffer, fragments: &[Fragment]) {
    for fragment in fragments {
        framebuffer.set_pixel(fragment.x, fragment.y, fragment.color);
    }
}

/// Pipeline completo para una lista de vértices.
pub fn render(framebuffer: &mut Framebuffer, vertices: &[Vertex], mode: TriangleMode) -> usize {
    let triangles = primitive_assembly(vertices);
    let fragments = rasterize(&triangles, mode);
    fragment_processing(framebuffer, &fragments);
    fragments.len()
}
