//! Bounded, deterministic native throughput baseline; not a browser FPS test.
//! Run release with --report <absolute-path>, optionally --features raster.
use kartis::{
    mesh::{MeshPoint, PointMesh},
    render::to_svg,
    *,
};
use std::{fmt::Write as _, hint::black_box, path::PathBuf, time::Instant};

type BenchResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Metric {
    operation: &'static str,
    size: usize,
    first: f64,
    samples: Vec<f64>,
    output_units: usize,
}

fn measure(
    operation: &'static str,
    size: usize,
    iterations: usize,
    mut run: impl FnMut() -> BenchResult<usize>,
) -> BenchResult<Metric> {
    let start = Instant::now();
    let output_units = black_box(run()?);
    let first = start.elapsed().as_secs_f64() * 1000.;
    for _ in 0..2 {
        assert_eq!(black_box(run()?), output_units);
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let output = black_box(run()?);
        samples.push(start.elapsed().as_secs_f64() * 1000.);
        assert_eq!(output, output_units);
    }
    samples.sort_by(f64::total_cmp);
    Ok(Metric {
        operation,
        size,
        first,
        samples,
        output_units,
    })
}

fn config() -> ChartConfig {
    ChartConfig {
        width: 1200.,
        height: 720.,
        title: "Density baseline".into(),
        ..Default::default()
    }
}

fn series(size: usize) -> Vec<Series> {
    vec![Series::new(
        "density",
        (0..size)
            .map(|i| Point::new(i as f64, ((i * 104_729) % 80_021) as f64 / 100.))
            .collect(),
    )]
}

