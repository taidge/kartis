use kartis::*;
use std::collections::BTreeSet;

struct Oracle {
    points: &'static [[f64; 2]],
    links: &'static [[usize; 2]],
    triangles: &'static [[usize; 3]],
}
include!("fixtures/delaunay_d3.rs");

fn graph(points: &[[f64; 2]]) -> Delaunay {
    Delaunay::new(
        points
            .iter()
            .enumerate()
            .map(|(i, p)| DelaunaySite::new(format!("site-{i:04}"), p[0], p[1]))
            .collect(),
    )
    .unwrap()
}
fn orient(a: &DelaunaySite, b: &DelaunaySite, c: &DelaunaySite) -> i128 {
    let [ax, ay, bx, by, cx, cy] = [a.x, a.y, b.x, b.y, c.x, c.y].map(|v| v as i128);
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
}
fn circle(a: &DelaunaySite, b: &DelaunaySite, c: &DelaunaySite, d: &DelaunaySite) -> i128 {
    let [ax, ay, bx, by, cx, cy] = [
        a.x as i128 - d.x as i128,
        a.y as i128 - d.y as i128,
        b.x as i128 - d.x as i128,
        b.y as i128 - d.y as i128,
        c.x as i128 - d.x as i128,
        c.y as i128 - d.y as i128,
    ];
    (ax * ax + ay * ay) * (bx * cy - by * cx) - (bx * bx + by * by) * (ax * cy - ay * cx)
        + (cx * cx + cy * cy) * (ax * by - ay * bx)
}
fn certify(g: &Delaunay) {
    let sites = g.sites();
    let next = |e: usize| if e % 3 == 2 { e - 2 } else { e + 1 };
    for t in g.triangles().as_chunks::<3>().0 {
        let [a, b, c] = t.map(|i| &sites[i]);
        assert!(orient(a, b, c) > 0, "CCW nondegenerate triangle");
        for (j, p) in sites.iter().enumerate() {
            if !t.contains(&j) {
                assert!(
                    circle(a, b, c, p) <= 0,
                    "empty circumcircle violation {t:?} {j}"
                );
            }
        }
    }
    for (edge, twin) in g.halfedges().iter().enumerate() {
        if let Some(twin) = *twin {
            assert_eq!(g.halfedges()[twin], Some(edge));
            assert_eq!(g.triangles()[edge], g.triangles()[next(twin)]);
            assert_eq!(g.triangles()[next(edge)], g.triangles()[twin]);
        }
    }
    for (i, edge) in g.inedges().iter().enumerate() {
        if let Some(edge) = *edge {
            assert_eq!(g.triangles()[next(edge)], i);
        }
    }
    for &[a, b] in g.links() {
        assert!(a < b);
        assert!(g.neighbors(&sites[a].id).unwrap().contains(&&sites[b]));
        assert!(g.neighbors(&sites[b].id).unwrap().contains(&&sites[a]));
        // Independently prove links do not properly cross.
        for &[c, d] in g.links() {
            if [a, b].iter().any(|i| [c, d].contains(i)) {
                continue;
            }
            let ab1 = orient(&sites[a], &sites[b], &sites[c]);
            let ab2 = orient(&sites[a], &sites[b], &sites[d]);
            let cd1 = orient(&sites[c], &sites[d], &sites[a]);
            let cd2 = orient(&sites[c], &sites[d], &sites[b]);
            assert!(!(ab1.signum() * ab2.signum() < 0 && cd1.signum() * cd2.signum() < 0));
        }
    }
    if !g.triangles().is_empty() {
        assert_eq!(
            sites.len() + g.triangles().len() / 3,
            g.links().len() + 1,
            "Euler disk identity"
        );
        assert_eq!(
            g.triangles().len() / 3,
            2 * sites.len() - g.hull().len() - 2
        );
        let mut twice_area = 0i128;
        for i in 0..g.hull().len() {
            let a = &sites[g.hull()[i]];
            let b = &sites[g.hull()[(i + 1) % g.hull().len()]];
            twice_area += a.x as i128 * b.y as i128 - a.y as i128 * b.x as i128;
        }
        assert!(twice_area > 0);
    }
}

