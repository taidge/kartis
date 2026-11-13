use std::fs;
use std::path::PathBuf;

#[cfg(feature = "raster")]
use kartis::render::raster::{FaceDescriptor, FontRegistry, RasterOptions, render_raster};
use kartis::*;

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c <= '\u{1f}' => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output.push('"');
    output
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let axes = ["speed", "cost <&> Å café", "risk"].map(str::to_owned);
    let data = [
        RadarSeries::new("Revenue", vec![7., 7., 3.]),
        RadarSeries::new("Costs", vec![5., 2., 1.]),
    ];
    let mut config = ChartConfig::default();
    config.theme.font.family = Some(vec![FontFamily::Named("Lato".into())]);
    let mut options = RadarLayoutOptions {
        domain: Some([0., 10.]),
        ..Default::default()
    };
    options.points.labels = true;
    options.points.value_format = ValueFormat::Fixed(1);
    let prefix = String::from("Team <&> Å café");
    let mut contexts = vec![];
    let mut point_data = vec![];
    let layout = radar_layout_with_value_scale_and_point_and_label_factories(
        &config,
        &axes,
        &data,
        &options,
        &RadarValueScaleOptions {
            kind: RadarValueScaleKind::SymLog { constant: 1. },
            ..Default::default()
        },
        &Default::default(),
        |c| {
            Ok(c.inherit(Geometry::Circle {
                x: c.position[0],
                y: c.position[1],
                radius: 4.,
            }))
        },
        |c| {
            contexts.push(format!("{}/{}={}", c.key, c.index, c.value));
            let text = format!("{prefix} {}/{}: {}", c.key, c.index, c.formatted_value);
            let fill = c.point.fill.unwrap();
            point_data.push(format!(
                "{{\"id\":{},\"index\":{},\"key\":{},\"value\":{},\"formattedValue\":{},\"position\":[{},{}],\"text\":{},\"fill\":[{},{},{},{}]}}",
                json_string(c.point.mark_id), json_string(c.index), json_string(c.key), c.value,
                json_string(c.formatted_value), c.point.position[0], c.point.position[1], json_string(&text),
                fill.0, fill.1, fill.2, fill.3,
            ));
            Ok(text)
        },
    )?;
    assert_eq!(contexts.len(), 6);
    assert_eq!(layout.layout.interaction_binding().sites.len(), 6);
    let app = glory::App::memory();
    app.mount(chart(layout.layout.scene.clone()))?;
    app.flush()?;
    let svg = render::to_svg(&layout.layout.scene)?;
    assert!(svg.contains("Team &lt;&amp;&gt; Å café"));
    let canvas = render::to_canvas_script(&layout.layout.scene, "radar-point-labels")?;
    if let Some(dir) = std::env::args().nth(1).map(PathBuf::from) {
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("radar.svg"), &svg)?;
        fs::write(dir.join("radar.js"), &canvas)?;
        fs::write(dir.join("contexts.txt"), contexts.join("\n"))?;
        fs::write(
            dir.join("metadata.json"),
            format!(
                "{{\"schema\":\"radar-point-label-example-v1\",\"width\":{},\"height\":{},\"domain\":[0,10],\"scale\":\"symlog-constant1\",\"axisIds\":[{}],\"source\":[{}],\"points\":[{}]}}\n",
                config.width,
                config.height,
                axes.iter()
                    .map(|s| json_string(s))
                    .collect::<Vec<_>>()
                    .join(","),
                data.iter()
                    .map(|d| format!(
                        "{{\"id\":{},\"values\":[{}]}}",
                        json_string(&d.id),
                        d.values
                            .iter()
                            .map(f64::to_string)
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .collect::<Vec<_>>()
                    .join(","),
                point_data.join(","),
            ),
        )?;
        fs::write(
            dir.join("Lato-Regular.ttf"),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fonts/Lato-Regular.ttf"
            )),
        )?;
        fs::write(
            dir.join("index.html"),
            format!(
                "<!doctype html><meta charset=utf-8><style>@font-face{{font-family:Lato;src:url(Lato-Regular.ttf)}}</style>{svg}<canvas id=radar-point-labels width={} height={}></canvas><script type=module>await document.fonts.load('12px Lato');await document.fonts.ready;const source=await(await fetch('radar.js')).text();new Function(source)();window.radarPointLabelsReady=true;</script>",
                config.width, config.height
            ),
        )?;
        #[cfg(feature = "raster")]
        {
            let mut fonts = FontRegistry::new();
            let face = fonts.register(
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/fonts/Lato-Regular.ttf"
                ))
                .as_slice(),
                0,
            )?;
            fonts.bind_face(
                face,
                FaceDescriptor {
                    family: FontFamily::Named("Lato".into()),
                    weight: 400.,
                },
            )?;
            let image = render_raster(&layout.layout.scene, &fonts, &RasterOptions::default())?;
            fs::write(dir.join("radar.png"), image.png()?)?;
        }
    }
    println!(
        "6 borrowed point-label contexts; real Memory/SVG/Canvas{} export passed",
        if cfg!(feature = "raster") { "/CPU" } else { "" }
    );
    Ok(())
}
