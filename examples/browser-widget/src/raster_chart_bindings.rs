//! Actual Glory WebApp CPU-image input. No application pointer event bridge.
#[path = "../../shared/raster_chart_bindings.rs"]
mod model;

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::render::{Event, EventKind, Props};
use glory::style::{Length, Style};
use glory::web::WebApp;
use glory::{Bond, Cage, Widget};
use kartis::render::raster::{FontRegistry, RasterOptions};
use kartis::surface::SurfaceFrameToken;
use kartis::*;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
struct Input {
    source: model::Source,
    options: RasterOptions,
    hidden: BTreeSet<String>,
    selected: Option<String>,
    phase: u8,
    view: u8,
}
#[derive(Default)]
struct Counters {
    publications: Cell<u32>,
    factories: Cell<u32>,
    cleanups: Cell<u32>,
    fail_next: Cell<bool>,
    events: RefCell<Vec<Value>>,
    last_token: RefCell<Option<SurfaceFrameToken>>,
}
type Probe = Rc<RefCell<Option<Bond<TypedPlotWidgetFrame<model::Data>>>>>;
struct Fixture {
    web: WebApp,
    input: Cage<Input>,
    counters: Rc<Counters>,
    probe: Probe,
    root: web_sys::Element,
    css: web_sys::Element,
}
thread_local! {static ACTIVE:RefCell<Option<Fixture>>=const{RefCell::new(None)};static RETIRED:RefCell<Option<Rc<Counters>>>=const{RefCell::new(None)};}
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}
fn log(counters: &Counters, event: Value) {
    let mut events = counters.events.borrow_mut();
    if events.len() >= 512 {
        events.remove(0);
    }
    events.push(event);
}
fn camera(view: u8) -> Result<ZoomTransform> {
    match view {
        0 => Ok(ZoomTransform::default()),
        1 => ZoomTransform::new(1.25, -12., -8.),
        2 => ZoomTransform::new(2., -40., -40.),
        3 => ZoomTransform::new(0.5, 40., 30.),
        _ => Err(ChartError("unknown fixture view".into())),
    }
}
fn next(phase: u8, view: u8, dpr: f64, hidden: bool) -> Result<Input> {
    let (value, shift) = match phase {
        0 => (None, 0.),
        1 => (Some(0.), 28.),
        2 => (Some(42.), 8.),
        3 => (None, 0.),
        _ => return Err(ChartError("unknown fixture phase".into())),
    };
    let mut source = model::source(value, shift, u32::from(phase), 0)?;
    if phase == 3 {
        source
            .frame
            .interaction
            .as_mut()
            .unwrap()
            .ink_owners
            .insert(model::ALIAS.into(), "missing-owner".into());
    }
    Ok(Input {
        source,
        options: RasterOptions {
            pixel_ratio: dpr,
            view: camera(view)?,
            ..Default::default()
        },
        hidden: if hidden {
            BTreeSet::from(["route".into()])
        } else {
            BTreeSet::new()
        },
        selected: if phase == 2 {
            Some("other".into())
        } else {
            None
        },
        phase,
        view,
    })
}

