# Radar 点标签文字工厂

本候选增加借用型 `FnMut(&RadarPointLabelContext) -> Result<String>` 入口。开启 `points.labels` 后，可按轴和 series 身份生成 owned label 的纯文本。下列是保存于 `examples/radar_point_labels.rs` 的完整可运行例子。

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

`index` 是稳定轴 ID，`key` 是稳定 series ID，`value` 是原始有限数值，`formatted_value` 使用该点既有格式器。`point` 复用已有 `RadarPointContext`，包含原 series、显示轴标题、mark ID、逻辑位置和已解析 paint。`point.color` 保留既有 polygon swatch fallback，不把任意自定义 paint profile 冒称为 Nivo palette 等价。显示标题改名不改身份；不凭空提供 RawDatum 或动态属性 getter。

Scene 入口为 `radar_with_point_label_factory` 和 `radar_with_value_scale_and_point_label_factory`。typed layout 入口为 `radar_layout_with_point_label_factory` 和 `radar_layout_with_value_scale_and_point_label_factory`。组合几何和文字使用 `radar_layout_with_point_and_label_factories` 或 `radar_layout_with_value_scale_and_point_and_label_factories`。

执行顺序保持已有 native series-major paint order，包含逆时针轴顺序。每点先执行并验证 geometry，再按实际 owned label 资格执行文字回调。labels=false 只跳过文字；隐藏 series 或禁用 points 跳过两者。没有 glyph override 时配置 size0 仍然报错；有合法 glyph override 时，未使用的旧 size0 不抑制标签。合法的自定义零墨迹几何也可以拥有标签。透明 fill/stroke/text 或合法 size0 glyph 不会取消本来实际拥有的 label slot。空数据不伪造回调上下文。

新文字 API 在调用用户代码前预检已知 shared Scene、实际生成的 tooltip 字符串及点投影；旧入口保留原错误时序。实际 geometry 或返回文字仍需回调后验证。错误拒绝整次构建，但先前已执行的外部计数等副作用不能回滚。

回调只改变 owned label 纯文本；ID、owner、tooltip、raw source metadata、paint、逻辑位置、文字 offset/style 和 interaction anchor 保持原行为。自定义几何可移动墨迹，文字仍跟随逻辑点。回调不保存在布局中，真实 hover 不会重新调用它。允许空字符串；UTF-8 长度最多 1024 bytes。SVG 转义文字，Canvas 接收普通文字。不新增 options 字段。返回 `formatted_value` 时，Scene、layout、SVG 和 Canvas 与对应的既有构建入口一致。显式 normal kerning 同时用于既有和新增 Canvas 文字路径，不改变标签工厂的源数据或几何契约。

源码 oracle 使用真实 Lodash，执行完整原 `getPropertyAccessor`、原 `RadarDots` 完整 pure useMemo points 段，并执行原公共 defaults 对象。10 profiles 共 60 个真实 pointData records。公共 dotLabel 默认是 formattedValue，内部 RadarDots 未传参时 fallback 才是 value；二者不混淆。原代码 axes-major 与 native series-major 顺序不同，按 index/key 匹配。数值 accessor 结果需在 Rust 回调显式转成 String；缺失属性在 JavaScript 是 undefined，Rust 示例选择空字符串，而不是新增动态 getter。JSX 仅 pin 源码、不 mount，不声称 React hook/lifecycle、任意组件、浏览器或全 renderer 等价。

原始 SVG/Canvas、HTML/确切 Lato 字体及实际 source/point metadata 随例子导出；raster 开启时另写 CPU PNG。原 JSON oracle 单项测试由已有 geojson feature 门控，其余专项保持最小配置可运行。以下命令在 crate workspace 执行：

```powershell
cargo metadata --no-deps --format-version 1
node scripts/radar-point-label-reference.mjs --check
cargo test --offline --no-default-features --test radar_point_labels
cargo test --offline --all-features --test radar_point_labels
cargo run --offline --example radar_point_labels -- target/radar-point-label-export
cargo run --offline --features raster --example radar_point_labels -- target/radar-point-label-raster-export
```
