use glory::render::{GraphicPoint as P, PathCommand as C, StrokeCap};
use kartis::*;
fn line(length: f64) -> Geometry {
    Geometry::Path(vec![C::Move(P::new(0., 0.)), C::Line(P::new(length, 0.))])
}
fn dash(segments: &[f64], offset: f64) -> StrokeDash {
    StrokeDash {
        segments: segments.to_vec(),
        offset,
    }
}
fn runs(geometry: &Geometry) -> Vec<Vec<P>> {
    let Geometry::Path(path) = geometry else {
        panic!("expected dashed path")
    };
    let mut output: Vec<Vec<P>> = Vec::new();
    for command in path {
        match command {
            C::Move(point) => output.push(vec![*point]),
            C::Line(point) => output.last_mut().unwrap().push(*point),
            C::Close => {}
            _ => panic!("expected flattened path"),
        }
    }
    output
}
fn ends(geometry: &Geometry) -> Vec<[f64; 2]> {
    runs(geometry)
        .iter()
        .map(|run| [run[0].x, run.last().unwrap().x])
        .collect()
}

#[test]
fn linear_goldens_odd_arrays_offsets_and_boundary_zero_dashes_follow_svg_positions() {
    let limits = StrokeDashLimits::default();
    assert_eq!(
        ends(&dash_geometry(&line(25.), &dash(&[5., 3.], 0.), &limits).unwrap()),
        [[0., 5.], [8., 13.], [16., 21.], [24., 25.]]
    );
    assert_eq!(
        ends(&dash_geometry(&line(10.), &dash(&[3.], 0.), &limits).unwrap()),
        [[0., 3.], [6., 9.]]
    );
    assert_eq!(
        ends(&dash_geometry(&line(12.), &dash(&[5., 3.], 2.), &limits).unwrap()),
        [[0., 3.], [6., 11.]]
    );
    assert_eq!(
        ends(&dash_geometry(&line(12.), &dash(&[5., 3.], -2.), &limits).unwrap()),
        [[2., 7.], [10., 12.]]
    );
    assert_eq!(
        ends(&dash_geometry(&line(12.), &dash(&[5., 3.], 5.), &limits).unwrap()),
        [[0., 0.], [3., 8.], [11., 12.]]
    );
    for offset in [-26., -18., -10., -2., 6., 14., 22.] {
        assert_eq!(
            dash_geometry(&line(12.), &dash(&[5., 3.], offset), &limits).unwrap(),
            dash_geometry(&line(12.), &dash(&[5., 3.], -2.), &limits).unwrap()
        );
    }
    assert_eq!(
        dash(&[3., 2., 1.], -1.).normalized().unwrap(),
        dash(&[3., 2., 1., 3., 2., 1.], 11.)
    );
}

#[test]
fn moves_reset_phase_and_move_only_or_zero_length_subpaths_have_explicit_semantics() {
    let limits = StrokeDashLimits::default();
    let geometry = Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Line(P::new(8., 0.)),
        C::Move(P::new(100., 0.)),
        C::Line(P::new(108., 0.)),
    ]);
    assert_eq!(
        ends(&dash_geometry(&geometry, &dash(&[3., 2.], 1.), &limits).unwrap()),
        [[0., 2.], [4., 7.], [100., 102.], [104., 107.]]
    );
    let geometry = Geometry::Path(vec![
        C::Move(P::new(1., 1.)),
        C::Move(P::new(2., 2.)),
        C::Line(P::new(2., 2.)),
    ]);
    let result = dash_geometry_detailed(&geometry, &dash(&[3., 2.], 0.), &limits).unwrap();
    assert_eq!(result.dots.len(), 1);
    assert_eq!(result.dots[0].point, P::new(2., 2.));
    assert!(runs(&dash_geometry(&geometry, &dash(&[3., 2.], 4.), &limits).unwrap()).is_empty());
}

