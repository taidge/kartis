use kartis::surface::{SurfaceInput, SurfaceModel, SurfaceOptions, SurfaceViewMode};
use kartis::*;
fn plot() -> ClipRect {
    ClipRect {
        x: 20.,
        y: 20.,
        width: 400.,
        height: 200.,
    }
}
fn scene() -> Scene {
    ChartConfig {
        width: 440.,
        height: 240.,
        ..Default::default()
    }
    .scene()
    .unwrap()
}
fn controller() -> DomainViewport {
    DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
            right: None,
        },
        plot(),
        ViewportLimits {
            max_zoom: 4.,
            ..Default::default()
        },
    )
    .unwrap()
}
fn range(a: [f64; 2], b: [f64; 2]) {
    for (a, b) in a.into_iter().zip(b) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
}
#[test]
fn virtual_relayout_extent_is_admissible_while_geometry_mode_keeps_original_limits() {
    let mut surface = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    surface
        .relayout(
            scene(),
            plot(),
            ZoomTransform::default(),
            [0.000001, 1_000_000.],
            false,
        )
        .unwrap();
    assert_eq!(surface.options().view_mode, SurfaceViewMode::Relayout);
    surface
        .apply(SurfaceInput::Wheel {
            point: (220., 120.),
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    assert_eq!(surface.view().scale, 2.);
    assert!(
        SurfaceModel::new(
            scene(),
            SurfaceOptions {
                min_scale: 0.000001,
                max_scale: 1_000_000.,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn one_category_limits_x_without_freezing_independent_value_navigation() {
    let mut v = CategoryViewport::with_value_axes(
        vec!["Only".into()],
        plot(),
        ViewportLimits::default(),
        Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
        None,
    )
    .unwrap();
    v.apply(ViewportRequest::Zoom {
        factor: 2.,
        anchor: (200., 100.),
        axes: ViewportSelection::Y,
    })
    .unwrap();
    assert_eq!(v.controller().domains().x, [0., 1.]);
    range(v.controller().domains().left.unwrap(), [25., 75.]);
    v.apply(ViewportRequest::Zoom {
        factor: 100.,
        anchor: (200., 100.),
        axes: ViewportSelection::Both,
    })
    .unwrap();
    assert_eq!(v.controller().zoom(), (1., 100.));
    assert_eq!(v.window().unwrap().categories, ["Only"]);
}

#[test]
fn installing_a_category_x_cap_does_not_apply_the_gesture_minimum_during_construction() {
    let categories = (0..8).map(|i| format!("Category {i}")).collect::<Vec<_>>();
    let v = CategoryViewport::with_value_axes(
        categories.clone(),
        plot(),
        ViewportLimits {
            min_zoom: 2.,
            max_zoom: 20.,
            bounded: true,
        },
        Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
        None,
    )
    .unwrap();
    assert_eq!(v.controller().zoom(), (1., 1.));
    assert_eq!(v.window().unwrap().categories, categories);
    assert_eq!(v.controller().domains().x, [0., 8.]);
}
#[test]
fn independent_axis_saturation_preserves_the_wheel_anchor_and_other_axis_progress() {
    let mut v = controller();
    v.apply(ViewportRequest::TransformXY {
        transform: ViewportTransform {
            scale_x: 4.,
            scale_y: 2.,
            translate_x: -400.,
            translate_y: -100.,
        },
        axes: ViewportSelection::Both,
    })
    .unwrap();
    let old_x = v.domains().x;
    let result = v
        .apply_gesture(
            ViewportGesture {
                factor: 2.,
                anchor: (0., 50.),
                pan: (0., 0.),
            },
            ViewportSelection::Both,
        )
        .unwrap();
    assert_eq!(
        result.x, old_x,
        "a saturated axis must not drift towards the proposed window center"
    );
    range(result.left.unwrap(), [43.75, 68.75]);
    assert_eq!(v.zoom(), (4., 4.));
    let panned = v
        .apply_gesture(
            ViewportGesture {
                factor: 2.,
                anchor: (0., 50.),
                pan: (20., -10.),
            },
            ViewportSelection::Both,
        )
        .unwrap();
    range(panned.x, [23.75, 48.75]);
    range(panned.left.unwrap(), [42.5, 67.5]);
}
#[test]
fn real_surface_pinch_decomposition_keeps_centroid_pan_at_zoom_limit_then_rebases() {
    let mut v = controller();
    v.apply(ViewportRequest::TransformXY {
        transform: ViewportTransform {
            scale_x: 4.,
            scale_y: 2.,
            translate_x: -400.,
            translate_y: -100.,
        },
        axes: ViewportSelection::Both,
    })
    .unwrap();
    let mut surface = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    surface
        .relayout(
            scene(),
            plot(),
            ZoomTransform::new(2., -200., -100.).unwrap(),
            [0.000001, 1_000_000.],
            false,
        )
        .unwrap();
    surface
        .apply(SurfaceInput::TouchStart {
            id: 1,
            point: (120., 100.),
        })
        .unwrap();
    surface
        .apply(SurfaceInput::TouchStart {
            id: 2,
            point: (220., 100.),
        })
        .unwrap();
    let previous = surface.clone();
    surface
        .apply(SurfaceInput::TouchMove {
            id: 2,
            point: (320., 120.),
        })
        .unwrap();
    let gesture = surface.relayout_gesture(&previous).unwrap();
    range([gesture.anchor.0, gesture.anchor.1], [150., 80.]);
    range([gesture.pan.0, gesture.pan.1], [50., 10.]);
    let d = v.apply_gesture(gesture, ViewportSelection::Both).unwrap();
    range(d.x, [21.875, 46.875]);
    range(d.left.unwrap(), [41.25, 66.25]);
    let accepted = v.transform_for(ViewportSelection::Both).unwrap();
    surface
        .relayout(scene(), plot(), accepted, [0.000001, 1_000_000.], true)
        .unwrap();
    let previous = surface.clone();
    surface
        .apply(SurfaceInput::TouchMove {
            id: 2,
            point: (320., 120.),
        })
        .unwrap();
    let unchanged = v
        .apply_gesture(
            surface.relayout_gesture(&previous).unwrap(),
            ViewportSelection::Both,
        )
        .unwrap();
    assert_eq!(d, unchanged);
}
#[test]
fn actual_wheel_decomposition_matches_anchored_data_zoom_without_double_mapping() {
    let mut v = controller();
    let mut surface = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    surface
        .relayout(
            scene(),
            plot(),
            ZoomTransform::default(),
            [0.000001, 1_000_000.],
            false,
        )
        .unwrap();
    let previous = surface.clone();
    surface
        .apply(SurfaceInput::Wheel {
            point: (120., 70.),
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    let gesture = surface.relayout_gesture(&previous).unwrap();
    range([gesture.anchor.0, gesture.anchor.1], [100., 50.]);
    range([gesture.pan.0, gesture.pan.1], [0., 0.]);
    let domains = v.apply_gesture(gesture, ViewportSelection::Both).unwrap();
    range(domains.x, [12.5, 62.5]);
    range(domains.left.unwrap(), [37.5, 87.5]);
    // Responsive plot changes happen after interpreting old-frame input. The
    // point's 25% fractional data anchor must not become 12.5% after doubling.
    v.resize(ClipRect {
        x: 30.,
        y: 30.,
        width: 800.,
        height: 400.,
    })
    .unwrap();
    assert_eq!(v.domains(), domains);
}
#[test]
fn gesture_overflow_and_x_cap_validation_are_atomic_with_original_domains_retained() {
    let mut v = controller();
    let old = v.clone();
    for gesture in [
        ViewportGesture {
            factor: 0.,
            anchor: (0., 0.),
            pan: (0., 0.),
        },
        ViewportGesture {
            factor: 2.,
            anchor: (f64::NAN, 0.),
            pan: (0., 0.),
        },
        ViewportGesture {
            factor: 2.,
            anchor: (0., 0.),
            pan: (0., f64::INFINITY),
        },
    ] {
        assert!(v.apply_gesture(gesture, ViewportSelection::Both).is_err());
        assert_eq!(v, old);
    }
    assert!(v.clone().with_x_max_zoom(0.).is_err());
    assert_eq!(v, old);
    v = v.with_x_max_zoom(2.).unwrap();
    v.apply_gesture(
        ViewportGesture {
            factor: f64::MAX,
            anchor: (200., 100.),
            pan: (0., 0.),
        },
        ViewportSelection::Both,
    )
    .unwrap();
    assert_eq!(v.zoom(), (2., 4.));
    assert_eq!(v.reset(), old.domains());
}
