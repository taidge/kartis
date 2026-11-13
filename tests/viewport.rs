use kartis::*;

fn plot() -> ClipRect {
    ClipRect {
        x: 40.,
        y: 30.,
        width: 400.,
        height: 200.,
    }
}
fn axes() -> ViewportAxes {
    ViewportAxes {
        x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
        left: Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
        right: Some(ViewportAxis::new([1., 10000.], ScaleKind::Log)),
    }
}
fn viewport() -> DomainViewport {
    DomainViewport::new(axes(), plot(), ViewportLimits::default()).unwrap()
}
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
fn range(a: [f64; 2], b: [f64; 2]) {
    close(a[0], b[0]);
    close(a[1], b[1]);
}

#[test]
fn absolute_transform_rescales_original_domains_and_independent_value_units() {
    // Original pixel interval is inverted through T^-1, exactly D3's
    // rescaleX/rescaleY contract; margins never enter the local transform.
    let mut v = viewport();
    let request = ViewportRequest::Transform {
        transform: ZoomTransform::new(2., -100., -50.).unwrap(),
        axes: ViewportSelection::Both,
    };
    let d = v.apply(request).unwrap();
    range(d.x, [12.5, 62.5]);
    range(d.left.unwrap(), [37.5, 87.5]);
    range(d.right.unwrap(), [10_f64.powf(1.5), 10_f64.powf(3.5)]);
    let first = v.clone();
    assert_eq!(
        v.apply(request).unwrap(),
        d,
        "absolute transforms must not compound"
    );
    assert_eq!(v, first);
    assert_eq!(
        v.transform_for(ViewportSelection::Both).unwrap(),
        ZoomTransform::new(2., -100., -50.).unwrap()
    );
    // The resolved ranges feed real chart builders, with fixed screen axes.
    let c = ChartConfig {
        width: 480.,
        height: 260.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    };
    let scene = dual_axis(
        &c,
        &[
            DualAxisSeries::new(
                Series::new(
                    "left",
                    vec![
                        Point::new(d.x[0], d.left.unwrap()[0]),
                        Point::new(d.x[1], d.left.unwrap()[1]),
                    ],
                ),
                YAxisSide::Left,
            ),
            DualAxisSeries::new(
                Series::new(
                    "right",
                    vec![
                        Point::new(d.x[0], d.right.unwrap()[0]),
                        Point::new(d.x[1], d.right.unwrap()[1]),
                    ],
                ),
                YAxisSide::Right,
            ),
        ],
        &DualAxisOptions {
            x_domain: Some(d.x),
            left: YAxisOptions {
                domain: d.left,
                ..Default::default()
            },
            right: YAxisOptions {
                domain: d.right,
                scale: ScaleKind::Log,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let points = scene
        .marks
        .iter()
        .filter_map(|m| match m.geometry {
            Geometry::Circle { x, y, .. } => Some((x, y)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(points.len(), 4);
    for pair in points.as_chunks::<2>().0 {
        close(pair[0].0, 40.);
        close(pair[0].1, 230.);
        close(pair[1].0, 440.);
        close(pair[1].1, 30.);
    }
}

#[test]
fn anchor_zoom_and_pixel_pan_have_exact_domain_meaning_and_bounded_edges() {
    let mut v = viewport();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 4.,
            anchor: (100., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [18.75, 43.75]);
    range(d.left.unwrap(), [37.5, 62.5]);
    let scale = ChartScale::new(ScaleKind::Linear, d.x, [0., 400.]).unwrap();
    close(scale.map(25.), 100.);
    let y = ChartScale::new(ScaleKind::Linear, d.left.unwrap(), [200., 0.]).unwrap();
    close(y.map(50.), 100.);
    let d = v
        .apply(ViewportRequest::Pan {
            delta: (40., -20.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [16.25, 41.25]);
    range(d.left.unwrap(), [35., 60.]);
    let d = v
        .apply(ViewportRequest::Pan {
            delta: (f64::MAX, -f64::MAX),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [0., 25.]);
    range(d.left.unwrap(), [0., 25.]);
    assert_eq!(v.reset().x, axes().x.domain);
    assert_eq!(v.zoom(), (1., 1.));
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: f64::MAX,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [49.5, 50.5]);
    assert_eq!(v.zoom(), (100., 100.));
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: f64::MIN_POSITIVE,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert_eq!(d.x, [0., 100.]);
    assert_eq!(d.left, Some([0., 100.]));
}

#[test]
fn independent_axis_selection_is_explicit_and_proposals_are_read_only() {
    let mut v = viewport();
    let initial = v.domains();
    let r = ViewportRequest::Zoom {
        factor: 2.,
        anchor: (200., 100.),
        axes: ViewportSelection::X,
    };
    let proposed = v.resolve(r).unwrap();
    assert_eq!(v.domains(), initial);
    assert_eq!(v.apply(r).unwrap(), proposed);
    assert_eq!(v.domains().left, initial.left);
    assert_eq!(v.domains().right, initial.right);
    assert_eq!(
        v.normalized_windows(),
        ViewportWindows {
            x: [0.25, 0.75],
            y: [0., 1.]
        }
    );
    assert!(v.transform_for(ViewportSelection::Both).is_err());
    assert_eq!(
        v.transform_for(ViewportSelection::X).unwrap(),
        ZoomTransform::new(2., -200., 0.).unwrap()
    );
    let r = ViewportRequest::Transform {
        transform: ZoomTransform::new(4., 0., -300.).unwrap(),
        axes: ViewportSelection::Y,
    };
    let d = v.apply(r).unwrap();
    assert_eq!(d.x, proposed.x);
    range(d.left.unwrap(), [37.5, 62.5]);
    assert_eq!(v.zoom(), (2., 4.));
}

#[test]
fn resize_preserves_numeric_ranges_and_rebuilds_absolute_pixel_transform() {
    let mut v = viewport();
    v.apply(ViewportRequest::Zoom {
        factor: 4.,
        anchor: (100., 150.),
        axes: ViewportSelection::Both,
    })
    .unwrap();
    let before = v.clone();
    let d = v.domains();
    assert_eq!(
        v.resize(ClipRect {
            x: 100.,
            y: 100.,
            width: 800.,
            height: 400.
        })
        .unwrap(),
        d
    );
    assert_eq!(v.normalized_windows(), before.normalized_windows());
    let t0 = before.transform_for(ViewportSelection::Both).unwrap();
    let t1 = v.transform_for(ViewportSelection::Both).unwrap();
    assert_eq!(t1.scale, t0.scale);
    close(t1.translate_x, 2. * t0.translate_x);
    close(t1.translate_y, 2. * t0.translate_y);
    let mut old = before;
    let expected = old
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert_eq!(
        v.apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (400., 200.),
            axes: ViewportSelection::Both
        })
        .unwrap(),
        expected
    );
    let before = v.clone();
    assert!(
        v.resize(ClipRect {
            width: 0.,
            ..plot()
        })
        .is_err()
    );
    assert_eq!(v, before);
}

#[test]
fn brush_uses_current_domains_clips_to_plot_and_obeys_zoom_limits() {
    let mut v = viewport();
    let d = v
        .apply(ViewportRequest::Brush {
            bounds: Brush::from_points((100., 50.), (300., 100.)).unwrap(),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [25., 75.]);
    range(d.left.unwrap(), [50., 75.]);
    range(d.right.unwrap(), [100., 1000.]);
    let d = v
        .apply(ViewportRequest::Brush {
            bounds: Brush::from_points((100., 0.), (300., 0.)).unwrap(),
            axes: ViewportSelection::X,
        })
        .unwrap();
    range(d.x, [37.5, 62.5]);
    range(d.left.unwrap(), [50., 75.]);
    let before = v.clone();
    for bounds in [
        Brush {
            x0: 500.,
            x1: 600.,
            y0: 0.,
            y1: 200.,
        },
        Brush {
            x0: 100.,
            x1: 100.,
            y0: 0.,
            y1: 100.,
        },
    ] {
        assert!(
            v.apply(ViewportRequest::Brush {
                bounds,
                axes: ViewportSelection::Both
            })
            .is_err()
        );
        assert_eq!(v, before);
    }
    let mut v = DomainViewport::new(
        axes(),
        plot(),
        ViewportLimits {
            max_zoom: 4.,
            ..Default::default()
        },
    )
    .unwrap();
    let d = v
        .apply(ViewportRequest::Brush {
            bounds: Brush::from_points((199., -20.), (201., 220.)).unwrap(),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [37.5, 62.5]);
    range(d.left.unwrap(), [0., 100.]);
}

#[test]
fn logarithmic_negative_and_symmetric_domains_are_inverted_in_scale_space() {
    let mut a = axes();
    a.x = ViewportAxis::new([-1000., -1.], ScaleKind::Log);
    a.left = Some(ViewportAxis::new(
        [-99., 99.],
        ScaleKind::SymLog { constant: 1. },
    ));
    let mut v = DomainViewport::new(a, plot(), ViewportLimits::default()).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [-10_f64.powf(2.25), -10_f64.powf(0.75)]);
    range(d.left.unwrap(), [-9., 9.]);
    let x = ChartScale::new(ScaleKind::Log, d.x, [0., 400.]).unwrap();
    close(x.map(-10_f64.powf(1.5)), 200.);
    let y = ChartScale::new(
        ScaleKind::SymLog { constant: 1. },
        d.left.unwrap(),
        [200., 0.],
    )
    .unwrap();
    close(y.map(0.), 100.);
    assert_eq!(v.reset().x, [-1000., -1.]);
}

#[test]
fn utc_windows_remain_integral_and_stop_at_one_millisecond() {
    let mut a = axes();
    a.x = ViewportAxis::new([0., 1000.], ScaleKind::Utc);
    let mut v = DomainViewport::new(
        a,
        plot(),
        ViewportLimits {
            max_zoom: 1_000_000.,
            ..Default::default()
        },
    )
    .unwrap();
    range(
        v.apply(ViewportRequest::Zoom {
            factor: 100.,
            anchor: (200., 100.),
            axes: ViewportSelection::X,
        })
        .unwrap()
        .x,
        [495., 505.],
    );
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: f64::MAX,
            anchor: (200., 100.),
            axes: ViewportSelection::X,
        })
        .unwrap();
    assert!(d.x.iter().all(|v| v.fract() == 0.));
    assert_eq!(d.x[1] - d.x[0], 1.);
    assert_eq!(v.zoom().0, 1000.);
    a.x = ViewportAxis::new(
        [MAX_TIMESTAMP_MS as f64 - 4., MAX_TIMESTAMP_MS as f64],
        ScaleKind::Utc,
    );
    let mut v = DomainViewport::new(a, plot(), ViewportLimits::default()).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 100.,
            anchor: (400., 0.),
            axes: ViewportSelection::X,
        })
        .unwrap();
    assert!(d.x.iter().all(|v| v.fract() == 0.));
    assert!(d.x[0] >= a.x.domain[0]);
    assert!(d.x[1] <= a.x.domain[1]);
    assert!(d.x[1] > d.x[0]);
    assert_eq!(v.reset().x, a.x.domain);
}

#[test]
fn invalid_profiles_and_failed_gestures_are_atomic_including_inverse_overflow() {
    for axis in [
        ViewportAxis::new([2., 1.], ScaleKind::Linear),
        ViewportAxis::new([1., 1.], ScaleKind::Linear),
        ViewportAxis::new([-1., 1.], ScaleKind::Log),
        ViewportAxis::new([0., 1.], ScaleKind::Log),
        ViewportAxis::new([0.5, 1.5], ScaleKind::Utc),
        ViewportAxis::new([0., f64::NAN], ScaleKind::Linear),
        ViewportAxis::new([0., 1.], ScaleKind::SymLog { constant: 0. }),
    ] {
        let mut a = axes();
        a.x = axis;
        assert!(DomainViewport::new(a, plot(), ViewportLimits::default()).is_err());
    }
    let mut v = viewport();
    for r in [
        ViewportRequest::Zoom {
            factor: 0.,
            anchor: (0., 0.),
            axes: ViewportSelection::Both,
        },
        ViewportRequest::Zoom {
            factor: 2.,
            anchor: (401., 0.),
            axes: ViewportSelection::Both,
        },
        ViewportRequest::Pan {
            delta: (f64::NAN, 0.),
            axes: ViewportSelection::X,
        },
        ViewportRequest::Transform {
            transform: ZoomTransform {
                scale: 0.,
                translate_x: 0.,
                translate_y: 0.,
            },
            axes: ViewportSelection::X,
        },
    ] {
        let before = v.clone();
        assert!(v.apply(r).is_err());
        assert_eq!(v, before);
    }
    let mut v = DomainViewport::new(
        axes(),
        plot(),
        ViewportLimits {
            bounded: false,
            ..Default::default()
        },
    )
    .unwrap();
    let before = v.clone();
    assert!(
        v.apply(ViewportRequest::Pan {
            delta: (f64::MAX, 0.),
            axes: ViewportSelection::X
        })
        .is_err()
    );
    assert_eq!(v, before);
    let mut a = axes();
    a.x = ViewportAxis::new([-f64::MAX / 4., f64::MAX / 4.], ScaleKind::Linear);
    let mut v = DomainViewport::new(a, plot(), ViewportLimits::default()).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [-f64::MAX / 8., f64::MAX / 8.]);
    assert!(d.x.iter().all(|v| v.is_finite()));
}

#[test]
fn unbounded_zoom_out_and_gesture_minimum_have_documented_reset_behavior() {
    let limits = ViewportLimits {
        min_zoom: 0.5,
        max_zoom: 10.,
        bounded: false,
    };
    let mut v = DomainViewport::new(axes(), plot(), limits).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 0.1,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [-50., 150.]);
    range(d.left.unwrap(), [-50., 150.]);
    assert_eq!(v.limits(), limits);
    let mut v = DomainViewport::new(
        axes(),
        plot(),
        ViewportLimits {
            min_zoom: 2.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(v.zoom(), (1., 1.));
    range(
        v.apply(ViewportRequest::Zoom {
            factor: 1.,
            anchor: (200., 100.),
            axes: ViewportSelection::X,
        })
        .unwrap()
        .x,
        [25., 75.],
    );
    assert_eq!(v.apply(ViewportRequest::Reset).unwrap().x, [0., 100.]);
}

#[test]
fn absent_axes_time_resolution_and_floating_precision_do_not_corrupt_state() {
    let mut a = axes();
    a.left = None;
    a.right = None;
    let mut v = DomainViewport::new(a, plot(), ViewportLimits::default()).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [25., 75.]);
    assert_eq!(d.left, None);
    assert_eq!(d.right, None);
    assert_eq!(
        v.transform_for(ViewportSelection::Both).unwrap(),
        ZoomTransform::new(2., -200., 0.).unwrap()
    );
    let before = v.clone();
    assert!(
        v.apply(ViewportRequest::Pan {
            delta: (0., 20.),
            axes: ViewportSelection::Y
        })
        .is_err()
    );
    assert_eq!(v, before);
    a.right = Some(ViewportAxis::new([100., 101.], ScaleKind::Utc));
    let mut v = DomainViewport::new(a, plot(), ViewportLimits::default()).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 10.,
            anchor: (200., 100.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    range(d.x, [45., 55.]);
    assert_eq!(d.right, Some([100., 101.]));
    assert_eq!(v.zoom(), (10., 1.));
    assert!(
        v.transform_for(ViewportSelection::Both).is_err(),
        "one transform cannot change the saturated millisecond axis"
    );
    a.x = ViewportAxis::new([1., 1. + f64::EPSILON], ScaleKind::Linear);
    let mut v = DomainViewport::new(a, plot(), ViewportLimits::default()).unwrap();
    let before = v.clone();
    assert!(
        v.apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (0., 0.),
            axes: ViewportSelection::X
        })
        .is_err(),
        "unrepresentable endpoint differences must reject atomically"
    );
    assert_eq!(v, before);
}
