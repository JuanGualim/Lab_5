//! Rasterización de triángulos: contorno, relleno baricéntrico y relleno por líneas horizontales.
//!
//! # Regla común de cobertura
//!
//! Los dos rellenos usan exactamente la misma regla, para que generen las mismas posiciones:
//!
//! 1. Se evalúa el **centro** de cada píxel, `(x + 0.5, y + 0.5)`.
//! 2. Las posiciones se convierten a **punto fijo** con 8 bits de subpíxel (1/256 px), como hacen
//!    las GPU. Así todas las comparaciones son con enteros exactos: no hay errores de redondeo que
//!    hagan que un algoritmo diga "dentro" y el otro "fuera" justo sobre una arista.
//! 3. Si un centro cae *exactamente* sobre una arista se aplica la **regla top-left**: el píxel
//!    pertenece al triángulo solo si esa arista es superior (horizontal, con el interior debajo) o
//!    izquierda (el interior está a su derecha). En una fila eso equivale a un intervalo
//!    `[x_izquierda, x_derecha)`, y en vertical a `[y_superior, y_inferior)`. Con esta regla dos
//!    triángulos que comparten una arista no dejan huecos ni generan fragmentos duplicados.
//! 4. Un triángulo con área cero (vértices colineales o repetidos) es degenerado y no genera
//!    fragmentos de relleno.

use crate::fragment::Fragment;
use crate::framebuffer::{HEIGHT, WIDTH};
use crate::line::line;
use crate::vertex::Vertex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriangleMode {
    Wireframe,
    FilledBarycentric,
    FilledScanline,
}

impl TriangleMode {
    pub const ALL: [TriangleMode; 3] = [
        TriangleMode::Wireframe,
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TriangleMode::Wireframe => "Wireframe",
            TriangleMode::FilledBarycentric => "FilledBarycentric",
            TriangleMode::FilledScanline => "FilledScanline",
        }
    }
}

/// Un triángulo ya ensamblado: tres vértices.
pub type Triangle = [Vertex; 3];

/// Bits de precisión subpíxel.
const SUBPIXEL_BITS: u32 = 8;
/// Un píxel en unidades de punto fijo.
const ONE: i64 = 1 << SUBPIXEL_BITS;
/// Medio píxel: desplazamiento del centro del píxel.
const HALF: i64 = ONE / 2;
/// Límite de coordenadas (en píxeles) para que los productos quepan en un `i64`.
const MAX_COORD: f32 = (1 << 20) as f32;

/// Punto en coordenadas de punto fijo (unidades de 1/256 de píxel).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FixedPoint {
    x: i64,
    y: i64,
}

fn to_fixed(v: &Vertex) -> FixedPoint {
    let snap = |c: f32| (c.clamp(-MAX_COORD, MAX_COORD) * ONE as f32).round() as i64;
    FixedPoint {
        x: snap(v.position.x),
        y: snap(v.position.y),
    }
}

/// Centro del píxel `i` en punto fijo: `i + 0.5`.
fn pixel_center(i: i64) -> i64 {
    i * ONE + HALF
}

/// Función de arista: el doble del área con signo del triángulo `(a, b, p)`.
/// Es positiva de un lado de la recta `a→b`, negativa del otro y cero sobre ella.
#[inline(always)]
fn edge_function(a: FixedPoint, b: FixedPoint, p: FixedPoint) -> i64 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// Regla top-left para una arista `a→b` de un triángulo con área positiva
/// (el interior queda donde `edge_function > 0`). En pantalla, `y` crece hacia abajo.
fn is_top_left(a: FixedPoint, b: FixedPoint) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    // Izquierda: la arista sube (dy < 0) y el interior queda a su derecha.
    // Superior: la arista es horizontal y el interior queda debajo (dx > 0).
    dy < 0 || (dy == 0 && dx > 0)
}

/// División entera redondeando hacia +infinito (`d > 0`).
fn ceil_div(n: i64, d: i64) -> i64 {
    -((-n).div_euclid(d))
}

/// Primer índice de píxel `i` cuyo centro cumple `i + 0.5 >= num / den` (`den > 0`).
fn first_center_at_or_after(num: i64, den: i64) -> i64 {
    ceil_div(num - HALF * den, ONE * den)
}

// ---------------------------------------------------------------------------------------------
// Modo contorno
// ---------------------------------------------------------------------------------------------

/// Implementación original: los tres bordes con tres llamadas a `line`.
pub fn triangle_wireframe(v1: &Vertex, v2: &Vertex, v3: &Vertex) -> Vec<Fragment> {
    let color = v1.color;
    let mut fragments = line(v1, v2, color);
    fragments.extend(line(v2, v3, color));
    fragments.extend(line(v3, v1, color));
    fragments
}

