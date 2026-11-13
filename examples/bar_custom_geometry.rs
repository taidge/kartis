use std::{fmt::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::args().nth(1).ok_or("output directory")?);
    std::fs::create_dir_all(&out)?;
    use kartis::*;
    use render::raster::*;
    let mut fonts = FontRegistry::new();
    fonts.register(
        include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    let mut html = String::from("<!doctype html><meta charset=utf-8><h1>Typed custom bars</h1>");
    let mut reports = Vec::new();
    for horizontal in [false, true] {
        for stacked in [false, true] {
            let index = usize::from(horizontal) * 2 + usize::from(stacked);
            let c = ChartConfig {
                width: 420.,
                height: 260.,
                margin: Margin {
                    left: 64.,
                    right: 24.,
                    top: 32.,
                    bottom: 52.,
                },
                ..Default::default()
            };
            let categories = ["A", "B", "C"].map(str::to_string);
            let data = [
                BarSeries::new("s", vec![Some(4.), Some(-2.), Some(0.)]),
                BarSeries::new("other", vec![None, Some(3.), Some(1.)]),
            ];
            let o = BarOptions {
                horizontal,
                layout: if stacked {
                    BarLayout::Stacked
                } else {
                    BarLayout::Grouped
                },
                ..Default::default()
            };
            let style = BarStyleOptions {
                category_scale: Some(scales::BandScaleOptions {
                    padding_inner: 0.2,
                    padding_outer: 0.2,
                    round: true,
                    ..Default::default()
                }),
                labels: BarLabels {
                    enabled: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let layout = bar_layout_with_bar_factory(&c, &categories, &data, &o, &style, |ctx| {
                use glory::render::{
                    GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap,
                };
                let [x, y] = ctx.position;
                let mut g = ctx.inherit(Geometry::Path(vec![
                    C::Move(P::new(x - 14., y - 12.)),
                    C::Line(P::new(x + 14., y - 12.)),
                    C::Line(P::new(x + 20., y)),
                    C::Line(P::new(x + 14., y + 12.)),
                    C::Line(P::new(x - 14., y + 12.)),
                    C::Close,
                ]));
                if ctx.category == "A" {
                    g.fill_paint = Some(FillPaint::Linear {
                        start: [x - 14., y],
                        end: [x + 14., y],
                        stops: vec![
                            GradientStop::new(0., Color(230, 60, 20, 255)),
                            GradientStop::new(1., Color(20, 80, 220, 255)),
                        ],
                    });
                } else if ctx.category == "B" {
                    g.fill_paint = Some(FillPaint::Pattern(PatternFill {
                        shape: PatternShape::Lines { width: 2. },
                        spacing: 6.,
                        foreground: Color(20, 70, 110, 255),
                        background: Color(235, 245, 250, 255),
                        ..Default::default()
                    }));
                } else {
                    g.fill = None;
                    g.stroke = Some(GraphicStroke {
                        color: Color(180, 35, 130, 255),
                        width: 3.,
                        cap: StrokeCap::Round,
                    });
                    g.stroke_dash = Some(StrokeDash {
                        segments: vec![4., 2.],
                        offset: 0.,
                    });
                }
                Ok(g)
            })?;
            let binding = layout.interaction_binding()?;
            let scene = &layout.layout.scene;
            let svg = render::to_svg_with_namespace(scene, &format!("factory{index}"))?;
            let js = render::to_canvas_script(scene, &format!("cv{index}"))?;
            write!(
                html,
                "<article data-index=\"{index}\"><div>{svg}</div><canvas id=\"cv{index}\"></canvas></article><script>{js}</script>"
            )?;
            for dpr in [1., 2.] {
                let image = render_raster(
                    scene,
                    &fonts,
                    &RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )?;
                let colors = image.rgba.as_chunks::<4>().0.iter();
                let ink = colors
                    .filter(|p| p[3] > 0 && (p[0] < 220 || p[1] < 220 || p[2] < 220))
                    .count();
                assert!(ink > 500);
                std::fs::write(
                    out.join(format!("custom-{index}-dpr{}.png", dpr as usize)),
                    image.png()?,
                )?;
                reports.push(format!(
                    "{{\"index\":{index},\"dpr\":{dpr},\"inkPixels\":{ink},\"sourceBars\":{}}}",
                    binding.sites.len()
                ));
            }
        }
    }
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(
        out.join("cpu-report.json"),
        format!("[{}]", reports.join(",")),
    )?;
    println!("4 typed custom bar scenes and 8 CPU PNG");
    Ok(())
}