#[test]
fn d3_twenty_nondegenerate_oracles_plus_exact_integer_certificates() {
    for oracle in ORACLES {
        let g = graph(oracle.points);
        assert_eq!(g.links(), oracle.links);
        let mut triangles = g
            .triangles()
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let mut t = *t;
                t.sort_unstable();
                t
            })
            .collect::<Vec<_>>();
        triangles.sort_unstable();
        assert_eq!(triangles, oracle.triangles);
        certify(&g);
    }
}

#[test]
fn duplicate_owners_signed_zero_and_permutation_stability() {
    let source = vec![
        DelaunaySite::new("z", 0., -0.),
        DelaunaySite::new("a", -0., 0.),
        DelaunaySite::new("c", 10., 0.),
        DelaunaySite::new("b", 0., 10.),
        DelaunaySite::new("d", 4., 3.),
    ];
    let expected = Delaunay::new(source.clone()).unwrap();
    assert_eq!(expected.owner("z").unwrap().id, "a");
    assert_eq!(expected.owner_index("a"), expected.owner_index("z"));
    assert_eq!(expected.neighbors("z"), expected.neighbors("a"));
    assert!(expected.neighbors("missing").is_none());
    assert_eq!(expected.raw_owners().len(), 5);
    assert_eq!(expected.sites().len(), 4);
    let mut rotated = source;
    for _ in 0..rotated.len() {
        rotated.rotate_left(1);
        assert_eq!(Delaunay::new(rotated.clone()).unwrap(), expected);
    }
    rotated.reverse();
    assert_eq!(Delaunay::new(rotated).unwrap(), expected);
    certify(&expected);
}

#[test]
fn truly_collinear_chain_empty_singleton_pair_and_duplicate_only() {
    for n in 0..=20 {
        let points = (0..n)
            .map(|i| [f64::from(i), f64::from(i) * 3. - 2.])
            .collect::<Vec<_>>();
        let g = graph(&points);
        assert!(g.triangles().is_empty());
        assert!(g.halfedges().is_empty());
        assert_eq!(g.hull(), (0..n as usize).collect::<Vec<_>>());
        assert_eq!(g.links().len(), (n as usize).saturating_sub(1));
        for i in 0..n as usize {
            let neighbors = g.neighbors(&g.sites()[i].id).unwrap();
            assert_eq!(
                neighbors.len(),
                usize::from(i > 0) + usize::from(i + 1 < n as usize)
            );
        }
    }
    let g = Delaunay::new(vec![
        DelaunaySite::new("b", 5., 6.),
        DelaunaySite::new("a", 5., 6.),
    ])
    .unwrap();
    assert_eq!(g.sites().len(), 1);
    assert_eq!(g.raw_owners(), [0, 0]);
    assert!(g.links().is_empty());
}

#[test]
fn cocircular_and_grid_are_legal_without_jitter_and_stable_on_permutation() {
    for points in [
        vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
        vec![
            [5., 0.],
            [4., 3.],
            [3., 4.],
            [0., 5.],
            [-3., 4.],
            [-4., 3.],
            [-5., 0.],
            [0., -5.],
        ],
        (0..5)
            .flat_map(|y| (0..5).map(move |x| [f64::from(x), f64::from(y)]))
            .collect(),
    ] {
        let g = graph(&points);
        certify(&g);
        let raw = g.raw_sites().iter().cloned().rev().collect();
        let reversed = Delaunay::new(raw).unwrap();
        assert_eq!(reversed, g);
        assert_eq!(
            g.sites().iter().map(|s| [s.x, s.y]).collect::<Vec<_>>(),
            points
        );
    }
}

#[test]
fn adaptive_predicates_resolve_exact_area_one_despite_naive_cancellation() {
    let points = [
        [0., 0.],
        [1_000_000_000., 999_999_999.],
        [999_999_999., 999_999_998.],
        [900_000_000., -1.],
    ];
    let naive = points[1][0] * points[2][1] - points[1][1] * points[2][0];
    assert_eq!(naive, 0.);
    let g = graph(&points);
    certify(&g);
    assert_eq!(g.triangles().len(), 9);
    assert!(g.triangles().as_chunks::<3>().0.iter().any(|t| orient(
        &g.sites()[t[0]],
        &g.sites()[t[1]],
        &g.sites()[t[2]]
    ) == 1));
}

