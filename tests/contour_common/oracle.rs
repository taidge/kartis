use std::cmp::Ordering;

use kartis::{
    ContourLevel as Level, ContourPoint as Point, ContourPolygon as Polygon, ContourRing as Ring,
};

pub fn parse_levels(source: &str) -> Vec<Level> {
    if source.is_empty() {
        return vec![];
    }
    source
        .split('!')
        .map(|text| {
            let (value, body) = text.split_once('#').unwrap();
            let polygons = if body.is_empty() {
                vec![]
            } else {
                body.split('~')
                    .map(|polygon| {
                        let mut rings = polygon.split('/').map(|ring| {
                            ring.split(';')
                                .map(|p| {
                                    let (x, y) = p.split_once(',').unwrap();
                                    [x.parse().unwrap(), y.parse().unwrap()]
                                })
                                .collect::<Ring>()
                        });
                        Polygon {
                            outer: rings.next().unwrap(),
                            holes: rings.collect(),
                        }
                    })
                    .collect()
            };
            Level {
                value: value.parse().unwrap(),
                polygons,
            }
        })
        .collect()
}
fn point_cmp(a: &Point, b: &Point) -> Ordering {
    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
}
fn ring_cmp(a: &Ring, b: &Ring) -> Ordering {
    a.iter()
        .zip(b)
        .find_map(|(a, b)| {
            let c = point_cmp(a, b);
            (c != Ordering::Equal).then_some(c)
        })
        .unwrap_or(a.len().cmp(&b.len()))
}
fn canonical_ring(ring: &mut Ring) {
    assert!(ring.len() >= 4);
    assert_eq!(ring.first(), ring.last());
    ring.pop();
    let n = ring.len();
    let (mut i, mut j, mut offset) = (0, 1, 0);
    while i < n && j < n && offset < n {
        match point_cmp(&ring[(i + offset) % n], &ring[(j + offset) % n]) {
            Ordering::Equal => offset += 1,
            Ordering::Greater => {
                i += offset + 1;
                if i == j {
                    i += 1;
                }
                offset = 0;
            }
            Ordering::Less => {
                j += offset + 1;
                if i == j {
                    j += 1;
                }
                offset = 0;
            }
        }
    }
    ring.rotate_left(i.min(j));
    ring.push(ring[0]);
}
fn canonical(level: &mut Level) {
    for p in &mut level.polygons {
        canonical_ring(&mut p.outer);
        p.holes.iter_mut().for_each(canonical_ring);
        p.holes.sort_by(ring_cmp);
    }
    level.polygons.sort_by(|a, b| ring_cmp(&a.outer, &b.outer));
}
pub fn compare(mut actual: Level, mut expected: Level, context: &str) {
    canonical(&mut actual);
    canonical(&mut expected);
    assert_eq!(actual.value, expected.value, "{context}");
    assert_eq!(
        actual.polygons.len(),
        expected.polygons.len(),
        "{context}: polygon count"
    );
    for (a, b) in actual.polygons.iter().zip(&expected.polygons) {
        assert_eq!(
            a.holes.len(),
            b.holes.len(),
            "{context}: hole count; actual={actual:?} expected={expected:?}"
        );
        for (a, b) in std::iter::once(&a.outer)
            .chain(&a.holes)
            .zip(std::iter::once(&b.outer).chain(&b.holes))
        {
            assert_eq!(
                a.len(),
                b.len(),
                "{context}: ring point count: {a:?} vs {b:?}"
            );
            for (a, b) in a.iter().zip(b) {
                for (a, b) in a.iter().zip(b) {
                    assert!(
                        (a - b).abs() <= 2e-12 * b.abs().max(1.),
                        "{context}: coordinate {a} vs {b}"
                    );
                }
            }
        }
    }
}
