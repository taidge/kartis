//! Real Glory plot Widget slots over both application-owned browser surfaces.
use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, Style};

use super::*;

#[derive(Default)]
struct Counts {
    factories: Cell<usize>,
    cleanups: Cell<usize>,
    selections: Cell<usize>,
    reentrant_reads: Cell<usize>,
    reject_relayout: Cell<bool>,
    errors: RefCell<Vec<String>>,
}
struct Fixture {
    _chart: BrowserChart,
    handle: BrowserHandle,
    counts: Rc<Counts>,
    relayout: bool,
}
thread_local! {
    static FIXTURES: RefCell<BTreeMap<String, Fixture>> = const { RefCell::new(BTreeMap::new()) };
}

fn plot(width: f64, height: f64) -> ClipRect {
    ClipRect {
        x: 20.,
        y: 20.,
        width: width - 40.,
        height: height - 40.,
    }
}

fn ring(x: f64, y: f64, enclosing_hole: bool) -> ClipPath {
    let rect = |half_width, half_height| Geometry::Rect {
        x: x - half_width,
        y: y - half_height,
        width: 2. * half_width,
        height: 2. * half_height,
    };
    let (outer, inner) = if enclosing_hole {
        (rect(48., 28.), rect(28., 13.))
    } else {
        (rect(20., 8.), rect(6., 4.))
    };
    let mut commands = outer.commands();
    commands.extend(inner.commands());
    ClipPath {
        geometry: Geometry::Path(commands),
        rule: ClipRule::EvenOdd,
    }
}

fn source(width: f64, height: f64, x: f64, value: f64, mask: bool) -> Result<SceneInteraction> {
    let mut scene = ChartConfig {
        width,
        height,
        title: "Browser measured plot Widgets".into(),
        ..Default::default()
    }
    .scene()?;
    let mut binding = InteractionBinding::default();
    for (id, x, y, datum, color) in [
        ("alpha", x, 75., value, Color(48, 119, 192, 255)),
        ("beta", width * 0.71, 130., 20., Color(211, 121, 48, 255)),
    ] {
        let geometry = if id == "alpha" {
            Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(x - 9., y - 5.)),
                PathCommand::Line(GraphicPoint::new(x + 15., y - 5.)),
                PathCommand::Line(GraphicPoint::new(x - 6., y + 10.)),
                PathCommand::Close,
            ])
        } else {
            Geometry::Circle { x, y, radius: 13. }
        };
        if id == "alpha" {
            binding.anchors.insert(
                id.into(),
                InteractionAnchor::path_points(&geometry, [x, y], &[0, 1, 2])?,
            );
        }
        binding.sites.push(InteractionSite {
            mark_id: id.into(),
            series: id.into(),
            label: id.into(),
            value: Some(datum),
            formatted_value: Some(datum.to_string()),
            color,
            position: [x, y],
        });
        let mut mark = Mark::new(id, geometry, color);
        mark.series = id.into();
        // Target and binding agree. The overlay must use this new business
        // value while the animation still paints the preceding coordinates.
        mark.value = datum;
        if id == "alpha" && mask {
            mark.clip_path = Some(Box::new(ring(x, y, false)));
        }
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color,
        });
    }
    Ok(SceneInteraction {
        scene,
        interaction: Some(binding),
        plot: Some(plot(width, height)),
    })
}

fn slots(revision: u64, partially_clipped: bool) -> PlotWidgetsLayout {
    PlotWidgetsLayout {
        plot: None,
        slots: vec![
            PlotWidgetSlot {
                mark_id: "alpha".into(),
                revision,
                placement: if partially_clipped {
                    PlotWidgetPlacement::Rect(LayoutRect {
                        x: 12.,
                        y: 65.,
                        width: 48.,
                        height: 20.,
                    })
                } else {
                    PlotWidgetPlacement::AtDatum {
                        offset: [-24., -10.],
                        measured: TextSize {
                            width: 48.,
                            height: 20.,
                        },
                    }
                },
            },
            PlotWidgetSlot {
                mark_id: "beta".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-24., 18.],
                    measured: TextSize {
                        width: 48.,
                        height: 20.,
                    },
                },
            },
        ],
    }
}

fn root(id: &str) -> std::result::Result<web_sys::Element, JsValue> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .ok_or_else(|| js("missing slot root"))
}

