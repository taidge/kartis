use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, NodeId, PathCommand as C, Props};
use glory::{App, Widget};
use kartis::*;

#[path = "../examples/shared/scatter_widgets.rs"]
mod shared;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn source() -> Vec<ScatterSeries> {
    let mut first = ScatterDatum::new("one", 1., 2.);
    first.size_value = Some(1.);
    first.label = Some("Original one".into());
    first.color = Some(Color(77, 88, 99, 255));
    let mut zero = ScatterDatum::new("zero", "1", 0.);
    zero.size_value = Some(0.);
    let missing = ScatterDatum::new("missing", "middle", 3.);
    let mut signed = ScatterDatum::new("signed", "tail", -2.);
    signed.size_value = Some(2.);
    let mut row = ScatterSeries::new("north", vec![first, zero, missing, signed]);
    row.label = "Original north".into();
    row.color = Some(Color(12, 34, 56, 255));
    vec![row]
}
fn options() -> ScatterLayoutOptions {
    ScatterLayoutOptions {
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        scales: ScatterScaleOptions {
            x: ScatterAxisScale::Point {
                options: Default::default(),
                domain: Some(vec![
                    1_f64.into(),
                    "1".into(),
                    "middle".into(),
                    "tail".into(),
                ]),
                window: None,
            },
            y: ScatterAxisScale::Continuous {
                options: ContinuousScaleOptions {
                    nice: ScaleNice::Off,
                    ..Default::default()
                },
                min: ScatterBound::Fixed(-5.),
                max: ScatterBound::Fixed(8.),
            },
        },
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Linear {
                    values: [0., 2.],
                    sizes: [0., 18.],
                    clamp: false,
                    missing: ScatterMissingSize::Skip,
                },
            },
            ..Default::default()
        },
        x_format: ValueFormat::Fixed(1),
        y_format: ValueFormat::Fixed(1),
        ..Default::default()
    }
}
fn placement(_: &ScatterNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-16., -11.],
        measured: TextSize {
            width: 32.,
            height: 22.,
        },
    }))
}

#[test]
fn actual_roles_borrow_complete_source_share_owned_series_and_leave_layout_unchanged() {
    let source = source();
    let layout = scatter_layout(&config(), &source, &options()).unwrap();
    let before = layout.scene.clone();
    let metadata = layout.metadata.clone();
    let sites = layout.sites.clone();
    let mut called = vec![];
    let packet = scatter_node_widgets(&layout, &source, 7, |c| {
        assert!(std::ptr::eq(c.layout, &layout));
        assert!(std::ptr::eq(c.series, &source[c.site.series_index]));
        assert!(std::ptr::eq(c.datum, &c.series.data[c.site.source_index]));
        assert!(std::ptr::eq(c.mark, &layout.scene.marks[c.site.mark_index]));
        called.push(c.site.source_index);
        placement(c)
    })
    .unwrap();
    assert_eq!(called, [0, 1, 3]);
    assert_eq!(packet.layout.plot, Some(layout.plot));
    assert_eq!(layout.scene, before);
    assert_eq!(layout.metadata, metadata);
    assert_eq!(layout.sites, sites);
    let owned = packet.data.values().collect::<Vec<_>>();
    assert_eq!(*owned[0].series, source[0]);
    assert!(
        owned
            .iter()
            .all(|data| Arc::ptr_eq(&owned[0].series, &data.series))
    );
    assert_eq!(owned[0].series.data[2].size_value, None);
    assert!(
        owned
            .iter()
            .any(|d| d.site.size_value == Some(0.) && d.site.node_size == 0.)
    );
    assert_eq!(owned[0].series.label, "Original north");
    assert_eq!(owned[0].series.data[0].color, Some(Color(77, 88, 99, 255)));
    assert_eq!(packet.layout.slots[0].revision, 7);
    assert!(
        owned
            .iter()
            .any(|d| matches!(d.site.raw_x, CategoryKey::Number(1.)))
    );
    assert!(owned.iter().any(|d| d.site.raw_x == CategoryKey::from("1")));
}

