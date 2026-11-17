use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, GraphicPoint as P, NodeId, PathCommand as C, Props};
use glory::{App, Widget};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 25.,
            bottom: 25.,
        },
        ..Default::default()
    }
}
fn categories() -> Vec<String> {
    vec!["A".into(), "B".into(), "猫".into()]
}
fn data() -> Vec<BarSeries> {
    vec![
        BarSeries::new("z", vec![Some(-0.), None, Some(-3.)]),
        BarSeries::new("a", vec![Some(2.), Some(4.), Some(6.)]),
    ]
}
fn options() -> BarOptions {
    let mut o = BarOptions::default();
    o.category_axis.visible = false;
    o.value_axis.visible = false;
    o.value_axis.grid = false;
    o
}
fn placement(_: &BarWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-12., -8.],
        measured: TextSize {
            width: 24.,
            height: 16.,
        },
    }))
}
fn custom() -> CustomBarLayout {
    bar_layout_with_factories(
        &config(),
        &categories(),
        &data(),
        &options(),
        &Default::default(),
        Default::default(),
    )
    .unwrap()
}

#[test]
fn default_factories_keep_all_ordinary_bar_configuration_and_share_complete_source() {
    for horizontal in [false, true] {
        for mode in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            for radius in [0., 8.] {
                let o = BarOptions {
                    horizontal,
                    layout: mode,
                    corner_radius: radius,
                    category_window: Some([0.15, 0.85]),
                    ..options()
                };
                let style = BarStyleOptions {
                    labels: BarLabels {
                        enabled: true,
                        ..Default::default()
                    },
                    totals: BarTotals {
                        enabled: true,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let plain =
                    bar_layout_with_style(&config(), &categories(), &data(), &o, &style).unwrap();
                let layout = bar_layout_with_factories(
                    &config(),
                    &categories(),
                    &data(),
                    &o,
                    &style,
                    Default::default(),
                )
                .unwrap();
                assert_eq!(layout.layout, plain);
                let before = layout.layout.clone();
                let packet = bar_widgets(&layout, 7, |c| {
                    assert!(std::ptr::eq(c.layout, &layout));
                    assert_eq!(c.series, &data()[c.bar.source_series]);
                    assert_eq!(c.category, categories()[c.bar.source_category]);
                    assert_eq!(c.horizontal, horizontal);
                    assert_eq!(
                        c.position,
                        layout
                            .interaction_binding()
                            .unwrap()
                            .sites
                            .iter()
                            .find(|s| s.mark_id == c.bar.id)
                            .unwrap()
                            .position
                    );
                    placement(c)
                })
                .unwrap();
                assert_eq!(layout.layout, before);
                assert_eq!(packet.data.len(), plain.bars.len());
                assert_eq!(packet.layout.plot, Some(ClipRect::plot(&config())));
                let frame = bar_widget_frame(&layout).unwrap();
                assert_eq!(frame.scene, plain.scene);
                assert_eq!(
                    frame.interaction.as_ref().unwrap().picking_profile,
                    PickingProfile::PaintedGeometry
                );
                assert_eq!(
                    frame.interaction.as_ref().unwrap().sites.len(),
                    plain.bars.len()
                );
                assert_eq!(
                    render::to_svg(&frame.scene).unwrap(),
                    render::to_svg(&plain.scene).unwrap()
                );
                let all = packet.data.values().collect::<Vec<_>>();
                for a in &all {
                    assert_eq!(*a.categories, categories());
                    assert_eq!(*a.series, data()[a.bar.source_series]);
                    assert_eq!(
                        a.series.values[a.bar.source_category].unwrap().to_bits(),
                        a.bar.raw_value.to_bits()
                    );
                    assert!(!a.series.values.is_empty());
                    for b in &all {
                        assert!(Arc::ptr_eq(&a.categories, &b.categories));
                        if a.bar.source_series == b.bar.source_series {
                            assert!(Arc::ptr_eq(&a.series, &b.series));
                        }
                    }
                }
                assert!(
                    !all.iter()
                        .any(|d| d.bar.source_series == 0 && d.bar.source_category == 1)
                );
                let zero = all
                    .iter()
                    .find(|d| d.bar.source_series == 0 && d.bar.source_category == 0)
                    .unwrap();
                assert_eq!(zero.bar.raw_value.to_bits(), (-0_f64).to_bits());
                assert!(all.iter().any(|d| d.series.values[1].is_none()));
            }
        }
    }
}

#[test]
fn all_factory_entrypoints_capture_original_rect_endpoints_and_true_empty_plot() {
    for horizontal in [false, true] {
        for label_factory in [false, true] {
            for scaled in [false, true] {
                let mut captures = BTreeMap::new();
                let mut geometry = |c: &BarDatumContext<'_>| {
                    captures.insert(
                        c.mark_id.to_string(),
                        (c.position, c.original_rect, c.endpoints, c.horizontal),
                    );
                    Ok(c.inherit(c.geometry.clone()))
                };
                let mut label = |c: &BarLabelContext<'_>| Ok(c.default_text.to_string());
                let o = BarOptions {
                    horizontal,
                    value_domain: Some([-2., 8.]),
                    category_window: Some([0.1, 0.9]),
                    ..options()
                };
                let style = BarStyleOptions {
                    labels: BarLabels {
                        enabled: true,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let scales = ContinuousScaleOptions {
                    nice: ScaleNice::Off,
                    reverse: true,
                    round: true,
                    clamp: true,
                    ..Default::default()
                };
                let factories = BarFactories {
                    bar: Some(&mut geometry),
                    label: if label_factory {
                        Some(&mut label)
                    } else {
                        None
                    },
                };
                let layout = if scaled {
                    bar_layout_with_scale_options_and_factories(
                        &config(),
                        &categories(),
                        &data(),
                        &o,
                        &style,
                        &scales,
                        factories,
                    )
                    .unwrap()
                    .layout
                } else {
                    bar_layout_with_factories(
                        &config(),
                        &categories(),
                        &data(),
                        &o,
                        &style,
                        factories,
                    )
                    .unwrap()
                };
                let packet = bar_widgets(&layout, 0, placement).unwrap();
                assert_eq!(packet.data.len(), captures.len());
                for d in packet.data.values() {
                    assert_eq!(
                        (d.position, d.original_rect, d.endpoints, d.horizontal),
                        captures[&d.bar.id]
                    );
                    assert_eq!(d.horizontal, horizontal);
                }
                assert!(
                    packet
                        .data
                        .values()
                        .any(|d| d.original_rect != Some(d.bar.visible_rect)),
                    "cropping must not replace original geometry metadata"
                );
            }
        }
    }
    for kind in 0..4 {
        for labels in [false, true] {
            for scaled in [false, true] {
                let cats = if kind == 0 { vec![] } else { categories() };
                let source = if kind == 0 {
                    vec![]
                } else if kind == 1 {
                    vec![BarSeries::new("none", vec![None; 3])]
                } else {
                    data()
                };
                let mut o = options();
                if kind == 2 {
                    o.hidden = source.iter().map(|s| s.id.clone()).collect();
                }
                if kind == 3 {
                    o.value_domain = Some([10., 20.]);
                }
                let mut label = |c: &BarLabelContext<'_>| Ok(c.default_text.to_string());
                let factories = BarFactories {
                    bar: None,
                    label: if labels { Some(&mut label) } else { None },
                };
                let layout = if scaled {
                    bar_layout_with_scale_options_and_factories(
                        &config(),
                        &cats,
                        &source,
                        &o,
                        &Default::default(),
                        &ContinuousScaleOptions::default(),
                        factories,
                    )
                    .unwrap()
                    .layout
                } else {
                    bar_layout_with_factories(
                        &config(),
                        &cats,
                        &source,
                        &o,
                        &Default::default(),
                        factories,
                    )
                    .unwrap()
                };
                assert!(layout.layout.bars.is_empty());
                let packet =
                    bar_widgets(&layout, 0, |_| panic!("absent bar role invoked placement"))
                        .unwrap();
                assert!(packet.data.is_empty());
                assert_eq!(packet.layout.plot, Some(ClipRect::plot(&config())));
                let frame = bar_widget_frame(&layout).unwrap();
                assert_eq!(frame.plot, packet.layout.plot);
                assert_eq!(frame.scene, layout.layout.scene);
                assert!(frame.interaction.unwrap().sites.is_empty());
            }
        }
    }
}

#[test]
fn controlled_opacity_keeps_original_widget_data_and_private_frame_preflight_is_complete() {
    let mut layout = custom();
    let original = bar_widgets(&layout, 0, placement).unwrap();
    let active = layout.layout.bars[0].id.clone();
    layout.set_active(Some(&active), 0.).unwrap();
    let updated = bar_widgets(&layout, 0, placement).unwrap();
    assert_eq!(updated, original);
    let frame = bar_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, layout.layout.scene);
    assert!(
        frame
            .scene
            .marks
            .iter()
            .filter(|m| m.id != active && m.id.starts_with("styled-bar:"))
            .all(|m| m.opacity == 0.)
    );
    let mut opacity_seen = vec![];
    bar_widgets(&layout, 0, |c| {
        opacity_seen.push((c.bar.id.clone(), c.mark.opacity));
        Ok(None)
    })
    .unwrap();
    assert!(opacity_seen.iter().any(|(id, a)| id != &active && *a == 0.));
    type Mutate = Box<dyn FnOnce(&mut CustomBarLayout)>;
    let changes: Vec<Mutate> = vec![
        Box::new(|l| l.layout.bars[4].raw_value = 91.),
        Box::new(|l| l.layout.bars[4].source_series = 99),
        Box::new(|l| l.layout.bars[4].source_category = 99),
        Box::new(|l| l.layout.bars[4].category = "forged".into()),
        Box::new(|l| l.layout.bars[4].displayed_value += 1.),
        Box::new(|l| l.layout.bars[4].visible_rect.width = f64::NAN),
        Box::new(|l| l.layout.scene.marks.last_mut().unwrap().opacity = 0.5),
        Box::new(|l| l.layout.scene.marks.last_mut().unwrap().series = "fake".into()),
        Box::new(|l| {
            l.layout.bars.pop();
        }),
        Box::new(|l| l.layout.value_domain[0] -= 1.),
        Box::new(|l| {
            let b = l
                .layout
                .bars
                .iter_mut()
                .find(|b| b.source_series == 0 && b.source_category == 0)
                .unwrap();
            b.raw_value = 0.;
        }),
    ];
    for change in changes {
        let mut candidate = custom();
        change(&mut candidate);
        let calls = Cell::new(0);
        assert!(
            bar_widgets(&candidate, 0, |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
        assert!(bar_widget_frame(&candidate).is_err());
    }
}

#[test]
fn borrowed_placement_errors_skip_and_slot_budget_are_atomic() {
    let layout = custom();
    let before = layout.layout.clone();
    let mut calls = 0;
    assert!(
        bar_widgets(&layout, 0, |c| {
            calls += 1;
            if calls == 4 {
                Err(ChartError("placement refused".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 4);
    assert_eq!(layout.layout, before);
    let cats = (0..130).map(|v| v.to_string()).collect::<Vec<_>>();
    let source = [BarSeries::new("large", vec![Some(1.); 130])];
    let layout = bar_layout_with_factories(
        &config(),
        &cats,
        &source,
        &options(),
        &Default::default(),
        Default::default(),
    )
    .unwrap();
    let mut calls = 0;
    assert!(
        bar_widgets(&layout, 0, |c| {
            calls += 1;
            placement(c)
        })
        .unwrap_err()
        .0
        .contains("budget")
    );
    assert_eq!(calls, 129);
    assert_eq!(
        bar_widgets(&layout, 0, |c| if c.bar.source_category < 128 {
            placement(c)
        } else {
            Ok(None)
        })
        .unwrap()
        .data
        .len(),
        128
    );
    assert!(
        bar_widgets(&layout, 0, |_| Ok(None))
            .unwrap()
            .data
            .is_empty()
    );
}

#[test]
fn actual_owned_labels_keep_factory_text_empty_roles_zero_policy_clip_and_current_dim_color() {
    for empty in [false, true] {
        let mut label = |c: &BarLabelContext<'_>| {
            Ok(if empty {
                String::new()
            } else {
                format!("Custom {}", c.default_text)
            })
        };
        let style = BarStyleOptions {
            labels: BarLabels {
                enabled: true,
                style: BarTextStyle {
                    color: Some(Color(10, 20, 30, 200)),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let mut layout = bar_layout_with_factories(
            &config(),
            &categories(),
            &data(),
            &options(),
            &style,
            BarFactories {
                bar: None,
                label: Some(&mut label),
            },
        )
        .unwrap();
        let before = layout.layout.scene.clone();
        let packet = bar_widgets(&layout, 0, |c| {
            if let Some(index) = c.bar.label_index {
                assert!(std::ptr::eq(
                    c.label.unwrap(),
                    &layout.layout.scene.labels[index]
                ));
                assert!(std::ptr::eq(
                    c.label_clip.unwrap(),
                    &layout.layout.scene.label_clips[&index]
                ));
            } else {
                assert!(c.label.is_none());
                assert!(c.label_clip.is_none());
            }
            placement(c)
        })
        .unwrap();
        assert_eq!(layout.layout.scene, before);
        assert_eq!(
            packet.data.values().filter(|d| d.label.is_some()).count(),
            4
        );
        let zero = packet
            .data
            .values()
            .find(|d| d.bar.raw_value == 0.)
            .unwrap();
        assert!(zero.label.is_none());
        for d in packet.data.values().filter(|d| d.label.is_some()) {
            let label = d.label.as_ref().unwrap();
            assert_eq!(label.text.is_empty(), empty);
            if !empty {
                assert!(label.text.starts_with("Custom "));
            }
            assert!(d.label_clip.is_some());
        }
        let active = layout.layout.bars[0].id.clone();
        layout.set_active(Some(&active), 0.).unwrap();
        let dimmed = bar_widgets(&layout, 0, placement).unwrap();
        for (id, d) in &dimmed.data {
            let old = &packet.data[id];
            assert_eq!(d.bar, old.bar);
            assert!(Arc::ptr_eq(&d.series, &old.series));
            assert!(Arc::ptr_eq(&d.categories, &old.categories));
            assert_eq!(d.position, old.position);
            assert_eq!(d.label_clip, old.label_clip);
            if let Some(label) = &d.label {
                assert_eq!(label.text, old.label.as_ref().unwrap().text);
                assert_eq!(label.color.3, if id == &active { 200 } else { 0 });
            }
        }
        assert_eq!(
            bar_widget_frame(&layout).unwrap().scene.labels,
            layout.layout.scene.labels
        );
    }
    let style = BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            zero_extent: BarZeroExtentLabels::ValueAxis,
            ..Default::default()
        },
        ..Default::default()
    };
    let layout = bar_layout_with_factories(
        &config(),
        &categories(),
        &data(),
        &options(),
        &style,
        Default::default(),
    )
    .unwrap();
    let packet = bar_widgets(&layout, 0, placement).unwrap();
    assert!(
        packet
            .data
            .values()
            .find(|d| d.bar.raw_value == 0.)
            .unwrap()
            .label
            .is_some(),
        "explicit Rust zero-extent label policy retains its real owned label role"
    );
}

fn frame(
    value: f64,
    width: f64,
    revision: u64,
    path: bool,
) -> (SceneInteraction, PlotWidgetsComposition<BarWidgetData>) {
    let config = ChartConfig { width, ..config() };
    let source = [
        BarSeries::new("alpha", vec![Some(value), None, Some(0.)]),
        BarSeries::new("beta", vec![Some(2.), Some(-3.), Some(4.)]),
    ];
    let o = BarOptions {
        value_domain: Some([-5., 10.]),
        ..options()
    };
    let layout = if path {
        bar_layout_with_bar_factory(
            &config,
            &categories(),
            &source,
            &o,
            &Default::default(),
            |c| {
                let [x, y] = c.position;
                Ok(c.inherit(Geometry::Path(vec![
                    C::Move(P::new(x + 10., y - 5.)),
                    C::Line(P::new(x + 22., y - 5.)),
                    C::Line(P::new(x + 22., y + 5.)),
                    C::Line(P::new(x + 10., y + 5.)),
                    C::Close,
                ])))
            },
        )
        .unwrap()
    } else {
        bar_layout_with_factories(
            &config,
            &categories(),
            &source,
            &o,
            &Default::default(),
            Default::default(),
        )
        .unwrap()
    };
    let widgets = bar_widgets(&layout, revision, placement).unwrap();
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    (
        bar_widget_frame(&layout).unwrap(),
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let t = app.tree();
    let mut q = t.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = q.pop() {
        let n = t.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        q.extend(n.children.iter().copied());
    }
    out
}
fn label(app: &App, id: NodeId) -> String {
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
    while let Some(id) = q.pop() {
        let n = t.node(id).unwrap();
        out.push(format!("{id:?}:{n:?}"));
        q.extend(n.children.iter().copied());
    }
    format!("{:?}:{out:?}", t.roots())
}
fn child(
    context: PlotWidgetCompositionContext<BarWidgetData>,
    cleanup: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    if context.revision == 90 {
        return Err(ChartError("bar factory refused".into()));
    }
    let frame = context.frame;
    let revision = context.revision;
    Ok(glory::ui::widget(move |scope| {
        let cleanup = cleanup.clone();
        scope.on_cleanup(move || cleanup.set(cleanup.get() + 1))?;
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(frame.with(|f| {
                format!(
                    "bar-{}-{}",
                    f.data.bar.source_series, f.data.bar.source_category
                )
            }))
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "raw={}|whole={:?}|category={}|count={}|focus={}",
                        f.data.bar.raw_value,
                        f.data.series.values,
                        f.data.categories[f.data.bar.source_category],
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
            .on(EventKind::FocusChanged, move |event| {
                if let Event::FocusChanged(value) = event {
                    focus.revise(|v| *v = value);
                }
            })
            .show_in(scope)?;
        if revision == 91 {
            return Err(glory::render::Error::InvalidProps("bar child build refused").into());
        }
        Ok(())
    }))
}

#[test]
fn real_memory_state_raw_data_hidden_and_revision_updates_follow_accepted_frame() {
    let app = App::memory();
    let source = app.scope().cage(frame(4., 360., 0, false));
    let scene = app.scope().bond(move || source.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || source.with(|s| s.1.clone()));
    let hidden = app.scope().cage(BTreeSet::new());
    let cleanup = Rc::new(Cell::new(0));
    let clean = cleanup.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(scene)
            .legend(false)
            .hidden(hidden)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |context| child(context, clean.clone())),
    )
    .unwrap();
    assert!(nodes(&app, "bar-0-1").is_empty());
    let id = nodes(&app, "bar-0-0")[0];
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    source.revise(|s| *s = frame(7., 420., 0, false));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "bar-0-0"), [id]);
    assert!(label(&app, id).contains("raw=7|whole=[Some(7.0), None, Some(0.0)]"));
    assert!(label(&app, id).contains("count=1|focus=true"));
    hidden.revise(|h| {
        h.insert("alpha".into());
    });
    app.flush().unwrap();
    assert!(app.tree().inert(id));
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert!(!app.tree().inert(id));
    for revision in [90, 91] {
        let before = tree(&app);
        source.revise(|s| *s = frame(4., 360., revision, false));
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
    }
    assert_eq!(errors.borrow().len(), 2);
    source.revise(|s| *s = frame(7., 420., 1, false));
    app.flush().unwrap();
    let next = nodes(&app, "bar-0-0")[0];
    assert_ne!(next, id);
    assert!(label(&app, next).contains("count=0|focus=false"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(cleanup.get(), 11);
}

#[test]
fn actual_path_widgets_follow_original_captured_position_not_custom_ink_center() {
    let (scene, packet) = frame(4., 360., 0, true);
    let app = App::memory();
    let captured = Rc::new(RefCell::new(vec![]));
    let frames = captured.clone();
    app.mount(
        chart_layout(scene)
            .legend(false)
            .plot_widgets_composed(packet, move |context| {
                frames.borrow_mut().push(context.frame);
                Ok(glory::ui::button().label("bar path"))
            }),
    )
    .unwrap();
    for frame in captured.borrow().iter() {
        frame.with(|f| {
            assert_eq!(f.frame.rect.x, f.data.position[0] - 12.);
            assert_eq!(f.frame.rect.y, f.data.position[1] - 8.);
            let Geometry::Path(commands) = &f.frame.mark.geometry else {
                panic!("Path expected")
            };
            let C::Move(first) = commands[0] else {
                panic!("Move expected")
            };
            assert_eq!(first.x, f.data.position[0] + 10.);
            assert!(f.data.original_rect.is_some());
        });
    }
    assert_eq!(captured.borrow().len(), 5);
    app.clear().unwrap();
}

#[cfg(feature = "raster")]
#[test]
fn cpu_publisher_error_retains_png_binding_raw_source_geometry_and_child_focus() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let source = app.scope().cage((
            frame(4., 360., 0, false),
            RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        ));
        let scene = app.scope().bond(move || source.with(|s| s.0.0.clone()));
        let packet = app.scope().bond(move || source.with(|s| s.0.1.clone()));
        let options = app.scope().bond(move || source.with(|s| s.1.clone()));
        let fail = Rc::new(Cell::new(false));
        let refused = fail.clone();
        let cleanup = Rc::new(Cell::new(0));
        let clean = cleanup.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        app.mount(
            chart_layout(scene)
                .raster(FontRegistry::new(), options)
                .legend(false)
                .highlight(false)
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .raster_image_source(move |image| {
                    if refused.get() {
                        Err(ChartError("Bar PNG refused".into()))
                    } else {
                        image.data_url()
                    }
                })
                .plot_widgets_composed(packet, move |context| child(context, clean.clone())),
        )
        .unwrap();
        let image = |app: &App| {
            let t = app.tree();
            let mut q = t.roots().to_vec();
            while let Some(id) = q.pop() {
                let n = t.node(id).unwrap();
                if let Props::Image(p) = n.descriptor.view().props {
                    return p.source.clone();
                }
                q.extend(n.children.iter().copied());
            }
            panic!("Image expected")
        };
        let id = nodes(&app, "bar-0-0")[0];
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        let before = tree(&app);
        let png = image(&app);
        {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(png.split_once(',').unwrap().1)
                .unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
                .read_info()
                .unwrap();
            assert_eq!(reader.info().width, (360. * dpr) as u32);
            assert_eq!(reader.info().height, (240. * dpr) as u32);
            let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
            assert_eq!(
                reader.next_frame(&mut decoded).unwrap().color_type,
                png::ColorType::Rgba
            );
            assert_eq!(&decoded[..4], &[255, 255, 255, 255]);
        }
        fail.set(true);
        source.revise(|s| {
            s.0 = frame(7., 420., 0, false);
            s.1.view = ZoomTransform::new(1.25, -12., -8.).unwrap();
        });
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(image(&app), png);
        assert!(label(&app, id).contains("raw=4"));
        assert_eq!(errors.borrow().len(), 1);
        fail.set(false);
        source.revise(|s| s.0 = frame(7., 420., 0, false));
        app.flush().unwrap();
        assert_eq!(nodes(&app, "bar-0-0"), [id]);
        assert!(label(&app, id).contains("raw=7"));
        assert!(label(&app, id).contains("count=1|focus=true"));
        assert_ne!(image(&app), png);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
