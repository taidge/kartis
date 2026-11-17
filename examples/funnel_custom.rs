//! Unreleased stage factories: export one Scene through SVG, Canvas and CPU.
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
        ..Default::default()
    };
    let data = [
        FunnelDatum::new("visits", 80.),
        FunnelDatum::new("trials", 40.),
        FunnelDatum::new("sales", 20.),
    ];
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Custom Funnel stages</title><style>section{display:flex}svg,canvas{width:320px;height:240px}</style>",
    );
    let mut frames = Vec::new();
    for (direction, orientation) in [
        (FunnelDirection::Vertical, "vertical"),
        (FunnelDirection::Horizontal, "horizontal"),
    ] {
        for active in [false, true] {
            let name = format!("{orientation}-{active}");
            let options = FunnelLayoutOptions {
                direction,
                labels: None,
                legend: false,
                before: None,
                after: None,
                current: active.then(|| "trials".into()),
                current_size_extension: 4.,
                ..Default::default()
            };
            let layout = funnel_layout_with_stage_factory(
                &config,
                &data,
                &options,
                &mut |ctx: &FunnelStageContext<'_>| {
                    let x = ctx.part.center[0] + 18.;
                    let y = ctx.part.center[1];
                    let r = if ctx.part.is_current { 14. } else { 10. };
                    let mut stage = ctx.inherit(Geometry::Path(vec![
                        C::Move(P::new(x, y - r)),
                        C::Line(P::new(x + r, y)),
                        C::Line(P::new(x, y + r)),
                        C::Line(P::new(x - r, y)),
                        C::Close,
                    ]));
                    stage.retain_profile_border = false;
                    Ok(stage)
                },
            )?;
            let mut samples = Vec::new();
            let image = render_raster(
                &layout.scene,
                &FontRegistry::new(),
                &RasterOptions::default(),
            )?;
            for part in &layout.parts {
                let point = [part.center[0] + 18., part.center[1]];
                assert_eq!(
                    layout.scene.hit_test(point[0], point[1]).map(|m| &m.id),
                    Some(&part.mark_id)
                );
                let x = point[0].floor() as usize;
                let y = point[1].floor() as usize;
                let index = (y * image.width as usize + x) * 4;
                samples.push(serde_json::json!({"point":point,"anchor":part.center,"rgba":image.rgba[index..index+4],"mark":part.mark_id}));
                let edge = [point[0] + 12., point[1]];
                assert_eq!(
                    layout.scene.hit_test(edge[0], edge[1]).is_some(),
                    part.is_current
                );
                let index = (edge[1].floor() as usize * image.width as usize
                    + edge[0].floor() as usize)
                    * 4;
                samples.push(serde_json::json!({"point":edge,"rgba":image.rgba[index..index+4]}));
            }
            let svg = render::to_svg_with_namespace(&layout.scene, &name)?;
            let script = render::to_canvas_script(&layout.scene, &format!("canvas-{name}"))?;
            std::fs::write(out.join(format!("{name}.svg")), &svg)?;
            std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
            write!(
                html,
                "<section id=\"{name}\">{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{script}</script>"
            )?;
            frames.push(serde_json::json!({"name":name,"samples":samples}));
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