#[test]
fn uniform_extreme_scales_and_subnormal_are_exactly_supported() {
    let base = graph(&[[0., 0.], [1., 0.], [0., 1.], [1., 2.], [3., 1.]]);
    for scale in [f64::from_bits(1), 1e-300, 1e300, 2_f64.powi(1022)] {
        let g = graph(&[
            [0., 0.],
            [scale, 0.],
            [0., scale],
            [scale, scale * 2.],
            [scale * 3., scale],
        ]);
        assert_eq!(g.links(), base.links());
        assert_eq!(g.triangles(), base.triangles());
    }
    let maximum = graph(&[
        [f64::MAX, 0.],
        [0., f64::MAX],
        [-f64::MAX, 0.],
        [0., -f64::MAX],
    ]);
    assert_eq!(maximum.triangles().len(), 6);
    let endpoints = graph(&[[0., 0.], [2_f64.powi(-142), 0.], [0., 2_f64.powi(201)]]);
    assert_eq!(
        endpoints.triangles().len(),
        3,
        "both inclusive predicate bounds must be admitted without changing coordinates"
    );
}

#[test]
fn invalid_input_and_unrepresentable_dynamic_range_reject_without_mutation() {
    let accepted = graph(&[[0., 0.], [2., 0.], [1., 1.]]);
    let before = accepted.clone();
    for raw in [
        vec![
            DelaunaySite::new("a", 0., 0.),
            DelaunaySite::new("a", 1., 1.),
        ],
        vec![DelaunaySite::new("", 0., 0.)],
        vec![DelaunaySite::new("nan", f64::NAN, 0.)],
        vec![DelaunaySite::new("inf", 0., f64::INFINITY)],
        vec![
            DelaunaySite::new("small", f64::from_bits(1), 0.),
            DelaunaySite::new("large", f64::MAX, 0.),
        ],
    ] {
        assert!(Delaunay::new(raw).is_err());
        assert_eq!(accepted, before);
    }
    let max = MAX_DELAUNAY_SITES;
    assert!(
        Delaunay::new(
            (0..=max)
                .map(|i| DelaunaySite::new(i.to_string(), i as f64, 0.))
                .collect()
        )
        .is_err()
    );
    assert!(
        Delaunay::new(vec![
            DelaunaySite::new("min", 2_f64.powi(-142), 0.),
            DelaunaySite::new("beyond-max", 0., 2_f64.powi(201).next_up())
        ])
        .is_err()
    );
}

#[test]
fn maximum_graph_is_bounded_and_scene_rejection_is_separate() {
    let g = Delaunay::new(
        (0..MAX_DELAUNAY_SITES)
            .map(|i| DelaunaySite::new(format!("{i:05}"), (i % 200) as f64, (i / 200) as f64))
            .collect(),
    )
    .unwrap();
    assert!(g.links().len() < 3 * MAX_DELAUNAY_SITES);
    assert!(g.triangles().len() < 6 * MAX_DELAUNAY_SITES);
    assert!(
        g.diagram(&ChartConfig::default(), &DelaunayDiagramOptions::default())
            .is_err()
    );
}

