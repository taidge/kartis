//! Independent downstream Glory web PNG acceptance; no upstream CI dependency.
#[cfg(target_arch = "wasm32")]
mod color_fonts;
#[cfg(target_arch = "wasm32")]
mod raster_overlays;
#[cfg(target_arch = "wasm32")]
mod typed_surface;
#[cfg(target_arch = "wasm32")]
mod browser {
    use glory::{Cage, web::WebApp};
    use kartis::{render::raster::*, *};
    use serde_json::json;
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeSet,
        rc::Rc,
    };
    use wasm_bindgen::prelude::*;
    struct Fixture {
        web: WebApp,
        scene: Cage<Scene>,
        options: Cage<RasterOptions>,
        hidden: Cage<BTreeSet<String>>,
        events: Rc<RefCell<Vec<String>>>,
        fail: Rc<Cell<bool>>,
        publications: Rc<Cell<u32>>,
        drops: Rc<Cell<u32>>,
        vertical: bool,
        mixed: bool,
    }
    struct Capture(Rc<Cell<u32>>);
    impl Drop for Capture {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    thread_local! {static APP:RefCell<Option<Fixture>>=const{RefCell::new(None)};}
    fn js(error: impl std::fmt::Display) -> JsValue {
        error.to_string().into()
    }
    fn fonts(vertical: bool, mixed: bool) -> Result<FontRegistry> {
        let mut fonts = FontRegistry::new();
        let bytes = if vertical {
            fonts = fonts.with_writing_mode(RasterWritingMode::Vertical);
            include_bytes!("../../../tests/fonts/KartisFixtureVertical-Regular.ttf").as_slice()
        } else {
            include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf").as_slice()
        };
        fonts.register(bytes, 0)?;
        if mixed {
            fonts = fonts.with_vertical_layout(RasterVerticalLayout {
                orientation: RasterGlyphOrientation::Mixed,
                columns: RasterVerticalColumns::RightToLeft,
                column_advance_em: 1.5,
            });
        }
        Ok(fonts)
    }
    fn scene_profile(vertical: bool, mixed: bool) -> Scene {
        let mut scene = scene(vertical);
        if mixed {
            scene.labels[0].y = 30.;
            scene.labels[0].size = 40.;
            scene.labels[0].text = "日本AB\n（A）。".into();
        }
        scene
    }
    fn scene(vertical: bool) -> Scene {
        let mut s = ChartConfig {
            width: 400.,
            height: 220.,
            title: "Raster image chart".into(),
            margin: Margin {
                top: 10.,
                right: 10.,
                bottom: 10.,
                left: 10.,
            },
            ..Default::default()
        }
        .scene()
        .unwrap();
        for (id, x, color) in [
            ("alpha", 35., Color(220, 30, 40, 255)),
            ("beta", 230., Color(20, 160, 70, 255)),
        ] {
            let mut m = Mark::new(
                id,
                Geometry::Rect {
                    x,
                    y: 30.,
                    width: 120.,
                    height: 90.,
                },
                color,
            );
            m.series = id.into();
            m.label = id.into();
            m.value = 1.25;
            m.clip = Some(ClipRect {
                x: 15.,
                y: 15.,
                width: 370.,
                height: 180.,
            });
            s.marks.push(m);
            s.legend.push(LegendItem {
                id: id.into(),
                label: id.into(),
                color,
            });
            s.label_series.insert(s.labels.len(), id.into());
            s.labels.push(Label {
                x: x + 60.,
                y: 165.,
                text: format!("{id} fi"),
                color: Color(25, 35, 45, 255),
                size: 21.,
                anchor: TextAnchor::Middle,
                rotation: 12.,
            });
        }
        if vertical {
            s.label_series.clear();
            s.labels = vec![Label {
                x: 200.,
                y: 35.,
                text: "日本。".into(),
                color: Color(25, 35, 45, 255),
                size: 42.,
                anchor: TextAnchor::Start,
                rotation: 0.,
            }];
        } else {
            s.labels.push(Label {
                x: 200.,
                y: 205.,
                text: "CPU outlines: AV fi".into(),
                color: Color(25, 35, 45, 255),
                size: 18.,
                anchor: TextAnchor::Middle,
                rotation: 0.,
            });
        }
        s
    }
    fn record(events: &Rc<RefCell<Vec<String>>>, message: String) {
        let mut values = events.borrow_mut();
        if values.len() == 128 {
            values.remove(0);
        }
        values.push(message);
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("events"))
        {
            el.set_text_content(Some(&values.join("\n")));
        }
    }
    #[wasm_bindgen(start)]
    pub fn start() -> std::result::Result<(), JsValue> {
        mount_fixture()
    }
    #[wasm_bindgen]
    pub fn mount_fixture() -> std::result::Result<(), JsValue> {
        mount_mode(false)
    }
    #[wasm_bindgen]
    pub fn mount_vertical_fixture() -> std::result::Result<(), JsValue> {
        mount_mode(true)
    }
    #[wasm_bindgen]
    pub fn mount_mixed_vertical_fixture() -> std::result::Result<(), JsValue> {
        mount_profile(true, true)
    }
    #[wasm_bindgen]
    pub fn mixed_vertical_metrics() -> std::result::Result<String, JsValue> {
        let scene = scene_profile(true, true);
        let label = &scene.labels[0];
        let fonts = fonts(true, true).map_err(js)?;
        let metrics = fonts
            .measure_layout(&label.text, label.size, MissingGlyphPolicy::Error)
            .map_err(js)?;
        let bounds = fonts
            .measure_label(label, MissingGlyphPolicy::Error)
            .map_err(js)?
            .expect("fixture ink");
        Ok(json!({"advance":metrics.advance,"columns":metrics.columns.iter().map(|c|[c.x,c.advance]).collect::<Vec<_>>(),
            "ink":[bounds.x,bounds.y,bounds.width,bounds.height]}).to_string())
    }
    fn mount_mode(vertical: bool) -> std::result::Result<(), JsValue> {
        mount_profile(vertical, false)
    }
    fn mount_profile(vertical: bool, mixed: bool) -> std::result::Result<(), JsValue> {
        stop_fixture()?;
        let root = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("app"))
            .ok_or_else(|| js("missing app"))?;
        root.set_inner_html("");
        root.remove_attribute("data-gly-app")?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let fail = Rc::new(Cell::new(false));
        let publications = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let capture = Capture(drops.clone());
        let mut state = None;
        let web = glory::web::launch_with(root, |scope| {
            let scene = scope.cage(scene_profile(vertical, mixed));
            let options = scope.cage(RasterOptions::default());
            let hidden = scope.cage(BTreeSet::new());
            state = Some((scene, options, hidden));
            let errors = events.clone();
            let selected = events.clone();
            let hovered = events.clone();
            let proposals = events.clone();
            let failure = fail.clone();
            let count = publications.clone();
            raster_chart(
                scene,
                fonts(vertical, mixed).expect("explicit fixture font"),
                options,
            )
            .hidden(hidden)
            .tooltip_options(TooltipOptions {
                value_format: ValueFormat::Fixed(2),
                ..Default::default()
            })
            .raster_image_source(move |image| {
                let _ = &capture;
                count.set(count.get() + 1);
                if failure.get() {
                    return Err(ChartError("fixture PNG publication rejected".into()));
                }
                image.data_url()
            })
            .on_hidden_change(move |next| {
                record(
                    &proposals,
                    format!(
                        "hidden:{}",
                        next.iter().cloned().collect::<Vec<_>>().join(",")
                    ),
                );
                hidden.revise(|value| *value = next);
            })
            .on_select(move |m| record(&selected, format!("select:{}:{}", m.id, m.value)))
            .on_hover(move |m| {
                record(
                    &hovered,
                    format!("hover:{}", m.map_or_else(|| "none".into(), |m| m.id)),
                )
            })
            .on_reject(move |error| record(&errors, format!("reject:{error}")))
        })
        .map_err(js)?;
        let (scene, options, hidden) = state.ok_or_else(|| js("fixture construction"))?;
        APP.with(|slot| {
            *slot.borrow_mut() = Some(Fixture {
                web,
                scene,
                options,
                hidden,
                events,
                fail,
                publications,
                drops,
                vertical,
                mixed,
            })
        });
        Ok(())
    }
    fn with_fixture<T>(
        read: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
    ) -> std::result::Result<T, JsValue> {
        APP.with(|slot| {
            read(
                slot.borrow()
                    .as_ref()
                    .ok_or_else(|| js("fixture stopped"))?,
            )
        })
    }
    #[wasm_bindgen]
    pub fn update(value: f64, width: f64, ratio: f64) -> std::result::Result<(), JsValue> {
        with_fixture(|f| {
            f.scene.revise(|s| {
                s.width = width;
                s.marks[0].value = value;
                s.marks[0].fill = Some(Color(30, 60, 210, 255));
            });
            f.options.revise(|o| o.pixel_ratio = ratio);
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn fail_publication(fail: bool) -> std::result::Result<(), JsValue> {
        with_fixture(|f| {
            f.fail.set(fail);
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn restore() -> std::result::Result<(), JsValue> {
        with_fixture(|f| {
            f.fail.set(false);
            f.scene.revise(|s| *s = scene_profile(f.vertical, f.mixed));
            f.options.revise(|o| *o = RasterOptions::default());
            f.hidden.revise(BTreeSet::clear);
            f.web.app().flush().map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn snapshot() -> String {
        APP.with(|slot|{let s=slot.borrow();match s.as_ref(){Some(f)=>json!({"active":true,"nodes":f.web.app().stats().nodes,"handlers":f.web.app().stats().handlers,"publications":f.publications.get(),"drops":f.drops.get(),"events":f.events.borrow().clone()}).to_string(),None=>json!({"active":false,"nodes":0,"handlers":0}).to_string()}})
    }
    #[wasm_bindgen]
    pub fn clear_widget() -> std::result::Result<(), JsValue> {
        with_fixture(|f| f.web.app().clear().map_err(js))
    }
    #[wasm_bindgen]
    pub fn stop_fixture() -> std::result::Result<(), JsValue> {
        let old = APP.with(|slot| slot.borrow_mut().take());
        if let Some(f) = old {
            f.web.app().clear().map_err(js)?;
        }
        Ok(())
    }
}