#[wasm_bindgen]
pub fn browser_plot_widgets_mount(
    id: &str,
    canvas: bool,
    relayout: bool,
) -> std::result::Result<(), JsValue> {
    browser_plot_widgets_stop(id);
    let initial = source(400., 220., 100., 10., false).map_err(js)?;
    let mut options = BrowserOptions {
        height: Some(220.),
        interaction_data: initial.interaction,
        renderer: if canvas {
            BrowserRenderer::Canvas
        } else {
            BrowserRenderer::Svg
        },
        ..Default::default()
    };
    options.interaction.plot = initial.plot;
    let counters = Rc::new(Counts::default());
    let factory_counts = counters.clone();
    let handle_slot = Rc::new(RefCell::new(None::<BrowserHandle>));
    let callback_handle = handle_slot.clone();
    let chart = BrowserChart::mount_with_plot_widgets(
        root(id)?,
        initial.scene,
        options,
        slots(0, false),
        move |context| {
            factory_counts
                .factories
                .set(factory_counts.factories.get() + 1);
            if let Some(handle) = callback_handle.borrow().as_ref() {
                handle.snapshot()?;
                factory_counts
                    .reentrant_reads
                    .set(factory_counts.reentrant_reads.get() + 1);
                if context.revision == 4 {
                    handle.stop();
                    return Err(ChartError("retired Widget factory".into()));
                }
            }
            if context.revision == 1 {
                return Err(ChartError("browser slot factory refused revision 1".into()));
            }
            let counters = factory_counts.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = counters.clone();
                scope.on_cleanup(move || retired.cleanups.set(retired.cleanups.get() + 1))?;
                let count = scope.cage(0usize);
                let frame = context.frame;
                let revision = context.revision;
                glory::ui::button()
                    .class(format!("browser-slot-{}", context.mark_id))
                    .label(scope.bond(move || {
                        frame.with(|frame| {
                            format!(
                                "{} {} r{revision} ×{}",
                                frame.mark.id,
                                frame.mark.value,
                                count.get()
                            )
                        })
                    }))
                    .style(
                        Style::new()
                            .width(Length::Percent(100.))
                            .height(Length::Percent(100.)),
                    )
                    .on(EventKind::Activate, move |_| {
                        count.revise(|value| *value += 1)
                    })
                    .show_in(scope)?;
                if revision == 2 {
                    Err(
                        glory::render::Error::InvalidProps("browser slot child refused revision 2")
                            .into(),
                    )
                } else {
                    Ok(())
                }
            }))
        },
    )
    .map_err(js)?;
    let handle = chart.handle();
    *handle_slot.borrow_mut() = Some(handle.clone());
    let events = counters.clone();
    chart.on_event(move |event| match event {
        BrowserEvent::Error(error) => events.errors.borrow_mut().push(error.0),
        BrowserEvent::Interaction(SurfaceEvent::Selected(_)) => {
            events.selections.set(events.selections.get() + 1)
        }
        _ => {}
    });
    if relayout {
        let viewport = counters.clone();
        let controller = DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new([0., 10.], ScaleKind::Linear),
                left: None,
                right: None,
            },
            plot(400., 220.),
            Default::default(),
        )
        .map_err(js)?;
        chart
            .on_viewport_with_interaction(controller, ViewportSelection::X, move |request| {
                let [lo, hi] = request.domains.x;
                let x =
                    request.plot.x + (2. / 9. - lo / 10.) / ((hi - lo) / 10.) * request.plot.width;
                let mut target = source(request.width, request.height, x, 10., false)?;
                if viewport.reject_relayout.get() {
                    target.scene.marks[0].clip_path = Some(Box::new(ClipPath {
                        geometry: Geometry::Path(vec![
                            PathCommand::Move(GraphicPoint::new(x, 75.)),
                            PathCommand::Line(GraphicPoint::new(x + 10., 75.)),
                        ]),
                        rule: ClipRule::NonZero,
                    }));
                }
                Ok(target)
            })
            .map_err(js)?;
    }
    FIXTURES.with(|fixtures| {
        fixtures.borrow_mut().insert(
            id.into(),
            Fixture {
                _chart: chart,
                handle,
                counts: counters,
                relayout,
            },
        );
    });
    Ok(())
}

