//! Medición del tiempo de rasterización (solo la generación de fragmentos).

use std::hint::black_box;
use std::time::{Duration, Instant};

use crate::compare::compare_fragments;
use crate::fragment::Fragment;
use crate::pipeline::{primitive_assembly, rasterize};
use crate::scenes::{random_triangles, test_scenes};
use crate::triangle::{
    Triangle, TriangleMode, barycentric_coverage, bounding_box_pixel_count, scanline_coverage,
};

#[derive(Clone, Copy, Debug)]
pub struct BenchConfig {
    /// Ejecuciones de `rasterize` dentro de una medición.
    pub iterations: usize,
    /// Número de mediciones; se reporta la mediana.
    pub repeats: usize,
}

impl Default for BenchConfig {
    fn default() -> Self {
        BenchConfig {
            iterations: 100,
            repeats: 11,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BenchResult {
    pub mode: TriangleMode,
    pub fragments: usize,
    /// Mediana, mínimo y máximo del tiempo de `iterations` ejecuciones.
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    pub iterations: usize,
}

impl BenchResult {
    pub fn per_run(&self) -> Duration {
        self.median / self.iterations as u32
    }

    pub fn ns_per_fragment(&self) -> f64 {
        if self.fragments == 0 {
            return f64::NAN;
        }
        self.per_run().as_nanos() as f64 / self.fragments as f64
    }
}

/// Mide `rasterize` con los mismos triángulos `config.repeats` veces y toma la mediana.
pub fn measure(triangles: &[Triangle], mode: TriangleMode, config: BenchConfig) -> BenchResult {
    // Calentamiento: la primera ejecución paga fallos de caché y de página.
    let fragments = rasterize(black_box(triangles), mode).len();

    let mut times: Vec<Duration> = (0..config.repeats)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..config.iterations {
                let fragments = rasterize(black_box(triangles), mode);
                black_box(fragments);
            }
            start.elapsed()
        })
        .collect();
    times.sort();

    BenchResult {
        mode,
        fragments,
        median: times[times.len() / 2],
        min: times[0],
        max: times[times.len() - 1],
        iterations: config.iterations,
    }
}

fn ms(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1e3)
}

fn us(d: Duration) -> String {
    format!("{:.2} µs", d.as_secs_f64() * 1e6)
}

/// Conjuntos de triángulos medidos: los seis casos de prueba y una malla aleatoria.
pub fn benchmark_sets() -> Vec<(String, Vec<Triangle>)> {
    let mut sets: Vec<(String, Vec<Triangle>)> = test_scenes()
        .into_iter()
        .map(|s| (s.name.to_string(), primitive_assembly(&s.vertices)))
        .collect();
    sets.push((
        "7. Malla aleatoria (1000 triangulos)".to_string(),
        primitive_assembly(&random_triangles(1000, 42)),
    ));
    sets
}

/// Ejecuta el benchmark completo e imprime tablas en formato Markdown.
pub fn run(config: BenchConfig) {
    println!(
        "Benchmark de rasterizacion: mediana de {} mediciones de {} ejecuciones cada una.",
        config.repeats, config.iterations
    );
    println!("Solo se mide `rasterize` (sin textura, ventana ni esperas entre cuadros).\n");

    for (name, triangles) in benchmark_sets() {
        let bbox: u64 = triangles
            .iter()
            .map(|[a, b, c]| bounding_box_pixel_count(a, b, c))
            .sum();

        let bary = rasterize(&triangles, TriangleMode::FilledBarycentric);
        let scan = rasterize(&triangles, TriangleMode::FilledScanline);
        let cmp = compare_fragments(&bary, &scan);

        println!("### {name}\n");
        println!(
            "Triangulos: {} | pixeles evaluados por el baricentrico (rectangulos): {} | \
             baricentrico vs scanline: {}\n",
            triangles.len(),
            bbox,
            if cmp.identical() {
                "mismas posiciones".to_string()
            } else {
                format!(
                    "DIFERENTES ({} solo baricentrico, {} solo scanline)",
                    cmp.only_in_a.len(),
                    cmp.only_in_b.len()
                )
            }
        );
        println!(
            "| Modo | Fragmentos | Mediana ({} ej.) | Por ejecucion | ns/fragmento | Min | Max |",
            config.iterations
        );
        println!("|---|---:|---:|---:|---:|---:|---:|");
        for mode in TriangleMode::ALL {
            let r = measure(&triangles, mode, config);
            println!(
                "| {} | {} | {} | {} | {:.2} | {} | {} |",
                mode.name(),
                r.fragments,
                ms(r.median),
                us(r.per_run()),
                r.ns_per_fragment(),
                ms(r.min),
                ms(r.max)
            );
        }
        println!();
    }
}

