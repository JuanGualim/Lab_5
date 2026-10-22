//! Punto de entrada: ventana interactiva, benchmark, comparación y exportación de imágenes.

use std::time::Instant;

use lab5_triangulos::benchmark::{self, BenchConfig};
use lab5_triangulos::compare::compare_fragments;
use lab5_triangulos::framebuffer::{Framebuffer, HEIGHT, WIDTH};
use lab5_triangulos::pipeline::{fragment_processing, primitive_assembly, rasterize};
use lab5_triangulos::scenes::{Scene, reverse_case_original, test_scenes};
use lab5_triangulos::triangle::TriangleMode;
use raylib::prelude::*;

const HELP: &str = "\
lab5_triangulos — rellenar triangulos

USO:
    cargo run --release [-- opciones]

OPCIONES:
    (sin opciones)            Abre la ventana interactiva
    --benchmark-triangles     Mide la rasterizacion sin abrir la ventana
        --iterations N        Ejecuciones por medicion (100 por defecto)
        --repeats N           Mediciones; se reporta la mediana (11 por defecto)
        --breakdown           En lugar de las tablas, desglosa el tiempo por etapa
                              (geometria, escritura de fragmentos, memoria, Vec)
        --pprof-flamegraph F  Perfila el benchmark y guarda un flamegraph en F
                              (requiere compilar con --features pprof)
    --compare                 Compara las posiciones de ambos rellenos en cada caso
    --export-images DIR       Guarda un PNG por caso y por modo (sin ventana)
    --screenshots DIR         Abre la ventana, captura cada modo y cada caso, y sale
    --help                    Muestra esta ayuda

CONTROLES DE LA VENTANA:
    1 / 2 / 3                 Wireframe / FilledBarycentric / FilledScanline
    Flechas izq./der.         Caso de prueba anterior / siguiente
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value_of = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
    } else if args.iter().any(|a| a == "--benchmark-triangles") {
        let defaults = BenchConfig::default();
        let config = BenchConfig {
            iterations: value_of("--iterations").map_or(defaults.iterations, |v| parse(&v)),
            repeats: value_of("--repeats").map_or(defaults.repeats, |v| parse(&v)),
        };
        match value_of("--pprof-flamegraph") {
            Some(path) => profile_benchmark(config, &path),
            None if args.iter().any(|a| a == "--breakdown") => benchmark::run_breakdown(config),
            None => benchmark::run(config),
        }
    } else if args.iter().any(|a| a == "--compare") {
        compare_all();
    } else if let Some(dir) = value_of("--export-images") {
        export_images(&dir);
    } else if let Some(dir) = value_of("--screenshots") {
        run_window(Some(&dir));
    } else {
        run_window(None);
    }
}

fn parse(value: &str) -> usize {
    value
        .parse()
        .unwrap_or_else(|_| panic!("se esperaba un numero, se recibio {value:?}"))
}

#[cfg(feature = "pprof")]
fn profile_benchmark(config: BenchConfig, path: &str) {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(2000)
        .blocklist(&["vdso"])
        .build()
        .expect("no se pudo iniciar el perfilador");
    benchmark::run(config);
    let report = guard
        .report()
        .build()
        .expect("no se pudo generar el reporte");
    let file = std::fs::File::create(path).expect("no se pudo crear el archivo del flamegraph");
    report
        .flamegraph(file)
        .expect("no se pudo escribir el flamegraph");
    println!("Flamegraph guardado en {path}");
}

#[cfg(not(feature = "pprof"))]
fn profile_benchmark(_config: BenchConfig, _path: &str) {
    eprintln!("Compila con `--features pprof` para usar --pprof-flamegraph.");
    std::process::exit(1);
}

