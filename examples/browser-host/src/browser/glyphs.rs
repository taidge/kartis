//! Actual Line/Radar factory glyphs and explicit physical-frame anchors.
use kartis::surface::{Picking, SurfaceModel, SurfaceOptions};

use super::*;
struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    name: &'static str,
    case: u8,
    phase: Rc<Cell<u8>>,
    polar: Rc<Cell<bool>>,
    events: Rc<RefCell<Vec<Value>>>,
    stop: Rc<Cell<bool>>,
}
thread_local! {static GLYPHS:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};}
struct Source {
    candidate: SceneInteraction,
    glyph: GlyphGeometry,
    id: String,
    raw: f64,
}
fn glyph_options(case: u8, phase: u8) -> GlyphOptions {
    GlyphOptions {
        shape: match phase {
            2 => GlyphShape::Filled(FilledGlyph::Circle),
            3 => GlyphShape::Filled(FilledGlyph::Wye),
            4 => GlyphShape::Filled(FilledGlyph::Triangle),
            _ => match case {
                0 => GlyphShape::Filled(FilledGlyph::Star),
                1 | 3 => GlyphShape::Filled(FilledGlyph::Wye),
                _ => GlyphShape::Stroked(StrokedGlyph::Triangle),
            },
        },
        size: if phase == 0 { 900. } else { 1600. },
        rotation: if phase == 0 { 0.31 } else { 1.07 },
    }
}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 60.,
            right: 45.,
            top: 40.,
            bottom: 45.,
        },
        title: "Actual glyphs <safe>".into(),
        ..Default::default()
    }
}
fn source(case: u8, phase: u8, width: f64, height: f64, polar: bool) -> Result<Source> {
    if case > 3 || phase > 4 {
        return Err(ChartError("glyph fixture case/phase out of range".into()));
    }
    let c = config(width, height);
    let options = glyph_options(case, phase);
    let candidate: SceneInteraction = if case == 3 {
        radar_layout(
            &c,
            &["speed".into(), "quality".into(), "uptime".into()],
            &[
                RadarSeries::new(
                    "Alpha <safe>",
                    vec![if phase == 0 { 5. } else { 7. }, 6., 7.],
                ),
                RadarSeries::new("Beta", vec![8., 4., 5.]),
            ],
            &RadarLayoutOptions {
                domain: Some([0., 10.]),
                points: RadarPointOptions {
                    glyph: Some(options.clone()),
                    ..Default::default()
                },
                ..Default::default()
            },
            &RadarInteractionOptions {
                slices: polar,
                ..Default::default()
            },
        )?
        .into()
    } else {
        let mut layout = LineOptions {
            x_domain: Some([0., 10.]),
            y_domain: Some([0., 10.]),
            ..Default::default()
        };
        layout.axes.y.format = ValueFormat::Fixed(2);
        line_layout_with_style(
            &c,
            &[
                Series::new(
                    "Alpha <safe>",
                    vec![Point::new(
                        if phase == 0 { 3. } else { 4. },
                        if phase == 0 { 5. } else { 7. },
                    )],
                ),
                Series::new("Beta", vec![Point::new(7., 3.)]),
            ],
            &layout,
            &LineStyleOptions {
                series: LineSeriesStyle {
                    width: 0.,
                    point_glyph: Some(options.clone()),
                    point_border: (case == 2).then_some(GraphicStroke {
                        color: Color(25, 80, 180, 255),
                        width: 2.,
                        cap: StrokeCap::Round,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        )?
        .into()
    };
    let data = candidate.interaction.as_ref().unwrap();
    let target = data
        .sites
        .iter()
        .find(|s| s.series == "Alpha <safe>")
        .unwrap();
    Ok(Source {
        glyph: glyph_geometry(target.position, &options)?,
        id: target.mark_id.clone(),
        raw: target.value.unwrap(),
        candidate,
    })
}
fn probe(source: &Source, hit: bool) -> Result<[f64; 2]> {
    let mut binding = source.candidate.interaction.clone();
    if let Some(binding) = &mut binding {
        binding.polar = None;
        binding.polar_spoke = None;
    }
    let mut model = SurfaceModel::new_with_interaction(
        source.candidate.scene.clone(),
        SurfaceOptions {
            plot: source.candidate.plot,
            ..Default::default()
        },
        binding,
    )?;
    let mut check = |p: [f64; 2]| -> Result<bool> {
        for dx in [-1., 0., 1.] {
            for dy in [-1., 0., 1.] {
                model.apply(SurfaceInput::Move {
                    id: 1,
                    point: (p[0] + dx, p[1] + dy),
                })?;
                if model.hovered().iter().any(|id| id == &source.id) != hit {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    };
    if check(source.glyph.anchor)? {
        return Ok(source.glyph.anchor);
    }
    let b = source.glyph.bounds;
    for y in 2..19 {
        for x in 2..19 {
            let p = [
                b.x + b.width * x as f64 / 20.,
                b.y + b.height * y as f64 / 20.,
            ];
            if check(p)? {
                return Ok(p);
            }
        }
    }
    Err(ChartError(
        "glyph fixture lacks a nine-point-clear probe".into(),
    ))
}
#[wasm_bindgen]
pub fn glyphs_mount(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, false)
}
#[wasm_bindgen]
pub fn glyphs_nearest(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, true)
}
fn mount(case: u8, nearest: bool) -> std::result::Result<(), JsValue> {
    let initial = source(case, 0, 640., 360., false).map_err(js)?;
    glyphs_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("glyphs-{name}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.candidate.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                motion: browser::MotionPreference::Animate,
                highlight: browser::BrowserHighlight::Controlled,
                crosshair: Some(CrosshairOptions {
                    kind: CrosshairType::Cross,
                    dash: None,
                    stroke: GraphicStroke {
                        color: Color(210, 35, 65, 255),
                        width: 1.,
                        cap: StrokeCap::Butt,
                    },
                    opacity: 1.,
                }),
                interaction_data: initial.candidate.interaction.clone(),
                interaction: SurfaceOptions {
                    plot: initial.candidate.plot,
                    picking: if nearest {
                        Picking::Nearest { distance: 2. }
                    } else {
                        Picking::Geometry
                    },
                    pan: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let phase = Rc::new(Cell::new(0));
        let polar = Rc::new(Cell::new(false));
        let events = Rc::new(RefCell::new(Vec::new()));
        let stop = Rc::new(Cell::new(false));
        let capture = events.clone();
        let stop_event = stop.clone();
        let weak = chart.handle();
        chart.on_event(move|event|{let terminate=matches!(&event,BrowserEvent::Interaction(SurfaceEvent::Hover(ids))if !ids.is_empty())&&stop_event.replace(false);let value=match event{BrowserEvent::Error(e)=>json!({"type":"error","message":e.0}),BrowserEvent::Interaction(SurfaceEvent::Hover(ids))=>json!({"type":"hover","ids":ids}),BrowserEvent::Interaction(SurfaceEvent::Activated(id))=>json!({"type":"activate","id":id}),_=>json!({"type":"other"})};let mut log=capture.borrow_mut();if log.len()==64{log.remove(0);}log.push(value);drop(log);if terminate{weak.stop();}});
        let next_phase = phase.clone();
        let next_polar = polar.clone();
        chart.on_resize_with_interaction(move |w, h| {
            source(case, next_phase.get(), w, h, next_polar.get()).map(|s| s.candidate)
        });
        chart.flush().map_err(js)?;
        GLYPHS.with(|slot| {
            slot.borrow_mut().push(Entry {
                chart,
                root,
                name,
                case,
                phase: phase.clone(),
                polar: polar.clone(),
                events: events.clone(),
                stop: stop.clone(),
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn glyphs_phase(phase: u8) -> std::result::Result<(), JsValue> {
    change(phase, false)
}
#[wasm_bindgen]
pub fn glyphs_animate(phase: u8) -> std::result::Result<(), JsValue> {
    change(phase, true)
}
fn change(phase: u8, animate: bool) -> std::result::Result<(), JsValue> {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            let snap = entry.chart.snapshot().map_err(js)?;
            let source = source(
                entry.case,
                phase,
                snap.width,
                snap.height,
                entry.polar.get(),
            )
            .map_err(js)?;
            if animate {
                entry
                    .chart
                    .animate_to_with_interaction(
                        source.candidate.scene,
                        source.candidate.interaction.unwrap(),
                    )
                    .map_err(js)?;
            } else {
                entry.chart.set_layout(source.candidate).map_err(js)?;
            }
            entry.phase.set(phase);
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn glyphs_polar(enabled: bool) -> std::result::Result<(), JsValue> {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            entry.polar.set(enabled);
            let snap = entry.chart.snapshot().map_err(js)?;
            entry
                .chart
                .set_layout(
                    source(
                        entry.case,
                        entry.phase.get(),
                        snap.width,
                        snap.height,
                        enabled,
                    )
                    .map_err(js)?
                    .candidate,
                )
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn glyphs_flush() -> std::result::Result<(), JsValue> {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            if entry.chart.handle().is_active() {
                entry.chart.flush().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn glyphs_clear() -> std::result::Result<(), JsValue> {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            if entry.chart.handle().is_active() {
                entry
                    .chart
                    .handle()
                    .dispatch(SurfaceInput::Clear)
                    .map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn glyphs_first() -> std::result::Result<(), JsValue> {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            entry
                .chart
                .handle()
                .dispatch(SurfaceInput::First)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn glyphs_snapshot(name: &str) -> std::result::Result<String, JsValue> {
    GLYPHS.with(|slot|{let entries=slot.borrow();let Some(entry)=entries.iter().find(|e|e.name==name)else{return Ok(json!({"active":false}).to_string());};if !entry.chart.handle().is_active(){return Ok(json!({"active":false}).to_string());}let snap=entry.chart.snapshot().map_err(js)?;let source=source(entry.case,entry.phase.get(),snap.width,snap.height,entry.polar.get()).map_err(js)?;let hit=probe(&source,true).map_err(js)?;let miss=probe(&source,false).map_err(js)?;let b=source.glyph.bounds;let commands=source.glyph.geometry.commands();let support=source.glyph.anchor_indices.iter().map(|i|match commands[*i]{PathCommand::Move(p)|PathCommand::Line(p)=>[p.x,p.y],_=>unreachable!()}).collect::<Vec<_>>();Ok(json!({"active":snap.active,"case":entry.case,"phase":entry.phase.get(),"width":snap.width,"height":snap.height,"animating":snap.animating,"pending":snap.pending,"hovered":snap.hovered,"selected":snap.selected,"hidden":snap.hidden,"current":snap.interaction.map(|a|json!({"anchor":a.anchor,"ids":a.ids,"rows":a.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value,"position":r.position})).collect::<Vec<_>>()})),"target":{"id":source.id,"anchor":source.glyph.anchor,"raw":source.raw,"bounds":[b.x,b.y,b.width,b.height],"indices":source.glyph.anchor_indices,"support":support,"hit":hit,"miss":miss},"events":entry.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn glyphs_rejected() -> std::result::Result<bool, JsValue> {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            let before = entry.chart.snapshot().map_err(js)?;
            let source = source(
                entry.case,
                entry.phase.get(),
                before.width,
                before.height,
                entry.polar.get(),
            )
            .map_err(js)?;
            let dom = entry.root.inner_html();
            let mut invalid = source.candidate.clone();
            invalid.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
            if entry.chart.set_layout(invalid).is_ok() {
                return Ok(false);
            }
            let mut incompatible = source.candidate;
            let glyph = glyph_geometry(
                source.glyph.anchor,
                &GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Square),
                    size: 600.,
                    rotation: 0.,
                },
            )
            .map_err(js)?;
            let mark = incompatible
                .scene
                .marks
                .iter_mut()
                .find(|m| m.id == source.id)
                .unwrap();
            mark.geometry = glyph.geometry.clone();
            // Deliberately keep the old descriptor: this is genuinely invalid
            // metadata, unlike a valid new shape whose Fade union is supported.
            if entry
                .chart
                .animate_to_with_interaction(incompatible.scene, incompatible.interaction.unwrap())
                .is_ok()
            {
                return Ok(false);
            }
            if entry.chart.snapshot().map_err(js)? != before || dom != entry.root.inner_html() {
                return Ok(false);
            }
        }
        Ok(true)
    })
}
#[wasm_bindgen]
pub fn glyphs_stop_callback() {
    GLYPHS.with(|slot| {
        for entry in slot.borrow().iter() {
            entry.stop.set(true);
        }
    })
}
#[wasm_bindgen]
pub fn glyphs_stop() {
    let entries = GLYPHS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for entry in entries {
        entry.chart.stop();
        entry.root.remove();
    }
}
