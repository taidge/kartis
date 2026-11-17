use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 17.,
            bottom: 29.,
        },
        ..Default::default()
    }
}
fn source() -> Vec<FunnelDatum> {
    let mut raw = vec![
        FunnelDatum::new(-0., 10.),
        FunnelDatum::new("0", 4.),
        FunnelDatum::new("zero", -0.),
    ];
    raw[0].label = "Original first".into();
    raw[0].size = Some(-0.);
    raw[1].size = Some(24.);
    raw
}
fn placement(_: &FunnelWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-8., -4.],
        measured: TextSize {
            width: 16.,
            height: 8.,
        },
    }))
}

#[test]
fn actual_order_complete_shared_owned_pools_and_closing_separators() {
    let raw = source();
    let layout = funnel_layout(&config(), &raw, &Default::default()).unwrap();
    let original = layout.clone();
    let mut order = vec![];
    let packet = funnel_widgets(&layout, &raw, 5, |c| {
        assert!(std::ptr::eq(c.layout, &layout));
        assert!(std::ptr::eq(c.source, raw.as_slice()));
        if let Some(i) = c.source_index {
            assert!(std::ptr::eq(c.datum.unwrap(), &raw[i]));
            assert!(std::ptr::eq(c.part.unwrap(), &layout.parts[i]));
        } else {
            assert!(c.datum.is_none() && c.part.is_none());
            assert!(c.separator.unwrap().part_id.is_none());
        }
        order.push(c.mark.id.clone());
        placement(c)
    })
    .unwrap();
    assert_eq!(layout, original);
    assert_eq!(
        order,
        layout
            .scene
            .paint_marks()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(packet.data.len(), 13);
    let first = packet.data.values().next().unwrap();
    assert!(
        packet
            .data
            .values()
            .all(|d| Arc::ptr_eq(&d.source, &first.source)
                && Arc::ptr_eq(&d.parts, &first.parts)
                && Arc::ptr_eq(&d.separators, &first.separators)
                && Arc::ptr_eq(&d.labels, &first.labels))
    );
    assert_eq!(first.source.len(), 3);
    assert_eq!(first.source[0].size.unwrap().to_bits(), (-0_f64).to_bits());
    assert!(
        matches!(first.source[0].id, CategoryKey::Number(n) if n.to_bits()==(-0_f64).to_bits())
    );
    assert!(matches!(first.source[1].id, CategoryKey::String(ref s) if s=="0"));
    assert_eq!(first.labels.labels, layout.scene.labels);
    assert_eq!(first.labels.clips, layout.scene.label_clips);
    assert_eq!(first.labels.series, layout.scene.label_series);
    assert_eq!(
        packet
            .data
            .values()
            .filter(|d| d.source_index.is_none())
            .count(),
        2
    );
    for slot in &packet.layout.slots {
        assert_eq!(slot.revision, 5);
        let data = &packet.data[&slot.mark_id];
        assert_eq!(
            slot.placement,
            PlotWidgetPlacement::Rect(LayoutRect {
                x: data.position[0] - 8.,
                y: data.position[1] - 4.,
                width: 16.,
                height: 8.
            })
        );
    }
    let frame = funnel_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, layout.scene);
    assert_eq!(frame.plot, Some(layout.plot()));
    let binding = frame.interaction.unwrap();
    assert_eq!(binding.sites.len(), 2);
    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
    let mut detached = first.source.clone();
    Arc::make_mut(&mut detached)[0].label.clear();
    assert_eq!(first.source[0].label, "Original first");
}

