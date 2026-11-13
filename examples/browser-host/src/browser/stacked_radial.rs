//! Nivo-style ring stacks, rounded arcs and shared polar axes in real controllers.
use super::*;
struct RadialFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static STACKED_RADIAL:RefCell<Vec<RadialFixture>>=const{RefCell::new(Vec::new())};}
fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 100.,
            right: 100.,
            top: 60.,
            bottom: 60.,
        },
        title: "Stacked radial bars".into(),
        ..Default::default()
    }
}
fn categories() -> Vec<String> {
    ["Sales", "Support", "Other"].map(String::from).to_vec()
}
fn source(phase: u8) -> Result<StackedRadialBarLayout> {
    if phase > 2 {
        return Err(ChartError("radial fixture phase must be 0..2".into()));
    }
    let values = if phase == 1 {
        [[6., 2., 2.], [3., 4., 1.], [4., 3., 1.]]
    } else {
        [[4., 3., 1.], [2., 5., 3.], [5., 1., 2.]]
    };
    let rows = ["North", "South", "East"]
        .into_iter()
        .zip(values)
        .map(|(id, v)| BarSeries::new(id, v.into_iter().map(Some).collect()))
        .collect::<Vec<_>>();
    let mut options = StackedRadialBarOptions {
        inner_radius: if phase == 2 { 0.4 } else { 0.25 },
        max_value: Some(12.),
        padding: 0.2,
        pad_angle: 0.02,
        value_format: ValueFormat::Fixed(1),
        labels: Some(RadialBarLabels {
            text: if phase == 1 {
                RadialBarLabelText::Percent(ValueFormat::Percent(0))
            } else {
                RadialBarLabelText::Value(ValueFormat::Fixed(0))
            },
            rotation: if phase == 1 { -15. } else { 0. },
            color: Some(Color(20, 30, 45, 255)),
            size: Some(13.),
            ..Default::default()
        }),
        border: Some(GraphicStroke {
            color: Color(35, 50, 70, 255),
            width: 1.5,
            cap: StrokeCap::Round,
        }),
        ..Default::default()
    };
    if phase == 1 {
        options.start_angle = 0.2;
        options.end_angle = 0.2 - std::f64::consts::TAU * 0.75;
        options.pad_angle = 0.10;
    }
    if phase == 2 {
        options.start_angle = 5.8;
        options.end_angle = 5.8 + std::f64::consts::TAU;
    }
    options.colors = BTreeMap::from([
        ("Sales".into(), Color(55, 145, 95, 255)),
        ("Support".into(), Color(50, 110, 185, 255)),
        ("Other".into(), Color(225, 145, 45, 255)),
    ]);
    options.value_ticks.values = Some(vec![0., 4., 8., 12.]);
    options.axes.circular_outer.as_mut().unwrap().rotation = if phase == 1 {
        PolarTextRotation::Tangential
    } else {
        PolarTextRotation::Fixed(0.)
    };
    options.axes.grid.dash = Some(StrokeDash {
        segments: vec![5., 4.],
        offset: 0.,
    });
    if phase == 0 {
        options.paints.insert(
            "Support".into(),
            FillPaint::Linear {
                start: [200., 100.],
                end: [440., 300.],
                stops: vec![
                    GradientStop::new(0., Color(70, 145, 215, 255)),
                    GradientStop::new(1., Color(135, 90, 185, 255)),
                ],
            },
        );
    }
    if phase == 1 {
        layout_stacked_radial_bar_with_arc_style(
            &config(),
            &categories(),
            &rows,
            &options,
            &StackedRadialArcStyle {
                corner_radius: 14.,
                ..Default::default()
            },
        )
    } else {
        layout_stacked_radial_bar(&config(), &categories(), &rows, &options)
    }
}
#[wasm_bindgen]
pub fn stacked_radial_mount() -> std::result::Result<(), JsValue> {
    let scene = source(0).map_err(js)?.scene;
    stacked_radial_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("stacked-radial-{name}"));
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
                height: Some(400.),
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
            Ok(c) => c,
            Err(e) => {
                root.remove();
                stacked_radial_stop();
                return Err(js(e));
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
        if let Err(e) = chart.flush() {
            chart.stop();
            root.remove();
            stacked_radial_stop();
            return Err(js(e));
        }
        STACKED_RADIAL.with(|slot| {
            slot.borrow_mut().push(RadialFixture {
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
pub fn stacked_radial_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let scene = source(phase).map_err(js)?.scene;
    STACKED_RADIAL.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.set_scene(scene.clone()).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn stacked_radial_flush() -> std::result::Result<(), JsValue> {
    STACKED_RADIAL.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn stacked_radial_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    STACKED_RADIAL.with(|slot| {
        let entries = slot.borrow();
        let Some(f) = entries.iter().find(|f| f.renderer == renderer) else {
            return Ok(json!({"active": false}).to_string());
        };
        let state = f.chart.snapshot().map_err(js)?;
        let l = source(f.phase).map_err(js)?;
        let segment = l
            .segments
            .iter()
            .find(|s| s.ring_id == "North" && s.category == "Sales")
            .unwrap();
        let [x, y] = l.frame.point(
            (segment.inner_radius + segment.outer_radius) / 2.,
            (segment.angles[0] + segment.angles[1]) / 2.,
        );
        let mark = l
            .scene
            .marks
            .iter()
            .find(|m| m.id == segment.mark_id)
            .unwrap();
        let rounded_corner = (f.phase == 1).then(|| {
            let [x, y] = l
                .frame
                .point(segment.outer_radius - 1.3, segment.angles[0] - 0.065);
            json!({
                "x": x, "y": y,
                "sceneHit": l.scene.hit_test(x, y).map(|m| &m.id),
            })
        });
        Ok(json!({
            "active": state.active, "width": state.width, "height": state.height,
            "revision": state.revision, "pending": state.pending, "hovered": state.hovered,
            "selected": state.selected, "hidden": state.hidden, "phase": f.phase,
            "domain": l.domain, "sceneHit": l.scene.hit_test(x, y).map(|m| &m.id),
            "target": {
                "id": mark.id, "label": mark.label, "value": mark.value, "x": x, "y": y,
                "category": "Sales", "ring": "North"
            },
            "roundedCorner": rounded_corner,
            "ringTotals": l.rings.iter().map(|r| r.total).collect::<Vec<_>>(),
            "events": f.events.borrow().clone()
        })
        .to_string())
    })
}
#[wasm_bindgen]
pub fn stacked_radial_rejected() -> bool {
    STACKED_RADIAL.with(|slot| {
        slot.borrow().iter().all(|f| {
            let bad = StackedRadialBarOptions {
                max_value: Some(-1.),
                ..Default::default()
            };
            if stacked_radial_bar(&config(), &categories(), &[], &bad).is_ok() {
                return false;
            }
            let Ok(mut l) = source(f.phase) else {
                return false;
            };
            l.scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(l.scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn stacked_radial_stop() {
    let entries = STACKED_RADIAL.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
