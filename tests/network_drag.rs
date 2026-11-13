use kartis::{network::*, *};
use std::collections::BTreeSet;
fn simulation() -> ForceSimulation {
    let mut s = ForceSimulation::new(
        &[
            NetworkNode {
                id: "a".into(),
                label: "A".into(),
                radius: 12.,
            },
            NetworkNode {
                id: "b".into(),
                label: "B".into(),
                radius: 12.,
            },
        ],
        &[],
        ForceOptions {
            repulsion: 0.,
            link_strength: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            alpha: 0.,
            alpha_decay: 0.2,
            ..Default::default()
        },
    )
    .unwrap();
    s.set_position("a", [20., -10.]).unwrap();
    s.set_position("b", [-20., 10.]).unwrap();
    s
}
fn mapping() -> NetworkDragMapping {
    NetworkDragMapping::from_config(
        &ChartConfig {
            width: 500.,
            height: 300.,
            margin: Margin {
                left: 60.,
                right: 20.,
                top: 20.,
                bottom: 40.,
            },
            ..Default::default()
        },
        ClipRect {
            x: 100.,
            y: 200.,
            width: 250.,
            height: 450.,
        },
        ZoomTransform::new(2., -130., 15.).unwrap(),
    )
    .unwrap()
}
#[test]
fn dragging_preserves_grab_offset_through_css_scene_world_and_camera_mapping() {
    let m = mapping();
    let mut d = NetworkDragController::new(simulation(), Default::default()).unwrap();
    let pointer = m.world_to_client([25., -7.]).unwrap();
    assert_eq!(d.pick(pointer, &m, &BTreeSet::new()).unwrap(), Some("a"));
    d.begin(17, "a", pointer, &m).unwrap();
    assert_eq!(d.simulation().node("a").unwrap().position, [20., -10.]);
    let offset = d.active().unwrap().grab_offset;
    assert!((offset[0] + 5.).abs() < 1e-12 && (offset[1] + 3.).abs() < 1e-12);
    let next = [pointer[0] + 30., pointer[1] - 45.];
    d.move_pointer(17, next, &m).unwrap();
    let node = d.simulation().node("a").unwrap().clone();
    assert!((node.position[0] - 50.).abs() < 1e-10 && (node.position[1] + 25.).abs() < 1e-10);
    d.tick(5).unwrap();
    assert_eq!(d.simulation().node("a").unwrap().fixed, Some(node.position));
    assert!(d.end(17, next, &m).unwrap());
    assert_eq!(d.simulation().node("a").unwrap().fixed, None);
    assert_eq!(d.simulation().options().alpha_target, 0.);
}
#[test]
fn fixed_prior_release_policies_and_cancel_restore_real_node_and_heat_state() {
    let m = mapping();
    for prior in [false, true] {
        for release in [
            NetworkDragRelease::RestoreFixedness,
            NetworkDragRelease::Free,
            NetworkDragRelease::Pin,
        ] {
            let mut s = simulation();
            if prior {
                s.set_fixed("a", Some([20., -10.])).unwrap();
            } else {
                s.set_velocity("a", [3., 4.]).unwrap();
            }
            s.set_alpha_target(0.7).unwrap();
            let before = s.node("a").unwrap().clone();
            let mut d = NetworkDragController::new(
                s,
                NetworkDragOptions {
                    release,
                    ..Default::default()
                },
            )
            .unwrap();
            let p = m.world_to_client(before.position).unwrap();
            d.begin(1, "a", p, &m).unwrap();
            assert_eq!(d.simulation().options().alpha_target, 0.7);
            d.move_pointer(1, [p[0] + 20., p[1]], &m).unwrap();
            let end = d.simulation().node("a").unwrap().position;
            d.end(1, [p[0] + 20., p[1]], &m).unwrap();
            let pinned = release == NetworkDragRelease::Pin
                || prior && release == NetworkDragRelease::RestoreFixedness;
            assert_eq!(
                d.simulation().node("a").unwrap().fixed,
                pinned.then_some(end)
            );
            let mut s = simulation();
            if prior {
                s.set_fixed("a", Some(before.position)).unwrap();
            } else {
                s.set_velocity("a", before.velocity).unwrap();
            }
            let original = s.node("a").unwrap().clone();
            let mut d = NetworkDragController::new(s, Default::default()).unwrap();
            d.begin(1, "a", p, &m).unwrap();
            d.move_pointer(1, [p[0] + 20., p[1]], &m).unwrap();
            d.tick(3).unwrap();
            d.cancel().unwrap();
            assert_eq!(d.simulation().node("a").unwrap(), &original);
            assert_eq!(d.simulation().options().alpha_target, 0.);
        }
    }
}
#[test]
fn invalid_and_foreign_pointer_inputs_leave_the_drag_and_physics_unchanged() {
    let m = mapping();
    let mut d = NetworkDragController::new(simulation(), Default::default()).unwrap();
    let p = m.world_to_client([20., -10.]).unwrap();
    assert!(d.begin(1, "missing", p, &m).is_err());
    assert!(d.active().is_none());
    d.begin(1, "a", p, &m).unwrap();
    let before = d.simulation().nodes().to_vec();
    let session = d.active().cloned();
    let alpha = d.simulation().alpha();
    assert!(d.begin(2, "b", p, &m).is_err());
    assert!(!d.move_pointer(2, [f64::NAN, 0.], &m).unwrap());
    assert!(!d.end(2, p, &m).unwrap());
    assert!(d.move_pointer(1, [f64::NAN, 0.], &m).is_err());
    assert!(d.end(1, [f64::INFINITY, 0.], &m).is_err());
    assert!(d.tick(MAX_FORCE_TICKS_PER_STEP + 1).is_err());
    assert_eq!(d.simulation().nodes(), before);
    assert_eq!(d.active(), session.as_ref());
    assert_eq!(d.simulation().alpha(), alpha);
}
#[test]
fn rebasing_on_resize_camera_or_scroll_keeps_world_position_and_next_move_continuous() {
    let m = mapping();
    let mut d = NetworkDragController::new(simulation(), Default::default()).unwrap();
    let p = m.world_to_client([24., -8.]).unwrap();
    d.begin(5, "a", p, &m).unwrap();
    let mut next = m.clone();
    next.client_bounds.x -= 40.;
    next.client_bounds.width *= 0.8;
    next.scene_size = [600., 400.];
    next.world_offset = [310., 190.];
    next.view = ZoomTransform::new(3., -50., -20.).unwrap();
    d.rebase(&next).unwrap();
    let before = d.simulation().node("a").unwrap().position;
    d.move_pointer(5, p, &next).unwrap();
    assert_eq!(d.simulation().node("a").unwrap().position, before);
    let q = [p[0] + 12., p[1] - 18.];
    let a = next.client_to_world(p).unwrap();
    let b = next.client_to_world(q).unwrap();
    d.move_pointer(5, q, &next).unwrap();
    let actual = d.simulation().node("a").unwrap().position;
    assert!((actual[0] - before[0] - (b[0] - a[0])).abs() < 1e-10);
    assert!((actual[1] - before[1] - (b[1] - a[1])).abs() < 1e-10);
}
#[test]
fn picking_matches_current_layout_topmost_nodes_and_hidden_owned_labels() {
    let c = ChartConfig::default();
    let mut s = simulation();
    s.set_position("b", [20., -10.]).unwrap();
    let m = NetworkDragMapping::from_config(
        &c,
        ClipRect {
            x: 0.,
            y: 0.,
            width: c.width,
            height: c.height,
        },
        ZoomTransform::default(),
    )
    .unwrap();
    let d = NetworkDragController::new(s, Default::default()).unwrap();
    let p = m.world_to_client([20., -10.]).unwrap();
    assert_eq!(d.pick(p, &m, &BTreeSet::new()).unwrap(), Some("b"));
    let hidden = BTreeSet::from(["b".into()]);
    assert_eq!(d.pick(p, &m, &hidden).unwrap(), Some("a"));
    let scene = d.simulation().scene(&c).unwrap().without_series(&hidden);
    assert_eq!(scene.hit_test(p[0], p[1]).unwrap().series, "a");
    assert_eq!(scene.labels.len(), 1);
    assert_eq!(scene.labels[0].text, "A");
    assert_eq!(d.pick([-1., 0.], &m, &BTreeSet::new()).unwrap(), None);
}
#[test]
fn release_resumes_cooling_and_invalid_mapping_cannot_change_the_grab() {
    let m = mapping();
    let mut d = NetworkDragController::new(simulation(), Default::default()).unwrap();
    let p = m.world_to_client([20., -10.]).unwrap();
    d.begin(1, "a", p, &m).unwrap();
    assert!(d.simulation().is_active());
    let before = d.active().cloned();
    let mut invalid = m.clone();
    invalid.client_bounds.width = 0.;
    assert!(d.rebase(&invalid).is_err());
    assert_eq!(d.active(), before.as_ref());
    d.end(1, p, &m).unwrap();
    for _ in 0..100 {
        d.tick(1).unwrap();
    }
    assert!(!d.simulation().is_active());
}
#[test]
fn replacing_the_graph_retires_drag_heat_and_original_pin_without_leaking_to_new_graph() {
    let m = mapping();
    let mut old = simulation();
    old.set_fixed("a", Some([20., -10.])).unwrap();
    old.set_alpha_target(0.1).unwrap();
    let mut controller = NetworkDragController::new(old, Default::default()).unwrap();
    let p = m.world_to_client([20., -10.]).unwrap();
    controller.begin(1, "a", p, &m).unwrap();
    controller.move_pointer(1, [p[0] + 20., p[1]], &m).unwrap();
    let mut new = ForceSimulation::new(
        &[NetworkNode {
            id: "new".into(),
            label: "New graph".into(),
            radius: 8.,
        }],
        &[],
        ForceOptions {
            alpha: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    new.set_position("new", [12., 15.]).unwrap();
    let previous = controller.replace_simulation(new).unwrap();
    assert_eq!(previous.node("a").unwrap().fixed, Some([20., -10.]));
    assert_eq!(previous.options().alpha_target, 0.1);
    assert!(controller.active().is_none());
    assert_eq!(controller.simulation().nodes().len(), 1);
    assert_eq!(
        controller.simulation().node("new").unwrap().position,
        [12., 15.]
    );
    assert_eq!(controller.simulation().options().alpha_target, 0.);
}
