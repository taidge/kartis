//! Standalone public low-level Canvas preparation acceptance fixture.
//! This module makes no BrowserTree/Engine transaction or Widget-host claim.
use web_sys::js_sys::{Function, Reflect};
use web_sys::{CanvasRenderingContext2d as Ctx, HtmlCanvasElement};

use super::*;

#[derive(Default)]
struct Observation {
    rows: Vec<Value>,
    contexts: Vec<Ctx>,
}
struct Fixture {
    accepted: Option<PreparedTreeCanvas>,
    retained: Vec<Ctx>,
    attempt: Value,
}
thread_local! {
    static FIXTURES: RefCell<BTreeMap<String, Fixture>> = const { RefCell::new(BTreeMap::new()) };
}
fn host(value: JsValue) -> ChartError {
    ChartError(format!("native fixture host call failed: {value:?}"))
}
fn color(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn raw(frame: &TreeCanvasFrame) -> Value {
    json!(frame.source().iter().map(|d|json!({"id":d.id,"parent":d.parent,"label":d.label,"value":d.value,"bits":d.value.to_bits().to_string()})).collect::<Vec<_>>())
}
fn metadata(frame: &TreeCanvasFrame) -> Value {
    let nodes = frame.layout().nodes.iter().map(|n|json!({"index":n.source_index,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"value":n.value,"bits":n.raw_value.to_bits().to_string(),"mark":n.mark_id,"color":n.color.map(color)})).collect::<Vec<_>>();
    let links = frame.layout().links.iter().map(|l|json!({"index":l.link_index,"source":l.source_index,"target":l.target_index,"sourcePosition":l.source_position,"targetPosition":l.target_position,"mark":l.mark_id})).collect::<Vec<_>>();
    let labels = (0..frame.labels().len()).map(|i| {
        let c=frame.label_context(i).unwrap();
        json!({"index":i,"owner":c.owner,"sourceIndex":c.source_index,"sceneIndex":c.scene_index,"eligible":c.eligible,"text":c.label.text,"position":[c.label.x,c.label.y],"rotation":c.label.rotation,"anchor":format!("{:?}",c.label.anchor),"size":c.label.size,"color":color(c.label.color),"font":c.font.canvas_font(c.label.size),"whitespace":format!("{:?}",c.whitespace),"clip":c.clip.map(|c|format!("{c:?}"))})
    }).collect::<Vec<_>>();
    json!({"raw":raw(frame),"nodes":nodes,"links":links,"labels":labels,"styles":frame.computed_link_styles().iter().map(|s|json!({"width":s.thickness,"color":color(s.color)})).collect::<Vec<_>>(),"qualification":format!("{:?}",frame.label_qualification()),"plot":[frame.layout().plot.x,frame.layout().plot.y,frame.layout().plot.width,frame.layout().plot.height]})
}
fn state(ctx: &Ctx) -> Result<Value> {
    let get = Reflect::get(ctx, &"getTransform".into())
        .map_err(host)?
        .dyn_into::<Function>()
        .map_err(host)?;
    let m = get.call0(ctx).map_err(host)?;
    let matrix = ["a", "b", "c", "d", "e", "f"].map(|k| {
        Reflect::get(&m, &k.into())
            .map_err(host)
            .map(|v| v.as_f64().unwrap())
    });
    let matrix = matrix.into_iter().collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"matrix":matrix,"alpha":ctx.global_alpha(),"composite":ctx.global_composite_operation().map_err(host)?,"fill":Reflect::get(ctx,&"fillStyle".into()).map_err(host)?.as_string(),"font":ctx.font(),"dash":ctx.get_line_dash().iter().map(|v|v.as_f64().unwrap()).collect::<Vec<_>>(),"shadow":[ctx.shadow_offset_x(),ctx.shadow_offset_y(),ctx.shadow_blur()],"size":ctx.canvas().map(|c|[c.width(),c.height()])}),
    )
}
fn record(log: &Rc<RefCell<Observation>>, ctx: &Ctx, role: String, data: Value) -> Result<()> {
    let native = state(ctx)?;
    let mut log = log.borrow_mut();
    let same = log.contexts.first().is_none_or(|first| {
        let a: &JsValue = first.as_ref();
        let b: &JsValue = ctx.as_ref();
        a == b
    });
    log.contexts.push(ctx.clone());
    log.rows
        .push(json!({"role":role,"sameContext":same,"state":native,"data":data}));
    Ok(())
}
fn frame(mode: u8) -> Result<TreeCanvasFrame> {
    let c = ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let source = [
        ("a", Some("r"), 0.),
        ("c", Some("a"), 30.),
        ("r", None, 0.),
        ("b", Some("r"), -0.),
        ("d", Some("a"), 10.),
    ]
    .into_iter()
    .map(|(id, parent, value)| HierarchyDatum {
        id: id.into(),
        parent: parent.map(Into::into),
        label: format!("source-{id}"),
        value,
    })
    .collect::<Vec<_>>();
    let mut o = TreeOptions {
        node_radius: 12.,
        ..Default::default()
    };
    o.style.labels.position = TreeLabelPosition::Center;
    o.style.labels.size = Some(12.);
    o.style.links.curve = TreeLinkCurve::Straight;
    o.style.links.width = 6.;
    o.style.links.color = TreeLinkColor::Fixed(Color(230, 110, 20, 255));
    o.style.nodes.fill = TreeNodeFill::Fixed(Color(100, 20, 180, 255));
    o.hierarchy.labels = mode != 2;
    o.style.links.enabled = mode != 2;
    let mut tree = tree_canvas_layout(&c, &source, &o)?;
    tree.scene.background = Color(255, 255, 255, 255);
    if mode == 1 {
        for mark in &mut tree.scene.marks {
            mark.stroke = None;
            mark.stroke_dash = None;
            match mark.id.as_str() {
                "node:r" => {
                    mark.geometry = Geometry::Path(vec![
                        PathCommand::Move(GraphicPoint::new(40., 40.)),
                        PathCommand::Line(GraphicPoint::new(80., 40.)),
                        PathCommand::Line(GraphicPoint::new(80., 70.)),
                        PathCommand::Line(GraphicPoint::new(40., 70.)),
                        PathCommand::Close,
                    ]);
                    mark.opacity = 0.5;
                    mark.stroke = Some(GraphicStroke {
                        color: Color(0, 100, 255, 255),
                        width: 10.,
                        cap: StrokeCap::Butt,
                    });
                }
                "node:a" => {
                    mark.geometry = Geometry::Path(vec![
                        PathCommand::Move(GraphicPoint::new(120., 40.)),
                        PathCommand::Line(GraphicPoint::new(140., 60.)),
                        PathCommand::Line(GraphicPoint::new(120., 80.)),
                        PathCommand::Line(GraphicPoint::new(100., 60.)),
                        PathCommand::Close,
                    ]);
                    mark.fill_paint = Some(FillPaint::Linear {
                        start: [100., 40.],
                        end: [140., 80.],
                        stops: vec![
                            GradientStop::new(0., Color(230, 110, 20, 255)),
                            GradientStop::new(1., Color(0, 200, 220, 255)),
                        ],
                    });
                    mark.stroke = Some(GraphicStroke {
                        color: Color(0, 0, 0, 255),
                        width: 3.,
                        cap: StrokeCap::Round,
                    });
                    mark.stroke_dash = Some(StrokeDash {
                        segments: vec![6., 3.],
                        offset: 1.,
                    });
                }
                "node:b" => {
                    mark.geometry = Geometry::Path(vec![
                        PathCommand::Move(GraphicPoint::new(200., 60.)),
                        PathCommand::Line(GraphicPoint::new(200., 60.)),
                    ]);
                    mark.fill = None;
                    mark.stroke = Some(GraphicStroke {
                        color: Color(0, 200, 0, 255),
                        width: 20.,
                        cap: StrokeCap::Square,
                    });
                    mark.opacity = 0.7;
                }
                "node:c" => {
                    // Offset geometry uses an explicit Path under the existing
                    // strict source binding. The authored cubic curve and all
                    // pixel expectations remain identical to the first run.
                    let (x, y, r) = (270., 60., 16.);
                    let k = r * 0.5522847498307936;
                    mark.geometry = Geometry::Path(vec![
                        PathCommand::Move(GraphicPoint::new(x + r, y)),
                        PathCommand::Cubic {
                            first: GraphicPoint::new(x + r, y + k),
                            second: GraphicPoint::new(x + k, y + r),
                            end: GraphicPoint::new(x, y + r),
                        },
                        PathCommand::Cubic {
                            first: GraphicPoint::new(x - k, y + r),
                            second: GraphicPoint::new(x - r, y + k),
                            end: GraphicPoint::new(x - r, y),
                        },
                        PathCommand::Cubic {
                            first: GraphicPoint::new(x - r, y - k),
                            second: GraphicPoint::new(x - k, y - r),
                            end: GraphicPoint::new(x, y - r),
                        },
                        PathCommand::Cubic {
                            first: GraphicPoint::new(x + k, y - r),
                            second: GraphicPoint::new(x + r, y - k),
                            end: GraphicPoint::new(x + r, y),
                        },
                        PathCommand::Close,
                    ]);
                    mark.fill_paint = Some(FillPaint::Pattern(PatternFill {
                        shape: PatternShape::Squares {
                            size: 4.,
                            stagger: false,
                        },
                        spacing: 8.,
                        foreground: Color(255, 220, 0, 255),
                        background: Color(100, 20, 180, 255),
                        ..Default::default()
                    }));
                }
                "node:d" => {
                    mark.geometry = Geometry::Path(vec![
                        PathCommand::Move(GraphicPoint::new(60., 130.)),
                        PathCommand::Line(GraphicPoint::new(110., 130.)),
                        PathCommand::Line(GraphicPoint::new(110., 160.)),
                        PathCommand::Line(GraphicPoint::new(60., 160.)),
                        PathCommand::Close,
                    ]);
                    mark.fill = Some(Color(0, 200, 0, 255));
                    mark.clip = Some(ClipRect {
                        x: 80.,
                        y: 130.,
                        width: 30.,
                        height: 30.,
                    });
                    mark.clip_path = Some(Box::new(ClipPath {
                        geometry: Geometry::Path(vec![
                            PathCommand::Move(GraphicPoint::new(70., 125.)),
                            PathCommand::Line(GraphicPoint::new(115., 125.)),
                            PathCommand::Line(GraphicPoint::new(115., 165.)),
                            PathCommand::Line(GraphicPoint::new(70., 165.)),
                            PathCommand::Close,
                            PathCommand::Move(GraphicPoint::new(90., 140.)),
                            PathCommand::Line(GraphicPoint::new(100., 140.)),
                            PathCommand::Line(GraphicPoint::new(100., 150.)),
                            PathCommand::Line(GraphicPoint::new(90., 150.)),
                            PathCommand::Close,
                        ]),
                        rule: ClipRule::EvenOdd,
                    }));
                }
                _ => {}
            }
        }
        for (i, l) in tree.scene.labels.iter_mut().enumerate() {
            l.text = if i == 0 {
                "R  T\nX".into()
            } else {
                String::new()
            };
            l.x = 210.;
            l.y = 180.;
            l.rotation = 30.;
            l.size = 13.;
            l.color = Color(0, 100, 255, 204);
        }
        tree.scene.typography.default = FontOptions {
            family: Some(vec![FontFamily::Mono]),
            weight: Some(400.),
        };
        for i in 0..tree.scene.labels.len() {
            tree.scene
                .typography
                .label_whitespace
                .insert(i, LabelWhitespace::Preserve);
        }
        TreeCanvasFrame::new(c, tree)
    } else {
        TreeCanvasFrame::from_all_computed(c, tree)
    }
}
fn stamp(ctx: &Ctx) -> Result<()> {
    ctx.set_fill_style_str("#e66e14");
    ctx.fill_rect(20., 200., 20., 12.);
    let document = ctx.canvas().and_then(|c| c.owner_document()).unwrap();
    let tile = document
        .create_element("canvas")
        .map_err(host)?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|e| host(e.into()))?;
    tile.set_width(2);
    tile.set_height(2);
    let tc = tile
        .get_context("2d")
        .map_err(host)?
        .unwrap()
        .dyn_into::<Ctx>()
        .map_err(|e| host(e.into()))?;
    tc.set_fill_style_str("#ffdc00");
    tc.fill_rect(0., 0., 1., 2.);
    tc.set_fill_style_str("#00c8dc");
    tc.fill_rect(1., 0., 1., 2.);
    ctx.set_image_smoothing_enabled(false);
    ctx.draw_image_with_html_canvas_element_and_dw_and_dh(&tile, 60., 200., 16., 12.)
        .map_err(host)?;
    let gradient = ctx.create_linear_gradient(220., 200., 280., 200.);
    gradient.add_color_stop(0., "#e66e14").map_err(host)?;
    gradient.add_color_stop(1., "#00c8dc").map_err(host)?;
    ctx.set_fill_style_canvas_gradient(&gradient);
    ctx.fill_rect(220., 200., 60., 14.);
    let pixels = [
        255, 0, 255, 255, 0, 255, 255, 255, 255, 220, 0, 255, 0, 200, 0, 255,
    ];
    let image =
        web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&pixels), 2, 2)
            .map_err(host)?;
    ctx.put_image_data(&image, 300., 210.).map_err(host)
}

