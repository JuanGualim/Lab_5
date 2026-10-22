//! Comprueba que ambos rellenos generan exactamente las mismas posiciones.

use lab5_triangulos::color::Color;
use lab5_triangulos::compare::compare_fragments;
use lab5_triangulos::fragment::Fragment;
use lab5_triangulos::framebuffer::{HEIGHT, WIDTH};
use lab5_triangulos::pipeline::{primitive_assembly, rasterize};
use lab5_triangulos::scenes::{Rng, random_triangles, reverse_case_original, test_scenes};
use lab5_triangulos::triangle::{Triangle, TriangleMode};
use lab5_triangulos::vertex::Vertex;

const BARY: TriangleMode = TriangleMode::FilledBarycentric;
const SCAN: TriangleMode = TriangleMode::FilledScanline;

fn v(x: f32, y: f32) -> Vertex {
    Vertex::new(x, y, Color::FILL)
}

fn assert_same_positions(triangles: &[Triangle], context: &str) -> usize {
    let bary = rasterize(triangles, BARY);
    let scan = rasterize(triangles, SCAN);
    let cmp = compare_fragments(&bary, &scan);
    assert!(
        cmp.identical(),
        "{context}: {} vs {} fragmentos, solo baricentrico {:?}, solo scanline {:?}",
        cmp.count_a,
        cmp.count_b,
        &cmp.only_in_a[..cmp.only_in_a.len().min(10)],
        &cmp.only_in_b[..cmp.only_in_b.len().min(10)],
    );
    bary.len()
}

fn in_bounds(f: &Fragment) -> bool {
    f.x >= 0 && f.y >= 0 && (f.x as usize) < WIDTH && (f.y as usize) < HEIGHT
}

