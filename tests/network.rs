use kartis::network::*;
use kartis::*;

fn seeded_points(n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let x = ((i as u64 * 2_654_435_761 + 17) % 10_007) as f64 / 10.;
            let y = ((i as u64 * 1_103_515_245 + 31) % 10_009) as f64 / 10.;
            [x, y]
        })
        .collect()
}
fn brute_force(points: &[[f64; 2]], o: &ForceOptions) -> Vec<[f64; 2]> {
    let mut forces = vec![[0., 0.]; points.len()];
    for (i, a) in points.iter().enumerate() {
        for (j, b) in points.iter().enumerate() {
            if i == j {
                continue;
            }
            let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
            let distance = dx.hypot(dy);
            if o.distance_max.is_some_and(|max| distance > max) {
                continue;
            }
            let magnitude = o.repulsion / distance.max(o.distance_min);
            forces[i][0] += dx / distance * magnitude;
            forces[i][1] += dy / distance * magnitude;
        }
    }
    forces
}
fn relative_l2(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    let error = a
        .iter()
        .zip(b)
        .map(|(a, b)| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2))
        .sum::<f64>();
    let scale = b.iter().map(|b| b[0].powi(2) + b[1].powi(2)).sum::<f64>();
    (error / scale).sqrt()
}
fn quiet_options() -> ForceOptions {
    ForceOptions {
        repulsion: 0.,
        link_strength: 0.,
        center_strength: 0.,
        collision_strength: 0.,
        alpha_decay: 0.,
        ..Default::default()
    }
}
#[test]
fn theta_zero_matches_an_independent_brute_force_reference() {
    let points = seeded_points(180);
    let options = ForceOptions {
        theta: 0.,
        distance_min: 4.,
        ..Default::default()
    };
    let exact = many_body_forces(&points, &options).unwrap();
    let reference = brute_force(&points, &options);
    assert!(relative_l2(&exact.forces, &reference) < 1e-13);
    assert_eq!(exact.stats.approximated_cells, 0);
    assert_eq!(
        exact.stats.direct_interactions,
        (points.len() * (points.len() - 1)) as u64
    );
    let options = ForceOptions {
        distance_max: Some(250.),
        ..options
    };
    let exact = many_body_forces(&points, &options).unwrap();
    assert!(relative_l2(&exact.forces, &brute_force(&points, &options)) < 1e-13);
}
#[test]
fn barnes_hut_has_low_force_error_and_reduces_traversal_work() {
    let points = seeded_points(512);
    let exact = many_body_forces(
        &points,
        &ForceOptions {
            theta: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    let approx = many_body_forces(
        &points,
        &ForceOptions {
            theta: 0.5,
            ..Default::default()
        },
    )
    .unwrap();
    let error = relative_l2(&approx.forces, &exact.forces);
    assert!(error < 0.025, "relative L2 force error {error}");
    assert!(approx.stats.approximated_cells > 0);
    let points = seeded_points(4096);
    let large = many_body_forces(&points, &ForceOptions::default()).unwrap();
    let interactions = large.stats.direct_interactions + large.stats.approximated_cells;
    assert!(
        interactions < (points.len() * points.len() / 8) as u64,
        "unexpected traversal count {interactions}"
    );
    assert!(large.stats.max_depth <= MAX_QUADTREE_DEPTH);
    assert_eq!(large.forces.len(), 4096);
}
#[test]
fn coincident_and_deeply_clustered_quadtree_leaves_are_finite_and_bounded() {
    let points = vec![[0., 0.]; 128];
    let options = ForceOptions::default();
    let a = many_body_forces(&points, &options).unwrap();
    let b = many_body_forces(&points, &options).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.stats.quadtree_cells, 1);
    assert_eq!(a.stats.max_depth, 0);
    assert!(a.forces.iter().flatten().all(|f| f.is_finite()));
    let total = a
        .forces
        .iter()
        .fold([0., 0.], |a, b| [a[0] + b[0], a[1] + b[1]]);
    assert!(total[0].hypot(total[1]) < 1e-8);
    let mut clustered: Vec<_> = (0..64).map(|i| [i as f64 * 1e-20, 0.]).collect();
    clustered.push([MAX_FORCE_COORDINATE, MAX_FORCE_COORDINATE]);
    let result = many_body_forces(&clustered, &options).unwrap();
    assert!(result.stats.max_depth <= MAX_QUADTREE_DEPTH);
    assert!(result.forces.iter().flatten().all(|v| v.is_finite()));
}
#[test]
fn minimum_distance_softens_force_and_maximum_distance_culls_it() {
    let result = many_body_forces(
        &[[0., 0.], [2., 0.], [5., 0.], [20., 0.]],
        &ForceOptions {
            repulsion: 6.,
            theta: 0.,
            distance_min: 3.,
            distance_max: Some(6.),
            ..Default::default()
        },
    )
    .unwrap();
    assert!((result.forces[0][0] + 3.2).abs() < 1e-12);
    assert_eq!(result.forces[0][1], 0.);
}
#[test]
fn fixed_drag_positions_hold_exactly_then_release_and_reheat() {
    let nodes = [NetworkNode::new("fixed"), NetworkNode::new("free")];
    let mut sim = ForceSimulation::new(&nodes, &[], ForceOptions::default()).unwrap();
    sim.set_fixed("fixed", Some([15., -20.])).unwrap();
    sim.set_position("free", [15., -20.]).unwrap();
    sim.step(30).unwrap();
    let fixed = sim.node("fixed").unwrap();
    assert_eq!(fixed.position, [15., -20.]);
    assert_eq!(fixed.velocity, [0., 0.]);
    assert_eq!(fixed.fixed, Some([15., -20.]));
    let before = sim.node("fixed").unwrap().position;
    sim.set_fixed("fixed", None).unwrap();
    sim.reheat(1.).unwrap();
    sim.set_alpha_target(0.3).unwrap();
    sim.step(10).unwrap();
    assert_ne!(sim.node("fixed").unwrap().position, before);
    assert!(sim.alpha() > 0.3);
    sim.scene(&ChartConfig::default())
        .unwrap()
        .validate()
        .unwrap();
}
#[test]
fn links_converge_to_target_distance_and_center_force_is_reusable() {
    let nodes = [NetworkNode::new("a"), NetworkNode::new("b")];
    let mut sim = ForceSimulation::new(
        &nodes,
        &[NetworkLink::new("a", "b", 1.)],
        ForceOptions {
            link_distance: 50.,
            link_strength: 0.2,
            velocity_decay: 0.5,
            ..quiet_options()
        },
    )
    .unwrap();
    sim.set_fixed("a", Some([0., 0.])).unwrap();
    sim.set_position("b", [120., 0.]).unwrap();
    sim.step(200).unwrap();
    let b = sim.node("b").unwrap();
    assert!((b.position[0] - 50.).abs() < 1e-6);
    let mut center = ForceSimulation::new(
        &nodes[..1],
        &[],
        ForceOptions {
            center: [10., 20.],
            center_strength: 0.1,
            velocity_decay: 0.5,
            ..quiet_options()
        },
    )
    .unwrap();
    center.set_position("a", [100., -100.]).unwrap();
    center.step(150).unwrap();
    let p = center.node("a").unwrap().position;
    assert!((p[0] - 10.).hypot(p[1] - 20.) < 1e-5);
}
#[test]
fn collision_projection_separates_coincident_free_nodes_and_respects_fixed_nodes() {
    let nodes = [NetworkNode::new("a"), NetworkNode::new("b")];
    let mut sim = ForceSimulation::new(
        &nodes,
        &[],
        ForceOptions {
            collision_strength: 1.,
            collision_padding: 3.,
            collision_iterations: 1,
            ..quiet_options()
        },
    )
    .unwrap();
    sim.set_fixed("a", Some([0., 0.])).unwrap();
    sim.set_position("b", [0., 0.]).unwrap();
    let stats = sim.tick().unwrap();
    assert_eq!(stats.collision_pairs, 1);
    let a = sim.node("a").unwrap().position;
    let b = sim.node("b").unwrap().position;
    assert_eq!(a, [0., 0.]);
    assert!((b[0].hypot(b[1]) - 19.).abs() < 1e-8);
}
#[test]
fn finite_additional_axis_and_radial_forces_are_composable() {
    let nodes = [NetworkNode::new("a")];
    let mut sim = ForceSimulation::new(&nodes, &[], quiet_options()).unwrap();
    sim.add_force(Force::Axis {
        axis: ForceAxis::X,
        target: 30.,
        strength: 0.1,
    })
    .unwrap();
    sim.step(200).unwrap();
    assert!((sim.node("a").unwrap().position[0] - 30.).abs() < 1e-6);
    sim.clear_additional_forces();
    sim.add_force(Force::Radial {
        center: [0., 0.],
        radius: 50.,
        strength: 0.1,
    })
    .unwrap();
    sim.step(200).unwrap();
    let p = sim.node("a").unwrap().position;
    assert!((p[0].hypot(p[1]) - 50.).abs() < 1e-6);
    let before = sim.additional_forces().to_vec();
    assert!(
        sim.add_force(Force::Radial {
            center: [0., 0.],
            radius: f64::NAN,
            strength: 1.
        })
        .is_err()
    );
    assert_eq!(sim.additional_forces(), before);
}
#[test]
fn rejection_preserves_simulation_state_and_a_later_valid_tick_recovers() {
    let nodes = [NetworkNode::new("a")];
    let mut sim = ForceSimulation::new(
        &nodes,
        &[],
        ForceOptions {
            max_speed: 10_000.,
            velocity_decay: 0.,
            ..quiet_options()
        },
    )
    .unwrap();
    sim.set_position("a", [MAX_FORCE_COORDINATE, 0.]).unwrap();
    sim.set_velocity("a", [10_000., 0.]).unwrap();
    let before = sim.nodes().to_vec();
    let alpha = sim.alpha();
    assert!(sim.tick().is_err());
    assert_eq!(sim.nodes(), before);
    assert_eq!(sim.alpha(), alpha);
    assert_eq!(sim.total_ticks(), 0);
    assert!(sim.step(MAX_FORCE_TICKS_PER_STEP + 1).is_err());
    assert!(sim.set_fixed("a", Some([f64::NAN, 0.])).is_err());
    assert!(sim.set_position("missing", [0., 0.]).is_err());
    assert_eq!(sim.nodes(), before);
    sim.set_velocity("a", [0., 0.]).unwrap();
    sim.tick().unwrap();
    assert_eq!(sim.total_ticks(), 1);
}
#[test]
fn simulation_cools_and_reheats_without_an_implicit_loop() {
    let mut sim = ForceSimulation::new(
        &[NetworkNode::new("a")],
        &[],
        ForceOptions {
            alpha_decay: 1.,
            ..quiet_options()
        },
    )
    .unwrap();
    assert!(sim.is_active());
    sim.tick().unwrap();
    assert_eq!(sim.alpha(), 0.);
    assert!(!sim.is_active());
    sim.reheat(0.8).unwrap();
    assert!(sim.is_active());
    sim.set_alpha_target(0.5).unwrap();
    sim.tick().unwrap();
    assert_eq!(sim.alpha(), 0.5);
    assert!(sim.reheat(f64::NAN).is_err());
}
#[test]
fn simulation_inputs_and_budgets_reject_malformed_values() {
    let n = NetworkNode::new("a");
    assert!(ForceSimulation::new(&[n.clone(), n.clone()], &[], Default::default()).is_err());
    assert!(
        ForceSimulation::new(
            std::slice::from_ref(&n),
            &[NetworkLink::new("a", "missing", 1.)],
            Default::default()
        )
        .is_err()
    );
    assert!(many_body_forces(&[[f64::INFINITY, 0.]], &Default::default()).is_err());
    assert!(many_body_forces(&vec![[0., 0.]; MAX_FORCE_NODES + 1], &Default::default()).is_err());
    assert!(
        ForceSimulation::new(
            &[n],
            &[],
            ForceOptions {
                theta: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn step_batching_is_deterministic_and_large_simulations_emit_valid_scenes() {
    let nodes: Vec<_> = (0..1024).map(|i| NetworkNode::new(i.to_string())).collect();
    let links: Vec<_> = (1..nodes.len())
        .map(|i| NetworkLink::new((i - 1).to_string(), i.to_string(), 1.))
        .collect();
    let options = ForceOptions {
        labels: false,
        collision_iterations: 1,
        ..Default::default()
    };
    let mut batch = ForceSimulation::new(&nodes, &links, options).unwrap();
    let mut ticks = batch.clone();
    batch.step(3).unwrap();
    for _ in 0..3 {
        ticks.tick().unwrap();
    }
    assert_eq!(batch.nodes(), ticks.nodes());
    assert_eq!(batch.alpha(), ticks.alpha());
    let before = batch.nodes().to_vec();
    assert_eq!(batch.step(0).unwrap().ticks, 0);
    assert_eq!(batch.nodes(), before);
    let scene = batch.scene(&ChartConfig::default()).unwrap();
    assert_eq!(scene.marks.len(), 2047);
    assert!(scene.labels.is_empty());
}
