use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 480.,
        height: 360.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 24.,
            bottom: 26.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<HierarchyDatum> {
    // Source order deliberately differs from root-first computed order.
    vec![
        HierarchyDatum::new("c", Some("north".into()), 3.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("e", Some("south".into()), 1.),
        HierarchyDatum::new("north", Some("root".into()), 0.),
        HierarchyDatum::new("b", Some("north".into()), -0.),
        HierarchyDatum::new("south", Some("root".into()), 0.),
        HierarchyDatum::new("d", Some("south".into()), 4.),
        HierarchyDatum::new("a", Some("north".into()), 2.),
    ]
}
fn style() -> HierarchyStyleOptions {
    HierarchyStyleOptions {
        leaf_labels: HierarchyLabelStyle {
            enabled: false,
            ..Default::default()
        },
        parent_labels: HierarchyLabelStyle {
            enabled: false,
            ..Default::default()
        },
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    }
}
fn view(kind: HierarchyWidgetKind, style: HierarchyStyleOptions) -> HierarchyView {
    match kind {
        HierarchyWidgetKind::Treemap => HierarchyView::Treemap(TreemapLayoutOptions {
            style,
            ..Default::default()
        }),
        HierarchyWidgetKind::CirclePacking => {
            HierarchyView::CirclePacking(CirclePackingLayoutOptions {
                style,
                ..Default::default()
            })
        }
        HierarchyWidgetKind::Icicle => HierarchyView::Icicle(IcicleLayoutOptions {
            style,
            ..Default::default()
        }),
    }
}
fn kinds() -> [HierarchyWidgetKind; 3] {
    [
        HierarchyWidgetKind::Treemap,
        HierarchyWidgetKind::CirclePacking,
        HierarchyWidgetKind::Icicle,
    ]
}
fn layout(
    kind: HierarchyWidgetKind,
    style: HierarchyStyleOptions,
    focus: Option<&str>,
) -> HierarchyLayout {
    let mut ctl = HierarchyFocusController::new(&data()).unwrap();
    if let Some(f) = focus {
        ctl.focus(&config(), f, &view(kind, style)).unwrap()
    } else {
        ctl.layout(&config(), &view(kind, style)).unwrap()
    }
}
fn placement(_: &HierarchyNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-8., -4.],
        measured: TextSize {
            width: 16.,
            height: 8.,
        },
    }))
}

#[test]
fn all_families_preserve_flat_raw_source_aggregate_identity_and_shared_ownership() {
    for kind in kinds() {
        let mut layout = layout(kind, style(), None);
        let before = layout.clone();
        let mut order = vec![];
        let packet = hierarchy_node_widgets(&layout, 7, |c| {
            assert!(std::ptr::eq(c.layout, &layout));
            assert!(std::ptr::eq(c.datum, &c.source[c.source_index]));
            assert_eq!(c.datum.id, c.node.id);
            assert_eq!(c.node.mark_id, c.mark.id);
            order.push(c.node.id.clone());
            placement(c)
        })
        .unwrap();
        assert_eq!(layout, before);
        assert_eq!(order, ["root", "north", "south", "a", "c", "d", "e"]);
        assert_eq!(packet.layout.plot, Some(layout.plot()));
        assert!(packet.layout.slots.iter().all(|s| s.revision == 7));
        let root = &packet.data["node:root"];
        assert_eq!(root.source_index, 1);
        assert_eq!(root.source[1].value, 0.);
        assert_eq!(root.node.value, 10.);
        assert_eq!(root.formatted_value, "10.0");
        assert_eq!(root.source[4].value.to_bits(), (-0_f64).to_bits());
        assert_eq!(root.kind, kind);
        assert!(
            packet
                .data
                .values()
                .all(|d| Arc::ptr_eq(&root.source, &d.source)
                    && Arc::ptr_eq(&root.focus_ancestors, &d.focus_ancestors))
        );
        assert_eq!(root.root_id.as_deref(), Some("root"));
        assert_eq!(root.focus_id.as_deref(), Some("root"));
        assert!(root.focus_ancestors.is_empty());
        assert!(
            packet
                .data
                .values()
                .all(|d| d.source.len() == 8 && d.source[d.source_index].id == d.node.id)
        );
        let mut cloned = root.source.clone();
        Arc::make_mut(&mut cloned)[0].value = 777.;
        assert_eq!(root.source[0].value, 3.);
        layout.nodes[0].value = 777.;
        assert_eq!(root.node.value, 10.);
        assert_eq!(hierarchy_widget_frame(&before).unwrap().scene, before.scene);
    }
}

