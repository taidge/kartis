use glory::render::PathCommand as Cmd;
use kartis::*;
fn spherical(u: f64, v: f64) -> [f64; 2] {
    [
        1_f64.atan2(-u).to_degrees(),
        v.atan2(u.hypot(1.)).to_degrees(),
    ]
}
fn ring(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    points.iter().map(|[u, v]| spherical(*u, *v)).collect()
}
fn exterior() -> Vec<[f64; 2]> {
    vec![
        [-2., -2.],
        [2., -2.],
        [2., 2.],
        [-2., 2.],
        [-2., 1.],
        [1., 1.],
        [1., -1.],
        [-2., -1.],
    ]
}
fn split_hole() -> Vec<[f64; 2]> {
    vec![
        [-1.8, -1.8],
        [1.8, -1.8],
        [1.8, 1.8],
        [-1.8, 1.8],
        [-1.8, 1.2],
        [1.3, 1.2],
        [1.3, -1.2],
        [-1.8, -1.2],
    ]
}
fn feature(rings: Vec<Vec<[f64; 2]>>) -> GeoFeature {
    GeoFeature {
        id: "nonconvex".into(),
        label: "Nonconvex".into(),
        value: Some(1.),
        polygons: vec![rings],
    }
}
fn opts(projection: GeoProjection) -> GeoMapOptions {
    GeoMapOptions {
        projection,
        border: None,
        fill: Some(Color(20, 100, 160, 255)),
        ..Default::default()
    }
}
fn projections() -> [GeoProjection; 4] {
    [
        GeoProjection::Orthographic,
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
    ]
}
fn inside(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}
fn moves(scene: &Scene) -> usize {
    scene
        .marks
        .iter()
        .flat_map(|m| m.geometry.commands())
        .filter(|c| matches!(c, Cmd::Move(_)))
        .count()
}
#[test]
fn concave_spherical_polygon_splits_visible_islands_instead_of_filling_a_false_bridge() {
    let c = ChartConfig::default();
    for projection in projections() {
        let layout =
            geographic_map_layout(&c, &[feature(vec![ring(&exterior())])], &opts(projection))
                .unwrap();
        assert_eq!(layout.scene.marks.len(), 1);
        assert_eq!(
            moves(&layout.scene),
            2,
            "{projection:?}: disconnected lobes need separate contours"
        );
        for (v, expected) in [(-1.5, true), (0., false), (1.5, true)] {
            let p = layout
                .projection
                .project(spherical(-1., v))
                .unwrap()
                .unwrap();
            assert_eq!(
                layout.scene.hit_test(p[0], p[1]).is_some(),
                expected,
                "{projection:?}: v={v}"
            );
        }
        render::to_svg(&layout.scene).unwrap();
        render::to_canvas_script(&layout.scene, "split").unwrap();
    }
}
#[test]
fn a_nonconvex_hole_can_split_into_both_exteriors_with_correct_nonzero_winding() {
    let c = ChartConfig::default();
    for projection in projections() {
        for reverse in [false, true] {
            let mut outer = ring(&exterior());
            let mut hole = ring(&split_hole());
            if reverse {
                outer.reverse();
                hole.reverse();
            }
            let layout =
                geographic_map_layout(&c, &[feature(vec![outer, hole])], &opts(projection))
                    .unwrap();
            assert_eq!(
                moves(&layout.scene),
                2,
                "clipped holes join into two genuine notched contours"
            );
            for u in [-1.9, -1.6, -0.7] {
                for v in [-1.9, -1.5, -1.1, 0., 1.1, 1.5, 1.9] {
                    let p = layout.projection.project(spherical(u, v)).unwrap().unwrap();
                    let expected = inside([u, v], &exterior()) && !inside([u, v], &split_hole());
                    assert_eq!(
                        layout.scene.hit_test(p[0], p[1]).is_some(),
                        expected,
                        "{projection:?}, reversed={reverse}, ({u},{v})"
                    );
                }
            }
        }
    }
}
#[test]
fn rotated_clipping_matches_original_projective_polygon_membership_on_a_point_grid() {
    let c = ChartConfig {
        width: 1000.,
        height: 800.,
        ..Default::default()
    };
    for projection in projections() {
        let o = GeoMapOptions {
            center: [8., -6.],
            rotation: 23.,
            ..opts(projection)
        };
        let layout = geographic_map_layout(
            &c,
            &[feature(vec![ring(&exterior()), ring(&split_hole())])],
            &o,
        )
        .unwrap();
        let mut tested = 0;
        for u in [-1.9, -1.6, -1.1, -0.7, -0.2, 0.2, 0.7, 1.1, 1.6, 1.9] {
            for v in [-1.9, -1.6, -1.1, -0.6, 0.2, 0.7, 1.1, 1.6, 1.9] {
                if let Some(p) = layout.projection.project(spherical(u, v)).unwrap() {
                    let expected = inside([u, v], &exterior()) && !inside([u, v], &split_hole());
                    assert_eq!(
                        layout.scene.hit_test(p[0], p[1]).is_some(),
                        expected,
                        "{projection:?}, ({u},{v})"
                    );
                    tested += 1;
                }
            }
        }
        assert!(tested > 20);
    }
}
#[test]
fn horizon_vertices_edges_and_tangencies_have_deterministic_topology() {
    let c = ChartConfig::default();
    for points in [
        vec![[-2., -1.], [0., -1.], [0., 1.], [-2., 1.]],
        vec![[0., 0.], [1., -1.], [1., 1.]],
        vec![[-1., -1.], [1., -1.], [0., 0.], [1., 1.], [-1., 1.]],
    ] {
        let a = geographic_map(
            &c,
            &[feature(vec![ring(&points)])],
            &opts(GeoProjection::Orthographic),
        )
        .unwrap();
        let mut reversed = points.clone();
        reversed.reverse();
        let b = geographic_map(
            &c,
            &[feature(vec![ring(&reversed)])],
            &opts(GeoProjection::Orthographic),
        )
        .unwrap();
        assert_eq!(moves(&a), moves(&b));
        a.validate().unwrap();
        b.validate().unwrap();
    }
}
#[test]
fn invalid_simple_ring_hole_and_work_budget_profiles_reject_without_mutating_sources() {
    let c = ChartConfig::default();
    let o = opts(GeoProjection::Orthographic);
    let crossing = feature(vec![ring(&[
        [-2., -1.],
        [2., 2.],
        [-2., 2.],
        [2., -2.],
        [-1., -2.],
    ])]);
    assert!(geographic_map(&c, &[crossing], &o).is_err());
    let crossing_hole = feature(vec![
        ring(&exterior()),
        ring(&[[-1., -1.5], [1.5, 0.], [-1., 1.5]]),
    ]);
    assert!(
        geographic_map(&c, &[crossing_hole], &o).is_err(),
        "checking only hole vertices would accept boundary crossings"
    );
    let outside = feature(vec![
        ring(&exterior()),
        ring(&[[-1., -0.3], [-0.5, -0.3], [-0.5, 0.3], [-1., 0.3]]),
    ]);
    assert!(geographic_map(&c, &[outside], &o).is_err());
    let large = |r: f64| {
        (0..3000)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 3000.;
                spherical(r * a.cos(), r * a.sin())
            })
            .collect()
    };
    let input = vec![feature(vec![large(2.), large(1.)])];
    let before = input.clone();
    let error = geographic_map(&c, &input, &o).unwrap_err();
    assert!(error.0.contains("work budget"));
    assert_eq!(input, before);
}
#[cfg(feature = "raster")]
#[test]
fn cpu_raster_pixels_preserve_island_gap_and_hole_notches() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let c = ChartConfig::default();
    for projection in projections() {
        let layout = geographic_map_layout(
            &c,
            &[feature(vec![ring(&exterior()), ring(&split_hole())])],
            &opts(projection),
        )
        .unwrap();
        let image = render_raster(
            &layout.scene,
            &FontRegistry::new(),
            &RasterOptions::default(),
        )
        .unwrap();
        for (u, v, filled) in [
            (-1.9, 1.5, true),
            (-1., 1.1, true),
            (-1., 1.5, false),
            (-1., 0., false),
            (-1.9, -1.5, true),
            (-1., -1.5, false),
        ] {
            let p = layout.projection.project(spherical(u, v)).unwrap().unwrap();
            let offset = (p[1].round() as usize * image.width as usize + p[0].round() as usize) * 4;
            if filled {
                assert_eq!(
                    &image.rgba[offset..offset + 4],
                    &[20, 100, 160, 255],
                    "{projection:?}: ({u},{v})"
                );
            } else {
                assert_eq!(
                    &image.rgba[offset..offset + 4],
                    &[255, 255, 255, 255],
                    "{projection:?}: ({u},{v})"
                );
            }
        }
    }
}
fn front(u: f64, v: f64) -> [f64; 2] {
    [u.atan().to_degrees(), v.atan2(u.hypot(1.)).to_degrees()]
}
fn front_ring(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    points.iter().map(|[u, v]| front(*u, *v)).collect()
}
#[test]
fn small_circle_handles_double_intersections_and_complete_cap_containment() {
    let c = ChartConfig::default();
    for projection in projections().into_iter().chain([GeoProjection::Gnomonic]) {
        let o = GeoMapOptions {
            clip_angle: Some(30.),
            ..opts(projection)
        };
        // Every subject vertex is outside, but each long horizontal edge
        // enters and exits the cap. A one-intersection clip loses the strip.
        let band = vec![[-2., -0.2], [2., -0.2], [2., 0.2], [-2., 0.2]];
        let layout = geographic_map_layout(&c, &[feature(vec![front_ring(&band)])], &o).unwrap();
        for u in [-0.4, 0., 0.4] {
            for v in [-0.3, -0.1, 0., 0.1, 0.3] {
                let p = layout.projection.project(front(u, v)).unwrap().unwrap();
                assert_eq!(
                    layout.scene.hit_test(p[0], p[1]).is_some(),
                    v.abs() < 0.2,
                    "{projection:?} at {u},{v}"
                );
            }
        }
        let square = vec![[-2., -2.], [2., -2.], [2., 2.], [-2., 2.]];
        let cap = geographic_map_layout(&c, &[feature(vec![front_ring(&square)])], &o).unwrap();
        assert_eq!(moves(&cap.scene), 1);
        for v in [-0.4, 0., 0.4] {
            let p = cap.projection.project(front(0., v)).unwrap().unwrap();
            assert!(
                cap.scene.hit_test(p[0], p[1]).is_some(),
                "{projection:?}: full cap {v}"
            );
        }
        assert!(cap.projection.project([31., 0.]).unwrap().is_none());
        render::to_svg(&cap.scene).unwrap();
        render::to_canvas_script(&cap.scene, "cap").unwrap();
    }
}
#[test]
fn small_circle_holes_and_nonconvex_contours_match_independent_membership() {
    let c = ChartConfig::default();
    let poly = vec![
        [-2., -2.],
        [2., -2.],
        [2., 2.],
        [-2., 2.],
        [-2., 0.2],
        [0.3, 0.2],
        [0.3, -0.2],
        [-2., -0.2],
    ];
    let hole = vec![[-0.25, 0.35], [0.25, 0.35], [0.25, 0.5], [-0.25, 0.5]];
    for projection in projections().into_iter().chain([GeoProjection::Gnomonic]) {
        let o = GeoMapOptions {
            clip_angle: Some(30.),
            ..opts(projection)
        };
        let layout = geographic_map_layout(
            &c,
            &[feature(vec![front_ring(&poly), front_ring(&hole)])],
            &o,
        )
        .unwrap();
        for u in [-0.4, -0.15, 0., 0.15, 0.4] {
            for v in [-0.4, -0.1, 0.1, 0.3, 0.4] {
                if let Some(p) = layout.projection.project(front(u, v)).unwrap() {
                    assert_eq!(
                        layout.scene.hit_test(p[0], p[1]).is_some(),
                        inside([u, v], &poly) && !inside([u, v], &hole),
                        "{projection:?} at {u},{v}"
                    );
                }
            }
        }
        let outer = front_ring(&[[-2., -2.], [2., -2.], [2., 2.], [-2., 2.]]);
        let hole = front_ring(&[[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]);
        let empty = geographic_map_layout(&c, &[feature(vec![outer, hole])], &o).unwrap();
        assert_eq!(
            moves(&empty.scene),
            0,
            "cap fully inside a hole emits no phantom outline"
        );
        let p = empty.projection.project([0., 0.]).unwrap().unwrap();
        assert!(empty.scene.hit_test(p[0], p[1]).is_none());
    }
}
#[test]
fn gnomonic_known_points_fit_and_explicit_clip_profile() {
    let o = opts(GeoProjection::Gnomonic);
    for (p, expected) in [
        ([0., 0.], [0., 0.]),
        ([30., 0.], [1. / 3_f64.sqrt(), 0.]),
        ([0., 30.], [0., 1. / 3_f64.sqrt()]),
        ([30., 30.], [1. / 3_f64.sqrt(), 2. / 3.]),
    ] {
        let actual = project_geo_point(p, &o).unwrap().unwrap();
        assert!((actual[0] - expected[0]).abs() < 1e-12 && (actual[1] - expected[1]).abs() < 1e-12);
    }
    assert!(project_geo_point([61., 0.], &o).unwrap().is_none());
    let c = ChartConfig::default();
    let layout = geographic_map_layout(&c, &[], &o).unwrap();
    let plot = layout.projection.plot();
    assert!(
        (layout.projection.scale() - plot.width.min(plot.height) / (2. * 3_f64.sqrt())).abs()
            < 1e-10
    );
    for (projection, angle) in [
        (GeoProjection::Equirectangular, 30.),
        (GeoProjection::Gnomonic, 90.),
        (GeoProjection::Orthographic, 0.),
        (GeoProjection::Orthographic, 91.),
        (GeoProjection::Orthographic, f64::NAN),
    ] {
        assert!(
            project_geo_point(
                [0., 0.],
                &GeoMapOptions {
                    projection,
                    clip_angle: Some(angle),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert!(
        project_geo_point(
            [0., 0.],
            &GeoMapOptions {
                clip_angle: Some(89.),
                ..o
            }
        )
        .unwrap()
        .is_some()
    );
}
#[test]
fn complete_cap_exterior_retains_interior_holes_and_zero_fill_has_no_outline() {
    let c = ChartConfig::default();
    for projection in projections().into_iter().chain([GeoProjection::Gnomonic]) {
        let o = GeoMapOptions {
            clip_angle: Some(30.),
            ..opts(projection)
        };
        let outer = front_ring(&[[-2., -2.], [2., -2.], [2., 2.], [-2., 2.]]);
        let inner = front_ring(&[[-0.1, -0.1], [0.1, -0.1], [0.1, 0.1], [-0.1, 0.1]]);
        let layout = geographic_map_layout(&c, &[feature(vec![outer.clone(), inner])], &o).unwrap();
        assert_eq!(
            moves(&layout.scene),
            2,
            "complete clip circle plus enclosed hole"
        );
        for (u, expected) in [(0., false), (0.3, true)] {
            let p = layout.projection.project(front(u, 0.)).unwrap().unwrap();
            assert_eq!(layout.scene.hit_test(p[0], p[1]).is_some(), expected);
        }
        let large_hole = front_ring(&[[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]);
        let empty = geographic_map(
            &c,
            &[feature(vec![outer, large_hole])],
            &GeoMapOptions {
                border: Some(Color(250, 0, 0, 255)),
                border_width: 5.,
                ..o
            },
        )
        .unwrap();
        assert!(
            empty.marks.is_empty(),
            "no filled or stroked phantom cap {projection:?}"
        );
    }
}
#[test]
fn cap_extremes_and_overlay_visibility_share_the_fitted_context() {
    let c = ChartConfig::default();
    for projection in projections().into_iter().chain([GeoProjection::Gnomonic]) {
        let o = GeoMapOptions {
            clip_angle: Some(0.01),
            ..opts(projection)
        };
        let source = feature(vec![front_ring(&[
            [-2., -2.],
            [2., -2.],
            [2., 2.],
            [-2., 2.],
        ])]);
        let mut layout = geographic_map_layout(&c, &[source], &o).unwrap();
        assert!(layout.projection.project([0.009, 0.]).unwrap().is_some());
        assert!(layout.projection.project([0.011, 0.]).unwrap().is_none());
        let p = layout.projection.project([0.005, 0.]).unwrap().unwrap();
        assert!(layout.scene.hit_test(p[0], p[1]).is_some());
        layout
            .add_symbols(
                &[
                    GeoSymbol::new("in", [0., 0.], Some(1.)),
                    GeoSymbol::new("out", [0.02, 0.], Some(1.)),
                ],
                &GeoSymbolOptions::default(),
            )
            .unwrap();
        assert_eq!(
            layout
                .scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("geo-symbol:"))
                .count(),
            1
        );
    }
    let o = GeoMapOptions {
        clip_angle: Some(89.99999999),
        ..opts(GeoProjection::Gnomonic)
    };
    assert!(
        project_geo_point([90., 0.], &o).unwrap().is_none(),
        "near-singular cap must not admit the horizon"
    );
    let layout = geographic_map_layout(&c, &[feature(vec![ring(&exterior())])], &o).unwrap();
    layout.scene.validate().unwrap();
    render::to_svg(&layout.scene).unwrap();
}
