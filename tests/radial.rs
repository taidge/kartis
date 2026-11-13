use kartis::*;
use std::f64::consts::TAU;

fn values() -> Vec<RadialDatum> {
    vec![
        RadialDatum::new("A", 4.),
        RadialDatum::new("B", 3.),
        RadialDatum::new("C", 3.),
    ]
}
fn hierarchy_data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("a", Some("group".into()), 1.),
        HierarchyDatum::new("b", Some("group".into()), 3.),
        HierarchyDatum::new("c", Some("root".into()), 6.),
    ]
}
fn area(mark: &Mark) -> f64 {
    match mark.geometry {
        Geometry::Rect { width, height, .. } => width * height,
        _ => panic!("expected rectangle"),
    }
}
#[test]
fn radial_families_produce_valid_shared_scenes() {
    let c = ChartConfig::default();
    let d = values();
    let scenes = [
        pie(&c, &d, &PieOptions::default()).unwrap(),
        pie(
            &c,
            &d,
            &PieOptions {
                inner_radius: 0.55,
                ..Default::default()
            },
        )
        .unwrap(),
        radial_bar(&c, &d, &RadialBarOptions::default()).unwrap(),
        funnel(&c, &d, &FunnelOptions::default()).unwrap(),
        waffle(&c, &d, &WaffleOptions::default()).unwrap(),
        radar(
            &c,
            &["X".into(), "Y".into(), "Z".into()],
            &[RadarSeries::new("one", vec![1., 2., 3.])],
            &RadarOptions::default(),
        )
        .unwrap(),
    ];
    for scene in scenes {
        scene.validate().unwrap();
        assert!(!scene.marks.is_empty());
    }
}
#[test]
fn donut_hole_is_not_hit_and_full_arc_is_valid() {
    let c = ChartConfig::default();
    let scene = pie(
        &c,
        &[RadialDatum::new("only", 1.)],
        &PieOptions {
            inner_radius: 0.5,
            ..Default::default()
        },
    )
    .unwrap();
    let cx = c.margin.left + c.inner_width() / 2.;
    let cy = c.margin.top + c.inner_height() / 2.;
    let radius = c.inner_width().min(c.inner_height()) / 2.;
    assert!(scene.hit_test(cx, cy).is_none());
    assert_eq!(scene.hit_test(cx + radius * 0.8, cy).unwrap().id, "only");
    let zero = pie(&c, &[RadialDatum::new("zero", 0.)], &PieOptions::default()).unwrap();
    assert!(zero.marks.is_empty());
    assert_eq!(zero.legend.len(), 1);
}
#[test]
fn waffle_preserves_occupied_total_and_largest_remainders() {
    let c = ChartConfig::default();
    let d = vec![
        RadialDatum::new("a", 1.),
        RadialDatum::new("b", 1.),
        RadialDatum::new("c", 1.),
    ];
    let scene = waffle(
        &c,
        &d,
        &WaffleOptions {
            rows: 1,
            columns: 10,
            total: Some(4.),
            padding: 0.,
            round_cells: false,
        },
    )
    .unwrap();
    assert_eq!(scene.marks.iter().filter(|m| m.series == "a").count(), 3);
    assert_eq!(scene.marks.iter().filter(|m| m.series == "b").count(), 3);
    assert_eq!(scene.marks.iter().filter(|m| m.series == "c").count(), 2);
    assert_eq!(
        scene.marks.iter().filter(|m| m.series.is_empty()).count(),
        2
    );
}
#[test]
fn treemap_conserves_leaf_area_and_ratio() {
    let c = ChartConfig::default();
    let scene = treemap(
        &c,
        &hierarchy_data(),
        &HierarchyOptions {
            padding: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(scene.marks.len(), 3);
    let a = scene.marks.iter().find(|m| m.series == "a").unwrap();
    let b = scene.marks.iter().find(|m| m.series == "b").unwrap();
    assert!((area(b) / area(a) - 3.).abs() < 1e-10);
    assert!(
        (scene.marks.iter().map(area).sum::<f64>() - c.inner_width() * c.inner_height()).abs()
            < 1e-7
    );
}
#[test]
fn hierarchy_families_accept_unsorted_flat_data() {
    let c = ChartConfig::default();
    let mut data = hierarchy_data();
    data.reverse();
    for scene in [
        treemap(&c, &data, &Default::default()).unwrap(),
        sunburst(&c, &data, &Default::default()).unwrap(),
        icicle(&c, &data, &Default::default()).unwrap(),
        tree(&c, &data, &Default::default()).unwrap(),
        circle_packing(&c, &data, &Default::default()).unwrap(),
    ] {
        scene.validate().unwrap();
        assert!(!scene.marks.is_empty());
    }
}
#[test]
fn packing_siblings_do_not_overlap_and_children_stay_in_parent() {
    let c = ChartConfig::default();
    let data = hierarchy_data();
    let scene = circle_packing(
        &c,
        &data,
        &HierarchyOptions {
            padding: 0.,
            include_root: true,
            ..Default::default()
        },
    )
    .unwrap();
    let circle = |id: &str| {
        let mark = scene.marks.iter().find(|m| m.series == id).unwrap();
        if let Geometry::Circle { x, y, radius } = mark.geometry {
            (x, y, radius)
        } else {
            panic!()
        }
    };
    for d in &data {
        if let Some(parent) = &d.parent {
            let (x, y, r) = circle(&d.id);
            let (px, py, pr) = circle(parent);
            assert!((x - px).hypot(y - py) + r <= pr + 1e-8);
        }
    }
    for (a, b) in [("a", "b"), ("group", "c")] {
        let (x, y, r) = circle(a);
        let (xx, yy, rr) = circle(b);
        assert!((x - xx).hypot(y - yy) + 1e-8 >= r + rr);
    }
    let (_, _, a) = circle("a");
    let (_, _, b) = circle("b");
    assert!((b * b / (a * a) - 3.).abs() < 1e-10);
}
#[test]
fn hierarchy_cycles_missing_parents_and_internal_weight_reject() {
    let c = ChartConfig::default();
    let options = HierarchyOptions::default();
    let cycle = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("b".into()), 1.),
        HierarchyDatum::new("b", Some("a".into()), 0.),
    ];
    assert!(tree(&c, &cycle, &options).is_err());
    let missing = vec![HierarchyDatum::new("root", Some("absent".into()), 1.)];
    assert!(treemap(&c, &missing, &options).is_err());
    let mut weighted = hierarchy_data();
    weighted[0].value = 2.;
    assert!(sunburst(&c, &weighted, &options).is_err());
}
#[test]
fn sankey_widths_conserve_flow_and_cycles_reject() {
    let c = ChartConfig::default();
    let nodes = vec![FlowNode::new("a"), FlowNode::new("b"), FlowNode::new("c")];
    let links = vec![
        FlowLink {
            source: "a".into(),
            target: "b".into(),
            value: 3.,
        },
        FlowLink {
            source: "a".into(),
            target: "c".into(),
            value: 1.,
        },
    ];
    let scene = sankey(&c, &nodes, &links, &Default::default()).unwrap();
    scene.validate().unwrap();
    let heights: Vec<f64> = ["a", "b", "c"]
        .iter()
        .map(|id| {
            let m = scene
                .marks
                .iter()
                .find(|m| m.id == format!("node:{id}"))
                .unwrap();
            if let Geometry::Rect { height, .. } = m.geometry {
                height
            } else {
                panic!()
            }
        })
        .collect();
    assert!((heights[0] - heights[1] - heights[2]).abs() < 1e-8);
    let mut cyclic = links;
    cyclic.push(FlowLink {
        source: "b".into(),
        target: "a".into(),
        value: 1.,
    });
    assert!(sankey(&c, &nodes, &cyclic, &Default::default()).is_err());
}
#[test]
fn malformed_radial_options_and_values_return_errors() {
    let c = ChartConfig::default();
    for value in [-1., f64::NAN, f64::INFINITY] {
        assert!(pie(&c, &[RadialDatum::new("x", value)], &Default::default()).is_err());
    }
    let duplicate = vec![RadialDatum::new("x", 1.), RadialDatum::new("x", 2.)];
    assert!(waffle(&c, &duplicate, &Default::default()).is_err());
    assert!(
        pie(
            &c,
            &values(),
            &PieOptions {
                end_angle: TAU * 2.,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        radial_bar(
            &c,
            &values(),
            &RadialBarOptions {
                max_value: Some(1.),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        waffle(
            &c,
            &values(),
            &WaffleOptions {
                rows: usize::MAX,
                columns: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn fractional_auto_domains_fill_the_available_extent() {
    let c = ChartConfig::default();
    let d = [RadialDatum::new("fraction", 0.2)];
    let a = waffle(&c, &d, &WaffleOptions::default()).unwrap();
    assert!(a.marks.iter().all(|m| m.series == "fraction"));
    let bars = radial_bar(
        &c,
        &d,
        &RadialBarOptions {
            tracks: false,
            ..Default::default()
        },
    )
    .unwrap();
    let full = radial_bar(
        &c,
        &[RadialDatum::new("fraction", 1.)],
        &RadialBarOptions {
            tracks: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(bars.marks[0].geometry, full.marks[0].geometry);
    let axes = vec!["a".into(), "b".into(), "c".into()];
    let radar_small = radar(
        &c,
        &axes,
        &[RadarSeries::new("s", vec![0.2; 3])],
        &Default::default(),
    )
    .unwrap();
    let radar_full = radar(
        &c,
        &axes,
        &[RadarSeries::new("s", vec![1.; 3])],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        radar_small.marks.last().unwrap().geometry,
        radar_full.marks.last().unwrap().geometry
    );
    radial_bar(
        &c,
        &[RadialDatum::new("large", f64::MAX)],
        &Default::default(),
    )
    .unwrap();
    radar(
        &c,
        &axes,
        &[RadarSeries::new("large", vec![f64::MAX; 3])],
        &Default::default(),
    )
    .unwrap();
}

#[test]
fn treemap_tiles_conserve_area_and_squarify_improves_aspect_ratios() {
    let c = ChartConfig {
        width: 600.,
        height: 400.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    };
    let weights = [6., 6., 4., 3., 2., 2., 1.];
    let mut data = vec![HierarchyDatum::new("root", None, 0.)];
    data.extend(
        weights
            .iter()
            .enumerate()
            .map(|(i, v)| HierarchyDatum::new(format!("n{i}"), Some("root".into()), *v)),
    );
    let options = HierarchyOptions {
        padding: 0.,
        labels: false,
        ..Default::default()
    };
    let mut worst = vec![];
    for tile in [
        TreemapTile::Binary,
        TreemapTile::Squarify,
        TreemapTile::Slice,
        TreemapTile::Dice,
        TreemapTile::SliceDice,
    ] {
        let scene = treemap_with_tile(&c, &data, &options, tile).unwrap();
        assert_eq!(scene, treemap_with_tile(&c, &data, &options, tile).unwrap());
        assert!((scene.marks.iter().map(area).sum::<f64>() - c.width * c.height).abs() < 1e-7);
        let mut aspect = 0f64;
        for (a, m) in scene.marks.iter().enumerate() {
            let Geometry::Rect {
                x,
                y,
                width,
                height,
            } = m.geometry
            else {
                panic!()
            };
            assert!((area(m) / (c.width * c.height) - m.value / 24.).abs() < 1e-10);
            assert!(
                x >= -1e-9
                    && y >= -1e-9
                    && x + width <= c.width + 1e-9
                    && y + height <= c.height + 1e-9
            );
            aspect = aspect.max((width / height).max(height / width));
            for other in &scene.marks[a + 1..] {
                let Geometry::Rect {
                    x: xx,
                    y: yy,
                    width: ww,
                    height: hh,
                } = other.geometry
                else {
                    panic!()
                };
                assert!(
                    x + width <= xx + 1e-8
                        || xx + ww <= x + 1e-8
                        || y + height <= yy + 1e-8
                        || yy + hh <= y + 1e-8
                );
            }
        }
        worst.push(aspect);
    }
    assert!(
        worst[1] < worst[2] * 0.5,
        "squarify {} vs slice {}",
        worst[1],
        worst[2]
    );
}
fn tree_fixture() -> Vec<HierarchyDatum> {
    let relationships = [
        ("r", None),
        ("a", Some("r")),
        ("b", Some("r")),
        ("c", Some("r")),
        ("a1", Some("a")),
        ("a2", Some("a")),
        ("a21", Some("a2")),
        ("b1", Some("b")),
        ("b11", Some("b1")),
        ("b12", Some("b1")),
        ("c1", Some("c")),
    ];
    relationships
        .iter()
        .map(|(id, parent)| {
            let internal = relationships.iter().any(|(_, p)| *p == Some(*id));
            HierarchyDatum::new(
                *id,
                parent.map(str::to_owned),
                if internal { 0. } else { 1. },
            )
        })
        .collect()
}
fn disk(scene: &Scene, id: &str) -> (f64, f64, f64) {
    let m = scene
        .marks
        .iter()
        .find(|m| m.id == format!("node:{id}"))
        .unwrap();
    let Geometry::Circle { x, y, radius } = m.geometry else {
        panic!()
    };
    (x, y, radius)
}
#[test]
fn tidy_tree_centers_parents_mirrors_subtrees_and_supports_all_orientations() {
    let c = ChartConfig::default();
    let data = tree_fixture();
    let options = TreeOptions::default();
    let tidy = tree_with_options(&c, &data, &options).unwrap();
    let mut mirror_data = data.clone();
    mirror_data.reverse();
    let mirror = tree_with_options(&c, &mirror_data, &options).unwrap();
    for d in &data {
        let (_, y, _) = disk(&tidy, &d.id);
        let (_, my, _) = disk(&mirror, &d.id);
        assert!(
            (y + my - (2. * c.margin.top + c.inner_height())).abs() < 1e-8,
            "mirror mismatch {}",
            d.id
        );
        let children: Vec<_> = data
            .iter()
            .filter(|child| child.parent.as_ref() == Some(&d.id))
            .collect();
        if !children.is_empty() {
            let (_, first, _) = disk(&tidy, &children.first().unwrap().id);
            let (_, last, _) = disk(&tidy, &children.last().unwrap().id);
            assert!((y - (first + last) / 2.).abs() < 1e-8);
        }
    }
    let cluster = tree_with_options(
        &c,
        &data,
        &TreeOptions {
            mode: TreeMode::Cluster,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        data.iter()
            .any(|d| (disk(&tidy, &d.id).1 - disk(&cluster, &d.id).1).abs() > 1.)
    );
    for orientation in [
        TreeOrientation::LeftToRight,
        TreeOrientation::RightToLeft,
        TreeOrientation::TopToBottom,
        TreeOrientation::BottomToTop,
        TreeOrientation::Radial,
    ] {
        let s = tree_with_options(
            &c,
            &data,
            &TreeOptions {
                orientation,
                ..Default::default()
            },
        )
        .unwrap();
        s.validate().unwrap();
        for d in &data {
            let (x, y, r) = disk(&s, &d.id);
            assert!(
                x - r >= c.margin.left - 1e-8
                    && x + r <= c.width - c.margin.right + 1e-8
                    && y - r >= c.margin.top - 1e-8
                    && y + r <= c.height - c.margin.bottom + 1e-8
            );
        }
        let (x, y, _) = disk(&s, "r");
        if orientation == TreeOrientation::Radial {
            assert!((x - c.margin.left - c.inner_width() / 2.).abs() < 1e-8);
            assert!((y - c.margin.top - c.inner_height() / 2.).abs() < 1e-8);
        }
    }
}
#[test]
fn tidy_threads_handle_large_unbalanced_and_multisibling_trees() {
    let c = ChartConfig::default();
    for seed in 0..12 {
        let mut data: Vec<HierarchyDatum> = (0..150)
            .map(|i| {
                HierarchyDatum::new(
                    format!("n{i}"),
                    if i == 0 {
                        None
                    } else {
                        Some(format!("n{}", (i * 37 + seed * 19) % i))
                    },
                    1.,
                )
            })
            .collect();
        let parents: std::collections::BTreeSet<_> =
            data.iter().filter_map(|d| d.parent.clone()).collect();
        for d in &mut data {
            if parents.contains(&d.id) {
                d.value = 0.;
            }
        }
        tree_with_options(&c, &data, &TreeOptions::default()).unwrap();
    }
}
#[test]
fn front_chain_is_dense_nonoverlapping_and_preserves_leaf_area_ratios() {
    let c = ChartConfig {
        width: 600.,
        height: 600.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    };
    let mut data = vec![HierarchyDatum::new("root", None, 0.)];
    data.extend((0..64).map(|i| HierarchyDatum::new(format!("n{i}"), Some("root".into()), 1.)));
    let options = HierarchyOptions {
        padding: 0.,
        labels: false,
        include_root: true,
    };
    let front = circle_packing(&c, &data, &options).unwrap();
    let shelf =
        circle_packing_with_algorithm(&c, &data, &options, CirclePackingAlgorithm::Shelf).unwrap();
    assert_eq!(front, circle_packing(&c, &data, &options).unwrap());
    let (_, _, fr) = disk(&front, "n0");
    let (_, _, sr) = disk(&shelf, "n0");
    assert!(
        fr * fr > sr * sr * 1.15,
        "front-chain occupancy {} vs shelf {}",
        fr * fr * 64. / 90000.,
        sr * sr * 64. / 90000.
    );
    let (px, py, pr) = disk(&front, "root");
    for i in 0..64 {
        let (x, y, r) = disk(&front, &format!("n{i}"));
        assert!((x - px).hypot(y - py) + r <= pr + 1e-7);
        for j in i + 1..64 {
            let (xx, yy, rr) = disk(&front, &format!("n{j}"));
            assert!((x - xx).hypot(y - yy) + 1e-7 >= r + rr, "overlap {i} {j}");
        }
    }
    for (i, datum) in data.iter_mut().enumerate().skip(1) {
        datum.value = ((i * 37) % 97 + 1) as f64;
    }
    let varying = circle_packing(&c, &data, &options).unwrap();
    for i in 1..data.len() {
        let (x, y, r) = disk(&varying, &data[i].id);
        for d in &data[i + 1..] {
            let (xx, yy, rr) = disk(&varying, &d.id);
            assert!((x - xx).hypot(y - yy) + 1e-7 >= r + rr);
        }
        let (_, _, first) = disk(&varying, &data[1].id);
        assert!((r * r / first.powi(2) - data[i].value / data[1].value).abs() < 1e-8);
    }
}
fn flow_crossings(scene: &Scene, links: &[FlowLink]) -> usize {
    let center = |id: &str| {
        let m = scene
            .marks
            .iter()
            .find(|m| m.id == format!("node:{id}"))
            .unwrap();
        let Geometry::Rect { x, y, height, .. } = m.geometry else {
            panic!()
        };
        (x, y + height / 2.)
    };
    let mut count = 0;
    for (i, a) in links.iter().enumerate() {
        let (sa, ta) = (center(&a.source), center(&a.target));
        for b in &links[i + 1..] {
            let (sb, tb) = (center(&b.source), center(&b.target));
            if sa.0 == sb.0 && ta.0 == tb.0 && (sa.1 - sb.1) * (ta.1 - tb.1) < 0. {
                count += 1;
            }
        }
    }
    count
}
#[test]
fn sankey_relaxation_removes_crossings_without_losing_flow_or_colliding() {
    let c = ChartConfig::default();
    let ids = ["a", "b", "c", "d", "w", "x", "y", "z"];
    let nodes: Vec<_> = ids.iter().map(|id| FlowNode::new(*id)).collect();
    let links: Vec<_> = (0..4)
        .map(|i| FlowLink {
            source: ids[i].into(),
            target: ids[7 - i].into(),
            value: 1.,
        })
        .collect();
    let options = SankeyOptions::default();
    let initial = sankey_with_iterations(&c, &nodes, &links, &options, 0).unwrap();
    let optimized = sankey_with_iterations(&c, &nodes, &links, &options, 8).unwrap();
    assert_eq!(flow_crossings(&initial, &links), 6);
    assert_eq!(flow_crossings(&optimized, &links), 0);
    assert_eq!(
        optimized,
        sankey_with_iterations(&c, &nodes, &links, &options, 8).unwrap()
    );
    for m in optimized.marks.iter().filter(|m| m.id.starts_with("node:")) {
        let Geometry::Rect { x, y, height, .. } = m.geometry else {
            panic!()
        };
        assert!(y >= c.margin.top - 1e-8 && y + height <= c.height - c.margin.bottom + 1e-8);
        for n in optimized
            .marks
            .iter()
            .filter(|n| n.id.starts_with("node:") && n.id != m.id)
        {
            let Geometry::Rect {
                x: xx,
                y: yy,
                height: hh,
                ..
            } = n.geometry
            else {
                panic!()
            };
            if x == xx {
                assert!(
                    y + height + options.node_padding <= yy + 1e-8
                        || yy + hh + options.node_padding <= y + 1e-8
                );
            }
        }
    }
    assert!(sankey_with_iterations(&c, &nodes, &links, &options, 129).is_err());
    let tiny = [FlowLink {
        source: "a".into(),
        target: "b".into(),
        value: f64::from_bits(1),
    }];
    let scene = sankey_with_iterations(&c, &nodes[..2], &tiny, &options, 4).unwrap();
    let Geometry::Rect { height, .. } = scene
        .marks
        .iter()
        .find(|m| m.id == "node:a")
        .unwrap()
        .geometry
    else {
        panic!()
    };
    assert!((height - c.inner_height()).abs() < 1e-8);
}

#[test]
fn randomized_front_chain_hierarchies_keep_every_sibling_disjoint() {
    let c = ChartConfig::default();
    let options = HierarchyOptions {
        padding: 0.5,
        labels: false,
        include_root: true,
    };
    for seed in 1..9 {
        let mut data = vec![HierarchyDatum::new("root", None, 0.)];
        for group in 0..7 {
            let parent = format!("g{group}");
            data.push(HierarchyDatum::new(parent.clone(), Some("root".into()), 0.));
            for j in 0..19 {
                let value = (((j * 193 + group * 61 + seed * 37) % 997) + 1) as f64;
                data.push(HierarchyDatum::new(
                    format!("n{group}_{j}"),
                    Some(parent.clone()),
                    value,
                ));
            }
        }
        let scene = circle_packing(&c, &data, &options).unwrap();
        for (i, d) in data.iter().enumerate() {
            let Some(parent) = &d.parent else {
                continue;
            };
            let (x, y, r) = disk(&scene, &d.id);
            let (px, py, pr) = disk(&scene, parent);
            assert!((x - px).hypot(y - py) + r <= pr + 1e-7);
            for other in data[i + 1..]
                .iter()
                .filter(|other| other.parent == d.parent)
            {
                let (xx, yy, rr) = disk(&scene, &other.id);
                assert!(
                    (x - xx).hypot(y - yy) + 1e-7 >= r + rr,
                    "seed {seed}: {} and {}",
                    d.id,
                    other.id
                );
            }
        }
    }
}
#[test]
fn sankey_best_layout_never_increases_layer_pair_crossings() {
    let c = ChartConfig::default();
    let options = SankeyOptions {
        node_padding: 3.,
        ..Default::default()
    };
    let nodes: Vec<_> = (0..12).map(|i| FlowNode::new(format!("n{i}"))).collect();
    for seed in 0..10 {
        let links: Vec<_> = (0..24)
            .map(|j| FlowLink {
                source: format!("n{}", j % 6),
                target: format!("n{}", 6 + (j * 7 + seed * 3 + j / 6) % 6),
                value: ((j * 7 + seed) % 9 + 1) as f64,
            })
            .collect();
        let initial = sankey_with_iterations(&c, &nodes, &links, &options, 0).unwrap();
        let optimized = sankey_with_iterations(&c, &nodes, &links, &options, 16).unwrap();
        assert!(flow_crossings(&optimized, &links) <= flow_crossings(&initial, &links));
    }
}

fn assert_enclosure(circle: PackingCircle, disks: &[PackingCircle], tolerance: f64) {
    for disk in disks {
        assert!(
            (circle.x - disk.x).hypot(circle.y - disk.y) + disk.radius <= circle.radius + tolerance,
            "disk outside enclosure: {disk:?} in {circle:?}"
        );
    }
}
#[test]
fn exact_enclosure_matches_analytic_one_two_and_three_disk_geometry() {
    let single = [PackingCircle {
        x: 3.,
        y: 4.,
        radius: 2.,
    }];
    assert_eq!(enclosing_circle(&single).unwrap(), Some(single[0]));
    let pair = [
        PackingCircle {
            x: -3.,
            y: 0.,
            radius: 1.,
        },
        PackingCircle {
            x: 3.,
            y: 0.,
            radius: 2.,
        },
    ];
    let circle = enclosing_circle(&pair).unwrap().unwrap();
    assert!(
        (circle.x - 0.5).abs() < 1e-12
            && circle.y.abs() < 1e-12
            && (circle.radius - 4.5).abs() < 1e-12
    );
    let triangle = [
        PackingCircle {
            x: 0.,
            y: 0.,
            radius: 1.,
        },
        PackingCircle {
            x: 2.,
            y: 0.,
            radius: 1.,
        },
        PackingCircle {
            x: 1.,
            y: 3f64.sqrt(),
            radius: 1.,
        },
    ];
    let circle = enclosing_circle(&triangle).unwrap().unwrap();
    assert!(
        (circle.x - 1.).abs() < 1e-12
            && (circle.y - 1. / 3f64.sqrt()).abs() < 1e-12
            && (circle.radius - (1. + 2. / 3f64.sqrt())).abs() < 1e-12
    );
    let weighted = [
        PackingCircle {
            x: 4.,
            y: 0.,
            radius: 1.,
        },
        PackingCircle {
            x: -1.5,
            y: 3. * 3f64.sqrt() / 2.,
            radius: 2.,
        },
        PackingCircle {
            x: -1.,
            y: -3f64.sqrt(),
            radius: 3.,
        },
    ];
    let circle = enclosing_circle(&weighted).unwrap().unwrap();
    assert!(circle.x.hypot(circle.y) < 1e-11 && (circle.radius - 5.).abs() < 1e-11);
    assert_enclosure(circle, &weighted, 1e-12);
}
#[test]
fn exact_enclosure_handles_containment_coincidence_collinearity_and_scale() {
    let nested = [
        PackingCircle {
            x: 10.,
            y: -10.,
            radius: 10.,
        },
        PackingCircle {
            x: 10.,
            y: -10.,
            radius: 0.,
        },
        PackingCircle {
            x: 12.,
            y: -10.,
            radius: 1.,
        },
    ];
    assert_eq!(enclosing_circle(&nested).unwrap(), Some(nested[0]));
    let collinear = [
        PackingCircle {
            x: -2.,
            y: 1e-14,
            radius: 1.,
        },
        PackingCircle {
            x: 0.,
            y: 0.,
            radius: 0.1,
        },
        PackingCircle {
            x: 4.,
            y: 0.,
            radius: 2.,
        },
    ];
    let circle = enclosing_circle(&collinear).unwrap().unwrap();
    assert!((circle.x - 1.5).abs() < 1e-10 && (circle.radius - 4.5).abs() < 1e-10);
    assert_enclosure(circle, &collinear, 1e-12);
    for scale in [1e-200, 1e-100, 1., 1e5] {
        let disks = [
            PackingCircle {
                x: -3. * scale,
                y: 0.,
                radius: scale,
            },
            PackingCircle {
                x: 3. * scale,
                y: 0.,
                radius: 2. * scale,
            },
        ];
        let circle = enclosing_circle(&disks)
            .unwrap_or_else(|error| panic!("scale {scale}: {error}"))
            .unwrap();
        assert!(
            (circle.x / scale - 0.5).abs() < 1e-10 && (circle.radius / scale - 4.5).abs() < 1e-10
        );
    }
    let translated = [
        PackingCircle {
            x: 999997.,
            y: 999999.,
            radius: 1.,
        },
        PackingCircle {
            x: 999991.,
            y: 999999.,
            radius: 2.,
        },
    ];
    let circle = enclosing_circle(&translated).unwrap().unwrap();
    assert!((circle.x - 999993.5).abs() < 1e-8 && (circle.radius - 4.5).abs() < 1e-8);
    assert_enclosure(circle, &translated, 1e-10);
    assert!(enclosing_circle(&[]).unwrap().is_none());
    assert!(
        enclosing_circle(&[PackingCircle {
            x: 0.,
            y: 0.,
            radius: f64::NAN
        }])
        .is_err()
    );
    assert!(
        enclosing_circle(&vec![
            PackingCircle {
                x: 0.,
                y: 0.,
                radius: 1.
            };
            2001
        ])
        .is_err()
    );
}
#[test]
fn exact_enclosure_is_permutation_stable_and_has_a_convex_minimality_certificate() {
    for seed in 0..24 {
        let disks: Vec<_> = (0..40)
            .map(|i| PackingCircle {
                x: ((i * 7919 + seed * 977) % 997) as f64 - 500.,
                y: ((i * 6151 + seed * 613) % 991) as f64 - 500.,
                radius: ((i * 71 + seed * 23) % 47) as f64,
            })
            .collect();
        let circle = enclosing_circle(&disks).unwrap().unwrap();
        assert_enclosure(circle, &disks, 1e-8);
        let mut reversed = disks.clone();
        reversed.reverse();
        assert_eq!(enclosing_circle(&reversed).unwrap(), Some(circle));
        // Independent optimality certificate: 0 belongs to the convex hull of
        // active distance gradients, so no center displacement can reduce R.
        let mut angles: Vec<_> = disks
            .iter()
            .filter_map(|d| {
                let distance = (circle.x - d.x).hypot(circle.y - d.y);
                ((distance + d.radius - circle.radius).abs() < 1e-7 && distance > 1e-8)
                    .then_some((circle.y - d.y).atan2(circle.x - d.x))
            })
            .collect();
        angles.sort_by(f64::total_cmp);
        assert!(angles.len() >= 2);
        let maxgap = angles
            .windows(2)
            .map(|a| a[1] - a[0])
            .chain(std::iter::once(
                angles[0] + std::f64::consts::TAU - angles[angles.len() - 1],
            ))
            .fold(0., f64::max);
        assert!(
            maxgap <= std::f64::consts::PI + 1e-7,
            "minimality failed for seed {seed}: gap {maxgap}"
        );
    }
}
#[test]
fn exact_enclosure_handles_many_boundary_constraints_with_known_optimum() {
    let disks: Vec<_> = (0..2000)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / 2000.;
            let radius = ((i * 71) % 25 + 1) as f64;
            PackingCircle {
                x: (50. - radius) * angle.cos(),
                y: (50. - radius) * angle.sin(),
                radius,
            }
        })
        .collect();
    let circle = enclosing_circle(&disks).unwrap().unwrap();
    assert!(circle.x.hypot(circle.y) < 1e-8 && (circle.radius - 50.).abs() < 1e-8);
    assert_enclosure(circle, &disks, 1e-9);
}
#[test]
fn front_chain_exact_enclosure_preserves_identity_under_flat_input_permutation() {
    let c = ChartConfig::default();
    let mut data = vec![HierarchyDatum::new("root", None, 0.)];
    for g in 0..5 {
        let group = format!("g{g}");
        data.push(HierarchyDatum::new(group.clone(), Some("root".into()), 0.));
        for i in 0..7 {
            data.push(HierarchyDatum::new(
                format!("{group}-{i}"),
                Some(group.clone()),
                (i % 3 + 1) as f64,
            ));
        }
    }
    let options = HierarchyOptions {
        padding: 0.,
        labels: false,
        include_root: true,
    };
    let original = circle_packing(&c, &data, &options).unwrap();
    data.reverse();
    let permuted = circle_packing(&c, &data, &options).unwrap();
    assert_eq!(original.marks.len(), permuted.marks.len());
    for d in &data {
        let (x, y, r) = disk(&original, &d.id);
        let (xx, yy, rr) = disk(&permuted, &d.id);
        assert!(
            (x - xx).hypot(y - yy) < 1e-8 && (r - rr).abs() < 1e-8,
            "permutation changed {}",
            d.id
        );
    }
}
fn node_column(
    scene: &Scene,
    id: &str,
    columns: usize,
    o: &SankeyOptions,
    c: &ChartConfig,
) -> usize {
    let m = scene
        .marks
        .iter()
        .find(|m| m.id == format!("node:{id}"))
        .unwrap();
    let Geometry::Rect { x, .. } = m.geometry else {
        panic!()
    };
    if columns <= 1 {
        0
    } else {
        ((x - c.margin.left) / (c.inner_width() - o.node_width) * (columns - 1) as f64).round()
            as usize
    }
}
#[test]
fn sankey_alignments_use_source_depth_sink_height_and_dangling_path_rules() {
    let c = ChartConfig::default();
    let ids = ["a", "b", "c", "d", "u", "leaf", "v", "w", "solo"];
    let nodes: Vec<_> = ids.iter().map(|id| FlowNode::new(*id)).collect();
    let pairs = [
        ("a", "b"),
        ("b", "c"),
        ("c", "d"),
        ("u", "c"),
        ("b", "leaf"),
        ("v", "w"),
    ];
    let links: Vec<_> = pairs
        .iter()
        .map(|(a, b)| FlowLink {
            source: (*a).into(),
            target: (*b).into(),
            value: 1.,
        })
        .collect();
    let expectations = [
        (SankeyAlignment::Left, [0, 1, 2, 3, 0, 2, 0, 1, 0]),
        (SankeyAlignment::Right, [0, 1, 2, 3, 1, 3, 2, 3, 3]),
        (SankeyAlignment::Center, [0, 1, 2, 3, 1, 2, 0, 1, 0]),
        (SankeyAlignment::Justify, [0, 1, 2, 3, 0, 3, 0, 3, 3]),
    ];
    for (alignment, expected) in expectations {
        for iterations in [0, 8] {
            let options = SankeyLayoutOptions {
                alignment,
                iterations,
                ..Default::default()
            };
            let scene = sankey_with_layout(&c, &nodes, &links, &options).unwrap();
            assert_eq!(
                ids.iter()
                    .map(|id| node_column(&scene, id, 4, &options.appearance, &c))
                    .collect::<Vec<_>>(),
                expected
            );
            for link in &links {
                assert!(
                    node_column(&scene, &link.source, 4, &options.appearance, &c)
                        < node_column(&scene, &link.target, 4, &options.appearance, &c)
                );
            }
            scene.validate().unwrap();
        }
    }
}
#[test]
fn sankey_alignment_is_topological_for_zero_flow_and_input_permutations() {
    let c = ChartConfig::default();
    let mut nodes: Vec<_> = ["a", "b", "c", "d", "source", "sink", "isolated"]
        .into_iter()
        .map(FlowNode::new)
        .collect();
    let links: Vec<_> = [
        ("a", "b"),
        ("b", "c"),
        ("c", "d"),
        ("source", "c"),
        ("b", "sink"),
    ]
    .into_iter()
    .map(|(a, b)| FlowLink {
        source: a.into(),
        target: b.into(),
        value: 0.,
    })
    .collect();
    for alignment in [
        SankeyAlignment::Left,
        SankeyAlignment::Right,
        SankeyAlignment::Center,
        SankeyAlignment::Justify,
    ] {
        let options = SankeyLayoutOptions {
            alignment,
            ..Default::default()
        };
        let scene = sankey_with_layout(&c, &nodes, &links, &options).unwrap();
        nodes.reverse();
        let reversed = sankey_with_layout(&c, &nodes, &links, &options).unwrap();
        for node in &nodes {
            assert_eq!(
                node_column(&scene, &node.id, 4, &options.appearance, &c),
                node_column(&reversed, &node.id, 4, &options.appearance, &c)
            );
        }
        if alignment == SankeyAlignment::Center {
            assert_eq!(node_column(&scene, "source", 4, &options.appearance, &c), 1);
        }
        let a: std::collections::BTreeSet<_> = scene.marks.iter().map(|m| &m.id).collect();
        let b: std::collections::BTreeSet<_> = reversed.marks.iter().map(|m| &m.id).collect();
        assert_eq!(a, b);
    }
}
#[test]
fn sankey_alignment_preserves_best_crossing_count_and_vertical_nonoverlap() {
    let c = ChartConfig::default();
    let nodes: Vec<_> = (0..14).map(|i| FlowNode::new(format!("n{i}"))).collect();
    let mut links: Vec<_> = (0..24)
        .map(|i| FlowLink {
            source: format!("n{}", i % 6),
            target: format!("n{}", 6 + (i * 7 + i / 6) % 6),
            value: (i % 5 + 1) as f64,
        })
        .collect();
    links.push(FlowLink {
        source: "n12".into(),
        target: "n0".into(),
        value: 3.,
    });
    for alignment in [
        SankeyAlignment::Left,
        SankeyAlignment::Right,
        SankeyAlignment::Center,
        SankeyAlignment::Justify,
    ] {
        let appearance = SankeyOptions {
            node_padding: 4.,
            ..Default::default()
        };
        let initial = sankey_with_layout(
            &c,
            &nodes,
            &links,
            &SankeyLayoutOptions {
                appearance: appearance.clone(),
                alignment,
                iterations: 0,
            },
        )
        .unwrap();
        let optimized = sankey_with_layout(
            &c,
            &nodes,
            &links,
            &SankeyLayoutOptions {
                appearance: appearance.clone(),
                alignment,
                iterations: 16,
            },
        )
        .unwrap();
        assert!(flow_crossings(&optimized, &links) <= flow_crossings(&initial, &links));
        let rects: Vec<_> = optimized
            .marks
            .iter()
            .filter(|m| m.id.starts_with("node:"))
            .collect();
        for (i, a) in rects.iter().enumerate() {
            let Geometry::Rect { x, y, height, .. } = a.geometry else {
                panic!()
            };
            assert!(y >= c.margin.top - 1e-8 && y + height <= c.height - c.margin.bottom + 1e-8);
            for b in &rects[i + 1..] {
                let Geometry::Rect {
                    x: xx,
                    y: yy,
                    height: hh,
                    ..
                } = b.geometry
                else {
                    panic!()
                };
                if x == xx {
                    assert!(
                        y + height + appearance.node_padding <= yy + 1e-8
                            || yy + hh + appearance.node_padding <= y + 1e-8
                    );
                }
            }
        }
    }
}

#[test]
fn exact_enclosure_handles_nearly_degenerate_weighted_supports() {
    for exponent in 1..15 {
        let epsilon = 10_f64.powi(-exponent);
        let disks = [
            PackingCircle {
                x: 4.,
                y: 0.,
                radius: 1.,
            },
            PackingCircle {
                x: -3. * epsilon.cos(),
                y: 3. * epsilon.sin(),
                radius: 2.,
            },
            PackingCircle {
                x: -2. * epsilon.cos(),
                y: -2. * epsilon.sin(),
                radius: 3.,
            },
        ];
        let circle = enclosing_circle(&disks)
            .unwrap_or_else(|error| panic!("epsilon {epsilon}: {error}"))
            .unwrap();
        assert!(
            (circle.radius - 5.).abs() < 1e-8,
            "near-degenerate support epsilon {epsilon}: {circle:?}"
        );
        assert_enclosure(circle, &disks, 1e-10);
    }
}
