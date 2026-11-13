//! Explicit IDs and metadata drive real ordinary/circular, horizontal/vertical
//! Sankey acceptance. No mark role is inferred from a substring of its ID.
use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: u8,
    hide_target: bool,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static SANKEY_STYLES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn source(case: u8, phase: u8, hide_target: bool) -> Result<SankeyStyledLayout> {
    if case > 3 || phase > 1 {
        return Err(ChartError("invalid Sankey style case/phase".into()));
    }
    let config = ChartConfig {
        width: 640.,
        height: 420.,
        margin: Margin {
            left: 56.,
            right: 56.,
            top: 46.,
            bottom: 46.,
        },
        title: "Styled stable-ID Sankey".into(),
        ..Default::default()
    };
    let mut nodes = ["A", "B", "C", "D"].map(FlowNode::new).to_vec();
    nodes[0].label = "Source <safe>".into();
    let shift = f64::from(phase) * 2.;
    let mut links = vec![
        SankeyStyleLink::new("ab", "A", "B", 6. + shift),
        SankeyStyleLink::new("ac", "A", "C", 4.),
        SankeyStyleLink::new("bd", "B", "D", 6. + shift),
        SankeyStyleLink::new("cd", "C", "D", 4.),
    ];
    if case >= 2 {
        links.push(SankeyStyleLink::new("return", "D", "A", 2.));
        links.push(SankeyStyleLink::new("self", "B", "B", 1.));
    }
    let mut style = SankeyStyleOptions {
        orientation: if case % 2 == 1 {
            SankeyOrientation::Vertical
        } else {
            SankeyOrientation::Horizontal
        },
        node: SankeyNodeStyle {
            corner_radius: if phase == 0 { 7. } else { 5. },
            border: Some(GraphicStroke {
                color: Color(30, 35, 40, 255),
                width: 2.,
                cap: StrokeCap::Round,
            }),
            ..Default::default()
        },
        link: SankeyLinkStyle {
            color: SankeyLinkColor::Gradient,
            opacity: Some(0.7),
        },
        label: SankeyLabelStyle {
            position: SankeyLabelPosition::Inside,
            orientation: if case % 2 == 1 {
                SankeyLabelOrientation::Vertical
            } else {
                SankeyLabelOrientation::Horizontal
            },
            padding: 10.,
            size: Some(15.),
            color: Some(Color(40, 20, 100, 255)),
            ..Default::default()
        },
        forward_curvature: if phase == 0 { 0.25 } else { 0.8 },
        ..Default::default()
    };
    for (id, color) in [
        ("A", Color(230, 35, 35, 255)),
        ("B", Color(25, 65, 220, 255)),
        ("C", Color(20, 150, 70, 255)),
        ("D", Color(170, 70, 175, 255)),
    ] {
        style.nodes.insert(
            id.into(),
            SankeyNodeStyle {
                color: Some(color),
                ..style.node.clone()
            },
        );
    }
    style.links.insert(
        "ac".into(),
        SankeyLinkStyle {
            color: SankeyLinkColor::Target,
            opacity: Some(0.55),
        },
    );
    style.links.insert(
        "cd".into(),
        SankeyLinkStyle {
            color: SankeyLinkColor::Source,
            opacity: Some(0.6),
        },
    );
    if hide_target {
        style.hidden.insert("B".into());
    }
    if case >= 2 {
        circular_sankey_with_style(
            &config,
            &nodes,
            &links,
            &CircularSankeyOptions::default(),
            &style,
        )
    } else {
        sankey_with_style(
            &config,
            &nodes,
            &links,
            &SankeyLayoutOptions::default(),
            &style,
        )
    }
}
fn target(layout: &SankeyStyledLayout) -> Result<(&Mark, [f64; 2])> {
    let link = layout
        .links
        .iter()
        .find(|l| l.id == "ab")
        .ok_or_else(|| ChartError("missing fixture link".into()))?;
    let mark = layout
        .scene
        .marks
        .iter()
        .find(|m| m.id == link.mark_id)
        .ok_or_else(|| ChartError("fixture target is hidden".into()))?;
    let Geometry::Path(cmds) = &link.centerline else {
        return Err(ChartError("missing cubic centerline".into()));
    };
    let (
        PathCommand::Move(a),
        PathCommand::Cubic {
            first: b,
            second: c,
            end: d,
        },
    ) = (&cmds[0], &cmds[1])
    else {
        return Err(ChartError("unexpected cubic centerline".into()));
    };
    for t in [0.5, 0.35, 0.65, 0.2, 0.8] {
        let q: f64 = 1. - t;
        let p = [
            q * q * q * a.x + 3. * q * q * t * b.x + 3. * q * t * t * c.x + t * t * t * d.x,
            q * q * q * a.y + 3. * q * q * t * b.y + 3. * q * t * t * c.y + t * t * t * d.y,
        ];
        if layout
            .scene
            .hit_test(p[0], p[1])
            .is_some_and(|m| m.id == mark.id)
        {
            return Ok((mark, p));
        }
    }
    Err(ChartError("no exposed fixture ribbon center".into()))
}
#[wasm_bindgen]
pub fn sankey_styles_mount(case: u8) -> std::result::Result<(), JsValue> {
    let layout = source(case, 0, false).map_err(js)?;
    sankey_styles_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("sankey-style-{name}"));
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
                height: Some(420.),
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
                sankey_styles_stop();
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
        SANKEY_STYLES.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                case,
                phase: 0,
                hide_target: false,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn sankey_styles_phase(phase: u8) -> std::result::Result<(), JsValue> {
    SANKEY_STYLES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            let layout = source(f.case, phase, f.hide_target).map_err(js)?;
            f.chart.set_scene(layout.scene).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn sankey_styles_hide_target(hidden: bool) -> std::result::Result<(), JsValue> {
    SANKEY_STYLES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            let layout = source(f.case, f.phase, hidden).map_err(js)?;
            f.chart.set_scene(layout.scene).map_err(js)?;
            f.hide_target = hidden;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn sankey_styles_flush() -> std::result::Result<(), JsValue> {
    SANKEY_STYLES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn sankey_styles_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    SANKEY_STYLES.with(|slot|{
        let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};
        let state=f.chart.snapshot().map_err(js)?;let layout=source(f.case,f.phase,f.hide_target).map_err(js)?;
        let target=if f.hide_target{Value::Null}else{let(mark,p)=target(&layout).map_err(js)?;json!({"id":mark.id,"series":mark.series,"label":mark.label,"value":mark.value,"x":p[0],"y":p[1]})};
        let nodes=layout.nodes.iter().map(|n|json!({"id":n.id,"markId":n.mark_id,"rect":[n.rect.x,n.rect.y,n.rect.width,n.rect.height],"inflow":n.inflow,"outflow":n.outflow,"value":n.value,"visible":n.visible})).collect::<Vec<_>>();
        let links=layout.links.iter().map(|l|{let route=match l.route {SankeyStyledRoute::Forward=>json!({"type":"forward"}),SankeyStyledRoute::Return{side,lane,coordinate}=>json!({"type":"return","side":format!("{side:?}"),"lane":lane,"coordinate":coordinate})};json!({"id":l.id,"markId":l.mark_id,"source":l.source,"target":l.target,"value":l.value,"width":l.width,"sourcePort":l.source_port,"targetPort":l.target_port,"route":route,"visible":l.visible})}).collect::<Vec<_>>();
        let labels=layout.scene.labels.iter().enumerate().map(|(i,l)|json!({"text":l.text,"series":layout.scene.label_series.get(&i),"size":l.size,"rotation":l.rotation,"x":l.x,"y":l.y})).collect::<Vec<_>>();
        Ok(json!({"active":state.active,"case":f.case,"phase":f.phase,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"target":target,"nodes":nodes,"links":links,"labels":labels,"markIds":layout.scene.marks.iter().map(|m|m.id.clone()).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn sankey_styles_rejected() -> bool {
    SANKEY_STYLES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(mut layout) = source(f.case, f.phase, f.hide_target) else {
                return false;
            };
            layout.scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(layout.scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn sankey_styles_stop() {
    let entries = SANKEY_STYLES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