#[test]
fn direction_curve_size_current_profiles_and_explicit_paint_order() {
    let raw = source();
    for direction in [FunnelDirection::Horizontal, FunnelDirection::Vertical] {
        for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth] {
            for size in [
                FunnelSize::Value,
                FunnelSize::DatumPixels,
                FunnelSize::OrdinalPixels(vec![80., 0., 35.]),
            ] {
                for gap in [0., 5.3] {
                    let options = FunnelLayoutOptions {
                        direction,
                        interpolation,
                        size: size.clone(),
                        spacing: gap,
                        current: Some("0".into()),
                        current_size_extension: 8.2,
                        before: Some(FunnelSeparatorOptions {
                            length: 13.7,
                            offset: 3.2,
                            stroke: None,
                        }),
                        after: Some(FunnelSeparatorOptions {
                            length: 7.1,
                            offset: 2.3,
                            stroke: None,
                        }),
                        ..Default::default()
                    };
                    let mut layout = funnel_layout(&config(), &raw, &options).unwrap();
                    let packet = funnel_widgets(&layout, &raw, 0, placement)
                        .unwrap_or_else(|error| panic!("{direction:?}/{interpolation:?}/{size:?}/spacing={gap}: {error:?}; parts={:?}; separators={:?}",layout.parts,layout.separators));
                    assert_eq!(packet.data.len(), layout.scene.marks.len());
                    layout.scene.draw_order = layout.scene.draw_items().into_iter().rev().collect();
                    let packet = funnel_widgets(&layout, &raw, 0, placement).unwrap();
                    assert_eq!(
                        packet
                            .layout
                            .slots
                            .iter()
                            .map(|s| &s.mark_id)
                            .collect::<Vec<_>>(),
                        layout
                            .scene
                            .paint_marks()
                            .map(|m| &m.id)
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

#[test]
fn valid_small_positive_plot_at_large_scene_origin_is_not_rejected_by_subtraction() {
    let raw = source();
    for size in [1., 0.01, 0.000001] {
        let c = ChartConfig {
            width: 16000.,
            height: 12000.,
            margin: Margin {
                left: 16000. - size,
                right: 0.,
                top: 12000. - size,
                bottom: 0.,
            },
            ..Default::default()
        };
        for direction in [FunnelDirection::Horizontal, FunnelDirection::Vertical] {
            let o = FunnelLayoutOptions {
                direction,
                labels: None,
                ..Default::default()
            };
            let l = funnel_layout(&c, &raw, &o).unwrap();
            assert_eq!(
                funnel_widgets(&l, &raw, 0, placement)
                    .unwrap_or_else(|e| panic!("size={size}/{direction:?}: {e:?}"))
                    .data
                    .len(),
                l.scene.marks.len()
            );
        }
    }
}

#[test]
fn custom_offset_ink_keeps_original_center_and_transparent_actual_roles() {
    let raw = source();
    let layout = funnel_layout_with_stage_factory(&config(), &raw, &Default::default(), &mut |c| {
        let [x, y] = c.part.center;
        let mut out = c.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 20., y - 5.)),
            C::Line(P::new(x + 30., y - 5.)),
            C::Line(P::new(x + 30., y + 5.)),
            C::Line(P::new(x + 20., y + 5.)),
            C::Close,
        ]));
        out.retain_profile_border = false;
        out.body.fill = None;
        out.body.fill_paint = None;
        out.body.stroke = None;
        out.body.opacity = 0.;
        Ok(out)
    })
    .unwrap();
    let packet = funnel_widgets(&layout, &raw, 0, |c| {
        if c.role == FunnelWidgetRole::Body {
            assert_eq!(c.position, c.part.unwrap().center);
            assert_eq!(c.mark.opacity, 0.);
            assert!(c.mark.fill.is_none() && c.mark.stroke.is_none());
        }
        placement(c)
    })
    .unwrap();
    assert_eq!(
        packet
            .data
            .values()
            .filter(|d| d.role == FunnelWidgetRole::Body)
            .count(),
        2
    );
    let retained: Vec<_> = packet
        .data
        .values()
        .filter(|d| d.role == FunnelWidgetRole::Border)
        .collect();
    assert_eq!(
        retained.len(),
        1,
        "zero-width stage never invokes the body factory, so its independent border remains"
    );
    assert_eq!(retained[0].source_index, Some(2));
    assert_eq!(funnel_widget_frame(&layout).unwrap().scene, layout.scene);
}