#[test]
fn zero_patterns_are_solid_but_zero_dashes_remain_real_dots_and_keep_source_tangents() {
    let limits = StrokeDashLimits::default();
    let source = Geometry::Circle {
        x: 50.,
        y: 50.,
        radius: 20.,
    };
    for segments in [vec![], vec![0.], vec![0., 0., 0.]] {
        assert_eq!(
            dash_geometry(
                &source,
                &StrokeDash {
                    segments,
                    offset: f64::MAX
                },
                &limits
            )
            .unwrap(),
            source
        );
    }
    let result = dash_geometry_detailed(&line(20.), &dash(&[0., 10.], 0.), &limits).unwrap();
    assert_eq!(ends(&result.geometry), [[0., 0.], [10., 10.]]);
    assert_eq!(result.dots.len(), 2); // No extra dot at the terminal gap boundary.
    let diagonal = Geometry::Path(vec![C::Move(P::new(0., 0.)), C::Line(P::new(30., 40.))]);
    let result = dash_geometry_detailed(&diagonal, &dash(&[0., 10.], 0.), &limits).unwrap();
    assert_eq!(result.dots.len(), 5);
    assert_eq!(result.dots[2].point, P::new(12., 16.));
    assert_eq!(result.dots[2].incoming, [0.6, 0.8]);
    assert_eq!(result.dots[2].outgoing, [0.6, 0.8]);
    let dot = result.dots[2];
    assert!(
        matches!(dash_dot_geometry(&dot,4.,StrokeCap::Butt).unwrap(),Geometry::Path(path) if path.is_empty())
    );
    assert_eq!(
        dash_dot_geometry(&dot, 4., StrokeCap::Round).unwrap(),
        Geometry::Circle {
            x: 12.,
            y: 16.,
            radius: 2.
        }
    );
    let square = dash_dot_geometry(&dot, 4., StrokeCap::Square).unwrap();
    let mut scene = ChartConfig::default().scene().unwrap();
    scene
        .marks
        .push(Mark::new("square", square, Color(0, 0, 0, 255)));
    assert!(scene.hit_test(12., 16.).is_some());
    assert!(
        scene.hit_test(14.5, 16.).is_some(),
        "rotated cap reaches beyond the axis-aligned square"
    );
    assert!(scene.hit_test(14.5, 18.5).is_none());
}

#[test]
fn closed_subpaths_preserve_continuous_seam_joins_and_complete_closed_dash() {
    let source = Geometry::Rect {
        x: 0.,
        y: 0.,
        width: 10.,
        height: 10.,
    };
    let result =
        dash_geometry(&source, &dash(&[12., 5.], 0.), &StrokeDashLimits::default()).unwrap();
    let paths = runs(&result);
    assert_eq!(paths.len(), 2);
    assert_eq!(
        paths[0],
        [
            P::new(0., 6.),
            P::new(0., 0.),
            P::new(10., 0.),
            P::new(10., 2.)
        ]
    );
    for (actual, expected) in
        paths[1]
            .iter()
            .zip([P::new(10., 7.), P::new(10., 10.), P::new(1., 10.)])
    {
        assert!((actual.x - expected.x).abs() < 1e-12 && (actual.y - expected.y).abs() < 1e-12);
    }
    let whole = dash_geometry(
        &source,
        &dash(&[100., 1.], 0.),
        &StrokeDashLimits::default(),
    )
    .unwrap();
    assert_eq!(whole, Geometry::Path(source.commands()));
    let dotted_whole = dash_geometry_detailed(
        &source,
        &dash(&[0., 0., 100., 1.], 0.),
        &StrokeDashLimits::default(),
    )
    .unwrap();
    assert_eq!(dotted_whole.dots.len(), 1);
    assert_eq!(
        dotted_whole
            .geometry
            .commands()
            .iter()
            .filter(|c| matches!(c, C::Close))
            .count(),
        1,
        "a leading zero dash must not remove a following full closed dash's join"
    );
    let partial =
        dash_geometry(&source, &dash(&[7., 7.], 0.), &StrokeDashLimits::default()).unwrap();
    assert!(
        !partial
            .commands()
            .iter()
            .any(|command| matches!(command, C::Close))
    );
}