fn owned(id: &str) -> std::result::Result<(BrowserHandle, Rc<Counts>, bool), JsValue> {
    FIXTURES
        .with(|fixtures| {
            fixtures.borrow().get(id).map(|fixture| {
                (
                    fixture.handle.clone(),
                    fixture.counts.clone(),
                    fixture.relayout,
                )
            })
        })
        .ok_or_else(|| js("plot Widget fixture is not mounted"))
}

#[wasm_bindgen]
pub fn browser_plot_widgets_phase(id: &str, phase: u8) -> std::result::Result<(), JsValue> {
    let (handle, counts, _) = owned(id)?;
    let snapshot = handle.snapshot().map_err(js)?;
    match phase {
        1 => {
            let target = source(snapshot.width, snapshot.height, 125., 42., false).map_err(js)?;
            handle
                .animate_to_with_interaction(target.scene, target.interaction.unwrap())
                .map_err(js)
        }
        2 => handle.set_plot_widgets(slots(0, true)).map_err(js),
        3 => handle.set_plot_widgets(slots(1, false)).map_err(js),
        4 => handle.set_plot_widgets(slots(2, false)).map_err(js),
        5 => handle.set_plot_widgets(slots(0, false)).map_err(js),
        6 => handle.set_plot_widgets(slots(3, false)).map_err(js),
        7 => {
            let mut invalid = slots(3, false);
            invalid.slots.push(invalid.slots[0].clone());
            let target = source(470., snapshot.height, 200., 55., false).map_err(js)?;
            handle
                .set_layout_with_plot_widgets(target, invalid)
                .map_err(js)
        }
        8 => {
            let target = source(snapshot.width, snapshot.height, 160., 99., true).map_err(js)?;
            handle
                .set_scene_with_interaction(target.scene, target.interaction.unwrap())
                .map_err(js)
        }
        9 => {
            let mut target =
                source(snapshot.width, snapshot.height, 125., 42., false).map_err(js)?;
            target.scene.marks.remove(0);
            let binding = target.interaction.as_mut().unwrap();
            binding.sites.remove(0);
            binding.anchors.clear();
            let mut remaining = slots(0, false);
            remaining.slots.remove(0);
            handle
                .set_layout_with_plot_widgets(target, remaining)
                .map_err(js)
        }
        10 => {
            let mut layout = slots(4, false);
            layout.slots.remove(0);
            layout.slots[0].revision = 4;
            handle.set_plot_widgets(layout).map_err(js)
        }
        11 => {
            counts.reject_relayout.set(!counts.reject_relayout.get());
            Ok(())
        }
        12 | 13 => {
            let x = 160.;
            let mut target = source(snapshot.width, snapshot.height, x, 99., false).map_err(js)?;
            let clip = if phase == 12 {
                ClipPath::circle(x, 75., 40.)
            } else {
                ring(x, 75., true)
            };
            target.scene.marks[0].clip_path = Some(Box::new(clip));
            handle
                .set_scene_with_interaction(target.scene, target.interaction.unwrap())
                .map_err(js)
        }
        _ => Err(js("unknown browser plot Widget phase")),
    }
}

#[wasm_bindgen]
pub fn browser_plot_widgets_flush(id: &str) -> std::result::Result<(), JsValue> {
    owned(id)?.0.flush().map_err(js)
}

#[wasm_bindgen]
pub fn browser_plot_widgets_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    let Ok((handle, counts, relayout)) = owned(id) else {
        return Ok(json!({"active":false}).to_string());
    };
    let snapshot = handle.snapshot().ok();
    Ok(json!({
        "active": snapshot.is_some(), "relayout": relayout,
        "state": snapshot.map(|state| json!({
            "width": state.width, "height": state.height, "pending": state.pending,
            "hovered": state.hovered, "selected": state.selected, "animating": state.animating,
            "hidden": state.hidden, "view": [state.view.scale, state.view.translate_x, state.view.translate_y],
            "domains": state.domains.map(|domains| domains.x),
        })),
        "factories": counts.factories.get(), "cleanups": counts.cleanups.get(),
        "selections": counts.selections.get(), "reentrantReads": counts.reentrant_reads.get(),
        "errors": counts.errors.borrow().clone(),
    }).to_string())
}

#[wasm_bindgen]
pub fn browser_plot_widgets_stop(id: &str) {
    let previous = FIXTURES.with(|fixtures| fixtures.borrow_mut().remove(id));
    drop(previous);
}