#[test]
fn whole_frame_conflicts_and_late_source_errors_precede_all_callbacks() {
    type Mutation = fn(&mut FunnelLayout);
    let mutations: &[(&str, Mutation)] = &[
        ("part index", |l| l.parts[2].source_index = 1),
        ("late source", |l| l.parts[2].datum.value = 1.),
        ("raw size", |l| l.parts[0].datum.size = Some(0.)),
        ("raw id", |l| l.parts[0].datum.id = 0_f64.into()),
        ("duplicate part", |l| {
            l.parts[2].mark_id = l.parts[0].mark_id.clone()
        }),
        ("series", |l| l.parts[1].series = "other".into()),
        ("bounds", |l| l.parts[2].bounds.width += 1.),
        ("center", |l| l.parts[2].center[0] += 1.),
        ("profile", |l| l.parts[2].profile[3].after[1] += 1.),
        ("profile nan", |l| l.parts[2].profile[3].after[0] = f64::NAN),
        ("domain", |l| l.value_domain[1] += 1.),
        ("range", |l| l.size_range[1] += 1.),
        ("band", |l| l.bandwidth += 1.),
        ("current", |l| {
            l.parts[0].is_current = true;
            l.parts[1].is_current = true;
        }),
        ("orphan mark", |l| {
            l.scene.marks.push(Mark::new(
                "invented",
                Geometry::Circle {
                    x: 100.,
                    y: 100.,
                    radius: 0.,
                },
                Color(0, 0, 0, 0),
            ))
        }),
        ("missing separator", |l| {
            l.separators.pop();
        }),
        ("separator source", |l| {
            l.separators[2].part_id = Some("missing".into())
        }),
        ("separator midpoint", |l| l.separators[2].to[0] += 1.),
        ("separator tail", |l| l.separators[0].part_id = None),
        ("label owner", |l| {
            l.scene.label_series.insert(0, "missing".into());
        }),
        ("body series", |l| {
            let id = &l.parts[1].mark_id;
            l.scene
                .marks
                .iter_mut()
                .find(|m| &m.id == id)
                .unwrap()
                .series = "other".into();
        }),
        ("body value", |l| {
            let id = &l.parts[1].mark_id;
            l.scene
                .marks
                .iter_mut()
                .find(|m| &m.id == id)
                .unwrap()
                .value = 1.;
        }),
        ("late border", |l| {
            let m = l
                .scene
                .marks
                .iter_mut()
                .rev()
                .find(|m| m.id.starts_with("funnel:border:"))
                .unwrap();
            if let Geometry::Path(p) = &mut m.geometry {
                p[0] = C::Move(P::new(1., 1.));
            }
        }),
        ("late body anchor", |l| {
            let id = &l.parts[1].mark_id;
            let m = l.scene.marks.iter_mut().find(|m| &m.id == id).unwrap();
            if let Geometry::Path(p) = &mut m.geometry {
                p[0] = C::Move(P::new(1., 1.));
            }
        }),
    ];
    let raw = source();
    let baseline = funnel_layout(&config(), &raw, &Default::default()).unwrap();
    for (name, mutation) in mutations {
        let mut layout = baseline.clone();
        mutation(&mut layout);
        let mut calls = 0;
        let result = funnel_widgets(&layout, &raw, 0, |c| {
            calls += 1;
            placement(c)
        });
        assert!(result.is_err(), "{name}");
        assert_eq!(calls, 0, "{name}");
        if !["late source", "raw size", "raw id"].contains(name) {
            assert!(funnel_widget_frame(&layout).is_err(), "{name}");
        }
    }
    let mut bad = raw.clone();
    bad[2].size = Some(f64::NAN);
    let mut calls = 0;
    assert!(
        funnel_widgets(&baseline, &bad, 0, |c| {
            calls += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(calls, 0);
}

#[test]
fn exact_raw_bits_cover_unselected_source_parts_separators_and_value_allocations() {
    let raw = source();
    let layout = funnel_layout(&config(), &raw, &Default::default()).unwrap();
    let p = funnel_widgets(&layout, &raw, 0, placement).unwrap();
    let original = p.data.values().next().unwrap();
    assert_eq!(original, &original.clone());
    let mut a = original.clone();
    a.source = Arc::new((*a.source).clone());
    a.parts = Arc::new((*a.parts).clone());
    a.separators = Arc::new((*a.separators).clone());
    assert_eq!(original, &a);
    Arc::make_mut(&mut a.source)[2].value = 0.;
    assert_ne!(original, &a);
    let mut a = original.clone();
    Arc::make_mut(&mut a.source)[0].size = Some(0.);
    assert_ne!(original, &a);
    let mut a = original.clone();
    Arc::make_mut(&mut a.source)[0].id = 0_f64.into();
    assert_ne!(original, &a);
    let mut a = original.clone();
    Arc::make_mut(&mut a.parts)[2].datum.value = 0.;
    assert_ne!(original, &a);
    let mut a = original.clone();
    Arc::make_mut(&mut a.separators)[0].part_id = Some(0_f64.into());
    assert_ne!(original, &a);
    let mut a = original.clone();
    a.position[0] = -0.;
    let mut b = a.clone();
    b.position[0] = 0.;
    assert_eq!(a, b, "computed geometry keeps old numeric equality");
}

#[test]
fn current_owned_labels_clips_and_coherent_public_edits_remain_layer_data() {
    let mut raw = source();
    let mut layout = funnel_layout(&config(), &raw, &Default::default()).unwrap();
    layout.scene.labels[0].text.clear();
    let mut extra = layout.scene.labels[0].clone();
    extra.text = "second current line".into();
    extra.y += 20.;
    let slot = layout.scene.labels.len();
    layout.scene.labels.push(extra);
    layout
        .scene
        .label_series
        .insert(slot, layout.parts[0].series.clone());
    let plot = layout.plot();
    layout.scene.label_clips.insert(
        slot,
        ClipPath {
            geometry: Geometry::Rect {
                x: plot.x,
                y: plot.y,
                width: plot.width,
                height: plot.height,
            },
            rule: ClipRule::NonZero,
        },
    );
    // No immutable source freeze is claimed: a complete, coherent raw-only
    // update that changes no geometry can be supplied with the public layout.
    raw[2].value = 0.;
    layout.parts[2].datum.value = 0.;
    let packet = funnel_widgets(&layout, &raw, 0, placement).unwrap();
    let data = packet.data.values().next().unwrap();
    assert!(data.labels.labels[0].text.is_empty());
    assert_eq!(data.labels.labels[slot].text, "second current line");
    assert_eq!(data.labels.clips[&slot], layout.scene.label_clips[&slot]);
    assert_eq!(data.labels.series[&slot], layout.parts[0].series);
    assert_eq!(data.source[2].value.to_bits(), 0);
    assert_eq!(funnel_widget_frame(&layout).unwrap().scene, layout.scene);
    let none = funnel_layout(
        &config(),
        &raw,
        &FunnelLayoutOptions {
            labels: None,
            before: None,
            after: None,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        funnel_widgets(&none, &raw, 0, placement)
            .unwrap()
            .data
            .values()
            .all(|d| d.labels.labels.is_empty())
    );
}

#[test]
fn empty_zero_width_skip_callback_failure_and_one_shared_budget() {
    let layout = funnel_layout(&config(), &[], &Default::default()).unwrap();
    let packet = funnel_widgets(&layout, &[], 0, |_| panic!("no role")).unwrap();
    assert!(packet.data.is_empty());
    assert_eq!(packet.layout.plot, Some(layout.plot()));
    assert_eq!(funnel_widget_frame(&layout).unwrap().scene, layout.scene);
    let raw = source();
    let layout = funnel_layout(
        &config(),
        &raw,
        &FunnelLayoutOptions {
            size: FunnelSize::OrdinalPixels(vec![0.]),
            ..Default::default()
        },
    )
    .unwrap();
    let p = funnel_widgets(&layout, &raw, 0, placement).unwrap();
    assert!(!p.data.values().any(|d| d.role == FunnelWidgetRole::Body));
    assert_eq!(
        funnel_widget_frame(&layout)
            .unwrap()
            .interaction
            .unwrap()
            .sites
            .len(),
        0
    );
    let mut calls = 0;
    let before = layout.clone();
    assert!(
        funnel_widgets(&layout, &raw, 0, |c| {
            calls += 1;
            if calls == 4 {
                Err(ChartError("late callback rejected".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 4);
    assert_eq!(layout, before);
    let zero = vec![FunnelDatum::new("a", -0.), FunnelDatum::new("b", 0.)];
    let equal = funnel_layout(&config(), &zero, &Default::default()).unwrap();
    assert_eq!(
        funnel_widgets(&equal, &zero, 0, placement)
            .unwrap()
            .data
            .values()
            .filter(|d| d.role == FunnelWidgetRole::Body)
            .count(),
        2,
        "an all-zero value domain has default half-width bodies, so its real zero-valued body roles remain eligible"
    );
    let raw: Vec<_> = (0..32)
        .map(|i| FunnelDatum::new(format!("stage{i}"), 1.))
        .collect();
    let layout = funnel_layout(&config(), &raw, &Default::default()).unwrap();
    let mut calls = 0;
    assert!(
        funnel_widgets(&layout, &raw, 0, |c| {
            calls += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    let mut calls = 0;
    let p = funnel_widgets(&layout, &raw, 0, |c| {
        calls += 1;
        if calls <= 128 { placement(c) } else { Ok(None) }
    })
    .unwrap();
    assert_eq!(p.data.len(), 128);
}

#[test]
fn actual_memory_retains_bit_only_data_count_focus_and_strict_frame_rejection() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use glory::render::{Event, EventKind, Props};
    use glory::{App, Widget};
    #[derive(Clone)]
    struct Source {
        frame: SceneInteraction,
        packet: PlotWidgetsComposition<FunnelWidgetData>,
    }
    let candidate = |positive: bool, revision| {
        let mut raw = source();
        let mut layout = funnel_layout(&config(), &raw, &Default::default()).unwrap();
        if positive {
            raw[2].value = 0.;
            layout.parts[2].datum.value = 0.;
        }
        let widgets = funnel_widgets(&layout, &raw, revision, |c| {
            if c.role == FunnelWidgetRole::Body && c.source_index == Some(0) {
                placement(c)
            } else {
                Ok(None)
            }
        })
        .unwrap();
        Source {
            frame: funnel_widget_frame(&layout).unwrap(),
            packet: PlotWidgetsComposition {
                replace_ink: widgets.data.keys().cloned().collect(),
                widgets,
            },
        }
    };
    let a = candidate(false, 0);
    let b = candidate(true, 0);
    assert_eq!(a.frame, b.frame);
    assert_eq!(
        render::to_svg(&a.frame.scene).unwrap(),
        render::to_svg(&b.frame.scene).unwrap()
    );
    assert_ne!(a.packet, b.packet);
    let app = App::memory();
    let source = app.scope().cage(a);
    let frame = app.scope().bond(move || source.with(|s| s.frame.clone()));
    let data = app.scope().bond(move || source.with(|s| s.packet.clone()));
    let rejected = Rc::new(RefCell::new(vec![]));
    let errors = rejected.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .on_reject(move |e| errors.borrow_mut().push(e.0))
            .plot_widgets_composed(data, |c| {
                if c.revision == 2 {
                    return Err(ChartError("candidate factory rejected".into()));
                }
                let frame = c.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0usize);
                    let focus = scope.cage(false);
                    glory::ui::button()
                        .class("funnel-owned-role")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{} count={} focus={}",
                                    f.data.source[2].value.to_bits(),
                                    count.get(),
                                    focus.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                        .on(EventKind::FocusChanged, move |e| {
                            if let Event::FocusChanged(v) = e {
                                focus.revise(|f| *f = v);
                            }
                        })
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    let find = || {
        let tree = app.tree();
        let mut q = tree.roots().to_vec();
        let mut found = vec![];
        while let Some(id) = q.pop() {
            let n = tree.node(id).unwrap();
            if n.descriptor
                .view()
                .classes
                .iter()
                .any(|c| c == "funnel-owned-role")
            {
                found.push(id);
            }
            q.extend(n.children.iter().copied());
        }
        assert_eq!(found.len(), 1);
        found[0]
    };
    let label = |id| {
        let tree = app.tree();
        let Props::Button(p) = &tree.node(id).unwrap().descriptor.view().props else {
            panic!("actual button")
        };
        p.label.clone()
    };
    let id = find();
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert_eq!(label(id), "9223372036854775808 count=1 focus=true");
    source.revise(|s| *s = b);
    app.flush().unwrap();
    assert_eq!(find(), id);
    assert_eq!(label(id), "0 count=1 focus=true");
    let accepted_nodes = || {
        let tree = app.tree();
        let mut q = tree.roots().to_vec();
        let mut rows = vec![];
        while let Some(id) = q.pop() {
            let n = tree.node(id).unwrap();
            rows.push((
                id,
                n.parent,
                n.children.clone(),
                format!("{:?}", n.descriptor),
            ));
            q.extend(n.children.iter().copied());
        }
        rows
    };
    let tree = accepted_nodes();
    source.revise(|s| *s = candidate(false, 2));
    app.flush().unwrap();
    assert_eq!(
        accepted_nodes(),
        tree,
        "accepted live nodes/descriptors/parents/child order are exact; disposed candidate allocation statistics are not accepted state"
    );
    assert_eq!(label(id), "0 count=1 focus=true");
    assert!(
        rejected
            .borrow()
            .last()
            .unwrap()
            .contains("candidate factory rejected")
    );
    source.revise(|s| *s = candidate(true, 1));
    app.flush().unwrap();
    assert_ne!(find(), id);
    assert_eq!(label(find()), "0 count=0 focus=false");
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}
