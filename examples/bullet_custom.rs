//! Unreleased custom ranges/measures/markers, typed picking and shared exports.
use std::fmt::Write;
use std::path::PathBuf;

use glory::render::{GraphicPoint as P, PathCommand as C, Props};
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::{render, *};

fn shifted(kind: BulletPartKind, origin: [f64; 2], horizontal: bool) -> [f64; 2] {
    let offset = match kind {
        BulletPartKind::Range => 0.,
        BulletPartKind::Measure => 18.,
        BulletPartKind::Marker => -18.,
    };
    if horizontal {
        [origin[0], origin[1] + offset]
    } else {
        [origin[0] + offset, origin[1]]
    }
}

fn memory_mount(
    custom: &CustomBulletLayout,
) -> std::result::Result<usize, Box<dyn std::error::Error>> {
    let app = glory::App::memory();
    app.mount(
        chart_layout(custom.clone().into_painted_scene_interaction()?)
            .highlight(false)
            .on_reject(|error| eprintln!("Bullet Memory mount rejected: {error}")),
    )?;
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut data = 0;
    let mut graphics = 0;
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        let view = node.descriptor.view();
        if view.classes.iter().any(|c| c == "kartis-datum") {
            data += 1;
        }
        if view.classes.iter().any(|c| c == "kartis-mark") {
            assert!(matches!(view.props, Props::Graphic(_)));
            graphics += 1;
        }
        pending.extend(node.children.iter().copied());
    }
    assert_eq!(data, custom.interaction.sites.len());
    assert_eq!(graphics, custom.layout.scene.marks.len());
    Ok(data)
}

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
    let data = ["one", "two"].map(|id| BulletDatum {
        id: id.into(),
        ranges: vec![30., 70.],
        measures: vec![20., 60.],
        markers: vec![45.],
    });
    let mut frames = Vec::new();
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Custom Bullet parts</title><style>section{display:flex}svg,canvas{width:320px;height:240px}</style>",
    );
    for (orientation, direction) in [
        (StatisticsOrientation::Horizontal, "horizontal"),
        (StatisticsOrientation::Vertical, "vertical"),
    ] {
        let horizontal = orientation == StatisticsOrientation::Horizontal;
        for reverse in [false, true] {
            for profile in ["legacy", "linear", "symlog", "log"] {
                let hidden = matches!(profile, "linear" | "log");
                let name = format!("{direction}-{reverse}-{profile}");
                let options = BulletLayoutOptions {
                    orientation,
                    reverse,
                    min: Some(if profile == "log" { 1. } else { 0. }),
                    max: Some(100.),
                    domain_mode: if hidden {
                        BulletDomainMode::PerRow
                    } else {
                        BulletDomainMode::Shared
                    },
                    hidden: if hidden {
                        ["two".into()].into()
                    } else {
                        Default::default()
                    },
                    spacing: 20.,
                    axis: AxisOptions {
                        visible: false,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut calls = 0;
                let mut factory = |ctx: &BulletPartContext<'_>| {
                    calls += 1;
                    let [x, y] = shifted(ctx.part.kind, ctx.position, horizontal);
                    let geometry = if ctx.part.kind == BulletPartKind::Range {
                        Geometry::Circle { x, y, radius: 5.5 }
                    } else {
                        Geometry::Path(vec![
                            C::Move(P::new(x, y - 6.)),
                            C::Line(P::new(x + 6., y)),
                            C::Line(P::new(x, y + 6.)),
                            C::Line(P::new(x - 6., y)),
                            C::Close,
                        ])
                    };
                    let mut ink = ctx.inherit(geometry);
                    ink.fill = Some(match ctx.part.kind {
                        BulletPartKind::Range => Color(40, 100, 200, 255),
                        BulletPartKind::Measure => Color(210, 40, 90, 255),
                        BulletPartKind::Marker => Color(20, 140, 80, 255),
                    });
                    ink.fill_paint = None;
                    ink.stroke = None;
                    Ok(ink)
                };
                let custom = if profile == "legacy" {
                    bullet_layout_with_part_factory(&config, &data, &options, &mut factory)?
                } else {
                    bullet_layout_with_scale_options_and_part_factory(
                        &config,
                        &data,
                        &options,
                        &ContinuousScaleOptions {
                            kind: match profile {
                                "symlog" => ScaleKind::SymLog { constant: 10. },
                                "log" => ScaleKind::Log,
                                _ => ScaleKind::Linear,
                            },
                            ..Default::default()
                        },
                        &mut factory,
                    )?
                    .layout
                };
                assert_eq!(calls, custom.interaction.sites.len());
                let mounted = memory_mount(&custom)?;
                let mut exact = SurfaceModel::new_with_interaction(
                    custom.layout.scene.clone(),
                    SurfaceOptions {
                        plot: Some(custom.plot),
                        picking: Picking::Geometry,
                        ..Default::default()
                    },
                    Some(custom.interaction.clone()),
                )?;
                let mut nearest = SurfaceModel::new_with_interaction(
                    custom.layout.scene.clone(),
                    SurfaceOptions {
                        plot: Some(custom.plot),
                        picking: Picking::Nearest { distance: 0.001 },
                        ..Default::default()
                    },
                    Some(custom.interaction.clone()),
                )?;
                let mut scene = custom.layout.scene.clone();
                scene.labels.clear();
                scene.label_series.clear();
                scene.label_clips.clear();
                scene.typography.labels.clear();
                scene.typography.label_whitespace.clear();
                scene.draw_order.clear();
                let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default())?;
                let mut samples = Vec::new();
                let mut parts = Vec::new();
                for site in &custom.interaction.sites {
                    let row = custom
                        .layout
                        .rows
                        .iter()
                        .find(|r| r.id == site.series)
                        .unwrap();
                    let part = row.parts.iter().find(|p| p.id == site.mark_id).unwrap();
                    let point = shifted(part.kind, site.position, horizontal);
                    let mark = scene.marks.iter().find(|m| m.id == site.mark_id).unwrap();
                    assert_eq!(mark.geometry, part.geometry);
                    assert_eq!(mark.clip, Some(row.track));
                    exact.apply(SurfaceInput::Move {
                        id: 1,
                        point: (point[0], point[1]),
                    })?;
                    assert!(exact.hovered().contains(&site.mark_id));
                    nearest.apply(SurfaceInput::Move {
                        id: 1,
                        point: (site.position[0], site.position[1]),
                    })?;
                    assert!(nearest.hovered().contains(&site.mark_id));
                    let mut probes = vec![point];
                    if point != site.position {
                        exact.apply(SurfaceInput::Move {
                            id: 1,
                            point: (site.position[0], site.position[1]),
                        })?;
                        assert!(!exact.hovered().contains(&site.mark_id));
                        probes.push(site.position);
                    }
                    for point in probes {
                        let i = (point[1].floor() as usize * image.width as usize
                            + point[0].floor() as usize)
                            * 4;
                        samples.push(serde_json::json!({"point":point,"rgba":image.rgba[i..i+4]}));
                    }
                    parts.push(serde_json::json!({"id":part.id,"kind":format!("{:?}",part.kind),"sourceIndex":part.source_index,"raw":part.raw_value,"interval":part.interval,"visibleInterval":part.visible_interval,"origin":site.position,"ink":point}));
                }
                let svg = render::to_svg_with_namespace(&scene, &name)?;
                std::fs::write(out.join(format!("{name}.svg")), &svg)?;
                std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
                write!(
                    html,
                    "<section id=\"{name}\">{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{}</script>",
                    render::to_canvas_script(&scene, &format!("canvas-{name}"))?
                )?;
                frames.push(serde_json::json!({"name":name,"samples":samples,"parts":parts,"memoryMountedDataControls":mounted,"callbacks":calls}));
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
