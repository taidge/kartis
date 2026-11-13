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
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn interval(actual: [f64; 2], expected: [f64; 2]) {
    near(actual[0], expected[0]);
    near(actual[1], expected[1]);
}

#[test]
fn ordinal_y_cap_keeps_x_independent_and_pan_live_at_both_caps() {
    let mut viewport = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([0., 4.], ScaleKind::Linear)),
            right: Some(ViewportAxis::new([0., 40.], ScaleKind::Linear)),
        },
        plot(),
        ViewportLimits::default(),
    )
    .unwrap()
    .with_directions(ViewportDirections {
        x: false,
        left: true,
        right: false,
    })
    .unwrap()
    .with_x_max_zoom(10.)
    .unwrap()
    .with_y_max_zoom(4.)
    .unwrap();
    let domains = viewport
        .apply(ViewportRequest::Zoom {
            factor: 100.,
            anchor: (100., 150.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert_eq!(viewport.zoom(), (10., 4.));
    interval(domains.x, [22.5, 32.5]);
    interval(domains.left.unwrap(), [2.25, 3.25]);
    interval(domains.right.unwrap(), [7.5, 17.5]);
    let panned = viewport
        .apply_gesture(
            ViewportGesture {
                factor: 2.,
                anchor: (100., 150.),
                pan: (40., 20.),
            },
            ViewportSelection::Both,
        )
        .unwrap();
    assert_eq!(viewport.zoom(), (10., 4.));
    interval(panned.x, [21.5, 31.5]);
    interval(panned.left.unwrap(), [2.15, 3.15]);
    interval(panned.right.unwrap(), [8.5, 18.5]);
    interval(viewport.reset().left.unwrap(), [0., 4.]);
}

#[test]
fn singleton_y_cap_preserves_reset_and_does_not_apply_y_limit_to_numeric_x() {
    let mut viewport = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([-0.5, 0.5], ScaleKind::Linear)),
            right: None,
        },
        plot(),
        ViewportLimits {
            min_zoom: 5.,
            max_zoom: 100.,
            bounded: true,
        },
    )
    .unwrap()
    .with_y_max_zoom(1.)
    .unwrap();
    assert_eq!(viewport.zoom(), (1., 1.));
    let domains = viewport
        .apply(ViewportRequest::Zoom {
            factor: 20.,
            anchor: (100., 150.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert_eq!(viewport.zoom(), (20., 1.));
    interval(domains.x, [23.75, 28.75]);
    interval(domains.left.unwrap(), [-0.5, 0.5]);
    interval(viewport.reset().x, [0., 100.]);
    assert_eq!(viewport.zoom(), (1., 1.));
}

#[test]
fn installing_a_lower_y_cap_keeps_y_center_and_the_entire_x_window() {
    let mut viewport = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([100., 180.], ScaleKind::Linear)),
            right: None,
        },
        plot(),
        ViewportLimits::default(),
    )
    .unwrap();
    let before = viewport
        .apply(ViewportRequest::Zoom {
            factor: 8.,
            anchor: (100., 50.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    let capped = viewport.with_y_max_zoom(2.).unwrap();
    assert_eq!(capped.zoom(), (8., 2.));
    assert_eq!(capped.domains().x, before.x);
    interval(capped.domains().left.unwrap(), [137.5, 177.5]);
    assert!(capped.clone().with_y_max_zoom(f64::NAN).is_err());
    assert!(capped.with_y_max_zoom(0.5).is_err());
    let x_only = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 10.], ScaleKind::Linear),
            left: None,
            right: None,
        },
        plot(),
        ViewportLimits::default(),
    )
    .unwrap();
    assert!(x_only.with_y_max_zoom(2.).is_err());
}
