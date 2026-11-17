use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 220.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn layout(
    value: f64,
    hidden: bool,
    mode: WaffleMode,
    direction: WaffleFillDirection,
) -> WaffleLayout {
    waffle_layout(
        &config(),
        &[
            RadialDatum {
                id: "one".into(),
                label: "Original one".into(),
                value,
            },
            RadialDatum::new("two", 25.5),
        ],
        &WaffleLayoutOptions {
            rows: 2,
            columns: 3,
            total: Some(100.),
            mode,
            direction,
            padding: 6.,
            value_format: ValueFormat::Fixed(1),
            hidden: if hidden {
                BTreeSet::from(["one".into()])
            } else {
                BTreeSet::new()
            },
            ..Default::default()
        },
    )
    .unwrap()
}
fn measured(context: &WaffleCellWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    let b = context.cell.bounds;
    Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
        x: b.x,
        y: b.y,
        width: b.width,
        height: b.height,
    })))
}
fn frame(
    value: f64,
    revision: u64,
    hidden: bool,
) -> (
    SceneInteraction,
    PlotWidgetsComposition<WaffleCellWidgetData>,
) {
    let mut layout = layout(
        value,
        hidden,
        WaffleMode::Cells,
        WaffleFillDirection::Bottom,
    );
    layout.interaction.picking_profile = PickingProfile::PaintedGeometry;
    kartis::surface::SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        kartis::surface::SurfaceOptions {
            plot: Some(ClipRect::plot(&config())),
            ..Default::default()
        },
        Some(layout.interaction.clone()),
    )
    .unwrap();
    let widgets = waffle_cell_widgets(&layout, revision, measured).unwrap();
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    (
        layout.into_scene_interaction(),
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    )
}

#[test]
fn actual_cells_empty_cells_and_areas_preserve_owned_numeric_source() {
    for direction in [
        WaffleFillDirection::Top,
        WaffleFillDirection::Right,
        WaffleFillDirection::Bottom,
        WaffleFillDirection::Left,
    ] {
        for mode in [WaffleMode::Cells, WaffleMode::Areas] {
            for hidden in [false, true] {
                let layout = layout(17.25, hidden, mode, direction);
                let before = layout.clone();
                let mut calls = vec![];
                let packet = waffle_cell_widgets(&layout, 7, |context| {
                    assert!(std::ptr::eq(context.layout, &layout));
                    assert!(std::ptr::eq(
                        context.cell,
                        &layout.cells[context.cell.index]
                    ));
                    assert_eq!(
                        context.cell.mark_id.as_deref(),
                        Some(context.mark.id.as_str())
                    );
                    assert_eq!(context.group.map(|g| g.source_index), context.cell.owner);
                    calls.push(context.cell.index);
                    measured(context)
                })
                .unwrap();
                assert_eq!(layout, before);
                assert_eq!(
                    calls,
                    layout
                        .cells
                        .iter()
                        .filter(|c| c.mark_id.is_some())
                        .map(|c| c.index)
                        .collect::<Vec<_>>()
                );
                assert_eq!(packet.layout.plot, Some(ClipRect::plot(&config())));
                assert_eq!(packet.layout.slots.len(), calls.len());
                assert!(packet.layout.slots.iter().all(|s| s.revision == 7));
                for data in packet.data.values() {
                    assert_eq!(data.cell, layout.cells[data.cell.index]);
                    assert_eq!(data.total, 100.);
                    assert_eq!(data.unit, 100. / 6.);
                    assert_eq!(data.grid, layout.grid);
                    assert_eq!(data.cell_size, layout.cell_size);
                    if let Some(group) = &data.group {
                        assert_eq!(group.as_ref(), &layout.groups[group.source_index]);
                        if group.source_index == 0 {
                            assert_eq!(group.source.value, 17.25);
                            assert_eq!(group.source.label, "Original one");
                            assert_ne!(group.formatted_value, "17.25");
                        }
                    } else {
                        assert!(data.cell.owner.is_none());
                    }
                    if mode == WaffleMode::Areas {
                        assert!(
                            data.group.is_none(),
                            "occupied union has no fabricated cell Widget"
                        );
                    }
                }
            }
        }
    }
    let layout = layout(40., false, WaffleMode::Cells, WaffleFillDirection::Bottom);
    let packet = waffle_cell_widgets(&layout, 0, measured).unwrap();
    let same_group: Vec<_> = packet
        .data
        .values()
        .filter_map(|d| d.group.as_ref())
        .filter(|g| g.source_index == 0)
        .collect();
    assert!(same_group.len() > 1);
    assert!(
        Arc::ptr_eq(same_group[0], same_group[1]),
        "large group snapshots are shared, not copied per cell"
    );
}

