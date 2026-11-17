//! Public gesture contracts surrounding live Tree activity publication.
//! These are model contracts, not evidence of native browser capture delivery.
use kartis::surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn tree(focus: Option<&str>) -> (SceneInteraction, (f64, f64), String) {
    let source = [
        HierarchyDatum::new("leaf", Some("root".into()), 4.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("other", Some("root".into()), 7.),
    ];
    let mut active = |_: &TreeNodeModifierContext<'_>| Ok(40.);
    let layout = tree_layout_with_activity(
        &ChartConfig {
            width: 460.,
            height: 320.,
            margin: Margin {
                left: 37.,
                right: 31.,
                top: 29.,
                bottom: 33.,
            },
            ..Default::default()
        },
        &source,
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions {
            focus_id: focus.map(str::to_owned),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active),
            ..Default::default()
        },
    )
    .unwrap();
    let node = &layout.layout.nodes[0];
    let point = (node.position[0], node.position[1]);
    let id = node.mark_id.clone().unwrap();
    (tree_activity_frame(&layout).unwrap(), point, id)
}

fn model() -> (SurfaceModel, (f64, f64), String) {
    let (frame, point, id) = tree(None);
    let model = SurfaceModel::new_with_interaction(
        frame.scene,
        SurfaceOptions {
            plot: frame.plot,
            pan: false,
            brush: false,
            zoom: false,
            pinch: false,
            ..Default::default()
        },
        frame.interaction,
    )
    .unwrap();
    (model, point, id)
}

fn down(model: &mut SurfaceModel, point: (f64, f64)) {
    model
        .apply(SurfaceInput::Down {
            id: 7,
            point,
            brush: false,
        })
        .unwrap();
}

fn up(model: &mut SurfaceModel, point: (f64, f64)) -> Vec<SurfaceEvent> {
    model.apply(SurfaceInput::Up { id: 7, point }).unwrap()
}

#[test]
fn click_is_an_active_gesture_from_down_and_leave_does_not_cancel_capture_contract() {
    let (mut model, point, id) = model();
    down(&mut model, point);
    assert!(
        model.dragging(),
        "a press counts even with pan/brush disabled"
    );
    model.apply(SurfaceInput::Leave).unwrap();
    assert!(model.dragging(), "a captured press may leave the surface");
    model.apply(SurfaceInput::Cancel { id: 8 }).unwrap();
    assert!(
        model.dragging(),
        "a foreign pointer cannot release this gesture"
    );
    let events = up(&mut model, point);
    assert!(!model.dragging());
    assert_eq!(model.selected(), Some(id.as_str()));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, SurfaceEvent::Activated(value) if value == &id))
            .count(),
        1
    );
    assert!(
        up(&mut model, point).is_empty(),
        "a duplicate Up cannot reactivate"
    );
}

#[test]
fn successful_explicit_layout_even_identical_retires_press_but_rejected_layout_keeps_it() {
    for focused in [false, true] {
        let (mut model, point, _) = model();
        down(&mut model, point);
        let frame = tree(focused.then_some("leaf")).0;
        model.set_layout(frame).unwrap();
        assert!(
            !model.dragging(),
            "explicit source/layout acceptance cancels the old press"
        );
        assert!(up(&mut model, point).is_empty());
        assert_eq!(model.selected(), None);
    }
    let (mut model, point, id) = model();
    down(&mut model, point);
    let token = model.frame_token();
    let before = model.scene().clone();
    let mut invalid = tree(Some("leaf")).0;
    invalid.scene.width = f64::NAN;
    assert!(model.set_layout(invalid).is_err());
    assert_eq!(model.scene(), &before);
    assert_eq!(model.frame_token(), token);
    assert!(
        model.dragging(),
        "failed candidate does not mutate accepted gesture"
    );
    assert!(
        up(&mut model, point)
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Activated(value) if value == &id))
    );
}

#[test]
fn release_movement_threshold_and_cancel_retire_gestures_without_stale_activation() {
    for distance in [3., 3.01] {
        let (mut model, point, _) = model();
        down(&mut model, point);
        let released = (point.0, point.1 + distance);
        let events = up(&mut model, released);
        assert!(!model.dragging());
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, SurfaceEvent::Activated(_))),
            distance == 3.,
            "the click threshold uses logical scene pixels, not elapsed RAFs"
        );
    }
    let (mut model, point, id) = model();
    down(&mut model, point);
    model.apply(SurfaceInput::Cancel { id: 7 }).unwrap();
    assert!(!model.dragging());
    assert!(up(&mut model, point).is_empty());
    assert_eq!(model.selected(), None);
    down(&mut model, point);
    assert!(
        up(&mut model, point)
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Activated(value) if value == &id)),
        "a cancelled input must not poison the next click"
    );
}

#[test]
fn touch_contacts_and_full_cancel_are_included_in_the_public_dragging_barrier() {
    let (mut model, point, id) = model();
    model
        .apply(SurfaceInput::TouchStart { id: 12, point })
        .unwrap();
    assert!(model.dragging());
    assert_eq!(model.touch_count(), 1);
    model.cancel_drag();
    assert!(!model.dragging());
    assert_eq!(model.touch_count(), 0);
    assert!(
        model
            .apply(SurfaceInput::TouchEnd { id: 12, point })
            .unwrap()
            .is_empty()
    );
    assert_eq!(model.selected(), None);
    down(&mut model, point);
    assert!(
        up(&mut model, point)
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Activated(value) if value == &id))
    );
}
