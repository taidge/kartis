use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn scene(x: f64) -> Scene {
    let mut scene = ChartConfig {
        width: 200.,
        height: 200.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "datum",
        Geometry::Circle {
            x,
            y: 100.,
            radius: 5.,
        },
        Color(30, 90, 150, 255),
    );
    mark.series = "series".into();
    mark.label = "Datum".into();
    scene.marks.push(mark);
    scene
}
fn plot() -> ClipRect {
    ClipRect {
        x: 0.,
        y: 0.,
        width: 200.,
        height: 200.,
    }
}
fn model(picking: Picking) -> SurfaceModel {
    let mut model = SurfaceModel::new(
        scene(100.),
        SurfaceOptions {
            picking,
            ..Default::default()
        },
    )
    .unwrap();
    model
        .relayout(
            scene(100.),
            plot(),
            ZoomTransform::default(),
            [1., 20.],
            false,
        )
        .unwrap();
    model
}

#[test]
fn a_relayout_must_not_retain_hover_for_a_constant_radius_mark_that_moved_away() {
    let mut model = model(Picking::Geometry);
    let pointer = (104., 100.);
    model
        .apply(SurfaceInput::Wheel {
            point: pointer,
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    assert_eq!(
        model.hovered(),
        ["datum"],
        "the previous pixels contain the pointer"
    );
    let view = model.view();
    assert_eq!(view.scale, 2.);
    // Fixed-axis relayout moves the datum center from 100 to 96 while keeping
    // its 5px radius: the wheel anchor 104 is now 8px away, outside the paint.
    model
        .relayout(scene(96.), plot(), view, [1., 20.], true)
        .unwrap();
    let retained = model.hovered().to_vec();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: pointer,
        })
        .unwrap();
    assert!(
        model.hovered().is_empty(),
        "fresh picking observes the accepted pixels"
    );
    assert_eq!(
        retained,
        model.hovered(),
        "relayout must not expose a stale hover until another pointer move"
    );
}

#[test]
fn relayout_picking_uses_new_pixels_without_inverse_or_scaled_pick_distances() {
    for picking in [
        Picking::Geometry,
        Picking::Nearest { distance: 5. },
        Picking::SliceX { distance: 5. },
        Picking::SliceY { distance: 5. },
    ] {
        let mut model = model(picking);
        let view = ZoomTransform::new(4., -150., -150.).unwrap();
        model
            .relayout(scene(50.), plot(), view, [1., 20.], false)
            .unwrap();
        assert_eq!(model.paint_transform(), ZoomTransform::default());
        model
            .apply(SurfaceInput::Move {
                id: 7,
                point: (54., 100.),
            })
            .unwrap();
        assert_eq!(
            model.hovered(),
            ["datum"],
            "{picking:?}: radius/distance remain logical pixels"
        );
    }
}

#[test]
fn accepted_relayouts_preserve_continuous_pan_and_pinch_baselines() {
    let mut panned = model(Picking::Geometry);
    panned
        .relayout(
            scene(100.),
            plot(),
            ZoomTransform::new(2., -100., -100.).unwrap(),
            [1., 20.],
            false,
        )
        .unwrap();
    panned
        .apply(SurfaceInput::Down {
            id: 1,
            point: (100., 100.),
            brush: false,
        })
        .unwrap();
    panned
        .apply(SurfaceInput::Move {
            id: 1,
            point: (110., 100.),
        })
        .unwrap();
    let once = panned.view();
    assert_eq!(once.translate_x, -90.);
    panned
        .relayout(scene(110.), plot(), once, [1., 20.], true)
        .unwrap();
    panned
        .apply(SurfaceInput::Move {
            id: 1,
            point: (120., 100.),
        })
        .unwrap();
    assert_eq!(
        panned.view().translate_x,
        -80.,
        "the next move is a 10px delta, not the full drag"
    );
    let mut pinched = model(Picking::Geometry);
    pinched
        .apply(SurfaceInput::TouchStart {
            id: 10,
            point: (75., 100.),
        })
        .unwrap();
    pinched
        .apply(SurfaceInput::TouchStart {
            id: 11,
            point: (125., 100.),
        })
        .unwrap();
    pinched
        .apply(SurfaceInput::TouchMove {
            id: 11,
            point: (175., 100.),
        })
        .unwrap();
    let once = pinched.view();
    assert_eq!(once.scale, 2.);
    pinched
        .relayout(scene(100.), plot(), once, [1., 20.], true)
        .unwrap();
    pinched
        .apply(SurfaceInput::TouchMove {
            id: 11,
            point: (175., 100.),
        })
        .unwrap();
    assert_eq!(
        pinched.view(),
        once,
        "same contact geometry must not compound after relayout"
    );
    assert!(pinched.pinching());
    assert_eq!(pinched.touch_count(), 2);
}

#[test]
fn rejected_relayout_keeps_scene_view_drag_and_pinch_state() {
    let mut model = model(Picking::Geometry);
    model
        .apply(SurfaceInput::TouchStart {
            id: 10,
            point: (75., 100.),
        })
        .unwrap();
    model
        .apply(SurfaceInput::TouchStart {
            id: 11,
            point: (125., 100.),
        })
        .unwrap();
    let old_scene = model.scene().clone();
    let old_view = model.view();
    let mut invalid = scene(96.);
    invalid.marks[0].opacity = f64::NAN;
    assert!(
        model
            .relayout(
                invalid,
                plot(),
                ZoomTransform::new(2., -100., -100.).unwrap(),
                [1., 20.],
                true
            )
            .is_err()
    );
    assert_eq!(model.scene(), &old_scene);
    assert_eq!(model.view(), old_view);
    assert_eq!(model.touch_count(), 2);
    assert!(model.pinching());
    model
        .apply(SurfaceInput::TouchMove {
            id: 11,
            point: (175., 100.),
        })
        .unwrap();
    assert_eq!(model.view().scale, 2.);
}
