//! Independent raw-source bit changes through real retained Glory children.
//! Memory event dispatch is explicit; no browser/OS transport claim is made.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

type Frame<T> = (SceneInteraction, PlotWidgetsData<T>);
fn config() -> ChartConfig {
    ChartConfig {
        width: 280.,
        height: 180.,
        margin: Margin {
            left: 24.,
            right: 20.,
            top: 18.,
            bottom: 22.,
        },
        ..Default::default()
    }
}
fn raw(negative: bool) -> f64 {
    if negative { -0. } else { 0. }
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::Rect(LayoutRect {
        x: 45.,
        y: 40.,
        width: 36.,
        height: 20.,
    })
}
fn painted(mut frame: SceneInteraction) -> SceneInteraction {
    frame
        .interaction
        .get_or_insert_with(Default::default)
        .picking_profile = PickingProfile::PaintedGeometry;
    frame.validate().unwrap();
    frame
}
fn bar(negative: bool) -> Frame<BarWidgetData> {
    let mut options = BarOptions {
        value_domain: Some([-1., 4.]),
        ..Default::default()
    };
    options.value_axis.visible = false;
    options.value_axis.grid = false;
    options.category_axis.visible = false;
    options.category_axis.grid = false;
    let layout = bar_layout_with_factories(
        &config(),
        &["selected".into(), "raw sibling".into()],
        &[BarSeries::new(
            "series",
            vec![Some(3.), Some(raw(negative))],
        )],
        &options,
        &BarStyleOptions::default(),
        BarFactories::default(),
    )
    .unwrap();
    let packet = bar_widgets(&layout, 9, |c| {
        Ok((c.bar.source_category == 0).then(placement))
    })
    .unwrap();
    (bar_widget_frame(&layout).unwrap(), packet)
}
fn bar_bits(data: &BarWidgetData) -> u64 {
    data.series.values[1].unwrap().to_bits()
}
fn stream(negative: bool) -> Frame<StreamDotWidgetData> {
    let mut options = StreamLayoutOptions {
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        stack: kartis::stack::StackOptions {
            offset: kartis::stack::StackOffset::Zero,
            ..Default::default()
        },
        y_bounds: [Some(0.), Some(5.)],
        legend: false,
        ..Default::default()
    };
    options.axes.x.visible = false;
    options.axes.x.grid = false;
    options.axes.y.visible = false;
    options.axes.y.grid = false;
    let layout = stream_layout(
        &config(),
        &["selected".into(), "raw sibling".into()],
        &[BarSeries::new(
            "series",
            vec![Some(3.), Some(raw(negative))],
        )],
        &options,
    )
    .unwrap();
    let packet = stream_dot_widgets(&layout, 9, |c| {
        Ok((c.sample.source_index == 0).then(placement))
    })
    .unwrap();
    (painted(layout.into_scene_interaction().unwrap()), packet)
}
fn stream_bits(data: &StreamDotWidgetData) -> u64 {
    data.series.values[1].unwrap().to_bits()
}
fn stream_category(negative: bool) -> Frame<StreamDotWidgetData> {
    let mut options = StreamLayoutOptions {
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        legend: false,
        ..Default::default()
    };
    options.axes.x.visible = false;
    options.axes.x.grid = false;
    options.axes.y.visible = false;
    options.axes.y.grid = false;
    let layout = stream_layout(
        &config(),
        &[CategoryKey::Number(raw(negative)), CategoryKey::Number(1.)],
        &[BarSeries::new("series", vec![Some(3.), Some(2.)])],
        &options,
    )
    .unwrap();
    let packet = stream_dot_widgets(&layout, 9, |c| {
        Ok((c.sample.source_index == 0).then(placement))
    })
    .unwrap();
    (painted(layout.into_scene_interaction().unwrap()), packet)
}
fn stream_category_bits(data: &StreamDotWidgetData) -> u64 {
    let CategoryKey::Number(value) = data.sample.category else {
        panic!("original numeric category was not retained");
    };
    value.to_bits()
}
fn heatmap_layout_for(negative: bool) -> HeatmapLayout {
    heatmap_layout(
        &config(),
        &[
            HeatCell::new("row", "selected", raw(negative)),
            HeatCell::new("row", "positive", 3.),
        ],
        &HeatmapLayoutOptions {
            labels: false,
            axes: HeatmapAxes::none(),
            ..Default::default()
        },
    )
    .unwrap()
}
fn heatmap(negative: bool) -> Frame<HeatmapCellWidgetData> {
    let layout = heatmap_layout_for(negative);
    let packet = heatmap_cell_widgets(&layout, 9, |c| {
        Ok((c.cell.key.column == "selected").then(placement))
    })
    .unwrap();
    (layout.into_painted_scene_interaction().unwrap(), packet)
}
fn heatmap_bits(data: &HeatmapCellWidgetData) -> u64 {
    data.datum.value.unwrap().to_bits()
}
fn bullet(negative: bool) -> Frame<BulletPartWidgetData> {
    // A range ending at zero has real nonzero ink from domain minimum -1.
    // The payload contains this part, not the original full BulletDatum arrays.
    let mut options = BulletLayoutOptions {
        min: Some(-1.),
        max: Some(1.),
        legend: false,
        ..Default::default()
    };
    options.axis.visible = false;
    options.axis.grid = false;
    options.titles.insert("row".into(), String::new());
    let layout = bullet_layout(
        &config(),
        &[BulletDatum {
            id: "row".into(),
            ranges: vec![raw(negative)],
            measures: vec![],
            markers: vec![],
        }],
        &options,
    )
    .unwrap();
    let packet = bullet_part_widgets(&layout, 9, |c| {
        Ok(c.part.source_index.is_some().then(placement))
    })
    .unwrap();
    let frame = SceneInteraction {
        scene: layout.scene,
        interaction: Some(InteractionBinding {
            picking_profile: PickingProfile::PaintedGeometry,
            ..Default::default()
        }),
        plot: Some(ClipRect::plot(&config())),
    };
    frame.validate().unwrap();
    (frame, packet)
}
fn bullet_bits(data: &BulletPartWidgetData) -> u64 {
    data.part.raw_value.to_bits()
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = queue.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        queue.extend(node.children.iter().copied());
    }
    result
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(props) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("expected real Button");
    };
    props.label.clone()
}
fn expected(negative: bool) -> String {
    format!("bits={:016x};count=1;focus=true", raw(negative).to_bits())
}
fn retained<T: Clone + PartialEq + std::fmt::Debug + 'static>(
    name: &str,
    build: fn(bool) -> Frame<T>,
    bits: fn(&T) -> u64,
) {
    let before = build(true);
    let after = build(false);
    // Value equality must still admit fresh allocations of unchanged source.
    assert_eq!(before.1, build(true).1);
    assert_eq!(after.1, build(false).1);
    assert_eq!(before.0, after.0, "{name}: actual host frame changed");
    assert_eq!(before.1.layout, after.1.layout);
    assert_eq!(before.1.data.len(), 1);
    assert_eq!(
        bits(before.1.data.values().next().unwrap()),
        (-0_f64).to_bits()
    );
    assert_eq!(bits(after.1.data.values().next().unwrap()), 0_f64.to_bits());
    let app = App::memory();
    let source = app.scope().cage(before);
    let frame = app.scope().bond(move || source.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || source.with(|s| s.1.clone()));
    let errors = Rc::new(RefCell::new(vec![]));
    let error = errors.clone();
    let factories = Rc::new(Cell::new(0));
    let called = factories.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_reject(move |e| error.borrow_mut().push(e.0))
            .plot_widgets_with_data(packet, move |context| {
                called.set(called.get() + 1);
                let typed = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0);
                    let focus = scope.cage(false);
                    glory::ui::button()
                        .class("raw-bits-role")
                        .label(scope.bond(move || {
                            typed.with(|f| {
                                format!(
                                    "bits={:016x};count={};focus={}",
                                    bits(&f.data),
                                    count.get(),
                                    focus.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                        .on(EventKind::FocusChanged, move |e| {
                            if let Event::FocusChanged(v) = e {
                                focus.revise(|f| *f = v)
                            }
                        })
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    let child = nodes(&app, "raw-bits-role")[0];
    app.dispatch(child, Event::Activate).unwrap();
    app.dispatch(child, Event::FocusChanged(true)).unwrap();
    assert_eq!(label(&app, child), expected(true));
    source.revise(|s| *s = after);
    app.flush().unwrap();
    assert_eq!(nodes(&app, "raw-bits-role"), [child]);
    assert_eq!(factories.get(), 1);
    assert_eq!(
        label(&app, child),
        expected(false),
        "{name}: raw bits were lost"
    );
    let mut invalid = build(true);
    invalid.1.layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
        x: f64::NAN,
        y: 40.,
        width: 36.,
        height: 20.,
    });
    source.revise(|s| *s = invalid);
    app.flush().unwrap();
    assert_eq!(errors.borrow().len(), 1);
    assert_eq!(label(&app, child), expected(false));
    source.revise(|s| *s = build(true));
    app.flush().unwrap();
    assert_eq!(label(&app, child), expected(true));
    assert_eq!(nodes(&app, "raw-bits-role"), [child]);
    assert_eq!(factories.get(), 1);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[test]
fn bar_full_nullable_series_bit_only_change_updates_retained_child() {
    retained("Bar", bar, bar_bits);
}
#[test]
fn stream_full_nullable_series_bit_only_change_updates_retained_child() {
    retained("Stream", stream, stream_bits);
}
#[test]
fn stream_numeric_category_sign_is_typed_source_not_business_key_normalization() {
    retained(
        "Stream numeric category",
        stream_category,
        stream_category_bits,
    );
}
#[test]
fn heatmap_original_cell_bit_only_change_updates_retained_child() {
    retained("Heatmap", heatmap, heatmap_bits);
}
#[test]
fn bullet_actual_zero_endpoint_bit_only_change_updates_retained_child() {
    retained("Bullet", bullet, bullet_bits);
}
#[test]
fn heatmap_computed_value_cannot_change_source_sign_before_callbacks() {
    for custom in [false, true] {
        let mut layout = if custom {
            heatmap_layout_with_cell_factory(
                &config(),
                &[
                    HeatCell::new("row", "selected", -0.),
                    HeatCell::new("row", "positive", 3.),
                ],
                &HeatmapLayoutOptions {
                    labels: false,
                    axes: HeatmapAxes::none(),
                    ..Default::default()
                },
                &mut |c: &HeatmapCellContext<'_>| Ok(c.inherit(c.geometry.clone())),
            )
            .unwrap()
        } else {
            heatmap_layout_for(true)
        };
        assert!(layout.interaction_binding().is_ok());
        let cell = layout
            .cells
            .iter_mut()
            .find(|c| c.key.column == "selected")
            .unwrap();
        cell.value = Some(0.);
        let mut callbacks = 0;
        let result = heatmap_cell_widgets(&layout, 9, |_| {
            callbacks += 1;
            Ok(Some(placement()))
        });
        assert!(
            result.is_err(),
            "computed +0 was accepted for private source -0 (custom={custom})"
        );
        assert_eq!(callbacks, 0);
        assert!(
            layout.interaction_binding().is_err(),
            "original binding accepted forged sign (custom={custom})"
        );
    }
}
#[test]
fn bar_and_stream_private_source_sign_checks_precede_all_callbacks() {
    let mut options = BarOptions::default();
    options.value_axis.visible = false;
    options.category_axis.visible = false;
    let mut layout = bar_layout_with_factories(
        &config(),
        &["zero".into(), "positive".into()],
        &[BarSeries::new("series", vec![Some(-0.), Some(3.)])],
        &options,
        &BarStyleOptions::default(),
        BarFactories::default(),
    )
    .unwrap();
    let zero = layout
        .layout
        .bars
        .iter_mut()
        .find(|b| b.source_category == 0)
        .unwrap();
    assert_eq!(zero.raw_value.to_bits(), (-0_f64).to_bits());
    zero.raw_value = 0.;
    let mut bar_callbacks = 0;
    assert!(
        bar_widgets(&layout, 9, |_| {
            bar_callbacks += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(bar_callbacks, 0);
    let mut layout = stream_layout(
        &config(),
        &["zero".into(), "positive".into()],
        &[BarSeries::new("series", vec![Some(-0.), Some(3.)])],
        &StreamLayoutOptions {
            dots: Some(StreamDotOptions::default()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        layout.layers[0].samples[0].value.unwrap().to_bits(),
        (-0_f64).to_bits()
    );
    layout.layers[0].samples[0].value = Some(0.);
    let mut stream_callbacks = 0;
    assert!(
        stream_dot_widgets(&layout, 9, |_| {
            stream_callbacks += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(stream_callbacks, 0);
    let mut layout = stream_layout(
        &config(),
        &[CategoryKey::Number(-0.), CategoryKey::Number(1.)],
        &[BarSeries::new("series", vec![Some(3.), Some(2.)])],
        &StreamLayoutOptions {
            dots: Some(StreamDotOptions::default()),
            ..Default::default()
        },
    )
    .unwrap();
    layout.layers[0].samples[0].category = CategoryKey::Number(0.);
    let mut category_callbacks = 0;
    assert!(
        stream_dot_widgets(&layout, 9, |_| {
            category_callbacks += 1;
            Ok(None)
        })
        .is_err(),
        "private original numeric category -0 was silently normalized by metadata equality"
    );
    assert_eq!(category_callbacks, 0);
}
#[test]
fn waffle_zero_group_has_no_owned_cell_and_is_not_fabricated_into_live_payload() {
    let build = |negative| {
        waffle_layout(
            &config(),
            &[
                RadialDatum::new("positive", 2.),
                RadialDatum::new("zero", raw(negative)),
            ],
            &WaffleLayoutOptions {
                rows: 2,
                columns: 2,
                total: Some(4.),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let before = build(true);
    let after = build(false);
    assert_eq!(before.groups[1].source.value.to_bits(), (-0_f64).to_bits());
    assert_eq!(after.groups[1].source.value.to_bits(), 0_f64.to_bits());
    assert!(before.groups[1].occupied.is_empty());
    assert_eq!(before.scene, after.scene);
    let packet = |l: &WaffleLayout| waffle_cell_widgets(l, 9, |_| Ok(Some(placement()))).unwrap();
    let a = packet(&before);
    let b = packet(&after);
    assert_eq!(a, b);
    assert_eq!(a.data.len(), 4);
    assert_eq!(a.data.values().filter(|d| d.group.is_none()).count(), 2);
    for datum in a.data.values().filter_map(|d| d.group.as_ref()) {
        assert_eq!(datum.source.id, "positive");
        assert_eq!(datum.source.value.to_bits(), 2_f64.to_bits());
    }
}

#[cfg(feature = "raster")]
fn image_source(app: &App) -> String {
    let images = nodes(app, "kartis-raster-image");
    assert_eq!(images.len(), 1);
    let tree = app.tree();
    let Props::Image(props) = tree.node(images[0]).unwrap().descriptor.view().props else {
        panic!("expected actual raster Image");
    };
    props.source.clone()
}
#[cfg(feature = "raster")]
fn raster_retained<T: Clone + PartialEq + std::fmt::Debug + 'static>(
    name: &str,
    build: fn(bool) -> Frame<T>,
    bits: fn(&T) -> u64,
) {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let before = build(true);
    let after = build(false);
    let fonts = FontRegistry::new();
    for dpr in [1., 2.] {
        let options = RasterOptions {
            pixel_ratio: dpr,
            ..Default::default()
        };
        // Independent real render of the two authored frames proves equal ink.
        let old = render_raster(&before.0.scene, &fonts, &options).unwrap();
        let new = render_raster(&after.0.scene, &fonts, &options).unwrap();
        assert_eq!(
            old.png().unwrap(),
            new.png().unwrap(),
            "{name}: rendered ink differs"
        );
        let app = App::memory();
        let source = app.scope().cage(before.clone());
        let frame = app.scope().bond(move || source.with(|s| s.0.clone()));
        let packet = app.scope().bond(move || source.with(|s| s.1.clone()));
        let reject = Rc::new(Cell::new(false));
        let rejecting = reject.clone();
        let calls = Rc::new(Cell::new(0));
        let publications = calls.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let error = errors.clone();
        app.mount(
            chart_layout(frame)
                .legend(false)
                .data_controls(false)
                .tooltip(false)
                .raster(FontRegistry::new(), options)
                .raster_image_source(move |image| {
                    publications.set(publications.get() + 1);
                    if rejecting.get() {
                        Err(ChartError("raw-bit PNG candidate denied".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| error.borrow_mut().push(e.0))
                .plot_widgets_with_data(packet, move |context| {
                    let typed = context.frame;
                    Ok(glory::ui::widget(move |scope| {
                        glory::ui::button()
                            .class("raw-bits-role")
                            .label(
                                scope.bond(move || {
                                    typed.with(|f| format!("{:016x}", bits(&f.data)))
                                }),
                            )
                            .show_in(scope)
                    }))
                }),
        )
        .unwrap();
        let child = nodes(&app, "raw-bits-role")[0];
        let accepted_image = image_source(&app);
        assert_eq!(label(&app, child), "8000000000000000");
        reject.set(true);
        let mut denied = after.clone();
        // Force real image publication as well as a typed raw-only payload.
        denied.0.scene.width += 4.;
        let count = calls.get();
        source.revise(|s| *s = denied);
        app.flush().unwrap();
        assert_eq!(calls.get(), count + 1);
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(image_source(&app), accepted_image);
        assert_eq!(label(&app, child), "8000000000000000");
        assert_eq!(nodes(&app, "raw-bits-role"), [child]);
        reject.set(false);
        source.revise(|s| *s = after.clone());
        app.flush().unwrap();
        assert_eq!(label(&app, child), "0000000000000000", "{name} DPR{dpr}");
        assert_eq!(nodes(&app, "raw-bits-role"), [child]);
        assert_eq!(image_source(&app), accepted_image);
        app.clear().unwrap();
    }
}
#[cfg(feature = "raster")]
#[test]
fn actual_png_equal_ink_and_publisher_rejection_keep_raw_bits_atomic_all_four_roles() {
    raster_retained("Bar", bar, bar_bits);
    raster_retained("Stream", stream, stream_bits);
    raster_retained(
        "Stream numeric category",
        stream_category,
        stream_category_bits,
    );
    raster_retained("Heatmap", heatmap, heatmap_bits);
    raster_retained("Bullet", bullet, bullet_bits);
}
