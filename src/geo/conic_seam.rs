//! Antimeridian fragment/rejoin for Conic polygons only.
//! Adapted from d3-geo 3.1.1 src/clip/{index,antimeridian,rejoin}.js.
//! Copyright 2010-2024 Mike Bostock. ISC; see tests/fixtures/geo-inverse-d3-LICENSE.txt.
//! Inputs retain the existing bounded, non-pole-enclosing polygon validation.
use super::*;
const D3_EPS: f64 = 1e-6;
fn equal(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < D3_EPS && (a[1] - b[1]).abs() < D3_EPS
}
fn canonical(mut p: [f64; 2]) -> [f64; 2] {
    while p[0] > PI {
        p[0] -= TAU;
    }
    while p[0] < -PI {
        p[0] += TAU;
    }
    p
}
fn fragments(ring: &Ring) -> Result<Vec<Ring>> {
    let mut lines = vec![vec![canonical(ring[0])]];
    let mut a = canonical(ring[0]);
    let mut cut = false;
    for i in 1..=ring.len() {
        let b = canonical(ring[i % ring.len()]);
        let sa = if a[0] > 0. { PI } else { -PI };
        let sb = if b[0] > 0. { PI } else { -PI };
        if sa != sb && (b[0] - a[0]).abs() >= PI {
            let cross = seam_intersection(a, b, sa)?;
            lines.last_mut().unwrap().push(cross);
            lines.push(vec![[sb, cross[1]]]);
            cut = true;
        }
        lines.last_mut().unwrap().push(b);
        a = b;
    }
    if cut && lines.len() > 1 {
        let mut last = lines.pop().unwrap();
        last.extend(lines.remove(0));
        lines.push(last);
    }
    Ok(lines)
}
#[derive(Clone)]
struct Node {
    p: [f64; 2],
    segment: usize,
    other: usize,
    next: usize,
    prev: usize,
    entry: bool,
    visited: bool,
}
fn interpolate(from: [f64; 2], to: [f64; 2], direction: f64, out: &mut Ring) {
    if (from[0] - to[0]).abs() > D3_EPS {
        let lambda = if from[0] < to[0] { PI } else { -PI };
        let phi = direction * lambda / 2.;
        out.extend([[-lambda, phi], [0., phi], [lambda, phi]]);
    } else {
        out.push(to);
    }
}
fn link(nodes: &mut [Node], indices: &[usize]) {
    for (i, &n) in indices.iter().enumerate() {
        nodes[n].next = indices[(i + 1) % indices.len()];
        nodes[n].prev = indices[(i + indices.len() - 1) % indices.len()];
    }
}
fn rejoin(segments: &[Ring]) -> Result<Vec<Ring>> {
    let mut out = vec![];
    let mut nodes = vec![];
    let mut subject = vec![];
    let mut clip = vec![];
    for (s, segment) in segments.iter().enumerate() {
        if segment.len() < 2 {
            continue;
        }
        let a = segment[0];
        let b = *segment.last().unwrap();
        if equal(a, b) {
            out.push(segment[..segment.len() - 1].to_vec());
            continue;
        }
        for (p, entry) in [(a, true), (b, false)] {
            let n = nodes.len();
            nodes.push(Node {
                p,
                segment: s,
                other: n + 1,
                next: 0,
                prev: 0,
                entry,
                visited: false,
            });
            nodes.push(Node {
                p,
                segment: s,
                other: n,
                next: 0,
                prev: 0,
                entry: !entry,
                visited: false,
            });
            subject.push(n);
            clip.push(n + 1);
        }
    }
    if subject.is_empty() {
        return Ok(out);
    }
    let key = |p: [f64; 2]| {
        if p[0] < 0. {
            p[1] - PI / 2. - D3_EPS
        } else {
            PI / 2. - p[1]
        }
    };
    clip.sort_by(|&a, &b| key(nodes[a].p).total_cmp(&key(nodes[b].p)));
    link(&mut nodes, &subject);
    link(&mut nodes, &clip);
    // The validated unwrapped exterior excludes either pole; winding is
    // normalized below, so the south-pole clip start is outside.
    let mut entry = false;
    for &c in &clip {
        entry = !entry;
        nodes[c].entry = entry;
    }
    for &start in &subject {
        if nodes[start].visited {
            continue;
        }
        let mut ring = vec![];
        let mut current = start;
        let mut is_subject = true;
        let mut visits = 0;
        loop {
            visits += 1;
            ensure(
                visits <= nodes.len() + 1,
                "conic seam rejoin traversal budget exceeded",
            )?;
            let n = nodes[current].clone();
            nodes[current].visited = true;
            nodes[n.other].visited = true;
            let next = if n.entry { n.next } else { n.prev };
            if is_subject {
                let points = &segments[if n.entry {
                    n.segment
                } else {
                    nodes[n.prev].segment
                }];
                if n.entry {
                    ring.extend(points.iter().copied());
                } else {
                    ring.extend(points.iter().rev().copied());
                }
            } else {
                interpolate(
                    n.p,
                    nodes[next].p,
                    if n.entry { 1. } else { -1. },
                    &mut ring,
                );
            }
            current = nodes[next].other;
            is_subject = !is_subject;
            if nodes[current].visited {
                break;
            }
        }
        ring.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-14 && (a[1] - b[1]).abs() < 1e-14);
        if ring.len() > 1 && equal(ring[0], *ring.last().unwrap()) {
            ring.pop();
        }
        if ring.len() >= 3 {
            out.push(ring);
        }
    }
    Ok(out)
}
pub(super) fn polygon(
    mut rings: Vec<Ring>,
    projection: GeoProjection,
    mut adaptive: Option<&mut path::Sampler<'_>>,
    fit_bounds: &mut [f64; 4],
) -> Result<Vec<Vec<Ring>>> {
    let mut segments = vec![];
    for (index, ring) in rings.iter_mut().enumerate() {
        // Input API treats exterior/hole roles independently of source winding.
        if (winding(ring) > 0.) == (index == 0) {
            ring.reverse();
        }
        if projection.latitude_bounded() {
            *ring = clip(
                &clip(ring, 1, -MERCATOR_LIMIT, true)?,
                1,
                MERCATOR_LIMIT,
                false,
            )?;
        }
        if ring.len() >= 3 {
            segments.extend(fragments(ring)?);
        }
    }
    let rings = rejoin(&segments)?;
    let mut projected = vec![];
    let mut points = 0usize;
    for ring in rings {
        let mut baseline = vec![];
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let parallel = projection.latitude_bounded()
                && (a[1] - b[1]).abs() < EPS
                && (a[1].abs() - MERCATOR_LIMIT).abs() < EPS;
            if let Some(bounds) = adaptive
                .is_none()
                .then(|| projection.conic_arc_bounds(a, b, parallel))
                .flatten()
            {
                ensure(
                    bounds.iter().all(|v| v.is_finite()),
                    "conic edge fit bounds are not representable",
                )?;
                merge_geo_bounds(fit_bounds, bounds);
            }
            baseline.push(a);
            if (a[0].abs() - PI).abs() < EPS && (b[0] - a[0]).abs() < EPS {
                let n = ((b[1] - a[1]).abs() / PRECISION).ceil().max(1.) as usize;
                for j in 1..n {
                    baseline.push([a[0], a[1] + (b[1] - a[1]) * j as f64 / n as f64]);
                }
            }
        }
        points = points.saturating_add(baseline.len());
        ensure(
            points <= 180_000,
            "conic seam polygon sampling budget exceeded",
        )?;
        projected.push(if let Some(sampler) = adaptive.as_deref_mut() {
            sampler.planar_ring(&baseline)?
        } else {
            baseline.into_iter().map(|p| raw(p, projection)).collect()
        });
    }
    Ok(if projected.is_empty() {
        vec![]
    } else {
        vec![projected]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(points: &[[f64; 2]]) -> Ring {
        points.iter().map(|p| p.map(f64::to_radians)).collect()
    }
    #[test]
    fn exact_seam_vertices_and_two_holes_do_not_acquire_border_across_holes() {
        // Explicit vertices on 180 degrees exercise endpoint intersections.
        let rings = vec![
            r(&[
                [170., -30.],
                [170., 30.],
                [180., 30.],
                [190., 30.],
                [190., -30.],
                [180., -30.],
            ]),
            r(&[
                [175., -20.],
                [180., -20.],
                [185., -20.],
                [185., -10.],
                [180., -10.],
                [175., -10.],
            ]),
            r(&[
                [175., 10.],
                [180., 10.],
                [185., 10.],
                [185., 20.],
                [180., 20.],
                [175., 20.],
            ]),
        ];
        let mut segments = vec![];
        for ring in rings {
            segments.extend(fragments(&ring).unwrap());
        }
        let result = rejoin(&segments).unwrap();
        assert_eq!(result.len(), 2);
        for ring in &result {
            assert!(
                winding(ring) < 0.,
                "seam exteriors must keep clockwise spherical orientation"
            );
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                if (a[0].abs() - PI).abs() < 1e-12 && (a[0] - b[0]).abs() < 1e-12 {
                    for lat in [-15_f64, 15.] {
                        let lat = lat.to_radians();
                        assert!(
                            !(a[1].min(b[1]) < lat && lat < a[1].max(b[1])),
                            "extra seam border crosses a hole: {a:?}->{b:?}"
                        );
                    }
                }
            }
        }
    }
}
