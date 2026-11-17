use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[path = "../examples/shared/heatmap_widgets.rs"]
mod model;

fn placement(_ctx: &HeatmapCellWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-8., -6.],
        measured: TextSize {
            width: 16.,
            height: 12.,
        },
    }))
}

#[test]
fn nullable_sources_original_indices_and_sparse_pairs_are_not_inferred_from_mark_scalars() {
    let (a, first) = model::source(0, 600., 7).unwrap();
    let (b, next) = model::source(1, 600., 7).unwrap();
    assert_eq!(a.scene, b.scene);
    assert_eq!(first.widgets.layout, next.widgets.layout);
    assert_eq!(first.widgets.data.len(), 5);
    let null = HeatmapCellKey::new("north:温度", "null").mark_id();
    let zero = HeatmapCellKey::new("north:温度", "zero").mark_id();
    assert_eq!(first.widgets.data[&null].datum.value, None);
    assert_eq!(first.widgets.data[&zero].datum.value, Some(0.));
    assert_eq!(first.widgets.data[&null].source_index, 1);
    assert_eq!(next.widgets.data[&null].source_index, 3);
    assert_eq!(
        a.scene.marks.iter().find(|m| m.id == null).unwrap().value,
        0.
    );
    let binding = a.interaction.as_ref().unwrap();
    assert_eq!(
        binding
            .sites
            .iter()
            .find(|s| s.mark_id == null)
            .unwrap()
            .value,
        None
    );
    assert!(
        !first
            .widgets
            .data
            .contains_key(&HeatmapCellKey::new("south", "null").mark_id())
    );
    assert_eq!(first.widgets.layout.plot, a.plot);
}

#[test]
fn custom_geometry_current_hover_and_paint_decorations_keep_original_cell_source() {
    let cells = [
        HeatCell::new("row", "col", 2.),
        HeatCell::new("row", "other", 5.),
    ];
    let mut layout = heatmap_layout_with_cell_factory(
        &ChartConfig::default(),
        &cells,
        &HeatmapLayoutOptions::default(),
        &mut |c: &HeatmapCellContext<'_>| {
            Ok(c.inherit(Geometry::Circle {
                x: c.position[0],
                y: c.position[1],
                radius: 7.,
            }))
        },
    )
    .unwrap();
    layout
        .set_active(Some(HeatmapCellKey::new("row", "col")))
        .unwrap();
    let before = layout.scene.clone();
    let packet = heatmap_cell_widgets(&layout, 3, |ctx| {
        assert_eq!(ctx.cell.geometry.as_ref(), Some(&ctx.mark.geometry));
        assert_eq!(ctx.cell.opacity, ctx.mark.opacity);
        assert!(matches!(
            ctx.mark.geometry,
            Geometry::Circle { radius: 7., .. }
        ));
        placement(ctx)
    })
    .unwrap();
    assert_eq!(packet.data.len(), 2);
    assert_eq!(before, layout.scene);
    let mut ordinary =
        heatmap_layout(&ChartConfig::default(), &cells, &Default::default()).unwrap();
    let mark = ordinary
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == ordinary.cells[0].mark_id)
        .unwrap();
    mark.fill = Some(Color(1, 2, 3, 255));
    // Default layouts keep their public appearance-editing policy. Source
    // data/index stay authenticated, and geometry must agree with real paint.
    let center = ordinary.cells[0].center.unwrap();
    let geometry = Geometry::Circle {
        x: center[0],
        y: center[1],
        radius: 8.,
    };
    ordinary.cells[0].geometry = Some(geometry.clone());
    ordinary.cells[0].color = Color(240, 30, 180, 255);
    ordinary
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == ordinary.cells[0].mark_id)
        .unwrap()
        .geometry = geometry;
    ordinary.columns.reverse();
    heatmap_cell_widgets(&ordinary, 0, |ctx| {
        if ctx.cell.key.column == "col" {
            assert_eq!(ctx.mark.fill, Some(Color(1, 2, 3, 255)));
        }
        placement(ctx)
    })
    .unwrap();
}

#[test]
fn every_invalid_source_is_rejected_before_any_placement_callback() {
    let input = [
        HeatCell {
            row: "row".into(),
            column: "null".into(),
            value: None,
        },
        HeatCell::new("row", "zero", 0.),
    ];
    let base = heatmap_layout(&ChartConfig::default(), &input, &Default::default()).unwrap();
    for variant in 0..10 {
        let mut next = base.clone();
        match variant {
            0 => next.cells[0].value = Some(0.),
            1 => next.cells[0].formatted_value = Some("0".into()),
            2 => next.cells[1].mark_id = "forged".into(),
            3 => {
                next.cells[1].geometry = Some(Geometry::Circle {
                    x: 50.,
                    y: 50.,
                    radius: 2.,
                })
            }
            4 => next.cells[1].opacity = 0.5,
            5 => next.scene.marks.retain(|m| m.id != next.cells[1].mark_id),
            6 => next.rows.push("missing".into()),
            7 => next.rows.push(next.rows[0].clone()),
            8 => next.scene.marks.push(Mark::new(
                "heatmap-cell:orphan",
                Geometry::Circle {
                    x: 20.,
                    y: 20.,
                    radius: 3.,
                },
                Color(0, 0, 0, 255),
            )),
            _ => next.cells.reverse(),
        }
        let mut calls = 0;
        assert!(
            heatmap_cell_widgets(&next, 0, |c| {
                calls += 1;
                placement(c)
            })
            .is_err(),
            "variant {variant}"
        );
        assert_eq!(calls, 0);
    }
}