#[wasm_bindgen]
pub fn raster_bindings_mount(dpr: f64) -> std::result::Result<(), JsValue> {
    raster_bindings_stop()?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id("raster-chart-bindings");
    root.set_attribute("style", "width:240px;margin:12px 0;transform-origin:0 0")?;
    let css = document.create_element("style")?;
    css.set_text_content(Some("#raster-chart-bindings .bound-raster-widget{font:8px monospace!important;width:100%!important;height:100%!important;min-width:0!important;padding:0!important;border:0!important;border-radius:0!important;outline:none!important;box-shadow:none!important}"));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let counters = Rc::new(Counters::default());
    let probe: Probe = Rc::new(RefCell::new(None));
    let mut source = None;
    let web = glory::web::launch_with(root.clone(), |scope| {
        let input = scope.cage(next(0, 0, dpr, false).expect("fixture initial source"));
        source = Some(input);
        let frame = scope.bond(move || input.with(|p| p.source.frame.clone()));
        let widgets = scope.bond(move || input.with(|p| p.source.widgets.clone()));
        let options = scope.bond(move || input.with(|p| p.options.clone()));
        let hidden = scope.bond(move || input.with(|p| p.hidden.clone()));
        let selected = scope.bond(move || input.with(|p| p.selected.clone()));
        let (publications, rejected, physical, hover, selection, focus, factory) = (
            counters.clone(),
            counters.clone(),
            counters.clone(),
            counters.clone(),
            counters.clone(),
            counters.clone(),
            counters.clone(),
        );
        let capture = probe.clone();
        chart_layout(frame).raster(FontRegistry::new(),options).legend(false).highlight(false).hidden(hidden).selected(selected)
            .tooltip_options(TooltipOptions{missing:MissingValue::Placeholder("missing".into()),..Default::default()})
            .raster_image_source(move|image|{publications.publications.set(publications.publications.get()+1);if publications.fail_next.replace(false){log(&publications,json!({"event":"publication-rejected"}));Err(ChartError("fixture CPU image publisher refused".into()))}else{log(&publications,json!({"event":"published","width":image.width,"height":image.height,"dpr":image.pixel_ratio}));image.data_url()}})
            .on_reject(move|e|log(&rejected,json!({"event":"rejected","error":e.0})))
            .on_physical_hover(move|hit|{let same=hit.as_ref().and_then(|h|physical.last_token.borrow().as_ref().map(|old|old==&h.frame_token));if let Some(hit)=&hit{*physical.last_token.borrow_mut()=Some(hit.frame_token.clone());}log(&physical,json!({"event":"physical-hover","sameToken":same,"hit":hit.map(|h|json!({"physical":h.physical_mark_id,"owner":h.logical_site_id,"point":h.scene_point,"token":format!("{:?}",h.frame_token),"source":h.source_data.map(|d|json!({"id":d.source_id,"value":d.value}))}))}));})
            .on_hover(move|mark|log(&hover,json!({"event":"hover","id":mark.map(|m|m.id)})))
            .on_select(move|mark|log(&selection,json!({"event":"selected","id":mark.id})))
            .on_focus(move|mark|log(&focus,json!({"event":"focused","id":mark.map(|m|m.id)})))
            .plot_widgets_with_data(widgets,move|context|{
                factory.factories.set(factory.factories.get()+1);let frame=context.frame;*capture.borrow_mut()=Some(frame);let retired=factory.clone();let probe=capture.clone();
                Ok(glory::ui::widget(move|scope|{let retired=retired.clone();let probe=probe.clone();scope.on_cleanup(move||{retired.cleanups.set(retired.cleanups.get()+1);probe.borrow_mut().take();})?;
                    let count=scope.cage(0u32);let focus=scope.cage(false);
                    glory::ui::button().class("bound-raster-widget")
                        .style(Style{background:Some(Color(235,200,30,255)),..Style::new().width(Length::Percent(100.)).height(Length::Percent(100.))})
                        .label(scope.bond(move||frame.with(|f|format!("raw={:?}|note={}|live={}|count={}|focus={}",f.data.value,f.data.note,f.frame.mark.value,count.get(),focus.get()))))
                        .on(EventKind::Activate,move|_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move|event|{if let Event::FocusChanged(value)=event{focus.revise(|v|*v=value);}}).show_in(scope)
                }))
            })
    });
    let web = match web {
        Ok(web) => web,
        Err(error) => {
            root.remove();
            css.remove();
            return Err(js(error));
        }
    };
    let input = source.ok_or_else(|| js("fixture source was not installed"))?;
    ACTIVE.with(|slot| {
        *slot.borrow_mut() = Some(Fixture {
            web,
            input,
            counters,
            probe,
            root,
            css,
        })
    });
    raster_bindings_flush()
}