/// Imprime, para cada caso, si ambos rellenos generan las mismas posiciones.
fn compare_all() {
    println!(
        "| Caso | Baricentrico | Scanline | Solo baricentrico | Solo scanline | Duplicados | Resultado |"
    );
    println!("|---|---:|---:|---:|---:|---:|---|");
    for scene in test_scenes() {
        let triangles = primitive_assembly(&scene.vertices);
        let bary = rasterize(&triangles, TriangleMode::FilledBarycentric);
        let scan = rasterize(&triangles, TriangleMode::FilledScanline);
        let cmp = compare_fragments(&bary, &scan);
        println!(
            "| {} | {} | {} | {} | {} | {} / {} | {} |",
            scene.name,
            cmp.count_a,
            cmp.count_b,
            cmp.only_in_a.len(),
            cmp.only_in_b.len(),
            cmp.duplicates_a,
            cmp.duplicates_b,
            if cmp.identical() {
                "identicos"
            } else {
                "DIFERENTES"
            }
        );
    }

    // Caso 3: el mismo triángulo en orden original y en orden inverso.
    let [a, b, c] = reverse_case_original();
    println!("\nOrden original (a, b, c) vs. orden inverso (c, b, a):");
    for mode in [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ] {
        let forward = rasterize(&[[a, b, c]], mode);
        let reverse = rasterize(&[[c, b, a]], mode);
        let cmp = compare_fragments(&forward, &reverse);
        println!(
            "  {}: {} vs {} fragmentos -> {}",
            mode.name(),
            cmp.count_a,
            cmp.count_b,
            if cmp.identical() {
                "identicos"
            } else {
                "DIFERENTES"
            }
        );
    }
}

/// Renderiza una escena en un framebuffer nuevo con el pipeline completo.
fn render_scene(scene: &Scene, mode: TriangleMode) -> (Framebuffer, usize) {
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let triangles = primitive_assembly(&scene.vertices);
    let fragments = rasterize(&triangles, mode);
    fragment_processing(&mut framebuffer, &fragments);
    (framebuffer, fragments.len())
}

fn file_stem(scene_index: usize, mode: TriangleMode) -> String {
    format!("caso{}_{}", scene_index + 1, mode.name())
}

fn export_images(dir: &str) {
    raylib::core::logging::set_trace_log(TraceLogLevel::LOG_WARNING);
    std::fs::create_dir_all(dir).expect("no se pudo crear el directorio");
    for (i, scene) in test_scenes().iter().enumerate() {
        for mode in TriangleMode::ALL {
            let (framebuffer, count) = render_scene(scene, mode);
            let mut image = Image::gen_image_color(WIDTH as i32, HEIGHT as i32, Color::BLACK);
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let c = framebuffer.get_pixel(x, y);
                    image.draw_pixel(x as i32, y as i32, Color::new(c.r, c.g, c.b, c.a));
                }
            }
            let path = format!("{dir}/{}.png", file_stem(i, mode));
            image.export_image(&path);
            println!("{path}: {count} fragmentos");
        }
    }
}

/// Estado mostrado en la ventana.
struct ViewState {
    scenes: Vec<Scene>,
    scene: usize,
    mode: TriangleMode,
    /// Resumen de la comparación baricéntrico vs. scanline del caso actual.
    comparison: String,
}

impl ViewState {
    fn update_comparison(&mut self) {
        let triangles = primitive_assembly(&self.scenes[self.scene].vertices);
        let bary = rasterize(&triangles, TriangleMode::FilledBarycentric);
        let scan = rasterize(&triangles, TriangleMode::FilledScanline);
        let cmp = compare_fragments(&bary, &scan);
        self.comparison = if cmp.identical() {
            format!(
                "Baricentrico vs scanline: identicos ({} fragmentos)",
                cmp.count_a
            )
        } else {
            format!(
                "Baricentrico vs scanline: {} vs {} ({} / {} distintos)",
                cmp.count_a,
                cmp.count_b,
                cmp.only_in_a.len(),
                cmp.only_in_b.len()
            )
        };
    }
}

