//! Actual fitted USA maps and per-component symbol/glyph export diagnostics.
use std::fmt::Write;
use std::path::PathBuf;

use glory::render::{GraphicStroke, StrokeCap};
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::{render, *};

fn centers() -> [[f64; 2]; 3] {
    [[-122.4194, 37.7749], [-149.9, 61.2], [-157.8583, 21.3069]]
}
fn box_ring([x, y]: [f64; 2], dx: f64, dy: f64) -> Vec<[f64; 2]> {
    vec![
        [x - dx, y - dy],
        [x - dx, y + dy],
        [x + dx, y + dy],
        [x + dx, y - dy],
    ]
}
fn features() -> Vec<GeoFeature> {
    let polygons = centers()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut rings = vec![box_ring(*p, 2., 1.5)];
            if i == 0 {
                let mut hole = box_ring(*p, 0.4, 0.3);
                hole.reverse();
                rings.push(hole);
            }
            rings
        })
        .collect();
    vec![GeoFeature {
        id: "three-regions".into(),
        label: "One source feature".into(),
        value: Some(7.),
        polygons,
    }]
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let config = ChartConfig {
        width: 800.,
        height: 500.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Fitted USA maps and region masks</title><style>section{display:flex}svg,canvas{width:800px;height:500px}</style>",
    );
    let mut frames = Vec::new();
    let mut scenes = Vec::new();
    for fit in [GeoFit::World, GeoFit::Data] {
        let map = geographic_usa_map_layout(
            &config,
            &features(),
            &GeoUsaMapOptions {
                fit,
                padding: 10.,
                fill: Some(Color(30, 90, 160, 255)),
                border: None,
                ..Default::default()
            },
        )?;
        assert_eq!(map.scene.marks.len(), 1);
        assert_eq!(map.scene.marks[0].id, "map:three-regions");
        let ctx = &map.projection;
        let core = ctx.usa_projection().ok_or("missing composite")?;
        let mut probes = Vec::new();
        for (i, p) in centers().iter().enumerate() {
            probes.push((ctx.project(*p)?.ok_or("missing center")?, i != 0));
            probes.push((ctx.project([p[0] + 1., p[1]])?.ok_or("missing land")?, true));
            probes.push((
                ctx.project([p[0], p[1] + 0.9])?.ok_or("missing land")?,
                true,
            ));
        }
        scenes.push((format!("map-{fit:?}"), map.scene.clone(), probes));
        for (i, p) in centers().iter().enumerate() {
            let selected = ctx.project_usa(*p)?.ok_or("missing region")?;
            let rect = core.region_clip(selected.region);
            let symbols = [GeoSymbol::new(format!("place-{i}"), *p, Some(7.))];
            for mode in ["original", "circle-mask", "glyph-mask"] {
                let mut scene = config.scene()?;
                let radius = (core.scale() * 0.12).min(300.);
                let options = GeoSymbolOptions {
                    clip: false,
                    clip_to_projection: mode != "original",
                    radius_range: [radius, radius],
                    color: GeoSymbolColor::Fixed(Color(30, 90, 160, 255)),
                    stroke: Some(GraphicStroke {
                        color: Color(30, 90, 160, 255),
                        width: 8.,
                        cap: StrokeCap::Round,
                    }),
                    ..Default::default()
                };
                if mode == "glyph-mask" {
                    let typed = ctx.add_glyphs(
                        &mut scene,
                        &symbols,
                        &GeoGlyphOptions {
                            shape: GlyphShape::Filled(FilledGlyph::Star),
                            rotation: 0.37,
                            symbols: options,
                            ..Default::default()
                        },
                    )?;
                    assert_eq!(typed.sites.len(), 1);
                    typed.interaction_binding(&scene)?.validate(&scene)?;
                } else {
                    ctx.add_symbols(&mut scene, &symbols, &options)?;
                }
                let mark = &scene.marks[0];
                assert!(mark.clip_path.is_none());
                assert_eq!(mark.clip.is_some(), mode != "original");
                let mut probes = vec![(selected.position, true)];
                // Probe real removed ink inside the broad Lower48 rectangle.
                // The unmasked counterpart establishes the original paint witness.
                let mut unmasked = scene.clone();
                unmasked.marks[0].clip_path = None;
                unmasked.marks[0].clip = None;
                // Flat-color probes use the fill interior, never the stroke's
                // antialiasing fringe. The exported Scene keeps its real stroke.
                unmasked.marks[0].stroke = None;
                let mut inside = 0;
                let mut outside = 0;
                let reach = (radius - 5.).floor() as i32;
                for dx in (-reach..=reach).step_by(5) {
                    for dy in (-reach..=reach).step_by(5) {
                        let q = [
                            selected.position[0] + f64::from(dx),
                            selected.position[1] + f64::from(dy),
                        ];
                        if !(4. ..796.).contains(&q[0]) || !(4. ..496.).contains(&q[1]) {
                            continue;
                        }
                        if unmasked.hit_test(q[0], q[1]).is_none() {
                            continue;
                        }
                        // Keep probes at least four logical pixels off both
                        // source contours and rectangle edges, at either DPR.
                        if [
                            q[0] - rect.x,
                            q[0] - rect.x - rect.width,
                            q[1] - rect.y,
                            q[1] - rect.y - rect.height,
                        ]
                        .iter()
                        .any(|v| v.abs() < 4.)
                        {
                            continue;
                        }
                        if ![-12., 0., 12.].iter().all(|a| {
                            [-12., 0., 12.]
                                .iter()
                                .all(|b| unmasked.hit_test(q[0] + a, q[1] + b).is_some())
                        }) {
                            continue;
                        }
                        let in_region = rect.contains(q[0], q[1]);
                        let expected = mode == "original" || in_region;
                        if in_region && inside < 4 {
                            probes.push((q, expected));
                            inside += 1;
                        }
                        if !in_region && outside < 4 {
                            probes.push((q, expected));
                            outside += 1;
                        }
                    }
                }
                assert_eq!(inside, 4);
                // Mainland positions can be well inside their much larger clip.
                if selected.region != GeoUsaRegion::Lower48 {
                    assert_eq!(
                        outside,
                        4,
                        "{fit:?}/{i}/{mode}: scale {} center {:?} clip {rect:?}",
                        core.scale(),
                        selected.position
                    );
                }
                scenes.push((format!("{fit:?}-{i}-{mode}"), scene, probes));
            }
        }
    }
    assert_eq!(scenes.len(), 20);
    let mut comparisons = 0;
    for (name, scene, probes) in scenes {
        scene.validate()?;
        let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default())?;
        let mut samples = Vec::new();
        for (p, hit) in probes {
            assert_eq!(scene.hit_test(p[0], p[1]).is_some(), hit, "{name} {p:?}");
            let index = (p[1].floor() as usize * image.width as usize + p[0].floor() as usize) * 4;
            let rgba = &image.rgba[index..index + 4];
            let expected = if hit {
                Color(30, 90, 160, 255)
            } else {
                scene.background
            };
            assert!(
                rgba.iter()
                    .zip([expected.0, expected.1, expected.2, expected.3])
                    .all(|(a, b)| a.abs_diff(b) <= 1),
                "flat painted probe {name} {p:?} {rgba:?}"
            );
            samples.push(serde_json::json!({"point":p,"rgba":rgba,"hit":hit}));
            comparisons += 2;
        }
        let svg = render::to_svg_with_namespace(&scene, &name)?;
        std::fs::write(out.join(format!("{name}.svg")), &svg)?;
        std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
        write!(
            html,
            "<section>{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{}</script>",
            render::to_canvas_script(&scene, &format!("canvas-{name}"))?
        )?;
        frames.push(serde_json::json!({"name":name,"samples":samples}));
    }
    write!(
        html,
        "<script id=\"frames\" type=\"application/json\">{}</script>",
        serde_json::to_string(&frames)?
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(out.join("frames.json"), serde_json::to_vec_pretty(&frames)?)?;
    println!(
        "{} frames, {comparisons} comparisons per browser context",
        frames.len()
    );
    Ok(())
}