#[test]
fn focus_hidden_leaves_and_include_focus_match_actual_marks_without_invented_zero_roles() {
    for kind in kinds() {
        for leaves_only in [false, true] {
            for include_focus in [false, true] {
                for focus in ["root", "north", "a"] {
                    let mut s = style();
                    s.leaves_only = leaves_only;
                    s.include_focus = include_focus;
                    s.hidden.insert("c".into());
                    let l = layout(kind, s, Some(focus));
                    let packet = hierarchy_node_widgets(&l, 0, placement).unwrap();
                    assert_eq!(packet.layout.slots.len(), l.scene.marks.len());
                    assert!(!packet.data.contains_key("node:b"));
                    assert!(!packet.data.contains_key("node:c"));
                    if focus != "root" {
                        assert!(!packet.data.contains_key("node:d"));
                    }
                    if !include_focus {
                        assert!(!packet.data.contains_key(&format!("node:{focus}")));
                    }
                    if leaves_only {
                        assert!(packet.data.values().all(|d| d.node.leaf));
                    }
                    let frame = hierarchy_widget_frame(&l).unwrap();
                    assert_eq!(frame.plot, Some(l.plot()));
                    assert_eq!(
                        frame.interaction.as_ref().unwrap().sites.len(),
                        packet.data.len()
                    );
                    assert!(
                        packet
                            .data
                            .values()
                            .all(|d| d.node.value > 0. && d.node.relative_depth.is_some())
                    );
                }
            }
        }
    }
    for kind in kinds() {
        for raw in [
            vec![],
            vec![
                HierarchyDatum::new("root", None, 0.),
                HierarchyDatum::new("a", Some("root".into()), 0.),
            ],
        ] {
            let mut c = HierarchyFocusController::new(&raw).unwrap();
            let l = c.layout(&config(), &view(kind, style())).unwrap();
            let packet = hierarchy_node_widgets(&l, 4, |_| {
                panic!("empty or zero-weight allocation has no component")
            })
            .unwrap();
            assert!(packet.data.is_empty());
            assert_eq!(packet.layout.plot, Some(l.plot()));
            assert_eq!(hierarchy_widget_frame(&l).unwrap().plot, Some(l.plot()));
        }
    }
}

#[test]
fn zero_opacity_and_no_paint_real_marks_keep_components_and_typed_sites() {
    for kind in kinds() {
        let mut s = style();
        s.leaf.fill = HierarchyFill::None;
        s.parent.fill = HierarchyFill::None;
        let mut l = layout(kind, s, None);
        assert_eq!(l.scene.marks.len(), 7);
        assert!(l.interaction_binding().unwrap().sites.is_empty());
        let packet = hierarchy_node_widgets(&l, 0, placement).unwrap();
        assert_eq!(packet.data.len(), 7);
        let frame = hierarchy_widget_frame(&l).unwrap();
        assert_eq!(frame.scene, l.scene);
        assert_eq!(frame.interaction.as_ref().unwrap().sites.len(), 7);
        assert!(
            frame
                .interaction
                .as_ref()
                .unwrap()
                .sites
                .iter()
                .all(|s| s.color == Color(0, 0, 0, 0))
        );
        let model = surface::SurfaceModel::new_with_interaction(
            frame.scene.clone(),
            surface::SurfaceOptions {
                plot: frame.plot,
                ..Default::default()
            },
            frame.interaction.clone(),
        )
        .unwrap();
        for site in &frame.interaction.as_ref().unwrap().sites {
            assert!(model.logical_datum(&site.mark_id).is_some());
            assert!(
                model
                    .physical_hit_at((site.position[0], site.position[1]))
                    .unwrap()
                    .is_none()
            );
        }
        for mark in &mut l.scene.marks {
            mark.opacity = 0.;
            mark.fill = Some(Color(9, 8, 7, 0));
        }
        assert_eq!(
            hierarchy_node_widgets(&l, 1, placement).unwrap().data.len(),
            7
        );
        assert_eq!(hierarchy_widget_frame(&l).unwrap().scene, l.scene);
    }
}

