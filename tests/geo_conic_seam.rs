use glory::render::PathCommand;
use kartis::*;
fn feature(reverse: bool) -> GeoFeature {
    let mut rings = vec![
        vec![
            [-125., 25.],
            [-125., 50.],
            [-65., 50.],
            [-65., 25.],
            [-125., 25.],
        ],
        vec![
            [-105., 32.],
            [-95., 32.],
            [-95., 40.],
            [-105., 40.],
            [-105., 32.],
        ],
    ];
    if reverse {
        for ring in &mut rings {
            ring.reverse();
        }
    }
    GeoFeature {
        id: "seam-hole".into(),
        label: "seam-hole".into(),
        value: Some(1.),
        polygons: vec![rings],
    }
}
fn paths(
    reverse: bool,
    projection: GeoProjection,
    center: [f64; 2],
    rotation: f64,
    path: GeoPathOptions,
) -> Result<Vec<Vec<[f64; 2]>>> {
    let map = geographic_map_layout_with_path_options(
        &ChartConfig {
            width: 760.,
            height: 320.,
            ..Default::default()
        },
        &[feature(reverse)],
        &GeoMapOptions {
            projection,
            center,
            rotation,
            fit: GeoFit::Data,
            ..Default::default()
        },
        &path,
    )?;
    let mut rings: Vec<Vec<[f64; 2]>> = vec![];
    for mark in &map.scene.marks {
        if let Geometry::Path(commands) = &mark.geometry {
            for c in commands {
                match c {
                    PathCommand::Move(p) => rings.push(vec![[p.x, p.y]]),
                    PathCommand::Line(p) => rings.last_mut().unwrap().push([p.x, p.y]),
                    PathCommand::Close => {}
                    _ => panic!("nonlinear path"),
                }
            }
        }
    }
    Ok(rings)
}
fn distance(p: [f64; 2], rings: &[Vec<[f64; 2]>]) -> f64 {
    rings
        .iter()
        .flat_map(|r| (0..r.len()).map(|i| (r[i], r[(i + 1) % r.len()])))
        .map(|(a, b)| {
            let x = b[0] - a[0];
            let y = b[1] - a[1];
            let t = (((p[0] - a[0]) * x + (p[1] - a[1]) * y) / (x * x + y * y)).clamp(0., 1.);
            (p[0] - a[0] - t * x).hypot(p[1] - a[1] - t * y)
        })
        .fold(f64::INFINITY, f64::min)
}
#[test]
fn exterior_and_hole_rejoin_together_for_spherical_cameras_and_both_windings() {
    for projection in [
        GeoProjection::conic_equal_area_default(),
        GeoProjection::conic_equidistant_default(),
        GeoProjection::conic_conformal_default(),
        GeoProjection::conic_equal_area([29.5, 45.5]).unwrap(),
    ] {
        for (center, roll) in [([0., 0.], 0.), ([-35., -20.], -25.), ([80., -30.], 30.)] {
            let options = GeoPathOptions {
                precision: 0.0001,
                ..Default::default()
            };
            let a = paths(false, projection, center, roll, options.clone()).unwrap();
            let b = paths(true, projection, center, roll, options).unwrap();
            assert_eq!(
                a.len(),
                2,
                "exterior and hole must rejoin as one polygon on each seam side: {projection:?} {center:?}"
            );
            assert_eq!(b.len(), 2);
            for (x, y) in [(&a, &b), (&b, &a)] {
                for p in x.iter().flatten() {
                    assert!(distance(*p, y) < 1e-8, "winding changed border");
                }
            }
            let area = |r: &Vec<[f64; 2]>| {
                r.iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let q = r[(i + 1) % r.len()];
                        p[0] * q[1] - q[0] * p[1]
                    })
                    .sum::<f64>()
                    / 2.
            };
            let areas = a.iter().map(area).collect::<Vec<_>>();
            if center == [0., 0.] {
                assert!(
                    areas[0] * areas[1] < 0.,
                    "unclipped hole lost opposite winding"
                );
            }
            if center == [80., -30.] {
                assert!(
                    areas.iter().all(|a| *a > 0.),
                    "two seam exteriors must retain positive winding: {areas:?}"
                );
            }
        }
    }
}
#[test]
fn rejoined_seam_hole_respects_adaptive_point_and_work_budgets() {
    for options in [
        GeoPathOptions {
            max_points: 10,
            ..Default::default()
        },
        GeoPathOptions {
            max_work: 1,
            ..Default::default()
        },
    ] {
        let error = paths(
            false,
            GeoProjection::conic_equal_area_default(),
            [80., -30.],
            30.,
            options,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("budget"), "{error}");
    }
}
