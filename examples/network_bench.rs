//! `cargo run --release --example network_bench`
//! Informational timings; correctness is asserted in tests/network.rs.
use kartis::network::{ForceOptions, ForceSimulation, many_body_forces};
use kartis::{NetworkLink, NetworkNode};
use std::hint::black_box;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = 4096;
    let points: Vec<_> = (0..count)
        .map(|i| {
            [
                ((i as u64 * 2_654_435_761 + 17) % 10_007) as f64 / 10.,
                ((i as u64 * 1_103_515_245 + 31) % 10_009) as f64 / 10.,
            ]
        })
        .collect();
    let started = Instant::now();
    let exact = many_body_forces(
        black_box(&points),
        &ForceOptions {
            theta: 0.,
            ..Default::default()
        },
    )?;
    let exact_time = started.elapsed();
    let started = Instant::now();
    let approximate = many_body_forces(black_box(&points), &ForceOptions::default())?;
    let approximate_time = started.elapsed();
    let error: f64 = approximate
        .forces
        .iter()
        .zip(&exact.forces)
        .map(|(a, b)| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2))
        .sum();
    let norm: f64 = exact
        .forces
        .iter()
        .map(|f| f[0].powi(2) + f[1].powi(2))
        .sum();
    println!(
        "nodes,exact_ms,barnes_hut_ms,relative_l2,direct_pairs,aggregate_cells,quadtree_cells"
    );
    println!(
        "{count},{:.3},{:.3},{:.6},{},{},{}",
        exact_time.as_secs_f64() * 1000.,
        approximate_time.as_secs_f64() * 1000.,
        (error / norm).sqrt(),
        approximate.stats.direct_interactions,
        approximate.stats.approximated_cells,
        approximate.stats.quadtree_cells
    );
    let nodes: Vec<_> = (0..1024).map(|i| NetworkNode::new(i.to_string())).collect();
    let links: Vec<_> = (1..nodes.len())
        .map(|i| NetworkLink::new((i - 1).to_string(), i.to_string(), 1.))
        .collect();
    let mut simulation = ForceSimulation::new(&nodes, &links, ForceOptions::default())?;
    let started = Instant::now();
    let stats = simulation.step(20)?;
    println!("simulation_nodes,ticks,total_ms,collision_pairs");
    println!(
        "{},{},{:.3},{}",
        nodes.len(),
        stats.ticks,
        started.elapsed().as_secs_f64() * 1000.,
        stats.collision_pairs
    );
    Ok(())
}