#[test]
fn cubics_bound_arc_length_error_and_keep_collinear_backtracking() {
    let source = Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Cubic {
            first: P::new(20., 0.),
            second: P::new(-10., 0.),
            end: P::new(10., 0.),
        },
    ]);
    let limits = StrokeDashLimits {
        tolerance: 0.001,
        length_error: 0.0001,
        ..Default::default()
    };
    let result = dash_geometry_detailed(&source, &dash(&[1000., 1.], 0.), &limits).unwrap();
    let points = &runs(&result.geometry)[0];
    let length = points
        .windows(2)
        .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
        .sum::<f64>();
    let analytic = 10. + 4. * 5_f64.sqrt();
    assert!(length > 18.94);
    assert!(analytic - length <= result.length_error_bound + 1e-10);
    assert!(result.length_error_bound <= limits.length_error);
    let source = Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Cubic {
            first: P::new(100., 0.),
            second: P::new(100., 100.),
            end: P::new(0., 100.),
        },
    ]);
    let result = dash_geometry_detailed(&source, &dash(&[1000., 1.], 0.), &limits).unwrap();
    let points = &runs(&result.geometry)[0];
    let length = points
        .windows(2)
        .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
        .sum::<f64>();
    // This cubic has exact length 200 (speed 300*(2t²-2t+1)).
    assert!(200. - length <= result.length_error_bound + 1e-9);
    assert!(result.length_error_bound <= limits.length_error);
    let circle = dash_geometry_detailed(
        &Geometry::Circle {
            x: 0.,
            y: 0.,
            radius: 100.,
        },
        &dash(&[10., 5.], 0.),
        &StrokeDashLimits::default(),
    )
    .unwrap();
    assert!(circle.flattened_segments > 16 && circle.length_error_bound <= 0.01);
}

#[test]
fn emitted_stroke_geometry_is_reusable_for_gap_aware_hit_and_brush_inputs() {
    let geometry = dash_geometry(
        &line(100.),
        &dash(&[10., 20.], 0.),
        &StrokeDashLimits::default(),
    )
    .unwrap();
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new("dash", geometry, Color(0, 0, 0, 255));
    mark.fill = None;
    mark.stroke = Some(glory::render::GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 2.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(mark);
    assert!(scene.hit_test(5., 0.).is_some());
    assert!(scene.hit_test(20., 0.).is_none());
    assert!(scene.hit_test(35., 0.).is_some());
}

#[test]
fn invalid_input_and_independent_work_flatten_output_budgets_fail_without_mutation() {
    let source = line(100.);
    let saved = source.clone();
    for pattern in [
        dash(&[-1., 2.], 0.),
        dash(&[f64::NAN], 0.),
        dash(&[1., 2.], f64::INFINITY),
        dash(&[f64::MAX, f64::MAX], 0.),
        dash(&[1.; 65], 0.),
    ] {
        assert!(dash_geometry(&source, &pattern, &StrokeDashLimits::default()).is_err());
    }
    for limits in [
        StrokeDashLimits {
            max_output_commands: 1,
            ..Default::default()
        },
        StrokeDashLimits {
            max_work: 1,
            ..Default::default()
        },
        StrokeDashLimits {
            max_flattened_segments: 0,
            ..Default::default()
        },
        StrokeDashLimits {
            length_error: f64::NAN,
            ..Default::default()
        },
    ] {
        assert!(dash_geometry(&source, &dash(&[1., 1.], 0.), &limits).is_err());
    }
    assert!(
        dash_geometry(
            &source,
            &dash(&[1e-12, 1e-12], 0.),
            &StrokeDashLimits {
                max_work: 1000,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(source, saved);
    for invalid in [
        Geometry::Path(vec![C::Line(P::new(0., 0.))]),
        Geometry::Path(vec![C::Move(P::new(f64::NAN, 0.))]),
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: -1.,
            height: 2.,
        },
    ] {
        assert!(
            dash_geometry(
                &invalid,
                &StrokeDash::default(),
                &StrokeDashLimits::default()
            )
            .is_err()
        );
    }
}
