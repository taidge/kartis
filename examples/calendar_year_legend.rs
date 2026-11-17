//! Export explicit year-label positions and real-font outlined text.
use std::fmt::Write;
use std::path::PathBuf;

use kartis::render::raster::*;
use kartis::{render, *};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let config = ChartConfig {
        width: 500.,
        height: 400.,
        margin: Margin {
            top: 60.,
            right: 60.,
            bottom: 60.,
            left: 60.,
        },
        ..Default::default()
    };
    let family = FontFamily::Named("YearFixture".into());
    let mut fonts = FontRegistry::new();
    let face = fonts.register(
        include_bytes!("../tests/fonts/Lato-Regular.ttf").as_slice(),
        0,
    )?;
    fonts.bind_face(
        face,
        FaceDescriptor {
            family: family.clone(),
            weight: 400.,
        },
    )?;
    let mut frames = Vec::new();
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Calendar year labels</title><style>section{display:flex}svg,canvas{width:500px;height:400px}</style>",
    );
    for (orientation, direction) in [
        (CalendarOrientation::Horizontal, "horizontal"),
        (CalendarOrientation::Vertical, "vertical"),
    ] {
        for (position, position_name) in [
            (CalendarLegendPosition::Before, "before"),
            (CalendarLegendPosition::After, "after"),
        ] {
            for offset in [-12., 0., 20.] {
                for spacing in [0., 3.] {
                    let name = format!("{direction}-{position_name}-{offset}-{spacing}");
                    let options = CalendarLayoutOptions {
                        mode: CalendarLayoutMode::FullYears,
                        orientation,
                        start: Some((2023, 1, 1)),
                        end: Some((2024, 12, 31)),
                        month_spacing: spacing,
                        month_labels: false,
                        weekday_ticks: vec![],
                        year_style: CalendarTextStyle {
                            size: Some(14.),
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    let legend = CalendarYearLegendOptions { position, offset };
                    let layout = calendar_layout_with_year_legend(
                        &config,
                        &[],
                        &options,
                        Some(&legend),
                        Some(&mut |ctx| Ok(format!("FY {}", ctx.year))),
                    )?;
                    let mut scene = layout.scene.clone();
                    assert_eq!(scene.labels.len(), 2);
                    scene.typography.default = FontOptions {
                        family: Some(vec![family.clone()]),
                        weight: Some(400.),
                    };
                    std::fs::write(
                        out.join(format!("{name}-chart.svg")),
                        render::to_svg(&scene)?,
                    )?;
                    scene.marks.clear();
                    scene.legend.clear();
                    scene.draw_order.clear();
                    let labels = scene.labels.iter().map(|l| serde_json::json!({"text":l.text,"x":l.x,"y":l.y,"size":l.size,"rotation":l.rotation})).collect::<Vec<_>>();
                    let years = layout.panels.iter().map(|p| serde_json::json!({"year":p.year,"bbox":{"x":p.grid.x,"y":p.grid.y,"width":p.grid.width,"height":p.grid.height}})).collect::<Vec<_>>();
                    let image = render_raster(&scene, &fonts, &RasterOptions::default())?;
                    let mut ink = [image.width as usize, image.height as usize, 0, 0];
                    for (i, rgba) in image.rgba.as_chunks::<4>().0.iter().enumerate() {
                        if rgba[0] < 180 && rgba[1] < 180 && rgba[2] < 180 && rgba[3] > 128 {
                            let x = i % image.width as usize;
                            let y = i / image.width as usize;
                            ink[0] = ink[0].min(x);
                            ink[1] = ink[1].min(y);
                            ink[2] = ink[2].max(x + 1);
                            ink[3] = ink[3].max(y + 1);
                        }
                    }
                    assert!(ink[2] > ink[0] && ink[3] > ink[1]);
                    let outlined = scene_with_painted_labels(
                        &scene,
                        &fonts,
                        &PaintedLabelOptions {
                            rotated_only: false,
                            ..Default::default()
                        },
                        &name,
                    )?;
                    let svg = render::to_svg_with_namespace(&outlined, &name)?;
                    std::fs::write(out.join(format!("{name}.svg")), &svg)?;
                    std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
                    write!(
                        html,
                        "<section id=\"{name}\">{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{}</script>",
                        render::to_canvas_script(&outlined, &format!("canvas-{name}"))?
                    )?;
                    frames.push(serde_json::json!({"name":name,"direction":direction,"position":position_name,"offset":offset,"spacing":spacing,"years":years,"labels":labels,"ink":ink}));
                }
            }
        }
    }
    write!(
        html,
        "<script id=\"frames\" type=\"application/json\">{}</script>",
        serde_json::to_string(&frames)?
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(out.join("frames.json"), serde_json::to_vec_pretty(&frames)?)?;
    Ok(())
}
