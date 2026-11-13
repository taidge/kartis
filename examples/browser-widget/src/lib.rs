//! Real Glory web host acceptance fixture, kept independent from Glory's CI.
#[cfg(target_arch = "wasm32")]
mod border_color_roles;
#[cfg(target_arch = "wasm32")]
mod box_colorby_hover;
#[cfg(target_arch = "wasm32")]
mod box_colors_function;
#[cfg(target_arch = "wasm32")]
mod box_legend_factory;
#[cfg(target_arch = "wasm32")]
mod box_legend_labels;
#[cfg(target_arch = "wasm32")]
mod box_parts_pointer;
#[cfg(target_arch = "wasm32")]
mod plot_widgets;
#[cfg(target_arch = "wasm32")]
mod radial_box_stroke_roles;

#[cfg(target_arch = "wasm32")]
mod browser {
    use glory::{Cage, web::WebApp};
    use kartis::*;
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeSet,
        rc::Rc,
    };
    use wasm_bindgen::prelude::*;
    struct WidgetRoot(web_sys::Element);
    impl Drop for WidgetRoot {
        fn drop(&mut self) {
            self.0.remove();
        }
    }
    struct Fixture {
        web: WebApp,
        scene: Cage<Scene>,
        hidden: Cage<BTreeSet<String>>,
        selected: Cage<Option<String>>,
        events: Rc<RefCell<Vec<String>>>,
        accept_hidden: Rc<Cell<bool>>,
        _mount_root: Option<WidgetRoot>,
    }
    thread_local! {static APP:RefCell<Option<Fixture>>=const{RefCell::new(None)};}
    fn js(error: impl std::fmt::Display) -> JsValue {
        error.to_string().into()
    }
    fn scene() -> Scene {
        let c = ChartConfig {
            width: 640.,
            height: 300.,
            title: "Glory chart interaction".into(),
            ..Default::default()
        };
        let mut s = c.scene().expect("fixture config");
        for (id, x, value, color) in [
            ("alpha", 10., 10., Color(65, 120, 185, 255)),
            ("beta", 260., 20., Color(232, 139, 48, 255)),
        ] {
            let mut mark = Mark::new(
                format!("datum-{id}"),
                Geometry::Rect {
                    x,
                    y: 35.,
                    width: 190.,
                    height: 190.,
                },
                color,
            );
            mark.series = id.into();
            mark.label = id.into();
            mark.value = value;
            mark.clip = Some(ClipRect {
                x: 30.,
                y: 30.,
                width: 560.,
                height: 220.,
            });
            s.marks.push(mark);
            s.legend.push(LegendItem {
                id: id.into(),
                label: id.into(),
                color,
            });
            s.label_series.insert(s.labels.len(), id.into());
            s.labels.push(Label {
                rotation: 0.,
                x: x + 95.,
                y: 270.,
                text: id.into(),
                color: Color(38, 50, 68, 255),
                size: 14.,
                anchor: TextAnchor::Middle,
            });
        }
        s
    }
    fn log(events: &Rc<RefCell<Vec<String>>>, message: String) {
        {
            let mut events = events.borrow_mut();
            if events.len() == 128 {
                events.remove(0);
            }
            events.push(message);
        }
        if let Some(element) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("events"))
        {
            element.set_text_content(Some(&events.borrow().join("\n")));
        }
    }
    #[wasm_bindgen(start)]
    pub fn start() -> std::result::Result<(), JsValue> {
        // Other example pages reuse this module with an intentionally hidden
        // default root. Their bootstrap mounts its own visible fixture after
        // init; ordinary Text must only be measured in a layout-visible host.
        let hidden = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("app"))
            .is_some_and(|root| root.has_attribute("hidden"));
        if hidden { Ok(()) } else { mount_fixture() }
    }
    #[wasm_bindgen]
    pub fn mount_fixture() -> std::result::Result<(), JsValue> {
        mount_mode(false)
    }
    #[wasm_bindgen]
    pub fn mount_outlined_fixture() -> std::result::Result<(), JsValue> {
        mount_mode(true)
    }
    fn dash_scene(mode: u8) -> Scene {
        use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
        let mut s = ChartConfig {
            width: 640.,
            height: 300.,
            title: "Semantic dash geometry".into(),
            ..Default::default()
        }
        .scene()
        .unwrap();
        let mut a = Mark::new(
            "dash-round",
            Geometry::Path(vec![C::Move(P::new(100., 80.)), C::Line(P::new(300., 80.))]),
            Color(255, 0, 0, 255),
        );
        a.fill = None;
        a.stroke = Some(GraphicStroke {
            color: Color(255, 0, 0, if mode == 2 { 128 } else { 255 }),
            width: 20.,
            cap: StrokeCap::Round,
        });
        a.stroke_dash = (mode != 3).then(|| StrokeDash {
            segments: vec![20., 0., 0., 30.],
            offset: if mode == 1 { 10. } else { 0. },
        });
        a.opacity = 0.5;
        a.series = "stroke".into();
        a.label = "Round stroke".into();
        a.value = if mode == 2 {
            999.
        } else if mode == 1 {
            41.
        } else {
            12.
        };
        let mut b = Mark::new(
            "dash-square",
            Geometry::Path(vec![
                C::Move(P::new(400., 60.)),
                C::Line(P::new(480., 140.)),
            ]),
            Color(0, 160, 70, 255),
        );
        b.fill = None;
        b.stroke = Some(GraphicStroke {
            color: Color(0, 160, 70, 255),
            width: 12.,
            cap: StrokeCap::Square,
        });
        b.stroke_dash = Some(StrokeDash {
            segments: vec![0., 28.],
            offset: 0.,
        });
        b.series = "stroke".into();
        b.label = "Square dots".into();
        s.marks = vec![a, b];
        s.legend.push(LegendItem {
            id: "stroke".into(),
            label: "Stroke".into(),
            color: Color(255, 0, 0, 255),
        });
        s
    }
    #[wasm_bindgen]
    pub fn mount_dashed_fixture() -> std::result::Result<(), JsValue> {
        mount_fixture()?;
        update_dashes(0)
    }
    #[wasm_bindgen]
    pub fn update_dashes(mode: u8) -> std::result::Result<(), JsValue> {
        if mode > 3 {
            return Err(js("invalid dash fixture mode"));
        }
        APP.with(|slot| {
            let stored = slot.borrow();
            let f = stored.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| *s = dash_scene(mode));
            f.web.app().flush().map_err(js)
        })
    }
    fn mount_mode(outlined: bool) -> std::result::Result<(), JsValue> {
        stop_fixture()?;
        let root = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("app"))
            .ok_or_else(|| js("missing #app"))?;
        root.set_inner_html("");
        root.remove_attribute("data-gly-app")?;
        // Keep the measuring line beside the claimed mount, under the same
        // application styles. Glory requires its own mount to start empty.
        let (mount_root, measurer) = if outlined {
            (None, None)
        } else {
            let document = root
                .owner_document()
                .ok_or_else(|| js("missing document"))?;
            let mount = document.create_element("div")?;
            mount.set_id("widget-root");
            root.append_child(&mount)?;
            let mount = WidgetRoot(mount);
            let parent = root.clone().dyn_into::<web_sys::HtmlElement>()?;
            let measurer =
                kartis::browser::BrowserSemanticTextMeasurer::new_in(&parent).map_err(js)?;
            (Some(mount), Some(measurer))
        };
        let host = mount_root
            .as_ref()
            .map_or_else(|| root.clone(), |mount| mount.0.clone());
        let events = Rc::new(RefCell::new(Vec::new()));
        let accept_hidden = Rc::new(Cell::new(true));
        let mut state = None;
        let web = glory::web::launch_with(host, |scope| {
            let mut initial = scene();
            if outlined {
                initial.labels[0].rotation = 35.;
                initial.labels[0].text = "AV fi O".into();
                initial.labels[1].rotation = -25.;
                initial.labels[1].text = "beta O".into();
            }
            let source = scope.cage(initial);
            let hidden = scope.cage(BTreeSet::new());
            let selected = scope.cage(None);
            state = Some((source, hidden, selected));
            let hidden_events = events.clone();
            let accept = accept_hidden.clone();
            let select_events = events.clone();
            let hover_events = events.clone();
            let focus_events = events.clone();
            let nav_events = events.clone();
            let reject_events = events.clone();
            let widget = chart(source)
                .hidden(hidden)
                .selected(selected)
                .on_hidden_change(move |proposal| {
                    log(
                        &hidden_events,
                        format!(
                            "hidden:{}",
                            proposal.iter().cloned().collect::<Vec<_>>().join(",")
                        ),
                    );
                    if accept.get() {
                        hidden.revise(|value| *value = proposal);
                    }
                })
                .on_selection_change(move |mark| {
                    selected.revise(|value| *value = mark.map(|mark| mark.id));
                })
                .on_select(move |mark| {
                    log(&select_events, format!("select:{}:{}", mark.id, mark.value))
                })
                .on_hover(move |mark| {
                    log(
                        &hover_events,
                        format!("hover:{}", mark.map_or_else(|| "none".into(), |m| m.id)),
                    )
                })
                .on_focus(move |mark| {
                    log(
                        &focus_events,
                        format!("focus:{}", mark.map_or_else(|| "none".into(), |m| m.id)),
                    )
                })
                .on_navigate(move |mark| log(&nav_events, format!("navigate:{}", mark.id)))
                .on_reject(move |error| log(&reject_events, format!("reject:{error}")));
            let widget = if let Some(measurer) = measurer {
                widget.semantic_text_measurer(SemanticTextProfile::default(), move |request| {
                    measurer.measure(request)
                })
            } else {
                widget
            };
            if outlined {
                let mut fonts = kartis::render::raster::FontRegistry::new();
                fonts
                    .register(
                        include_bytes!("../../../tests/fonts/Lato-Regular.ttf").as_slice(),
                        0,
                    )
                    .expect("explicit fixture font");
                widget.outline_labels(fonts, Default::default())
            } else {
                widget
            }
        })
        .map_err(js)?;
        let (scene, hidden, selected) = state.ok_or_else(|| js("fixture not constructed"))?;
        APP.with(|slot| {
            *slot.borrow_mut() = Some(Fixture {
                web,
                scene,
                hidden,
                selected,
                events,
                accept_hidden,
                _mount_root: mount_root,
            })
        });
        Ok(())
    }
    #[wasm_bindgen]
    pub fn update_outline(rotation: f64, text: &str) -> std::result::Result<(), JsValue> {
        APP.with(|slot| {
            let borrowed = slot.borrow();
            let f = borrowed.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| {
                s.labels[0].rotation = rotation;
                s.labels[0].text = text.into();
            });
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn flush_fixture() -> std::result::Result<(), JsValue> {
        APP.with(|slot| {
            if let Some(f) = slot.borrow().as_ref() {
                f.web.app().pump().map_err(js)?;
            }
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn stop_fixture() -> std::result::Result<(), JsValue> {
        let previous = APP.with(|slot| slot.borrow_mut().take());
        if let Some(f) = previous {
            f.web.app().clear().map_err(js)?;
            drop(f);
        }
        Ok(())
    }
    #[wasm_bindgen]
    pub fn update_alpha(value: f64) -> std::result::Result<(), JsValue> {
        if !value.is_finite() {
            return Err(js("value must be finite"));
        }
        APP.with(|slot| {
            let borrowed = slot.borrow();
            let f = borrowed.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| {
                if let Some(m) = s.marks.iter_mut().find(|m| m.id == "datum-alpha") {
                    m.value = value;
                }
            });
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn remove_alpha() -> std::result::Result<(), JsValue> {
        APP.with(|slot| {
            let borrowed = slot.borrow();
            let f = borrowed.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| {
                *s = s.without_series(&BTreeSet::from(["alpha".into()]));
                s.legend.retain(|i| i.id != "alpha");
            });
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn mixed_paint_order() -> std::result::Result<(), JsValue> {
        APP.with(|slot| {
            let borrowed = slot.borrow();
            let f = borrowed.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| {
                if s.marks.len() == 2 && s.labels.len() == 2 {
                    s.draw_order = vec![
                        DrawItem::Mark(0),
                        DrawItem::Label(0),
                        DrawItem::Mark(1),
                        DrawItem::Label(1),
                    ];
                }
            });
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn reject_scene() -> std::result::Result<(), JsValue> {
        APP.with(|slot| {
            let borrowed = slot.borrow();
            let f = borrowed.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| s.width = f64::NAN);
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn restore_scene() -> std::result::Result<(), JsValue> {
        APP.with(|slot| {
            let borrowed = slot.borrow();
            let f = borrowed.as_ref().ok_or_else(|| js("fixture stopped"))?;
            f.scene.revise(|s| *s = scene());
            f.hidden.revise(BTreeSet::clear);
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn accept_legend_proposals(accept: bool) {
        APP.with(|slot| {
            if let Some(f) = slot.borrow().as_ref() {
                f.accept_hidden.set(accept);
            }
        })
    }
    #[wasm_bindgen]
    pub fn selected_id() -> String {
        APP.with(|slot| {
            slot.borrow()
                .as_ref()
                .and_then(|f| f.selected.try_get().ok().flatten())
                .unwrap_or_default()
        })
    }
    #[wasm_bindgen]
    pub fn event_log() -> String {
        APP.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|f| f.events.borrow().join("\n"))
                .unwrap_or_default()
        })
    }
    #[wasm_bindgen]
    pub fn live_nodes() -> usize {
        APP.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or(0, |f| f.web.app().stats().nodes)
        })
    }
}
