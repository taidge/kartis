use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::{surface::*, *};
use std::collections::{BTreeMap, BTreeSet};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Brush {
    Brush { x0, y0, x1, y1 }
}
fn plot() -> ClipRect {
    ClipRect {
        x: 50.,
        y: 40.,
        width: 300.,
        height: 200.,
    }
}
fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 400.,
        height: 300.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, x, y) in [("a", 100., 100.), ("b", 250., 150.)] {
        let mut mark = Mark::new(
            id,
            Geometry::Circle { x, y, radius: 7. },
            Color(20, 80, 180, 255),
        );
        mark.series = id.into();
        mark.label = id.into();
        scene.marks.push(mark);
    }
    scene
}
fn plain() -> SurfaceModel {
    SurfaceModel::new(
        scene(),
        SurfaceOptions {
            plot: Some(plot()),
            ..Default::default()
        },
    )
    .unwrap()
}
fn enable(model: &mut SurfaceModel, axis: BrushAxis) {
    model
        .configure_editable_brush(
            Some(EditableBrushOptions {
                axis,
                extent: rect(50., 40., 350., 240.),
                ..Default::default()
            }),
            BrushReconfigure::Preserve,
        )
        .unwrap();
}
fn begin(model: &mut SurfaceModel, point: (f64, f64), target: BrushTarget) -> BrushSession {
    model
        .apply_editable_brush(EditableBrushInput::Begin {
            id: 7,
            point,
            target,
            modifiers: Default::default(),
        })
        .unwrap();
    model.editable_brush().unwrap().session().unwrap()
}
fn event_ids(events: &[SurfaceEvent], phase: BrushPhase) -> Vec<String> {
    events
        .iter()
        .find_map(|event| match event {
            SurfaceEvent::EditableBrush { event, ids } if event.phase == phase => Some(ids.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn viewport_selection_projects_transformed_data_once_and_stays_in_screen_pixels() {
    let mut model = plain();
    model
        .apply(SurfaceInput::Wheel {
            point: (50., 40.),
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    assert_eq!(
        model.paint_transform().project((100., 100.)).unwrap(),
        (150., 160.)
    );
    enable(&mut model, BrushAxis::XY);
    let selection = rect(145., 155., 155., 165.);
    let events = model.set_brush_selection(Some(selection)).unwrap();
    assert_eq!(event_ids(&events, BrushPhase::Change), ["a"]);
    assert_eq!(model.editable_brush().unwrap().selection(), Some(selection));
    assert!(
        model.brush().is_none(),
        "legacy coordinates are not mixed into the editor"
    );
    assert_eq!(model.editable_brush_ids().unwrap(), ["a"]);
}

#[test]
fn typed_path_anchor_not_its_bounding_box_selects_current_delivered_frame() {
    let mut scene = scene();
    scene.marks.truncate(1);
    scene.marks[0].value = 42.;
    scene.marks[0].label = "raw a".into();
    scene.marks[0].geometry = Geometry::Path(vec![
        C::Move(P::new(80., 70.)),
        C::Line(P::new(140., 70.)),
        C::Line(P::new(80., 160.)),
        C::Close,
    ]);
    let binding = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "a".into(),
            series: "a".into(),
            label: "raw a".into(),
            value: Some(42.),
            formatted_value: Some("42 original".into()),
            color: Color(20, 80, 180, 255),
            position: [100., 100.],
        }],
        anchors: BTreeMap::from([(
            "a".into(),
            InteractionAnchor::path_points(&scene.marks[0].geometry, [100., 100.], &[0, 1, 2])
                .unwrap(),
        )]),
        ..Default::default()
    };
    let mut model = SurfaceModel::new_with_interaction(
        scene.clone(),
        SurfaceOptions {
            plot: Some(plot()),
            ..Default::default()
        },
        Some(binding),
    )
    .unwrap();
    enable(&mut model, BrushAxis::XY);
    model
        .set_brush_selection(Some(rect(99., 99., 101., 101.)))
        .unwrap();
    assert_eq!(model.editable_brush_ids().unwrap(), ["a"]);
    let Geometry::Path(commands) = &mut scene.marks[0].geometry else {
        unreachable!()
    };
    for command in commands {
        if let C::Move(p) | C::Line(p) = command {
            p.x += 20.;
        }
    }
    model.set_scene_preserving_interaction(scene).unwrap();
    assert!(model.editable_brush_ids().unwrap().is_empty());
    model
        .set_brush_selection(Some(rect(119., 99., 121., 101.)))
        .unwrap();
    assert_eq!(model.editable_brush_ids().unwrap(), ["a"]);
    model.set_hidden(BTreeSet::from(["a".into()])).unwrap();
    assert!(model.editable_brush_ids().unwrap().is_empty());
    model.set_hidden(BTreeSet::new()).unwrap();
    assert_eq!(model.editable_brush_ids().unwrap(), ["a"]);
}

#[test]
fn resize_cancels_old_session_and_clamps_pixels_without_reinterpreting_data() {
    let mut model = plain();
    enable(&mut model, BrushAxis::XY);
    model
        .set_brush_selection(Some(rect(280., 170., 330., 210.)))
        .unwrap();
    let session = begin(&mut model, (60., 60.), BrushTarget::Overlay);
    model
        .apply_editable_brush(EditableBrushInput::Move {
            session,
            id: 7,
            point: (160., 140.),
        })
        .unwrap();
    let mut smaller = scene();
    smaller.width = 320.;
    smaller.height = 220.;
    let events = model
        .set_layout(SceneInteraction {
            scene: smaller,
            plot: Some(ClipRect {
                x: 30.,
                y: 20.,
                width: 260.,
                height: 160.,
            }),
            interaction: None,
        })
        .unwrap();
    assert!(events.iter().any(|e| matches!(e, SurfaceEvent::EditableBrush { event, .. } if event.phase == BrushPhase::Cancel)));
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(rect(280., 170., 290., 180.))
    );
    assert_eq!(
        model.editable_brush().unwrap().options().extent,
        rect(50., 40., 290., 180.)
    );
    let accepted = model.editable_brush().unwrap().clone();
    assert!(
        model
            .apply_editable_brush(EditableBrushInput::Move {
                session,
                id: 7,
                point: (100., 100.)
            })
            .unwrap()
            .is_empty()
    );
    assert_eq!(model.editable_brush(), Some(&accepted));
}

#[test]
fn invalid_config_layout_and_selection_preserve_live_session_and_accepted_frame() {
    let mut model = plain();
    enable(&mut model, BrushAxis::XY);
    let session = begin(&mut model, (80., 80.), BrushTarget::Overlay);
    model
        .apply_editable_brush(EditableBrushInput::Move {
            session,
            id: 7,
            point: (140., 120.),
        })
        .unwrap();
    let accepted = model.editable_brush().unwrap().clone();
    let old_scene = model.scene().clone();
    let mut options = *accepted.options();
    options.extent.x1 = 360.;
    assert!(
        model
            .configure_editable_brush(Some(options), BrushReconfigure::Clamp)
            .is_err()
    );
    assert!(
        model
            .set_brush_selection(Some(rect(90., 90., f64::NAN, 120.)))
            .is_err()
    );
    let mut bad = scene();
    bad.width = f64::NAN;
    assert!(model.set_scene(bad).is_err());
    assert_eq!(model.scene(), &old_scene);
    assert_eq!(model.editable_brush(), Some(&accepted));
    model
        .apply_editable_brush(EditableBrushInput::End {
            session,
            id: 7,
            point: (160., 140.),
        })
        .unwrap();
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(rect(80., 80., 160., 140.))
    );
}

#[test]
fn active_editor_arbitrates_pan_pinch_wheel_and_program_update_retires_queued_up() {
    let mut model = plain();
    enable(&mut model, BrushAxis::XY);
    let session = begin(&mut model, (80., 80.), BrushTarget::Overlay);
    let view = model.view();
    for input in [
        SurfaceInput::Down {
            id: 8,
            point: (100., 100.),
            brush: false,
        },
        SurfaceInput::Move {
            id: 8,
            point: (130., 130.),
        },
        SurfaceInput::TouchStart {
            id: 9,
            point: (120., 120.),
        },
        SurfaceInput::Wheel {
            point: (120., 120.),
            delta: -300.,
        },
    ] {
        assert!(model.apply(input).unwrap().is_empty());
    }
    assert_eq!(model.view(), view);
    assert_eq!(model.touch_count(), 0);
    assert!(model.dragging());
    let replacement = rect(90., 90., 110., 110.);
    model.set_brush_selection(Some(replacement)).unwrap();
    assert!(!model.dragging());
    assert!(
        model
            .apply_editable_brush(EditableBrushInput::End {
                session,
                id: 7,
                point: (300., 200.)
            })
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(replacement)
    );
}

#[test]
fn relayout_retains_same_pixel_gesture_but_scene_replacement_and_reset_cancel_it() {
    let mut model = plain();
    model
        .relayout(scene(), plot(), ZoomTransform::default(), [1., 20.], false)
        .unwrap();
    enable(&mut model, BrushAxis::X);
    let session = begin(&mut model, (80., 100.), BrushTarget::Overlay);
    model
        .relayout(
            scene(),
            plot(),
            ZoomTransform::new(2., -50., -20.).unwrap(),
            [1., 20.],
            true,
        )
        .unwrap();
    assert_eq!(model.editable_brush().unwrap().session(), Some(session));
    model
        .apply_editable_brush(EditableBrushInput::Move {
            session,
            id: 7,
            point: (160., 140.),
        })
        .unwrap();
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(rect(80., 40., 160., 240.))
    );
    assert_eq!(model.paint_transform(), ZoomTransform::default());
    model.apply(SurfaceInput::Reset).unwrap();
    assert!(!model.editable_brush().unwrap().active());
    assert!(model.editable_brush().unwrap().selection().is_none());
    let next_session = begin(&mut model, (80., 100.), BrushTarget::Overlay);
    assert_ne!(next_session, session);
    model.set_scene(scene()).unwrap();
    assert!(!model.editable_brush().unwrap().active());
}

