//! Actual Albers USA line/graticule exports, selected-region full-ink masks,
//! cap/dash witnesses and flat interior CPU/browser comparison points.
use std::fmt::Write;
use std::path::PathBuf;

use glory::render::{GraphicStroke, StrokeCap};
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::surface::{SurfaceModel, SurfaceOptions};
use kartis::{render, *};

const INK: Color = Color(30, 90, 160, 255);
type Probe = ([f64; 2], bool);

fn painted(scene: &Scene) -> Result<SurfaceModel> {
    // Diagnostic eligibility only: each actual part is made a site on a copy.
    // Original exports, dash, caps, masks and geometry remain unchanged.
    let mut scene = scene.clone();
    let mut binding = InteractionBinding {
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    for mark in &mut scene.marks {
        if mark.label.is_empty() {
            mark.label = mark.id.clone();
        }
        if mark.series.is_empty() {
            mark.series = mark.id.clone();
        }
        let path = mark.geometry.commands();
        let glory::render::PathCommand::Move(p) = path[0] else {
            panic!("stream must begin with Move")
        };
        let position = [p.x, p.y];
        binding.anchors.insert(
            mark.id.clone(),
            InteractionAnchor::path_points(&mark.geometry, position, &[0])?,
        );
        binding.sites.push(InteractionSite {
            mark_id: mark.id.clone(),
            series: mark.series.clone(),
            label: mark.label.clone(),
            value: Some(mark.value),
            formatted_value: None,
            color: INK,
            position,
        });
    }
    SurfaceModel::new_with_interaction(scene, SurfaceOptions::default(), Some(binding))
}
fn flat(scene: &SurfaceModel, p: [f64; 2], expected: bool) -> bool {
    [-3., 0., 3.].iter().all(|dx| {
        [-3., 0., 3.].iter().all(|dy| {
            scene
                .physical_hit_at((p[0] + dx, p[1] + dy))
                .unwrap()
                .is_some()
                == expected
        })
    })
}
fn safe(p: [f64; 2]) -> bool {
    (5. ..795.).contains(&p[0]) && (5. ..495.).contains(&p[1])
}
fn probes(original: &Scene, rect: ClipRect, masked: bool) -> Vec<Probe> {
    let original = painted(original).unwrap();
    let mut result = Vec::new();
    let mut inside = 0;
    let mut spill = 0;
    for x in ((rect.x + 4.).ceil() as i32..(rect.x + rect.width - 4.).floor() as i32).step_by(2) {
        for y in ((rect.y - 12.).floor() as i32..(rect.y + 22.).ceil() as i32).step_by(2) {
            let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            if !safe(p) || (p[1] - rect.y).abs() < 4. || !flat(&original, p, true) {
                continue;
            }
            let contained = rect.contains(p[0], p[1]);
            if contained && inside < 3 {
                result.push((p, true));
                inside += 1;
            } else if !contained && spill < 3 {
                result.push((p, !masked));
                spill += 1;
            }
        }
    }
    assert_eq!(
        inside, 3,
        "must witness original flat ink inside the region"
    );
    assert_eq!(
        spill, 3,
        "must witness original flat stroke spill outside the region"
    );
    result
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
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    };
    let base = geographic_usa_map_layout(
        &config,
        &[],
        &GeoUsaMapOptions {
            scale: Some(800.),
            translate: Some([400., 250.]),
            ..Default::default()
        },
    )?;
    let core = base
        .projection
        .usa_projection()
        .ok_or("missing composite")?;
    let mut scenes = Vec::new();
    for region in GeoUsaRegion::ALL {
        let rect = core.region_clip(region);
        let p = [rect.x + (rect.width * 0.2).min(20.), rect.y + 4.];
        let q = [rect.x + rect.width * 0.6, rect.y + 4.];
        let mut source = GeoLineFeature::new(
            format!("{region:?}"),
            vec![
                core.invert_in_region(region, p)?
                    .ok_or("missing start inverse")?,
                core.invert_in_region(region, q)?
                    .ok_or("missing end inverse")?,
            ],
        );
        source.value = Some(7.);
        for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
            for dashed in [false, true] {
                let options = GeoStreamOptions {
                    clip_plot: false,
                    clip_projection: false,
                    style: GeoLineStyle {
                        stroke: GraphicStroke {
                            color: INK,
                            width: 32.,
                            cap,
                        },
                        opacity: 1.,
                        dash: dashed.then(|| StrokeDash {
                            segments: vec![20., 70.],
                            offset: 0.,
                        }),
                    },
                    ..Default::default()
                };
                let mut original = base.clone();
                let original_metadata =
                    original.add_usa_lines(std::slice::from_ref(&source), &options)?;
                assert!(
                    original_metadata.lines[0]
                        .parts
                        .iter()
                        .any(|part| part.region == region)
                );
                assert_eq!(
                    original_metadata
                        .interaction_binding(&original.scene)?
                        .sites
                        .len(),
                    1
                );
                for masked in [false, true] {
                    let name = format!(
                        "{region:?}-{cap:?}-{}-{}",
                        if dashed { "dash" } else { "solid" },
                        if masked { "mask" } else { "original" }
                    );
                    let mut actual = base.clone();
                    let metadata = actual.add_usa_lines(
                        std::slice::from_ref(&source),
                        &GeoStreamOptions {
                            clip_projection: masked,
                            ..options.clone()
                        },
                    )?;
                    assert_eq!(metadata.lines, original_metadata.lines);
                    // The comparison uses the same source centerlines; masks
                    // never alter the delivered paths, dash or stroke style.
                    for mark in &actual.scene.marks {
                        let old = original
                            .scene
                            .marks
                            .iter()
                            .find(|m| m.id == mark.id)
                            .ok_or("missing original part")?;
                        assert_eq!(mark.geometry, old.geometry);
                        assert_eq!(mark.stroke, old.stroke);
                        assert_eq!(mark.stroke_dash, old.stroke_dash);
                        assert!(mark.clip_path.is_none());
                        assert_eq!(mark.clip.is_some(), masked);
                    }
                    scenes.push((name, actual.scene, probes(&original.scene, rect, masked)));
                }
            }
        }
    }
    let mut grid = base.clone();
    let metadata = grid.add_usa_graticule(
        &Default::default(),
        &GeoStreamOptions {
            style: GeoLineStyle {
                stroke: GraphicStroke {
                    color: INK,
                    width: 12.,
                    cap: StrokeCap::Butt,
                },
                ..Default::default()
            },
            clip_plot: false,
            ..Default::default()
        },
    )?;
    assert!(metadata.interaction_binding(&grid.scene)?.sites.is_empty());
    let mut samples = Vec::new();
    let grid_ink = painted(&grid.scene)?;
    // A union interior can still cross a second stroke's antialiasing edge.
    // Require the same three-pixel clearance for every actual painted part.
    let grid_parts = grid
        .scene
        .marks
        .iter()
        .map(|mark| {
            let mut scene = config.scene()?;
            scene.marks.push(mark.clone());
            painted(&scene)
        })
        .collect::<Result<Vec<_>>>()?;
    for region in GeoUsaRegion::ALL {
        let rect = core.region_clip(region);
        let mut count = 0;
        for x in ((rect.x + 6.).ceil() as i32..(rect.x + rect.width - 6.).floor() as i32).step_by(3)
        {
            for y in
                ((rect.y + 6.).ceil() as i32..(rect.y + rect.height - 6.).floor() as i32).step_by(3)
            {
                let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                if count < 8
                    && safe(p)
                    && flat(&grid_ink, p, true)
                    && grid_parts
                        .iter()
                        .all(|part| flat(part, p, false) || flat(part, p, true))
                {
                    samples.push((p, true));
                    count += 1;
                }
            }
        }
        assert_eq!(count, 8, "actual graticule ink in {region:?}");
    }
    scenes.push(("graticule".into(), grid.scene, samples));
    assert_eq!(scenes.len(), 37);
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Albers USA streams</title><style>section{display:flex}svg,canvas{width:800px;height:500px}</style>",
    );
    let mut frames = Vec::new();
    for (name, scene, probes) in scenes {
        scene.validate()?;
        let physical = painted(&scene)?;
        let mut samples = Vec::new();
        for dpr in [1., 2.] {
            let image = render_raster(
                &scene,
                &FontRegistry::new(),
                &RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
            )?;
            for (p, hit) in &probes {
                assert_eq!(
                    physical.physical_hit_at((p[0], p[1]))?.is_some(),
                    *hit,
                    "{name}: {p:?}"
                );
                let index = ((p[1] * dpr).floor() as usize * image.width as usize
                    + (p[0] * dpr).floor() as usize)
                    * 4;
                let rgba = &image.rgba[index..index + 4];
                let expected = if *hit { INK } else { scene.background };
                assert!(
                    rgba.iter()
                        .zip([expected.0, expected.1, expected.2, expected.3])
                        .all(|(a, b)| a.abs_diff(b) <= 1),
                    "{name}/DPR{dpr}: {p:?} {rgba:?}"
                );
                if dpr == 1. {
                    samples.push(serde_json::json!({"point":p,"rgba":rgba,"hit":hit}));
                }
            }
            std::fs::write(out.join(format!("{name}-{dpr}.png")), image.png()?)?;
        }
        let svg = render::to_svg_with_namespace(&scene, &name)?;
        std::fs::write(out.join(format!("{name}.svg")), &svg)?;
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
        "{} frames, {} browser pixel comparisons/context, both CPU DPRs validated",
        frames.len(),
        frames
            .iter()
            .map(|f| f["samples"].as_array().unwrap().len() * 2)
            .sum::<usize>()
    );
    Ok(())
}
