use std::cell::RefCell;
use std::rc::Rc;

use glory::App;
use glory::render::{Event, GraphicStroke, NodeId, PointerPhase, StrokeCap};
use glory::style::Length;
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 260.,
        margin: Margin {
            left: 28.,
            right: 28.,
            top: 28.,
            bottom: 28.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<BoxObservation> {
    let mut v = Vec::new();
    for g in ["B", "A", "C"] {
        for s in ["x", "y", ""] {
            if g == "C" && s == "y" {
                v.push(BoxObservation::new("missing", g, s, None));
                continue;
            }
            for (i, x) in [0., 2., 4., 6., 8., 10.].iter().enumerate() {
                v.push(BoxObservation::new(format!("{g}/{s}/{i}"), g, s, Some(*x)));
            }
        }
    }
    v.push(BoxObservation::new("B/x/outlier", "B", "x", Some(100.)));
    v
}
fn stroke(c: Color, w: f64) -> GraphicStroke {
    GraphicStroke {
        color: c,
        width: w,
        cap: StrokeCap::Square,
    }
}
fn options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(["B", "A", "C"].map(str::to_string).to_vec()),
        subgroups: Some(["x", "y", ""].map(str::to_string).to_vec()),
        domain: Some([0., 120.]),
        fill_opacity: 0.4,
        border: Some(stroke(Color(70, 80, 90, 101), 3.)),
        median: stroke(Color(20, 30, 40, 160), 4.),
        whisker: stroke(Color(80, 100, 120, 190), 2.),
        border_color_profile: Some(InheritedColor::FromContext(vec![
            ColorModifier::Darker(1.),
            ColorModifier::Opacity(0.5),
        ])),
        whiskers: BoxWhiskers::Tukey { factor: 1.5 },
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn key() -> BoxPlotKey {
    BoxPlotKey::new("A", "y")
}
fn build() -> Result<BoxPlotInteraction> {
    BoxPlotInteraction::new(&config(), &data(), &options(), &Default::default())
}
fn without_groups(mut layout: BoxPlotLayout) -> BoxPlotLayout {
    layout.scene.opacity_groups.clear();
    layout
}
#[test]
fn association_is_group_or_subgroup_independent_of_palette_and_legend_roles() -> Result<()> {
    for color_by in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        let o = BoxPlotLayoutOptions {
            color_by,
            ..options()
        };
        let mut c = BoxPlotInteraction::new(&config(), &data(), &o, &Default::default())?;
        let base = c.layout.clone();
        c.set_active(Some(key()))?;
        let actual = c
            .active_keys()
            .iter()
            .map(|k| (k.group.as_str(), k.subgroup.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(actual, vec![("B", "y"), ("A", "x"), ("A", "y"), ("A", "")]);
        assert_eq!(c.related(&key())?.len(), 4);
        assert_eq!(c.layout.scene.opacity_groups.len(), 4);
        assert!(
            c.layout
                .scene
                .opacity_groups
                .iter()
                .all(|g| g.opacity == 0.25)
        );
        assert_eq!(without_groups(c.layout.clone()), base);
        assert!(
            c.layout
                .scene
                .marks
                .iter()
                .all(|m| m.series.is_empty() || m.series.starts_with("boxplot-series:"))
        );
        c.set_active(None)?;
        assert_eq!(c.layout, base);
        assert!(c.active_keys().is_empty());
    }
    Ok(())
}
#[test]
fn default_wrapper_preserves_real_builder_layout_scene_svg_and_canvas() -> Result<()> {
    for horizontal in [false, true] {
        for rounded in [0., 8.] {
            for width in [None, Some(0.), Some(3.)] {
                let o = BoxPlotLayoutOptions {
                    orientation: if horizontal {
                        StatisticsOrientation::Horizontal
                    } else {
                        StatisticsOrientation::Vertical
                    },
                    border_radius: rounded,
                    border: width.map(|w| stroke(Color(1, 2, 3, 160), w)),
                    ..options()
                };
                let old = boxplot_layout(&config(), &data(), &o)?;
                let new = BoxPlotInteraction::new(&config(), &data(), &o, &Default::default())?;
                assert_eq!(new.layout, old);
                assert_eq!(
                    render::to_svg(&new.layout.scene)?,
                    render::to_svg(&old.scene)?
                );
                assert_eq!(
                    render::to_canvas_script(&new.layout.scene, "c")?,
                    render::to_canvas_script(&old.scene, "c")?
                );
                assert_eq!(new.interaction_binding()?.sites.len(), old.boxes.len());
                new.scene_interaction()?.validate()?;
            }
        }
    }
    Ok(())
}
#[test]
fn group_opacity_never_accumulates_or_changes_member_alpha_source_and_fill_factor() -> Result<()> {
    let op = BoxPlotInteractionOptions {
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
    };
    let mut c = BoxPlotInteraction::new(&config(), &data(), &options(), &op)?;
    let pristine = boxplot_layout(&config(), &data(), &options())?;
    assert_eq!(c.layout.scene.opacity_groups.len(), 8);
    assert!(
        c.layout
            .scene
            .opacity_groups
            .iter()
            .all(|g| g.opacity == 0.8)
    );
    for _ in 0..16 {
        c.set_active(Some(key()))?;
        assert_eq!(without_groups(c.layout.clone()), pristine);
        for g in &c.layout.scene.opacity_groups {
            let b = c.layout.boxes.iter().find(|b| b.id == g.id).unwrap();
            assert_eq!(
                g.opacity,
                if b.group == "A" || b.subgroup == "y" {
                    0.9
                } else {
                    0.2
                }
            );
        }
        c.set_active(None)?;
        assert_eq!(without_groups(c.layout.clone()), pristine);
        assert!(
            c.layout
                .scene
                .opacity_groups
                .iter()
                .all(|g| g.opacity == 0.8)
        );
    }
    let before = c.layout.clone();
    assert!(
        c.set_options(&BoxPlotInteractionOptions {
            inactive_opacity: f64::NAN,
            ..op
        })
        .is_err()
    );
    assert_eq!(c.layout, before);
    c.set_options(&Default::default())?;
    assert_eq!(c.layout, pristine);
    Ok(())
}
#[test]
fn emitted_parts_and_outliers_have_exact_typed_owners_without_extra_datum_sites() -> Result<()> {
    let mut c = build()?;
    let summary = c
        .layout
        .boxes
        .iter()
        .find(|b| b.group == "B" && b.subgroup == "x")
        .unwrap()
        .clone();
    for suffix in ["", ":border", ":median", ":whiskers", ":caps"] {
        let id = format!("{}{suffix}", summary.id);
        assert_eq!(c.key_for_mark(&id)?, Some(&BoxPlotKey::new("B", "x")));
        c.set_hovered_mark(Some(&id))?;
        assert_eq!(c.active(), Some(&BoxPlotKey::new("B", "x")));
    }
    let id = format!(
        "{}:outlier:{}:{}",
        summary.id,
        "B/x/outlier".len(),
        "B/x/outlier"
    );
    assert_eq!(c.outlier_for_mark(&id)?.unwrap().value, 100.);
    assert_eq!(c.outlier_for_mark(&id)?.unwrap().source_id, "B/x/outlier");
    c.set_hovered_mark(Some(&id))?;
    assert_eq!(c.key_for_mark(&(summary.id.clone() + ":forged"))?, None);
    assert_eq!(c.outlier_for_mark(&summary.id)?, None);
    let binding = c.interaction_binding()?;
    assert_eq!(binding.sites.len(), c.layout.boxes.len());
    for b in &c.layout.boxes {
        let s = binding.sites.iter().find(|s| s.mark_id == b.id).unwrap();
        assert_eq!(s.value, Some(b.quantiles[2]));
        assert_eq!(
            s.position,
            [b.rect.x + b.rect.width / 2., b.rect.y + b.rect.height / 2.]
        );
    }
    let before = c.layout.clone();
    assert!(c.set_hovered_mark(Some("axis/decorative")).is_err());
    assert!(c.set_active(Some(BoxPlotKey::new("foreign", "x"))).is_err());
    assert_eq!(c.layout, before);
    c.set_hovered_mark(None)?;
    assert!(c.active().is_none());
    Ok(())
}
#[test]
fn constant_zero_keeps_logical_site_and_existing_median_ink_without_fake_rectangle() -> Result<()> {
    let data = [
        BoxObservation::new("zero0", "g", "", Some(0.)),
        BoxObservation::new("zero1", "g", "", Some(0.)),
        BoxObservation::new("none", "empty", "", None),
    ];
    let o = BoxPlotLayoutOptions {
        domain: Some([-1., 1.]),
        groups: None,
        subgroups: None,
        border: None,
        ..options()
    };
    let old = boxplot_layout(&config(), &data, &o)?;
    let mut c = BoxPlotInteraction::new(&config(), &data, &o, &Default::default())?;
    assert_eq!(c.layout, old);
    assert_eq!(c.layout.boxes.len(), 1);
    let site = c.interaction_binding()?.sites.remove(0);
    assert_eq!(site.value, Some(0.));
    assert!(matches!(
        c.layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == site.mark_id)
            .unwrap()
            .geometry,
        Geometry::Rect { height: 0., .. }
    ));
    assert_eq!(
        c.hit_test(site.position[0], site.position[1])?,
        Some(&BoxPlotKey::new("g", ""))
    );
    let frame = c.scene_interaction()?;
    surface::SurfaceModel::new_with_interaction(
        frame.scene,
        surface::SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction,
    )?;
    c.set_options(&BoxPlotInteractionOptions {
        opacity: 0.,
        ..Default::default()
    })?;
    assert_eq!(c.hit_test(site.position[0], site.position[1])?, None);
    assert_eq!(c.interaction_binding()?.sites.len(), 1);
    assert_eq!(without_groups(c.layout), old);
    Ok(())
}
#[test]
fn whole_frame_guard_rejects_source_paint_geometry_order_and_role_tampering_then_allows_exact_repair()
-> Result<()> {
    let baseline = build()?;
    for mutation in 0..18 {
        let mut c = baseline.clone();
        match mutation {
            0 => c.layout.boxes[0].group = "forged".into(),
            1 => c.layout.boxes[0].subgroup = "forged".into(),
            2 => c.layout.boxes[0].quantiles[2] += 1.,
            3 => c.layout.boxes[0].source_ids[0] = "forged".into(),
            4 => c.layout.domain[0] -= 1.,
            5 => c.layout.groups.reverse(),
            6 => c.layout.subgroups.reverse(),
            7 => c.layout.scene.marks[0].opacity = 0.2,
            8 => c.layout.scene.marks[0].fill = Some(Color(1, 2, 3, 4)),
            9 => {
                c.layout.scene.marks[0].geometry = Geometry::Rect {
                    x: 0.,
                    y: 0.,
                    width: 1.,
                    height: 1.,
                }
            }
            10 => c.layout.scene.marks[0].clip = None,
            11 => c.layout.scene.marks[0].label = "forged".into(),
            12 => c.layout.scene.marks.swap(0, 1),
            13 => {
                c.layout.scene.draw_order = c.layout.scene.draw_items().into_iter().rev().collect()
            }
            14 => c
                .layout
                .scene
                .legend_roles
                .insert(c.layout.scene.legend[0].id.clone(), LegendRole::Category)
                .map(|_| ())
                .unwrap_or(()),
            15 => c.layout.boxes[0].outliers[0].value += 1.,
            16 => c.layout.scene.opacity_groups.push(MarkOpacityGroup {
                id: "forged".into(),
                members: vec![c.layout.boxes[0].id.clone()],
                opacity: 0.5,
            }),
            _ => c.layout.boxes[0].count += 1,
        }
        let corrupt = c.layout.clone();
        assert!(c.set_active(Some(key())).is_err(), "mutation {mutation}");
        assert!(c.interaction_binding().is_err());
        assert!(c.hit_test(10., 10.).is_err());
        assert!(c.replace_source(&config(), &data(), &options()).is_err());
        assert_eq!(c.layout, corrupt);
        c.layout = baseline.layout.clone();
        c.set_active(Some(key()))?;
    }
    Ok(())
}
#[test]
fn source_replacement_rebuilds_transactionally_retains_live_key_and_clears_removed_key()
-> Result<()> {
    let mut c = build()?;
    c.set_active(Some(key()))?;
    let before = c.layout.clone();
    let original = data();
    let mut bad = original.clone();
    bad[0].value = Some(f64::NAN);
    assert!(c.replace_source(&config(), &bad, &options()).is_err());
    assert_eq!(c.layout, before);
    assert_eq!(c.active(), Some(&key()));
    assert_eq!(original, data());
    let mut resized = config();
    resized.width = 500.;
    c.replace_source(&resized, &original, &options())?;
    assert_eq!(c.active(), Some(&key()));
    assert_eq!(c.layout.scene.width, 500.);
    let mut hidden = options();
    hidden.hidden.insert(boxplot_series_id("y"));
    c.replace_source(&resized, &original, &hidden)?;
    assert!(c.active().is_none());
    assert!(c.layout.scene.opacity_groups.is_empty());
    assert!(c.layout.boxes.iter().all(|b| b.subgroup != "y"));
    c.replace_source(&config(), &original, &options())?;
    c.set_active(Some(key()))?;
    Ok(())
}
#[test]
fn shared_scale_replacement_and_rounded_anchors_retain_actual_source_coordinates() -> Result<()> {
    let mut o = options();
    o.border_radius = 8.;
    o.domain = Some([1., 128.]);
    let data = data()
        .into_iter()
        .map(|mut d| {
            d.value = d.value.map(|v| v + 1.);
            d
        })
        .collect::<Vec<_>>();
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Log,
        reverse: true,
        round: true,
        ..Default::default()
    };
    let old = boxplot_layout_with_scale_options(&config(), &data, &o, &scale)?;
    let mut c =
        BoxPlotInteraction::with_scale_options(&config(), &data, &o, &scale, &Default::default())?;
    assert_eq!(c.layout, old.layout);
    assert!(c.resolved_value_scale().is_some());
    assert!(!c.interaction_binding()?.anchors.is_empty());
    c.set_active(Some(key()))?;
    let mut resized = config();
    resized.height = 340.;
    c.replace_source(&resized, &data, &o)?;
    assert_eq!(c.active(), Some(&key()));
    let mut invalid = scale;
    invalid.kind = ScaleKind::Pow { exponent: 0. };
    let before = c.layout.clone();
    assert!(
        c.replace_source_with_scale_options(&config(), &data, &o, &invalid)
            .is_err()
    );
    assert_eq!(c.layout, before);
    c.scene_interaction()?.validate()?;
    Ok(())
}
#[test]
fn invalid_opacity_options_reject_empty_sources_without_inventing_statistics() -> Result<()> {
    for opacity in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(
            BoxPlotInteraction::new(
                &config(),
                &[],
                &Default::default(),
                &BoxPlotInteractionOptions {
                    opacity,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let mut c = BoxPlotInteraction::new(&config(), &[], &Default::default(), &Default::default())?;
    assert!(c.layout.boxes.is_empty());
    assert!(c.interaction_binding()?.sites.is_empty());
    assert!(c.set_active(Some(BoxPlotKey::new("g", "s"))).is_err());
    c.set_active(None)?;
    c.scene_interaction()?.validate()?;
    Ok(())
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let t = app.tree();
    let mut stack = t.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = vec![];
    while let Some(id) = stack.pop() {
        let n = t.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        stack.extend(n.children.iter().rev().copied());
    }
    out
}
#[test]
fn semantic_group_wrappers_update_once_and_highlight_false_keeps_events_without_extra_dim() {
    let c = BoxPlotInteraction::new(
        &config(),
        &data(),
        &options(),
        &BoxPlotInteractionOptions {
            opacity: 0.8,
            active_opacity: 0.9,
            inactive_opacity: 0.2,
        },
    )
    .unwrap();
    let c = Rc::new(RefCell::new(c));
    let app = App::memory();
    let scene = app.scope().cage(c.borrow().layout.scene.clone());
    let controlled = app.scope().cage(None::<String>);
    let records = Rc::new(RefCell::new(Vec::new()));
    let a = c.clone();
    let e = records.clone();
    let f = records.clone();
    let s = records.clone();
    app.mount(
        chart(scene)
            .legend(false)
            .tooltip(false)
            .highlight(false)
            .hovered(controlled)
            .on_hover(move |mark| {
                e.borrow_mut().push("hover");
                a.borrow_mut()
                    .set_hovered_mark(mark.as_ref().map(|m| m.id.as_str()))
                    .unwrap();
                scene.revise(|v| *v = a.borrow().layout.scene.clone());
            })
            .on_focus(move |_| f.borrow_mut().push("focus"))
            .on_select(move |_| s.borrow_mut().push("select")),
    )
    .unwrap();
    assert_eq!(nodes(&app, "kartis-opacity-group").len(), 8);
    let button = nodes(&app, "kartis-datum")[0];
    app.dispatch(
        button,
        Event::Pointer {
            from: button,
            related: None,
            phase: PointerPhase::Enter,
        },
    )
    .unwrap();
    app.flush().unwrap();
    assert!(c.borrow().active().is_some());
    for id in nodes(&app, "kartis-mark") {
        let t = app.tree();
        let n = t.node(id).unwrap();
        let v = n.descriptor.view();
        assert!(v.style.opacity == Some(0.4) || v.style.opacity == Some(1.));
        assert_eq!(v.style.left, Some(Length::Px(-28.)));
    }
    app.dispatch(button, Event::FocusChanged(true)).unwrap();
    app.dispatch(button, Event::Activate).unwrap();
    app.flush().unwrap();
    assert!(records.borrow().contains(&"hover"));
    assert!(records.borrow().contains(&"focus"));
    assert!(records.borrow().contains(&"select"));
    for id in nodes(&app, "kartis-mark") {
        let t = app.tree();
        assert!(matches!(
            t.node(id).unwrap().descriptor.view().style.opacity,
            Some(0.4) | Some(1.)
        ));
    }
    c.borrow_mut().set_options(&Default::default()).unwrap();
    c.borrow_mut().set_active(None).unwrap();
    scene.revise(|v| *v = c.borrow().layout.scene.clone());
    app.flush().unwrap();
    assert!(nodes(&app, "kartis-opacity-group").is_empty());
    assert_eq!(
        nodes(&app, "kartis-mark-clip").len(),
        c.borrow().layout.scene.marks.len()
    );
}
#[cfg(feature = "raster")]
#[test]
fn group_composites_fill_and_median_once_rather_than_two_per_mark_opacities() -> Result<()> {
    let c = ChartConfig {
        width: 240.,
        height: 160.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let data = (0..5)
        .map(|i| BoxObservation::new(format!("p{i}"), "g", "s", Some(i as f64)))
        .collect::<Vec<_>>();
    let o = BoxPlotLayoutOptions {
        domain: Some([0., 4.]),
        colors: [("s".into(), Color(0, 0, 255, 255))].into(),
        fill_opacity: 1.,
        border: None,
        median: stroke(Color(0, 0, 0, 255), 12.),
        whisker: stroke(Color(0, 0, 0, 255), 0.),
        cap_size: 0.,
        legend: false,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let state = BoxPlotInteraction::new(
        &c,
        &data,
        &o,
        &BoxPlotInteractionOptions {
            opacity: 0.25,
            ..Default::default()
        },
    )?;
    assert!(state.layout.scene.marks.iter().all(|m| m.opacity == 1.));
    let image = render::raster::render_raster(
        &state.layout.scene,
        &render::raster::FontRegistry::new(),
        &Default::default(),
    )?;
    for y in 78..83 {
        for x in 118..123 {
            let i = (y * 240 + x) * 4;
            let p = &image.rgba[i..i + 4];
            assert_eq!(p[0], p[1]);
            assert_eq!(p[1], p[2]);
            assert!(
                p[0].abs_diff(191) <= 1,
                "source-over group black 0.25 on white: {p:?}"
            );
            assert_eq!(p[3], 255);
        }
    }
    Ok(())
}

fn index_probe_scene() -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    for i in 0..3 {
        let mut m = Mark::new(
            format!("m{i}"),
            Geometry::Rect {
                x: 20. + i as f64 * 40.,
                y: 30.,
                width: 25.,
                height: 35.,
            },
            Color(20, 40, 60, 255),
        );
        m.series = format!("s{i}");
        m.label = format!("datum{i}");
        s.marks.push(m);
    }
    s
}
fn index_probe_nodes(app: &App, class: &str) -> Vec<NodeId> {
    let t = app.tree();
    let mut stack = t.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = stack.pop() {
        let n = t.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        stack.extend(n.children.iter().copied());
    }
    out.reverse();
    out
}
#[test]
fn semantic_zero_group_reconciles_controls_hover_hidden_rejection_and_restore() {
    let app = App::memory();
    let source = app.scope().cage(index_probe_scene());
    let hidden = app.scope().cage(std::collections::BTreeSet::new());
    let hover = Rc::new(RefCell::new(Vec::new()));
    let h = hover.clone();
    let reject = Rc::new(RefCell::new(Vec::new()));
    let r = reject.clone();
    app.mount(
        chart(source)
            .legend(false)
            .tooltip(false)
            .highlight(false)
            .hidden(hidden)
            .on_hover(move |m| h.borrow_mut().push(m.map(|m| m.id)))
            .on_reject(move |e| r.borrow_mut().push(e.0)),
    )
    .unwrap();
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 3);
    let first = index_probe_nodes(&app, "kartis-datum")[0];
    app.dispatch(
        first,
        Event::Pointer {
            from: first,
            related: None,
            phase: PointerPhase::Enter,
        },
    )
    .unwrap();
    app.flush().unwrap();
    assert_eq!(hover.borrow().last().unwrap(), &Some("m0".into()));
    source.revise(|s| {
        s.opacity_groups = vec![MarkOpacityGroup {
            id: "g".into(),
            members: vec!["m0".into(), "m1".into()],
            opacity: 0.,
        }]
    });
    app.flush().unwrap();
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 1);
    assert_eq!(hover.borrow().last().unwrap(), &None);
    let accepted = source.get_untracked();
    source.revise(|s| s.opacity_groups[0].members[0] = "unknown".into());
    app.flush().unwrap();
    assert_eq!(reject.borrow().len(), 1);
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 1);
    source.revise(|s| *s = accepted.clone());
    app.flush().unwrap();
    hidden.revise(|h| {
        h.insert("s2".into());
    });
    app.flush().unwrap();
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 0);
    hidden.revise(|h| h.clear());
    app.flush().unwrap();
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 1);
    source.revise(|s| s.opacity_groups.clear());
    app.flush().unwrap();
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 3);
    assert!(index_probe_nodes(&app, "kartis-opacity-group").is_empty());
    source.revise(|s| {
        s.opacity_groups = vec![MarkOpacityGroup {
            id: "g".into(),
            members: vec!["m0".into(), "m1".into()],
            opacity: 0.5,
        }];
        s.marks[2].opacity = 0.;
    });
    app.flush().unwrap();
    assert_eq!(index_probe_nodes(&app, "kartis-datum").len(), 2);
}
