use std::collections::{BTreeMap, BTreeSet};

use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 400.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    }
}
fn options() -> HierarchyOptions {
    HierarchyOptions {
        padding: 0.,
        labels: false,
        ..Default::default()
    }
}
fn flat(weights: &[f64]) -> Vec<HierarchyDatum> {
    let mut data = vec![HierarchyDatum::new("root", None, 0.)];
    data.extend(
        weights
            .iter()
            .enumerate()
            .map(|(i, value)| HierarchyDatum::new(format!("n{i}"), Some("root".into()), *value)),
    );
    data
}
fn geometries(scene: &Scene) -> BTreeMap<&str, &Geometry> {
    scene
        .marks
        .iter()
        .map(|mark| (mark.id.as_str(), &mark.geometry))
        .collect()
}
fn assert_areas(scene: &Scene, c: &ChartConfig) {
    let total = scene.marks.iter().map(|mark| mark.value).sum::<f64>();
    let plot = c.inner_width() * c.inner_height();
    let mut area = 0.;
    for (i, mark) in scene.marks.iter().enumerate() {
        let Geometry::Rect {
            x,
            y,
            width,
            height,
        } = mark.geometry
        else {
            panic!()
        };
        assert!(
            x >= c.margin.left - 1e-8
                && y >= c.margin.top - 1e-8
                && x + width <= c.width - c.margin.right + 1e-8
                && y + height <= c.height - c.margin.bottom + 1e-8
        );
        if total > 0. {
            assert!((width * height / plot - mark.value / total).abs() < 1e-10);
        }
        area += width * height;
        for other in &scene.marks[i + 1..] {
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
    if total > 0. {
        assert!((area - plot).abs() < 1e-6);
    }
    scene.validate().unwrap();
}
#[test]
fn changing_values_reuses_real_row_topology_instead_of_running_squarify_again() {
    let c = config();
    let o = options();
    let mut layout = Resquarify::default();
    let initial = layout
        .layout(&c, &flat(&[6., 5., 4., 3., 2., 1.]), &o)
        .unwrap();
    let rows = layout.rows("root").unwrap().to_vec();
    assert_eq!(
        layout.last_stats(),
        ResquarifyStats {
            reused_parents: 0,
            rebuilt_parents: 1
        }
    );
    let data = flat(&[1., 2., 4., 20., 3., 5.]);
    let updated = layout.layout(&c, &data, &o).unwrap();
    assert_eq!(layout.rows("root").unwrap(), rows);
    assert_eq!(
        layout.last_stats(),
        ResquarifyStats {
            reused_parents: 1,
            rebuilt_parents: 0
        }
    );
    let mut fresh = Resquarify::default();
    let fresh_scene = fresh.layout(&c, &data, &o).unwrap();
    assert_ne!(fresh.rows("root").unwrap(), rows);
    assert_ne!(geometries(&updated), geometries(&fresh_scene));
    assert_ne!(geometries(&initial), geometries(&updated));
    assert_areas(&updated, &c);
    let ids: BTreeSet<_> = rows.iter().flat_map(|row| row.children.iter()).collect();
    assert_eq!(ids.len(), data.len() - 1);
    assert_eq!(layout.cached_children(), data.len() - 1);
}
#[test]
fn container_resize_keeps_cached_strip_orientations_and_exact_area_allocations() {
    let c = config();
    let data = flat(&[6., 5., 4., 3., 2., 1.]);
    let mut layout = Resquarify::new(1.).unwrap();
    layout.layout(&c, &data, &options()).unwrap();
    let rows = layout.rows("root").unwrap().to_vec();
    let resized = ChartConfig {
        width: 250.,
        height: 900.,
        ..c
    };
    let scene = layout.layout(&resized, &data, &options()).unwrap();
    assert_eq!(layout.rows("root").unwrap(), rows);
    assert_eq!(layout.last_stats().reused_parents, 1);
    assert_areas(&scene, &resized);
    let mut fresh = Resquarify::new(1.).unwrap();
    fresh.layout(&resized, &data, &options()).unwrap();
    assert_ne!(fresh.rows("root").unwrap(), rows);
}
#[test]
fn target_ratio_changes_initial_row_objective_and_invalidates_only_when_changed() {
    let c = config();
    let data = flat(&[1.; 6]);
    let mut square = Resquarify::new(1.).unwrap();
    square.layout(&c, &data, &options()).unwrap();
    assert_eq!(square.rows("root").unwrap()[0].children.len(), 2);
    let before = square.clone();
    square.set_ratio(1.).unwrap();
    assert_eq!(square, before);
    let mut wide = Resquarify::new(4.).unwrap();
    let wide_scene = wide.layout(&c, &data, &options()).unwrap();
    assert_eq!(wide.rows("root").unwrap()[0].children.len(), 4);
    assert_ne!(square.rows("root"), wide.rows("root"));
    assert_eq!(
        squarified_treemap(&c, &data, &options(), 4.).unwrap(),
        wide_scene
    );
    square.set_ratio(4.).unwrap();
    assert_eq!(square.cached_parents(), 0);
    assert_eq!(square.last_stats(), ResquarifyStats::default());
    let rebuilt = square.layout(&c, &data, &options()).unwrap();
    assert_eq!(rebuilt, wide_scene);
    assert_eq!(square.rows("root"), wide.rows("root"));
}
fn hierarchy_data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("g1", Some("root".into()), 0.),
        HierarchyDatum::new("g2", Some("root".into()), 0.),
        HierarchyDatum::new("a", Some("g1".into()), 2.),
        HierarchyDatum::new("b", Some("g1".into()), 3.),
        HierarchyDatum::new("c", Some("g2".into()), 4.),
        HierarchyDatum::new("d", Some("g2".into()), 5.),
    ]
}
#[test]
fn only_changed_direct_child_sets_rebuild_and_removed_parents_are_evicted() {
    let c = config();
    let o = options();
    let mut data = hierarchy_data();
    let mut layout = Resquarify::default();
    layout.layout(&c, &data, &o).unwrap();
    assert_eq!(layout.cached_parents(), 3);
    assert_eq!(layout.last_stats().rebuilt_parents, 3);
    let root = layout.rows("root").unwrap().to_vec();
    let g2 = layout.rows("g2").unwrap().to_vec();
    data[3].value = 11.;
    layout.layout(&c, &data, &o).unwrap();
    assert_eq!(layout.last_stats().reused_parents, 3);
    data.push(HierarchyDatum::new("e", Some("g1".into()), 1.));
    layout.layout(&c, &data, &o).unwrap();
    assert_eq!(
        layout.last_stats(),
        ResquarifyStats {
            reused_parents: 2,
            rebuilt_parents: 1
        }
    );
    assert_eq!(layout.rows("root").unwrap(), root);
    assert_eq!(layout.rows("g2").unwrap(), g2);
    data.iter_mut().find(|node| node.id == "b").unwrap().parent = Some("g2".into());
    layout.layout(&c, &data, &o).unwrap();
    assert_eq!(
        layout.last_stats(),
        ResquarifyStats {
            reused_parents: 1,
            rebuilt_parents: 2
        }
    );
    data.retain(|node| !["g2", "b", "c", "d"].contains(&node.id.as_str()));
    let scene = layout.layout(&c, &data, &o).unwrap();
    assert_eq!(
        layout.last_stats(),
        ResquarifyStats {
            reused_parents: 1,
            rebuilt_parents: 1
        }
    );
    assert!(layout.rows("g2").is_none());
    assert_eq!(layout.cached_parents(), 2);
    assert_eq!(layout.cached_children(), data.len() - 1);
    assert_areas(&scene, &c);
    let empty = layout.layout(&c, &[], &o).unwrap();
    assert!(empty.marks.is_empty());
    assert_eq!(layout.cached_parents(), 0);
    assert_eq!(layout.cached_children(), 0);
}
#[test]
fn flat_input_permutation_preserves_cached_membership_geometry_and_mark_identity() {
    let c = config();
    let mut data = hierarchy_data();
    let mut layout = Resquarify::default();
    let a = layout.layout(&c, &data, &options()).unwrap();
    let root = layout.rows("root").unwrap().to_vec();
    data.reverse();
    let b = layout.layout(&c, &data, &options()).unwrap();
    assert_eq!(layout.last_stats().reused_parents, 3);
    assert_eq!(layout.rows("root").unwrap(), root);
    assert_eq!(geometries(&a), geometries(&b));
    assert_eq!(
        a.marks.iter().map(|m| &m.id).collect::<BTreeSet<_>>(),
        b.marks.iter().map(|m| &m.id).collect()
    );
    assert_areas(&b, &c);
}
#[test]
fn zero_weights_stay_in_topology_across_disappearance_and_return() {
    let c = config();
    let mut layout = Resquarify::default();
    let initial = flat(&[0., 0., 0., 0.]);
    assert!(
        layout
            .layout(&c, &initial, &options())
            .unwrap()
            .marks
            .is_empty()
    );
    let rows = layout.rows("root").unwrap().to_vec();
    assert_eq!(rows.iter().map(|row| row.children.len()).sum::<usize>(), 4);
    let populated = layout
        .layout(&c, &flat(&[2., 0., 4., 1.]), &options())
        .unwrap();
    assert_eq!(layout.rows("root").unwrap(), rows);
    assert_eq!(layout.last_stats().reused_parents, 1);
    assert_eq!(populated.marks.len(), 3);
    assert_areas(&populated, &c);
    assert!(
        layout
            .layout(&c, &initial, &options())
            .unwrap()
            .marks
            .is_empty()
    );
    assert_eq!(layout.rows("root").unwrap(), rows);
    let all = layout
        .layout(&c, &flat(&[1., 1., 1., 1.]), &options())
        .unwrap();
    assert_eq!(all.marks.len(), 4);
    assert_areas(&all, &c);
    let mut mixed = Resquarify::default();
    mixed
        .layout(&c, &flat(&[3., 0., 2., 0.]), &options())
        .unwrap();
    let mixed_rows = mixed.rows("root").unwrap().to_vec();
    let next = mixed
        .layout(&c, &flat(&[0., 4., 0., 1.]), &options())
        .unwrap();
    assert_eq!(mixed.rows("root").unwrap(), mixed_rows);
    assert_areas(&next, &c);
}
#[test]
fn invalid_layouts_and_ratio_proposals_preserve_the_entire_cache() {
    let c = config();
    let o = options();
    let mut layout = Resquarify::default();
    let data = hierarchy_data();
    layout.layout(&c, &data, &o).unwrap();
    let before = layout.clone();
    for ratio in [0., -1., 0.5, f64::NAN, f64::INFINITY, 1e9] {
        assert!(layout.set_ratio(ratio).is_err());
        assert_eq!(layout, before);
        assert!(Resquarify::new(ratio).is_err());
    }
    let mut invalid = data.clone();
    invalid[3].value = f64::NAN;
    assert!(layout.layout(&c, &invalid, &o).is_err());
    assert_eq!(layout, before);
    invalid = data.clone();
    invalid[3].parent = Some("missing".into());
    assert!(layout.layout(&c, &invalid, &o).is_err());
    assert_eq!(layout, before);
    invalid = data.clone();
    invalid[0].value = 1.;
    assert!(layout.layout(&c, &invalid, &o).is_err());
    assert_eq!(layout, before);
    invalid = data.clone();
    invalid.push(invalid[3].clone());
    assert!(layout.layout(&c, &invalid, &o).is_err());
    assert_eq!(layout, before);
    assert!(
        layout
            .layout(
                &ChartConfig {
                    width: f64::NAN,
                    ..c.clone()
                },
                &data,
                &o
            )
            .is_err()
    );
    assert_eq!(layout, before);
    assert!(
        layout
            .layout(
                &c,
                &data,
                &HierarchyOptions {
                    padding: f64::NAN,
                    ..o
                }
            )
            .is_err()
    );
    assert_eq!(layout, before);
}
#[test]
fn large_and_subnormal_weights_produce_finite_proportional_geometry() {
    let c = config();
    for values in [
        vec![f64::MAX / 4., f64::MAX / 8., f64::MAX / 16.],
        vec![f64::from_bits(1), f64::from_bits(2), f64::from_bits(3)],
        vec![f64::MAX / 4., f64::from_bits(1), 0.],
    ] {
        let mut layout = Resquarify::default();
        let data = flat(&values);
        let a = layout.layout(&c, &data, &options()).unwrap();
        assert_areas(&a, &c);
        let b = layout.layout(&c, &data, &options()).unwrap();
        assert_eq!(a, b);
        assert_eq!(layout.cached_children(), values.len());
    }
}
#[test]
fn label_appearance_changes_reuse_rows_but_publish_current_owned_labels() {
    let c = config();
    let data = flat(&[6., 5., 4., 3., 2., 1.]);
    let mut layout = Resquarify::default();
    let o = HierarchyOptions {
        padding: 4.,
        labels: true,
        ..Default::default()
    };
    let scene = layout.layout(&c, &data, &o).unwrap();
    let rows = layout.rows("root").unwrap().to_vec();
    assert!(!scene.labels.is_empty());
    assert_eq!(scene.label_series.len(), scene.labels.len());
    assert!(scene.labels.iter().all(|label| label.rotation == 0.));
    let hidden = scene.without_series(&BTreeSet::from(["n0".into()]));
    assert!(hidden.labels.iter().all(|label| label.text != "n0"));
    hidden.validate().unwrap();
    let updated = layout
        .layout(
            &ChartConfig {
                title: "Current frame".into(),
                ..c.clone()
            },
            &data,
            &options(),
        )
        .unwrap();
    assert!(updated.labels.is_empty());
    assert_eq!(updated.title, "Current frame");
    assert_eq!(layout.rows("root").unwrap(), rows);
    assert_eq!(layout.last_stats().reused_parents, 1);
    assert_areas(&updated, &c);
}
#[test]
fn retained_cache_is_bounded_and_does_not_rebuild_stable_large_hierarchies() {
    let c = config();
    let mut data = flat(&(0..2048).map(|i| (i % 97 + 1) as f64).collect::<Vec<_>>());
    let mut layout = Resquarify::default();
    layout.layout(&c, &data, &options()).unwrap();
    let rows = layout.rows("root").unwrap().to_vec();
    for frame in 0..12 {
        for (i, node) in data.iter_mut().enumerate().skip(1) {
            node.value = ((i * 71 + frame * 29) % 101 + 1) as f64;
        }
        let scene = layout.layout(&c, &data, &options()).unwrap();
        assert_eq!(layout.last_stats().reused_parents, 1);
        assert_eq!(layout.last_stats().rebuilt_parents, 0);
        assert_eq!(layout.rows("root").unwrap(), rows);
        assert_eq!(layout.cached_parents(), 1);
        assert_eq!(layout.cached_children(), 2048);
        assert_eq!(scene.marks.len(), 2048);
        scene.validate().unwrap();
    }
}
