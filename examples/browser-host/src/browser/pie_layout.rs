//! Padded D3/Nivo arcs, exact partial fit and actual Canvas text measurements.
use std::collections::BTreeSet;
use std::f64::consts::FRAC_PI_2;

use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<PieLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static PIES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "D3 padding / measured Pie".into(),
        margin: Margin {
            left: 120.,
            right: 120.,
            top: 50.,
            bottom: 50.,
        },
        ..Default::default()
    }
}
fn text_measurements(layout: &PieLayout) -> Result<Vec<PieMeasuredLinkLabel>> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .create_element("canvas")
        .map_err(|_| ChartError("measurement canvas creation failed".into()))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    let context = canvas
        .get_context("2d")
        .map_err(|_| ChartError("measurement context failed".into()))?
        .ok_or_else(|| ChartError("measurement context unavailable".into()))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    layout
        .link_candidates
        .iter()
        .map(|c| {
            context.set_font(&format!("{}px sans-serif", c.label.size));
            context.set_text_baseline("alphabetic");
            context.set_text_align(match c.label.anchor {
                TextAnchor::Start => "start",
                TextAnchor::Middle => "center",
                TextAnchor::End => "end",
            });
            let m = context
                .measure_text(&c.label.text)
                .map_err(|_| ChartError("Canvas text measurement failed".into()))?;
            let xmin = -m.actual_bounding_box_left();
            let xmax = m.actual_bounding_box_right();
            let ymin = -m.actual_bounding_box_ascent();
            let ymax = m.actual_bounding_box_descent();
            let (sin, cos) = c.label.rotation.to_radians().sin_cos();
            let corners = [[xmin, ymin], [xmax, ymin], [xmax, ymax], [xmin, ymax]]
                .map(|p| [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos]);
            let x = corners.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
            let y = corners.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
            Ok(c.measured(LayoutRect {
                x,
                y,
                width: corners
                    .iter()
                    .map(|p| p[0])
                    .fold(f64::NEG_INFINITY, f64::max)
                    - x,
                height: corners
                    .iter()
                    .map(|p| p[1])
                    .fold(f64::NEG_INFINITY, f64::max)
                    - y,
            }))
        })
        .collect()
}
fn proposal(
    case: u8,
    phase: u8,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
) -> Result<PieLayout> {
    if case > 3 || phase > 1 {
        return Err(ChartError(
            "pie layout case/phase outside fixture profile".into(),
        ));
    }
    let data = if case == 3 {
        (0..12)
            .map(|i| RadialDatum {
                id: if i == 0 {
                    "alpha".into()
                } else {
                    format!("slice-{i}")
                },
                label: format!("Measured label {i}"),
                value: if i == 0 && phase == 1 { 2. } else { 1. },
            })
            .collect::<Vec<_>>()
    } else {
        vec![
            RadialDatum::new("alpha", if phase == 1 { 40. } else { 25. }),
            RadialDatum::new("beta", 35.),
            RadialDatum::new("gamma", 40.),
            RadialDatum::new("zero", 0.),
        ]
    };
    let mut o = PieLayoutOptions::default();
    o.pie.hidden = hidden;
    o.pie.inner_radius = 0.5;
    o.pie.pad_angle = if phase == 1 { 0.16 } else { 0.08 };
    o.pie.style.corner_radius = if phase == 1 { 18. } else { 10. };
    o.pie.style.border = Some(GraphicStroke {
        color: Color(255, 255, 255, 255),
        width: if case == 2 { 4. } else { 2. },
        cap: StrokeCap::Butt,
    });
    o.pie.style.arc_labels.legacy_fit = false;
    o.pie.style.arc_labels.skip_angle = 0.12;
    o.pie.style.arc_labels.text = PieLabelText::Value(ValueFormat::Fixed(0));
    o.pie.style.arc_labels.style.size = Some(14.);
    match case {
        1 => {
            o.pie.start_angle = -FRAC_PI_2;
            o.pie.end_angle = FRAC_PI_2;
            o.pie.inner_radius = 0.65;
            o.fit = if phase == 0 {
                PieFit::NivoSector
            } else {
                PieFit::Centered
            };
        }
        2 => {
            o.pie.end_angle = FRAC_PI_2;
            o.pie.pad_angle = 1.2;
            o.pie.inner_radius = 0.2;
            o.pie.labels = false;
            o.pie.style.corner_radius = 8.;
            o.padding = if phase == 0 {
                PiePadding::NivoRequested {
                    pad_radius: PiePadRadius::Fixed(40.),
                }
            } else {
                PiePadding::D3Applied {
                    pad_radius: PiePadRadius::Fixed(40.),
                }
            };
        }
        3 => {
            o.fit = PieFit::Centered;
            o.pie.start_angle = 0.2;
            o.pie.end_angle = 2.8;
            o.pie.pad_angle = 0.012;
            o.pie.labels = false;
            o.pie.style.link_labels.enabled = true;
            o.pie.style.link_labels.style.size = Some(if phase == 0 { 17. } else { 19. });
            o.pie.style.link_labels.style.rotation = if phase == 1 { 12. } else { 0. };
            o.pie.style.link_labels.color = PieLinkColor::Slice;
            o.pie.style.link_labels.thickness = 2.;
            o.pie.style.link_labels.diagonal_length = 14.;
            o.pie.style.link_labels.straight_length = 20.;
        }
        _ => {
            o.pie.style.fill_opacity = if phase == 0 { 1. } else { 0.65 };
        }
    }
    let raw = pie_layout(&config(width, height), &data, &o)?;
    if case == 3 {
        let m = text_measurements(&raw)?;
        raw.place_link_labels(
            &m,
            &PieLinkPlacementOptions {
                collision: LabelCollisionOptions {
                    viewport: LayoutRect {
                        x: 4.,
                        y: 4.,
                        width: width - 8.,
                        height: height - 8.,
                    },
                    padding: 3.,
                    ..Default::default()
                },
                y_offsets: if phase == 1 {
                    vec![-20., 20., -40., 40., -60., 60.]
                } else {
                    vec![]
                },
            },
        )
    } else {
        Ok(raw)
    }
}
#[wasm_bindgen]
pub fn pie_layout_mount(case: u8) -> std::result::Result<(), JsValue> {
    pie_layout_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let initial = proposal(case, 0, 640., 400., Default::default()).map_err(js)?;
        let root = document.create_element("div")?;
        root.set_id(&format!("pie-layout-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let cache = Rc::new(RefCell::new(initial.clone()));
        let phase = Rc::new(Cell::new(0));
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene,
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
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
        let controller = CategoryViewport::new(
            vec!["fixed".into()],
            ClipRect::plot(&config(640., 400.)),
            ViewportLimits::default(),
        )
        .map_err(js)?
        .into_controller();
        let p = phase.clone();
        let accepted = cache.clone();
        chart
            .on_viewport(controller, ViewportSelection::X, move |request| {
                let next = proposal(case, p.get(), request.width, request.height, request.hidden)?;
                let scene = next.scene.clone();
                *accepted.borrow_mut() = next;
                Ok(scene)
            })
            .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        chart.on_event(move |e| {
            let mut log = log.borrow_mut();
            if log.len() == 64 {
                log.remove(0);
            }
            log.push(event_value(e));
        });
        PIES.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: renderer.into(),
                case,
                phase,
                accepted: cache,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn pie_layout_phase(phase: u8) -> std::result::Result<(), JsValue> {
    PIES.with(|slot| {
        for f in slot.borrow().iter() {
            proposal(f.case, phase, 640., 400., Default::default()).map_err(js)?;
            f.phase.set(phase);
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn pie_layout_flush() -> std::result::Result<(), JsValue> {
    PIES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
fn target(layout: &PieLayout) -> Option<Value> {
    for slice in &layout.slices {
        let Some(index) = slice.mark_index else {
            continue;
        };
        let m = &layout.scene.marks[index];
        for angular in [0.5, 0.3, 0.7] {
            for radial in [0.5, 0.7, 0.3, 0.85] {
                let a = slice.start_angle + (slice.end_angle - slice.start_angle) * angular;
                let r = slice.inner_radius + (slice.outer_radius - slice.inner_radius) * radial;
                let x = layout.center[0] + r * a.sin();
                let y = layout.center[1] - r * a.cos();
                if [-1., 0., 1.].into_iter().all(|dx| {
                    [-1., 0., 1.].into_iter().all(|dy| {
                        layout
                            .scene
                            .hit_test(x + dx, y + dy)
                            .is_some_and(|hit| hit.id == m.id)
                    })
                }) {
                    return Some(json!({"id":m.id,"x":x,"y":y,"value":m.value,"label":m.label}));
                }
            }
        }
    }
    None
}
#[wasm_bindgen]
pub fn pie_layout_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    PIES.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string())};let s=f.chart.snapshot().map_err(js)?;let l=f.accepted.borrow();Ok(json!({"active":s.active,"case":f.case,"phase":f.phase.get(),"revision":s.revision,"pending":s.pending,"width":s.width,"height":s.height,"hidden":s.hidden,"selected":s.selected,"hovered":s.hovered,"target":target(&l),"center":l.center,"radius":l.radius,"bounds":[l.bounds.x,l.bounds.y,l.bounds.width,l.bounds.height],"slices":l.slices.iter().map(|p|json!({"id":p.id,"source":p.source_index,"rank":p.rank,"raw":p.value,"fraction":p.fraction,"start":p.start_angle,"end":p.end_angle,"requested":p.requested_pad,"applied":p.applied_pad,"geometryPad":p.geometry_pad,"innerAngles":p.inner_angles,"outerAngles":p.outer_angles,"innerCollapsed":p.inner_collapsed,"outerCollapsed":p.outer_collapsed,"mark":p.mark_index})).collect::<Vec<_>>(),"candidates":l.link_candidates.iter().map(|p|json!({"id":p.id,"owner":p.series,"text":p.label.text,"signature":p.signature,"position":[p.label.x,p.label.y],"points":p.points.map(|q|[q.x,q.y])})).collect::<Vec<_>>(),"placements":l.link_placements.iter().map(|p|json!({"id":p.id,"bounds":p.bounds.map(|r|[r.x,r.y,r.width,r.height]),"label":p.label_index,"leader":p.leader_index,"position":p.label_index.map(|i|[l.scene.labels[i].x,l.scene.labels[i].y]),"leaderPoints":p.leader_index.map(|i|l.scene.marks[i].geometry.commands().iter().filter_map(|q|match q{PathCommand::Move(p)|PathCommand::Line(p)=>Some([p.x,p.y]),_=>None}).collect::<Vec<_>>())})).collect::<Vec<_>>(),"owned":l.scene.label_series.values().cloned().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn pie_layout_rejected() -> bool {
    PIES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let mut o = PieLayoutOptions::default();
            o.pie.pad_angle = f64::NAN;
            let invalid =
                pie_layout(&config(640., 400.), &[RadialDatum::new("invalid", 1.)], &o).is_err();
            let l = f.accepted.borrow();
            let stale = if l.link_candidates.is_empty() {
                true
            } else {
                let mut measured = text_measurements(&l).unwrap();
                measured[0].signature.push('x');
                l.place_link_labels(&measured, &Default::default()).is_err()
            };
            let mut bad = l.scene.clone();
            if let Some(mark) = bad.marks.first_mut() {
                mark.opacity = f64::NAN;
            } else {
                bad.width = f64::NAN;
            }
            invalid && stale && f.chart.set_scene(bad).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn pie_layout_stop() {
    let old = PIES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in old {
        f.chart.stop();
        f.root.remove();
    }
}
