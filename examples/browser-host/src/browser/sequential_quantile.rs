//! Retained consumers of empirical-rank colors with an exact data-axis legend.
use super::*;
use std::{cell::Cell, collections::BTreeSet, rc::Weak};

#[derive(Clone, Debug, Default)]
struct Settings {
    phase: u8,
    hidden: bool,
    reordered: bool,
    added: bool,
    window: u8,
    unclamped: bool,
    profile: u8,
}
#[derive(Clone)]
struct Row {
    id: String,
    index: usize,
    value: Option<f64>,
}
impl Row {
    fn recent(&self) -> bool {
        self.index >= 3
    }
    fn group(&self) -> &str {
        if self.recent() { "recent" } else { "older" }
    }
    fn column(&self) -> String {
        format!("slot-{}", if self.index == 6 { 3 } else { self.index % 3 })
    }
    fn date(&self) -> (i32, u8, u8) {
        if self.index < 3 {
            (2023, 12, 29 + self.index as u8)
        } else {
            (2024, 1, self.index as u8 - 2)
        }
    }
}
#[derive(Clone)]
struct Source {
    published: SceneInteraction,
    sites: Vec<InteractionSite>,
    site_sources: BTreeMap<String, String>,
    rows: Vec<Row>,
    scale: ColorScale,
    legend: ColorLegendLayout,
    settings: Settings,
}
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    consumer: u8,
    accepted: Rc<RefCell<Source>>,
    publish_errors: Rc<Cell<usize>>,
    pending_resize: Rc<RefCell<Option<Source>>>,
    events: Rc<RefCell<Vec<Value>>>,
    callback: Weak<()>,
}
thread_local! {
    static SEQUENTIAL_QUANTILE:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};
    static RETIRED:RefCell<Vec<(String,BrowserHandle,Weak<()>)>>=const{RefCell::new(Vec::new())};
}
fn checked(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ChartError(message.into()))
    }
}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 24.,
            bottom: 160.,
        },
        ..Default::default()
    }
}
fn options(s: &Settings) -> Result<(ColorScaleOptions, ColorScaleProfile)> {
    checked(
        s.phase <= 5 && s.window <= 2 && s.profile <= 1,
        "invalid observation color settings",
    )?;
    let domain = match s.window {
        0 => None,
        1 => Some([2., 8.]),
        _ => Some([-5., 15.]),
    };
    Ok((
        ColorScaleOptions {
            kind: ColorScaleKind::SequentialQuantile,
            domain,
            clamp: !s.unclamped,
            colors: vec![Color(10, 30, 80, 255), Color(220, 200, 30, 255)],
            unknown: Color(123, 45, 67, 255),
            ..Default::default()
        },
        ColorScaleProfile::Interpolate(if s.profile == 0 {
            ColorInterpolation::Srgb
        } else {
            ColorInterpolation::Lab
        }),
    ))
}
fn rows(s: &Settings) -> Vec<Row> {
    let values = match s.phase {
        0 => [Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None],
        1 => [Some(-20.), Some(0.), Some(0.), Some(1.), Some(100.), None],
        2 => [Some(0.), Some(10.), None, None, None, None],
        3 => [Some(7.), None, None, None, None, None],
        4 => [None; 6],
        _ => [Some(7.), Some(7.), Some(7.), None, None, None],
    };
    let mut rows = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| Row {
            id: format!("sample-{index}"),
            index,
            value,
        })
        .collect::<Vec<_>>();
    if s.added {
        rows.push(Row {
            id: "sample-6".into(),
            index: 6,
            value: Some(2.),
        });
    }
    if s.reordered {
        rows.reverse();
    }
    rows
}
fn source(consumer: u8, s: &Settings, width: f64, height: f64) -> Result<Source> {
    checked(
        width.is_finite()
            && height.is_finite()
            && (400. ..=800.).contains(&width)
            && (360. ..=640.).contains(&height),
        "invalid observation chart dimensions",
    )?;
    checked(
        consumer <= 2,
        "color consumer must be HeatMap, Calendar or Choropleth",
    )?;
    let (colors, profile) = options(s)?;
    let rows = rows(s);
    let mut site_sources = BTreeMap::new();
    let (mut scene, scale, sites, anchors) = match consumer {
        0 => {
            let cells = rows
                .iter()
                .map(|r| HeatCell {
                    row: r.group().into(),
                    column: r.column(),
                    value: r.value,
                })
                .collect::<Vec<_>>();
            let layout = heatmap_layout_with_color_profile(
                &config(width, height),
                &cells,
                &HeatmapLayoutOptions {
                    colors,
                    hidden_rows: if s.hidden {
                        BTreeSet::from(["recent".into()])
                    } else {
                        BTreeSet::new()
                    },
                    axes: HeatmapAxes::none(),
                    labels: false,
                    ..Default::default()
                },
                profile,
            )?;
            let sites = layout
                .cells
                .iter()
                .filter(|c| c.visible)
                .map(|c| {
                    let raw = c.tooltip_datum();
                    let original = rows
                        .iter()
                        .find(|r| r.group() == c.key.row && r.column() == c.key.column);
                    if let Some(row) = original {
                        site_sources.insert(c.mark_id.clone(), row.id.clone());
                    }
                    InteractionSite {
                        mark_id: raw.id.into(),
                        series: raw.series.into(),
                        label: original.map_or_else(
                            || format!("HeatMap missing {} {}", c.key.row, c.key.column),
                            |r| format!("HeatMap {} ({})", r.id, r.group()),
                        ),
                        value: raw.value,
                        formatted_value: c.formatted_value.clone(),
                        color: c.color,
                        position: c.center.expect("visible heat cell center"),
                    }
                })
                .collect::<Vec<_>>();
            (layout.scene, layout.color_scale, sites, BTreeMap::new())
        }
        1 => {
            let data = rows
                .iter()
                .map(|r| CalendarCell::new(r.date(), r.value))
                .collect::<Vec<_>>();
            let layout = calendar_layout_with_color_profile(
                &config(width, height),
                &data,
                &CalendarLayoutOptions {
                    colors,
                    start: Some((2023, 12, 29)),
                    end: Some((2024, 1, 4)),
                    hidden_years: if s.hidden {
                        BTreeSet::from([2024])
                    } else {
                        BTreeSet::new()
                    },
                    weekday_ticks: vec![],
                    weekday_space: 0.,
                    month_labels: false,
                    year_labels: Some(false),
                    year_legend: false,
                    day_border: None,
                    month_border: None,
                    ..Default::default()
                },
                profile,
            )?;
            let sites = layout
                .days
                .iter()
                .filter(|d| d.visible)
                .map(|d| {
                    let raw = d.tooltip_datum();
                    let original = rows.iter().find(|r| r.date() == d.date);
                    if let Some(row) = original {
                        site_sources.insert(d.id.clone(), row.id.clone());
                    }
                    InteractionSite {
                        mark_id: raw.id.into(),
                        series: format!("year:{}", d.year),
                        label: original.map_or_else(
                            || format!("Calendar {} (missing)", d.id),
                            |r| format!("Calendar {} ({})", r.id, d.id),
                        ),
                        value: raw.value,
                        formatted_value: d.formatted_value.clone(),
                        color: d.color,
                        position: [d.rect.x + d.rect.width / 2., d.rect.y + d.rect.height / 2.],
                    }
                })
                .collect::<Vec<_>>();
            (layout.scene, layout.color_scale, sites, BTreeMap::new())
        }
        _ => {
            // This consumer has no hidden-feature option. Explicit source
            // filtering rebuilds its distribution; projection clipping does not.
            let features = rows
                .iter()
                .filter(|r| !(s.hidden && r.recent()))
                .map(|r| {
                    let x = -60. + r.index as f64 * 18.;
                    GeoFeature {
                        id: r.id.clone(),
                        label: format!("Region {}", r.id),
                        value: r.value,
                        polygons: vec![vec![vec![
                            [x, 0.],
                            [x + 12., 0.],
                            [x + 12., 12.],
                            [x, 12.],
                            [x, 0.],
                        ]]],
                    }
                })
                .collect::<Vec<_>>();
            let layout = choropleth_with_color_profile(
                &config(width, height),
                &features,
                &GeoMapOptions {
                    fit: GeoFit::Data,
                    border: None,
                    ..Default::default()
                },
                &colors,
                profile,
            )?;
            let mut sites = Vec::new();
            let mut anchors = BTreeMap::new();
            for mark in &layout.scene.marks {
                // Nullable values come from retained GeoFeature source, never
                // the Scene mark's legacy finite placeholder.
                let raw = features
                    .iter()
                    .find(|f| f.id == mark.series)
                    .ok_or_else(|| ChartError("geographic source identity missing".into()))?;
                let Geometry::Path(commands) = &mark.geometry else {
                    return Err(ChartError("geographic datum must be a path".into()));
                };
                let points = commands
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| match c {
                        PathCommand::Move(p) | PathCommand::Line(p) => Some((i, [p.x, p.y])),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let position = [
                    points.iter().map(|p| p.1[0]).sum::<f64>() / points.len() as f64,
                    points.iter().map(|p| p.1[1]).sum::<f64>() / points.len() as f64,
                ];
                anchors.insert(
                    mark.id.clone(),
                    InteractionAnchor::path_points(
                        &mark.geometry,
                        position,
                        &points.iter().map(|p| p.0).collect::<Vec<_>>(),
                    )?,
                );
                site_sources.insert(mark.id.clone(), raw.id.clone());
                sites.push(InteractionSite {
                    mark_id: mark.id.clone(),
                    series: raw.id.clone(),
                    label: raw.label.clone(),
                    value: raw.value,
                    formatted_value: raw.value.map(|v| v.to_string()),
                    color: layout.color_scale.map(raw.value),
                    position,
                });
            }
            (layout.scene, layout.color_scale, sites, anchors)
        }
    };
    let legend = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            id: "sequential-quantile".into(),
            position: [80., 310.],
            length: width - 190.,
            thickness: 24.,
            steps: 16,
            tick_count: 5,
            title: "Observed value".into(),
            unknown_label: Some("Missing".into()),
            ..Default::default()
        },
    )?;
    let published = SceneInteraction {
        scene,
        interaction: Some(InteractionBinding {
            sites: sites.clone(),
            anchors,
            ..Default::default()
        }),
        plot: Some(ClipRect::plot(&config(width, height))),
    };
    published.validate()?;
    Ok(Source {
        published,
        sites,
        site_sources,
        rows,
        scale,
        legend,
        settings: s.clone(),
    })
}
#[wasm_bindgen]
pub fn sequential_quantile_mount(consumer: u8) -> std::result::Result<(), JsValue> {
    let initial = source(consumer, &Settings::default(), 640., 420.).map_err(js)?;
    sequential_quantile_stop();
    RETIRED.with(|s| s.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("sequential-quantile-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0;background:white",
        )?;
        document
            .get_element_by_id("sequential-quantile-charts")
            .unwrap_or_else(|| document.body().unwrap().into())
            .append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            initial.published.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(420.),
                legend: false,
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                interaction_data: initial.published.interaction.clone(),
                tooltip_options: TooltipOptions {
                    missing: MissingValue::Placeholder("missing".into()),
                    lines: vec![vec![
                        TooltipPart::Label,
                        TooltipPart::Text(": ".into()),
                        TooltipPart::Value,
                    ]],
                    ..Default::default()
                },
                interaction: surface::SurfaceOptions {
                    plot: initial.published.plot,
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                sequential_quantile_stop();
                return Err(js(error));
            }
        };
        if let Err(error) = chart.flush() {
            chart.stop();
            root.remove();
            sequential_quantile_stop();
            return Err(js(error));
        }
        let accepted = Rc::new(RefCell::new(initial.clone()));
        let publish_errors = Rc::new(Cell::new(0usize));
        let pending_resize = Rc::new(RefCell::new(None::<Source>));
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let token = Rc::new(());
        let callback = Rc::downgrade(&token);
        let keeper = token.clone();
        let errors = publish_errors.clone();
        let pending = pending_resize.clone();
        let published = accepted.clone();
        chart.on_event(move |value| {
            let _keep = &keeper;
            match &value {
                BrowserEvent::Error(_) => {
                    errors.set(errors.get().saturating_add(1));
                    pending.borrow_mut().take();
                }
                BrowserEvent::Resized { width, height } => {
                    if let Some(candidate) = pending.borrow_mut().take()
                        && candidate.published.scene.width == *width
                        && candidate.published.scene.height == *height
                    {
                        *published.borrow_mut() = candidate;
                    }
                }
                _ => {}
            }
            let mut log = observed.borrow_mut();
            if log.len() == 512 {
                log.remove(0);
            }
            log.push(event_value(value));
        });
        let published = accepted.clone();
        let pending = pending_resize.clone();
        chart.on_resize_with_interaction(move |width, height| {
            let _keep = &token;
            let settings = published.borrow().settings.clone();
            let next = source(consumer, &settings, width, height)?;
            let output = next.published.clone();
            *pending.borrow_mut() = Some(next);
            Ok(output)
        });
        SEQUENTIAL_QUANTILE.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                consumer,
                accepted,
                publish_errors,
                pending_resize,
                events,
                callback,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn sequential_quantile_update(
    phase: u8,
    hidden: bool,
    reordered: bool,
    added: bool,
    window: u8,
    unclamped: bool,
    profile: u8,
) -> std::result::Result<(), JsValue> {
    let settings = Settings {
        phase,
        hidden,
        reordered,
        added,
        window,
        unclamped,
        profile,
    };
    // Preflight every renderer before enqueueing; publishing is a separate
    // transaction per chart. Earlier successful renderers keep their accepted
    // source if a later renderer fails. No registry borrow crosses callbacks.
    let entries = SEQUENTIAL_QUANTILE.with(|slot| {
        slot.borrow()
            .iter()
            .map(|f| {
                (
                    f.chart.handle(),
                    f.consumer,
                    f.accepted.clone(),
                    f.publish_errors.clone(),
                )
            })
            .collect::<Vec<_>>()
    });
    let candidates = entries
        .iter()
        .map(|(_, consumer, accepted, _)| {
            let previous = accepted.borrow();
            source(
                *consumer,
                &settings,
                previous.published.scene.width,
                previous.published.scene.height,
            )
        })
        .collect::<Result<Vec<_>>>()
        .map_err(js)?;
    for ((chart, _, accepted, errors), next) in entries.iter().zip(candidates) {
        let before = errors.get();
        chart.set_layout(next.published.clone()).map_err(js)?;
        chart.flush().map_err(js)?;
        if errors.get() != before {
            return Err(js(
                "Observation color proposal was not published; accepted frame retained",
            ));
        }
        *accepted.borrow_mut() = next;
    }
    Ok(())
}
#[wasm_bindgen]
pub fn sequential_quantile_flush() -> std::result::Result<(), JsValue> {
    let charts = SEQUENTIAL_QUANTILE.with(|slot| {
        slot.borrow()
            .iter()
            .map(|f| f.chart.handle())
            .collect::<Vec<_>>()
    });
    for chart in charts {
        chart.flush().map_err(js)?;
    }
    Ok(())
}
#[wasm_bindgen]
pub fn sequential_quantile_resize(width: f64) -> std::result::Result<(), JsValue> {
    checked(
        width.is_finite() && (400. ..=800.).contains(&width),
        "invalid observation chart width",
    )
    .map_err(js)?;
    let roots = SEQUENTIAL_QUANTILE.with(|slot| {
        slot.borrow()
            .iter()
            .map(|f| f.root.clone())
            .collect::<Vec<_>>()
    });
    for root in roots {
        root.dyn_ref::<web_sys::HtmlElement>()
            .unwrap()
            .style()
            .set_property("width", &format!("{width}px"))?;
    }
    Ok(())
}
fn rgba(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
#[wasm_bindgen]
pub fn sequential_quantile_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    SEQUENTIAL_QUANTILE.with(|slot|{
        let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(RETIRED.with(|slot|{let old=slot.borrow();let old=old.iter().find(|f|f.0==renderer);json!({"active":false,"retired":old.is_some(),"stopped":old.is_none_or(|f|!f.1.is_active()),"callbacksReleased":old.is_none_or(|f|f.2.upgrade().is_none())}).to_string()}));};
        let state=f.chart.snapshot().map_err(js)?;let source=f.accepted.borrow();let s=&source.settings;
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"publishErrors":f.publish_errors.get(),"pendingResize":f.pending_resize.borrow().is_some(),"acceptedSize":[source.published.scene.width,source.published.scene.height],"consumer":f.consumer,"phase":s.phase,"hidden":s.hidden,"reordered":s.reordered,"added":s.added,"window":s.window,"unclamped":s.unclamped,"profile":s.profile,"domain":source.scale.domain(),"samples":source.scale.sample_domain(),"quantiles":source.scale.quantiles(4).map_err(js)?,"rankColors":source.scale.rank_colors().map(|v|v.into_iter().map(rgba).collect::<Vec<_>>()),"hovered":state.hovered,"selected":state.selected,
            "source":source.rows.iter().map(|r|json!({"id":r.id,"value":r.value,"group":r.group(),"included":!(s.hidden&&r.recent())})).collect::<Vec<_>>(),
            "sites":source.sites.iter().map(|site|json!({"id":site.mark_id,"sourceId":source.site_sources.get(&site.mark_id),"series":site.series,"label":site.label,"value":site.value,"position":site.position,"color":rgba(site.color),"rank":site.value.and_then(|v|source.scale.rank(v))})).collect::<Vec<_>>(),
            "queries":([-100.,0.,2.,5.,8.,10.,100.].into_iter().map(|v|json!({"value":v,"rank":source.scale.rank(v),"color":rgba(source.scale.map(Some(v)))})).collect::<Vec<_>>()),
            "ticks":source.legend.ticks.iter().map(|t|json!({"value":t.value,"position":t.position,"color":rgba(t.color),"label":t.label})).collect::<Vec<_>>(),"stops":source.legend.stops.iter().map(|stop|json!({"offset":stop.offset,"value":stop.value,"color":rgba(stop.color)})).collect::<Vec<_>>(),
            "interaction":state.interaction.map(|i|json!({"ids":i.ids,"rows":i.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"label":r.label,"color":rgba(r.color)})).collect::<Vec<_>>()})),"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn sequential_quantile_rejected(kind: u8) -> std::result::Result<String, JsValue> {
    let mut options = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![
            Color(0, 0, 0, 255),
            Color(255, 0, 0, 255),
            Color(0, 255, 0, 255),
            Color(0, 0, 255, 255),
        ],
        ..Default::default()
    };
    let mut mapping_supported = None;
    let attempt: Result<()> = match kind {
        0 => options.resolve([Some(f64::NAN)]).map(|_| ()),
        1 => {
            options.transform = ColorScaleTransform::Log;
            options.resolve([Some(1.), Some(10.)]).map(|_| ())
        }
        2 => options
            .resolve((0..2048).map(|v| Some(v as f64)))
            .and_then(|scale| {
                mapping_supported = Some(
                    scale
                        .sample_domain()
                        .is_some_and(|samples| samples.len() == 2048)
                        && scale.rank(500.).is_some()
                        && scale.map(Some(500.)) != scale.unknown(),
                );
                scale.stops(16).map(|_| ())
            }),
        3 => {
            options.domain = Some([0., 1.]);
            options
                .resolve([0., 0.5, 0.500000001, 1.].map(Some))
                .and_then(|scale| scale.stops(16).map(|_| ()))
        }
        4 => {
            options.domain = Some([-f64::MAX, f64::MAX]);
            options
                .resolve([0., f64::from_bits(1), 1.].map(Some))
                .and_then(|scale| scale.stops(16).map(|_| ()))
        }
        5 => SEQUENTIAL_QUANTILE.with(|slot| {
            for f in slot.borrow().iter() {
                let mut invalid = f.accepted.borrow().published.clone();
                invalid.interaction.as_mut().unwrap().sites[0].value = Some(f64::NAN);
                f.chart.set_layout(invalid)?;
            }
            Ok(())
        }),
        6 => options
            .resolve([Some(-f64::MAX), Some(f64::MAX)])
            .and_then(|scale| scale.quantiles(2).map(|_| ())),
        _ => return Err(js("invalid rejected-proposal profile")),
    };
    Ok(match attempt {
        Err(error) => json!({"rejected":true,"kind":kind,"message":error.0,"mappingSupported":mapping_supported}),
        Ok(()) => json!({"rejected":false,"kind":kind,"mappingSupported":mapping_supported}),
    }
    .to_string())
}
#[wasm_bindgen]
pub fn sequential_quantile_stop() {
    let old = SEQUENTIAL_QUANTILE.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in old {
        let handle = f.chart.handle();
        f.chart.stop();
        f.root.remove();
        RETIRED.with(|slot| slot.borrow_mut().push((f.renderer, handle, f.callback)));
    }
}
