//! Real controller-owned crosshairs, typed slices and current safe tooltip rows.
use std::collections::BTreeSet;

use super::*;
struct LineCrosshairFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: &'static str,
    case: u8,
    phase: Rc<Cell<u8>>,
    events: Rc<RefCell<Vec<Value>>>,
    stop_on_event: Rc<Cell<bool>>,
    reject_viewport: Rc<Cell<bool>>,
    stop_viewport: Rc<Cell<bool>>,
}
thread_local! {static LINE_CROSSHAIRS:RefCell<Vec<LineCrosshairFixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 50.,
            right: 40.,
            top: 35.,
            bottom: 45.,
        },
        title: "Typed line crosshairs".into(),
        ..Default::default()
    }
}
fn source(width: f64, height: f64, phase: u8) -> Result<LineLayout> {
    source_domains(width, height, phase, None, BTreeSet::new())
}
fn source_domains(
    width: f64,
    height: f64,
    phase: u8,
    domains: Option<ResolvedDomains>,
    hidden: BTreeSet<String>,
) -> Result<LineLayout> {
    if phase > 1 {
        return Err(ChartError("line crosshair phase must be 0 or 1".into()));
    }
    let p = f64::from(phase);
    let mut options = LineOptions {
        x_domain: Some(domains.as_ref().map_or([0., 10.], |d| d.x)),
        y_domain: Some(domains.and_then(|d| d.left).unwrap_or([0., 10.])),
        hidden,
        points: false,
        ..Default::default()
    };
    options.axes.y.format = ValueFormat::Fixed(2);
    line_layout_with_style(
        &config(width, height),
        &[
            Series::new(
                "Alpha <safe>",
                vec![
                    Point::new(0., 2.),
                    Point::new(5., 6. + p),
                    Point::gap(7.),
                    Point::new(10., 8.),
                ],
            ),
            Series::new(
                "Beta",
                vec![
                    Point::new(0., 8.),
                    Point::new(5., 2.),
                    Point::new(10., 6. + p),
                ],
            ),
        ],
        &options,
        &LineStyleOptions {
            interactive_sites: true,
            connect_missing: true,
            series: LineSeriesStyle {
                width: 3.,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
fn active_value(active: Option<ActiveInteraction>) -> Value {
    active.map_or(Value::Null,|a|json!({"slice":a.slice_id,"label":a.label,"anchor":a.anchor,"ids":a.ids,"rows":a.rows.iter().map(|r|json!({"id":r.mark_id,"series":r.series,"label":r.label,"value":r.value,"formatted":r.formatted_value,"position":r.position,"color":[r.color.0,r.color.1,r.color.2,r.color.3]})).collect::<Vec<_>>()}))
}
#[wasm_bindgen]
pub fn line_crosshair_mount(case: u8) -> std::result::Result<(), JsValue> {
    if case > 2 {
        return Err(js("line crosshair case must be 0..=2"));
    }
    let layout = source(640., 360., 0).map_err(js)?;
    line_crosshair_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("line-crosshair-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let picking = match case {
            0 => surface::Picking::Nearest { distance: 45. },
            1 => surface::Picking::SliceX { distance: 640. },
            _ => surface::Picking::SliceY { distance: 360. },
        };
        let chart = match BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                motion: browser::MotionPreference::Animate,
                crosshair: Some(CrosshairOptions {
                    kind: CrosshairType::BottomLeft,
                    stroke: GraphicStroke {
                        color: Color(210, 35, 65, 255),
                        width: 2.,
                        cap: StrokeCap::Butt,
                    },
                    dash: None,
                    opacity: 1.,
                }),
                interaction_data: Some(layout.interaction_binding()),
                interaction: surface::SurfaceOptions {
                    plot: Some(ClipRect::plot(&config(640., 360.))),
                    picking,
                    pan: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                line_crosshair_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        let handle = chart.handle();
        let stop_on_event = Rc::new(Cell::new(false));
        let stop = stop_on_event.clone();
        chart.on_event(move|event| {
            let should_stop=matches!(&event,BrowserEvent::Interaction(SurfaceEvent::Hover(ids))if !ids.is_empty())&&stop.replace(false);
            let mut value=event_value(event);value["business"]=active_value(handle.snapshot().ok().and_then(|s|s.interaction));
            let mut logs=capture.borrow_mut();let sequence=logs.last().and_then(|v:&Value|v.get("sequence")).and_then(Value::as_u64).unwrap_or(0).saturating_add(1);value["sequence"]=json!(sequence);if logs.len()==64{logs.remove(0);}logs.push(value);drop(logs);
            if should_stop{handle.stop();}
        });
        if let Err(error) = chart.flush() {
            chart.stop();
            root.remove();
            line_crosshair_stop();
            return Err(js(error));
        }
        LINE_CROSSHAIRS.with(|slot| {
            slot.borrow_mut().push(LineCrosshairFixture {
                chart,
                root,
                renderer: name,
                case,
                phase: Rc::new(Cell::new(0)),
                events,
                stop_on_event,
                reject_viewport: Rc::new(Cell::new(false)),
                stop_viewport: Rc::new(Cell::new(false)),
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn line_crosshair_phase(phase: u8) -> std::result::Result<(), JsValue> {
    change(phase, false)
}
#[wasm_bindgen]
pub fn line_crosshair_animate(phase: u8) -> std::result::Result<(), JsValue> {
    change(phase, true)
}
fn change(phase: u8, animate: bool) -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            let s = f.chart.snapshot().map_err(js)?;
            let layout = source(s.width, s.height, phase).map_err(js)?;
            let binding = layout.interaction_binding();
            if animate {
                f.chart
                    .animate_to_with_interaction(layout.scene, binding)
                    .map_err(js)?;
            } else {
                f.chart
                    .set_scene_with_interaction(layout.scene, binding)
                    .map_err(js)?;
            }
            f.phase.set(phase);
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn line_crosshair_flush() -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            if f.chart.handle().is_active() {
                f.chart.flush().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn line_crosshair_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    LINE_CROSSHAIRS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};if !f.chart.handle().is_active(){return Ok(json!({"active":false,"events":f.events.borrow().clone()}).to_string());}let s=f.chart.snapshot().map_err(js)?;let layout=source_domains(s.width,s.height,f.phase.get(),s.domains,BTreeSet::new()).map_err(js)?;let target=layout.sites.iter().find(|p|p.series_id=="Alpha <safe>"&&p.raw_x==5.).unwrap();let plot=ClipRect::plot(&config(s.width,s.height));Ok(json!({"active":s.active,"case":f.case,"phase":f.phase.get(),"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"animating":s.animating,"hovered":s.hovered,"selected":s.selected,"hidden":s.hidden,"view":{"scale":s.view.scale,"x":s.view.translate_x,"y":s.view.translate_y},"domains":s.domains.map(|d|json!({"x":d.x,"left":d.left,"right":d.right})),"plot":{"x":plot.x,"y":plot.y,"width":plot.width,"height":plot.height},"current":active_value(s.interaction),"target":{"id":target.mark_id,"x":target.position[0],"y":target.position[1],"rawX":target.raw_x,"rawY":target.raw_y,"formattedX":target.formatted_x,"formattedY":target.formatted_y},"sites":layout.sites.iter().map(|p|json!({"id":p.mark_id,"series":p.series_id,"x":p.raw_x,"y":p.raw_y,"position":p.position,"formattedX":p.formatted_x,"formattedY":p.formatted_y,"sourceIndex":p.source_index})).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn line_crosshair_rejected() -> bool {
    LINE_CROSSHAIRS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(s) = f.chart.snapshot() else {
                return false;
            };
            let Ok(layout) = source(s.width, s.height, f.phase.get()) else {
                return false;
            };
            let mut bad = layout.interaction_binding();
            bad.sites[0].position[0] += 1.;
            if f.chart
                .set_scene_with_interaction(layout.scene.clone(), bad)
                .is_ok()
            {
                return false;
            }
            let mut conflict = layout.scene;
            conflict.marks[0].id = CROSSHAIR_X_ID.into();
            f.chart.set_scene(conflict).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn line_crosshair_motion_rejected() -> std::result::Result<bool, JsValue> {
    use kartis::motion::{
        ContourBindings, ContourMorphOptions, MorphContour, PathMorphOptions, PropertyMotionOptions,
    };
    let layout = source(640., 360., 0).map_err(js)?;
    let binding = layout.interaction_binding();
    let target = binding
        .sites
        .iter()
        .find(|s| s.mark_id == "line-point:12:Alpha <safe>:1")
        .unwrap()
        .position;
    let mut scene = layout.scene;
    let mut audit = Mark::new(
        "motion-audit",
        Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 10.,
        },
        Color(0, 0, 0, 0),
    );
    audit.label.clear();
    scene.marks.push(audit);
    let mut options = PropertyMotionOptions::default();
    options.geometry.contour_morphs.insert(
        "motion-audit".into(),
        ContourMorphOptions {
            sampling: PathMorphOptions {
                samples: 8,
                ..Default::default()
            },
            contours: vec![
                MorphContour {
                    key: "outer".into(),
                    closed: true,
                    collapse: [100., 100.],
                },
                MorphContour {
                    key: "inner".into(),
                    closed: true,
                    collapse: [100., 100.],
                },
            ],
        },
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_attribute("style", "width:640px")?;
    document.body().unwrap().append_child(&root)?;
    let result = (|| -> Result<bool> {
        let chart = BrowserChart::mount(
            root.clone(),
            scene.clone(),
            BrowserOptions {
                renderer: BrowserRenderer::Svg,
                height: Some(360.),
                legend: false,
                data_controls: false,
                tooltip: false,
                motion: browser::MotionPreference::Reduced,
                interaction_data: Some(binding.clone()),
                interaction: surface::SurfaceOptions {
                    picking: surface::Picking::Nearest { distance: 20. },
                    ..Default::default()
                },
                animation: BrowserAnimation::PropertiesWithContours {
                    options,
                    bindings: ContourBindings::from([(
                        "motion-audit".into(),
                        vec!["outer".into()],
                    )]),
                },
                ..Default::default()
            },
        )?;
        chart.handle().dispatch(SurfaceInput::Move {
            id: 1,
            point: (target[0], target[1]),
        })?;
        chart.flush()?;
        let before = chart.snapshot()?;
        let dom = root.inner_html();
        let frame = root
            .query_selector("svg")
            .ok()
            .flatten()
            .ok_or_else(|| ChartError("motion audit SVG missing".into()))?;
        let frame_dom = frame.inner_html();
        let mut invalid = scene;
        invalid.marks.last_mut().unwrap().geometry = Geometry::Rect {
            x: 90.,
            y: 90.,
            width: 20.,
            height: 20.,
        };
        let old = chart.set_scene(invalid.clone()).is_err();
        let typed = chart
            .set_scene_with_interaction(invalid.clone(), binding.clone())
            .is_err();
        let whole = chart
            .set_layout(SceneInteraction {
                scene: invalid,
                interaction: Some(binding),
                plot: None,
            })
            .is_err();
        let after = chart.snapshot()?;
        let atomic = before.revision == after.revision
            && before.pending == after.pending
            && before.interaction == after.interaction;
        let same_before_flush = dom == root.inner_html();
        chart.flush()?;
        let same = same_before_flush
            && frame_dom == frame.inner_html()
            && root.query_selector("svg").ok().flatten().as_ref() == Some(&frame);
        chart.stop();
        Ok(old && typed && whole && atomic && same)
    })();
    root.remove();
    result.map_err(js)
}
fn budget_source(prefix: &str, count: usize, value: f64) -> Result<SceneInteraction> {
    let mut scene = ChartConfig {
        width: 120.,
        height: 100.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
    .scene()?;
    for i in 0..count {
        let mut mark = Mark::new(
            format!("{prefix}{i}"),
            Geometry::Circle {
                x: 60.,
                y: 50.,
                radius: if i == 0 { 4. } else { 0. },
            },
            if i == 0 {
                Color(25, 80, 180, 255)
            } else {
                Color(0, 0, 0, 0)
            },
        );
        mark.label.clear();
        if i == 0 {
            mark.label = format!("{prefix} budget");
            mark.series = "budget".into();
            mark.value = value;
        }
        scene.marks.push(mark);
    }
    Ok(SceneInteraction {
        scene,
        plot: None,
        interaction: Some(InteractionBinding {
            sites: vec![InteractionSite {
                mark_id: format!("{prefix}0"),
                series: "budget".into(),
                label: "budget".into(),
                value: Some(value),
                formatted_value: Some(format!("{value:.2}")),
                color: Color(25, 80, 180, 255),
                position: [60., 50.],
            }],
            ..Default::default()
        }),
    })
}
/// Tiny actual Canvas hosts exercise the full 20k transient union without
/// making 20k accessible controls or retaining any extra fixture owners.
#[wasm_bindgen]
pub fn line_crosshair_budget_rejected() -> std::result::Result<String, JsValue> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();
    let mut reports = Vec::new();
    for property in [false, true] {
        let root = document.create_element("div")?;
        root.set_attribute("style", "width:120px")?;
        document.body().unwrap().append_child(&root)?;
        let outcome = (|| -> Result<Value> {
            let initial = budget_source("old", 10_000, 6.)?;
            let chart = BrowserChart::mount(
                root.clone(),
                initial.scene,
                BrowserOptions {
                    renderer: BrowserRenderer::Canvas,
                    height: Some(100.),
                    legend: false,
                    data_controls: false,
                    tooltip: false,
                    motion: browser::MotionPreference::Animate,
                    animation: if property {
                        BrowserAnimation::Properties(Default::default())
                    } else {
                        BrowserAnimation::Scene
                    },
                    crosshair: Some(CrosshairOptions {
                        kind: CrosshairType::Cross,
                        dash: None,
                        ..Default::default()
                    }),
                    interaction_data: initial.interaction,
                    ..Default::default()
                },
            )?;
            let errors = Rc::new(Cell::new(0u32));
            let capture = errors.clone();
            chart.on_event(move |event| {
                if matches!(event, BrowserEvent::Error(_)) {
                    capture.set(capture.get() + 1);
                }
            });
            chart.handle().dispatch(SurfaceInput::Move {
                id: 1,
                point: (60., 50.),
            })?;
            chart.flush()?;
            let canvas = root
                .query_selector("canvas")
                .ok()
                .flatten()
                .ok_or_else(|| ChartError("budget Canvas missing".into()))?
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .map_err(|_| ChartError("budget Canvas type".into()))?;
            let png = canvas
                .to_data_url()
                .map_err(|_| ChartError("budget PNG export".into()))?;
            let dom = root.inner_html();
            let before = chart.snapshot()?;
            let target = budget_source("new", 10_000, 7.)?;
            let legacy_error = chart.animate_to(target.scene.clone()).err().map(|e| e.0);
            let typed_error = chart
                .animate_to_with_interaction(
                    target.scene.clone(),
                    target.interaction.clone().unwrap(),
                )
                .err()
                .map(|e| e.0);
            let contour_error = if property {
                chart
                    .animate_to_with_contours(target.scene.clone(), Default::default())
                    .err()
                    .map(|e| e.0)
            } else {
                None
            };
            let after = chart.snapshot()?;
            let unchanged = before == after && dom == root.inner_html();
            chart.flush()?;
            let pixels_unchanged = png
                == canvas
                    .to_data_url()
                    .map_err(|_| ChartError("budget PNG export".into()))?;
            chart.set_layout(target)?;
            chart.flush()?;
            let instant = chart.snapshot()?;
            let instant_accepted = !instant.animating
                && instant.interaction.as_ref().is_some_and(|a| {
                    a.rows
                        .first()
                        .is_some_and(|r| r.mark_id == "new0" && r.value == Some(7.))
                });
            chart.set_layout(budget_source("old", 9_999, 6.)?)?;
            chart.flush()?;
            let boundary = budget_source("new", 9_999, 7.)?;
            chart.animate_to_with_interaction(boundary.scene, boundary.interaction.unwrap())?;
            chart.flush()?;
            chart.handle().dispatch(SurfaceInput::Move {
                id: 1,
                point: (60., 50.),
            })?;
            chart.flush()?;
            let accepted = chart.snapshot()?;
            let boundary_accepted = accepted.active
                && accepted.animating
                && accepted.interaction.as_ref().is_some_and(|a| {
                    a.rows
                        .first()
                        .is_some_and(|r| r.mark_id == "new0" && r.value == Some(7.))
                });
            chart.stop();
            Ok(
                json!({"mode":if property{"properties"}else{"scene"},"oldAndNew":[10000,10000],"legacyError":legacy_error,"typedError":typed_error,"contourError":contour_error,"unchangedBefore":unchanged,"pixelsUnchanged":pixels_unchanged,"instantAccepted":instant_accepted,"boundary":[9999,9999],"boundaryAccepted":boundary_accepted,"errors":errors.get()}),
            )
        })();
        root.remove();
        reports.push(outcome.map_err(js)?);
    }
    Ok(Value::Array(reports).to_string())
}
fn dashed_budget_source(width: f64, height: f64) -> Result<SceneInteraction> {
    let mut source = budget_source("dash", 1, 6.)?;
    source.scene.width = width;
    source.scene.height = height;
    let mut decoration = Mark::new(
        "dash-budget-decoration",
        Geometry::Path(vec![
            glory::render::PathCommand::Move(glory::render::GraphicPoint::new(0., -10.)),
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(1500., -10.)),
        ]),
        Color(0, 0, 0, 0),
    );
    decoration.label.clear();
    decoration.fill = None;
    decoration.stroke = Some(glory::render::GraphicStroke {
        color: Color(60, 70, 80, 255),
        width: 1.,
        cap: glory::render::StrokeCap::Butt,
    });
    decoration.opacity = 0.;
    decoration.stroke_dash = Some(StrokeDash {
        segments: vec![0.01, 0.01],
        offset: 0.,
    });
    source.scene.marks.push(decoration);
    source.validate()?;
    Ok(source)
}
/// The actual controller must reject a scalar-valid dash before claiming DOM,
/// and account for overlay and business dash expansion in one shared budget.
#[wasm_bindgen]
pub fn line_crosshair_dash_rejected() -> std::result::Result<String, JsValue> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();
    let mut reports = Vec::new();
    for renderer in [BrowserRenderer::Svg, BrowserRenderer::Canvas] {
        let root = document.create_element("div")?;
        root.set_attribute("style", "width:120px")?;
        document.body().unwrap().append_child(&root)?;
        let tiny = CrosshairOptions {
            kind: CrosshairType::Cross,
            dash: Some(StrokeDash {
                segments: vec![1e-8, 1e-8],
                offset: 0.,
            }),
            ..Default::default()
        };
        tiny.validate().map_err(js)?;
        let source = budget_source("tiny", 1, 6.).map_err(js)?;
        let mount_error = match BrowserChart::mount(
            root.clone(),
            source.scene,
            BrowserOptions {
                renderer,
                height: Some(100.),
                legend: false,
                data_controls: false,
                crosshair: Some(tiny),
                interaction_data: source.interaction,
                ..Default::default()
            },
        ) {
            Ok(chart) => {
                chart.stop();
                None
            }
            Err(error) => Some(error.0),
        };
        let unclaimed = !root.has_child_nodes()
            && !root.has_attribute("data-kartis-browser")
            && !root.has_attribute("data-gly-app");
        root.remove();
        reports.push(json!({"renderer":format!("{renderer:?}"),"mountError":mount_error,"unclaimed":unclaimed}));
    }
    let root = document.create_element("div")?;
    root.set_attribute("style", "width:120px")?;
    document.body().unwrap().append_child(&root)?;
    let outcome = (|| -> Result<Value> {
        let source = dashed_budget_source(120., 100.)?;
        let chart = BrowserChart::mount(
            root.clone(),
            source.scene,
            BrowserOptions {
                renderer: BrowserRenderer::Canvas,
                height: Some(100.),
                legend: false,
                data_controls: false,
                tooltip: false,
                interaction_data: source.interaction,
                crosshair: Some(CrosshairOptions {
                    kind: CrosshairType::Cross,
                    dash: Some(StrokeDash {
                        segments: vec![0.01, 0.01],
                        offset: 0.,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )?;
        let errors = Rc::new(RefCell::new(Vec::new()));
        let capture = errors.clone();
        chart.on_event(move |event| {
            if let BrowserEvent::Error(error) = event {
                capture.borrow_mut().push(error.0);
            }
        });
        chart.handle().dispatch(SurfaceInput::Move {
            id: 1,
            point: (60., 50.),
        })?;
        chart.flush()?;
        let before = chart.snapshot()?;
        let dom = root.inner_html();
        let canvas = root
            .query_selector("canvas")
            .map_err(|_| ChartError("dash Canvas lookup".into()))?
            .ok_or_else(|| ChartError("dash Canvas missing".into()))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ChartError("dash Canvas type".into()))?;
        let png = canvas
            .to_data_url()
            .map_err(|_| ChartError("dash PNG export".into()))?;
        let target = dashed_budget_source(500., 100.)?;
        let setter_error = chart.set_layout(target).err().map(|e| e.0);
        let setter_atomic = chart.snapshot()? == before && root.inner_html() == dom;
        chart.on_resize_with_interaction(dashed_budget_source);
        root.set_attribute("style", "width:500px")
            .map_err(|_| ChartError("dash resize root".into()))?;
        chart.flush()?;
        let after = chart.snapshot()?;
        let resize_atomic = after.active
            && after.width == before.width
            && after.height == before.height
            && after.view == before.view
            && after.hovered == before.hovered
            && after.interaction == before.interaction
            && after.pending == 0
            && !after.animating;
        let pixels_unchanged = canvas
            .to_data_url()
            .map_err(|_| ChartError("dash PNG export".into()))?
            == png;
        chart.stop();
        Ok(
            json!({"sharedBudget":true,"setterError":setter_error,"setterAtomic":setter_atomic,"resizeAtomic":resize_atomic,"pixelsUnchanged":pixels_unchanged,"resizeErrors":errors.borrow().clone()}),
        )
    })();
    root.remove();
    reports.push(outcome.map_err(js)?);
    Ok(Value::Array(reports).to_string())
}
#[wasm_bindgen]
pub fn line_crosshair_resize_binding() -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            let phase = f.phase.clone();
            f.chart
                .on_resize_with_interaction(move |w, h| source(w, h, phase.get()).map(Into::into));
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn line_crosshair_stop_callback() {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            f.stop_on_event.set(true);
        }
    });
}
#[wasm_bindgen]
pub fn line_crosshair_hover() -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            let s = f.chart.snapshot().map_err(js)?;
            let layout =
                source_domains(s.width, s.height, f.phase.get(), s.domains, BTreeSet::new())
                    .map_err(js)?;
            let target = layout
                .sites
                .iter()
                .find(|p| p.series_id == "Alpha <safe>" && p.raw_x == 5.)
                .unwrap();
            let p = layout.plot;
            let point = if s.domains.is_some() {
                (target.position[0], target.position[1])
            } else {
                (
                    p.x + (target.position[0] - p.x) * s.view.scale + s.view.translate_x,
                    p.y + (target.position[1] - p.y) * s.view.scale + s.view.translate_y,
                )
            };
            f.chart
                .handle()
                .dispatch(SurfaceInput::Move { id: 1, point })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn line_crosshair_viewport_binding() -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            let s = f.chart.snapshot().map_err(js)?;
            let phase = f.phase.clone();
            let reject = f.reject_viewport.clone();
            let stop = f.stop_viewport.clone();
            let handle = f.chart.handle();
            let controller = DomainViewport::new(
                ViewportAxes {
                    x: ViewportAxis::new([0., 10.], ScaleKind::Linear),
                    left: Some(ViewportAxis::new([0., 10.], ScaleKind::Linear)),
                    right: None,
                },
                ClipRect::plot(&config(s.width, s.height)),
                ViewportLimits {
                    min_zoom: 1.,
                    max_zoom: 20.,
                    bounded: true,
                },
            )
            .map_err(js)?;
            f.chart
                .on_viewport_with_interaction(controller, ViewportSelection::Both, move |request| {
                    if reject.get() {
                        return Err(ChartError("fixture typed viewport rejected".into()));
                    }
                    let candidate = source_domains(
                        request.width,
                        request.height,
                        phase.get(),
                        Some(request.domains),
                        request.hidden,
                    )
                    .map(Into::into);
                    if stop.replace(false) {
                        handle.stop();
                    }
                    candidate
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn line_crosshair_viewport_reject(reject: bool) {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            f.reject_viewport.set(reject);
        }
    });
}
#[wasm_bindgen]
pub fn line_crosshair_viewport_stop(renderer: &str) -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        let entries = slot.borrow();
        let Some(f) = entries.iter().find(|f| f.renderer == renderer) else {
            return Err(js("line viewport renderer missing"));
        };
        f.stop_viewport.set(true);
        f.chart.handle().refresh_viewport().map_err(js)
    })
}
#[wasm_bindgen]
pub fn line_crosshair_clear() -> std::result::Result<(), JsValue> {
    LINE_CROSSHAIRS.with(|slot| {
        for f in slot.borrow().iter() {
            if f.chart.handle().is_active() {
                f.chart.handle().dispatch(SurfaceInput::Clear).map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn line_crosshair_stop() {
    let entries = LINE_CROSSHAIRS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