#[test]
fn legacy_shift_brush_and_noop_editor_disable_keep_existing_input_contract() {
    let mut model = plain();
    model
        .apply(SurfaceInput::Down {
            id: 1,
            point: (80., 80.),
            brush: true,
        })
        .unwrap();
    assert!(
        model
            .configure_editable_brush(None, BrushReconfigure::Clear)
            .unwrap()
            .is_empty()
    );
    assert!(model.dragging());
    let events = model
        .apply(SurfaceInput::Up {
            id: 1,
            point: (120., 120.),
        })
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SurfaceEvent::Brush { complete: true, ids, .. } if ids == &["a"]))
    );
    assert!(model.editable_brush().is_none());
}

#[test]
fn recreated_editor_never_accepts_an_old_session_with_the_same_pointer_id() {
    let mut model = plain();
    enable(&mut model, BrushAxis::XY);
    let old = begin(&mut model, (80., 80.), BrushTarget::Overlay);
    model
        .configure_editable_brush(None, BrushReconfigure::Clear)
        .unwrap();
    enable(&mut model, BrushAxis::XY);
    let current = begin(&mut model, (90., 90.), BrushTarget::Overlay);
    assert_ne!(old, current);
    let accepted = model.editable_brush().unwrap().clone();
    assert!(
        model
            .apply_editable_brush(EditableBrushInput::End {
                session: old,
                id: 7,
                point: (300., 200.),
            })
            .unwrap()
            .is_empty()
    );
    assert_eq!(model.editable_brush(), Some(&accepted));
    model
        .apply_editable_brush(EditableBrushInput::End {
            session: current,
            id: 7,
            point: (130., 140.),
        })
        .unwrap();
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(rect(90., 90., 130., 140.))
    );
}

