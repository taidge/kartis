//! Actual SVG/Canvas consumers for borrowed Tree function configuration.
use super::*;
#[path = "../../../shared/tree_functions.rs"]
mod model;
struct Fixture {
    key: String,
    root: web_sys::Element,
    chart: BrowserChart,
    accepted: Value,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const {RefCell::new(Vec::new())};}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn metadata(source: &model::Source) -> Value {
    let l = &source.layout;
    json!({"raw":l.source().iter().map(|d|json!({"id":d.id,"label":d.label,"parent":d.parent,"value":d.value,"bits":d.value.to_bits().to_string()})).collect::<Vec<_>>(),"log":source.log,"plot":[l.plot.x,l.plot.y,l.plot.width,l.plot.height],"nodes":l.nodes.iter().map(|n|json!({"sourceIndex":n.source_index,"parentIndex":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"rawValue":n.raw_value,"rawBits":n.raw_value.to_bits().to_string(),"value":n.value,"position":n.position,"radius":n.radius,"color":n.color.map(rgba),"markId":n.mark_id})).collect::<Vec<_>>(),"links":l.links.iter().map(|n|json!({"linkIndex":n.link_index,"sourceIndex":n.source_index,"targetIndex":n.target_index,"sourcePosition":n.source_position,"targetPosition":n.target_position,"position":n.position,"markId":n.mark_id})).collect::<Vec<_>>(),"marks":l.scene.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"color":rgba(s.color),"width":s.width})),"opacity":m.opacity,"geometry":format!("{:?}",m.geometry)})).collect::<Vec<_>>(),"labels":l.scene.labels.iter().enumerate().map(|(i,l)|json!({"text":l.text,"position":[l.x,l.y],"color":rgba(l.color),"owner":source.layout.scene.label_series.get(&i)})).collect::<Vec<_>>(),"binding":tree_widget_frame(l).unwrap().interaction.unwrap().sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"label":s.label,"value":s.value,"color":rgba(s.color),"position":s.position})).collect::<Vec<_>>()})
}
#[wasm_bindgen]
pub fn tree_functions_mount(key: &str) -> std::result::Result<(), JsValue> {
    let renderer = match key {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("bad Tree function renderer")),
    };
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("div")?;
    root.set_id(&format!("tree-functions-{key}"));
    root.set_attribute(
        "style",
        "width:720px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    doc.body().unwrap().append_child(&root)?;
    let source = model::source(0, 720., 0).map_err(js)?;
    let accepted = metadata(&source);
    let frame = tree_widget_frame(&source.layout).map_err(js)?;
    let options = BrowserOptions {
        renderer,
        height: Some(320.),
        legend: false,
        interaction_data: frame.interaction,
        interaction: surface::SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        highlight: browser::BrowserHighlight::Controlled,
        motion: browser::MotionPreference::Reduced,
        ..Default::default()
    };
    let chart = match BrowserChart::mount(root.clone(), frame.scene, options) {
        Ok(c) => c,
        Err(e) => {
            root.remove();
            return Err(js(e));
        }
    };
    chart.flush().map_err(js)?;
    FIXTURES.with(|f| {
        f.borrow_mut().push(Fixture {
            key: key.into(),
            root,
            chart,
            accepted,
        })
    });
    Ok(())
}
fn with<T>(
    key: &str,
    f: impl FnOnce(&mut Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with(|all| {
        f(all
            .borrow_mut()
            .iter_mut()
            .find(|f| f.key == key)
            .ok_or_else(|| js("missing Tree function host"))?)
    })
}
#[wasm_bindgen]
pub fn tree_functions_set(
    key: &str,
    phase: u8,
    width: f64,
    failure: u8,
) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let source =
            model::source(phase, width, if failure < 100 { failure } else { 0 }).map_err(js)?;
        let accepted = metadata(&source);
        let mut frame = tree_widget_frame(&source.layout).map_err(js)?;
        match failure {
            101 => frame.scene.width = f64::NAN,
            102 => frame.interaction.as_mut().unwrap().sites[0].value = Some(991.),
            _ => {}
        }
        f.chart.set_layout(frame).map_err(js)?;
        f.chart.flush().map_err(js)?;
        f.accepted = accepted;
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_functions_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.chart.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_functions_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let s = f.chart.snapshot().map_err(js)?;
        Ok(json!({"accepted":f.accepted,"state":{"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"hovered":s.hovered,"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"interaction":s.interaction.as_ref().map(|i|i.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value})).collect::<Vec<_>>())}}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_functions_width(key: &str, width: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))
    })
}
#[wasm_bindgen]
pub fn tree_functions_zoom(key: &str, zoom: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.chart.handle().dispatch(SurfaceInput::Reset).map_err(js)?;
        if zoom {
            let s = f.chart.snapshot().map_err(js)?;
            f.chart
                .handle()
                .dispatch(SurfaceInput::Wheel {
                    point: (s.width / 2., s.height / 2.),
                    delta: -1.1f64.ln() / 0.002,
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_functions_stop(key: &str) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|all| {
        let mut all = all.borrow_mut();
        let i = all
            .iter()
            .position(|f| f.key == key)
            .ok_or_else(|| js("missing Tree function host"))?;
        let f = all.remove(i);
        f.chart.stop();
        f.root.remove();
        Ok(())
    })
}
