//! Independent endpoint text, combined point geometry and real-font exports.
use std::fmt::Write;
use std::path::PathBuf;

use base64::Engine as _;
use glory::render::Props;
use kartis::render::raster::*;
use kartis::{render, *};

fn ordinary_texts(scene: &Scene) -> std::result::Result<usize, Box<dyn std::error::Error>> {
    let app = glory::App::memory();
    app.mount(chart(scene.clone()).highlight(false))?;
    let tree = app.tree();
    let mut pending = tree
        .roots()
        .iter()
        .copied()
        .map(|id| (id, false))
        .collect::<Vec<_>>();
    let mut actual = Vec::new();
    while let Some((id, inside)) = pending.pop() {
        let node = tree.node(id).unwrap();
        let view = node.descriptor.view();
        let inside = inside || view.classes.iter().any(|c| c == "kartis-label");
        if inside && let Props::Text(text) = view.props {
            actual.push(text.content.clone());
        }
        pending.extend(node.children.iter().copied().map(|id| (id, inside)));
    }
    let mut expected = scene
        .labels
        .iter()
        .map(|l| l.text.clone())
        .collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    Ok(actual.len())
}

fn bounds(image: &RenderedImage) -> Option<[usize; 4]> {
    let mut b = [image.width as usize, image.height as usize, 0, 0];
    for (i, p) in image.rgba.as_chunks::<4>().0.iter().enumerate() {
        if p[0] < 180 && p[1] < 180 && p[2] < 180 && p[3] > 128 {
            let x = i % image.width as usize;
            let y = i / image.width as usize;
            b[0] = b[0].min(x);
            b[1] = b[1].min(y);
            b[2] = b[2].max(x + 1);
            b[3] = b[3].max(y + 1);
        }
    }
    (b[2] > b[0]).then_some(b)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let config = ChartConfig {
        width: 620.,
        height: 320.,
        margin: Margin {
            left: 150.,
            right: 150.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    };
    let source = [
        BumpSeries::new(
            "A",
            vec![
                BumpDatum::new("Q1", 1.),
                BumpDatum::new("Q2", 3.),
                BumpDatum::new("Q3", 2.),
            ],
        ),
        BumpSeries::new(
            "B",
            vec![
                BumpDatum::new("Q1", 2.),
                BumpDatum::new("Q2", 1.),
                BumpDatum::gap("Q3"),
            ],
        ),
        BumpSeries::new(
            "C",
            vec![
                BumpDatum::gap("Q1"),
                BumpDatum::new("Q2", 2.),
                BumpDatum::new("Q3", 3.),
            ],
        ),
    ]
    .map(|mut series| {
        series.label = "Shared source".into();
        series
    });
    let bytes = include_bytes!("../tests/fonts/Lato-Regular.ttf");
    let family = FontFamily::Named("BumpFixture".into());
    let mut fonts = FontRegistry::new();
    let face = fonts.register(bytes.as_slice(), 0)?;
    fonts.bind_face(
        face,
        FaceDescriptor {
            family: family.clone(),
            weight: 400.,
        },
    )?;
    let font_url = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Bump endpoint labels</title><style>@font-face{{font-family:BumpFixture;src:url(data:font/ttf;base64,{font_url})}}section{{display:flex}}svg,canvas{{width:620px;height:320px}}</style>"
    );
    let mut frames = Vec::new();
    let mut scripts = String::new();
    for reverse in [false, true] {
        for hidden in [false, true] {
            for mode in ["both", "start", "end", "disabled"] {
                let name = format!("bump-{reverse}-{hidden}-{mode}");
                let mut options = BumpLayoutOptions {
                    axes: BumpAxes::none(),
                    legend: false,
                    hidden: if hidden {
                        ["A".into()].into()
                    } else {
                        Default::default()
                    },
                    ..Default::default()
                };
                options.x_scale.reverse = reverse;
                options.labels = BumpEndpointLabels {
                    start: matches!(mode, "both" | "start"),
                    end: matches!(mode, "both" | "end"),
                    padding: 12.,
                    size: Some(16.),
                    color: InheritedColor::Fixed(Color(20, 20, 20, 255)),
                    end_style: Some(BumpEndpointLabelStyle {
                        padding: Some(16.),
                        color: Some(InheritedColor::Fixed(Color(80, 30, 40, 255))),
                    }),
                    ..Default::default()
                };
                let mut point_calls = 0;
                let mut label_calls = 0;
                let mut point = |ctx: &BumpPointContext<'_>| {
                    point_calls += 1;
                    Ok(ctx.inherit(Geometry::Rect {
                        x: ctx.position[0] - 3.,
                        y: ctx.position[1] - 3.,
                        width: 6.,
                        height: 6.,
                    }))
                };
                let mut label = |ctx: &BumpEndpointLabelContext<'_>| {
                    label_calls += 1;
                    assert_eq!(ctx.default_text, "Shared source");
                    if ctx.series.id == "B" && ctx.endpoint == BumpEndpoint::Start {
                        return Ok(String::new());
                    }
                    let datum = if ctx.endpoint == BumpEndpoint::Start {
                        ctx.series.data.first()
                    } else {
                        ctx.series.data.last()
                    }
                    .unwrap();
                    Ok(if ctx.endpoint == BumpEndpoint::Start {
                        format!("S<{}>:{}", ctx.series.id, datum.rank.unwrap())
                    } else {
                        format!("E&{}:{}", ctx.series.id, datum.rank.unwrap())
                    })
                };
                let layout = bump_layout_with_factories(
                    &config,
                    &source,
                    &options,
                    BumpFactories {
                        point: Some(&mut point),
                        endpoint_label: Some(&mut label),
                    },
                )?;
                assert_eq!(label_calls, layout.scene.labels.len());
                assert_eq!(point_calls, layout.interaction.sites.len());
                assert!(
                    layout
                        .series
                        .iter()
                        .all(|s| s.source.label == "Shared source")
                );
                let mut scene = layout.scene.clone();
                scene.typography.default = FontOptions {
                    family: Some(vec![family.clone()]),
                    weight: Some(400.),
                };
                let mounted = ordinary_texts(&scene)?;
                std::fs::write(
                    out.join(format!("{name}-chart.svg")),
                    render::to_svg(&scene)?,
                )?;
                scene.marks.clear();
                scene.draw_order.clear();
                let labels=scene.labels.iter().map(|l|serde_json::json!({"text":l.text,"x":l.x,"y":l.y,"anchor":format!("{:?}",l.anchor).to_lowercase()})).collect::<Vec<_>>();
                let image = render_raster(&scene, &fonts, &RasterOptions::default())?;
                let ink = bounds(&image);
                let outlined = scene_with_painted_labels(
                    &scene,
                    &fonts,
                    &PaintedLabelOptions {
                        rotated_only: false,
                        ..Default::default()
                    },
                    &name,
                )?;
                let plain_svg = render::to_svg_with_namespace(&scene, &format!("plain-{name}"))?;
                let outlined_svg = render::to_svg_with_namespace(&outlined, &name)?;
                std::fs::write(out.join(format!("{name}.svg")), &outlined_svg)?;
                std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
                write!(
                    html,
                    "<section id=\"{name}\">{plain_svg}<canvas id=\"native-{name}\"></canvas><canvas id=\"canvas-{name}\"></canvas></section>"
                )?;
                writeln!(
                    scripts,
                    "{}",
                    render::to_canvas_script(&scene, &format!("native-{name}"))?
                )?;
                writeln!(
                    scripts,
                    "{}",
                    render::to_canvas_script(&outlined, &format!("canvas-{name}"))?
                )?;
                frames.push(serde_json::json!({"name":name,"labels":labels,"ink":ink,"pointCalls":point_calls,"labelCalls":label_calls,"ordinaryTextNodes":mounted}));
            }
        }
    }
    let recorder = r#"window.bumpNativeCalls=[];const originalFillText=CanvasRenderingContext2D.prototype.fillText;CanvasRenderingContext2D.prototype.fillText=function(text,x,y,...rest){if(this.canvas.id.startsWith('native-')){const m=this.getTransform(),s=this.canvas.width/620;window.bumpNativeCalls.push({id:this.canvas.id,text,x:(m.a*x+m.c*y+m.e)/s,y:(m.b*x+m.d*y+m.f)/s,anchor:this.textAlign,baseline:this.textBaseline,font:this.font});}return originalFillText.call(this,text,x,y,...rest);};"#;
    write!(
        html,
        "<script id=\"frames\" type=\"application/json\">{}</script><script id=\"render-source\" type=\"text/plain\">{scripts}</script><script>{recorder}document.fonts.load('16px BumpFixture').then(()=>document.fonts.ready).then(()=>{{(0,eval)(document.getElementById('render-source').textContent);CanvasRenderingContext2D.prototype.fillText=originalFillText;window.bumpReady=true}}).catch(e=>{{window.bumpError=String(e)}})</script>",
        serde_json::to_string(&frames)?
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(out.join("frames.json"), serde_json::to_vec_pretty(&frames)?)?;
    Ok(())
}