#[test]
fn keyboard_nudge_retires_legacy_pointer_without_a_late_click_or_pan() {
    let mut model = plain();
    enable(&mut model, BrushAxis::XY);
    model
        .set_brush_selection(Some(rect(80., 80., 140., 130.)))
        .unwrap();
    model
        .apply(SurfaceInput::Down {
            id: 9,
            point: (100., 100.),
            brush: false,
        })
        .unwrap();
    assert!(model.dragging());
    model
        .apply_editable_brush(EditableBrushInput::Nudge {
            target: BrushTarget::Selection,
            delta: (10., 0.),
        })
        .unwrap();
    assert!(!model.dragging());
    let view = model.view();
    assert!(
        model
            .apply(SurfaceInput::Up {
                id: 9,
                point: (160., 160.)
            })
            .unwrap()
            .is_empty()
    );
    assert_eq!(model.view(), view);
    assert!(model.selected().is_none());
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(rect(90., 80., 150., 130.))
    );
}

#[test]
fn viewport_request_subtracts_margin_once_and_respects_active_and_reversed_axes() {
    for axis in [BrushAxis::XY, BrushAxis::X, BrushAxis::Y] {
        let mut model = plain();
        model
            .apply(SurfaceInput::Wheel {
                point: (50., 40.),
                delta: -2_f64.ln() / 0.002,
            })
            .unwrap();
        enable(&mut model, axis);
        assert!(model.editable_brush_viewport_request().unwrap().is_none());
        let session = begin(&mut model, (110., 80.), BrushTarget::Overlay);
        assert!(model.editable_brush_viewport_request().unwrap().is_none());
        model
            .apply_editable_brush(EditableBrushInput::End {
                session,
                id: 7,
                point: (230., 180.),
            })
            .unwrap();
        let request = model.editable_brush_viewport_request().unwrap().unwrap();
        let ViewportRequest::Brush { bounds, axes } = request else {
            unreachable!()
        };
        assert_eq!(
            axes,
            match axis {
                BrushAxis::X => ViewportSelection::X,
                BrushAxis::Y => ViewportSelection::Y,
                BrushAxis::XY => ViewportSelection::Both,
            }
        );
        assert_eq!(
            bounds,
            match axis {
                BrushAxis::X => rect(60., 0., 180., 200.),
                BrushAxis::Y => rect(0., 40., 300., 140.),
                BrushAxis::XY => rect(60., 40., 180., 140.),
            }
        );
        let mut viewport = DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
                left: Some(ViewportAxis::new([0., 200.], ScaleKind::Linear)),
                right: Some(ViewportAxis::new([1., 10_000.], ScaleKind::Log)),
            },
            plot(),
            ViewportLimits::default(),
        )
        .unwrap()
        .with_directions(ViewportDirections {
            x: true,
            left: false,
            right: true,
        })
        .unwrap();
        let domains = viewport.apply(request).unwrap();
        let expected_x = if axis == BrushAxis::Y {
            [0., 100.]
        } else {
            [40., 80.]
        };
        let expected_left = if axis == BrushAxis::X {
            [0., 200.]
        } else {
            [60., 160.]
        };
        let expected_right = if axis == BrushAxis::X {
            [1., 10_000.]
        } else {
            [10_f64.powf(0.8), 10_f64.powf(2.8)]
        };
        for (actual, expected) in [
            (domains.x, expected_x),
            (domains.left.unwrap(), expected_left),
            (domains.right.unwrap(), expected_right),
        ] {
            for i in 0..2 {
                assert!(
                    (actual[i] - expected[i]).abs() <= 1e-12 * expected[i].abs().max(1.),
                    "{axis:?}: {actual:?} != {expected:?}"
                );
            }
        }
    }
}