#[test]
fn hidden_cells_skip_callbacks_but_zero_ink_and_zero_opacity_actual_marks_do_not() {
    let source = [HeatCell::new("row", "a", 0.), HeatCell::new("row", "b", 1.)];
    let options = HeatmapLayoutOptions {
        size_variation: Some(HeatmapSizeVariation {
            sizes: [0., 0.],
            ..Default::default()
        }),
        opacity: 0.,
        ..Default::default()
    };
    let layout = heatmap_layout(&ChartConfig::default(), &source, &options).unwrap();
    let mut calls = 0;
    let packet = heatmap_cell_widgets(&layout, 0, |ctx| {
        calls += 1;
        assert_eq!(ctx.mark.opacity, 0.);
        assert_eq!(ctx.mark.fill, None);
        placement(ctx)
    })
    .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(packet.data.len(), 2);
    let hidden = heatmap_layout(
        &ChartConfig::default(),
        &source,
        &HeatmapLayoutOptions {
            hidden_rows: ["row".into()].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let packet = heatmap_cell_widgets(&hidden, 0, |_| {
        panic!("hidden cells must not invoke placement")
    })
    .unwrap();
    assert!(packet.data.is_empty());
}

#[test]
fn selective_placement_and_bounded_failure_leave_layout_unmodified() {
    let source: Vec<_> = (0..129)
        .map(|i| HeatCell::new("row", i.to_string(), i as f64))
        .collect();
    let layout = heatmap_layout(
        &ChartConfig {
            width: 2000.,
            ..Default::default()
        },
        &source,
        &HeatmapLayoutOptions {
            axes: HeatmapAxes {
                top: None,
                right: None,
                bottom: None,
                left: None,
            },
            ..Default::default()
        },
    )
    .unwrap();
    let before = layout.scene.clone();
    let mut calls = 0;
    assert!(
        heatmap_cell_widgets(&layout, 0, |c| {
            calls += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    assert_eq!(layout.scene, before);
    let packet = heatmap_cell_widgets(&layout, 9, |c| {
        if c.source_index == 4 {
            placement(c)
        } else {
            Ok(None)
        }
    })
    .unwrap();
    assert_eq!(packet.data.len(), 1);
    assert_eq!(packet.layout.slots[0].revision, 9);
    assert!(
        heatmap_cell_widgets(&layout, 9, |c| if c.source_index == 4 {
            Err(ChartError("placement stopped".into()))
        } else {
            placement(c)
        })
        .is_err()
    );
}

fn nodes(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = todo.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|s| s == "heat-cell-child")
        {
            out.push(id)
        }
        todo.extend(n.children.iter().copied());
    }
    out
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(b) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("button")
    };
    b.label.clone()
}

#[test]
fn actual_glory_children_keep_nullable_data_state_and_reject_whole_failed_revisions() {
    let app = App::memory();
    let input = app.scope().cage(model::source(0, 600., 0).unwrap());
    let layout = app.scope().bond(move || input.with(|p| p.0.clone()));
    let packet = app.scope().bond(move || input.with(|p| p.1.clone()));
    let rejects = Rc::new(RefCell::new(vec![]));
    let errors = rejects.clone();
    let count = Rc::new(Cell::new(0));
    let called = count.clone();
    app.mount(
        chart_layout(layout)
            .on_reject(move |e| errors.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |ctx| {
                called.set(called.get() + 1);
                if ctx.revision == 3 {
                    return Err(ChartError("heatmap child rejected".into()));
                }
                let frame = ctx.frame;
                Ok(glory::ui::widget(move |scope| {
                    let clicks = scope.cage(0usize);
                    glory::ui::button()
                        .class("heat-cell-child")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}:{} {:?} index={} count={}",
                                    f.data.datum.row,
                                    f.data.datum.column,
                                    f.data.datum.value,
                                    f.data.source_index,
                                    clicks.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| clicks.revise(|v| *v += 1))
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    let id = nodes(&app)
        .into_iter()
        .find(|id| text(&app, *id).contains("north:温度:null"))
        .unwrap();
    app.dispatch(id, Event::Activate).unwrap();
    let before = text(&app, id);
    assert!(before.contains("None index=1 count=1"));
    input.revise(|p| *p = model::source(1, 600., 0).unwrap());
    app.flush().unwrap();
    assert!(nodes(&app).contains(&id));
    assert!(text(&app, id).contains("None index=3 count=1"));
    assert_eq!(count.get(), 5);
    input.revise(|p| *p = model::source(2, 600., 0).unwrap());
    app.flush().unwrap();
    assert!(text(&app, id).contains("Some(0.0)"));
    let accepted = text(&app, id);
    input.revise(|p| *p = model::source(3, 640., 3).unwrap());
    app.flush().unwrap();
    assert_eq!(text(&app, id), accepted);
    assert!(!rejects.borrow().is_empty());
    input.revise(|p| *p = model::source(3, 640., 0).unwrap());
    app.flush().unwrap();
    assert!(nodes(&app).contains(&id));
    assert!(text(&app, id).ends_with("count=1"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

#[test]
fn explicit_painted_conversion_checks_solid_ink_before_returning_a_native_candidate() {
    let layout = heatmap_layout(
        &ChartConfig::default(),
        &[HeatCell::new("r", "c", 4.)],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        layout
            .clone()
            .into_scene_interaction()
            .unwrap()
            .interaction
            .unwrap()
            .picking_profile,
        PickingProfile::LegacyGeometry
    );
    assert_eq!(
        layout
            .clone()
            .into_painted_scene_interaction()
            .unwrap()
            .interaction
            .unwrap()
            .picking_profile,
        PickingProfile::PaintedGeometry
    );
    let mut gradient = layout;
    gradient
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == gradient.cells[0].mark_id)
        .unwrap()
        .fill_paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [1., 1.],
        stops: vec![
            GradientStop {
                offset: 0.,
                color: Color(255, 0, 0, 255),
            },
            GradientStop {
                offset: 1.,
                color: Color(0, 0, 255, 255),
            },
        ],
    });
    assert!(gradient.clone().into_scene_interaction().is_ok());
    assert!(gradient.into_painted_scene_interaction().is_err());
}

#[cfg(feature = "raster")]
#[test]
fn actual_raster_publisher_failure_preserves_nullable_child_data_and_previous_pixels() {
    use kartis::render::raster::{FontRegistry, RasterOptions, RenderedImage};
    for dpr in [1., 2.] {
        let app = App::memory();
        let input = app.scope().cage(model::source(0, 600., 0).unwrap());
        let scene = app.scope().bond(move || input.with(|p| p.0.scene.clone()));
        let packet = app.scope().bond(move || input.with(|p| p.1.clone()));
        let reject = Rc::new(Cell::new(false));
        let denied = reject.clone();
        let image = Rc::new(RefCell::new(None::<RenderedImage>));
        let output = image.clone();
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
            // Scene-only raster_chart has no nullable typed source pointer
            // binding. Actual child controls read the adapter's raw metadata.
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .raster_image_source(move |frame| {
                if denied.get() {
                    return Err(ChartError("heatmap PNG denied".into()));
                }
                let url = frame.data_url()?;
                *output.borrow_mut() = Some(frame.clone());
                Ok(url)
            })
            .on_reject(move |error| seen.borrow_mut().push(error.0))
            .plot_widgets_composed(packet, |ctx| {
                let frame = ctx.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0usize);
                    glory::ui::button()
                        .class("heat-cell-child")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}:{} {:?} count={}",
                                    f.data.datum.row,
                                    f.data.datum.column,
                                    f.data.datum.value,
                                    count.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                        .show_in(scope)
                }))
            }),
        )
        .unwrap();
        let child = nodes(&app)
            .into_iter()
            .find(|id| text(&app, *id).contains("north:温度:null"))
            .unwrap();
        app.dispatch(child, Event::Activate).unwrap();
        let initial = image.borrow().clone().unwrap();
        assert!(
            initial
                .rgba
                .chunks_exact(4)
                .all(|p| p == [255, 255, 255, 255])
        );
        let accepted = text(&app, child);
        assert!(accepted.contains("None count=1"));
        reject.set(true);
        input.revise(|p| *p = model::source(3, 640., 0).unwrap());
        app.flush().unwrap();
        assert_eq!(text(&app, child), accepted);
        assert_eq!(image.borrow().as_ref(), Some(&initial));
        assert_eq!(errors.borrow().len(), 1);
        reject.set(false);
        input.revise(|p| *p = model::source(2, 640., 0).unwrap());
        app.flush().unwrap();
        assert!(nodes(&app).contains(&child));
        assert!(text(&app, child).contains("Some(0.0) count=1"));
        assert_eq!(image.borrow().as_ref().unwrap().width, (640. * dpr) as u32);
        input.revise(|p| p.1.replace_ink.clear());
        app.flush().unwrap();
        assert!(
            image
                .borrow()
                .as_ref()
                .unwrap()
                .rgba
                .chunks_exact(4)
                .any(|p| p != [255, 255, 255, 255])
        );
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
        assert_eq!(app.stats().handlers, 0);
    }
}