#[test]
fn scene_projection_preserves_metric_ids_ownership_and_topology() {
    let g = graph(&[[0., 0.], [10., 0.], [0., 20.], [5., 7.]]);
    let options = DelaunayDiagramOptions {
        labels: true,
        triangle_fill: Some(Color(20, 80, 150, 32)),
        ..Default::default()
    };
    let diagram = g.diagram(&ChartConfig::default(), &options).unwrap();
    diagram.scene.validate().unwrap();
    let dx = diagram.sites[1].x - diagram.sites[0].x;
    let dy = diagram.sites[2].y - diagram.sites[0].y;
    assert!((dy / dx - 2.).abs() < 1e-14);
    assert_eq!(
        diagram.scene.marks.len(),
        g.sites().len() + g.links().len() + g.triangles().len() / 3
    );
    assert_eq!(diagram.scene.label_series.len(), 4);
    assert_eq!(
        diagram
            .scene
            .marks
            .iter()
            .filter(|m| !m.label.is_empty())
            .count(),
        g.sites().len(),
        "decorative links/triangles must not fabricate scalar data controls"
    );
    let mut surface =
        surface::SurfaceModel::new(diagram.scene.clone(), surface::SurfaceOptions::default())
            .unwrap();
    let triangle = &g.triangles()[..3];
    let x = triangle.iter().map(|&i| diagram.sites[i].x).sum::<f64>() / 3.;
    let y = triangle.iter().map(|&i| diagram.sites[i].y).sum::<f64>() / 3.;
    surface
        .apply(surface::SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert!(
        surface.hovered().is_empty(),
        "decorative triangle fill must not intercept the data surface"
    );
    let hidden = diagram
        .scene
        .without_series(&BTreeSet::from(["sites".into()]));
    assert!(hidden.labels.is_empty());
    assert_eq!(
        hidden.marks.len(),
        g.links().len() + g.triangles().len() / 3
    );
    let graph2 = graph(&[[0., 0.], [11., 0.], [0., 20.], [5., 7.]])
        .diagram(&ChartConfig::default(), &options)
        .unwrap();
    assert_eq!(
        diagram.sites.iter().map(|s| &s.mark_id).collect::<Vec<_>>(),
        graph2.sites.iter().map(|s| &s.mark_id).collect::<Vec<_>>()
    );
    assert!(
        g.diagram(
            &ChartConfig::default(),
            &DelaunayDiagramOptions {
                site_radius: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn unchanged_triangle_ids_survive_insertion_before_existing_ids() {
    let original = vec![
        DelaunaySite::new("b", 0., 0.),
        DelaunaySite::new("c", 100., 0.),
        DelaunaySite::new("d", 0., 100.),
        DelaunaySite::new("e", 30., 35.),
    ];
    let options = DelaunayDiagramOptions {
        triangle_fill: Some(Color(10, 10, 10, 50)),
        ..Default::default()
    };
    let before = Delaunay::new(original.clone())
        .unwrap()
        .diagram(&ChartConfig::default(), &options)
        .unwrap();
    let mut added = original;
    added.push(DelaunaySite::new("a", 130., 130.));
    let after = Delaunay::new(added)
        .unwrap()
        .diagram(&ChartConfig::default(), &options)
        .unwrap();
    let old = before
        .scene
        .marks
        .iter()
        .filter(|m| m.series == "triangles")
        .map(|m| m.id.as_str())
        .collect::<BTreeSet<_>>();
    let new = after
        .scene
        .marks
        .iter()
        .filter(|m| m.series == "triangles" && !m.id.starts_with("delaunay:triangle:1:a:"))
        .map(|m| m.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        new.len(),
        2,
        "fixture retains two old triangles; the outer third is legitimately retriangulated"
    );
    assert!(
        new.is_subset(&old),
        "unchanged triangles must retain business identity despite index shifts"
    );
}

#[cfg(feature = "raster")]
#[test]
fn actual_cpu_site_and_link_ink_not_just_metadata() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let g = graph(&[[0., 0.], [100., 0.], [0., 100.], [100., 100.], [43., 56.]]);
    let d = g
        .diagram(
            &ChartConfig {
                width: 400.,
                height: 300.,
                ..Default::default()
            },
            &DelaunayDiagramOptions {
                site_color: Color(20, 80, 190, 255),
                link_color: Color(20, 20, 20, 255),
                link_width: 3.,
                ..Default::default()
            },
        )
        .unwrap();
    let image = render_raster(&d.scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    let pixel = |x: f64, y: f64| {
        let i = (y as usize * image.width as usize + x as usize) * 4;
        &image.rgba[i..i + 4]
    };
    for site in &d.sites {
        assert_eq!(pixel(site.x, site.y), [20, 80, 190, 255]);
    }
    let [a, b] = g.links()[0];
    let a = &d.sites[a];
    let b = &d.sites[b];
    let p = pixel((a.x + b.x) / 2., (a.y + b.y) / 2.);
    assert!(p[0] < 40 && p[1] < 40 && p[2] < 40);
}
