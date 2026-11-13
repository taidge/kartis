//! Real combo projections and style changes; independent from the host Entries.
use super::*;
struct ComboFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static COMBOS:RefCell<Vec<ComboFixture>>=const{RefCell::new(Vec::new())};}
fn source(case: u8, phase: u8) -> Result<Scene> {
    if case > 2 || phase > 1 {
        return Err(ChartError("invalid combo fixture case/phase".into()));
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
        title: "Shared combo scales and series styles".into(),
        ..Default::default()
    };
    let shift = f64::from(phase) * 5.;
    let categories = if case == 2 {
        vec!["A", "B", "C", "D", "E"]
    } else {
        vec!["A", "B", "C"]
    };
    let opts = ComboOptions {
        x: ComboXAxis::Categories {
            values: categories.into_iter().map(str::to_owned).collect(),
            padding: 0.,
        },
        left: YAxisOptions {
            domain: Some([0., 100.]),
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([0., 1000.]),
            ..Default::default()
        },
        ..Default::default()
    };
    let labels = ComboPointLabels {
        format: ValueFormat::Fixed(1),
        offset: [2., -12.],
        color: Some(Color(40, 80, 150, 255)),
        size: Some(17.),
    };
    let data = match case {
        0 => {
            let mut boxes = ComboSeries::new(
                "Box",
                ComboKind::BoxPlot,
                ["A", "B", "C"]
                    .into_iter()
                    .map(|x| {
                        ComboPoint::box_plot(
                            x,
                            [
                                100. + shift * 10.,
                                300. + shift * 10.,
                                500. + shift * 10.,
                                700. + shift * 10.,
                                900. + shift * 10.,
                            ],
                        )
                        .identified(x)
                    })
                    .collect(),
            );
            boxes.axis = YAxisSide::Right;
            boxes.color = Some(Color(40, 140, 100, 255));
            boxes.style.line_width = 1.;
            boxes.style.whisker_width = 3.;
            boxes.style.median_width = 5.;
            boxes.style.point_labels = Some(labels);
            let mut bars = ComboSeries::new(
                "Bars",
                ComboKind::Bar,
                vec![
                    ComboPoint::new("A", 20. + shift),
                    ComboPoint::new("B", 40. + shift),
                    ComboPoint::new("C", 60. + shift),
                ],
            );
            bars.color = Some(Color(215, 130, 30, 255));
            let mut line = ComboSeries::new(
                "Line",
                ComboKind::Line,
                vec![
                    ComboPoint::new("A", 40. + shift),
                    ComboPoint::new("B", 50. + shift),
                    ComboPoint::new("C", 60. + shift),
                ],
            );
            line.color = Some(Color(40, 90, 190, 255));
            line.style.line_width = 5.;
            line.style.point_radius = Some(7.);
            vec![boxes, bars, line]
        }
        1 => {
            let mut errors = ComboSeries::new(
                "Errors",
                ComboKind::ErrorBars,
                ["A", "B", "C"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| {
                        let y = 25. + i as f64 * 15. + shift;
                        ComboPoint::new(x, y)
                            .with_bounds(y - 15., y + 20.)
                            .identified(x)
                    })
                    .collect(),
            );
            errors.color = Some(Color(180, 40, 90, 255));
            errors.style.whisker_width = 4.;
            errors.style.error_cap_width = 20.;
            errors.style.point_labels = Some(labels);
            let mut scatter = ComboSeries::new(
                "Scatter",
                ComboKind::Scatter,
                vec![
                    ComboPoint::new("A", 700. + shift * 10.),
                    ComboPoint::new("B", 600. + shift * 10.),
                    ComboPoint::new("C", 800. + shift * 10.),
                ],
            );
            scatter.axis = YAxisSide::Right;
            scatter.color = Some(Color(30, 90, 200, 255));
            scatter.style.point_radius = Some(8.);
            scatter.style.point_color = Some(Color(70, 130, 230, 255));
            scatter.style.point_border = Some(GraphicStroke {
                color: Color(20, 50, 110, 255),
                width: 3.,
                cap: StrokeCap::Round,
            });
            vec![errors, scatter]
        }
        _ => {
            let mut range = ComboSeries::new(
                "Range",
                ComboKind::RangeBand,
                vec![
                    ComboPoint::range("A", 15. + shift, 45. + shift).identified("A"),
                    ComboPoint::range("B", 25. + shift, 55. + shift).identified("B"),
                    ComboPoint::range("D", 35. + shift, 65. + shift).identified("D"),
                    ComboPoint::range("E", 45. + shift, 75. + shift).identified("E"),
                ],
            );
            range.color = Some(Color(30, 135, 95, 255));
            range.style.line_width = 4.;
            range.style.area_opacity = 0.18;
            range.style.opacity = 0.7;
            range.style.point_radius = Some(6.);
            range.style.point_color = Some(Color(20, 150, 110, 255));
            range.style.point_border = Some(GraphicStroke {
                color: Color(10, 70, 50, 255),
                width: 2.,
                cap: StrokeCap::Round,
            });
            range.style.point_labels = Some(labels);
            let mut line = ComboSeries::new(
                "Line",
                ComboKind::Line,
                vec![
                    ComboPoint::new("A", 65. + shift),
                    ComboPoint::new("B", 70. + shift),
                    ComboPoint::new("C", 75. + shift),
                    ComboPoint::new("D", 80. + shift),
                    ComboPoint::new("E", 85. + shift),
                ],
            );
            line.color = Some(Color(160, 70, 180, 255));
            line.style.line_width = 3.;
            line.style.points = Some(false);
            vec![range, line]
        }
    };
    combo(&config, &data, &opts)
}
fn target(scene: &Scene, case: u8) -> Result<(&Mark, [f64; 2])> {
    let series = match case {
        0 => "Box",
        1 => "Errors",
        _ => "Range",
    };
    let mark = scene
        .marks
        .iter()
        .find(|m| m.series == series && m.id.contains(":datum:") && m.id.ends_with("p1:A"))
        .ok_or_else(|| ChartError("combo target missing".into()))?;
    let point = match &mark.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => [x + width * 0.65, y + height * 0.6],
        Geometry::Circle { x, y, .. } => [*x, *y],
        Geometry::Path(p) => match (&p[0], &p[1]) {
            (PathCommand::Move(a), PathCommand::Line(b)) => [a.x, (a.y + b.y) / 2.],
            _ => return Err(ChartError("unexpected interval target".into())),
        },
    };
    Ok((mark, point))
}
#[wasm_bindgen]
pub fn combo_styles_mount(case: u8) -> std::result::Result<(), JsValue> {
    let scene = source(case, 0).map_err(js)?;
    combo_styles_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("combo-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
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
                combo_styles_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();
        chart.on_event(move |event| {
            let mut events = captured.borrow_mut();
            if events.len() == 32 {
                events.remove(0);
            }
            events.push(event_value(event));
        });
        chart.flush().map_err(js)?;
        COMBOS.with(|slot| {
            slot.borrow_mut().push(ComboFixture {
                chart,
                root,
                renderer: name.into(),
                case,
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn combo_styles_phase(phase: u8) -> std::result::Result<(), JsValue> {
    COMBOS.with(|slot| {
        for fixture in slot.borrow_mut().iter_mut() {
            fixture
                .chart
                .set_scene(source(fixture.case, phase).map_err(js)?)
                .map_err(js)?;
            fixture.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn combo_styles_flush() -> std::result::Result<(), JsValue> {
    COMBOS.with(|slot| {
        for fixture in slot.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn combo_styles_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    COMBOS.with(|slot|{
    let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let s=f.chart.snapshot().map_err(js)?;let scene=source(f.case,f.phase).map_err(js)?;let (mark,point)=target(&scene,f.case).map_err(js)?;
    let owned=scene.label_series.iter().filter(|(_,owner)|**owner==mark.series).map(|(i,_)|{let l=&scene.labels[*i];json!({"x":l.x,"y":l.y,"text":l.text,"size":l.size})}).collect::<Vec<_>>();
    Ok(json!({"active":s.active,"case":f.case,"phase":f.phase,"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"hovered":s.hovered,"selected":s.selected,"hidden":s.hidden,"target":{"id":mark.id,"series":mark.series,"value":mark.value,"label":mark.label,"x":point[0],"y":point[1]},"ownedLabels":owned,"events":f.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn combo_styles_rejected() -> bool {
    COMBOS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(mut scene) = source(f.case, f.phase) else {
                return false;
            };
            scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn combo_styles_stop() {
    let entries = COMBOS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