#[test]
fn managed_measured_label_placement_hidden_restoration_and_current_owned_clips_remain_valid() {
    for kind in kinds() {
        let mut s = style();
        s.leaf_labels.enabled = true;
        s.parent_labels.enabled = true;
        let raw = layout(kind, s, None)
            .with_label_font(FontOptions::default())
            .unwrap();
        let measurements = raw
            .label_candidates()
            .iter()
            .map(|c| {
                c.measured(LayoutRect {
                    x: -2.,
                    y: -2.,
                    width: 4.,
                    height: 4.,
                })
            })
            .collect::<Vec<_>>();
        let mut opts = HierarchyLabelPlacementOptions::for_layout(&raw);
        opts.hidden.insert("c".into());
        let placed = raw.place_labels(&measurements, &opts).unwrap();
        assert_eq!(raw.scene.marks, placed.scene.marks);
        let p = hierarchy_node_widgets(&placed, 2, placement).unwrap();
        assert!(p.data["node:c"].labels.is_empty());
        for d in p.data.values() {
            assert_eq!(d.labels.len(), d.node.label_slots.len());
            for (label, slot) in d.labels.iter().zip(&d.node.label_slots) {
                assert_eq!(label.label, placed.scene.labels[*slot]);
                assert_eq!(label.clip, placed.scene.label_clips.get(slot).cloned());
            }
        }
        opts.hidden.clear();
        let restored = placed.place_labels(&measurements, &opts).unwrap();
        let packet = hierarchy_node_widgets(&restored, 2, placement).unwrap();
        assert_eq!(packet.data.len(), 7);
        assert!(!packet.data["node:c"].labels.is_empty());
        assert_eq!(
            hierarchy_widget_frame(&restored).unwrap().scene,
            restored.scene
        );
    }
}

