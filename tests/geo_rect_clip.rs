use kartis::*;

fn rect() -> ClipRect {
    ClipRect {
        x: 0.,
        y: 0.,
        width: 10.,
        height: 10.,
    }
}

fn ring(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    points.to_vec()
}

fn signed_area(ring: &[[f64; 2]]) -> f64 {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        / 2.
}

fn area(rings: &[Vec<[f64; 2]>]) -> f64 {
    rings.iter().map(|r| signed_area(r)).sum::<f64>().abs()
}

fn contains(rings: &[Vec<[f64; 2]>], point: [f64; 2]) -> bool {
    let mut winding = 0i64;
    for ring in rings {
        for (a, b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            let cross = (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0]);
            if a[1] <= point[1] {
                if b[1] > point[1] && cross > 0. {
                    winding += 1;
                }
            } else if b[1] <= point[1] && cross < 0. {
                winding -= 1;
            }
        }
    }
    winding != 0
}

#[test]
fn a_concave_polygon_splits_without_bridging_the_empty_middle() {
    let source = [ring(&[
        [-2., -2.],
        [3., -2.],
        [3., 12.],
        [7., 12.],
        [7., -2.],
        [12., -2.],
        [12., 15.],
        [-2., 15.],
    ])];
    let out = clip_geo_planar_polygon(&source, rect(), &Default::default()).unwrap();
    assert_eq!(out.rings.len(), 2);
    assert!((area(&out.rings) - 60.).abs() < 1e-10);
    assert!(contains(&out.rings, [1., 5.]));
    assert!(contains(&out.rings, [9., 5.]));
    assert!(!contains(&out.rings, [5., 5.]));
    assert!(
        out.rings
            .iter()
            .flatten()
            .all(|p| rect().contains(p[0], p[1]))
    );
}

#[test]
fn holes_rejoin_at_the_clip_edge_and_can_remove_the_whole_window() {
    let outer = ring(&[[-5., -5.], [15., -5.], [15., 15.], [-5., 15.]]);
    let through = ring(&[[5., 2.], [5., 8.], [12., 8.], [12., 2.]]);
    let out =
        clip_geo_planar_polygon(&[outer.clone(), through], rect(), &Default::default()).unwrap();
    assert_eq!(out.rings.len(), 1);
    assert!((area(&out.rings) - 70.).abs() < 1e-10);
    assert!(contains(&out.rings, [1., 5.]));
    assert!(!contains(&out.rings, [7., 5.]));
    assert!(contains(&out.rings, [9., 1.]));
    let inner = ring(&[[2., 2.], [2., 4.], [4., 4.], [4., 2.]]);
    let out =
        clip_geo_planar_polygon(&[outer.clone(), inner], rect(), &Default::default()).unwrap();
    assert_eq!(out.rings.len(), 2);
    assert!((area(&out.rings) - 96.).abs() < 1e-10);
    assert!(!contains(&out.rings, [3., 3.]));
    let hole = ring(&[[-2., -2.], [-2., 12.], [12., 12.], [12., -2.]]);
    let empty = clip_geo_planar_polygon(&[outer, hole], rect(), &Default::default()).unwrap();
    assert!(empty.rings.is_empty());
}