#[test]
fn all_shared_profiles_and_glyph_units_keep_actual_roles_including_zero_and_off_plot() {
    for (phase, count) in [
        (0, 7),
        (1, 8),
        (2, 3),
        (3, 7),
        (4, 7),
        (5, 7),
        (6, 6),
        (7, 4),
    ] {
        for width in [320., 420.] {
            let shared = shared::source(phase, width, 0).unwrap();
            assert_eq!(shared.composition.widgets.data.len(), count);
            assert_eq!(shared.composition.widgets.layout.plot, shared.frame.plot);
        }
    }
    for shape in [
        ScatterNodeShape::Circle {
            diameter: ScatterSize::Fixed(0.),
        },
        ScatterNodeShape::FilledGlyph {
            shape: FilledGlyph::Triangle,
            area: ScatterSize::Fixed(40.),
            rotation: 0.3,
        },
        ScatterNodeShape::StrokedGlyph {
            shape: StrokedGlyph::Plus,
            size: ScatterSize::Fixed(40.),
            rotation: 0.3,
        },
    ] {
        let mut o = options();
        o.node.shape = shape.clone();
        o.node.opacity = 0.;
        if matches!(shape, ScatterNodeShape::StrokedGlyph { .. }) {
            o.node.stroke = Some(glory::render::GraphicStroke {
                color: Color(9, 10, 11, 255),
                width: 2.,
                cap: glory::render::StrokeCap::Butt,
            });
        }
        let source = source();
        let layout = scatter_layout(&config(), &source, &o).unwrap();
        let packet = scatter_node_widgets(&layout, &source, 0, placement).unwrap();
        assert_eq!(packet.data.len(), 4);
        assert!(layout.scene.marks.iter().all(|m| m.opacity == 0.));
    }
    let data = [ScatterSeries::new(
        "outside",
        vec![ScatterDatum::new("far", 100., 2.)],
    )];
    let mut o = ScatterLayoutOptions {
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        ..Default::default()
    };
    o.scales.x = ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(1.),
    };
    let layout = scatter_layout(&config(), &data, &o).unwrap();
    assert!(layout.sites[0].position[0] > layout.plot.x + layout.plot.width);
    assert_eq!(
        scatter_node_widgets(&layout, &data, 0, placement)
            .unwrap()
            .data
            .len(),
        1
    );
}