#[test]
fn series_visibility_change_cancels_before_pointer_up_but_unchanged_visibility_does_not() {
    let mut model = plain();
    enable(&mut model, BrushAxis::XY);
    let origin = rect(80., 80., 140., 130.);
    model.set_brush_selection(Some(origin)).unwrap();
    let session = begin(&mut model, (160., 120.), BrushTarget::Overlay);
    model
        .apply_editable_brush(EditableBrushInput::Move {
            session,
            id: 7,
            point: (300., 200.),
        })
        .unwrap();
    let hidden = BTreeSet::from(["a".into()]);
    let events = model.set_hidden(hidden.clone()).unwrap();
    assert!(!model.editable_brush().unwrap().active());
    assert!(!model.dragging());
    assert_eq!(model.editable_brush().unwrap().selection(), Some(origin));
    assert!(event_ids(&events, BrushPhase::Cancel).is_empty());
    assert!(
        model
            .apply_editable_brush(EditableBrushInput::End {
                session,
                id: 7,
                point: (330., 220.),
            })
            .unwrap()
            .is_empty()
    );
    assert_eq!(model.editable_brush().unwrap().selection(), Some(origin));

    let current = begin(&mut model, (60., 60.), BrushTarget::Overlay);
    assert!(model.set_hidden(hidden).unwrap().is_empty());
    assert_eq!(model.editable_brush().unwrap().session(), Some(current));
    model
        .apply_editable_brush(EditableBrushInput::End {
            session: current,
            id: 7,
            point: (120., 100.),
        })
        .unwrap();
    assert_eq!(
        model.editable_brush().unwrap().selection(),
        Some(rect(60., 60., 120., 100.))
    );
}

