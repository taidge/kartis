//! Export shared, real-font line wrapping; requires raster and geojson.
use std::fmt::Write;
use std::path::PathBuf;

use base64::Engine as _;
use kartis::render::raster::*;
use kartis::{render, *};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let bytes = include_bytes!("../tests/fonts/ABeeZee-Regular.ttf");
    let family = FontFamily::Named("WrapFixture".into());
    let mut fonts = FontRegistry::new();
    let id = fonts.register(bytes.as_slice(), 0)?;
    fonts.bind_face(
        id,
        FaceDescriptor {
            family: family.clone(),
            weight: 400.,
        },
    )?;
    let mut scene = ChartConfig {
        width: 500.,
        height: 360.,
        ..Default::default()
    }
    .scene()?;
    scene.typography.default = FontOptions {
        family: Some(vec![family]),
        weight: Some(400.),
    };
    let mut blocks = Vec::new();
    for (text, x, y, rotation, width, mode, lines, overflow) in [
        (
            "Revenue forecast and operating income across several reporting periods",
            30.,
            35.,
            0.,
            160.,
            TextWrapMode::Words,
            8,
            TextWrapOverflow::Error,
        ),
        (
            "AVATARAVATARAVATARAVATAR",
            300.,
            35.,
            25.,
            80.,
            TextWrapMode::Graphemes,
            8,
            TextWrapOverflow::Error,
        ),
        (
            "A deliberately shortened paragraph demonstrates measured ellipsis at the final permitted line",
            30.,
            240.,
            0.,
            180.,
            TextWrapMode::Words,
            2,
            TextWrapOverflow::Ellipsis,
        ),
    ] {
        let index = scene.labels.len();
        scene.labels.push(Label {
            x,
            y,
            rotation,
            text: text.into(),
            color: Color(20, 40, 75, 255),
            size: 18.,
            anchor: TextAnchor::Start,
        });
        let result = fonts.wrap_scene_label(
            &mut scene,
            index,
            &TextWrapOptions {
                max_width: width,
                mode,
                max_lines: lines,
                overflow,
                line_height: 1.4,
                ..Default::default()
            },
            MissingGlyphPolicy::Error,
        )?;
        let labels: Vec<_> = result.labels.iter().zip(&result.measurements).map(|(label,metrics)| {
            assert!(metrics.metrics().advance <= width);
            serde_json::json!({"text":label.text,"x":label.x,"y":label.y,"rotation":label.rotation,"advance":metrics.metrics().advance})
        }).collect();
        blocks.push(serde_json::json!({"source":text,"width":width,"truncated":result.truncated,"lines":labels}));
    }
    let image = render_raster(&scene, &fonts, &RasterOptions::default())?;
    std::fs::write(out.join("wrapped.png"), image.png()?)?;
    let outlined = scene_with_painted_labels(
        &scene,
        &fonts,
        &PaintedLabelOptions {
            rotated_only: false,
            ..Default::default()
        },
        "wrap",
    )?;
    std::fs::write(out.join("wrapped.svg"), render::to_svg(&scene)?)?;
    std::fs::write(out.join("outlined.svg"), render::to_svg(&outlined)?)?;
    let font_url = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut html = format!(
        "<!doctype html><meta charset=\"utf-8\"><style>@font-face{{font-family:WrapFixture;src:url(data:font/ttf;base64,{font_url})}}svg,canvas{{border:0;width:500px;height:360px}}</style>{}<canvas id=\"wrapped\"></canvas><canvas id=\"outlined\"></canvas>",
        render::to_svg_with_namespace(&scene, "plain")?
    );
    let scripts = format!(
        "{}\n{}",
        render::to_canvas_script(&scene, "wrapped")?,
        render::to_canvas_script(&outlined, "outlined")?
    );
    write!(
        html,
        "<script id=\"render-source\" type=\"text/plain\">{scripts}</script><script id=\"blocks\" type=\"application/json\">{}</script><script>document.fonts.load('18px WrapFixture').then(()=>document.fonts.ready).then(()=>{{(0,eval)(document.getElementById('render-source').textContent);window.wrapReady=true}}).catch(e=>{{window.wrapError=String(e)}})</script>",
        serde_json::to_string(&blocks)?
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(out.join("blocks.json"), serde_json::to_vec_pretty(&blocks)?)?;
    Ok(())
}
