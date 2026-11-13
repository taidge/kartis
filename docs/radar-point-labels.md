# Radar owned point-label text

Use a borrowed `FnMut(&RadarPointLabelContext) -> Result<String>` builder to supply owned point-label text. Enable `options.points.labels`, then use:

```rust
use std::{fs, path::PathBuf};
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
```

`index` is the stable axis ID, `key` the stable series ID, and `value` the raw finite source number. `formatted_value` uses that point's configured formatter. `point` exposes the existing `RadarPointContext`, including series, axis caption, mark ID, logical position and configured paint. Its `color` retains the documented legacy polygon-swatch fallback; this does not assert Nivo palette-color equivalence under every custom style or paint profile. It does not invent an arbitrary `RawDatum` or string-property getter. Display caption overrides do not rename either identity.

The Scene APIs are `radar_with_point_label_factory` and `radar_with_value_scale_and_point_label_factory`. Typed interaction layouts use `radar_layout_with_point_label_factory` and `radar_layout_with_value_scale_and_point_label_factory`. Combine existing point geometry with text using `radar_layout_with_point_and_label_factories` or `radar_layout_with_value_scale_and_point_and_label_factories`.

Callbacks follow existing native series-major paint order, including the established reversed axis order for counterclockwise layouts. For each point, geometry executes and validates first, then label text executes if owned labels are enabled. Geometry callbacks still run with labels off. Hidden series and disabled points skip both; `labels=false` skips only text. Configured `size=0` without a glyph override remains an existing validation error; with a valid glyph override the unused legacy diameter does not suppress labels. A valid custom zero-ink geometry can still own a label. Transparent fill/stroke/text or a valid zero-size glyph does not suppress an otherwise owned label slot. Empty source generates no fabricated callback contexts.

The callback changes only the owned label's plain text. Logical position, label offset/style/owner, point identity, tooltip values/text, raw series metadata, paint and interaction anchors retain existing behavior. Custom geometry may move its ink, while text stays anchored to the logical point. The callback is borrowed for one build and never stored or rerun by hover. Either callback's error rejects the build; prior caller-owned callback side effects cannot be rolled back. New label builders preflight known shared Scene, actual generated tooltip strings and point projections before callbacks; old entry points retain their error timing.

Empty text is allowed. Each returned UTF-8 string must fit the existing 1024-byte label budget, including multibyte characters; Scene output and source/option validation still apply. SVG escapes text and Canvas receives ordinary text, never HTML. No option fields changed. Returning `formatted_value` preserves the corresponding existing builder's Scene, layout, SVG and Canvas output. Explicit normal kerning applies to both existing and new Canvas text paths; it does not change the label factory's source or geometry contract.

The source oracle executes original Nivo `getPropertyAccessor` with real Lodash and the entire original `RadarDots` pure `useMemo` points stage. Original `pointData` includes `index/key/value/formattedValue/color`; ten accessor profiles exercise 60 actual records. Nivo iterates axes then keys, while native Radar preserves series-major paint order; matching uses the typed index/key identity. A numeric accessor result needs an explicit string conversion in the typed Rust callback. The missing-property oracle returns JavaScript `undefined`; the Rust example deliberately returns an empty string and does not implement a dynamic property accessor. JSX is source-pinned but not mounted. This does not assert React hook/lifecycle, arbitrary component, browser or renderer equivalence. Nivo's public `svgDefaultProps.dotLabel` is `formattedValue`, while the internal `RadarDots` destructuring fallback is `value`. The source oracle executes the public defaults object too; native default text remains its existing formatted string.

The complete example above is saved as `examples/radar_point_labels.rs`. The export includes original SVG, Canvas script, HTML/font and actual input/point metadata; the raster feature also writes CPU PNG. The source-oracle test is gated by the existing `geojson` feature for JSON support; the other tests remain portable.

Run from the crate workspace:

```powershell
cargo metadata --no-deps --format-version 1
node scripts/radar-point-label-reference.mjs --check
cargo test --offline --no-default-features --test radar_point_labels
cargo test --offline --all-features --test radar_point_labels
cargo run --offline --example radar_point_labels -- target/radar-point-label-export
cargo run --offline --features raster --example radar_point_labels -- target/radar-point-label-raster-export
```
