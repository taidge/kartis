//! Independent configured parallel-coordinate controllers, sharing no Entries.
use super::*;
struct ParallelFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    vertical: bool,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static PARALLEL:RefCell<Vec<ParallelFixture>>=const{RefCell::new(Vec::new())};}
fn source(vertical: bool, phase: u8) -> Result<ParallelCoordinatesLayout> {
    if phase > 1 {
        return Err(ChartError("parallel fixture phase must be 0 or 1".into()));
    }
    let config = ChartConfig {
        width: 640.,
        height: 360.,
        margin: Margin {
            left: 60.,
            right: 70.,
            top: 30.,
            bottom: 50.,
        },
        title: "Configured parallel coordinates".into(),
        ..Default::default()
    };
    let mut dimensions = ["cost", "speed", "balance", "quality"]
        .map(ParallelDimension::new)
        .to_vec();
    for d in &mut dimensions {
        d.domain = Some([0., 100.]);
        d.axis.tick_values = Some(vec![0., 50., 100.]);
        d.style.caption = ParallelTextStyle {
            color: Some(Color(20, 70, 140, 255)),
            size: Some(17.),
        };
        d.style.caption_offset = 26.;
    }
    dimensions[0].label = Some("Cost".into());
    dimensions[0].value_format = Some(ValueFormat::Affix {
        prefix: "$".into(),
        suffix: String::new(),
        precision: 1,
    });
    dimensions[0].axis.format = ValueFormat::Fixed(0);
    dimensions[1].label = Some("Speed".into());
    dimensions[1].domain = Some([1., 1000.]);
    dimensions[1].scale = ScaleKind::Log;
    dimensions[1].axis.tick_values = Some(vec![1., 10., 100., 1000.]);
    dimensions[1].ticks_position = ParallelTicksPosition::Before;
    dimensions[2].label = Some("Balance".into());
    dimensions[2].domain = Some([-100., 100.]);
    dimensions[2].scale = ScaleKind::SymLog { constant: 1. };
    dimensions[2].axis.tick_values = Some(vec![-100., 0., 100.]);
    dimensions[2].style.caption_rotation = if vertical { 0. } else { -15. };
    dimensions[3].label = Some("Quality".into());
    let epoch = 1_709_078_400_000.;
    let day = 86_400_000.;
    let date = |offset| epoch + day * offset;
    dimensions[3].axis.format = ValueFormat::UtcDate;
    dimensions[3].axis.tick_values = Some(vec![date(0.), date(50.), date(100.)]);
    let shift = f64::from(phase) * 5.;
    let mut a = ParallelDatum::new(
        "row-a",
        vec![
            Some(25. + shift),
            Some(10. + shift * 2.),
            Some(shift * 2.),
            Some(date(75. - shift)),
        ],
    );
    a.label = "Sample <A>".into();
    a.group = Some("alpha".into());
    let mut b = ParallelDatum::new(
        "row-b",
        vec![Some(75.), Some(100.), Some(30.), Some(date(25.))],
    );
    b.label = "Sample B".into();
    b.group = Some("beta".into());
    let mut missing = ParallelDatum::new("row-gap", vec![Some(10.), None, Some(80.), None]);
    missing.label = "Missing dimensions".into();
    missing.group = Some("beta".into());
    parallel_layout_with_scale_options(
        &config,
        &dimensions,
        &[a, b, missing],
        &ParallelCoordinatesOptions {
            layout: if vertical {
                ParallelLayout::Vertical
            } else {
                ParallelLayout::Horizontal
            },
            curve: Curve::Natural,
            line: ParallelLineStyle {
                color: Some(Color(40, 100, 180, 255)),
                width: 4.,
                opacity: 0.8,
                cap: StrokeCap::Round,
                dash: Some(StrokeDash {
                    segments: vec![12., 7.],
                    offset: 4.,
                }),
            },
            series_styles: BTreeMap::from([(
                "beta".into(),
                ParallelLineStyle {
                    color: Some(Color(200, 80, 50, 255)),
                    width: 2.,
                    opacity: 0.6,
                    ..Default::default()
                },
            )]),
            legend: true,
            legend_labels: BTreeMap::from([
                ("alpha".into(), "Alpha group".into()),
                ("beta".into(), "Beta group".into()),
            ]),
            row_labels: true,
            clip_row_labels: true,
            row_label_style: ParallelTextStyle {
                color: Some(Color(20, 70, 140, 255)),
                size: Some(16.),
            },
            row_label_offset: if vertical { [10., 20.] } else { [-90., -15.] },
            ..Default::default()
        },
        &BTreeMap::from([
            (
                "cost".into(),
                ParallelScaleOptions {
                    bounds: [Some(0.), None],
                    scale: ContinuousScaleOptions {
                        round: true,
                        ..Default::default()
                    },
                },
            ),
            (
                "quality".into(),
                ParallelScaleOptions {
                    bounds: [Some(date(0.)), Some(date(100.))],
                    scale: ContinuousScaleOptions {
                        kind: ScaleKind::Utc,
                        reverse: true,
                        ..Default::default()
                    },
                },
            ),
        ]),
    )
}
fn probe(scene: &Scene, vertical: bool) -> Result<[f64; 2]> {
    let mark = scene
        .marks
        .iter()
        .find(|m| m.id == "parallel:5:row-a")
        .ok_or_else(|| ChartError("parallel row missing".into()))?;
    let detailed = stroke_dash::dash_geometry_detailed(
        &mark.geometry,
        mark.stroke_dash.as_ref().unwrap(),
        &StrokeDashLimits::default(),
    )?;
    let mut previous = None;
    for command in detailed.stroke_geometry().commands() {
        match command {
            PathCommand::Move(p) => previous = Some(p),
            PathCommand::Line(p) => {
                if let Some(a) = previous {
                    let mid = [(a.x + p.x) / 2., (a.y + p.y) / 2.];
                    let independent = if vertical { mid[1] } else { mid[0] };
                    let axes = if vertical {
                        [310., 310. - 280. / 3., 310. - 560. / 3., 30.]
                    } else {
                        [60., 230., 400., 570.]
                    };
                    if axes
                        .into_iter()
                        .all(|axis| (axis - independent).abs() > 12.)
                    {
                        return Ok(mid);
                    }
                }
                previous = Some(p);
            }
            _ => {}
        }
    }
    Err(ChartError(
        "parallel fixture has no interior painted dash".into(),
    ))
}
#[wasm_bindgen]
pub fn parallel_mount(vertical: bool) -> std::result::Result<(), JsValue> {
    let layout = source(vertical, 0).map_err(js)?;
    parallel_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("parallel-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    plot: Some(layout.plot),
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                interaction_data: Some(layout.interaction.clone()),
                ..Default::default()
            },
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                parallel_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut events = capture.borrow_mut();
            if events.len() == 32 {
                events.remove(0);
            }
            events.push(event_value(event));
        });
        if let Err(error) = chart.flush() {
            chart.stop();
            root.remove();
            parallel_stop();
            return Err(js(error));
        }
        PARALLEL.with(|slot| {
            slot.borrow_mut().push(ParallelFixture {
                chart,
                root,
                renderer: name.into(),
                vertical,
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn parallel_phase(phase: u8) -> std::result::Result<(), JsValue> {
    PARALLEL.with(|slot| {
        for fixture in slot.borrow_mut().iter_mut() {
            fixture
                .chart
                .set_layout(
                    source(fixture.vertical, phase)
                        .map_err(js)?
                        .into_scene_interaction(),
                )
                .map_err(js)?;
            fixture.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn parallel_flush() -> std::result::Result<(), JsValue> {
    PARALLEL.with(|slot| {
        for fixture in slot.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn parallel_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    PARALLEL.with(|slot| {
        let entries = slot.borrow();
        let Some(f) = entries.iter().find(|f| f.renderer == renderer) else {
            return Ok(json!({"active":false}).to_string());
        };
        let state = f.chart.snapshot().map_err(js)?;
        let layout = source(f.vertical, f.phase).map_err(js)?;
        let row = layout.scene.marks.iter().find(|m| m.id == "parallel:5:row-a").unwrap();
        let probe = probe(&layout.scene, f.vertical).map_err(js)?;
        Ok(json!({
            "active":state.active,"width":state.width,"height":state.height,"revision":state.revision,
            "pending":state.pending,"hidden":state.hidden,"hovered":state.hovered,"selected":state.selected,
            "phase":f.phase,"vertical":f.vertical,
            "target":{"id":row.id,"series":row.series,"value":row.value,"label":row.label,"x":probe[0],"y":probe[1]},
            "typed":{"values":layout.rows[0].datum.values,"positions":layout.rows[0].points,
                "axisDomain":layout.dimensions[0].source_domain,"dateDomain":layout.dimensions[3].source_domain},
            "events":f.events.borrow().clone()
        }).to_string())
    })
}
#[wasm_bindgen]
pub fn parallel_rejected() -> bool {
    PARALLEL.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(layout) = source(f.vertical, f.phase) else {
                return false;
            };
            let mut candidate = layout.into_scene_interaction();
            candidate.interaction.as_mut().unwrap().sites[0].position[0] += 4.;
            f.chart.set_layout(candidate).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn parallel_stop() {
    let previous = PARALLEL.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for fixture in previous {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