// ---------------------------------------------------------------------------------------------
// Algoritmo 1: coordenadas baricéntricas
// ---------------------------------------------------------------------------------------------

/// Rectángulo envolvente en píxeles `(min_x, min_y, max_x, max_y)`, inclusivo y recortado al
/// framebuffer. Si el triángulo queda fuera, `min > max` y el rectángulo está vacío.
fn bounding_box(a: FixedPoint, b: FixedPoint, c: FixedPoint) -> (i64, i64, i64, i64) {
    (
        a.x.min(b.x).min(c.x).div_euclid(ONE).max(0),
        a.y.min(b.y).min(c.y).div_euclid(ONE).max(0),
        a.x.max(b.x).max(c.x).div_euclid(ONE).min(WIDTH as i64 - 1),
        a.y.max(b.y).max(c.y).div_euclid(ONE).min(HEIGHT as i64 - 1),
    )
}

/// Cantidad de centros de píxel que el algoritmo baricéntrico evalúa para este triángulo.
pub fn bounding_box_pixel_count(v1: &Vertex, v2: &Vertex, v3: &Vertex) -> u64 {
    let (a, b, c) = (to_fixed(v1), to_fixed(v2), to_fixed(v3));
    if edge_function(a, b, c) == 0 {
        return 0;
    }
    let (min_x, min_y, max_x, max_y) = bounding_box(a, b, c);
    let width = (max_x - min_x + 1).max(0) as u64;
    let height = (max_y - min_y + 1).max(0) as u64;
    width * height
}

/// Recorre el rectángulo envolvente del triángulo (recortado al framebuffer) y genera un
/// fragmento por cada centro de píxel con las tres coordenadas baricéntricas no negativas.
pub fn triangle_barycentric(v1: &Vertex, v2: &Vertex, v3: &Vertex) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    let color = v1.color;
    barycentric_coverage(v1, v2, v3, |x, y| {
        fragments.push(Fragment::new(x, y, color))
    });
    fragments
}