fn run_window(screenshot_dir: Option<&str>) {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title("Lab 5 - Rellenar triangulos")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    rl.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let blank = Image::gen_image_color(WIDTH as i32, HEIGHT as i32, Color::BLACK);
    let mut texture = rl
        .load_texture_from_image(&thread, &blank)
        .expect("no se pudo crear la textura");

    let mut state = ViewState {
        scenes: test_scenes(),
        scene: 0,
        mode: TriangleMode::FilledBarycentric,
        comparison: String::new(),
    };
    state.update_comparison();

    // Para el modo de capturas: lista de (caso, modo) pendientes.
    let mut pending: Vec<(usize, TriangleMode)> = Vec::new();
    if let Some(dir) = screenshot_dir {
        std::fs::create_dir_all(dir).expect("no se pudo crear el directorio");
        for i in 0..state.scenes.len() {
            for mode in TriangleMode::ALL {
                pending.push((i, mode));
            }
        }
        pending.reverse();
    }
    // La pantalla que se lee puede ir uno o dos cuadros atrasada (doble/triple búfer), así que
    // cada estado se mantiene varios cuadros y se captura en el último.
    const FRAMES_PER_CAPTURE: u32 = 8;
    let mut frames_in_state = 0;
    let mut capture: Option<(usize, TriangleMode)> = None;
    let mut raster_ms = 0.0f64;

    while !rl.window_should_close() {
        let previous = (state.scene, state.mode);

        // Entrada
        if screenshot_dir.is_some() {
            frames_in_state += 1;
            if frames_in_state == FRAMES_PER_CAPTURE || pending.len() == state.scenes.len() * 3 {
                frames_in_state = 0;
                match pending.pop() {
                    Some((i, mode)) => {
                        state.scene = i;
                        state.mode = mode;
                    }
                    None => break,
                }
            }
            if frames_in_state == FRAMES_PER_CAPTURE - 1 {
                capture = Some((state.scene, state.mode));
            }
        } else {
            if rl.is_key_pressed(KeyboardKey::KEY_ONE) {
                state.mode = TriangleMode::Wireframe;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_TWO) {
                state.mode = TriangleMode::FilledBarycentric;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_THREE) {
                state.mode = TriangleMode::FilledScanline;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
                state.scene = (state.scene + 1) % state.scenes.len();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
                state.scene = (state.scene + state.scenes.len() - 1) % state.scenes.len();
            }
        }
        if state.scene != previous.0 {
            state.update_comparison();
        }
        if (state.scene, state.mode) != previous {
            raster_ms = 0.0; // reiniciar el promedio móvil del tiempo
        }

        // Pipeline: solo `rasterize` entra en el tiempo mostrado.
        let scene = &state.scenes[state.scene];
        framebuffer.clear();
        let triangles = primitive_assembly(&scene.vertices);
        let start = Instant::now();
        let fragments = rasterize(&triangles, state.mode);
        let elapsed = start.elapsed().as_secs_f64() * 1e3;
        raster_ms = if raster_ms == 0.0 {
            elapsed
        } else {
            raster_ms * 0.9 + elapsed * 0.1
        };
        fragment_processing(&mut framebuffer, &fragments);
        texture
            .update_texture(&framebuffer.to_rgba_bytes())
            .expect("tamano de textura incorrecto");

        // Dibujo
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture(&texture, 0, 0, Color::WHITE);
        draw_hud(&mut d, &state, fragments.len(), raster_ms);
        drop(d);

        if let (Some(dir), Some((i, mode))) = (screenshot_dir, capture.take()) {
            let image = rl.load_image_from_screen(&thread);
            let path = format!("{dir}/{}.png", file_stem(i, mode));
            image.export_image(&path);
            println!("{path}");
        }
    }
}

fn draw_hud(d: &mut RaylibDrawHandle, state: &ViewState, fragments: usize, raster_ms: f64) {
    let scene = &state.scenes[state.scene];
    d.draw_rectangle(8, 8, 520, 112, Color::new(0, 0, 0, 170));
    d.draw_text(
        &format!(
            "Caso {}/{}: {}",
            state.scene + 1,
            state.scenes.len(),
            scene.name
        ),
        16,
        14,
        20,
        Color::WHITE,
    );
    d.draw_text(scene.description, 16, 38, 10, Color::LIGHTGRAY);
    let modes = TriangleMode::ALL
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let mark = if *m == state.mode { ">" } else { " " };
            format!("{mark}[{}] {}", i + 1, m.name())
        })
        .collect::<Vec<_>>()
        .join("   ");
    d.draw_text(&modes, 16, 56, 10, Color::new(255, 200, 120, 255));
    d.draw_text(
        &format!("Fragmentos: {fragments}    rasterize: {raster_ms:.3} ms"),
        16,
        74,
        10,
        Color::WHITE,
    );
    d.draw_text(
        &state.comparison,
        16,
        90,
        10,
        Color::new(140, 230, 140, 255),
    );
    d.draw_text(
        "1/2/3: modo    <- / ->: caso    ESC: salir",
        16,
        106,
        10,
        Color::GRAY,
    );
}
