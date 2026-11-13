//! Actual typed ScatterLayout with shared domain navigation and native input.
use std::collections::BTreeSet;

use kartis::surface::{Picking, SurfaceModel, SurfaceOptions};

use super::*;

struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: &'static str,
    case: u8,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<ScatterLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
    stop_on_hover: Rc<Cell<bool>>,
    reject_layout: Rc<Cell<bool>>,
}
thread_local! {static SCATTERS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}

fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Typed scatter <safe>".into(),
        margin: Margin {
            left: 60.25,
            right: 44.75,
            top: 40.25,
            bottom: 59.75,
        },
        ..Default::default()
    }
}
fn key(key: &CategoryKey) -> Value {
    match key {
        CategoryKey::String(v) => json!({"type":"string","value":v}),
        CategoryKey::Number(v) => json!({"type":"number","value":v}),
    }
}
fn numeric(min: f64, max: f64, phase: u8, utc: bool) -> ScatterAxisScale {
    ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions {
            kind: if utc {
                ScaleKind::Utc
            } else {
                ScaleKind::Linear
            },
            nice: ScaleNice::Off,
            reverse: phase == 1,
            round: phase == 1 && !utc,
            ..Default::default()
        },
        min: ScatterBound::Fixed(min),
        max: ScatterBound::Fixed(max),
    }
}
fn datum(id: &str, x: CategoryKey, y: CategoryKey, size: Option<f64>) -> ScatterDatum {
    let mut node = ScatterDatum::new(id, x, y);
    node.size_value = size;
    node.label = Some(format!("{} <safe>", id));
    node
}
fn proposal(
    case: u8,
    phase: u8,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
    scales: Option<ScatterScaleOptions>,
) -> Result<ScatterLayout> {
    if case > 6 || phase > 1 {
        return Err(ChartError("scatter case/phase out of range".into()));
    }
    let p = f64::from(phase);
    let mut options = ScatterLayoutOptions {
        hidden,
        scales: ScatterScaleOptions {
            x: numeric(0., 10., if case == 0 { phase } else { 0 }, false),
            y: numeric(0., 10., if case == 0 { phase } else { 0 }, false),
        },
        ..Default::default()
    };
    options.axes.top = Some(Default::default());
    options.axes.right = Some(Default::default());
    options.labels.enabled = true;
    options.labels.text = ScatterLabelText::Coordinates;
    options.labels.offset = [0., -18.];
    options.labels.size = 10.;
    options.labels.clip_to_plot = true;
    options.x_format = ValueFormat::Fixed(2);
    options.y_format = ValueFormat::Fixed(2);
    let mut frame = config(width, height);
    let (mut alpha, mut beta) = if case == 4 {
        const DAY: f64 = 86_400_000.;
        let x = 1_709_078_400_000.;
        let y = 1_717_200_000_000.;
        options.scales = ScatterScaleOptions {
            x: numeric(x, x + 4. * DAY, phase, true),
            y: numeric(y, y + 4. * DAY, phase, true),
        };
        let locale = TimeLocale::default();
        let datum_format = ValueFormat::date_time("%m-%d %H:%M", &locale, TimeZone::Utc)?;
        let axis_format = ValueFormat::date_time("%m-%d", &locale, TimeZone::Utc)?;
        let daily_ticks = TimeCalendar {
            interval: Some(TimeInterval::new(TimeUnit::Day, 1)?),
            ..TimeCalendar::new(TimeZone::Utc)
        };
        options.x_format = datum_format.clone();
        options.y_format = datum_format;
        frame.title = "UTC scatter · 2024".into();
        frame.margin.left = 94.25;
        frame.margin.right = 89.75;
        for axis in [
            &mut options.axes.top,
            &mut options.axes.right,
            &mut options.axes.bottom,
            &mut options.axes.left,
        ]
        .into_iter()
        .flatten()
        {
            axis.format = axis_format.clone();
            axis.tick_calendar = Some(daily_ticks.clone());
            axis.tick_count = 4;
        }
        (
            vec![
                datum(
                    "interior",
                    (x + (2. + p) * DAY).into(),
                    (y + (1. + 2. * p) * DAY).into(),
                    None,
                ),
                datum("shared", (x + DAY).into(), (y + 3. * DAY).into(), None),
                datum("edge", x.into(), y.into(), None),
            ],
            vec![
                datum("shared", (x + 3. * DAY).into(), (y + 2. * DAY).into(), None),
                datum(
                    "other",
                    (x + 2.5 * DAY).into(),
                    (y + 3.5 * DAY).into(),
                    None,
                ),
            ],
        )
    } else if case == 5 {
        options.scales = ScatterScaleOptions {
            x: ScatterAxisScale::Point {
                options: PointScaleOptions {
                    padding: 0.35,
                    align: 0.7,
                    round: phase == 1,
                    reverse: phase == 1,
                },
                domain: Some(vec![
                    "1".into(),
                    1_f64.into(),
                    "North".into(),
                    "South".into(),
                ]),
                window: None,
            },
            y: ScatterAxisScale::Point {
                options: PointScaleOptions {
                    padding: 0.5,
                    align: 0.3,
                    round: phase == 1,
                    reverse: phase == 1,
                },
                domain: Some(vec!["1".into(), 1_f64.into(), "Low".into(), "High".into()]),
                window: None,
            },
        };
        options.x_format = ValueFormat::Auto;
        options.y_format = ValueFormat::Auto;
        (
            vec![
                datum(
                    "interior",
                    1_f64.into(),
                    if phase == 0 {
                        1_f64.into()
                    } else {
                        "Low".into()
                    },
                    None,
                ),
                datum("shared", "North".into(), "Low".into(), None),
                datum("edge", "South".into(), "High".into(), None),
                datum("omitted", "Outside".into(), "Low".into(), None),
            ],
            vec![
                datum("shared", "1".into(), "1".into(), None),
                datum("other", "North".into(), 1_f64.into(), None),
            ],
        )
    } else if case == 1 {
        options.node.shape = ScatterNodeShape::Circle {
            diameter: ScatterSize::Linear {
                values: [0., 100.],
                sizes: [0., 30.],
                clamp: false,
                missing: ScatterMissingSize::Fixed(9.),
            },
        };
        // Keep this unclamped. The center is outside the plot while actual
        // positive-radius ink extends across its clip into the visible plot.
        options.scales = ScatterScaleOptions {
            x: numeric(0., 10., 0, false),
            y: numeric(0., 10., 0, false),
        };
        (
            vec![
                datum(
                    "interior",
                    (3. + p).into(),
                    (5. + p).into(),
                    Some(50. + 30. * p),
                ),
                datum("zero", 6_f64.into(), 6_f64.into(), Some(0.)),
                datum("partial", (-0.04_f64).into(), 8_f64.into(), Some(100.)),
            ],
            vec![
                datum("shared", 7_f64.into(), 3_f64.into(), Some(150. + 20. * p)),
                datum("other", 8_f64.into(), 8_f64.into(), None),
            ],
        )
    } else {
        (
            vec![
                datum("interior", (3. + p).into(), (5. + 2. * p).into(), None),
                datum("shared", 2_f64.into(), 8_f64.into(), None),
                datum("edge", 0_f64.into(), 0_f64.into(), None),
            ],
            vec![
                datum("shared", 7_f64.into(), 3_f64.into(), None),
                datum("other", 8_f64.into(), 8_f64.into(), None),
            ],
        )
    };
    if case == 2 {
        options.node.shape = ScatterNodeShape::FilledGlyph {
            shape: if phase == 0 {
                FilledGlyph::Star
            } else {
                FilledGlyph::Wye
            },
            area: ScatterSize::Fixed(if phase == 0 { 900. } else { 1600. }),
            rotation: 0.31 + 0.6 * p,
        };
    }
    if case == 3 {
        options.node.shape = ScatterNodeShape::StrokedGlyph {
            shape: StrokedGlyph::Triangle,
            size: ScatterSize::Fixed(if phase == 0 { 900. } else { 1600. }),
            rotation: 0.31 + 0.6 * p,
        };
        options.node.stroke = Some(GraphicStroke {
            color: Color(30, 80, 175, 255),
            width: 2.,
            cap: StrokeCap::Round,
        });
    }
    if let Some(scales) = scales {
        options.scales = scales;
    }
    if phase == 1 {
        alpha.reverse();
        beta.reverse();
    }
    let mut a = ScatterSeries::new("alpha", alpha);
    a.label = "Alpha <safe>".into();
    a.color = Some(Color(25, 80, 180, 255));
    let mut b = ScatterSeries::new("beta", beta);
    b.label = "Beta".into();
    b.color = Some(Color(220, 110, 30, 255));
    let data = if phase == 0 { vec![a, b] } else { vec![b, a] };
    if case == 6 {
        options.node.shape = ScatterNodeShape::Circle {
            diameter: ScatterSize::Fixed(24. + 6. * p),
        };
        return scatter_layout_with_node_factory(&frame, &data, &options, custom_node);
    }
    scatter_layout(&frame, &data, &options)
}

