//! Independent audit of a real late force failure, rather than an input-limit
//! rejection. The simulation's first tick can commit; the drag batch cannot.
use kartis::{network::*, *};
fn simulation() -> ForceSimulation {
    let mut simulation = ForceSimulation::new(
        &[NetworkNode::new("drag"), NetworkNode::new("edge")],
        &[],
        ForceOptions {
            repulsion: 0.,
            link_strength: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            velocity_decay: 0.,
            max_speed: 10.,
            alpha: 0.8,
            alpha_decay: 0.1,
            ..Default::default()
        },
    )
    .unwrap();
    simulation.set_position("drag", [0., 0.]).unwrap();
    simulation.set_velocity("drag", [2., 3.]).unwrap();
    simulation
        .set_position("edge", [MAX_FORCE_COORDINATE - 15., 0.])
        .unwrap();
    simulation.set_velocity("edge", [10., 0.]).unwrap();
    simulation
}
fn mapping() -> NetworkDragMapping {
    let config = ChartConfig::default();
    NetworkDragMapping::from_config(
        &config,
        ClipRect {
            x: 25.,
            y: 40.,
            width: config.width,
            height: config.height,
        },
        ZoomTransform::default(),
    )
    .unwrap()
}
#[test]
fn late_force_failure_leaves_the_whole_drag_batch_and_session_uncommitted() {
    let mut ordinary = simulation();
    ordinary.tick().unwrap();
    assert_eq!(
        ordinary.node("edge").unwrap().position[0],
        MAX_FORCE_COORDINATE - 5.
    );
    assert!(
        ordinary.tick().is_err(),
        "fixture must fail during actual integration after one successful tick"
    );
    let mut controller = NetworkDragController::new(simulation(), Default::default()).unwrap();
    let mapping = mapping();
    let pointer = mapping.world_to_client([0., 0.]).unwrap();
    controller.begin(1, "drag", pointer, &mapping).unwrap();
    controller
        .move_pointer(1, [pointer[0] + 15., pointer[1] - 5.], &mapping)
        .unwrap();
    let nodes = controller.simulation().nodes().to_vec();
    let alpha = controller.simulation().alpha();
    let session = controller.active().cloned();
    let target = controller.simulation().options().alpha_target;
    assert!(controller.tick(2).is_err());
    assert_eq!(controller.simulation().nodes(), nodes);
    assert_eq!(controller.simulation().alpha(), alpha);
    assert_eq!(controller.simulation().total_ticks(), 0);
    assert_eq!(controller.simulation().options().alpha_target, target);
    assert_eq!(controller.active(), session.as_ref());
    controller.cancel().unwrap();
    assert!(controller.active().is_none());
    let restored = controller.simulation().node("drag").unwrap();
    assert_eq!(restored.position, [0., 0.]);
    assert_eq!(restored.velocity, [2., 3.]);
    assert!(restored.fixed.is_none());
    assert_eq!(controller.simulation().options().alpha_target, 0.);
}
