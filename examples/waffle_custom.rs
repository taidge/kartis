//! Unreleased typed cell factories, exported through three shared renderers.
use std::fmt::Write;
use std::path::PathBuf;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::{render, *};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let config = ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 20.,
            left: 20.,
        },
        ..Default::default()
    };
    let data = [RadialDatum::new("one", 3.), RadialDatum::new("two", 2.)];
    let mut frames = Vec::new();
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Custom Waffle cells</title><style>section{display:flex}svg,canvas{width:320px;height:240px}</style>",
    );
    for (direction, name) in [
        (WaffleFillDirection::Top, "top"),
        (WaffleFillDirection::Right, "right"),
        (WaffleFillDirection::Bottom, "bottom"),
        (WaffleFillDirection::Left, "left"),
    ] {
        for mode in [WaffleMode::Cells, WaffleMode::Areas] {
            for hidden in [false, true] {
                let name = format!("{name}-{mode:?}-{hidden}");
                let options = WaffleLayoutOptions {
                    rows: 2,
                    columns: 3,
                    total: Some(6.),
                    direction,
                    mode,
                    padding: 6.,
                    corner_radius: 4.,
                    hidden: if hidden {
                        ["one".into()].into()
                    } else {
                        Default::default()
                    },
                    ..Default::default()
                };
                let mut calls = 0;
                let layout =
                    waffle_layout_with_cell_factory(&config, &data, &options, &mut |ctx| {
                        calls += 1;
                        let [x, y] = [ctx.position[0] + 18., ctx.position[1]];
                        let mut result = ctx.inherit(Geometry::Path(vec![
                            C::Move(P::new(x - 7., y - 7.)),
                            C::Line(P::new(x + 7., y - 7.)),
                            C::Line(P::new(x + 7., y + 7.)),
                            C::Line(P::new(x - 7., y + 7.)),
                            C::Close,
                        ]));
                        result.fill = Some(if ctx.group.is_some() {
                            Color(210, 40, 90, 255)
                        } else {
                            Color(35, 110, 200, 255)
                        });
                        result.fill_paint = None;
                        Ok(result)
                    })?;
                assert_eq!(
                    calls,
                    layout.cells.iter().filter(|c| c.mark_id.is_some()).count()
                );
                let image = render_raster(
                    &layout.scene,
                    &FontRegistry::new(),
                    &RasterOptions::default(),
                )?;
                let mut samples = Vec::new();
                for cell in &layout.cells {
                    let center = [
                        cell.bounds.x + cell.bounds.width / 2.,
                        cell.bounds.y + cell.bounds.height / 2.,
                    ];
                    for (point, offset) in [(center, false), ([center[0] + 18., center[1]], true)] {
                        let expected = if offset || cell.mark_id.is_none() {
                            cell.mark_id.as_deref().or_else(|| {
                                cell.owner
                                    .and_then(|o| layout.groups[o].area_mark_id.as_deref())
                            })
                        } else {
                            None
                        };
                        assert_eq!(
                            layout
                                .scene
                                .hit_test(point[0], point[1])
                                .map(|m| m.id.as_str()),
                            expected
                        );
                        let index = (point[1].floor() as usize * image.width as usize
                            + point[0].floor() as usize)
                            * 4;
                        samples.push(
                            serde_json::json!({"point":point,"rgba":image.rgba[index..index+4]}),
                        );
                    }
                }
                let svg = render::to_svg_with_namespace(&layout.scene, &name)?;
                let script = render::to_canvas_script(&layout.scene, &format!("canvas-{name}"))?;
                std::fs::write(out.join(format!("{name}.svg")), &svg)?;
                std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
                write!(
                    html,
                    "<section id=\"{name}\">{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{script}</script>"
                )?;
                frames.push(serde_json::json!({"name":name,"samples":samples,"callbacks":calls}));
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