fn custom_node(node: &ScatterNodeContext<'_>) -> Result<ScatterNodeGeometry> {
    let [x, y] = node.position;
    let r = node.size / 2.;
    let p = |dx: f64, dy: f64| GraphicPoint::new(x + dx * r, y + dy * r);
    if node.series_id == "alpha" {
        let geometry = Geometry::Path(vec![
            PathCommand::Move(p(0.95, -1.05)),
            PathCommand::Line(p(3., -0.95)),
            PathCommand::Line(p(2.6, 1.)),
            PathCommand::Line(p(1.05, 1.1)),
            PathCommand::Close,
        ]);
        let mut shape = ScatterNodeGeometry::inherit(geometry, node);
        shape.fill = None;
        shape.stroke = Some(GraphicStroke {
            color: Color(24, 85, 162, 255),
            width: 3.,
            cap: StrokeCap::Round,
        });
        Ok(shape)
    } else {
        let geometry = Geometry::Path(vec![
            PathCommand::Move(p(-1.45, -0.9)),
            PathCommand::Line(p(0.95, -0.9)),
            PathCommand::Line(p(1.55, 0.)),
            PathCommand::Line(p(0.95, 0.9)),
            PathCommand::Line(p(-1.45, 0.9)),
            PathCommand::Line(p(-0.78, 0.)),
            PathCommand::Close,
        ]);
        Ok(ScatterNodeGeometry::inherit(geometry, node))
    }
}

