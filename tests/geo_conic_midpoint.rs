use glory::render::PathCommand;
use kartis::*;
fn options() -> GeoMapOptions {
    GeoMapOptions {
        projection: GeoProjection::Albers,
        center: [15., -5.],
        rotation: 18.,
        scale: Some(228.43197642765313),
        ..Default::default()
    }
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 720.,
        height: 400.,
        margin: Margin {
            left: 36.,
            right: 28.,
            top: 24.,
            bottom: 92.,
        },
        ..Default::default()
    }
}
fn layout(path: GeoPathOptions) -> Result<GeoStreamLayout> {
    let c = config();
    let mut view = options();
    let world = geographic_map_layout(&c, &[], &view)?
        .projection
        .fitted_raw_center();
    let raw = [0.6946014848802866, 0.6101266266362744];
    let scale = view.scale.unwrap();
    view.translate = Some([
        364. + (world[0] - raw[0]) * scale,
        166. + (raw[1] - world[1]) * scale,
    ]);
    geographic_lines(
        &c,
        &[GeoLineFeature::new(
            "polar-turn",
            vec![
                [-60., -89.999999],
                [-60., 9.999999974752427e-7],
                [-60., 89.999999],
            ],
        )],
        &view,
        &GeoStreamOptions {
            path: Some(path),
            clip_plot: false,
            ..Default::default()
        },
    )
}
fn distance(p: [f64; 2], parts: &[Vec<[f64; 2]>]) -> f64 {
    let mut d = f64::INFINITY;
    for part in parts {
        for edge in part.windows(2) {
            let (a, b) = (edge[0], edge[1]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0., 1.);
            d = d.min((p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy));
        }
    }
    d
}
#[test]
fn rotated_polar_meridian_meets_dense_original_direct_projection_at_existing_half_pixel() {
    let result = layout(GeoPathOptions::default()).unwrap();
    let mut parts = Vec::new();
    let Geometry::Path(commands) = &result
        .scene
        .marks
        .iter()
        .find(|m| m.id == "geo-line:polar-turn")
        .unwrap()
        .geometry
    else {
        panic!()
    };
    for command in commands {
        match command {
            PathCommand::Move(p) => parts.push(vec![[p.x, p.y]]),
            PathCommand::Line(p) => parts.last_mut().unwrap().push([p.x, p.y]),
            _ => panic!("open line only"),
        }
    }
    let mut max: f64 = 0.;
    let mut rows = 0;
    for line in include_str!("fixtures/conic-midpoint/original-direct.csv").lines() {
        let c: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
        max = max.max(distance([c[1], c[2]], &parts));
        rows += 1;
    }
    assert_eq!(rows, 25001);
    eprintln!(
        "dense-original-points={rows} maximum-logical-pixel-deviation={max:e} points={} work={} depth={}",
        result.metadata.sampling.points,
        result.metadata.sampling.work,
        result.metadata.sampling.deepest
    );
    assert!(
        max <= 0.5,
        "existing half-pixel precision must cover the independently authored dense original curve: {max}"
    );
}
#[test]
fn polar_refinement_respects_existing_depth_and_work_caps() {
    let low = GeoPathOptions {
        max_depth: 1,
        ..Default::default()
    };
    assert!(layout(low).unwrap_err().to_string().contains("max_depth"));
    let low = GeoPathOptions {
        max_work: 3,
        ..Default::default()
    };
    assert!(layout(low).unwrap_err().to_string().contains("work budget"));
}
#[test]
fn subpixel_conic_segment_does_not_refine_roundoff_midpoint_progress() {
    let path = GeoPathOptions {
        max_depth: 1,
        ..Default::default()
    };
    let data = [GeoLineFeature::new("tiny", vec![[0., 0.], [1e-10, 1e-10]])];
    let out = geographic_lines(
        &config(),
        &data,
        &options(),
        &GeoStreamOptions {
            path: Some(path),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out.metadata.sampling.deepest, 0);
    assert_eq!(out.metadata.lines[0].components, 1);
}