#[test]
fn empty_zero_opacity_and_checked_custom_ink_still_receive_callbacks() {
    let options = WaffleLayoutOptions {
        rows: 2,
        columns: 3,
        total: Some(100.),
        empty_opacity: 0.,
        padding: 6.,
        direction: WaffleFillDirection::Bottom,
        ..Default::default()
    };
    let custom = waffle_layout_with_cell_factory(
        &config(),
        &[RadialDatum::new("one", 20.)],
        &options,
        &mut |c| {
            Ok(c.inherit(Geometry::Circle {
                x: c.position[0],
                y: c.position[1],
                radius: 12.,
            }))
        },
    )
    .unwrap();
    let mut empty = 0;
    let packet = waffle_cell_widgets(&custom, 0, |c| {
        assert!(matches!(
            c.mark.geometry,
            Geometry::Circle { radius: 12., .. }
        ));
        if c.group.is_none() {
            assert_eq!(c.mark.opacity, 0.);
            empty += 1;
        }
        measured(c)
    })
    .unwrap();
    assert_eq!(packet.data.len(), 6);
    assert_eq!(empty, 5);
}

#[test]
fn all_public_metadata_is_checked_before_any_callback() {
    let source = layout(17.25, false, WaffleMode::Cells, WaffleFillDirection::Bottom);
    type Corruption = Box<dyn Fn(&mut WaffleLayout)>;
    let changes: Vec<Corruption> = vec![
        Box::new(|l| l.total = f64::NAN),
        Box::new(|l| l.unit = f64::NAN),
        Box::new(|l| l.unit += 1.),
        Box::new(|l| l.cell_size += 1.),
        Box::new(|l| l.grid.x += 1.),
        Box::new(|l| {
            l.cells.pop();
        }),
        Box::new(|l| l.cells[5].bounds.x = f64::NAN),
        Box::new(|l| l.cells[5].bounds.x += 1.),
        Box::new(|l| l.cells[5].index = 0),
        Box::new(|l| l.cells[5].row = 0),
        Box::new(|l| l.cells[5].owner = Some(100)),
        Box::new(|l| l.cells[5].mark_id = None),
        Box::new(|l| l.cells[5].mark_id = Some("waffle-cell:0:0".into())),
        Box::new(|l| l.groups[0].source.value = f64::NAN),
        Box::new(|l| l.groups[0].source.value += 5.),
        Box::new(|l| l.groups[0].source.id = "forged".into()),
        Box::new(|l| l.groups[0].source.label = "forged".into()),
        Box::new(|l| l.groups[0].source_index = 100),
        Box::new(|l| l.groups[0].hidden = true),
        Box::new(|l| l.groups[0].requested = 0..5),
        Box::new(|l| l.groups[0].occupied = 0..5),
        Box::new(|l| l.groups[0].polygons[0][0][0] = f64::NAN),
        Box::new(|l| l.groups[0].formatted_value = "forged".into()),
        Box::new(|l| l.groups[0].area_mark_id = Some("waffle-area:one".into())),
        Box::new(|l| l.scene.marks[0].value += 1.),
        Box::new(|l| l.scene.marks[0].series = "forged".into()),
        Box::new(|l| l.scene.marks[5].label = "fabricated empty data".into()),
        Box::new(|l| {
            l.scene.marks.push(Mark::new(
                "waffle-cell:orphan",
                Geometry::Rect {
                    x: 20.,
                    y: 20.,
                    width: 5.,
                    height: 5.,
                },
                Color(0, 0, 0, 255),
            ))
        }),
        Box::new(|l| {
            l.scene.marks.push(Mark::new(
                "waffle-area:orphan",
                Geometry::Rect {
                    x: 20.,
                    y: 20.,
                    width: 5.,
                    height: 5.,
                },
                Color(0, 0, 0, 255),
            ))
        }),
    ];
    for (index, change) in changes.into_iter().enumerate() {
        let mut candidate = source.clone();
        change(&mut candidate);
        let mut calls = 0;
        let result = waffle_cell_widgets(&candidate, 0, |_| {
            calls += 1;
            Ok(None)
        });
        assert!(result.is_err(), "corruption {index} admitted");
        assert_eq!(calls, 0, "corruption {index} called user code");
    }
    let mut area = layout(17.25, false, WaffleMode::Areas, WaffleFillDirection::Bottom);
    let area_mark = area.groups[0].area_mark_id.as_ref().unwrap().clone();
    area.scene
        .marks
        .iter_mut()
        .find(|m| m.id == area_mark)
        .unwrap()
        .geometry = Geometry::Rect {
        x: 20.,
        y: 20.,
        width: 4.,
        height: 4.,
    };
    assert!(waffle_cell_widgets(&area, 0, |_| panic!("invalid area called user code")).is_err());
}