// ---------------------------------------------------------------------------------------------
// Desglose: ¿dónde se va el tiempo?
// ---------------------------------------------------------------------------------------------

/// Mediana de `config.repeats` mediciones de `config.iterations` llamadas a `run`.
fn time_median(config: BenchConfig, mut run: impl FnMut() -> usize) -> (Duration, usize) {
    let count = run();
    let mut times: Vec<Duration> = (0..config.repeats)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..config.iterations {
                black_box(run());
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    (times[times.len() / 2], count)
}

/// Núcleo de un algoritmo de relleno, sin crear fragmentos.
#[inline(always)]
fn cover(mode: TriangleMode, [v1, v2, v3]: &Triangle, emit: impl FnMut(i32, i32)) {
    match mode {
        TriangleMode::FilledBarycentric => barycentric_coverage(v1, v2, v3, emit),
        TriangleMode::FilledScanline => scanline_coverage(v1, v2, v3, emit),
        TriangleMode::Wireframe => unreachable!("el desglose solo aplica a los rellenos"),
    }
}

/// Mide cada relleno por etapas, agregando una parte del trabajo a la vez:
///
/// 1. Solo la geometría: decidir qué píxeles están dentro (se cuentan, no se guardan).
/// 2. Además, escribir cada `Fragment` en un `Vec` reutilizado que ya tiene capacidad.
/// 3. Además, pedir memoria nueva cada vez (`Vec::with_capacity` exacto, sin crecimiento).
/// 4. La implementación completa: un `Vec` vacío por triángulo que crece con `push`,
///    y `rasterize` copia cada uno al resultado final con `extend`.
pub fn run_breakdown(config: BenchConfig) {
    let sets = benchmark_sets();
    let selected = [&sets[0], &sets[3], &sets[sets.len() - 1]];

    println!("## Desglose del tiempo por etapa\n");
    for (name, triangles) in selected {
        println!("### {name}\n");
        println!("| Algoritmo | Etapa | Por ejecucion | Diferencia con la etapa anterior |");
        println!("|---|---|---:|---:|");
        for mode in [
            TriangleMode::FilledBarycentric,
            TriangleMode::FilledScanline,
        ] {
            let color = triangles[0][0].color;
            let total = rasterize(triangles, mode).len();

            let (geometry, _) = time_median(config, || {
                let mut count = 0;
                for t in black_box(triangles) {
                    cover(mode, t, |_, _| count += 1);
                }
                count
            });

            let mut reused: Vec<Fragment> = Vec::with_capacity(total);
            let (write_reused, _) = time_median(config, || {
                reused.clear();
                for t in black_box(triangles) {
                    cover(mode, t, |x, y| reused.push(Fragment::new(x, y, color)));
                }
                black_box(&reused);
                reused.len()
            });

            let (fresh, _) = time_median(config, || {
                let mut fragments = Vec::with_capacity(total);
                for t in black_box(triangles) {
                    cover(mode, t, |x, y| fragments.push(Fragment::new(x, y, color)));
                }
                black_box(fragments).len()
            });

            let (full, _) = time_median(config, || rasterize(black_box(triangles), mode).len());

            let per_run = |d: Duration| d / config.iterations as u32;
            let stages = [
                ("1. Solo geometria (contar)", geometry),
                ("2. + escribir en Vec reutilizado", write_reused),
                ("3. + memoria nueva (with_capacity)", fresh),
                ("4. Completa (Vec crece + extend)", full),
            ];
            let mut previous: Option<Duration> = None;
            for (stage, time) in stages {
                let delta = match previous {
                    Some(p) => format!(
                        "{:+.2} µs",
                        (per_run(time).as_secs_f64() - per_run(p).as_secs_f64()) * 1e6
                    ),
                    None => "-".to_string(),
                };
                println!(
                    "| {} | {} | {} | {} |",
                    mode.name(),
                    stage,
                    us(per_run(time)),
                    delta
                );
                previous = Some(time);
            }
        }
        println!();
    }
}