/// Referencia lenta e independiente: prueba cada centro de píxel de la pantalla con `f64`.
/// Solo sirve para coordenadas "bonitas" (múltiplos de 1/4), donde `f64` es exacto.
fn reference_count(t: &Triangle) -> usize {
    let p: Vec<(f64, f64)> = t
        .iter()
        .map(|v| (v.position.x as f64, v.position.y as f64))
        .collect();
    let edge = |a: (f64, f64), b: (f64, f64), q: (f64, f64)| {
        (b.0 - a.0) * (q.1 - a.1) - (b.1 - a.1) * (q.0 - a.0)
    };
    let (a, mut b, mut c) = (p[0], p[1], p[2]);
    let area = edge(a, b, c);
    if area == 0.0 {
        return 0;
    }
    if area < 0.0 {
        std::mem::swap(&mut b, &mut c);
    }
    let top_left = |a: (f64, f64), b: (f64, f64)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        dy < 0.0 || (dy == 0.0 && dx > 0.0)
    };
    let covers = |w: f64, tl: bool| w > 0.0 || (w == 0.0 && tl);
    let mut count = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let q = (x as f64 + 0.5, y as f64 + 0.5);
            if covers(edge(b, c, q), top_left(b, c))
                && covers(edge(c, a, q), top_left(c, a))
                && covers(edge(a, b, q), top_left(a, b))
            {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn all_test_scenes_produce_identical_positions() {
    for scene in test_scenes() {
        assert_same_positions(&primitive_assembly(&scene.vertices), scene.name);
    }
}

#[test]
fn small_and_large_triangles_match_reference() {
    let scene = &test_scenes()[0];
    for t in primitive_assembly(&scene.vertices) {
        let count = assert_same_positions(&[t], "pequeno/grande");
        assert_eq!(count, reference_count(&t));
        assert!(count > 0);
    }
}

#[test]
fn edges_on_pixel_centers_follow_top_left_rule() {
    // Arista superior horizontal en y = 10.5: la fila 10 se incluye.
    let flat_top = [v(10.0, 10.5), v(30.0, 10.5), v(20.0, 30.0)];
    let fragments = rasterize(&[flat_top], BARY);
    assert!(fragments.iter().any(|f| f.y == 10));
    // Arista inferior horizontal en y = 30.5: la fila 30 se excluye.
    let flat_bottom = [v(20.0, 10.0), v(30.0, 30.5), v(10.0, 30.5)];
    let fragments = rasterize(&[flat_bottom], BARY);
    assert!(fragments.iter().all(|f| f.y < 30));
    // Arista vertical izquierda en x = 10.5: la columna 10 se incluye; derecha en x = 40.5: se excluye.
    let left = [v(10.5, 10.0), v(10.5, 40.0), v(30.0, 25.0)];
    assert!(rasterize(&[left], SCAN).iter().any(|f| f.x == 10));
    let right = [v(40.5, 10.0), v(20.0, 25.0), v(40.5, 40.0)];
    assert!(rasterize(&[right], SCAN).iter().all(|f| f.x < 40));

    for t in [flat_top, flat_bottom, left, right] {
        let count = assert_same_positions(&[t], "aristas sobre centros");
        assert_eq!(count, reference_count(&t));
    }
}

#[test]
fn reversed_vertex_order_produces_same_fragments() {
    let [a, b, c] = reverse_case_original();
    for mode in [BARY, SCAN] {
        let forward = rasterize(&[[a, b, c]], mode);
        assert!(!forward.is_empty());
        for order in [[c, b, a], [b, a, c], [a, c, b], [b, c, a], [c, a, b]] {
            let other = rasterize(&[order], mode);
            assert!(compare_fragments(&forward, &other).identical(), "{mode:?}");
        }
    }
}

#[test]
fn partially_outside_triangles_are_clipped() {
    let scene = &test_scenes()[3];
    let triangles = primitive_assembly(&scene.vertices);
    let count = assert_same_positions(&triangles, "parcialmente fuera");
    assert!(count > 0);
    for mode in TriangleMode::ALL {
        assert!(
            rasterize(&triangles, mode).iter().all(in_bounds),
            "{mode:?}"
        );
    }
    // Completamente fuera: ningún fragmento.
    let outside = [v(-300.0, -10.0), v(-10.0, -300.0), v(-50.0, -50.0)];
    for mode in TriangleMode::ALL {
        assert!(rasterize(&[outside], mode).is_empty());
    }
}

#[test]
fn collinear_vertices_are_degenerate() {
    let scene = &test_scenes()[4];
    let triangles = primitive_assembly(&scene.vertices);
    assert!(rasterize(&triangles, BARY).is_empty());
    assert!(rasterize(&triangles, SCAN).is_empty());
    // El contorno sí se dibuja: es una línea.
    assert!(!rasterize(&triangles, TriangleMode::Wireframe).is_empty());
}

#[test]
fn shared_edges_have_no_gaps_or_duplicates() {
    // Rectángulo [10, 50) x [20, 60) partido por su diagonal: exactamente 40 * 40 píxeles.
    let (p0, p1, p2, p3) = (v(10.0, 20.0), v(50.0, 20.0), v(50.0, 60.0), v(10.0, 60.0));
    let quad = [[p0, p1, p2], [p0, p2, p3]];
    for mode in [BARY, SCAN] {
        let fragments = rasterize(&quad, mode);
        let cmp = compare_fragments(&fragments, &fragments);
        assert_eq!(fragments.len(), 1600, "{mode:?}");
        assert_eq!(cmp.duplicates_a, 0, "{mode:?}");
    }

    // Abanico del caso 6: ningún píxel se genera dos veces.
    let fan = primitive_assembly(&test_scenes()[5].vertices);
    for mode in [BARY, SCAN] {
        let fragments = rasterize(&fan, mode);
        assert_eq!(
            compare_fragments(&fragments, &[]).duplicates_a,
            0,
            "{mode:?}"
        );
    }
}

#[test]
fn tiny_triangle_without_pixel_centers_generates_nothing() {
    let tiny = [v(10.6, 10.6), v(10.9, 10.6), v(10.6, 10.9)];
    assert_eq!(assert_same_positions(&[tiny], "diminuto"), 0);
}

#[test]
fn random_triangles_produce_identical_positions() {
    // Coordenadas arbitrarias.
    let triangles = primitive_assembly(&random_triangles(3000, 7));
    assert_same_positions(&triangles, "aleatorios");

    // Coordenadas en múltiplos de 1/2 px: muchos centros caen exactamente sobre aristas
    // y hay muchas aristas horizontales y verticales.
    let mut rng = Rng::new(99);
    let mut snapped = Vec::new();
    for _ in 0..2000 {
        let mut t = [v(0.0, 0.0); 3];
        for vertex in &mut t {
            let x = (rng.range(-40.0, 840.0) * 2.0).round() / 2.0;
            let y = (rng.range(-40.0, 640.0) * 2.0).round() / 2.0;
            *vertex = v(x, y);
        }
        snapped.push(t);
    }
    for t in &snapped[..20] {
        assert_eq!(
            assert_same_positions(&[*t], "medio pixel"),
            reference_count(t)
        );
    }
    assert_same_positions(&snapped, "medio pixel");
}