fn dense_scene(size: usize) -> Result<Scene> {
    let mut scene = config().scene()?;
    scene.marks.reserve(size);
    for i in 0..size {
        let mut mark = Mark::new(
            format!("p{i:05}"),
            Geometry::Circle {
                x: 10. + ((i * 7919) % 118_001) as f64 / 100.,
                y: 30. + ((i * 104_729) % 68_001) as f64 / 100.,
                radius: 2.,
            },
            Color(78, 121, 167, 255),
        );
        mark.series = "density".into();
        mark.label = format!("Point {i}");
        mark.value = i as f64;
        scene.marks.push(mark);
    }
    // Deliberately exercise actual font shaping in the optional raster run.
    scene.labels.push(Label {
        x: 16.,
        y: 20.,
        text: "20k density".into(),
        size: 14.,
        color: Color(20, 30, 40, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene.validate()?;
    Ok(scene)
}

fn queries(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|i| {
            (
                5. + ((i * 3571) % 119_001) as f64 / 100.,
                5. + ((i * 17431) % 70_001) as f64 / 100.,
            )
        })
        .collect()
}

fn verify_mesh(mesh: &PointMesh, queries: &[(f64, f64)]) {
    for &(x, y) in queries.iter().take(32) {
        let exact = mesh
            .points()
            .iter()
            .min_by(|a, b| {
                (a.x - x)
                    .hypot(a.y - y)
                    .total_cmp(&(b.x - x).hypot(b.y - y))
                    .then_with(|| a.id.cmp(&b.id))
            })
            .unwrap();
        assert_eq!(mesh.nearest(x, y, f64::INFINITY).unwrap().id, exact.id);
    }
}

fn rejected<T>(name: &str, result: Result<T>, output: &mut String) -> BenchResult<()> {
    let error = result
        .err()
        .ok_or_else(|| format!("{name} unexpectedly accepted"))?;
    writeln!(output, "- {name}: `{error}`")?;
    Ok(())
}

#[cfg(feature = "raster")]
fn raster_metrics(
    scene: &Scene,
    size: usize,
    iterations: usize,
    metrics: &mut Vec<Metric>,
) -> BenchResult<()> {
    use kartis::render::raster::*;
    let mut fonts = FontRegistry::new();
    fonts.register(
        include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    metrics.push(measure("raster_rgba", size, iterations, || {
        let image = render_raster(scene, &fonts, &RasterOptions::default())?;
        assert_eq!((image.width, image.height), (1200, 720));
        let bytes = image.rgba.len();
        black_box(image);
        Ok(bytes)
    })?);
    Ok(())
}

#[cfg(not(feature = "raster"))]
fn raster_metrics(_: &Scene, _: usize, _: usize, _: &mut Vec<Metric>) -> BenchResult<()> {
    Ok(())
}

fn main() -> BenchResult<()> {
    let mut report = None;
    let mut iterations = 20usize;
    let mut machine =
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "CPU unspecified".into());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--report" => {
                report = Some(PathBuf::from(
                    args.next().ok_or("--report requires a path")?,
                ))
            }
            "--iterations" => {
                iterations = args
                    .next()
                    .ok_or("--iterations requires a count")?
                    .parse()?
            }
            "--machine" => machine = args.next().ok_or("--machine requires a description")?,
            _ => {
                return Err(format!(
                    "unknown argument {arg}; use --report PATH [--iterations 20] [--machine NAME]"
                )
                .into());
            }
        }
    }
    if !(1..=200).contains(&iterations) {
        return Err("iterations must be 1..=200".into());
    }
    let report = report
        .ok_or("provide --report with an absolute path in the actual Cargo target directory")?;
    if !report.is_absolute() {
        return Err("--report must be an absolute path".into());
    }
    let rustc = std::process::Command::new("rustc").arg("-Vv").output()?;
    if !rustc.status.success() {
        return Err("rustc -Vv failed".into());
    }
    let mut output = format!(
        "# Kartis native density benchmark\n\nMachine: {machine}; OS/arch: {}/{}; available logical CPUs: {}.\n\n```text\n{}```\n\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::thread::available_parallelism()?.get(),
        String::from_utf8(rustc.stdout)?
    );
    writeln!(
        output,
        "Release profile: {}; raster feature: {}; Unix time seconds: {}.\n",
        !cfg!(debug_assertions),
        cfg!(feature = "raster"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    )?;
    writeln!(
        output,
        "First = first call for this operation/fixture in this process (not OS-cache cold). Then 2 untimed warmups and {iterations} individual timed samples. Timer includes result allocation and destruction; fixtures, independent correctness oracles and report I/O are outside timed samples. Mesh query sample = 2,000 nearest lookups; every other sample = one complete operation.\n"
    )?;
    let mut metrics = Vec::new();
    let cfg = config();
    for size in [1_000, 5_000, 10_000] {
        let data = series(size);
        let options = ScatterOptions {
            radius: 2.,
            ..Default::default()
        };
        metrics.push(measure("scatter_layout", size, iterations, || {
            let scene = scatter(&cfg, black_box(&data), &options)?;
            let marks = scene.marks.len();
            black_box(scene);
            Ok(marks)
        })?);
        let scene = scatter(&cfg, &data, &options)?;
        assert_eq!(
            scene.marks.iter().filter(|m| !m.label.is_empty()).count(),
            size
        );
        metrics.push(measure("scatter_svg", size, iterations, || {
            let svg = to_svg(black_box(&scene))?;
            let bytes = svg.len();
            black_box(svg);
            Ok(bytes)
        })?);
        raster_metrics(&scene, size, iterations, &mut metrics)?;
    }
    metrics.push(measure(
        "scene_build_validate",
        MAX_MARKS,
        iterations,
        || {
            let scene = dense_scene(MAX_MARKS)?;
            let marks = scene.marks.len();
            black_box(scene);
            Ok(marks)
        },
    )?);
    let boundary = dense_scene(MAX_MARKS)?;
    metrics.push(measure("scene_validate", MAX_MARKS, iterations, || {
        black_box(&boundary).validate()?;
        Ok(boundary.marks.len())
    })?);
    metrics.push(measure("boundary_svg", MAX_MARKS, iterations, || {
        let svg = to_svg(black_box(&boundary))?;
        let bytes = svg.len();
        black_box(svg);
        Ok(bytes)
    })?);
    raster_metrics(&boundary, MAX_MARKS, iterations, &mut metrics)?;
    let query_points = queries(2_000);
    for size in [1_000, 5_000, 10_000, MAX_MARKS] {
        let points = dense_scene(size)?
            .marks
            .into_iter()
            .map(|m| {
                let Geometry::Circle { x, y, .. } = m.geometry else {
                    unreachable!()
                };
                MeshPoint {
                    id: m.id,
                    x,
                    y,
                    clip: None,
                }
            })
            .collect::<Vec<_>>();
        metrics.push(measure("mesh_build", size, iterations, || {
            let mesh = PointMesh::new(black_box(points.clone()))?;
            let count = mesh.points().len();
            black_box(mesh);
            Ok(count)
        })?);
        let mesh = PointMesh::new(points)?;
        verify_mesh(&mesh, &query_points);
        metrics.push(measure("mesh_2000_queries", size, iterations, || {
            let mut checksum = 0;
            for &(x, y) in &query_points {
                checksum += black_box(
                    mesh.nearest(black_box(x), black_box(y), f64::INFINITY)
                        .unwrap(),
                )
                .id
                .len();
            }
            Ok(checksum)
        })?);
    }
    writeln!(
        output,
        "| Operation | Data points/marks | First ms | Warm median ms | Warm p95 ms | Warm min ms | Warm max ms | Output units |\n|---|---:|---:|---:|---:|---:|---:|---:|"
    )?;
    for m in metrics {
        let n = m.samples.len();
        let median = if n % 2 == 0 {
            (m.samples[n / 2 - 1] + m.samples[n / 2]) / 2.
        } else {
            m.samples[n / 2]
        };
        let p95 = m.samples[(95 * n).div_ceil(100) - 1];
        writeln!(
            output,
            "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {} |",
            m.operation,
            m.size,
            m.first,
            median,
            p95,
            m.samples[0],
            m.samples[n - 1],
            m.output_units
        )?;
    }
    writeln!(
        output,
        "\nOutput units: layout/build/validate = marks or sites; SVG = UTF-8 bytes; raster = RGBA bytes; queries = ID-length checksum (not a byte allocation). Mesh oracle: 32 brute-force exact distance/lexical-ID checks per size, 128 total, zero mismatches.\n\n## Verified rejections\n"
    )?;
    rejected(
        "scatter 10,001 points",
        scatter(&cfg, &series(10_001), &ScatterOptions::default()),
        &mut output,
    )?;
    let mut invalid = boundary.clone();
    let mut extra = invalid.marks[0].clone();
    extra.id = "over-budget".into();
    invalid.marks.push(extra);
    rejected("Scene 20,001 marks", invalid.validate(), &mut output)?;
    rejected("SVG 20,001 marks", to_svg(&invalid), &mut output)?;
    let mut too_many = vec![
        MeshPoint {
            id: "over-budget".into(),
            x: 0.,
            y: 0.,
            clip: None
        };
        MAX_MARKS + 1
    ];
    // Count rejection precedes duplicate IDs, so this exercises the actual budget.
    too_many[0].id = "first".into();
    rejected(
        "PointMesh 20,001 sites",
        PointMesh::new(too_many),
        &mut output,
    )?;
    #[cfg(feature = "raster")]
    {
        use kartis::render::raster::*;
        let mut fonts = FontRegistry::new();
        fonts.register(
            include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
            0,
        )?;
        rejected(
            "raster operation budget 1",
            render_raster(
                &boundary,
                &fonts,
                &RasterOptions {
                    max_operations: 1,
                    ..Default::default()
                },
            ),
            &mut output,
        )?;
        rejected(
            "raster pixel budget 1",
            render_raster(
                &boundary,
                &fonts,
                &RasterOptions {
                    max_pixels: 1,
                    ..Default::default()
                },
            ),
            &mut output,
        )?;
    }
    writeln!(
        output,
        "\nNo process RSS/peak memory sampling, browser DOM/Canvas timings, frame rates or cross-machine Nivo comparisons were performed. Raster uses explicitly embedded ABeeZee-Regular.ttf and default RasterOptions at DPR 1; PNG compression and file output are excluded. Scene SVG timings include validation; raster includes validation, buffer allocation, shaping, drawing, unpremultiplication and destruction.\n"
    )?;
    if let Some(parent) = report.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&report, &output)?;
    print!("{output}");
    eprintln!("Report: {}", report.display());
    Ok(())
}
