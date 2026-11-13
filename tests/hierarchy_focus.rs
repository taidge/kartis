use kartis::*;
fn data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("a", Some("group".into()), 1.),
        HierarchyDatum::new("b", Some("group".into()), 3.),
        HierarchyDatum::new("c", Some("root".into()), 2.),
    ]
}
fn view() -> HierarchyView {
    let mut o = TreemapLayoutOptions {
        tile: HierarchyTreemapTile::Resquarify,
        ..Default::default()
    };
    o.style.leaf_labels.enabled = false;
    o.style.parent_labels.enabled = false;
    HierarchyView::Treemap(o)
}
#[test]
fn real_focus_parent_back_root_and_bounded_history() {
    let c = ChartConfig::default();
    let v = view();
    let mut f = HierarchyFocusController::new(&data()).unwrap();
    f.layout(&c, &v).unwrap();
    assert_eq!(f.treemap_state().cached_parents(), 2);
    let group = f.focus(&c, "group", &v).unwrap();
    assert_eq!(group.focus_id.as_deref(), Some("group"));
    assert_eq!(f.ancestors(), ["root"]);
    f.focus(&c, "a", &v).unwrap();
    assert_eq!(f.ancestors(), ["root", "group"]);
    f.back(&c, &v).unwrap();
    assert_eq!(f.current(), Some("group"));
    f.parent(&c, &v).unwrap();
    assert_eq!(f.current(), Some("root"));
    for i in 0..140 {
        f.focus(&c, if i % 2 == 0 { "a" } else { "b" }, &v).unwrap();
    }
    assert_eq!(f.history().len(), MAX_HIERARCHY_FOCUS_HISTORY);
    f.root(&c, &v).unwrap();
    assert_eq!(f.current(), Some("root"));
}
#[test]
fn replace_recovers_nearest_surviving_ancestor_then_root_and_empty() {
    let c = ChartConfig::default();
    let v = view();
    let mut d = data();
    let mut f = HierarchyFocusController::new(&d).unwrap();
    f.focus(&c, "a", &v).unwrap();
    d.retain(|n| n.id != "a");
    let next = f.replace(&c, &d, &v).unwrap();
    assert_eq!(f.current(), Some("group"));
    assert!(next.node("a").is_none());
    assert_eq!(next.node("group").unwrap().value, 3.);
    d.retain(|n| n.id != "group");
    d.iter_mut().find(|n| n.id == "b").unwrap().parent = Some("root".into());
    f.replace(&c, &d, &v).unwrap();
    assert_eq!(f.current(), Some("root"));
    f.replace(&c, &[], &v).unwrap();
    assert!(f.current().is_none());
    assert!(f.history().is_empty());
    assert_eq!(f.treemap_state().cached_parents(), 0);
    f.replace(&c, &data(), &v).unwrap();
    assert_eq!(f.current(), Some("root"));
    f.focus(&c, "a", &v).unwrap();
    let mut moved = data();
    moved.iter_mut().find(|n| n.id == "a").unwrap().parent = Some("root".into());
    let layout = f.replace(&c, &moved, &v).unwrap();
    assert_eq!(f.current(), Some("a"));
    assert_eq!(f.ancestors(), ["root"]);
    assert_eq!(layout.node("a").unwrap().parent.as_deref(), Some("root"));
}
#[test]
fn invalid_updates_and_failed_views_preserve_accepted_data_focus_history_and_cache() {
    let c = ChartConfig::default();
    let v = view();
    let mut f = HierarchyFocusController::new(&data()).unwrap();
    f.layout(&c, &v).unwrap();
    f.focus(&c, "a", &v).unwrap();
    let before = f.clone();
    let assert_same = |f: &HierarchyFocusController| {
        assert_eq!(f.current(), before.current());
        assert_eq!(f.data(), before.data());
        assert_eq!(f.history(), before.history());
        assert_eq!(f.revision(), before.revision());
        assert_eq!(f.treemap_state(), before.treemap_state());
    };
    assert!(f.focus(&c, "unknown", &v).is_err());
    assert_same(&f);
    let mut bad = data();
    bad[2].parent = Some("a".into());
    assert!(f.replace(&c, &bad, &v).is_err());
    assert_same(&f);
    let invalid = HierarchyView::Treemap(TreemapLayoutOptions {
        outer_padding: f64::NAN,
        ..Default::default()
    });
    assert!(f.root(&c, &invalid).is_err());
    assert_same(&f);
    let mut valid = data();
    valid[2].value = 9.;
    assert!(f.replace(&c, &valid, &invalid).is_err());
    assert_same(&f);
    f.replace(&c, &valid, &v).unwrap();
    assert_eq!(f.current(), Some("a"));
    assert_eq!(f.data()[2].value, 9.);
}
#[test]
fn view_switches_share_raw_focus_and_do_not_mutate_camera() {
    let c = ChartConfig::default();
    let mut f = HierarchyFocusController::new(&data()).unwrap();
    f.focus(&c, "group", &view()).unwrap();
    let a = f
        .layout(&c, &HierarchyView::CirclePacking(Default::default()))
        .unwrap();
    let b = f
        .layout(
            &c,
            &HierarchyView::Icicle(IcicleLayoutOptions {
                direction: IcicleDirection::Left,
                ..Default::default()
            }),
        )
        .unwrap();
    assert_eq!(a.focus_id, b.focus_id);
    assert_eq!(a.focus_ancestors, b.focus_ancestors);
    assert_eq!(a.node("a").unwrap().mark_id, b.node("a").unwrap().mark_id);
    assert!(!a.node("c").unwrap().visible);
    assert!(!b.node("c").unwrap().visible);
}