fn bind(f: &Fixture) -> Result<()> {
    let snap = f.chart.snapshot()?;
    let base = proposal(
        f.case,
        f.phase.get(),
        snap.width,
        snap.height,
        snap.hidden,
        None,
    )?;
    let controller = base.viewport_controller(ViewportLimits {
        min_zoom: 1.,
        max_zoom: 8.,
        bounded: true,
    })?;
    let phase = f.phase.clone();
    let accepted = f.accepted.clone();
    let reject = f.reject_layout.clone();
    let case = f.case;
    f.chart
        .on_viewport_with_interaction(controller, ViewportSelection::Both, move |request| {
            if reject.get() {
                return Err(ChartError("scatter fixture rejects viewport layout".into()));
            }
            let scales = base.viewport_scale_options(request.domains)?;
            let next = proposal(
                case,
                phase.get(),
                request.width,
                request.height,
                request.hidden,
                Some(scales),
            )?;
            let published = next.clone().into();
            *accepted.borrow_mut() = next;
            Ok(published)
        })
}
fn mount(case: u8, nearest: bool) -> std::result::Result<(), JsValue> {
    let initial = proposal(case, 0, 640., 400., BTreeSet::new(), None).map_err(js)?;
    scatter_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, mode) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("scatter-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene.clone(),
            BrowserOptions {
                renderer: mode,
                height: Some(400.),
                motion: browser::MotionPreference::Animate,
                highlight: browser::BrowserHighlight::Controlled,
                tooltip_options: TooltipOptions {
                    lines: vec![vec![TooltipPart::Label]],
                    missing: MissingValue::Placeholder(String::new()),
                    ..Default::default()
                },
                interaction_data: Some(initial.interaction_binding()),
                interaction: SurfaceOptions {
                    plot: Some(initial.plot),
                    picking: if nearest {
                        Picking::Nearest { distance: 6. }
                    } else {
                        Picking::Geometry
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let accepted = Rc::new(RefCell::new(initial.clone()));
        let phase = Rc::new(Cell::new(0));
        let events = Rc::new(RefCell::new(Vec::new()));
        let stop_on_hover = Rc::new(Cell::new(false));
        let reject_layout = Rc::new(Cell::new(false));
        let capture = events.clone();
        let layout = accepted.clone();
        let stop = stop_on_hover.clone();
        let handle = chart.handle();
        chart.on_event(move|event|{
            let should_stop=matches!(&event,BrowserEvent::Interaction(SurfaceEvent::Hover(ids))if !ids.is_empty())&&stop.replace(false);
            let value=match &event{BrowserEvent::Interaction(SurfaceEvent::Hover(ids))=>json!({"type":"hover","ids":ids}),BrowserEvent::Interaction(SurfaceEvent::Activated(id))=>json!({"type":"activated","id":id}),BrowserEvent::Interaction(SurfaceEvent::Brush{bounds,ids,complete})=>json!({"type":"brush","bounds":[bounds.x0,bounds.y0,bounds.x1,bounds.y1],"ids":ids,"complete":complete}),BrowserEvent::Error(error)=>json!({"type":"error","message":error.0}),_=>json!({"type":"other"})};
            let mut log=capture.borrow_mut();if log.len()==128{log.remove(0);}log.push(value);drop(log);
            if let BrowserEvent::Interaction(SurfaceEvent::Brush{bounds,complete:true,..})=event{let plot=layout.borrow().plot;let local=Brush{x0:bounds.x0-plot.x,y0:bounds.y0-plot.y,x1:bounds.x1-plot.x,y1:bounds.y1-plot.y};if let Err(error)=handle.request_viewport(ViewportRequest::Brush{bounds:local,axes:ViewportSelection::Both}){capture.borrow_mut().push(json!({"type":"brush-error","message":error.0}));}}
            if should_stop{handle.stop();}
        });
        let f = Fixture {
            chart,
            root,
            renderer,
            case,
            phase,
            accepted,
            events,
            stop_on_hover,
            reject_layout,
        };
        bind(&f).map_err(js)?;
        f.chart.flush().map_err(js)?;
        SCATTERS.with(|slot| slot.borrow_mut().push(f));
    }
    Ok(())
}
/// Actual typed zero-circle cap paints, including boundary/dotted/all-zero phases.
#[wasm_bindgen]
pub fn scatter_zero_caps(mode: u8) -> std::result::Result<(), JsValue> {
    if mode > 11 {
        return Err(js(ChartError("zero cap mode out of range".into())));
    }
    let translucent = matches!(mode, 1 | 3 | 5 | 7);
    let (segments, offset) = match mode {
        0 | 1 => (vec![], 0.),
        2 | 3 => (vec![3., 3.], 1.),
        4 | 5 => (vec![3., 3.], 4.),
        6 | 7 => (vec![3., 3.], 3.),
        8 => (vec![0., 3.], 0.),
        9 => (vec![0., 3.], 1.),
        10 => (vec![0., 0.], 1.),
        _ => (vec![0., 3.], 3.),
    };
    let mut options = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: numeric(0., 512., 0, false),
            y: numeric(0., 256., 0, false),
        },
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        ..Default::default()
    };
    let mut data = Vec::new();
    for ((name, cap), x) in [
        ("butt", StrokeCap::Butt),
        ("round", StrokeCap::Round),
        ("square", StrokeCap::Square),
    ]
    .into_iter()
    .zip([64., 192., 384.])
    {
        let style = ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Fixed(0.),
            },
            fill: None,
            stroke: Some(GraphicStroke {
                color: Color(255, 0, 0, if translucent { 128 } else { 255 }),
                width: 100.,
                cap,
            }),
            opacity: if translucent { 0.5 } else { 1. },
            ..Default::default()
        };
        options.series_styles.insert(name.into(), style);
        data.push(ScatterSeries::new(
            name,
            vec![ScatterDatum::new("zero", x, 128.)],
        ));
    }
    let config = ChartConfig {
        width: 512.,
        height: 256.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    };
    let mut initial = scatter_layout(&config, &data, &options).map_err(js)?;
    // Dash is shared mark paint metadata applied to actual typed Scatter glyphs.
    for site in &initial.sites {
        initial.scene.marks[site.mark_index].stroke_dash = Some(StrokeDash {
            segments: segments.clone(),
            offset,
        });
    }
    initial.scene.validate().map_err(js)?;
    scatter_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, mode_renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("scatter-{renderer}"));
        root.set_attribute("style", "width:512px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene.clone(),
            BrowserOptions {
                renderer: mode_renderer,
                height: Some(256.),
                legend: false,
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(initial.interaction_binding()),
                interaction: SurfaceOptions {
                    plot: Some(initial.plot),
                    picking: Picking::Geometry,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        chart.flush().map_err(js)?;
        SCATTERS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer,
                case: 6,
                phase: Rc::new(Cell::new(mode)),
                accepted: Rc::new(RefCell::new(initial.clone())),
                events: Rc::new(RefCell::new(Vec::new())),
                stop_on_hover: Rc::new(Cell::new(false)),
                reject_layout: Rc::new(Cell::new(false)),
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn scatter_mount(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, false)
}
#[wasm_bindgen]
pub fn scatter_nearest(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, true)
}
#[wasm_bindgen]
pub fn scatter_exact(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, false)
}
#[wasm_bindgen]
pub fn scatter_phase(phase: u8) -> std::result::Result<(), JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            let snap = f.chart.snapshot().map_err(js)?;
            proposal(f.case, phase, snap.width, snap.height, snap.hidden, None).map_err(js)?;
            if f.phase.replace(phase) != phase {
                bind(f).map_err(js)?;
            } else {
                f.chart.handle().refresh_viewport().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_animate(phase: u8) -> std::result::Result<(), JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            let snap = f.chart.snapshot().map_err(js)?;
            let next =
                proposal(f.case, phase, snap.width, snap.height, snap.hidden, None).map_err(js)?;
            f.chart
                .animate_to_with_interaction(next.scene.clone(), next.interaction_binding())
                .map_err(js)?;
            f.phase.set(phase);
            *f.accepted.borrow_mut() = next;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_flush() -> std::result::Result<(), JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            if f.chart.handle().is_active() {
                f.chart.flush().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_clear() -> std::result::Result<(), JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            if f.chart.handle().is_active() {
                f.chart.handle().dispatch(SurfaceInput::Clear).map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_reset() -> std::result::Result<(), JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart
                .handle()
                .request_viewport(ViewportRequest::Reset)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_first() -> std::result::Result<(), JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.handle().dispatch(SurfaceInput::First).map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_reject_viewport(reject: bool) {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            f.reject_layout.set(reject);
        }
    })
}
#[wasm_bindgen]
pub fn scatter_stop_callback() {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            f.stop_on_hover.set(true);
        }
    })
}
#[wasm_bindgen]
pub fn scatter_stop() {
    let entries = SCATTERS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}