#[test]
fn borrowed_failure_and_selected_slot_budget_are_atomic() {
    let layout = layout(17.25, false, WaffleMode::Cells, WaffleFillDirection::Bottom);
    let before = layout.clone();
    let mut calls = 0;
    let result = waffle_cell_widgets(&layout, 0, |c| {
        calls += 1;
        if calls == 4 {
            Err(ChartError("placement refused".into()))
        } else {
            measured(c)
        }
    });
    assert!(result.is_err());
    assert_eq!(calls, 4);
    assert_eq!(layout, before);
    let large = waffle_layout(
        &ChartConfig::default(),
        &[],
        &WaffleLayoutOptions {
            rows: 13,
            columns: 10,
            ..Default::default()
        },
    )
    .unwrap();
    let mut called = 0;
    assert!(
        waffle_cell_widgets(&large, 0, |c| {
            called += 1;
            measured(c)
        })
        .unwrap_err()
        .0
        .contains("budget")
    );
    assert_eq!(called, 129);
    let selected = waffle_cell_widgets(&large, 0, |c| {
        if c.cell.index < 128 {
            measured(c)
        } else {
            Ok(None)
        }
    })
    .unwrap();
    assert_eq!(selected.data.len(), 128);
    assert!(
        waffle_cell_widgets(&large, 0, |_| Ok(None))
            .unwrap()
            .data
            .is_empty()
    );
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|v| v == class) {
            out.push(id);
        }
        queue.extend(n.children.iter().copied());
    }
    out
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("button expected")
    };
    p.label.clone()
}
fn live_tree(app: &App) -> String {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        out.push(format!("{id:?}:{n:?}"));
        queue.extend(n.children.iter().copied());
    }
    format!("{:?}:{out:?}", tree.roots())
}
fn child(
    ctx: PlotWidgetCompositionContext<WaffleCellWidgetData>,
    clean: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    if ctx.revision == 1 {
        return Err(ChartError("Waffle cell factory refused".into()));
    }
    let frame = ctx.frame;
    let revision = ctx.revision;
    Ok(glory::ui::widget(move |scope| {
        let clean = clean.clone();
        scope.on_cleanup(move || clean.set(clean.get() + 1))?;
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(frame.with(|f| format!("cell-{}-{}", f.data.cell.row, f.data.cell.column)))
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{:?}|value={:?}|count={}|focus={}",
                        f.data.group.as_ref().map(|g| g.source.id.as_str()),
                        f.data.group.as_ref().map(|g| g.source.value),
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
        if revision == 2 {
            return Err(glory::render::Error::InvalidProps("Waffle child build refused").into());
        }
        Ok(())
    }))
}

