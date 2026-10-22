//! Triángulos de prueba (coordenadas de pantalla en un framebuffer de 800x600).

use crate::color::Color;
use crate::vertex::Vertex;

pub struct Scene {
    pub name: &'static str,
    pub description: &'static str,
    /// Lista plana de vértices: cada grupo de tres forma un triángulo.
    pub vertices: Vec<Vertex>,
}

fn v(x: f32, y: f32) -> Vertex {
    Vertex::new(x, y, Color::FILL)
}

fn vc(x: f32, y: f32, color: Color) -> Vertex {
    Vertex::new(x, y, color)
}

/// Triángulo base del caso 3, en su orden original.
pub fn reverse_case_original() -> [Vertex; 3] {
    [v(150.25, 90.75), v(690.5, 210.5), v(330.0, 540.25)]
}

/// Los cinco casos que pide el laboratorio y un sexto con aristas compartidas.
pub fn test_scenes() -> Vec<Scene> {
    let [a, b, c] = reverse_case_original();
    vec![
        Scene {
            name: "1. Pequeno y grande",
            description: "Un triangulo de ~40 px y otro de ~133 000 px",
            vertices: vec![
                v(90.0, 430.0),
                v(98.0, 433.0),
                v(93.0, 441.0),
                v(220.0, 50.0),
                v(770.0, 290.0),
                v(300.0, 570.0),
            ],
        },
        Scene {
            name: "2. Borde horizontal y borde vertical",
            description: "Aristas exactamente sobre centros de pixel (y = 160.5 y x = 460.5)",
            vertices: vec![
                // Arista superior horizontal sobre la fila de centros y = 160.5.
                v(60.0, 160.5),
                v(380.0, 160.5),
                v(190.0, 540.0),
                // Arista izquierda vertical sobre la columna de centros x = 460.5.
                v(460.5, 140.0),
                v(460.5, 560.0),
                v(760.0, 420.0),
            ],
        },
        Scene {
            name: "3. Orden inverso",
            description: "Los mismos tres vertices en orden (c, b, a)",
            vertices: vec![c, b, a],
        },
        Scene {
            name: "4. Parcialmente fuera",
            description: "Vertices con coordenadas negativas y mayores que 800x600",
            vertices: vec![
                v(-180.0, 140.0),
                v(520.0, -160.0),
                v(430.0, 700.0),
                v(600.0, 250.0),
                v(1100.0, 420.0),
                v(690.0, 760.0),
            ],
        },
        Scene {
            name: "5. Vertices colineales",
            description: "Area cero: el relleno no genera fragmentos, el contorno es una linea",
            vertices: vec![
                v(80.0, 80.0),
                v(420.0, 330.0),
                v(760.0, 580.0),
                v(720.0, 60.0),
                v(720.0, 200.0),
                v(720.0, 400.0),
                v(200.0, 450.0),
                v(200.0, 450.0),
                v(200.0, 450.0),
            ],
        },
        Scene {
            name: "6. Aristas compartidas",
            description: "Abanico de 12 triangulos: sin huecos ni fragmentos duplicados",
            vertices: fan(400.0, 300.0, 250.0, 12),
        },
    ]
}

/// Abanico de `n` triángulos alrededor de `(cx, cy)` con colores alternos.
fn fan(cx: f32, cy: f32, radius: f32, n: usize) -> Vec<Vertex> {
    let colors = [Color::FILL, Color::new(230, 90, 120)];
    let point = |i: usize| {
        let angle = i as f32 / n as f32 * std::f32::consts::TAU + 0.3;
        (cx + radius * angle.cos(), cy + radius * angle.sin())
    };
    let mut vertices = Vec::with_capacity(n * 3);
    for i in 0..n {
        let color = colors[i % 2];
        let (x1, y1) = point(i);
        let (x2, y2) = point(i + 1);
        vertices.extend([vc(cx, cy, color), vc(x1, y1, color), vc(x2, y2, color)]);
    }
    vertices
}

/// Generador pseudoaleatorio pequeño (xorshift64*) para que el benchmark sea reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Número en [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * unit
    }
}

/// `count` triángulos aleatorios de tamaños variados, algunos parcialmente fuera de la pantalla.
pub fn random_triangles(count: usize, seed: u64) -> Vec<Vertex> {
    let mut rng = Rng::new(seed);
    let mut vertices = Vec::with_capacity(count * 3);
    for _ in 0..count {
        let cx = rng.range(-50.0, 850.0);
        let cy = rng.range(-50.0, 650.0);
        let size = rng.range(2.0, 120.0);
        for _ in 0..3 {
            vertices.push(v(cx + rng.range(-size, size), cy + rng.range(-size, size)));
        }
    }
    vertices
}
