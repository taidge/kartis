//! Style updates exercise the actual line pipeline and invisible Mesh sites.
use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static LINES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn data(phase: u8) -> Vec<Series> {
    vec![
        Series::new(
            "North:line",
            vec![
                Point::new(0., 2.),
                Point::new(1., if phase == 1 { 7.5 } else { 6. }),
                Point::gap(2.),
                Point::new(3., 8.),
                Point::new(4., 4.),
            ],
        ),
        Series::new(
            "South",
            vec![
                Point::new(0., 7.),
                Point::new(1., 3.),
                Point::new(2., 5.),
                Point::new(3., 2.),
                Point::new(4., 6.),
            ],
        ),
    ]
}
fn options(phase: u8) -> Result<(LineOptions, LineStyleOptions)> {
    if phase > 2 {
        return Err(ChartError("line fixture phase must be 0..2".into()));
    }
    let mut layout = LineOptions {
        x_domain: Some([0., 4.]),
        y_domain: Some([0., 10.]),
        area: phase != 2,
        ..Default::default()
    };
    layout.axes.y.format = ValueFormat::Fixed(1);
    let mut style = LineStyleOptions {
        connect_missing: phase == 1,
        interactive_sites: true,
        series: LineSeriesStyle {
            color: Some(Color(20, 95, 180, 255)),
            width: if phase == 2 { 0. } else { 5. },
            dash: if phase == 2 {
                None
            } else {
                Some(StrokeDash {
                    segments: vec![12., 7.],
                    offset: 2.,
                })
            },
            points: Some(phase != 1),
            point_radius: 6.,
            point_color: Some(Color(20, 155, 70, 255)),
            point_border: Some(GraphicStroke {
                color: Color(15, 25, 40, 255),
                width: 2.,
                cap: StrokeCap::Round,
            }),
            point_labels: if phase == 2 {
                None
            } else {
                Some(LinePointLabels {
                    clip_to_plot: true,
                    ..Default::default()
                })
            },
            area_opacity: 0.2,
            area_paint: Some(FillPaint::Linear {
                start: [0., 35.],
                end: [0., 305.],
                stops: vec![
                    GradientStop::new(0., Color(20, 95, 180, 255)),
                    GradientStop::new(1., Color(170, 220, 250, 255)),
                ],
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    style.series_styles.insert(
        "South".into(),
        LineSeriesStyle {
            color: Some(Color(220, 115, 30, 255)),
            width: if phase == 2 { 0. } else { 3. },
            points: Some(phase != 1),
            point_radius: 5.,
            area_opacity: 0.12,
            ..Default::default()
        },
    );
    Ok((layout, style))
}
fn source(phase: u8) -> Result<Scene> {
    let (layout, style) = options(phase)?;
    line_with_style(
        &ChartConfig {
            width: 640.,
            height: 360.,
            margin: Margin {
                left: 60.,
                right: 60.,
                top: 35.,
                bottom: 55.,
            },
            title: "Styled line".into(),
            ..Default::default()
        },
        &data(phase),
        &layout,
        &style,
    )
}
#[wasm_bindgen]
pub fn styled_line_mount() -> std::result::Result<(), JsValue> {
    let scene = source(0).map_err(js)?;
    styled_line_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("styled-line-{name}"));
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
                    picking: surface::Picking::Nearest { distance: 12. },
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(v) => v,
            Err(e) => {
                root.remove();
                styled_line_stop();
                return Err(js(e));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut e = capture.borrow_mut();
            if e.len() == 32 {
                e.remove(0);
            }
            e.push(event_value(event));
        });
        if let Err(e) = chart.flush() {
            chart.stop();
            root.remove();
            styled_line_stop();
            return Err(js(e));
        }
        LINES.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn styled_line_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let scene = source(phase).map_err(js)?;
    LINES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.set_scene(scene.clone()).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn styled_line_flush() -> std::result::Result<(), JsValue> {
    LINES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn styled_line_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    LINES.with(|slot|{
        let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};
        let state=f.chart.snapshot().map_err(js)?;let scene=source(f.phase).map_err(js)?;
        let id="line-point:10:North:line:1";let mark=scene.marks.iter().find(|m|m.id==id).unwrap();
        let Geometry::Circle{x,y,radius}=mark.geometry else{unreachable!()};
        let mesh=mesh::PointMesh::from_scene(&scene).map_err(js)?;
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"phase":f.phase,"meshHit":mesh.nearest(x,y,0.).map(|p|p.id.clone()),"target":{"id":id,"x":x,"y":y,"radius":radius,"value":mark.value},"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn styled_line_rejected() -> bool {
    LINES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok((layout, mut style)) = options(f.phase) else {
                return false;
            };
            style.series.width = f64::NAN;
            if line_with_style(&ChartConfig::default(), &data(f.phase), &layout, &style).is_ok() {
                return false;
            }
            let Ok(mut scene) = source(f.phase) else {
                return false;
            };
            scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn styled_line_stop() {
    let entries = LINES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
