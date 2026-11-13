//! Real projected glyphs, original origins, and azimuthal paint masks.
use std::collections::BTreeSet;

use kartis::surface::{Picking, SurfaceModel, SurfaceOptions};

use super::*;
struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    name: &'static str,
    case: u8,
    phase: Rc<Cell<u8>>,
    nearest: bool,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static GEO_GLYPHS:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};}
struct Source {
    candidate: SceneInteraction,
    layer: GeoGlyphLayer,
    disk: Option<GeoProjectionDisk>,
    backface: bool,
}
fn source(case: u8, phase: u8, width: f64, height: f64) -> Result<Source> {
    if case > 1 || phase > 1 {
        return Err(ChartError("Geo glyph case/phase out of range".into()));
    }
    let c = ChartConfig {
        width,
        height,
        margin: Margin {
            left: 25.,
            right: 25.,
            top: 25.,
            bottom: 25.,
        },
        ..Default::default()
    };
    let view = GeoMapOptions {
        projection: if case == 0 {
            GeoProjection::EqualEarth
        } else {
            GeoProjection::Orthographic
        },
        scale: Some(if case == 0 { width / 7. } else { height / 3. }),
        translate: Some([width * 0.48, height * 0.52]),
        fill: Some(Color(229, 237, 244, 255)),
        border: Some(Color(150, 170, 185, 255)),
        ..Default::default()
    };
    let features = vec![GeoFeature {
        id: "land".into(),
        label: String::new(),
        value: Some(1.),
        polygons: vec![vec![vec![
            [-35., -35.],
            [35., -35.],
            [35., 35.],
            [-35., 35.],
            [-35., -35.],
        ]]],
    }];
    let mut map = choropleth_layout(&c, &features, &view)?;
    map.scene.legend.clear();
    for m in &mut map.scene.marks {
        m.label.clear();
        m.series.clear();
    }
    let positions = if case == 0 {
        [[-35., 6.], [38., -10.], [0., -40.], [70., 28.]]
    } else {
        [[-20., 12.], [85., 0.], [-15., -40.], [30., 35.]]
    };
    let mut data = vec![
        GeoSymbol::new(
            "main",
            positions[0],
            Some(if phase == 0 { 36. } else { 81. }),
        ),
        GeoSymbol::new("partial", positions[1], Some(64.)),
        GeoSymbol::new("missing", positions[2], None),
        GeoSymbol::new("zero", positions[3], Some(0.)),
        GeoSymbol::new("back", [150., 0.], Some(25.)),
    ];
    for s in &mut data {
        s.series = if s.id == "partial" {
            "Beta"
        } else {
            "Alpha <safe>"
        }
        .into();
        s.label = format!("{} <safe>", s.id);
    }
    let shape = match (case, phase) {
        (0, 0) => GlyphShape::Filled(FilledGlyph::Star),
        (0, _) => GlyphShape::Filled(FilledGlyph::Wye),
        (1, 0) => GlyphShape::Stroked(StrokedGlyph::Triangle),
        _ => GlyphShape::Stroked(StrokedGlyph::Plus),
    };
    let options = GeoGlyphOptions {
        shape,
        rotation: if phase == 0 { 0.23 } else { 0.77 },
        symbols: GeoSymbolOptions {
            domain: Some([0., 100.]),
            radius_range: [0., 24.],
            zero_radius: 7.,
            missing: GeoMissingSymbol::Draw {
                radius: 12.,
                color: Color(195, 105, 30, 255),
                label: "not available".into(),
            },
            labels: true,
            label_size: 11.,
            value_format: ValueFormat::Fixed(2),
            stroke: Some(GraphicStroke {
                color: Color(30, 65, 110, 255),
                width: if case == 0 { 1. } else { 4. },
                cap: StrokeCap::Round,
            }),
            clip_to_projection: case == 1,
            clip_labels_to_projection: case == 1,
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions {
                    position: [25., 35.],
                    title: "Nominal size".into(),
                    text_size: 10.,
                    ..Default::default()
                },
                values: vec![16., 64.],
            }),
            color_legend: Some(GeoLegendOptions {
                position: [width - 130., 35.],
                title: "Series".into(),
                text_size: 10.,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let backface = map.projection.project([150., 0.])?.is_none();
    let disk = map.projection.projection_disk();
    let layer = map.add_glyphs(&data, &options)?;
    let interaction = layer.interaction_binding(&map.scene)?;
    Ok(Source {
        candidate: SceneInteraction {
            scene: map.scene,
            interaction: Some(interaction),
            plot: Some(map.projection.plot()),
        },
        layer,
        disk,
        backface,
    })
}
fn probe(s: &Source, site: &GeoGlyphSite, hit: bool) -> Result<[f64; 2]> {
    let mut model = SurfaceModel::new_with_interaction(
        s.candidate.scene.clone(),
        SurfaceOptions {
            plot: s.candidate.plot,
            ..Default::default()
        },
        s.candidate.interaction.clone(),
    )?;
    let mut good = |p: [f64; 2]| -> Result<bool> {
        for dx in [-1., 0., 1.] {
            for dy in [-1., 0., 1.] {
                model.apply(SurfaceInput::Move {
                    id: 1,
                    point: (p[0] + dx, p[1] + dy),
                })?;
                if model.hovered().iter().any(|id| id == &site.mark_id) != hit {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    };
    if good(site.glyph.anchor)? {
        return Ok(site.glyph.anchor);
    }
    let b = site.glyph.bounds;
    for y in 1..30 {
        for x in 1..30 {
            let p = [
                b.x + b.width * x as f64 / 30.,
                b.y + b.height * y as f64 / 30.,
            ];
            if good(p)? {
                return Ok(p);
            }
        }
    }
    Err(ChartError("Geo glyph lacks robust contour probe".into()))
}
fn masked_probe(s: &Source) -> Option<[f64; 2]> {
    let disk = s.disk?;
    let site = s.layer.sites.iter().find(|p| p.id == "partial")?;
    let mut unmasked = s.candidate.scene.clone();
    unmasked
        .marks
        .iter_mut()
        .find(|m| m.id == site.mark_id)?
        .clip_path = None;
    let b = site.glyph.bounds;
    for y in 0..60 {
        for x in 0..60 {
            let p = [
                b.x + b.width * x as f64 / 59.,
                b.y + b.height * y as f64 / 59.,
            ];
            if (p[0] - disk.center[0]).hypot(p[1] - disk.center[1]) > disk.radius + 4.
                && unmasked
                    .hit_test(p[0], p[1])
                    .is_some_and(|m| m.id == site.mark_id)
                && s.candidate
                    .scene
                    .hit_test(p[0], p[1])
                    .is_none_or(|m| m.id != site.mark_id)
            {
                return Some(p);
            }
        }
    }
    None
}
#[wasm_bindgen]
pub fn geo_glyphs_mount(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, false)
}
#[wasm_bindgen]
pub fn geo_glyphs_nearest(case: u8) -> std::result::Result<(), JsValue> {
    mount(case, true)
}
fn mount(case: u8, nearest: bool) -> std::result::Result<(), JsValue> {
    let initial = source(case, 0, 640., 360.).map_err(js)?;
    geo_glyphs_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("geo-glyphs-{name}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.candidate.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                interaction_data: initial.candidate.interaction.clone(),
                tooltip_options: TooltipOptions {
                    lines: vec![vec![TooltipPart::Label]],
                    ..Default::default()
                },
                interaction: SurfaceOptions {
                    plot: initial.candidate.plot,
                    picking: if nearest {
                        Picking::Nearest { distance: 2. }
                    } else {
                        Picking::Geometry
                    },
                    zoom: false,
                    pan: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |e| {
            let v = match e {
                BrowserEvent::Interaction(SurfaceEvent::Hover(ids)) => {
                    json!({"type":"hover","ids":ids})
                }
                BrowserEvent::Interaction(SurfaceEvent::Activated(id)) => {
                    json!({"type":"activate","id":id})
                }
                BrowserEvent::Error(e) => json!({"type":"error","message":e.0}),
                _ => json!({"type":"other"}),
            };
            let mut log = capture.borrow_mut();
            if log.len() == 64 {
                log.remove(0);
            }
            log.push(v);
        });
        let phase = Rc::new(Cell::new(0));
        let p = phase.clone();
        chart.on_resize_with_interaction(move |w, h| {
            source(case, p.get(), w, h).map(|s| s.candidate)
        });
        chart.flush().map_err(js)?;
        GEO_GLYPHS.with(|slot| {
            slot.borrow_mut().push(Entry {
                chart,
                root,
                name,
                case,
                phase,
                nearest,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn geo_glyphs_phase(phase: u8) -> std::result::Result<(), JsValue> {
    GEO_GLYPHS.with(|slot| {
        for e in slot.borrow().iter() {
            let snap = e.chart.snapshot().map_err(js)?;
            e.chart
                .set_layout(
                    source(e.case, phase, snap.width, snap.height)
                        .map_err(js)?
                        .candidate,
                )
                .map_err(js)?;
            e.phase.set(phase);
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_glyphs_flush() -> std::result::Result<(), JsValue> {
    GEO_GLYPHS.with(|slot| {
        for e in slot.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_glyphs_clear() -> std::result::Result<(), JsValue> {
    GEO_GLYPHS.with(|slot| {
        for e in slot.borrow().iter() {
            e.chart.handle().dispatch(SurfaceInput::Clear).map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_glyphs_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    GEO_GLYPHS.with(|slot| {
        for e in slot.borrow().iter() {
            e.chart
                .handle()
                .set_hidden(if hidden {
                    BTreeSet::from(["Alpha <safe>".into()])
                } else {
                    BTreeSet::new()
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_glyphs_snapshot(name: &str) -> std::result::Result<String, JsValue> {
    GEO_GLYPHS.with(|slot|{let entries=slot.borrow();let Some(e)=entries.iter().find(|e|e.name==name)else{return Ok(json!({"active":false}).to_string());};if !e.chart.handle().is_active(){return Ok(json!({"active":false}).to_string());}let snap=e.chart.snapshot().map_err(js)?;let s=source(e.case,e.phase.get(),snap.width,snap.height).map_err(js)?;let main=s.layer.sites.iter().find(|p|p.id=="main").unwrap();let b=main.glyph.bounds;
    let sites=s.layer.sites.iter().map(|p|json!({"id":p.id,"markId":p.mark_id,"series":p.series,"label":p.label,"raw":p.value,"formatted":p.formatted_value,"anchor":p.glyph.anchor})).collect::<Vec<_>>();
    let hit=probe(&s,main,true).map_err(js)?;let miss=probe(&s,main,false).map_err(js)?;
    let partial=s.layer.sites.iter().find(|p|p.id=="partial").unwrap();let partial_mark=s.candidate.scene.marks.iter().find(|m|m.id==partial.mark_id).unwrap();
    Ok(json!({"active":snap.active,"case":e.case,"phase":e.phase.get(),"nearest":e.nearest,"width":snap.width,"height":snap.height,"pending":snap.pending,"revision":snap.revision,"hovered":snap.hovered,"selected":snap.selected,"hidden":snap.hidden,"current":snap.interaction.map(|a|json!({"anchor":a.anchor,"ids":a.ids,"rows":a.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value,"position":r.position})).collect::<Vec<_>>()})),"sites":sites,"target":{"id":main.mark_id,"anchor":main.glyph.anchor,"raw":main.value,"hit":hit,"miss":miss,"bounds":[b.x,b.y,b.width,b.height]},"disk":s.disk.map(|d|json!({"center":d.center,"radius":d.radius})),"backfaceCulled":s.backface,"partial":{"id":partial.mark_id,"anchor":partial.glyph.anchor,"masked":partial_mark.clip_path.is_some(),"outside":masked_probe(&s)},"ownedLabels":s.candidate.scene.label_series,"events":e.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn geo_glyphs_rejected() -> std::result::Result<bool, JsValue> {
    GEO_GLYPHS.with(|slot| {
        for e in slot.borrow().iter() {
            let before = e.chart.snapshot().map_err(js)?;
            let dom = e.root.inner_html();
            let mut s = source(e.case, e.phase.get(), before.width, before.height).map_err(js)?;
            s.candidate.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
            if e.chart.set_layout(s.candidate).is_ok() {
                return Ok(false);
            }
            if before != e.chart.snapshot().map_err(js)? || dom != e.root.inner_html() {
                return Ok(false);
            }
        }
        Ok(true)
    })
}
#[wasm_bindgen]
pub fn geo_glyphs_stop() {
    for e in GEO_GLYPHS.with(|slot| std::mem::take(&mut *slot.borrow_mut())) {
        e.chart.stop();
        e.root.remove();
    }
}