#[test]
fn real_memory_children_keep_cell_identity_and_raw_data_then_reject_whole_candidates() {
    let app = App::memory();
    let source = app.scope().cage(frame(17.25, 0, false));
    let scene = app.scope().bond(move || source.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || source.with(|s| s.1.clone()));
    let clean = Rc::new(Cell::new(0));
    let cleaned = clean.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let seen = errors.clone();
    app.mount(
        chart_layout(scene)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_reject(move |e| seen.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |ctx| child(ctx, cleaned.clone())),
    )
    .unwrap_or_else(|error| panic!("{error:?}; rejected: {:?}", errors.borrow()));
    let id = nodes(&app, "cell-0-0")[0];
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert!(label(&app, id).contains("Some(17.25)|count=1|focus=true"));
    source.revise(|s| *s = frame(33.5, 0, false));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "cell-0-0"), [id]);
    assert!(label(&app, id).contains("Some(33.5)|count=1|focus=true"));
    source.revise(|s| *s = frame(33.5, 0, true));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "cell-0-0"), [id]);
    assert!(label(&app, id).contains("Some(\"two\")|value=Some(25.5)|count=1"));
    for revision in [1, 2] {
        let before = live_tree(&app);
        let live = app.stats().nodes;
        source.revise(|s| *s = frame(40., revision, false));
        app.flush().unwrap();
        assert_eq!(live_tree(&app), before);
        assert_eq!(app.stats().nodes, live);
        assert_eq!(nodes(&app, "cell-0-0"), [id]);
        assert!(label(&app, id).contains("Some(25.5)|count=1"));
    }
    assert_eq!(errors.borrow().len(), 2);
    source.revise(|s| *s = frame(40., 3, false));
    app.flush().unwrap();
    let replacement = nodes(&app, "cell-0-0")[0];
    assert_ne!(replacement, id);
    assert!(label(&app, replacement).contains("Some(40.0)|count=0"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(
        clean.get(),
        13,
        "six accepted old, one failed build, six final children cleaned once"
    );
}

#[cfg(feature = "raster")]
fn png_pixel(source: &str, x: usize, y: usize) -> [u8; 4] {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(source.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    rgba[4 * (y * info.width as usize + x)..][..4]
        .try_into()
        .unwrap()
}

#[cfg(feature = "raster")]
#[test]
fn raster_publisher_rejection_keeps_owned_cell_data_and_real_png() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let source = app.scope().cage(frame(17.25, 0, false));
        // The supported CPU image API accepts Scene plus typed slot payloads;
        // finite typed pointer binding is exercised by the Memory test above.
        let scene = app.scope().bond(move || source.with(|s| s.0.scene.clone()));
        let packet = app.scope().bond(move || source.with(|s| s.1.clone()));
        let reject = Rc::new(Cell::new(false));
        let refused = reject.clone();
        let clean = Rc::new(Cell::new(0));
        let cleaned = clean.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let seen = errors.clone();
        app.mount(
            raster_chart(
                scene,
                FontRegistry::new(),
                RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
            )
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_reject(move |e| seen.borrow_mut().push(e.0))
            .raster_image_source(move |image| {
                if refused.get() {
                    Err(ChartError("Waffle PNG publication refused".into()))
                } else {
                    image.data_url()
                }
            })
            .plot_widgets_composed(packet, move |ctx| child(ctx, cleaned.clone())),
        )
        .unwrap();
        let image = |app: &App| {
            let tree = app.tree();
            let mut queue = tree.roots().to_vec();
            while let Some(id) = queue.pop() {
                let n = tree.node(id).unwrap();
                if let Props::Image(p) = n.descriptor.view().props {
                    return p.source.clone();
                }
                queue.extend(n.children.iter().copied());
            }
            panic!("raster image expected")
        };
        let initial = image(&app);
        assert_eq!(
            png_pixel(&initial, (70. * dpr) as usize, (65. * dpr) as usize),
            [255, 255, 255, 255],
            "real source cell ink is replaced in the CPU image"
        );
        let tree = live_tree(&app);
        let id = nodes(&app, "cell-0-0")[0];
        reject.set(true);
        source.revise(|s| *s = frame(40., 3, false));
        app.flush().unwrap();
        assert_eq!(image(&app), initial);
        assert_eq!(live_tree(&app), tree);
        assert!(label(&app, id).contains("Some(17.25)"));
        assert_eq!(errors.borrow().len(), 1);
        reject.set(false);
        source.revise(|s| *s = frame(33.5, 3, false));
        app.flush().unwrap();
        assert!(label(&app, nodes(&app, "cell-0-0")[0]).contains("Some(33.5)"));
        let retained = nodes(&app, "cell-0-0")[0];
        source.revise(|s| s.1.replace_ink.clear());
        app.flush().unwrap();
        let color = source.with(|s| s.0.scene.marks[0].fill.unwrap());
        assert_eq!(
            png_pixel(&image(&app), (70. * dpr) as usize, (65. * dpr) as usize),
            [color.0, color.1, color.2, color.3]
        );
        assert_eq!(nodes(&app, "cell-0-0"), [retained]);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