fn axis_json(axis: &ResolvedScatterAxis, options: &ScatterAxisScale) -> Value {
    match options {
        ScatterAxisScale::Piecewise { options } => {
            let scale = axis.chart_scale().unwrap();
            json!({"kind":"Piecewise","transform":format!("{:?}",options.continuous.kind),
                "source":options.domain,"display":scale.domain_stops(),"range":scale.range_stops(),
                "round":options.continuous.round,"clamp":options.continuous.clamp,
                "reverse":options.continuous.reverse})
        }
        ScatterAxisScale::Continuous { options, .. } => {
            json!({"kind":format!("{:?}",options.kind),"source":axis.source_domain(),"display":axis.display_domain(),"range":axis.range(),"round":options.round,"clamp":options.clamp,"reverse":options.reverse})
        }
        ScatterAxisScale::Point { options, .. } => {
            json!({"kind":"Point","domain":axis.category_domain().unwrap().iter().map(key).collect::<Vec<_>>(),"range":axis.range(),"window":axis.window(),"padding":options.padding,"align":options.align,"round":options.round,"reverse":options.reverse})
        }
    }
}
fn geometry_json(geometry: &Geometry) -> Value {
    match geometry {
        Geometry::Circle { x, y, radius } => json!({"kind":"circle","x":x,"y":y,"radius":radius}),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => json!({"kind":"rect","x":x,"y":y,"width":width,"height":height}),
        Geometry::Path(commands) => {
            json!({"kind":"path","commands":commands.iter().map(|c|match c{PathCommand::Move(p)=>json!([0,p.x,p.y]),PathCommand::Line(p)=>json!([1,p.x,p.y]),PathCommand::Cubic{first,second,end}=>json!([2,first.x,first.y,second.x,second.y,end.x,end.y]),PathCommand::Close=>json!([3])}).collect::<Vec<_>>()})
        }
    }
}
fn site_json(layout: &ScatterLayout, site: &ScatterSite) -> Value {
    let b = site.bounds;
    let mark = &layout.scene.marks[site.mark_index];
    json!({"id":site.mark_id,"node":site.node_id,"series":site.series_id,"label":mark.label,"rawX":key(&site.raw_x),"rawY":key(&site.raw_y),"formattedX":site.formatted_x,"formattedY":site.formatted_y,"sizeValue":site.size_value,"size":site.node_size,"unit":format!("{:?}",site.size_unit),"position":site.position,"bounds":[b.x,b.y,b.width,b.height],"geometry":geometry_json(&mark.geometry),"labelIndex":site.label_index,"color":[site.color.0,site.color.1,site.color.2,site.color.3]})
}
fn probe(layout: &ScatterLayout, id: &str, hit: bool) -> Result<Option<[f64; 2]>> {
    let Some(site) = layout.sites.iter().find(|s| s.mark_id == id) else {
        return Ok(None);
    };
    let mut model = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(layout.plot),
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )?;
    let mut valid = |p: [f64; 2]| -> Result<bool> {
        for dx in [-1., 0., 1.] {
            for dy in [-1., 0., 1.] {
                model.apply(SurfaceInput::Move {
                    id: 1,
                    point: (p[0] + dx, p[1] + dy),
                })?;
                if model.hovered().iter().any(|m| m == id) != hit {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    };
    if valid(site.position)? {
        return Ok(Some(site.position));
    }
    let b = site.bounds;
    for y in 2..19 {
        for x in 2..19 {
            let p = [
                b.x + b.width * x as f64 / 20.,
                b.y + b.height * y as f64 / 20.,
            ];
            if layout.plot.contains(p[0], p[1]) && valid(p)? {
                return Ok(Some(p));
            }
        }
    }
    if !hit {
        for p in [
            [b.x - 3., b.y - 3.],
            [b.x + b.width + 3., b.y + b.height + 3.],
        ] {
            if layout.plot.contains(p[0], p[1]) && valid(p)? {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
}
#[wasm_bindgen]
pub fn scatter_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    SCATTERS.with(|slot|{
    let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};if !f.chart.handle().is_active(){return Ok(json!({"active":false}).to_string());}
    let snap=f.chart.snapshot().map_err(js)?;let layout=f.accepted.borrow();let target=layout.sites.iter().find(|s|s.series_id=="alpha"&&s.node_id=="interior");
    let target=target.map(|site|{let mut value=site_json(&layout,site);value["hit"]=json!(probe(&layout,&site.mark_id,true).map_err(js)?);value["miss"]=json!(probe(&layout,&site.mark_id,false).map_err(js)?);Ok::<_,JsValue>(value)}).transpose()?;
    let current=snap.interaction.map(|active|json!({"anchor":active.anchor,"ids":active.ids,"rows":active.rows.iter().map(|row|{let site=layout.sites.iter().find(|s|s.mark_id==row.mark_id);json!({"id":row.mark_id,"value":row.value,"position":row.position,"label":row.label,"rawX":site.map(|s|key(&s.raw_x)),"rawY":site.map(|s|key(&s.raw_y)),"formattedX":site.map(|s|&s.formatted_x),"formattedY":site.map(|s|&s.formatted_y)})}).collect::<Vec<_>>()}));
    let p=layout.plot;Ok(json!({"active":snap.active,"case":f.case,"phase":f.phase.get(),"width":snap.width,"height":snap.height,"revision":snap.revision,"pending":snap.pending,"animating":snap.animating,"selected":snap.selected,"hovered":snap.hovered,"hidden":snap.hidden,"view":[snap.view.scale,snap.view.translate_x,snap.view.translate_y],"domains":snap.domains.map(|d|json!({"x":d.x,"y":d.left})),"plot":[p.x,p.y,p.width,p.height],"xScale":axis_json(&layout.x_scale,&layout.scale_options.x),"yScale":axis_json(&layout.y_scale,&layout.scale_options.y),"target":target,"sites":layout.sites.iter().map(|s|site_json(&layout,s)).collect::<Vec<_>>(),"metadata":layout.metadata.iter().map(|m|json!({"id":m.mark_id,"node":m.node_id,"series":m.series_id,"hidden":m.hidden,"position":m.position,"size":m.node_size})).collect::<Vec<_>>(),"owned":layout.scene.label_series.values().collect::<Vec<_>>(),"labels":layout.scene.labels.iter().enumerate().map(|(i,l)|json!({"index":i,"text":l.text,"x":l.x,"y":l.y,"owner":layout.scene.label_series.get(&i)})).collect::<Vec<_>>(),"current":current,"events":f.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn scatter_rejected() -> std::result::Result<bool, JsValue> {
    SCATTERS.with(|slot| {
        for f in slot.borrow().iter() {
            let before = f.chart.snapshot().map_err(js)?;
            let layout = f.accepted.borrow();
            let dom = f.root.inner_html();
            let mut binding = layout.interaction_binding();
            binding.sites[0].position[0] += 1.;
            if f.chart
                .set_scene_with_interaction(layout.scene.clone(), binding)
                .is_ok()
            {
                return Ok(false);
            }
            let mut invalid = layout.scene.clone();
            invalid.width = f64::NAN;
            if f.chart.set_scene(invalid).is_ok() {
                return Ok(false);
            }
            if f.chart.snapshot().map_err(js)? != before || dom != f.root.inner_html() {
                return Ok(false);
            }
            let bad = [ScatterSeries::new(
                "duplicate",
                vec![
                    ScatterDatum::new("same", 1., 2.),
                    ScatterDatum::new("same", 3., 4.),
                ],
            )];
            if scatter_layout(&config(640., 400.), &bad, &Default::default()).is_ok() {
                return Ok(false);
            }
        }
        Ok(true)
    })
}