#[wasm_bindgen]
pub fn raster_bindings_update(
    phase: u8,
    view: u8,
    hidden: bool,
    fail: bool,
) -> std::result::Result<(), JsValue> {
    ACTIVE.with(|slot| {
        let slot = slot.borrow();
        let fixture = slot.as_ref().ok_or_else(|| js("fixture stopped"))?;
        let dpr = fixture.input.with(|p| p.options.pixel_ratio);
        let proposed = next(phase, view, dpr, hidden).map_err(js)?;
        if fail {
            fixture.counters.fail_next.set(true);
        }
        fixture.input.revise(|p| *p = proposed);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn raster_bindings_fail_next_publisher() -> std::result::Result<(), JsValue> {
    ACTIVE.with(|slot| {
        let slot = slot.borrow();
        slot.as_ref()
            .ok_or_else(|| js("fixture stopped"))?
            .counters
            .fail_next
            .set(true);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn raster_bindings_flush() -> std::result::Result<(), JsValue> {
    ACTIVE.with(|slot| {
        let slot = slot.borrow();
        let fixture = slot.as_ref().ok_or_else(|| js("fixture stopped"))?;
        fixture.web.app().flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn raster_bindings_state() -> std::result::Result<String, JsValue> {
    ACTIVE.with(|slot|{
    let slot=slot.borrow();let Some(f)=slot.as_ref()else{return Ok(RETIRED.with(|r|{let r=r.borrow();json!({"active":false,"cleanups":r.as_ref().map(|c|c.cleanups.get()),"events":r.as_ref().map(|c|c.events.borrow().clone())}).to_string()}));};
    let requested=f.input.get();let accepted=f.probe.borrow().as_ref().map(|p|p.try_get()).transpose().map_err(js)?;
    let accepted=accepted.map(|a|{let (x,y)=if let Geometry::Circle{x,y,..}=a.frame.mark.geometry{(x,y)}else{(f64::NAN,f64::NAN)};let scale=a.frame.rect.width/32.;json!({"value":a.data.value,"note":a.data.note,"live":a.frame.mark.value,"worldAnchor":[x,y],"view":[scale,a.frame.rect.x-(x-16.)*scale,a.frame.rect.y-26.*scale],"rect":[a.frame.rect.x,a.frame.rect.y,a.frame.rect.width,a.frame.rect.height],"visible":a.frame.visible.map(|r|[r.x,r.y,r.width,r.height]),"interactive":a.frame.interactive})});
    let tree=f.web.app().tree();let mut queue=tree.roots().to_vec();let mut image=None;let mut controls=vec![];let mut widget=None;let mut tooltip=vec![];let mut plots=0;
    while let Some(id)=queue.pop(){let node=tree.node(id).map_err(js)?;let descriptor=node.descriptor.view();let class=|name:&str|descriptor.classes.iter().any(|s|s==name);
        if class("kartis-raster-image")&&let Props::Image(p)=descriptor.props{image=Some(json!({"id":format!("{id:?}"),"source":p.source,"alt":p.alternative}));}
        if class("kartis-datum")&&let Props::Button(p)=descriptor.props{controls.push(json!({"id":format!("{id:?}"),"label":p.label,"pressed":descriptor.semantics.as_ref().and_then(|s|s.pressed)}));}
        if class("bound-raster-widget")&&let Props::Button(p)=descriptor.props{widget=Some(json!({"id":format!("{id:?}"),"label":p.label}));}
        if class("kartis-tooltip")&&let Props::Text(p)=descriptor.props{tooltip.push(p.content.clone());}
        if class("kartis-raster-plot"){plots+=1;}queue.extend(node.children.iter().copied());
    }
    Ok(json!({"active":true,"requested":{"phase":requested.phase,"view":requested.view,"hidden":requested.hidden,"selected":requested.selected,"dpr":requested.options.pixel_ratio},"accepted":accepted,"image":image,"controls":controls,"widget":widget,"tooltip":tooltip,"physicalPlots":plots,"publications":f.counters.publications.get(),"factories":f.counters.factories.get(),"cleanups":f.counters.cleanups.get(),"events":f.counters.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn raster_bindings_stop() -> std::result::Result<(), JsValue> {
    if let Some(f) = ACTIVE.with(|slot| slot.borrow_mut().take()) {
        f.web.app().clear().map_err(js)?;
        f.root.remove();
        f.css.remove();
        RETIRED.with(|slot| *slot.borrow_mut() = Some(f.counters));
    }
    Ok(())
}
