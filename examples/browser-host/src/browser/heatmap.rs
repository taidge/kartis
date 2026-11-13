//! Controlled HeatMap peers use the same public layout as application code.
use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: u8,
    layout: Rc<RefCell<HeatmapLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
#[derive(Default)]
struct Peers {
    hover: Option<HeatmapCellKey>,
    focus: Option<HeatmapCellKey>,
}
thread_local! {static HEATMAPS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn hover_target(mode: u8) -> Result<HeatmapHoverTarget> {
    Ok(match mode {
        0 => HeatmapHoverTarget::Cell,
        1 => HeatmapHoverTarget::Row,
        2 => HeatmapHoverTarget::Column,
        3 => HeatmapHoverTarget::RowColumn,
        4 => HeatmapHoverTarget::All,
        _ => return Err(ChartError("invalid HeatMap hover target".into())),
    })
}
fn source(
    case: u8,
    phase: u8,
    target: HeatmapHoverTarget,
    active: Option<HeatmapCellKey>,
    hidden: bool,
) -> Result<HeatmapLayout> {
    if case > 3 || phase > 1 {
        return Err(ChartError("invalid HeatMap case/phase".into()));
    }
    let c = ChartConfig {
        width: 640.,
        height: 420.,
        margin: Margin {
            left: 90.,
            right: 35.,
            top: 70.,
            bottom: 35.,
        },
        title: "Nullable controlled HeatMap".into(),
        ..Default::default()
    };
    let mut cells = vec![];
    for (row, values) in [
        ("North", vec![Some(0.), None, Some(9.), Some(-4.)]),
        (
            "South",
            vec![Some(2.), Some(4.), Some(6. + f64::from(phase)), Some(8.)],
        ),
        ("West", vec![Some(1.), Some(3.), Some(5.)]),
    ] {
        for (column, value) in ["alpha", "beta", "gamma", "delta"].into_iter().zip(values) {
            cells.push(HeatCell {
                row: row.into(),
                column: column.into(),
                value,
            });
        }
    }
    let colors = match case {
        1 => ColorScaleOptions {
            kind: ColorScaleKind::Diverging { midpoint: 0. },
            domain: Some([-5., 10.]),
            colors: vec![
                Color(40, 65, 200, 255),
                Color(245, 245, 245, 255),
                Color(210, 35, 40, 255),
            ],
            ..Default::default()
        },
        2 => ColorScaleOptions {
            kind: ColorScaleKind::Quantize,
            domain: Some([-5., 10.]),
            colors: vec![
                Color(40, 65, 200, 255),
                Color(60, 150, 210, 255),
                Color(245, 170, 50, 255),
                Color(210, 35, 40, 255),
            ],
            ..Default::default()
        },
        _ => ColorScaleOptions {
            domain: Some(if case == 3 { [10., -5.] } else { [-5., 10.] }),
            colors: vec![Color(35, 80, 190, 255), Color(220, 45, 50, 255)],
            ..Default::default()
        },
    };
    let top = HeatmapAxis {
        options: axes::CategoryAxisOptions {
            tick_rotation: -25.,
            ..Default::default()
        },
        text: HeatmapTextStyle {
            color: Some(Color(25, 60, 130, 255)),
            size: Some(17.),
        },
        ..Default::default()
    };
    heatmap_layout(
        &c,
        &cells,
        &HeatmapLayoutOptions {
            shape: if case % 2 == 1 {
                HeatmapShape::Circle
            } else {
                HeatmapShape::Rect
            },
            x_padding: HeatmapBandPadding {
                inner: if phase == 0 { 0.15 } else { 0.25 },
                outer: 0.1,
            },
            y_padding: HeatmapBandPadding {
                inner: 0.25,
                outer: 0.2,
            },
            size_variation: if case >= 2 {
                Some(HeatmapSizeVariation {
                    domain: Some([-5., 10.]),
                    sizes: if phase == 0 { [0.25, 1.] } else { [0.4, 0.95] },
                })
            } else {
                None
            },
            colors,
            corner_radius: 9.,
            border: Some(GraphicStroke {
                color: Color(15, 30, 50, 255),
                width: 2.,
                cap: StrokeCap::Butt,
            }),
            label_style: HeatmapTextStyle {
                color: Some(Color(30, 20, 70, 255)),
                size: Some(15.),
            },
            value_format: ValueFormat::Fixed(1),
            empty_label: "NULL".into(),
            row_order: Some(if phase == 0 {
                vec!["North".into(), "South".into(), "West".into()]
            } else {
                vec!["West".into(), "South".into(), "North".into()]
            }),
            column_order: Some(if phase == 0 {
                vec![
                    "alpha".into(),
                    "beta".into(),
                    "gamma".into(),
                    "delta".into(),
                ]
            } else {
                vec![
                    "delta".into(),
                    "gamma".into(),
                    "beta".into(),
                    "alpha".into(),
                ]
            }),
            row_labels: [("North".into(), "North <safe>".into())].into(),
            column_labels: [("alpha".into(), "Alpha".into())].into(),
            hidden_rows: if hidden {
                ["North".into()].into()
            } else {
                Default::default()
            },
            axes: HeatmapAxes {
                top: Some(top),
                left: Some(HeatmapAxis::default()),
                right: None,
                bottom: None,
            },
            row_legend: true,
            hover_target: target,
            active,
            opacity: 0.8,
            active_opacity: 0.9,
            inactive_opacity: 0.17,
            ..Default::default()
        },
    )
}
fn record(events: &Rc<RefCell<Vec<Value>>>, value: Value) {
    let mut e = events.borrow_mut();
    if e.len() == 64 {
        e.remove(0);
    }
    e.push(value);
}
#[wasm_bindgen]
pub fn heatmap_mount(case: u8) -> std::result::Result<(), JsValue> {
    let initial = source(case, 0, HeatmapHoverTarget::RowColumn, None, false).map_err(js)?;
    heatmap_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("heatmap-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            initial.scene.clone(),
            BrowserOptions {
                renderer,
                highlight: browser::BrowserHighlight::Controlled,
                height: Some(420.),
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
            Ok(c) => c,
            Err(error) => {
                root.remove();
                heatmap_stop();
                return Err(js(error));
            }
        };
        let layout = Rc::new(RefCell::new(initial.clone()));
        let captured_layout = layout.clone();
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured_events = events.clone();
        let weak = chart.handle();
        let peers = Rc::new(RefCell::new(Peers::default()));
        chart.on_event(move|event|{
            record(&captured_events,event_value(event.clone()));
            let base=captured_layout.borrow().clone();
            let find=|id:&str|base.cells.iter().find(|c|c.mark_id==id&&c.visible).map(|c|c.key.clone());
            let active={let mut peers=peers.borrow_mut();match &event{
                BrowserEvent::Interaction(SurfaceEvent::Hover(ids))=>{peers.hover=ids.first().and_then(|id|find(id));if let Some(id)=ids.first()&&let Some(cell)=base.cells.iter().find(|c|&c.mark_id==id){record(&captured_events,json!({"type":"heatmap-current","id":id,"value":cell.value,"label":cell.label}));}},
                BrowserEvent::Focus(id)=>{peers.focus=id.as_deref().and_then(find);},_=>return,
            }peers.hover.clone().or_else(||peers.focus.clone())};
            if active==base.active{return;}
            match base.with_active(active).and_then(|next|{weak.set_scene(next.scene.clone())?;Ok(next)}){
                Ok(next)=>*captured_layout.borrow_mut()=next,
                Err(error)=>record(&captured_events,json!({"type":"layout-error","message":error.to_string()})),
            }
        });
        chart.flush().map_err(js)?;
        HEATMAPS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                case,
                phase: 0,
                layout,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn heatmap_phase(phase: u8) -> std::result::Result<(), JsValue> {
    HEATMAPS.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            let previous = f.layout.borrow().clone();
            let next = source(
                f.case,
                phase,
                previous.hover_target,
                previous.active,
                !previous.rows.iter().any(|r| r == "North"),
            )
            .map_err(js)?;
            f.chart.set_scene(next.scene.clone()).map_err(js)?;
            *f.layout.borrow_mut() = next;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn heatmap_target(target: u8) -> std::result::Result<(), JsValue> {
    let target = hover_target(target).map_err(js)?;
    HEATMAPS.with(|slot| {
        for f in slot.borrow().iter() {
            let mut next = f.layout.borrow().clone();
            next.hover_target = target;
            let active = next.active.clone();
            next.set_active(active).map_err(js)?;
            f.chart.set_scene(next.scene.clone()).map_err(js)?;
            *f.layout.borrow_mut() = next;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn heatmap_hide_row(hidden: bool) -> std::result::Result<(), JsValue> {
    HEATMAPS.with(|slot| {
        for f in slot.borrow().iter() {
            let old = f.layout.borrow().clone();
            let next = source(f.case, f.phase, old.hover_target, old.active, hidden).map_err(js)?;
            f.chart.set_scene(next.scene.clone()).map_err(js)?;
            *f.layout.borrow_mut() = next;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn heatmap_flush() -> std::result::Result<(), JsValue> {
    HEATMAPS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn heatmap_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    HEATMAPS.with(|slot|{
    let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let s=f.chart.snapshot().map_err(js)?;let layout=f.layout.borrow();
    let cell_value=|cell:&ComputedHeatmapCell|{let geometry=match &cell.geometry{Some(Geometry::Circle{x,y,radius})=>json!({"type":"circle","x":x,"y":y,"radius":radius}),Some(Geometry::Rect{x,y,width,height})=>json!({"type":"rect","x":x,"y":y,"width":width,"height":height}),Some(Geometry::Path(_))=>json!({"type":"rounded"}),None=>Value::Null};json!({"id":cell.mark_id,"row":cell.key.row,"column":cell.key.column,"value":cell.value,"formatted":cell.formatted_value,"label":cell.label,"center":cell.center,"band":cell.band.map(|r|[r.x,r.y,r.width,r.height]),"geometry":geometry,"color":[cell.color.0,cell.color.1,cell.color.2,cell.color.3],"opacity":cell.opacity,"visible":cell.visible})};
    let target=layout.cell(&HeatmapCellKey::new("South","gamma")).unwrap();let null=layout.cell(&HeatmapCellKey::new("North","beta")).unwrap();let zero=layout.cell(&HeatmapCellKey::new("North","alpha")).unwrap();
    Ok(json!({"active":s.active,"case":f.case,"phase":f.phase,"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"hovered":s.hovered,"selected":s.selected,"hidden":s.hidden,"activeIds":layout.active_ids,"hoverTarget":format!("{:?}",layout.hover_target),"rows":layout.rows,"columns":layout.columns,"target":cell_value(target),"null":cell_value(null),"zero":cell_value(zero),"cells":layout.cells.iter().map(cell_value).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn heatmap_rejected() -> bool {
    HEATMAPS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let mut bad = f.layout.borrow().scene.clone();
            bad.marks.last_mut().unwrap().opacity = f64::NAN;
            f.chart.set_scene(bad).is_err()
                && f.layout
                    .borrow()
                    .with_active(Some(HeatmapCellKey::new("unknown", "unknown")))
                    .is_err()
        })
    })
}
#[wasm_bindgen]
pub fn heatmap_stop() {
    let fixtures = HEATMAPS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in fixtures {
        f.chart.stop();
        f.root.remove();
    }
}
