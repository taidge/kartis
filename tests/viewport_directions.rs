use kartis::*;
fn plot() -> ClipRect {
    ClipRect {
        x: 40.,
        y: 30.,
        width: 400.,
        height: 200.,
    }
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
fn interval(a: [f64; 2], b: [f64; 2]) {
    near(a[0], b[0]);
    near(a[1], b[1]);
}
#[test]
fn off_center_zoom_pan_and_brush_preserve_independent_range_directions() {
    let mut v = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
            right: Some(ViewportAxis::new([1., 10000.], ScaleKind::Log)),
        },
        plot(),
        ViewportLimits::default(),
    )
    .unwrap()
    .with_directions(ViewportDirections {
        x: true,
        left: true,
        right: false,
    })
    .unwrap();
    let original = v.clone();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (100., 50.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    interval(d.x, [37.5, 87.5]);
    interval(d.left.unwrap(), [12.5, 62.5]);
    interval(d.right.unwrap(), [10_f64.powf(1.5), 10_f64.powf(3.5)]);
    // These are the original off-center values under the unchanged pointer.
    near(d.x[1] - (d.x[1] - d.x[0]) * 0.25, 75.);
    near(d.left.unwrap()[0] + 50. * 0.25, 25.);
    let p = v
        .apply(ViewportRequest::Pan {
            delta: (40., 20.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    interval(p.x, [42.5, 92.5]);
    interval(p.left.unwrap(), [7.5, 57.5]);
    let b = v
        .apply(ViewportRequest::Brush {
            bounds: Brush {
                x0: 100.,
                x1: 300.,
                y0: 50.,
                y1: 150.,
            },
            axes: ViewportSelection::Both,
        })
        .unwrap();
    interval(b.x, [55., 80.]);
    interval(b.left.unwrap(), [20., 45.]);
    interval(b.right.unwrap(), [10_f64.powf(2.2), 10_f64.powf(3.2)]);
    v.apply(ViewportRequest::Reset).unwrap();
    assert_eq!(v, original);
}
#[test]
fn scaled_line_controller_derives_double_reversals_and_keeps_actual_anchor() {
    let c = ChartConfig {
        width: 480.,
        height: 280.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 50.,
        },
        ..Default::default()
    };
    let source = [Series::new(
        "s",
        vec![
            Point::new(25., 75.),
            Point::new(75., 25.),
            Point::new(25., 25.),
        ],
    )];
    let o = LineOptions {
        x_domain: Some([100., 0.]),
        y_domain: Some([100., 0.]),
        ..Default::default()
    };
    let scales = LineScaleOptions {
        x: ContinuousScaleOptions {
            reverse: true,
            ..Default::default()
        },
        y: ContinuousScaleOptions {
            reverse: false,
            ..Default::default()
        },
    };
    let r = line_layout_with_scale_options(&c, &source, &o, &Default::default(), &scales).unwrap();
    let mut v = r.viewport_controller(ViewportLimits::default()).unwrap();
    assert_eq!(
        v.directions(),
        ViewportDirections {
            x: false,
            left: true,
            right: false
        }
    );
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (100., 50.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    interval(d.x, [12.5, 62.5]);
    interval(d.left.unwrap(), [12.5, 62.5]);
    let updated = LineOptions {
        x_domain: Some(d.x),
        y_domain: d.left,
        ..o
    };
    // Layouts reconstructed from a canonical viewport keep the effective
    // direction. Re-using an original descending-domain flag would reverse twice.
    let current = line_layout_with_scale_options(
        &c,
        &source,
        &updated,
        &Default::default(),
        &r.viewport_scale_options(),
    )
    .unwrap();
    let middle =
        ChartScale::with_options(d.x, [40., 440.], &ContinuousScaleOptions::default()).unwrap();
    near(middle.map(25.), 140.);
    assert_eq!(current.layout.plot, r.layout.plot);
    assert_eq!(
        current
            .layout
            .sites
            .iter()
            .find(|s| s.source_index == 2)
            .unwrap()
            .position,
        [140., 80.]
    );
}
#[test]
fn invalid_navigation_is_atomic_with_reversed_metadata_and_tiny_symlog() {
    let kind = ScaleKind::SymLog { constant: 1e-300 };
    let mut v = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([-1e300, 1e300], kind),
            left: None,
            right: None,
        },
        plot(),
        ViewportLimits::default(),
    )
    .unwrap()
    .with_directions(ViewportDirections {
        x: true,
        ..Default::default()
    })
    .unwrap();
    let before = v.clone();
    assert!(
        v.apply(ViewportRequest::Zoom {
            factor: f64::NAN,
            anchor: (100., 50.),
            axes: ViewportSelection::X
        })
        .is_err()
    );
    assert_eq!(v, before);
    let domain = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (100., 50.),
            axes: ViewportSelection::X,
        })
        .unwrap()
        .x;
    assert!(domain.iter().all(|x| x.is_finite()) && domain[0] < domain[1]);
    ChartScale::with_options(
        domain,
        [0., 400.],
        &ContinuousScaleOptions {
            kind,
            ..Default::default()
        },
    )
    .unwrap();
}
#[test]
fn point_window_controller_keeps_reverse_indices_separate_from_value_zoom() {
    let data = [PointLineSeries::new(
        "s",
        (0..5)
            .map(|i| PointLineDatum::new(f64::from(i), f64::from(i)))
            .collect(),
    )];
    let o = PointLineOptions {
        point: PointScaleOptions {
            reverse: true,
            ..Default::default()
        },
        line: LineOptions {
            y_domain: Some([0., 10.]),
            ..Default::default()
        },
        ..Default::default()
    };
    let layout =
        point_line_layout(&ChartConfig::default(), &data, &o, &Default::default()).unwrap();
    let p = layout.plot;
    let mut v = layout
        .viewport_controller(ViewportLimits {
            max_zoom: 100.,
            ..Default::default()
        })
        .unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (p.width * 0.25, p.height * 0.75),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    interval(d.x, [1.5, 3.5]);
    interval(d.left.unwrap(), [1.25, 6.25]);
    let r = point_line_layout(
        &ChartConfig::default(),
        &data,
        &PointLineOptions {
            window: Some(d.x),
            line: LineOptions {
                y_domain: d.left,
                ..o.line.clone()
            },
            ..o
        },
        &Default::default(),
    )
    .unwrap();
    assert!(
        r.sites
            .iter()
            .all(|s| matches!(s.raw_x, CategoryKey::Number(2. | 3.)))
    );
    v.apply(ViewportRequest::Zoom {
        factor: 100.,
        anchor: (p.width * 0.5, p.height * 0.5),
        axes: ViewportSelection::Both,
    })
    .unwrap();
    near(v.domains().x[1] - v.domains().x[0], 1.);
    assert!(v.domains().left.unwrap()[1] - v.domains().left.unwrap()[0] < 1.);
}