#[wasm_bindgen]
pub fn tree_canvas_native_run(
    key: &str,
    mode: u8,
    ratio: f64,
    failure: u8,
) -> std::result::Result<String, JsValue> {
    if key.is_empty() || key.len() > 32 || mode > 2 || failure > 7 || failure == 7 && mode != 1 {
        return Err(js("invalid native Canvas fixture request"));
    }
    let frame = frame(mode).map_err(js)?;
    let layers = if mode == 1 {
        vec![
            TreeCanvasLayer::Custom("state".into()),
            TreeCanvasLayer::Nodes,
            TreeCanvasLayer::Labels,
            TreeCanvasLayer::Custom("tail".into()),
        ]
    } else {
        vec![
            TreeCanvasLayer::Custom("seed".into()),
            TreeCanvasLayer::Links,
            TreeCanvasLayer::Nodes,
            TreeCanvasLayer::Labels,
            TreeCanvasLayer::Nodes,
            TreeCanvasLayer::Custom("stamp".into()),
            TreeCanvasLayer::Custom("stamp".into()),
        ]
    };
    let budget = if failure == 7 {
        TreeCanvasBudget {
            live_pixels: 2 * (320. * ratio).ceil() as u64 * (240. * ratio).ceil() as u64,
            ..Default::default()
        }
    } else {
        TreeCanvasBudget::default()
    };
    let plan = TreeCanvasPlan::new(&frame, layers, budget).map_err(js)?;
    let log = Rc::new(RefCell::new(Observation::default()));
    let nl = log.clone();
    let ll = log.clone();
    let tl = log.clone();
    let sl = log.clone();
    let pl = log.clone();
    let dl = log.clone();
    let mut node = move |ctx: &Ctx, c: &TreeCanvasNodeContext<'_>| -> Result<()> {
        record(
            &nl,
            ctx,
            format!("node:{}", c.datum.id),
            json!({"index":c.source_index,"rawBits":c.datum.value.to_bits().to_string(),"label":c.datum.label,"position":c.node.position,"active":c.is_active}),
        )?;
        ctx.fill_rect(c.node.position[0] - 7., c.node.position[1] - 7., 14., 14.);
        if failure == 5 && c.datum.id == "a" {
            let canvas = ctx.canvas().unwrap();
            canvas.set_width(canvas.width() + 1);
        }
        if failure == 1 && c.datum.id == "a" {
            return Err(ChartError("fixture node:a rejected after paint".into()));
        }
        Ok(())
    };
    let mut link = move |ctx: &Ctx, c: &mut TreeCanvasLinkContext<'_>| -> Result<()> {
        let target = &c.frame.source()[c.link.target_index].id;
        record(
            &ll,
            ctx,
            format!("link:{target}"),
            json!({"index":c.link_index,"source":c.link.source_index,"target":c.link.target_index,"width":c.style.thickness,"color":color(c.style.color),"mark":c.mark.map(|m|m.id.as_str())}),
        )?;
        ctx.set_stroke_style_str("#e66e14");
        ctx.set_line_width(c.style.thickness);
        ctx.begin_path();
        if c.generator
            .generate(c.link.source_position, c.link.target_position)?
            .is_some()
        {
            return Err(ChartError("fixture generator was not bound".into()));
        }
        ctx.stroke();
        if failure == 2 && target == "c" {
            return Err(ChartError("fixture link:c rejected after paint".into()));
        }
        Ok(())
    };
    let mut label = move |ctx: &Ctx, c: &TreeCanvasLabelContext<'_>| -> Result<()> {
        let owner = c.owner.unwrap_or("unowned");
        record(
            &tl,
            ctx,
            format!("label:{owner}"),
            json!({"index":c.label_index,"sourceIndex":c.source_index,"sceneIndex":c.scene_index,"text":c.label.text,"position":[c.label.x,c.label.y],"eligible":c.eligible,"font":c.font.canvas_font(c.label.size)}),
        )?;
        ctx.set_fill_style_str("#0064ff");
        ctx.set_font("13px monospace");
        ctx.set_text_align("center");
        ctx.set_text_baseline("alphabetic");
        ctx.fill_text(&c.label.text, c.label.x, c.label.y)
            .map_err(host)?;
        if failure == 3 && owner == "b" {
            return Err(ChartError("fixture label:b rejected after paint".into()));
        }
        Ok(())
    };
    let mut seed = move |ctx: &Ctx, c: &mut TreeCanvasLayerContext<'_>| -> Result<()> {
        record(
            &sl,
            ctx,
            "custom:seed".into(),
            json!({"index":c.layer_index,"bound":c.generator.context().is_some(),"metadata":metadata(c.frame)}),
        )?;
        ctx.translate(3., 5.).map_err(host)?;
        ctx.set_global_alpha(0.75);
        ctx.set_fill_style_str("#6414b4");
        ctx.fill_rect(8., 8., 8., 8.);
        Ok(())
    };
    let mut image = move |ctx: &Ctx, c: &mut TreeCanvasLayerContext<'_>| -> Result<()> {
        record(
            &pl,
            ctx,
            "custom:stamp".into(),
            json!({"index":c.layer_index,"bound":c.generator.context().is_some(),"metadata":metadata(c.frame)}),
        )?;
        stamp(ctx)?;
        if failure == 4 {
            return Err(ChartError(
                "fixture custom:stamp rejected after paint".into(),
            ));
        }
        Ok(())
    };
    let mut defaults = move |ctx: &Ctx, c: &mut TreeCanvasLayerContext<'_>| -> Result<()> {
        record(
            &dl,
            ctx,
            format!(
                "custom:{}",
                if c.layer_index == 0 { "state" } else { "tail" }
            ),
            json!({"index":c.layer_index,"metadata":metadata(c.frame)}),
        )?;
        if c.layer_index == 0 {
            ctx.transform(1., 0.2, 0.1, 1., 7., 9.).map_err(host)?;
            ctx.set_global_alpha(0.6);
            ctx.set_global_composite_operation("multiply")
                .map_err(host)?;
            ctx.set_shadow_color("#00c8dc");
            ctx.set_shadow_offset_x(2.);
            ctx.set_shadow_offset_y(3.);
            let dash = web_sys::js_sys::Array::new();
            dash.push(&3.into());
            dash.push(&2.into());
            ctx.set_line_dash(&dash).map_err(host)?;
            if failure == 7 {
                ctx.scale(10., 10.).map_err(host)?;
            }
        }
        Ok(())
    };
    let mut callbacks = TreeCanvasCallbacks::default();
    if mode == 1 {
        callbacks
            .custom_layers
            .insert("state".into(), &mut defaults);
    } else {
        callbacks.render_node = Some(&mut node);
        callbacks.render_link = Some(&mut link);
        callbacks.render_label = Some(&mut label);
        callbacks.custom_layers.insert("seed".into(), &mut seed);
        callbacks.custom_layers.insert("stamp".into(), &mut image);
    }
    // Separate closure instances share observations; the executor still supplies
    // the same actual context across every occurrence.
    let tail = log.clone();
    let mut end = move |ctx: &Ctx, c: &mut TreeCanvasLayerContext<'_>| {
        record(
            &tail,
            ctx,
            "custom:tail".into(),
            json!({"index":c.layer_index,"metadata":metadata(c.frame)}),
        )
    };
    if mode == 1 {
        callbacks.custom_layers.insert("tail".into(), &mut end);
    }
    if failure == 6 {
        callbacks.custom_layers.remove("stamp");
    }
    let options = TreeCanvasOptions {
        pixel_ratio: ratio,
        ..Default::default()
    };
    let result = prepare_tree_canvas(
        &web_sys::window().unwrap().document().unwrap(),
        &frame,
        &plan,
        &options,
        &mut callbacks,
    );
    drop(callbacks);
    let observation = log.borrow();
    let attempt = json!({"mode":mode,"ratio":ratio,"failure":failure,"accepted":result.is_ok(),"error":result.as_ref().err().map(ToString::to_string),"metadata":metadata(&frame),"log":observation.rows,"plan":plan.layers().iter().map(|l|json!({"index":l.layer_index,"layer":format!("{:?}",l.layer),"roles":l.roles().iter().map(|r|format!("{r:?}")).collect::<Vec<_>>()})).collect::<Vec<_>>()});
    FIXTURES.with_borrow_mut(|all| {
        let f = all.entry(key.into()).or_insert_with(|| Fixture {
            accepted: None,
            retained: vec![],
            attempt: Value::Null,
        });
        if let Ok(prepared) = result {
            f.accepted = Some(prepared);
        }
        f.retained.extend(observation.contexts.iter().cloned());
        f.attempt = attempt;
    });
    tree_canvas_native_snapshot(key)
}
#[wasm_bindgen]
pub fn tree_canvas_native_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all|{let f=all.get(key).ok_or_else(||js("missing native Canvas fixture"))?;Ok(json!({"attempt":f.attempt,"accepted":f.accepted.as_ref().map(|a|Ok::<_,ChartError>(json!({"metadata":metadata(a.frame()),"size":a.backing_size(),"png":a.to_png_data_url()?}))).transpose().map_err(js)?,"retainedContexts":f.retained.len()}).to_string())})
}
#[wasm_bindgen]
pub fn tree_canvas_native_mutate_retained(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("missing native Canvas fixture"))?;
        for ctx in &f.retained {
            ctx.set_transform(1., 0., 0., 1., 0., 0.)?;
            ctx.set_global_alpha(1.);
            ctx.set_global_composite_operation("copy")?;
            ctx.set_fill_style_str("#ff00ff");
            ctx.fill_rect(0., 0., 1000., 1000.);
        }
        Ok::<_, JsValue>(())
    })?;
    tree_canvas_native_snapshot(key)
}
#[wasm_bindgen]
pub fn tree_canvas_native_stop(key: &str) -> bool {
    FIXTURES.with_borrow_mut(|all| all.remove(key).is_some())
}
