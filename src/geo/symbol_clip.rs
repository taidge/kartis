//! Planar symbol/disk intersections. References: d3-geo/clip/{circle,rejoin}.js
//! for directed boundary rejoining; circular arcs use the standard tangent
//! cubic k=4/3*tan(delta/4), with |delta|<=pi/32. Its radial error is <2e-11*r,
//! hence <0.00003 logical px at the admitted radius <=1,000,000. An additional
//! radius-dependent angular bound keeps Scene's 24-step cubic hit tessellation
//! within 0.004 px of the curve. This is a bounded approximation, not rational
//! circular-arc boolean geometry.
use super::GeoSymbolShape;
use crate::scene::ensure;
use crate::{ChartError, GeoProjectionDisk, Geometry, Result};
use glory::render::{GraphicPoint as P, PathCommand as Cmd};
use std::f64::consts::{PI, TAU};

pub(super) fn validate(disk: GeoProjectionDisk) -> Result<()> {
    ensure(
        disk.center
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            && disk.radius.is_finite()
            && (0.01..=1_000_000.).contains(&disk.radius),
        "projection symbol clip requires a bounded disk (radius 0.01..=1000000, center within Scene coordinates)",
    )
}
fn point(p: [f64; 2], offset: [f64; 2]) -> P {
    P::new(p[0] + offset[0], p[1] + offset[1])
}
fn circle(center: [f64; 2], radius: f64) -> Geometry {
    Geometry::Circle {
        x: center[0],
        y: center[1],
        radius,
    }
}
fn at(center: [f64; 2], radius: f64, a: f64) -> [f64; 2] {
    [
        radius.mul_add(a.cos(), center[0]),
        radius.mul_add(a.sin(), center[1]),
    ]
}
fn arc(
    path: &mut Vec<Cmd>,
    center: [f64; 2],
    radius: f64,
    start: f64,
    delta: f64,
    offset: [f64; 2],
) {
    let max_step = (PI / 32.).min((16. / radius).sqrt());
    let n = (delta / max_step).ceil().max(1.) as usize;
    let step = delta / n as f64;
    let k = 4. / 3. * (step / 4.).tan() * radius;
    for i in 0..n {
        let a = start + i as f64 * step;
        let b = a + step;
        let begin = at(center, radius, a);
        let end = at(center, radius, b);
        path.push(Cmd::Cubic {
            first: point([begin[0] - k * a.sin(), begin[1] + k * a.cos()], offset),
            second: point([end[0] + k * b.sin(), end[1] - k * b.cos()], offset),
            end: point(end, offset),
        });
    }
}