/// Núcleo del algoritmo baricéntrico: llama a `emit(x, y)` por cada píxel cubierto.
///
/// Las coordenadas baricéntricas son `λ0 = w0 / área`, `λ1 = w1 / área`, `λ2 = w2 / área`, con
/// `w0 + w1 + w2 = área`. Como el orden de los vértices se normaliza para que el área sea positiva,
/// el signo de cada `λi` es el signo de `wi`: para saber si el punto está dentro no hace falta
/// dividir (y nunca se divide por un área cero).
#[inline(always)]
pub fn barycentric_coverage(v1: &Vertex, v2: &Vertex, v3: &Vertex, mut emit: impl FnMut(i32, i32)) {
    let a = to_fixed(v1);
    let mut b = to_fixed(v2);
    let mut c = to_fixed(v3);

    let area = edge_function(a, b, c);
    if area == 0 {
        return; // Degenerado: los tres vértices están sobre una misma línea.
    }
    if area < 0 {
        // Orden inverso: intercambiar dos vértices invierte el signo del área.
        std::mem::swap(&mut b, &mut c);
    }

    let (min_x, min_y, max_x, max_y) = bounding_box(a, b, c);

    // Un centro exactamente sobre una arista solo cuenta si es top-left: en las demás
    // aristas se exige w > 0, que con enteros es lo mismo que w - 1 >= 0.
    let bias0 = if is_top_left(b, c) { 0 } else { -1 };
    let bias1 = if is_top_left(c, a) { 0 } else { -1 };
    let bias2 = if is_top_left(a, b) { 0 } else { -1 };

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = FixedPoint {
                x: pixel_center(x),
                y: pixel_center(y),
            };
            let w0 = edge_function(b, c, p); // peso del vértice a
            let w1 = edge_function(c, a, p); // peso del vértice b
            let w2 = edge_function(a, b, p); // peso del vértice c

            if w0 + bias0 >= 0 && w1 + bias1 >= 0 && w2 + bias2 >= 0 {
                emit(x as i32, y as i32);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Algoritmo 2: líneas horizontales (scanline)
// ---------------------------------------------------------------------------------------------

/// Columna del primer centro de píxel `>=` a la intersección de la arista `a→b` con la línea
/// horizontal `y = yc`. Requiere `a.y <= yc < b.y`, así que la arista nunca es horizontal.
///
/// La intersección es `x = a.x + (yc - a.y) * (b.x - a.x) / (b.y - a.y)`; se calcula como fracción
/// exacta `num / den` y se redondea con división entera, sin pasar por números flotantes.
fn first_center_on_edge(a: FixedPoint, b: FixedPoint, yc: i64) -> i64 {
    let den = b.y - a.y;
    let num = a.x * den + (yc - a.y) * (b.x - a.x);
    first_center_at_or_after(num, den)
}

/// Avanza una fila a la vez y genera los fragmentos entre los dos extremos de cada tramo.
pub fn triangle_scanline(v1: &Vertex, v2: &Vertex, v3: &Vertex) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    let color = v1.color;
    scanline_coverage(v1, v2, v3, |x, y| {
        fragments.push(Fragment::new(x, y, color))
    });
    fragments
}

/// Núcleo del algoritmo por líneas horizontales: llama a `emit(x, y)` por cada píxel cubierto.
///
/// En cada fila calcula dónde la línea horizontal que pasa por los centros de los píxeles cruza
/// las dos aristas activas y emite el tramo `[x_izquierda, x_derecha)`.
///
/// Casos especiales:
/// * Una arista solo está activa en `[y_superior, y_inferior)`: las aristas horizontales nunca
///   se intersecan (no se divide entre cero) y un vértice sobre una fila pertenece a la arista
///   que empieza ahí, no a la que termina. Por eso no hay huecos ni fragmentos duplicados.
/// * El centro sobre la arista izquierda se incluye y sobre la derecha no: es la misma regla
///   top-left que usa el algoritmo baricéntrico.
#[inline(always)]
pub fn scanline_coverage(v1: &Vertex, v2: &Vertex, v3: &Vertex, mut emit: impl FnMut(i32, i32)) {
    let mut points = [to_fixed(v1), to_fixed(v2), to_fixed(v3)];
    if edge_function(points[0], points[1], points[2]) == 0 {
        return; // Degenerado: el mismo criterio que el algoritmo baricéntrico.
    }

    // Ordenar por y: `top` arriba, `bottom` abajo. El orden original no importa.
    points.sort_by_key(|p| p.y);
    let [top, mid, bottom] = points;

    // Filas cuyo centro está en [top.y, bottom.y), recortadas al framebuffer.
    let y_start = first_center_at_or_after(top.y, 1).max(0);
    let y_end = first_center_at_or_after(bottom.y, 1).min(HEIGHT as i64);

    for y in y_start..y_end {
        let yc = pixel_center(y);

        // La arista larga top→bottom cruza todas las filas; la otra depende de la mitad.
        let (a, b) = if yc < mid.y {
            (top, mid)
        } else {
            (mid, bottom)
        };
        let x_long = first_center_on_edge(top, bottom, yc);
        let x_short = first_center_on_edge(a, b, yc);

        let x_start = x_long.min(x_short).max(0);
        let x_end = x_long.max(x_short).min(WIDTH as i64);

        for x in x_start..x_end {
            emit(x as i32, y as i32);
        }
    }
}

/// Genera los fragmentos de un triángulo según el modo.
pub fn triangle(v1: &Vertex, v2: &Vertex, v3: &Vertex, mode: TriangleMode) -> Vec<Fragment> {
    match mode {
        TriangleMode::Wireframe => triangle_wireframe(v1, v2, v3),
        TriangleMode::FilledBarycentric => triangle_barycentric(v1, v2, v3),
        TriangleMode::FilledScanline => triangle_scanline(v1, v2, v3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(x: i64, y: i64) -> FixedPoint {
        FixedPoint {
            x: x * ONE,
            y: y * ONE,
        }
    }

    #[test]
    fn ceil_div_rounds_up_for_negative_numbers() {
        assert_eq!(ceil_div(5, 2), 3);
        assert_eq!(ceil_div(4, 2), 2);
        assert_eq!(ceil_div(-5, 2), -2);
        assert_eq!(ceil_div(-4, 2), -2);
    }

    #[test]
    fn first_center_includes_exact_ties() {
        // El centro del píxel 3 es 3.5: si la intersección está en 3.5, el primer píxel es 3.
        assert_eq!(first_center_at_or_after(3 * ONE + HALF, 1), 3);
        assert_eq!(first_center_at_or_after(3 * ONE + HALF + 1, 1), 4);
        assert_eq!(first_center_at_or_after(-ONE, 1), -1); // -1 + 0.5 = -0.5 >= -1
    }

    #[test]
    fn top_left_classification() {
        // Triángulo con área positiva: a=(0,0), b=(10,0), c=(0,10) en pantalla (y hacia abajo).
        let (a, b, c) = (fp(0, 0), fp(10, 0), fp(0, 10));
        assert!(edge_function(a, b, c) > 0);
        assert!(is_top_left(a, b)); // arista superior horizontal
        assert!(!is_top_left(b, c)); // hipotenusa: derecha
        assert!(is_top_left(c, a)); // arista vertical izquierda
    }
}