#[test]
fn idle_programmatic_echo_does_not_interrupt_an_unrelated_legacy_pan_or_pinch() {
    let mut model = plain();
    model
        .apply(SurfaceInput::Wheel {
            point: (200., 140.),
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    let options = EditableBrushOptions {
        extent: rect(50., 40., 220., 200.),
        ..Default::default()
    };
    model
        .configure_editable_brush(Some(options), BrushReconfigure::Preserve)
        .unwrap();
    let selection = rect(80., 80., 140., 130.);
    model.set_brush_selection(Some(selection)).unwrap();
    model
        .apply(SurfaceInput::Down {
            id: 3,
            point: (280., 180.),
            brush: false,
        })
        .unwrap();
    let before = model.view();
    assert!(
        model
            .set_brush_selection(Some(selection))
            .unwrap()
            .is_empty()
    );
    assert!(model.dragging());
    model
        .apply(SurfaceInput::Move {
            id: 3,
            point: (300., 180.),
        })
        .unwrap();
    assert_eq!(model.view().translate_x, before.translate_x + 20.);
    model
        .apply(SurfaceInput::Up {
            id: 3,
            point: (300., 180.),
        })
        .unwrap();
    for (id, x) in [(4, 260.), (5, 320.)] {
        model
            .apply(SurfaceInput::TouchStart {
                id,
                point: (x, 150.),
            })
            .unwrap();
    }
    assert!(model.pinching());
    assert!(
        model
            .configure_editable_brush(Some(options), BrushReconfigure::Preserve)
            .unwrap()
            .is_empty()
    );
    assert_eq!(model.touch_count(), 2);
    assert!(model.pinching());
    model
        .set_brush_selection(Some(rect(90., 80., 150., 130.)))
        .unwrap();
    assert_eq!(model.touch_count(), 0);
    assert!(!model.pinching());
}

#[test]
fn outside_editor_extent_shift_uses_ordinary_pan_instead_of_an_invisible_legacy_brush() {
    let mut model = plain();
    model
        .apply(SurfaceInput::Wheel {
            point: (200., 140.),
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    model
        .configure_editable_brush(
            Some(EditableBrushOptions {
                extent: rect(100., 80., 220., 200.),
                ..Default::default()
            }),
            BrushReconfigure::Preserve,
        )
        .unwrap();
    let selection = rect(120., 100., 180., 160.);
    model.set_brush_selection(Some(selection)).unwrap();
    let before = model.view();
    model
        .apply(SurfaceInput::Down {
            id: 4,
            point: (80., 60.),
            brush: true,
        })
        .unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 4,
            point: (90., 65.),
        })
        .unwrap();
    let events = model
        .apply(SurfaceInput::Up {
            id: 4,
            point: (90., 65.),
        })
        .unwrap();
    assert_eq!(model.view().translate_x, before.translate_x + 10.);
    assert_eq!(model.view().translate_y, before.translate_y + 5.);
    assert!(model.brush().is_none());
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Brush { .. }))
    );
    assert_eq!(model.editable_brush().unwrap().selection(), Some(selection));
    model
        .configure_editable_brush(None, BrushReconfigure::Clear)
        .unwrap();
    model
        .apply(SurfaceInput::Down {
            id: 5,
            point: (80., 60.),
            brush: true,
        })
        .unwrap();
    assert!(
        model
            .apply(SurfaceInput::Up {
                id: 5,
                point: (90., 65.)
            })
            .unwrap()
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Brush { complete: true, .. }))
    );
}
