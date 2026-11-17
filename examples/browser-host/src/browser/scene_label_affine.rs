//! Fixed-source BrowserChart fixture for independent nonuniform CSS affine tests.
use glory::style::{Length, Position, Style};

use super::*;

type LiveFrame = Rc<RefCell<Option<glory::Bond<TypedSceneWidgetFrame<u32>>>>>;
struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    chart: BrowserChart,
    live: LiveFrame,
}
thread_local! {
    static FIXTURES: RefCell<BTreeMap<String, Fixture>> = const { RefCell::new(BTreeMap::new()) };
}
#[wasm_bindgen]
pub fn scene_label_affine_mount(
    key: &str,
    renderer: &str,
    ordered: bool,
) -> std::result::Result<(), JsValue> {
    if key.is_empty()
        || key.len() > 32
        || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err(js("invalid affine fixture key"));
    }
    let renderer = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("unknown affine renderer")),
    };
    scene_label_affine_stop(key)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("section")?;
    let id = format!("scene-label-affine-{key}");
    root.set_id(&id);
    root.set_attribute(
        "style",
        "width:320px;height:240px;position:relative;margin:12px 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!(
        "#{id} .kartis-browser,#{id} .kartis-browser-plot{{width:100%;height:100%}}\n#{id} .kartis-surface{{width:100%!important;height:100%!important}}\n#{id} .scene-label-affine-button{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:0px sans-serif;overflow:hidden}}"
    )));
    document.body().unwrap().append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let mut scene = ChartConfig {
        width: 320.,
        height: 240.,
        ..Default::default()
    }
    .scene()
    .map_err(js)?;
    scene.labels.push(Label {
        rotation: 30.,
        x: 120.,
        y: 100.,
        text: "source label".into(),
        color: Color(100, 20, 180, 255),
        size: 16.,
        anchor: TextAnchor::Middle,
    });
    let composition = SceneWidgetsComposition {
        widgets: SceneWidgetsData {
            layout: SceneWidgetsLayout {
                plot: None,
                slots: vec![SceneWidgetSlot {
                    key: "affine-label".into(),
                    target: SceneWidgetTarget::Label(0),
                    revision: 0,
                    placement: PlotWidgetPlacement::AtDatum {
                        offset: [-30., -12.],
                        measured: TextSize {
                            width: 60.,
                            height: 24.,
                        },
                    },
                }],
            },
            data: BTreeMap::from([("affine-label".into(), 7u32)]),
        },
        replace_ink: std::collections::BTreeSet::from(["affine-label".into()]),
    };
    let live = Rc::new(RefCell::new(None));
    let observed = live.clone();
    let factory = move |ctx: SceneWidgetCompositionContext<u32>| {
        *observed.borrow_mut() = Some(ctx.frame);
        Ok(glory::ui::button()
            .class("scene-label-affine-button")
            .label("affine label")
            .style(Style {
                background: Some(Color(100, 20, 180, 255)),
                color: Some(Color(100, 20, 180, 255)),
                ..Style::new()
                    .position(Position::Relative)
                    .width(Length::Percent(100.))
                    .height(Length::Percent(100.))
            })
            .fill(
                glory::ui::text()
                    .content("")
                    .class("scene-label-affine-dot")
                    .style(Style {
                        background: Some(Color(255, 220, 0, 255)),
                        ..Style::new()
                            .position(Position::Absolute)
                            .left(Length::Px(6.))
                            .top(Length::Px(4.))
                            .width(Length::Px(4.))
                            .height(Length::Px(4.))
                    }),
            ))
    };
    let options = BrowserOptions {
        renderer,
        height: Some(240.),
        motion: kartis::browser::MotionPreference::Reduced,
        ..Default::default()
    };
    let mounted = if ordered {
        BrowserChart::mount_with_scene_widgets_in_scene(
            root.clone(),
            scene,
            options,
            composition,
            factory,
        )
    } else {
        BrowserChart::mount_with_scene_widgets_composed(
            root.clone(),
            scene,
            options,
            composition,
            factory,
        )
    };
    let chart = match mounted {
        Ok(chart) => chart,
        Err(error) => {
            root.remove();
            css.remove();
            return Err(js(error));
        }
    };
    FIXTURES.with(|all| {
        all.borrow_mut().insert(
            key.into(),
            Fixture {
                root,
                css,
                chart,
                live,
            },
        );
    });
    Ok(())
}
#[wasm_bindgen]
pub fn scene_label_affine_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|all| {
        let all = all.borrow();
        let f = all.get(key).ok_or_else(|| js("unknown affine fixture"))?;
        let s = f.chart.snapshot().map_err(js)?;
        let bond = (*f.live.borrow()).ok_or_else(|| js("missing live affine frame"))?;
        let row = bond.try_get().map_err(js)?;
        let rect = row.frame.rect();
        Ok(json!({"rootId":f.root.id(),"scene":[s.width,s.height],"revision":s.revision,"pending":s.pending,"payload":row.data,"frame":{"rect":[rect.x,rect.y,rect.width,rect.height],"visible":row.frame.visible().map(|r|[r.x,r.y,r.width,r.height]),"interactive":row.frame.interactive(),"transform":row.frame.transform().map(|t|json!({"matrix":t.matrix,"measured":[t.measured.width,t.measured.height]}))}}).to_string())
    })
}
#[wasm_bindgen]
pub fn scene_label_affine_flush(key: &str) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|all| {
        all.borrow()
            .get(key)
            .ok_or_else(|| js("unknown affine fixture"))?
            .chart
            .flush()
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn scene_label_affine_stop(key: &str) -> std::result::Result<(), JsValue> {
    if let Some(f) = FIXTURES.with(|all| all.borrow_mut().remove(key)) {
        f.chart.stop();
        f.root.remove();
        f.css.remove();
    }
    Ok(())
}
