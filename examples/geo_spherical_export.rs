//! Standalone example: --features raster,geojson and an explicit output directory.
#[path = "shared/geo_spherical.rs"]
mod geo_spherical;
use std::fmt::Write;
use std::path::PathBuf;

use kartis::render::raster::*;
use kartis::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("provide output directory")?);
    let mut offscreen = false;
    let mut fallback = render::CanvasScriptFallback::Error;
    let mut fallback_requested = false;
    let mut family = None;
    while let Some(option) = args.next() {
        let value = args.next().ok_or("each option needs a value")?;
        match option.as_str() {
            "--surface" => {
                offscreen = match value.as_str() {
                    "direct" => false,
                    "offscreen" => true,
                    _ => return Err("surface must be direct or offscreen".into()),
                }
            }
            "--fallback" => {
                fallback_requested = true;
                fallback = match value.as_str() {
                    "error" => render::CanvasScriptFallback::Error,
                    "direct" => render::CanvasScriptFallback::Direct,
                    _ => return Err("fallback must be error or direct".into()),
                };
            }
            "--font" => {
                family = Some(match value.as_str() {
                    "sans" => FontFamily::Sans,
                    "system" => FontFamily::System,
                    _ => return Err("font must be sans or system".into()),
                })
            }
            _ => return Err(format!("unknown option {option}").into()),
        }
    }
    if fallback_requested && !offscreen {
        return Err("--fallback requires --surface offscreen".into());
    }
    let canvas_options = render::CanvasScriptOptions {
        surface: if offscreen {
            render::CanvasScriptSurface::Offscreen2d { fallback }
        } else {
            render::CanvasScriptSurface::Direct
        },
        ..Default::default()
    };
    std::fs::create_dir_all(&out)?;
    let mut fonts = FontRegistry::new();
    let font = fonts.register(
        include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    if let Some(family) = &family {
        // Native export binds the caller-selected generic alias to supplied
        // bytes. Browser generic-family resolution remains engine-specific.
        fonts.bind_face(
            font,
            FaceDescriptor {
                family: family.clone(),
                weight: 400.,
            },
        )?;
    }
    let c = ChartConfig {
        width: 720.,
        height: 400.,
        ..Default::default()
    };
    let mut html = String::from(
        "<!doctype html><html lang=en><meta charset=utf-8><title>Spherical public math</title><style>body{background:#eef2f6;font:15px sans-serif}article{background:white;margin:20px;padding:10px}.svg-view,canvas{width:720px}.canvas-view{display:none}</style><main>",
    );
    let mut reports = Vec::new();
    for (i, (title, mut scene)) in geo_spherical::gallery_scenes(&c)?.into_iter().enumerate() {
        let index = i + 246;
        scene.title = title.into();
        if let Some(family) = &family {
            scene.typography.default.family = Some(vec![family.clone()]);
        }
        let svg = render::to_svg(&scene)?;
        std::fs::write(out.join(format!("chart-{index}.svg")), &svg)?;
        let inline = render::to_svg_with_namespace(&scene, &format!("spherical-{index}"))?;
        let id = format!("canvas-{index}");
        let script = render::to_canvas_script_with_options(&scene, &id, &canvas_options)?;
        write!(
            html,
            "<article data-index=\"{index}\"><h2>{title}</h2><div class=svg-view>{inline}</div><div class=canvas-view><canvas id=\"{id}\"></canvas></div></article><script>{script}</script>"
        )?;
        for dpr in [1., 2.] {
            let image = render_raster(
                &scene,
                &fonts,
                &RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
            )?;
            let png = image.png()?;
            let decoder = png::Decoder::new(std::io::Cursor::new(&png));
            let mut reader = decoder.read_info()?;
            let mut data = vec![0; reader.output_buffer_size().ok_or("png size")?];
            let info = reader.next_frame(&mut data)?;
            if data[..info.buffer_size()] != image.rgba {
                return Err("decoded PNG differs from CPU raster".into());
            }
            let color = if i == 0 {
                [36, 126, 144, 255]
            } else {
                [89, 114, 192, 255]
            };
            let count = image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| **p == color)
                .count();
            if count < 5000 {
                return Err("missing actual filled spherical shape pixels".into());
            }
            std::fs::write(
                out.join(format!("chart-{index}-cpu-dpr{}.png", dpr as u8)),
                png,
            )?;
            reports.push(serde_json::json!({"index":index,"dpr":dpr,"width":image.width,"height":image.height,"actualDecodedPng":true,"fillColor":color,"exactFillPixels":count,"marks":scene.marks.len()}));
        }
    }
    html.push_str("</main></html>");
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(
        out.join("cpu-report.json"),
        serde_json::to_string_pretty(
            &serde_json::json!({"passed":true,"scope":"Actual shared Rust Scene SVG/CanvasScript/CPU PNG outputs. No OS window or pointer claim.","rows":reports}),
        )?,
    )?;
    println!("spherical consumers exported to {}", out.display());
    Ok(())
}
