//! Real sample BoxPlot and multi-row Bullet inputs, without remounting updates.
use std::collections::BTreeSet;

use super::*;
struct StatisticsFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<StatisticsProposal>>,
    events: Rc<RefCell<Vec<Value>>>,
}
#[derive(Clone)]
struct StatisticsProposal {
    scene: Scene,
    metadata: Value,
    target: Value,
}
thread_local! {static STATISTICS:RefCell<Vec<StatisticsFixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Configured statistics".into(),
        margin: Margin {
            left: 88.,
            right: 74.,
            top: 45.,
            bottom: 58.,
        },
        ..Default::default()
    }
}
fn proposal(
    case: u8,
    phase: u8,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
) -> Result<StatisticsProposal> {
    if case > 3 || phase > 1 {
        return Err(ChartError("invalid statistics case/phase".into()));
    }
    let c = config(width, height);
    let horizontal = case == 1 || case == 2;
    let fmt = ValueFormat::Affix {
        prefix: "$".into(),
        suffix: String::new(),
        precision: 0,
    };
    if case < 2 {
        let mut data = Vec::new();
        for (g, base) in [("Week 1", 0.), ("Week 2", 3.)] {
            for (s, offset) in [("Alpha", 0.), ("Beta", 2.)] {
                for (i, v) in [-8., 0., 4., 8., 12., 40.].into_iter().enumerate() {
                    data.push(BoxObservation::new(
                        format!("{g}/{s}/{i}"),
                        g,
                        s,
                        Some(v + base + offset + f64::from(phase) * 2.),
                    ));
                }
            }
        }
        data.push(BoxObservation::new("missing", "Week 1", "Alpha", None));
        let o = BoxPlotLayoutOptions {
            orientation: if horizontal {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            },
            groups: Some(vec!["Week 2".into(), "Week 1".into()]),
            subgroups: Some(vec!["Alpha".into(), "Beta".into()]),
            hidden,
            domain: Some([-12., 50.]),
            whiskers: if phase == 0 {
                BoxWhiskers::Quantiles
            } else {
                BoxWhiskers::Tukey { factor: 1.5 }
            },
            padding: 0.2,
            inner_padding: 12.,
            fill_opacity: if phase == 0 { 0.45 } else { 0.7 },
            border_radius: if phase == 0 { 0. } else { 8. },
            border: Some(GraphicStroke {
                color: Color(35, 45, 65, 255),
                width: 2.,
                cap: StrokeCap::Butt,
            }),
            median: GraphicStroke {
                color: Color(20, 25, 35, 255),
                width: 3.,
                cap: StrokeCap::Butt,
            },
            whisker: GraphicStroke {
                color: Color(75, 90, 115, 255),
                width: 2.,
                cap: StrokeCap::Butt,
            },
            cap_size: if phase == 0 { 0.4 } else { 0.9 },
            outlier_radius: 5.,
            outlier_shape: if phase == 0 {
                BoxOutlierShape::Circle
            } else {
                BoxOutlierShape::Diamond
            },
            outlier_fill: Some(Color(225, 70, 85, 255)),
            value_axis: AxisOptions {
                format: fmt.clone(),
                grid: true,
                ..Default::default()
            },
            value_format: fmt,
            ..Default::default()
        };
        let l = boxplot_layout(&c, &data, &o)?;
        let target = l
            .boxes
            .iter()
            .find(|b| b.group == "Week 1" && b.subgroup == "Alpha")
            .or(l.boxes.first()).map(|b| {let r=b.rect;json!({"id":b.id,"x":r.x+r.width*if horizontal{0.25}else{0.5},"y":r.y+r.height*if horizontal{0.5}else{0.25},"value":b.quantiles[2],"breadth":b.breadth,"series":boxplot_series_id(&b.subgroup)})}).unwrap_or(Value::Null);
        Ok(StatisticsProposal {
            metadata: json!({"family":"boxplot","horizontal":horizontal,"domain":l.domain,"count":l.boxes.len(),"boxes":l.boxes.iter().map(|b|json!({"id":b.id,"group":b.group,"subgroup":b.subgroup,"n":b.count,"quantiles":b.quantiles,"whiskers":b.whiskers,"outliers":b.outliers.len(),"breadth":b.breadth})).collect::<Vec<_>>() }),
            scene: l.scene,
            target,
        })
    } else {
        let data = [
            BulletDatum {
                id: "North".into(),
                ranges: vec![10., -10., 35.],
                measures: vec![6. + f64::from(phase) * 4., 18.],
                markers: vec![25.],
            },
            BulletDatum {
                id: "South".into(),
                ranges: vec![0., 20., 50.],
                measures: vec![14., 32.],
                markers: vec![45.],
            },
        ];
        let o = BulletLayoutOptions {
            orientation: if horizontal {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            },
            reverse: case == 3,
            hidden,
            min: if phase == 1 { Some(-15.) } else { None },
            max: if phase == 1 { Some(60.) } else { None },
            spacing: 60.,
            measure_size: if phase == 0 { 0.35 } else { 0.6 },
            marker_size: if phase == 0 { 0.6 } else { 0.9 },
            marker_width: 3.,
            title_position: if phase == 0 {
                BulletPosition::Before
            } else {
                BulletPosition::After
            },
            title_align: if horizontal {
                if phase == 0 {
                    TextAnchor::End
                } else {
                    TextAnchor::Start
                }
            } else {
                TextAnchor::Middle
            },
            title_offset: if horizontal {
                [if phase == 0 { -10. } else { 10. }, 0.]
            } else {
                [0., if phase == 0 { -12. } else { 36. }]
            },
            axis_position: if phase == 0 {
                BulletPosition::After
            } else {
                BulletPosition::Before
            },
            axis: AxisOptions {
                format: fmt.clone(),
                tick_count: 4,
                ..Default::default()
            },
            value_format: fmt,
            ..Default::default()
        };
        let l = bullet_layout(&c, &data, &o)?;
        let target = l
            .rows
            .iter()
            .find(|r| r.id == "North")
            .or(l.rows.first()).map(|row| {
        let p = row
            .parts
            .iter()
            .find(|p| p.kind == BulletPartKind::Measure && p.source_index == Some(0))
            .unwrap();
        let Geometry::Rect {
            x,
            y,
            width,
            height,
        } = &p.geometry
        else {
            unreachable!()
        };
        json!({"id":p.id,"x":x+width/2.,"y":y+height/2.,"value":p.raw_value,"breadth":if horizontal{row.track.height}else{row.track.width},"series":row.id})}).unwrap_or(Value::Null);
        Ok(StatisticsProposal {
            metadata: json!({"family":"bullet","horizontal":horizontal,"reverse":o.reverse,"sharedDomain":l.shared_domain,"rows":l.rows.iter().map(|r|json!({"id":r.id,"domain":r.domain,"parts":r.parts.iter().map(|p|json!({"id":p.id,"kind":format!("{:?}",p.kind),"source":p.source_index,"value":p.raw_value,"interval":p.interval})).collect::<Vec<_>>() })).collect::<Vec<_>>() }),
            scene: l.scene,
            target,
        })
    }
}
#[wasm_bindgen]
pub fn statistics_mount(case: u8) -> std::result::Result<(), JsValue> {
    statistics_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("statistics-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let initial = proposal(case, 0, 640., 400., Default::default()).map_err(js)?;
        let accepted = Rc::new(RefCell::new(initial.clone()));
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
                tooltip_options: TooltipOptions {
                    value_format: ValueFormat::Affix {
                        prefix: "$".into(),
                        suffix: String::new(),
                        precision: 0,
                    },
                    ..Default::default()
                },
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
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
        let cache = accepted.clone();
        chart
            .on_viewport(controller, ViewportSelection::X, move |request| {
                let next = proposal(case, p.get(), request.width, request.height, request.hidden)?;
                let scene = next.scene.clone();
                *cache.borrow_mut() = next;
                Ok(scene)
            })
            .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        chart.on_event(move |event| log.borrow_mut().push(json!({"event":format!("{event:?}")})));
        STATISTICS.with(|slot| {
            slot.borrow_mut().push(StatisticsFixture {
                chart,
                root,
                renderer: renderer.into(),
                case,
                phase,
                accepted,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn statistics_phase(phase: u8) -> std::result::Result<(), JsValue> {
    STATISTICS.with(|slot| {
        for f in slot.borrow().iter() {
            proposal(f.case, phase, 640., 400., Default::default()).map_err(js)?;
            f.phase.set(phase);
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn statistics_flush() -> std::result::Result<(), JsValue> {
    STATISTICS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn statistics_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    STATISTICS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string())};let s=f.chart.snapshot().map_err(js)?;let proposal=f.accepted.borrow();Ok(json!({"active":s.active,"revision":s.revision,"width":s.width,"height":s.height,"hidden":s.hidden,"selected":s.selected,"hovered":s.hovered,"pending":s.pending,"phase":f.phase.get(),"case":f.case,"target":proposal.target,"metadata":proposal.metadata,"marks":proposal.scene.marks.iter().map(|m|json!({"id":m.id,"value":m.value,"series":m.series,"label":m.label,"opacity":m.opacity})).collect::<Vec<_>>(),"owned":proposal.scene.label_series.values().cloned().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn statistics_rejected() -> bool {
    STATISTICS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let invalid = if f.case < 2 {
                boxplot_with_options(
                    &config(640., 400.),
                    &[BoxObservation::new("bad", "G", "S", Some(f64::NAN))],
                    &Default::default(),
                )
                .is_err()
            } else {
                bullet_with_options(
                    &config(640., 400.),
                    &[BulletDatum {
                        id: "bad".into(),
                        ranges: vec![f64::NAN],
                        measures: vec![],
                        markers: vec![],
                    }],
                    &Default::default(),
                )
                .is_err()
            };
            let mut scene = f.accepted.borrow().scene.clone();
            scene.marks[0].opacity = f64::NAN;
            invalid && f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn statistics_stop() {
    let entries = STATISTICS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
