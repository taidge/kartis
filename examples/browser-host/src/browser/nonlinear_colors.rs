//! Application-owned nonlinear color consumers; no alternate chart renderer.
use super::*;
#[derive(Clone)]
struct Source {
    published: SceneInteraction,
    sites: Vec<InteractionSite>,
    scale: ColorScale,
    legend: ColorLegendLayout,
}
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    consumer: u8,
    phase: u8,
    accepted: Source,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! { static NONLINEAR_COLORS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())}; }
fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 420.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 20.,
            bottom: 160.,
        },
        ..Default::default()
    }
}
fn profile(phase: u8) -> Result<(ColorScaleOptions, [f64; 3], Vec<f64>)> {
    let (transform, domain, midpoint, diverging, ticks) = match phase {
        0 => (
            ColorScaleTransform::Linear,
            [-11., 17.],
            3.,
            false,
            vec![-11., 0., 3., 10., 17.],
        ),
        1 => (
            ColorScaleTransform::Log,
            [1., 1000.],
            10.,
            false,
            vec![1., 10., 100., 1000.],
        ),
        2 => (
            ColorScaleTransform::Log,
            [-1000., -1.],
            -10.,
            false,
            vec![-1000., -100., -10., -1.],
        ),
        3 => (
            ColorScaleTransform::Pow { exponent: 2. },
            [-4., 9.],
            0.,
            false,
            vec![-4., -1., 0., 1., 4., 9.],
        ),
        4 => (
            ColorScaleTransform::Sqrt,
            [-4., 9.],
            0.,
            false,
            vec![-4., -1., 0., 1., 4., 9.],
        ),
        5 => (
            ColorScaleTransform::SymLog { constant: 2. },
            [-100., 200.],
            0.,
            false,
            vec![-100., -10., 0., 10., 200.],
        ),
        6 => (
            ColorScaleTransform::Log,
            [1., 1000.],
            10.,
            true,
            vec![1., 10., 100., 1000.],
        ),
        7 => (
            ColorScaleTransform::SymLog { constant: 2. },
            [-100., 200.],
            0.,
            true,
            vec![-100., -10., 0., 10., 200.],
        ),
        8 => (
            ColorScaleTransform::Pow { exponent: -1. },
            [1., 100.],
            10.,
            false,
            vec![1., 2., 10., 100.],
        ),
        9 => (
            ColorScaleTransform::Pow { exponent: -1. },
            [1., 100.],
            10.,
            true,
            vec![1., 2., 10., 100.],
        ),
        _ => return Err(ChartError("nonlinear color phase must be 0..=9".into())),
    };
    let colors = if diverging {
        vec![
            Color(10, 30, 80, 255),
            Color(30, 220, 130, 255),
            Color(220, 30, 40, 255),
        ]
    } else {
        vec![Color(5, 30, 90, 255), Color(245, 220, 20, 255)]
    };
    Ok((
        ColorScaleOptions {
            transform,
            domain: Some(domain),
            kind: if diverging {
                ColorScaleKind::Diverging { midpoint }
            } else {
                ColorScaleKind::Sequential
            },
            colors,
            unknown: Color(123, 45, 67, 255),
            ..Default::default()
        },
        [domain[0], midpoint, domain[1]],
        ticks,
    ))
}
fn source(consumer: u8, phase: u8) -> Result<Source> {
    let (options, values, ticks) = profile(phase)?;
    let data = [Some(values[0]), Some(values[1]), Some(values[2]), None];
    let (mut scene, scale, sites, anchors) = match consumer {
        0 => {
            let cells = data
                .iter()
                .enumerate()
                .map(|(i, value)| HeatCell {
                    row: "raw-row".into(),
                    column: format!("raw-{i}"),
                    value: *value,
                })
                .collect::<Vec<_>>();
            let layout = heatmap_layout(
                &config(),
                &cells,
                &HeatmapLayoutOptions {
                    colors: options,
                    axes: HeatmapAxes::none(),
                    labels: false,
                    ..Default::default()
                },
            )?;
            let sites = layout
                .cells
                .iter()
                .map(|c| InteractionSite {
                    mark_id: c.mark_id.clone(),
                    series: c.key.row.clone(),
                    label: format!("HeatMap raw-{}", c.key.column),
                    value: c.value,
                    formatted_value: c.value.map(|v| v.to_string()),
                    color: c.color,
                    position: c.center.unwrap(),
                })
                .collect();
            (layout.scene, layout.color_scale, sites, BTreeMap::new())
        }
        1 => {
            let cells = data
                .iter()
                .enumerate()
                .map(|(i, value)| CalendarCell::new((2024, 1, i as u8 + 1), *value))
                .collect::<Vec<_>>();
            let layout = calendar_layout(
                &config(),
                &cells,
                &CalendarLayoutOptions {
                    colors: options,
                    weekday_ticks: vec![],
                    weekday_space: 0.,
                    month_labels: false,
                    year_labels: Some(false),
                    year_legend: false,
                    day_border: None,
                    month_border: None,
                    ..Default::default()
                },
            )?;
            let sites = layout
                .days
                .iter()
                .map(|d| InteractionSite {
                    mark_id: d.id.clone(),
                    series: layout
                        .scene
                        .marks
                        .iter()
                        .find(|mark| mark.id == d.id)
                        .unwrap()
                        .series
                        .clone(),
                    label: format!("Calendar {}", d.id),
                    value: d.value,
                    formatted_value: d.formatted_value.clone(),
                    color: d.color,
                    position: [d.rect.x + d.rect.width / 2., d.rect.y + d.rect.height / 2.],
                })
                .collect();
            (layout.scene, layout.color_scale, sites, BTreeMap::new())
        }
        2 => {
            let features = data
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    let x = i as f64 * 20. - 40.;
                    GeoFeature {
                        id: format!("raw-region-{i}"),
                        label: format!("Choropleth raw-region-{i}"),
                        value: *value,
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
            let layout = choropleth_with_color_scale(
                &config(),
                &features,
                &GeoMapOptions {
                    fit: GeoFit::Data,
                    border: None,
                    ..Default::default()
                },
                &options,
            )?;
            let mut sites = Vec::new();
            let mut anchors = BTreeMap::new();
            for mark in &layout.scene.marks {
                let feature = features
                    .iter()
                    .find(|f| f.id == mark.series)
                    .ok_or_else(|| ChartError("unknown geographic color series".into()))?;
                let Geometry::Path(commands) = &mark.geometry else {
                    return Err(ChartError("geographic color datum is not a path".into()));
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
                let indices = points.iter().map(|p| p.0).collect::<Vec<_>>();
                anchors.insert(
                    mark.id.clone(),
                    InteractionAnchor::path_points(&mark.geometry, position, &indices)?,
                );
                sites.push(InteractionSite {
                    mark_id: mark.id.clone(),
                    series: feature.id.clone(),
                    label: feature.label.clone(),
                    value: feature.value,
                    formatted_value: feature.value.map(|v| v.to_string()),
                    color: layout.color_scale.map(feature.value),
                    position,
                });
            }
            (layout.scene, layout.color_scale, sites, anchors)
        }
        _ => return Err(ChartError("nonlinear color consumer must be 0..=2".into())),
    };
    let legend = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            id: "nonlinear".into(),
            position: [80., 310.],
            length: 450.,
            thickness: 24.,
            steps: 256,
            tick_values: Some(ticks),
            format: ValueFormat::Auto,
            unknown_label: Some("Missing".into()),
            ..Default::default()
        },
    )?;
    let binding = InteractionBinding {
        sites: sites.clone(),
        anchors,
        ..Default::default()
    };
    let published = SceneInteraction {
        scene,
        interaction: Some(binding),
        plot: Some(ClipRect {
            x: 40.,
            y: 20.,
            width: 560.,
            height: 240.,
        }),
    };
    published.validate()?;
    Ok(Source {
        published,
        sites,
        scale,
        legend,
    })
}
#[wasm_bindgen]
pub fn nonlinear_colors_mount(consumer: u8) -> std::result::Result<(), JsValue> {
    let initial = source(consumer, 0).map_err(js)?;
    nonlinear_colors_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("nonlinear-colors-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
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
                nonlinear_colors_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut rows = capture.borrow_mut();
            if rows.len() == 1024 {
                rows.remove(0);
            }
            rows.push(event_value(event));
        });
        if let Err(error) = chart.flush() {
            chart.stop();
            root.remove();
            nonlinear_colors_stop();
            return Err(js(error));
        }
        NONLINEAR_COLORS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                consumer,
                phase: 0,
                accepted: initial.clone(),
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn nonlinear_colors_phase(phase: u8) -> std::result::Result<(), JsValue> {
    NONLINEAR_COLORS.with(|slot| {
        for fixture in slot.borrow_mut().iter_mut() {
            let next = source(fixture.consumer, phase).map_err(js)?;
            fixture
                .chart
                .set_layout(next.published.clone())
                .map_err(js)?;
            fixture.accepted = next;
            fixture.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn nonlinear_colors_flush() -> std::result::Result<(), JsValue> {
    NONLINEAR_COLORS.with(|slot| {
        for fixture in slot.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
fn rgba(color: Color) -> [u8; 4] {
    [color.0, color.1, color.2, color.3]
}
#[wasm_bindgen]
pub fn nonlinear_colors_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    NONLINEAR_COLORS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let state=f.chart.snapshot().map_err(js)?;let source=&f.accepted;
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"consumer":f.consumer,"phase":f.phase,"transform":format!("{:?}",source.scale.transform()),"domain":source.scale.domain(),"hovered":state.hovered,"selected":state.selected,
            "sites":source.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"label":s.label,"value":s.value,"position":s.position,"color":rgba(s.color)})).collect::<Vec<_>>(),
            "interaction":state.interaction.as_ref().map(|i|json!({"ids":i.ids,"rows":i.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"label":r.label,"color":rgba(r.color)})).collect::<Vec<_>>()})),
            "ticks":source.legend.ticks.iter().map(|t|json!({"value":t.value,"position":t.position,"color":rgba(t.color),"label":t.label})).collect::<Vec<_>>(),"stops":source.legend.stops.iter().map(|s|json!({"offset":s.offset,"value":s.value,"color":rgba(s.color)})).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn nonlinear_colors_rejected() -> bool {
    NONLINEAR_COLORS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok((mut options, ..)) = profile(f.phase) else {
                return false;
            };
            options.transform = ColorScaleTransform::Pow { exponent: f64::NAN };
            if options.resolve([]).is_ok() {
                return false;
            }
            let mut invalid = f.accepted.published.clone();
            invalid.scene.marks[0].opacity = f64::NAN;
            f.chart.set_layout(invalid).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn nonlinear_colors_stop() {
    let fixtures = NONLINEAR_COLORS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in fixtures {
        f.chart.stop();
        f.root.remove();
    }
}