#[test]
fn complete_cover_outside_boundary_and_native_contacts_do_not_invent_area() {
    let cover = [ring(&[[-2., -2.], [12., -2.], [12., 12.], [-2., 12.]])];
    let out = clip_geo_planar_polygon(&cover, rect(), &Default::default()).unwrap();
    assert_eq!(
        out.rings,
        vec![ring(&[[0., 0.], [10., 0.], [10., 10.], [0., 10.]])]
    );
    assert_eq!(area(&out.rings), 100.);
    let outside = [ring(&[[-4., 2.], [-2., 2.], [-2., 8.], [-4., 8.]])];
    assert!(
        clip_geo_planar_polygon(&outside, rect(), &Default::default())
            .unwrap()
            .rings
            .is_empty()
    );
    let coincident = [ring(&[[-2., -2.], [0., -2.], [0., 12.], [-2., 12.]])];
    let touch = clip_geo_planar_polygon(&coincident, rect(), &Default::default()).unwrap();
    assert_eq!(area(&touch.rings), 0.);
    assert!(touch.rings.iter().flatten().all(|p| p[0] == 0.));
    let corners = clip_geo_planar_lines(
        &[ring(&[[-2., 2.], [2., -2.]])],
        rect(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(corners.lines.len(), 1);
    assert!(corners.lines[0].iter().all(|p| *p == [0., 0.]));
}

#[test]
fn open_lines_reenter_independently_without_cross_window_connections() {
    let source = [
        ring(&[
            [-5., 2.],
            [5., 2.],
            [15., 2.],
            [15., 8.],
            [5., 8.],
            [-5., 8.],
        ]),
        ring(&[[-2., 0.], [12., 0.]]),
        ring(&[[5., 5.]]),
    ];
    let out = clip_geo_planar_lines(&source, rect(), &Default::default()).unwrap();
    assert_eq!(
        out.lines,
        vec![
            ring(&[[0., 2.], [5., 2.], [10., 2.]]),
            ring(&[[10., 8.], [5., 8.], [0., 8.]]),
            ring(&[[0., 0.], [10., 0.]]),
            ring(&[[5., 5.]]),
        ]
    );
    assert_eq!(out.stats.output_points, 9);
}

#[test]
fn contours_keep_native_winding_without_inferring_holes_from_list_position() {
    let first = ring(&[[1., 1.], [4., 1.], [4., 4.], [1., 4.]]);
    let second = ring(&[[6., 6.], [9., 6.], [9., 9.], [6., 9.], [6., 6.]]);
    let out =
        clip_geo_planar_polygon(&[first.clone(), second], rect(), &Default::default()).unwrap();
    assert_eq!(out.rings.len(), 2);
    assert_eq!(area(&out.rings), 18.);
    assert!(contains(&out.rings, [2., 2.]));
    assert!(contains(&out.rings, [7., 7.]));
    assert!(!contains(&out.rings, [5., 5.]));
    assert_eq!(out.rings[0], first);
}

#[test]
fn finite_inputs_rectangles_and_exact_resource_limits_are_checked_transactionally() {
    let source = [ring(&[[2., 2.], [8., 2.], [8., 8.], [2., 8.]])];
    let options = GeoRectClipOptions {
        max_points: 4,
        ..Default::default()
    };
    let out = clip_geo_planar_polygon(&source, rect(), &options).unwrap();
    assert_eq!(out.stats.input_points, 4);
    assert_eq!(out.stats.buffered_points, 5);
    assert_eq!(out.stats.output_points, 4);
    assert!(out.stats.work > 0 && out.stats.work <= options.max_work);
    for options in [
        GeoRectClipOptions {
            max_points: 3,
            ..Default::default()
        },
        GeoRectClipOptions {
            max_work: 1,
            ..Default::default()
        },
        GeoRectClipOptions {
            max_points: 0,
            ..Default::default()
        },
    ] {
        assert!(clip_geo_planar_polygon(&source, rect(), &options).is_err());
    }
    for point in [[f64::NAN, 0.], [0., f64::INFINITY]] {
        assert!(clip_geo_planar_lines(&[vec![point]], rect(), &Default::default()).is_err());
    }
    assert!(
        clip_geo_planar_polygon(&[vec![[0., 0.], [1., 1.]]], rect(), &Default::default()).is_err()
    );
    let large = clip_geo_planar_lines(
        &[ring(&[[-1e12, 5.], [1e12, 5.]])],
        rect(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(large.lines.len(), 1);
    assert!((large.lines[0][0][0]).abs() < 1e-6);
    assert!((large.lines[0][1][0] - 10.).abs() < 1e-6);
    assert!(large.lines[0].iter().all(|p| p[1] == 5.));
    let huge = [ring(&[
        [-1e308, -1e308],
        [1e308, -1e308],
        [1e308, 1e308],
        [-1e308, 1e308],
    ])];
    let arithmetic = clip_geo_planar_polygon(&huge, rect(), &Default::default()).unwrap_err();
    assert!(arithmetic.0.contains("arithmetic"));
    for invalid in [
        ClipRect {
            width: 0.,
            ..rect()
        },
        ClipRect {
            height: -1.,
            ..rect()
        },
        ClipRect {
            x: f64::NAN,
            ..rect()
        },
    ] {
        assert!(clip_geo_planar_polygon(&source, invalid, &Default::default()).is_err());
    }
    assert_eq!(
        clip_geo_planar_polygon(&source, rect(), &Default::default())
            .unwrap()
            .rings,
        out.rings
    );
    assert!(
        clip_geo_planar_polygon(&[], rect(), &Default::default())
            .unwrap()
            .rings
            .is_empty()
    );
    assert!(
        clip_geo_planar_lines(&[], rect(), &Default::default())
            .unwrap()
            .lines
            .is_empty()
    );
}

#[test]
fn empty_line_headers_consume_work_even_when_they_emit_no_points() {
    let lines = vec![Vec::new(); 100];
    let options = GeoRectClipOptions {
        max_work: 3,
        ..Default::default()
    };
    assert!(clip_geo_planar_lines(&lines, rect(), &options).is_err());
    let enough = clip_geo_planar_lines(&lines, rect(), &Default::default()).unwrap();
    assert!(enough.lines.is_empty());
    assert_eq!(enough.stats.input_points, 0);
    assert_eq!(enough.stats.output_points, 0);
    assert!(enough.stats.work >= lines.len());
}

#[cfg(feature = "geojson")]
#[test]
fn original_d3_rectangle_stream_matches_all_frozen_cases_including_native_order() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/geo-rect-clip/values.json")).unwrap();
    assert_eq!(fixture["d3GeoVersion"], "3.1.1");
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 383);
    for case in cases {
        let source: Vec<Vec<[f64; 2]>> = serde_json::from_value(case["input"].clone()).unwrap();
        let expected: Vec<Vec<[f64; 2]>> = serde_json::from_value(case["output"].clone()).unwrap();
        let r: [f64; 4] = serde_json::from_value(case["rect"].clone()).unwrap();
        let rect = ClipRect {
            x: r[0],
            y: r[1],
            width: r[2] - r[0],
            height: r[3] - r[1],
        };
        let (actual, stats) = if case["kind"] == "polygon" {
            let out = clip_geo_planar_polygon(&source, rect, &Default::default()).unwrap();
            (out.rings, out.stats)
        } else {
            let out = clip_geo_planar_lines(&source, rect, &Default::default()).unwrap();
            (out.lines, out.stats)
        };
        assert_eq!(actual.len(), expected.len(), "{}: {actual:?}", case["name"]);
        assert_eq!(
            stats.output_points,
            actual.iter().map(Vec::len).sum::<usize>()
        );
        for (a, b) in actual.iter().zip(&expected) {
            assert_eq!(
                a.len(),
                b.len(),
                "{}: {actual:?} vs {expected:?}",
                case["name"]
            );
            for (a, b) in a.iter().zip(b) {
                assert!(
                    (a[0] - b[0]).abs() < 1e-8 && (a[1] - b[1]).abs() < 1e-8,
                    "{}: {a:?} != {b:?}",
                    case["name"]
                );
            }
        }
    }
}