#[test]
fn complete_late_source_site_mark_and_scale_conflicts_run_zero_callbacks() {
    type Mutate = Box<dyn FnOnce(&mut ScatterLayout, &mut Vec<ScatterSeries>)>;
    let changes: Vec<Mutate> = vec![
        Box::new(|_, s| s[0].data[3].x = 99_f64.into()),
        Box::new(|_, s| s[0].data[3].label = Some("changed".into())),
        Box::new(|_, s| s[0].data[3].size_value = Some(4.)),
        Box::new(|_, s| s[0].label = "changed".into()),
        Box::new(|_, s| s.push(s[0].clone())),
        Box::new(|l, _| {
            l.metadata.pop();
        }),
        Box::new(|l, _| l.metadata[3].global_index = 2),
        Box::new(|l, _| l.metadata[3].series_index = 1),
        Box::new(|l, _| l.metadata[3].source_index = 1),
        Box::new(|l, _| l.metadata[3].hidden = true),
        Box::new(|l, _| l.metadata[3].size_value = None),
        Box::new(|l, _| l.metadata[3].position = Some([f64::NAN, 1.])),
        Box::new(|l, _| l.metadata[3].node_size = Some(99.)),
        Box::new(|l, _| l.metadata[2].node_size = Some(0.)),
        Box::new(|l, _| l.sites[2].mark_index = 0),
        Box::new(|l, _| l.sites[2].series_label = "fake".into()),
        Box::new(|l, _| l.sites[2].position[0] += 1.),
        Box::new(|l, _| l.sites[2].bounds.width += 1.),
        Box::new(|l, _| l.sites[2].node_size = 100.),
        Box::new(|l, _| l.sites[2].label = "fake".into()),
        Box::new(|l, _| {
            l.sites.pop();
        }),
        Box::new(|l, _| l.scene.marks.last_mut().unwrap().series = "fake".into()),
        Box::new(|l, _| l.scene.marks.last_mut().unwrap().value = 99.),
        Box::new(|l, _| l.scene.marks.last_mut().unwrap().label = "fake".into()),
        Box::new(|l, _| {
            l.scene.marks.last_mut().unwrap().clip = Some(ClipRect {
                x: 1.,
                y: 1.,
                width: 1.,
                height: 1.,
            })
        }),
        Box::new(|l, _| {
            let mut m = l.scene.marks.last().unwrap().clone();
            m.id = "scatter-node:orphan".into();
            l.scene.marks.push(m);
        }),
        Box::new(|l, _| {
            let mut o = options();
            if let ScatterAxisScale::Continuous { options, .. } = &mut o.scales.y {
                options.clamp = true;
            }
            let source = source();
            l.y_scale = scatter_layout(&config(), &source, &o).unwrap().y_scale;
        }),
    ];
    for change in changes {
        let mut source = source();
        let mut layout = scatter_layout(&config(), &source, &options()).unwrap();
        change(&mut layout, &mut source);
        let calls = Cell::new(0);
        assert!(
            scatter_node_widgets(&layout, &source, 0, |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    let source = source();
    let mut layout = scatter_layout(&config(), &source, &options()).unwrap();
    layout.sites[0].color = Color(1, 2, 3, 255);
    let actual = layout.scene.marks[layout.sites[0].mark_index].fill;
    let packet = scatter_node_widgets(&layout, &source, 0, placement).unwrap();
    assert!(
        packet
            .data
            .values()
            .any(|d| d.site.color == Color(1, 2, 3, 255))
    );
    assert_eq!(
        layout.scene.marks[layout.sites[0].mark_index].fill, actual,
        "current metadata color is not fabricated current paint"
    );
}

#[test]
fn borrowed_failure_skip_and_selected_budget_never_return_a_partial_packet() {
    let source = source();
    let layout = scatter_layout(&config(), &source, &options()).unwrap();
    let before = layout.scene.clone();
    let mut calls = 0;
    assert!(
        scatter_node_widgets(&layout, &source, 0, |c| {
            calls += 1;
            if calls == 3 {
                Err(ChartError("placement refused".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 3);
    assert_eq!(layout.scene, before);
    let source = [ScatterSeries::new(
        "large",
        (0..130)
            .map(|i| ScatterDatum::new(i.to_string(), i as f64, 1.))
            .collect(),
    )];
    let layout = scatter_layout(&config(), &source, &ScatterLayoutOptions::default()).unwrap();
    let mut calls = 0;
    assert!(
        scatter_node_widgets(&layout, &source, 0, |c| {
            calls += 1;
            placement(c)
        })
        .unwrap_err()
        .0
        .contains("budget")
    );
    assert_eq!(calls, 129);
    assert_eq!(
        scatter_node_widgets(&layout, &source, 0, |c| if c.site.source_index < 128 {
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
        scatter_node_widgets(&layout, &source, 0, |_| Ok(None))
            .unwrap()
            .data
            .is_empty()
    );
}

#[test]
fn original_negative_zero_is_preserved_and_partial_sign_rewrites_are_rejected() {
    let mut source = vec![ScatterSeries::new(
        "signed-zero",
        vec![ScatterDatum::new("zero", -0., -0.)],
    )];
    let axis = ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        min: ScatterBound::Fixed(-1.),
        max: ScatterBound::Fixed(1.),
    };
    let options = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis.clone(),
            y: axis,
        },
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        ..Default::default()
    };
    let layout = scatter_layout(&config(), &source, &options).unwrap();
    let packet = scatter_node_widgets(&layout, &source, 0, placement).unwrap();
    let data = packet.data.values().next().unwrap();
    for value in [
        &data.site.raw_x,
        &data.site.raw_y,
        &data.series.data[0].x,
        &data.series.data[0].y,
    ] {
        let CategoryKey::Number(value) = value else {
            panic!("number expected")
        };
        assert_eq!(value.to_bits(), (-0_f64).to_bits());
    }
    for field in 0..4 {
        let mut candidate = layout.clone();
        match field {
            0 => candidate.metadata[0].raw_x = 0_f64.into(),
            1 => candidate.metadata[0].raw_y = 0_f64.into(),
            2 => candidate.sites[0].raw_x = 0_f64.into(),
            _ => candidate.sites[0].raw_y = 0_f64.into(),
        }
        let calls = Cell::new(0);
        assert!(
            scatter_node_widgets(&candidate, &source, 0, |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    source[0].data[0].x = 0_f64.into();
    source[0].data[0].y = 0_f64.into();
    let mut coherent = layout;
    coherent.metadata[0].raw_x = 0_f64.into();
    coherent.metadata[0].raw_y = 0_f64.into();
    coherent.sites[0].raw_x = 0_f64.into();
    coherent.sites[0].raw_y = 0_f64.into();
    assert_eq!(
        coherent.scene.marks[coherent.sites[0].mark_index]
            .value
            .to_bits(),
        (-0_f64).to_bits(),
        "the legacy paint scalar is independent of exact typed raw source"
    );
    assert!(scatter_node_widgets(&coherent, &source, 0, placement).is_ok());
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
    context: PlotWidgetCompositionContext<ScatterNodeWidgetData>,
    cleanup: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    if context.revision == 90 {
        return Err(ChartError("node factory refused".into()));
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
                    "node-{}-{}",
                    f.data.site.series_id, f.data.site.source_index
                )
            }))
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "whole-c={:?}|size={:?}|rawY={:?}|count={}|focus={}",
                        f.data.series.data[2].size_value,
                        f.data.site.size_value,
                        f.data.site.raw_y,
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
            return Err(glory::render::Error::InvalidProps("node build refused").into());
        }
        Ok(())
    }))
}

#[test]
fn real_memory_retains_state_accepts_new_raw_data_and_rejects_failed_children_atomically() {
    let app = App::memory();
    let source = app.scope().cage(shared::source(0, 320., 0).unwrap());
    let frame = app.scope().bond(move || source.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || source.with(|s| s.composition.clone()));
    let hidden = app.scope().cage(BTreeSet::new());
    let cleanup = Rc::new(Cell::new(0));
    let clean = cleanup.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .hidden(hidden)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |context| child(context, clean.clone())),
    )
    .unwrap();
    assert!(nodes(&app, "node-north-2").is_empty());
    let id = nodes(&app, "node-north-1")[0];
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert!(label(&app, id).contains("whole-c=None"));
    source.revise(|s| *s = shared::source(1, 420., 0).unwrap());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "node-north-1"), [id]);
    assert!(label(&app, id).contains("whole-c=Some(0.0)"));
    assert!(label(&app, id).contains("count=1|focus=true"));
    assert_eq!(nodes(&app, "node-north-2").len(), 1);
    hidden.revise(|s| {
        s.insert("north".into());
    });
    app.flush().unwrap();
    assert!(app.tree().inert(id));
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert!(!app.tree().inert(id));
    for revision in [90, 91] {
        let before = tree(&app);
        source.revise(|s| *s = shared::source(0, 320., revision).unwrap());
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(nodes(&app, "node-north-1"), [id]);
    }
    assert_eq!(errors.borrow().len(), 2);
    source.revise(|s| *s = shared::source(1, 320., 1).unwrap());
    app.flush().unwrap();
    let next = nodes(&app, "node-north-1")[0];
    assert_ne!(next, id);
    assert!(label(&app, next).contains("count=0|focus=false"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(cleanup.get(), 17);
}

#[test]
fn offset_path_live_widget_uses_typed_site_anchor_and_point_y_stays_raw() {
    let source = shared::source(3, 320., 0).unwrap();
    let app = App::memory();
    let captured = Rc::new(RefCell::new(vec![]));
    let frames = captured.clone();
    app.mount(
        chart_layout(source.frame)
            .legend(false)
            .plot_widgets_composed(source.composition, move |context| {
                frames.borrow_mut().push(context.frame);
                Ok(glory::ui::button().label("Path node"))
            }),
    )
    .unwrap();
    for frame in captured.borrow().iter() {
        frame.with(|f| {
            assert_eq!(f.frame.rect.x, f.data.site.position[0] - 16.);
            assert_eq!(f.frame.rect.y, f.data.site.position[1] - 11.);
            let Geometry::Path(commands) = &f.frame.mark.geometry else {
                panic!("Path expected")
            };
            let C::Move(first) = commands[0] else {
                panic!("Move expected")
            };
            assert_eq!(first.x, f.data.site.position[0] + 10.);
        });
    }
    assert_eq!(captured.borrow().len(), 7);
    app.clear().unwrap();
    let source = [ScatterSeries::new(
        "typed-y",
        vec![ScatterDatum::new("one", "raw-x", "raw-y")],
    )];
    let mut o = ScatterLayoutOptions::default();
    o.scales.x = ScatterAxisScale::Point {
        options: Default::default(),
        domain: None,
        window: None,
    };
    o.scales.y = o.scales.x.clone();
    let layout = scatter_layout(&config(), &source, &o).unwrap();
    let packet = scatter_node_widgets(&layout, &source, 0, placement).unwrap();
    assert_eq!(
        packet.data.values().next().unwrap().site.raw_y,
        CategoryKey::from("raw-y")
    );
    assert_eq!(layout.scene.marks[layout.sites[0].mark_index].value, 0.);
    assert_eq!(layout.interaction_binding().sites[0].value, None);
}

#[cfg(feature = "raster")]
#[test]
fn cpu_publisher_error_keeps_original_png_typed_source_options_and_child_focus() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let source = app.scope().cage((
            shared::source(0, 320., 0).unwrap(),
            RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        ));
        let frame = app.scope().bond(move || source.with(|s| s.0.frame.clone()));
        let packet = app
            .scope()
            .bond(move || source.with(|s| s.0.composition.clone()));
        let options = app.scope().bond(move || source.with(|s| s.1.clone()));
        let fail = Rc::new(Cell::new(false));
        let refused = fail.clone();
        let cleanup = Rc::new(Cell::new(0));
        let clean = cleanup.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        app.mount(
            chart_layout(frame)
                .raster(FontRegistry::new(), options)
                .legend(false)
                .highlight(false)
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .raster_image_source(move |image| {
                    if refused.get() {
                        Err(ChartError("node PNG refused".into()))
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
        let id = nodes(&app, "node-north-1")[0];
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
            assert_eq!(reader.info().width, (320. * dpr) as u32);
            assert_eq!(reader.info().height, (260. * dpr) as u32);
            let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut decoded).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            assert_eq!(&decoded[..4], &[255, 255, 255, 255]);
        }
        fail.set(true);
        source.revise(|s| {
            s.0 = shared::source(1, 420., 0).unwrap();
            s.1.view = ZoomTransform::new(1.25, -12., -8.).unwrap();
        });
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(image(&app), png);
        assert!(label(&app, id).contains("whole-c=None"));
        assert!(nodes(&app, "node-north-2").is_empty());
        assert_eq!(errors.borrow().len(), 1);
        fail.set(false);
        source.revise(|s| s.0 = shared::source(1, 420., 0).unwrap());
        app.flush().unwrap();
        assert_eq!(nodes(&app, "node-north-1"), [id]);
        assert!(label(&app, id).contains("whole-c=Some(0.0)"));
        assert!(label(&app, id).contains("count=1|focus=true"));
        assert_ne!(image(&app), png);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