#[test]
fn source_topology_aggregate_focus_indexes_and_late_actual_mark_conflicts_reject_before_callback() {
    type Mutation = (&'static str, fn(&mut HierarchyLayout));
    let cases: &[Mutation] = &[
        ("raw aggregate forged coherently", |l| {
            l.nodes[0].value = 777.;
            let m = l
                .scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:root")
                .unwrap();
            m.value = 777.;
            m.label = "root: 777.0".into();
        }),
        ("label source", |l| {
            l.nodes[0].label = "invented raw datum".into()
        }),
        ("parent source", |l| {
            l.nodes[3].parent = Some("south".into())
        }),
        ("children source", |l| l.nodes[0].children.reverse()),
        ("depth source", |l| l.nodes[3].depth = 0),
        ("relative depth", |l| l.nodes[3].relative_depth = None),
        ("leaf source", |l| l.nodes[0].leaf = true),
        ("mark role identity", |l| {
            l.nodes[3].mark_id = "node:north".into()
        }),
        ("source node id", |l| l.nodes[3].id = "other".into()),
        ("root", |l| l.root_id = Some("a".into())),
        ("focus", |l| l.focus_id = Some("a".into())),
        ("focus ancestors", |l| l.focus_ancestors.push("a".into())),
        ("node source order", |l| l.nodes.swap(0, 1)),
        ("missing node", |l| {
            l.nodes.pop();
        }),
        ("late NaN total", |l| {
            l.nodes.last_mut().unwrap().value = f64::NAN
        }),
        ("zero invented geometry", |l| {
            l.nodes.iter_mut().find(|n| n.id == "b").unwrap().geometry = Some(Geometry::Rect {
                x: 1.,
                y: 1.,
                width: 2.,
                height: 2.,
            })
        }),
        ("eligibility", |l| {
            l.nodes[0].visible = false;
            l.scene.marks.retain(|m| m.id != "node:root");
        }),
        ("metadata geometry", |l| {
            if let Some(Geometry::Rect { x, .. }) = &mut l.nodes[3].geometry {
                *x += 1.;
            }
        }),
        ("actual geometry", |l| {
            if let Geometry::Rect { x, .. } = &mut l.scene.marks[0].geometry {
                *x += 1.;
            }
        }),
        ("source scalar", |l| l.scene.marks[0].value = 42.),
        ("source label", |l| {
            l.scene.marks[0].label = "changed".into()
        }),
        ("source series", |l| l.scene.marks[0].series = "a".into()),
        ("source clip", |l| l.scene.marks[0].clip = None),
        ("missing mark", |l| {
            l.scene.marks.pop();
        }),
        ("orphan source mark", |l| {
            let mut m = l.scene.marks[0].clone();
            m.id = "node:fake".into();
            l.scene.marks.push(m);
        }),
        ("managed label", |l| {
            l.scene.labels.last_mut().unwrap().text = "fake".into()
        }),
        ("managed label slot", |l| l.nodes[0].label_slots.clear()),
        ("family changed coherently", |l| {
            let g = Geometry::Circle {
                x: 100.,
                y: 100.,
                radius: 10.,
            };
            l.nodes[3].geometry = Some(g.clone());
            let id = l.nodes[3].mark_id.clone();
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == id)
                .unwrap()
                .geometry = g;
        }),
    ];
    for (name, mutate) in cases {
        let mut s = style();
        s.leaf_labels.enabled = true;
        s.parent_labels.enabled = true;
        let mut l = layout(HierarchyWidgetKind::Treemap, s, None);
        mutate(&mut l);
        let mut calls = 0;
        assert!(
            hierarchy_node_widgets(&l, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err(),
            "accepted {name}"
        );
        assert_eq!(calls, 0, "callback before complete validation {name}");
        assert!(hierarchy_widget_frame(&l).is_err(), "frame accepted {name}");
    }
}

#[test]
fn budget_skip_callback_errors_and_motion_target_boundary_are_explicit() {
    let mut raw = vec![HierarchyDatum::new("root", None, 0.)];
    raw.extend(
        (0..129).map(|i| HierarchyDatum::new(format!("leaf{i:03}"), Some("root".into()), 1.)),
    );
    let mut s = style();
    s.leaves_only = true;
    let mut ctl = HierarchyFocusController::new(&raw).unwrap();
    let l = ctl
        .layout(&config(), &view(HierarchyWidgetKind::Treemap, s))
        .unwrap();
    let mut calls = 0;
    assert!(
        hierarchy_node_widgets(&l, 0, |c| {
            calls += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    let p = hierarchy_node_widgets(&l, 0, |c| {
        if c.datum.id == "leaf128" {
            Ok(None)
        } else {
            placement(c)
        }
    })
    .unwrap();
    assert_eq!(p.data.len(), 128);
    let mut calls = 0;
    assert!(
        hierarchy_node_widgets(&l, 0, |c| {
            calls += 1;
            if calls == 3 {
                Err(ChartError("refused late role".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 3);
    for kind in kinds() {
        let v = view(kind, style());
        let mut motion =
            HierarchyFocusMotion::new(&config(), &data(), &v, Default::default()).unwrap();
        motion.focus(&config(), "north", &v).unwrap();
        motion.tick(0.04).unwrap();
        assert!(hierarchy_node_widgets(motion.target_layout(), 0, placement).is_ok());
        let mut wrong = motion.target_layout().clone();
        wrong.scene = motion.scene().clone();
        let mut calls = 0;
        assert!(
            hierarchy_node_widgets(&wrong, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls, 0);
        motion.finish();
        assert_eq!(
            hierarchy_widget_frame(motion.target_layout())
                .unwrap()
                .scene,
            *motion.scene()
        );
    }
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let t = app.tree();
    let mut q = t.roots().to_vec();
    let mut out = vec![];
    while let Some(i) = q.pop() {
        let n = t.node(i).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(i);
        }
        q.extend(n.children.iter().copied());
    }
    out
}
fn text(app: &App, id: NodeId) -> String {
    let t = app.tree();
    let Props::Button(p) = t.node(id).unwrap().descriptor.view().props else {
        panic!("Button expected")
    };
    p.label.clone()
}
fn tree(app: &App) -> String {
    let t = app.tree();
    let mut q = t.roots().to_vec();
    let mut out = vec![];
    while let Some(i) = q.pop() {
        let n = t.node(i).unwrap();
        out.push(format!("{i:?}:{n:?}"));
        q.extend(n.children.iter().copied());
    }
    format!("{:?}{out:?}", t.roots())
}
fn memory_source(
    phase: usize,
    revision: u64,
) -> (
    SceneInteraction,
    PlotWidgetsComposition<HierarchyNodeWidgetData>,
) {
    let mut raw = data();
    if phase == 1 {
        raw[4].value = 2.;
    } else if phase == 2 {
        raw[4].value = 0.;
    }
    let mut s = style();
    s.leaf.fill = HierarchyFill::None;
    s.parent.fill = HierarchyFill::None;
    let mut ctl = HierarchyFocusController::new(&raw).unwrap();
    let l = ctl
        .layout(&config(), &view(HierarchyWidgetKind::Treemap, s))
        .unwrap();
    let widgets = hierarchy_node_widgets(&l, revision, placement).unwrap();
    let replace_ink = widgets.data.keys().cloned().collect();
    (
        hierarchy_widget_frame(&l).unwrap(),
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    )
}
fn child(
    c: PlotWidgetCompositionContext<HierarchyNodeWidgetData>,
    cleanup: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    if c.revision == 90 {
        return Err(ChartError("factory rejected".into()));
    }
    let revision = c.revision;
    let frame = c.frame;
    let class = frame.with(|f| format!("role-{}", f.data.node.id));
    Ok(glory::ui::widget(move |scope| {
        let drop = cleanup.clone();
        scope.on_cleanup(move || drop.set(drop.get() + 1))?;
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(class.clone())
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "raw={}|total={}|b={}|count={}|focus={}",
                        f.data.source[f.data.source_index].value,
                        f.data.node.value,
                        f.data.source[4].value,
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
            .on(EventKind::FocusChanged, move |e| {
                if let Event::FocusChanged(v) = e {
                    focus.revise(|f| *f = v)
                }
            })
            .show_in(scope)?;
        if revision == 91 {
            return Err(glory::render::Error::InvalidProps("child build rejected").into());
        }
        Ok(())
    }))
}
#[test]
fn real_memory_parent_raw_and_aggregate_data_remain_atomic_with_local_state() {
    let app = App::memory();
    let source = app.scope().cage(memory_source(0, 0));
    let frame = app.scope().bond(move || source.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || source.with(|s| s.1.clone()));
    let drops = Rc::new(Cell::new(0));
    let clean = drops.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |c| child(c, clean.clone())),
    )
    .unwrap();
    let root = nodes(&app, "role-root")[0];
    assert!(text(&app, root).starts_with("raw=0|total=10|b=-0"));
    app.dispatch(root, Event::Activate).unwrap();
    app.dispatch(root, Event::FocusChanged(true)).unwrap();
    let zero_changed = memory_source(2, 0);
    let before = memory_source(0, 0);
    assert_eq!(before.0, zero_changed.0);
    assert_ne!(before.1.widgets, zero_changed.1.widgets);
    source.revise(|s| *s = zero_changed);
    app.flush().unwrap();
    assert_eq!(nodes(&app, "role-root"), [root]);
    assert!(text(&app, root).contains("total=10|b=0|count=1|focus=true"));
    source.revise(|s| *s = memory_source(1, 0));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "role-root"), [root]);
    assert!(text(&app, root).contains("raw=0|total=12|b=2|count=1|focus=true"));
    assert_eq!(nodes(&app, "role-b").len(), 1);
    for rev in [90, 91] {
        let before = tree(&app);
        source.revise(|s| *s = memory_source(0, rev));
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(nodes(&app, "role-root"), [root]);
        assert!(text(&app, root).contains("total=12|b=2|count=1|focus=true"));
    }
    assert_eq!(errors.borrow().len(), 2);
    source.revise(|s| *s = memory_source(1, 1));
    app.flush().unwrap();
    let next = nodes(&app, "role-root")[0];
    assert_ne!(next, root);
    assert!(text(&app, next).ends_with("count=0|focus=false"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(drops.get(), 17);
}
