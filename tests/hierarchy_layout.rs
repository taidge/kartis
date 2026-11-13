use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use std::collections::BTreeMap;

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
fn data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("a", Some("group".into()), 1.),
        HierarchyDatum::new("b", Some("group".into()), 3.),
        HierarchyDatum::new("c", Some("root".into()), 4.),
    ]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn rect(l: &HierarchyLayout, id: &str) -> [f64; 4] {
    let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = l.node(id).unwrap().geometry.clone().unwrap()
    else {
        panic!()
    };
    [x, y, width, height]
}
fn circle(l: &HierarchyLayout, id: &str) -> [f64; 3] {
    let Geometry::Circle { x, y, radius } = l.node(id).unwrap().geometry.clone().unwrap() else {
        panic!()
    };
    [x, y, radius]
}
fn plain() -> HierarchyStyleOptions {
    let mut s = HierarchyStyleOptions::default();
    s.leaf_labels.enabled = false;
    s.parent_labels.enabled = false;
    s
}
#[test]
fn treemap_headers_reserve_real_content_on_each_side_and_labels_own_masks() {
    let c = config();
    for position in [
        TreemapParentLabelPosition::Top,
        TreemapParentLabelPosition::Right,
        TreemapParentLabelPosition::Bottom,
        TreemapParentLabelPosition::Left,
    ] {
        let o = TreemapLayoutOptions {
            tile: HierarchyTreemapTile::Static(TreemapTile::Dice),
            inner_padding: 8.,
            outer_padding: 10.,
            parent_label_size: 24.,
            parent_label_position: position,
            ..Default::default()
        };
        let l = treemap_layout(&c, &data(), None, &o).unwrap();
        let root = l.node("root").unwrap();
        let content = root.content_rect.unwrap();
        let expected = match position {
            TreemapParentLabelPosition::Top => [10., 34., 580., 356.],
            TreemapParentLabelPosition::Bottom => [10., 10., 580., 356.],
            TreemapParentLabelPosition::Left => [34., 10., 556., 380.],
            TreemapParentLabelPosition::Right => [10., 10., 556., 380.],
        };
        for (a, b) in [content.x, content.y, content.width, content.height]
            .into_iter()
            .zip(expected)
        {
            close(a, b);
        }
        let mut siblings = [rect(&l, "group"), rect(&l, "c")];
        siblings.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let [a, b] = siblings;
        close(a[0], content.x);
        close(b[0] + b[2], content.x + content.width);
        close(b[0] - (a[0] + a[2]), 8.);
        for n in &l.nodes {
            for slot in &n.label_slots {
                assert_eq!(l.scene.label_series.get(slot), Some(&n.id));
                assert!(l.scene.label_clips.contains_key(slot));
            }
        }
        let slot = root.label_slots[0];
        let clip = &l.scene.label_clips[&slot];
        assert!(!clip.contains(
            content.x + content.width / 2.,
            content.y + content.height / 2.
        ));
        if matches!(
            position,
            TreemapParentLabelPosition::Left | TreemapParentLabelPosition::Right
        ) {
            close(l.scene.labels[slot].rotation, -90.);
        }
        l.scene.validate().unwrap();
    }
}
#[test]
fn padded_stateful_resquarify_reuses_rows_focus_and_topology_and_rolls_back() {
    let c = config();
    let mut d = data();
    let o = TreemapLayoutOptions {
        tile: HierarchyTreemapTile::Resquarify,
        inner_padding: 2.,
        outer_padding: 2.,
        parent_label_size: 12.,
        ..Default::default()
    };
    assert!(treemap_layout(&c, &d, None, &o).is_err());
    let mut state = TreemapLayoutState::default();
    let first = state.layout(&c, &d, None, &o).unwrap();
    assert_eq!(first.resquarify_stats.unwrap().rebuilt_parents, 2);
    let rows = state.rows("root").unwrap().to_vec();
    d[2].value = 5.;
    d[3].value = 0.;
    let next = state.layout(&c, &d, None, &o).unwrap();
    assert_eq!(next.resquarify_stats.unwrap().reused_parents, 2);
    assert_eq!(state.rows("root").unwrap(), rows);
    assert!(!next.node("b").unwrap().visible);
    assert!(next.node("b").unwrap().geometry.is_none());
    let resized = ChartConfig {
        width: 400.,
        height: 600.,
        ..c.clone()
    };
    let sized = state.layout(&resized, &d, None, &o).unwrap();
    assert_eq!(sized.resquarify_stats.unwrap().reused_parents, 2);
    assert_eq!(state.rows("root").unwrap(), rows);
    assert_eq!(rect(&sized, "root"), [0., 0., 400., 600.]);
    let focused = state.layout(&c, &d, Some("group"), &o).unwrap();
    assert_eq!(focused.resquarify_stats.unwrap().reused_parents, 1);
    assert!(!focused.node("c").unwrap().visible);
    let before = state.clone();
    let mut bad = o.clone();
    bad.outer_padding = 10000.;
    assert!(state.layout(&c, &d, None, &bad).is_err());
    assert_eq!(state, before);
    d[3].parent = Some("root".into());
    state.layout(&c, &d, None, &o).unwrap();
    assert_eq!(state.last_stats().rebuilt_parents, 2);
    assert!(
        state
            .rows("group")
            .unwrap()
            .iter()
            .flat_map(|r| &r.children)
            .all(|id| id != "b")
    );
}
#[test]
fn treemap_unpadded_leaf_areas_conserve_values_for_every_tiler() {
    let c = config();
    for tile in [
        TreemapTile::Binary,
        TreemapTile::Slice,
        TreemapTile::Dice,
        TreemapTile::SliceDice,
        TreemapTile::Squarify,
    ] {
        let o = TreemapLayoutOptions {
            tile: HierarchyTreemapTile::Static(tile),
            inner_padding: 0.,
            outer_padding: 0.,
            parent_label_size: 0.,
            style: plain(),
            ..Default::default()
        };
        let l = treemap_layout(&c, &data(), None, &o).unwrap();
        for (id, mass) in [("a", 1.), ("b", 3.), ("c", 4.)] {
            let r = rect(&l, id);
            close(r[2] * r[3], 600. * 400. * mass / 8.);
        }
    }
}
#[test]
fn packing_focus_projects_actual_circle_and_preserves_ratios_and_metadata() {
    let c = config();
    let o = CirclePackingLayoutOptions {
        viewport_padding: 5.,
        style: plain(),
        ..Default::default()
    };
    let l = circle_packing_layout(&c, &data(), None, &o).unwrap();
    let legacy = circle_packing_with_algorithm(
        &c,
        &data(),
        &HierarchyOptions {
            padding: o.padding,
            labels: false,
            include_root: true,
        },
        o.algorithm,
    )
    .unwrap();
    for mark in &legacy.marks {
        let Geometry::Circle { x, y, radius } = mark.geometry else {
            panic!()
        };
        let projected = circle(&l, mark.id.strip_prefix("node:").unwrap());
        close(projected[0], 300. + (x - 300.) * 195. / 200.);
        close(projected[1], 200. + (y - 200.) * 195. / 200.);
        close(projected[2], radius * 195. / 200.);
    }
    let focused = circle_packing_layout(&c, &data(), Some("group"), &o).unwrap();
    let g = circle(&focused, "group");
    close(g[0], 300.);
    close(g[1], 200.);
    close(g[2], 195.);
    let original = circle(&l, "group");
    let ratio = g[2] / original[2];
    for id in ["a", "b"] {
        let p = circle(&focused, id);
        let q = circle(&l, id);
        close(p[2], q[2] * ratio);
        assert!((p[0] - g[0]).hypot(p[1] - g[1]) + p[2] <= g[2] + 1e-7);
    }
    let a = circle(&focused, "a");
    let b = circle(&focused, "b");
    assert!((a[0] - b[0]).hypot(a[1] - b[1]) + 1e-7 >= a[2] + b[2]);
    assert_eq!(focused.focus_ancestors, ["root"]);
    assert_eq!(
        focused
            .ancestors("a")
            .unwrap()
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        ["root", "group"]
    );
    assert_eq!(focused.descendants("group", false).unwrap().len(), 2);
    assert_eq!(focused.nodes.len(), 5);
    assert!(!focused.node("root").unwrap().visible);
    assert!(focused.node("c").unwrap().geometry.is_none());
    let leaf = circle_packing_layout(&c, &data(), Some("a"), &o).unwrap();
    close(circle(&leaf, "a")[2], 195.);
    assert_eq!(leaf.scene.marks.len(), 1);
    // These descendants are far below an ulp of the full-tree screen center.
    // Focus must retain parent-relative layout, not magnify rounded centers.
    for algorithm in [
        CirclePackingAlgorithm::FrontChain,
        CirclePackingAlgorithm::Shelf,
    ] {
        for (weight, big) in [(1e-300, 1.), (f64::from_bits(1), f64::MAX)] {
            for padding in [0., 1e-160, 2.] {
                for count in [2, 3] {
                    let mut tiny = vec![
                        HierarchyDatum::new("root", None, 0.),
                        HierarchyDatum::new("group", Some("root".into()), 0.),
                        HierarchyDatum::new("big", Some("root".into()), big),
                    ];
                    tiny.extend((0..count).map(|i| {
                        HierarchyDatum::new(format!("tiny{i}"), Some("group".into()), weight)
                    }));
                    let options = CirclePackingLayoutOptions {
                        algorithm,
                        padding,
                        viewport_padding: 5.,
                        style: plain(),
                    };
                    let focused =
                        circle_packing_layout(&c, &tiny, Some("group"), &options).unwrap();
                    let p = circle(&focused, "group");
                    close(p[2], 195.);
                    assert_eq!(focused.nodes.len(), count + 3);
                    assert_eq!(focused.focus_ancestors, ["root"]);
                    for i in 0..count {
                        let a = circle(&focused, &format!("tiny{i}"));
                        assert!((a[0] - p[0]).hypot(a[1] - p[1]) + a[2] <= p[2] + 1e-4);
                        for j in i + 1..count {
                            let b = circle(&focused, &format!("tiny{j}"));
                            assert!((a[0] - b[0]).hypot(a[1] - b[1]) + 1e-4 >= a[2] + b[2]);
                        }
                        if padding == 0.
                            && count == 2
                            && algorithm == CirclePackingAlgorithm::FrontChain
                        {
                            close(a[2], 97.5);
                            close(a[1], 200.);
                            close((a[0] - 300.).abs(), 97.5);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn icicle_repartitions_focused_depth_and_all_four_directions_mirror() {
    let c = config();
    let o = IcicleLayoutOptions {
        padding: 0.,
        style: plain(),
        ..Default::default()
    };
    let down = icicle_layout(&c, &data(), Some("group"), &o).unwrap();
    let a = rect(&down, "a");
    close(a[2], 150.);
    close(a[3], 200.);
    close(a[1], 200.);
    for direction in [
        IcicleDirection::Up,
        IcicleDirection::Right,
        IcicleDirection::Left,
    ] {
        let l = icicle_layout(
            &c,
            &data(),
            Some("group"),
            &IcicleLayoutOptions {
                direction,
                ..o.clone()
            },
        )
        .unwrap();
        let b = rect(&l, "a");
        match direction {
            IcicleDirection::Up => {
                close(b[0], a[0]);
                close(b[1], 400. - a[1] - a[3]);
            }
            IcicleDirection::Right => {
                close(b[0], 300.);
                close(b[1], 0.);
                close(b[2], 300.);
                close(b[3], 100.);
            }
            IcicleDirection::Left => {
                close(b[0], 0.);
                close(b[3], 100.);
            }
            _ => unreachable!(),
        }
        assert_eq!(l.focus_id.as_deref(), Some("group"));
        assert_eq!(l.node("group").unwrap().relative_depth, Some(0));
    }
    let leaf = icicle_layout(&c, &data(), Some("a"), &o).unwrap();
    assert_eq!(rect(&leaf, "a"), [0., 0., 600., 400.]);
}
#[test]
fn hidden_owned_labels_and_ids_styles_stay_stable_under_input_permutation() {
    let c = config();
    let d = data();
    let mut o = IcicleLayoutOptions::default();
    o.style.leaf.border = Some(GraphicStroke {
        color: Color(2, 3, 4, 255),
        width: 3.,
        cap: StrokeCap::Butt,
    });
    o.style.nodes.insert(
        "a".into(),
        HierarchyNodeStyle {
            fill: HierarchyFill::Fixed(Color(220, 0, 0, 255)),
            ..Default::default()
        },
    );
    let l = icicle_layout(&c, &d, None, &o).unwrap();
    let mut reversed = d.clone();
    reversed.reverse();
    let same = icicle_layout(&c, &reversed, None, &o).unwrap();
    assert_eq!(l, same);
    let marks = l
        .scene
        .marks
        .iter()
        .map(|m| (m.id.clone(), m.clone()))
        .collect::<BTreeMap<_, _>>();
    o.style.hidden.insert("group".into());
    let hidden = icicle_layout(&c, &d, None, &o).unwrap();
    assert!(!hidden.node("group").unwrap().visible);
    assert!(hidden.node("a").unwrap().visible);
    assert!(hidden.scene.label_series.values().all(|id| id != "group"));
    for m in &hidden.scene.marks {
        assert_eq!(m, &marks[&m.id]);
    }
}
#[test]
fn bounded_invalid_inputs_zero_focus_and_extreme_projection_are_explicit() {
    let c = config();
    let d = data();
    let mut o = TreemapLayoutOptions {
        ratio: f64::NAN,
        ..Default::default()
    };
    assert!(treemap_layout(&c, &d, None, &o).is_err());
    o = Default::default();
    o.style.leaf.opacity = f64::INFINITY;
    assert!(treemap_layout(&c, &d, None, &o).is_err());
    assert!(icicle_layout(&c, &d, Some("unknown"), &Default::default()).is_err());
    let mut bad = d.clone();
    bad[2].value = f64::MAX;
    bad[3].value = f64::MAX;
    assert!(icicle_layout(&c, &bad, None, &Default::default()).is_err());
    let mut zero = d.clone();
    zero[2].value = 0.;
    assert!(circle_packing_layout(&c, &zero, Some("a"), &Default::default()).is_err());
    assert!(
        treemap_layout(&c, &zero, Some("a"), &Default::default())
            .unwrap()
            .scene
            .marks
            .is_empty()
    );
    assert!(
        icicle_layout(&c, &zero, Some("a"), &Default::default())
            .unwrap()
            .scene
            .marks
            .is_empty()
    );
    let mut large = vec![HierarchyDatum::new("root", None, 0.)];
    large.extend((0..8000).map(|i| HierarchyDatum::new(format!("n{i}"), Some("root".into()), 1.)));
    assert!(treemap_layout(&c, &large, None, &Default::default()).is_err());
    assert!(circle_packing_layout(&c, &large[..2001], None, &Default::default()).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_focus_and_parent_header_change_real_pixels_not_scene_camera() {
    use render::raster::*;
    let c = config();
    let mut o = CirclePackingLayoutOptions {
        style: plain(),
        ..Default::default()
    };
    o.style.parent.opacity = 0.;
    o.style.leaf.fill = HierarchyFill::Fixed(Color(220, 10, 20, 255));
    let normal = circle_packing_layout(&c, &data(), None, &o).unwrap();
    let focused = circle_packing_layout(&c, &data(), Some("a"), &o).unwrap();
    let render = |s: &Scene| render_raster(s, &FontRegistry::new(), &Default::default()).unwrap();
    let before = render(&normal.scene);
    let after = render(&focused.scene);
    let count = |rgba: &[u8]| {
        rgba.as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 180 && p[1] < 80 && p[2] < 80)
            .count()
    };
    assert!(count(&after.rgba) > count(&before.rgba));
    let p = &after.rgba[(200 * 600 + 300) * 4..][..4];
    assert_eq!(p, [220, 10, 20, 255]);
    let mut t = TreemapLayoutOptions {
        inner_padding: 0.,
        outer_padding: 0.,
        parent_label_size: 30.,
        style: plain(),
        ..Default::default()
    };
    t.style.parent_labels.enabled = true;
    t.style.parent_labels.color = Some(Color(0, 0, 0, 0));
    t.style.parent.fill = HierarchyFill::Fixed(Color(0, 40, 220, 255));
    t.style.leaf.fill = HierarchyFill::Fixed(Color(220, 10, 20, 255));
    // Transparent labels still require a font in text rendering, so skip their ink
    // while retaining the reserved band via skip_width.
    t.style.parent_labels.skip_width = 16384.;
    let image = render(&treemap_layout(&c, &data(), None, &t).unwrap().scene);
    assert_eq!(&image.rgba[(10 * 600 + 300) * 4..][..4], [0, 40, 220, 255]);
}
