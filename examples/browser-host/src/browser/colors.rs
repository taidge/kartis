//! Shared data-color scales, real choropleth/calendar consumers and legends.
use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static COLORS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
struct Source {
    scene: Scene,
    target: [f64; 2],
    id: String,
    value: f64,
    color: Color,
    legend: ColorLegendLayout,
}
fn source(phase: u8) -> Result<Source> {
    if phase > 4 {
        return Err(ChartError("color fixture phase must be 0..4".into()));
    }
    let config = ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 40.,
            bottom: 130.,
        },
        title: "Shared data colors".into(),
        ..Default::default()
    };
    let colors = ColorScaleOptions {
        transform: ColorScaleTransform::Linear,
        kind: match phase {
            0 => ColorScaleKind::Diverging { midpoint: 0. },
            1 => ColorScaleKind::Quantize,
            3 => ColorScaleKind::Quantile,
            4 => ColorScaleKind::Threshold,
            _ => ColorScaleKind::Sequential,
        },
        domain: Some(if phase == 2 { [-5., 5.] } else { [-10., 10.] }),
        colors: vec![
            Color(20, 90, 180, 255),
            Color(30, 160, 90, 255),
            Color(245, 195, 40, 255),
        ],
        unknown: Color(238, 238, 238, 255),
        clamp: true,
        thresholds: if phase == 4 {
            vec![-5., 0.]
        } else {
            Vec::new()
        },
    };
    let (mut scene, scale, target, id, value) = if phase == 2 {
        let data = [
            CalendarDate::new(2024, 2, 28, -5.),
            CalendarDate::new(2024, 2, 29, 0.),
            CalendarDate::new(2024, 3, 1, 5.),
        ];
        let layout = calendar_range_with_color_scale(
            &config,
            &data,
            &CalendarRangeOptions {
                years: Some((2024, 2024)),
                ..Default::default()
            },
            &colors,
        )?;
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == "2024-02-29")
            .unwrap();
        let Geometry::Rect {
            x,
            y,
            width,
            height,
        } = mark.geometry
        else {
            unreachable!()
        };
        (
            layout.scene,
            layout.color_scale,
            [x + width / 2., y + height / 2.],
            "2024-02-29".to_string(),
            0.,
        )
    } else {
        let features = (0..3)
            .map(|i| GeoFeature {
                id: format!("region:{i}"),
                label: format!("Region <{}>", i + 1),
                value: if i == 2 {
                    None
                } else {
                    Some(if phase == 0 {
                        [-10., 0.][i]
                    } else {
                        [-5., 8.][i]
                    })
                },
                polygons: vec![vec![vec![
                    [-35. + i as f64 * 25., -10.],
                    [-15. + i as f64 * 25., -10.],
                    [-15. + i as f64 * 25., 20.],
                    [-35. + i as f64 * 25., 20.],
                ]]],
            })
            .collect::<Vec<_>>();
        let layout = choropleth_with_color_scale(
            &config,
            &features,
            &GeoMapOptions {
                fit: GeoFit::Data,
                ..Default::default()
            },
            &colors,
        )?;
        let target = layout.projection.project([-25., 5.])?.unwrap();
        (
            layout.scene,
            layout.color_scale,
            target,
            "map:region:0".into(),
            if phase == 0 { -10. } else { -5. },
        )
    };
    let legend = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            id: "fixture".into(),
            position: [120., 325.],
            length: 300.,
            thickness: 18.,
            steps: 2,
            tick_count: 3,
            format: ValueFormat::Fixed(1),
            title: "Observed value".into(),
            unknown_label: Some("Missing".into()),
            ..Default::default()
        },
    )?;
    Ok(Source {
        scene,
        target,
        id,
        value,
        color: scale.map(Some(value)),
        legend,
    })
}
#[wasm_bindgen]
pub fn color_mount() -> std::result::Result<(), JsValue> {
    let s = source(0).map_err(js)?;
    color_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("colors-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            s.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(400.),
                legend: false,
                motion: browser::MotionPreference::Reduced,
                tooltip_options: TooltipOptions {
                    lines: vec![vec![TooltipPart::Label]],
                    ..Default::default()
                },
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
            Ok(v) => v,
            Err(e) => {
                root.remove();
                color_stop();
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
            color_stop();
            return Err(js(e));
        }
        COLORS.with(|slot| {
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
pub fn color_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let s = source(phase).map_err(js)?;
    COLORS.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.set_scene(s.scene.clone()).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn color_flush() -> std::result::Result<(), JsValue> {
    COLORS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn color_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    COLORS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let state=f.chart.snapshot().map_err(js)?;let s=source(f.phase).map_err(js)?;Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"phase":f.phase,"hovered":state.hovered,"selected":state.selected,"target":{"id":s.id,"x":s.target[0],"y":s.target[1],"value":s.value,"color":[s.color.0,s.color.1,s.color.2,s.color.3]},"stops":s.legend.stops.iter().map(|s|json!({"offset":s.offset,"value":s.value,"color":[s.color.0,s.color.1,s.color.2,s.color.3]})).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn color_rejected() -> bool {
    COLORS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(mut s) = source(f.phase) else {
                return false;
            };
            let old = s.scene.clone();
            if append_color_legend(
                &mut s.scene,
                &ColorScaleOptions::default().resolve([]).unwrap(),
                &ColorLegendOptions {
                    length: f64::NAN,
                    ..Default::default()
                },
            )
            .is_ok()
                || s.scene != old
            {
                return false;
            }
            s.scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(s.scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn color_stop() {
    let entries = COLORS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
