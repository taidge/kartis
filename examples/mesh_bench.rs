//! Reproducible point-mesh/query baseline, independent of GUI frame rates.
use kartis::mesh::{MeshPoint, PointMesh};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let points: Vec<_> = (0..20_000)
        .map(|i| MeshPoint {
            id: format!("p{i:05}"),
            x: ((i * 7919) % 100003) as f64 / 100.,
            y: ((i * 104729) % 80021) as f64 / 100.,
            clip: None,
        })
        .collect();
    let queries: Vec<_> = (0..2000)
        .map(|i| {
            (
                ((i * 3571) % 100003) as f64 / 100.,
                ((i * 17431) % 80021) as f64 / 100.,
            )
        })
        .collect();
    let start = Instant::now();
    let mesh = PointMesh::new(points.clone())?;
    let build_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let results: Vec<_> = queries
        .iter()
        .map(|&(x, y)| black_box(mesh.nearest(x, y, f64::INFINITY).unwrap().id.clone()))
        .collect();
    let kd_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    for ((x, y), result) in queries.iter().zip(&results) {
        let expected = points
            .iter()
            .min_by(|a, b| {
                (a.x - x)
                    .hypot(a.y - y)
                    .total_cmp(&(b.x - x).hypot(b.y - y))
                    .then_with(|| a.id.cmp(&b.id))
            })
            .unwrap();
        assert_eq!(&expected.id, result);
    }
    let brute_ms = start.elapsed().as_secs_f64() * 1000.;
    println!(
        "{{\"points\":20000,\"queries\":2000,\"build_ms\":{build_ms:.3},\"kd_query_ms\":{kd_ms:.3},\"brute_query_ms\":{brute_ms:.3},\"mismatches\":0}}"
    );
    Ok(())
}
