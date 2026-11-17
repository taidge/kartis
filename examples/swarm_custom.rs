//! Export the same typed custom Swarm frame through SVG, Canvas and CPU raster.
#[path = "shared/swarm_custom.rs"]
mod model;
use std::fmt::Write;
use std::path::PathBuf;

use kartis::render;
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Custom Swarm nodes</title><style>section{display:flex}svg,canvas{display:block}</style><body>",
    );
    let mut frames = vec![];
    for width in [640., 480.] {
        for vertical in [false, true] {
            for phase in [0, 1] {
                let name = format!("{}-{vertical}-{phase}", width as u32);
                let layout = model::build(width, 360., vertical, phase, 0)?;
                let svg = render::to_svg_with_namespace(&layout.scene, &format!("swarm-{name}"))?;
                std::fs::write(out.join(format!("{name}.svg")), &svg)?;
                let script = render::to_canvas_script(&layout.scene, &format!("canvas-{name}"))?;
                write!(
                    html,
                    "<section id=\"{name}\">{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{script}</script>"
                )?;
                let mut samples = vec![];
                for dpr in [1., 2.] {
                    let image = render_raster(
                        &layout.scene,
                        &FontRegistry::new(),
                        &RasterOptions {
                            pixel_ratio: dpr,
                            ..Default::default()
                        },
                    )?;
                    std::fs::write(out.join(format!("{name}-{dpr}.png")), image.png()?)?;
                    let mut points = vec![([4., 4.], "background".to_owned())];
                    for site in &layout.sites {
                        let probe = model::ink_probe(site);
                        if site.node_id != "zero" {
                            assert_eq!(
                                layout.scene.hit_test(probe[0], probe[1]).map(|m| &m.id),
                                Some(&site.mark_id)
                            );
                        }
                        points.push((probe, site.node_id.clone()));
                        if site.node_id == "offset" {
                            points.push((site.position, "original-anchor-no-ink".into()));
                        }
                        if site.node_id == "dash" {
                            points.push((
                                [site.position[0] - 4., site.position[1]],
                                "dash-gap".into(),
                            ));
                        }
                    }
                    for (point, label) in points {
                        let x = (point[0] * dpr).floor() as usize;
                        let y = (point[1] * dpr).floor() as usize;
                        let index = (y * image.width as usize + x) * 4;
                        samples.push(serde_json::json!({"dpr":dpr,"point":point,"label":label,"rgba":image.rgba[index..index+4]}));
                    }
                }
                frames.push(serde_json::json!({"name":name,"width":width,"height":360,"vertical":vertical,"phase":phase,"samples":samples,"sites":layout.sites.iter().map(|s|serde_json::json!({"id":s.mark_id,"node":s.node_id,"position":s.position,"inkRadius":s.ink_radius})).collect::<Vec<_>>() }));
            }
        }
    }
    write!(
        html,
        "<script id=\"frames\" type=\"application/json\">{}</script></body></html>",
        serde_json::to_string(&frames)?
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(out.join("frames.json"), serde_json::to_vec_pretty(&frames)?)?;
    Ok(())
}
