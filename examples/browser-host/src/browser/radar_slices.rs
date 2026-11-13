//! Actual typed RadarLayout -> BrowserChart polar interaction.
use super::*;
struct RadarSlicesFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static RADAR_SLICES:RefCell<Vec<RadarSlicesFixture>>=const{RefCell::new(Vec::new())};}
fn source(case: u8, phase: u8, width: f64, height: f64) -> Result<RadarLayout> {
    let c = ChartConfig {
        width,
        height,
        margin: Margin {
            left: 75.,
            right: 75.,
            top: 42.,
            bottom: 42.,
        },
        title: "Typed radar slices".into(),
        ..Default::default()
    };
    let axes = ["speed", "quality", "cost", "safety", "uptime"].map(str::to_string);
    let mut o = RadarLayoutOptions {
        domain: Some([0., 100.]),
        radius_ratio: 0.82,
        rotation: match case {
            1 => 0.43,
            2 => -0.68,
            _ => 0.,
        },
        axis_labels_by_id: BTreeMap::from([("speed".into(), "Speed <safe>".into())]),
        ..Default::default()
    };
    o.points.enabled = case != 0;
    o.points.shape = if case == 1 {
        RadarPointShape::Triangle
    } else {
        RadarPointShape::Square
    };
    o.points.size = 10.;
    o.points.labels = case > 0;
    o.points.label_style.size = Some(10.);
    o.points.value_format = ValueFormat::Affix {
        prefix: String::new(),
        suffix: "%".into(),
        precision: 1,
    };
    o.series.fill_opacity = 0.12;
    let mut data = vec![
        RadarSeries::new(
            "alpha",
            vec![if phase == 0 { 0. } else { 85. }, 70., 25., 60., 90.],
        ),
        RadarSeries::new(
            "beta",
            vec![if phase == 0 { 0. } else { 40. }, 35., 80., 55., 45.],
        ),
        RadarSeries::new(
            "gamma",
            vec![if phase == 0 { 0. } else { 65. }, 55., 45., 75., 50.],
        ),
    ];
    for (row, label) in data.iter_mut().zip(["Alpha <A>", "Beta group", "Gamma"]) {
        row.label = label.into();
    }
    radar_layout(
        &c,
        &axes,
        &data,
        &o,
        &RadarInteractionOptions {
            direction: if case == 2 {
                PolarSliceDirection::CounterClockwise
            } else {
                PolarSliceDirection::Clockwise
            },
            ..Default::default()
        },
    )
}
#[wasm_bindgen]
pub fn radar_slices_mount(case: u8) -> std::result::Result<(), JsValue> {
    if case > 2 {
        return Err(js("unknown radar slice case"));
    }
    radar_slices_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("radar-slices-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let layout = source(case, 0, 640., 400.).map_err(js)?;
        let binding = layout.interaction_binding();
        let chart = BrowserChart::mount(
            root.clone(),
            layout.scene,
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                interaction_data: Some(binding),
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
        )
        .map_err(js)?;
        let phase = Rc::new(Cell::new(0));
        let current = phase.clone();
        chart.on_resize_with_interaction(move |w, h| {
            source(case, current.get(), w, h).map(Into::into)
        });
        let events = Rc::new(RefCell::new(vec![]));
        let log = events.clone();
        chart.on_event(move |e| log.borrow_mut().push(json!({"event":format!("{e:?}")})));
        RADAR_SLICES.with(|slot| {
            slot.borrow_mut().push(RadarSlicesFixture {
                chart,
                root,
                renderer: renderer.into(),
                case,
                phase,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn radar_slices_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 1 {
        return Err(js("unknown radar slice phase"));
    }
    RADAR_SLICES.with(|slot| {
        for f in slot.borrow().iter() {
            let state = f.chart.snapshot().map_err(js)?;
            let layout = source(f.case, phase, state.width, state.height).map_err(js)?;
            f.chart
                .set_scene_with_interaction(layout.scene.clone(), layout.interaction_binding())
                .map_err(js)?;
            f.phase.set(phase);
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn radar_slices_flush() -> std::result::Result<(), JsValue> {
    RADAR_SLICES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn radar_slices_plain() -> std::result::Result<(), JsValue> {
    RADAR_SLICES.with(|slot| {
        for f in slot.borrow().iter() {
            let state = f.chart.snapshot().map_err(js)?;
            f.chart
                .set_scene(
                    source(f.case, f.phase.get(), state.width, state.height)
                        .map_err(js)?
                        .scene,
                )
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn radar_slices_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    RADAR_SLICES.with(|slot| {
        let entries=slot.borrow();
        let Some(f)=entries.iter().find(|f|f.renderer==renderer) else{return Ok(json!({"active":false}).to_string())};
        let state=f.chart.snapshot().map_err(js)?;
        let layout=source(f.case,f.phase.get(),state.width,state.height).map_err(js)?;
        let targets=layout.slices.iter().map(|s| {
            // Interior of the true sector, deliberately away from any glyph.
            let angle=s.angle+if f.case==2 {-0.16} else {0.16};
            let world=[layout.center[0]+layout.radius*0.48*angle.sin(),layout.center[1]-layout.radius*0.48*angle.cos()];
            let screen=state.view.project((world[0],world[1])).unwrap();
            json!({"slice":s.id,"axis":s.axis_id,"label":s.axis_label,"angle":s.angle,"tip":s.tip,
                "x":screen.0,"y":screen.1,"world":world,
                "ids":s.rows.iter().map(|r|&r.point_mark_id).collect::<Vec<_>>(),
                "rows":s.rows.iter().map(|r|json!({"id":r.point_mark_id,"series":r.series_id,"label":r.series_label,"raw":r.raw_value,"formatted":r.formatted_value})).collect::<Vec<_>>()})
        }).collect::<Vec<_>>();
        let active=state.interaction.as_ref().map(|a|json!({"slice":a.slice_id,"label":a.label,"anchor":a.anchor,"ids":a.ids,
            "rows":a.rows.iter().map(|r|json!({"id":r.mark_id,"series":r.series,"label":r.label,"raw":r.value,"formatted":r.formatted_value,"position":r.position})).collect::<Vec<_>>() }));
        Ok(json!({"active":state.active,"revision":state.revision,"pending":state.pending,"width":state.width,"height":state.height,
            "case":f.case,"phase":f.phase.get(),"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,
            "interaction":active,"center":layout.center,"radius":layout.radius,"targets":targets,"view":{"scale":state.view.scale,"x":state.view.translate_x,"y":state.view.translate_y},"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn radar_slices_rejected() -> bool {
    RADAR_SLICES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(state) = f.chart.snapshot() else {
                return false;
            };
            let Ok(layout) = source(f.case, f.phase.get(), state.width, state.height) else {
                return false;
            };
            let mut bad = layout.interaction_binding();
            bad.sites[0].value = Some(999.);
            let mismatch = f
                .chart
                .set_scene_with_interaction(layout.scene.clone(), bad)
                .is_err();
            let mut scene = layout.scene.clone();
            scene.marks[0].opacity = f64::NAN;
            mismatch
                && f.chart
                    .set_scene_with_interaction(scene, layout.interaction_binding())
                    .is_err()
        })
    })
}
#[wasm_bindgen]
pub fn radar_slices_stop() {
    let entries = RADAR_SLICES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
