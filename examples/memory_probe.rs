//! Paused, process-owned memory checkpoints. Run through scripts/native-memory.ps1.
//! The parent samples OS process counters while each real output remains alive.
use kartis::{mesh::PointMesh, *};
use std::io::{BufRead, Write};

type ProbeResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct OutputSizes {
    marks: usize,
    svg: usize,
    mesh: usize,
    rgba: usize,
    png: usize,
}
fn checkpoint(stage: &str, cycle: usize, count: usize, sizes: &OutputSizes) -> ProbeResult<()> {
    println!(
        "{{\"stage\":\"{stage}\",\"pid\":{},\"cycle\":{cycle},\"count\":{count},\"marks\":{},\"svgBytes\":{},\"meshPoints\":{},\"rgbaBytes\":{},\"pngBytes\":{},\"raster\":{}}}",
        std::process::id(),
        sizes.marks,
        sizes.svg,
        sizes.mesh,
        sizes.rgba,
        sizes.png,
        cfg!(feature = "raster")
    );
    std::io::stdout().flush()?;
    let mut input = String::new();
    let read = std::io::stdin().lock().read_line(&mut input)?;
    if read == 0 || input.trim() != "sampled" {
        return Err("memory probe requires its sampling parent acknowledgment".into());
    }
    Ok(())
}
fn scene(count: usize) -> Result<Scene> {
    let config = ChartConfig {
        width: 1200.,
        height: 720.,
        title: "Native memory checkpoints".into(),
        ..Default::default()
    };
    let mut scene = config.scene()?;
    for i in 0..count {
        let mut mark = Mark::new(
            format!("memory-{i:05}"),
            Geometry::Circle {
                x: 10. + ((i * 7919) % 118001) as f64 / 100.,
                y: 30. + ((i * 104729) % 68001) as f64 / 100.,
                radius: 2.,
            },
            Color(45, 115, 175, 255),
        );
        mark.label = format!("Datum {i}");
        mark.series = "memory".into();
        mark.value = i as f64;
        scene.marks.push(mark);
    }
    scene.labels.push(Label {
        x: 20.,
        y: 20.,
        text: "Density memory".into(),
        size: 14.,
        color: Color(20, 30, 40, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene.validate()?;
    Ok(scene)
}
#[cfg(feature = "raster")]
struct Raster {
    fonts: render::raster::FontRegistry,
}
#[cfg(feature = "raster")]
impl Raster {
    fn new() -> Result<Self> {
        let mut fonts = render::raster::FontRegistry::new();
        fonts.register(
            include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
            0,
        )?;
        Ok(Self { fonts })
    }
    fn probe(
        &self,
        scene: &Scene,
        cycle: usize,
        count: usize,
        sizes: &mut OutputSizes,
    ) -> ProbeResult<()> {
        let image = render::raster::render_raster(scene, &self.fonts, &Default::default())?;
        sizes.rgba = image.rgba.len();
        assert_eq!(sizes.rgba, 1200 * 720 * 4);
        checkpoint("rgba-held", cycle, count, sizes)?;
        let png = image.png()?;
        sizes.png = png.len();
        assert!(sizes.png > 100);
        checkpoint("png-held", cycle, count, sizes)?;
        // Consume after the checkpoint so optimizer lifetime shortening cannot
        // drop the buffers while the OS sampling parent is reading counters.
        std::hint::black_box((&image, &png));
        drop(png);
        drop(image);
        sizes.rgba = 0;
        sizes.png = 0;
        Ok(())
    }
}
#[cfg(not(feature = "raster"))]
struct Raster;
#[cfg(not(feature = "raster"))]
impl Raster {
    fn new() -> Result<Self> {
        Ok(Self)
    }
    fn probe(&self, _: &Scene, _: usize, _: usize, _: &mut OutputSizes) -> ProbeResult<()> {
        Ok(())
    }
}
fn main() -> ProbeResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let cycles = match args.as_slice() {
        [] => 2,
        [flag, value] if flag == "--cycles" => value.parse::<usize>()?,
        _ => return Err("use scripts/native-memory.ps1 [-Cycles 1..3]".into()),
    };
    if !(1..=3).contains(&cycles) {
        return Err("cycles must be in 1..=3".into());
    }
    let raster = Raster::new()?;
    let mut sizes = OutputSizes {
        marks: 0,
        svg: 0,
        mesh: 0,
        rgba: 0,
        png: 0,
    };
    checkpoint("baseline", 0, 0, &sizes)?;
    for cycle in 1..=cycles {
        for count in [1000, 5000, 10000, 20000] {
            let scene = scene(count)?;
            sizes.marks = scene.marks.len();
            assert_eq!(sizes.marks, count);
            checkpoint("scene-held", cycle, count, &sizes)?;
            let svg = render::to_svg(&scene)?;
            sizes.svg = svg.len();
            checkpoint("svg-held", cycle, count, &sizes)?;
            let mesh = PointMesh::from_scene(&scene)?;
            sizes.mesh = mesh.points().len();
            assert_eq!(sizes.mesh, count);
            checkpoint("mesh-held", cycle, count, &sizes)?;
            raster.probe(&scene, cycle, count, &mut sizes)?;
            std::hint::black_box((&scene, &svg, &mesh));
            drop(mesh);
            drop(svg);
            drop(scene);
            sizes.marks = 0;
            sizes.svg = 0;
            sizes.mesh = 0;
            checkpoint("outputs-dropped", cycle, count, &sizes)?;
        }
    }
    checkpoint("complete", cycles, 0, &sizes)?;
    Ok(())
}