pub(super) fn intersect(
    shape: GeoSymbolShape,
    center: [f64; 2],
    r: f64,
    disk: GeoProjectionDisk,
) -> Result<Option<Geometry>> {
    let local = [center[0] - disk.center[0], center[1] - disk.center[1]];
    if shape == GeoSymbolShape::Circle {
        if local[0].hypot(local[1]) + r <= disk.radius {
            return Ok(Some(circle(center, r)));
        }
        return lens(local, r, disk);
    }
    let vertices = shape
        .vertices(r)
        .into_iter()
        .map(|p| [p[0] + local[0], p[1] + local[1]])
        .collect::<Vec<_>>();
    if vertices.iter().all(|p| p[0].hypot(p[1]) <= disk.radius) {
        return Ok(Some(shape.geometry(center[0], center[1], r)));
    }
    ensure(
        r >= 1e-6,
        "partially clipped geographic symbols require radius >=0.000001 logical pixels",
    )?;
    polygon(vertices, disk)
}
fn lens(c: [f64; 2], r: f64, disk: GeoProjectionDisk) -> Result<Option<Geometry>> {
    let d = c[0].hypot(c[1]);
    let cap = disk.radius;
    if d + r <= cap {
        return Ok(Some(circle(
            [c[0] + disk.center[0], c[1] + disk.center[1]],
            r,
        )));
    }
    if d + cap <= r {
        return Ok(Some(circle(disk.center, cap)));
    }
    if d >= r + cap {
        return Ok(None);
    }
    ensure(
        r >= 1e-6,
        "partially clipped geographic symbols require radius >=0.000001 logical pixels",
    )?;
    // Heron's height avoids acos cancellation for a small symbol on a large
    // cap. All products are bounded by the admitted logical coordinates.
    let h = ((r + cap - d) * (d + r - cap) * (d + cap - r) * (r + cap + d))
        .max(0.)
        .sqrt()
        / (2. * d);
    if h == 0. {
        return Ok(None);
    }
    let x = (d * d + (cap - r) * (cap + r)) / (2. * d);
    let y = (d * d + (r - cap) * (r + cap)) / (2. * d);
    let phi = c[1].atan2(c[0]);
    let cap_half = h.atan2(x);
    let symbol_half = h.atan2(y);
    let begin = phi - cap_half;
    let mut path = vec![Cmd::Move(point(at([0., 0.], cap, begin), disk.center))];
    arc(&mut path, [0., 0.], cap, begin, 2. * cap_half, disk.center);
    arc(
        &mut path,
        c,
        r,
        phi + PI - symbol_half,
        2. * symbol_half,
        disk.center,
    );
    path.push(Cmd::Close);
    Ok(Some(Geometry::Path(path)))
}
fn inside(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut hit = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            hit = !hit;
        }
    }
    hit
}
fn polygon(mut vertices: Vec<[f64; 2]>, disk: GeoProjectionDisk) -> Result<Option<Geometry>> {
    let cap = disk.radius;
    let tolerance = 1e-10_f64.max(cap * 1e-13);
    for p in &mut vertices {
        let norm = p[0].hypot(p[1]);
        if (norm - cap).abs() <= tolerance {
            p[0] *= cap / norm;
            p[1] *= cap / norm;
        }
    }
    let original = vertices.clone();
    let n = vertices.len();
    let mut edges = vec![];
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (vertices[i], vertices[j]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy);
        let unit = [dx / length, dy / length];
        let distance = -(a[0] * unit[0] + a[1] * unit[1]);
        let closest = [
            unit[0].mul_add(distance, a[0]),
            unit[1].mul_add(distance, a[1]),
        ];
        let norm = closest[0].hypot(closest[1]);
        let mut cuts = vec![(0., i), (1., j)];
        if norm <= cap + tolerance {
            let offset = ((cap - norm).max(0.) * (cap + norm)).sqrt();
            for t in [(distance - offset) / length, (distance + offset) / length] {
                if t > 1e-12 && t < 1. - 1e-12 && !cuts.iter().any(|(u, _)| (u - t).abs() < 1e-12) {
                    let mut p = [dx.mul_add(t, a[0]), dy.mul_add(t, a[1])];
                    let d = p[0].hypot(p[1]);
                    p[0] *= cap / d;
                    p[1] *= cap / d;
                    cuts.push((t, vertices.len()));
                    vertices.push(p);
                }
            }
        }
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in cuts.windows(2) {
            let t = (pair[0].0 + pair[1].0) / 2.;
            let p = [dx.mul_add(t, a[0]), dy.mul_add(t, a[1])];
            if p[0].hypot(p[1]) <= cap + tolerance {
                edges.push((pair[0].1, pair[1].1));
            }
        }
    }
    let mut next = vec![None; vertices.len()];
    let mut incoming = vec![0; vertices.len()];
    for (a, b) in edges {
        ensure(
            next[a].replace((b, false)).is_none(),
            "ambiguous geographic symbol clip branching",
        )?;
        incoming[b] += 1;
        ensure(
            incoming[b] <= 1,
            "ambiguous geographic symbol clip junction",
        )?;
    }
    let angle = |i: usize| vertices[i][1].atan2(vertices[i][0]).rem_euclid(TAU);
    let mut boundary = (0..vertices.len())
        .filter(|i| next[*i].is_some() != (incoming[*i] == 1))
        .collect::<Vec<_>>();
    boundary.sort_by(|a, b| angle(*a).total_cmp(&angle(*b)));
    ensure(
        boundary.len() % 2 == 0,
        "odd geographic symbol clip topology",
    )?;
    for i in 0..boundary.len() {
        let a = boundary[i];
        let b = boundary[(i + 1) % boundary.len()];
        let delta = (angle(b) - angle(a)).rem_euclid(TAU);
        if delta < 1e-12 {
            continue;
        }
        let p = at([0., 0.], cap, angle(a) + delta / 2.);
        if !inside(p, &original) {
            continue;
        }
        ensure(
            next[a].is_none() && incoming[a] == 1 && next[b].is_some() && incoming[b] == 0,
            "ambiguous geographic symbol clip endpoint order",
        )?;
        next[a] = Some((b, true));
        incoming[b] = 1;
    }
    if boundary.is_empty() && next.iter().all(Option::is_none) {
        return Ok(inside([0., 0.], &original).then(|| circle(disk.center, cap)));
    }
    let mut visited = vec![false; vertices.len()];
    let mut path = vec![];
    for start in 0..vertices.len() {
        if visited[start] || next[start].is_none() {
            continue;
        }
        let mut current = start;
        path.push(Cmd::Move(point(vertices[start], disk.center)));
        loop {
            ensure(!visited[current], "geographic symbol clip failed to close")?;
            visited[current] = true;
            let (end, circular) = next[current]
                .ok_or_else(|| ChartError("open geographic symbol clip contour".into()))?;
            if circular {
                arc(
                    &mut path,
                    [0., 0.],
                    cap,
                    angle(current),
                    (angle(end) - angle(current)).rem_euclid(TAU),
                    disk.center,
                );
            } else {
                path.push(Cmd::Line(point(vertices[end], disk.center)));
            }
            current = end;
            if current == start {
                break;
            }
        }
        path.push(Cmd::Close);
    }
    Ok((!path.is_empty()).then_some(Geometry::Path(path)))
}
