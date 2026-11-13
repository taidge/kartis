use glory::render::{Error, EventKind};
use glory::style::{Length, Style};
use glory::{Cage, Widget, web::WebApp};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::prelude::*;

#[derive(Default)]
struct Counters {
    factories: Cell<usize>,
    cleanups: Cell<usize>,
    selections: Cell<usize>,
    rejects: RefCell<Vec<String>>,
}

struct Fixture {
    web: WebApp,
    scene: Cage<Scene>,
    layout: Cage<PlotWidgetsLayout>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Fixture>> = const { RefCell::new(None) };
    static COUNTERS: RefCell<Rc<Counters>> = RefCell::new(Rc::new(Counters::default()));
}

fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}

fn model(phase: u8) -> (Scene, PlotWidgetsLayout) {
    let width = match phase {
        0 => 260.,
        6 | 7 => 350.,
        _ => 320.,
    };
    let mut scene = ChartConfig {
        width,
        height: 155.,
        title: "Measured plot Widgets".into(),
        ..Default::default()
    }
    .scene()
    .expect("fixture dimensions");
    let mut layout = PlotWidgetsLayout {
        plot: Some(ClipRect {
            x: 4.,
            y: 4.,
            width: width - 8.,
            height: 147.,
        }),
        slots: Vec::new(),
    };
    let data = if phase == 0 {
        [("alpha", 73., 65.), ("beta", 192., 97.)]
    } else {
        [("beta", 234., 105.), ("alpha", 97., 78.)]
    };
    for (id, x, y) in data {
        if phase == 8 && id == "alpha" {
            continue;
        }
        let color = if id == "alpha" {
            Color(48, 119, 192, 255)
        } else {
            Color(211, 121, 48, 255)
        };
        let mut mark = Mark::new(id, Geometry::Circle { x, y, radius: 15. }, color);
        mark.series = id.into();
        mark.value = if id == "alpha" {
            if phase == 0 { 10. } else { 42. }
        } else {
            20.
        };
        if phase == 2 && id == "alpha" {
            mark.clip = Some(ClipRect {
                x: 70.,
                y: 12.,
                width: 112.,
                height: 120.,
            });
        }
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color,
        });
        let revision = if id == "alpha" {
            match phase {
                3 => 1,
                4 => 2,
                5..=7 => 3,
                _ => 0,
            }
        } else {
            0
        };
        layout.slots.push(PlotWidgetSlot {
            mark_id: id.into(),
            revision,
            placement: PlotWidgetPlacement::AtDatum {
                offset: [-46., -14.],
                measured: TextSize {
                    width: 92.,
                    height: 28.,
                },
            },
        });
    }
    if phase == 6 {
        layout.slots.push(layout.slots[0].clone());
    }
    (scene, layout)
}

fn log(counters: &Counters) -> String {
    let errors = counters.rejects.borrow();
    format!(
        "factory {}\ncleanup {}\nselection {}\n{}",
        counters.factories.get(),
        counters.cleanups.get(),
        counters.selections.get(),
        errors.join("\n")
    )
}

fn publish(counters: &Counters) {
    if let Some(status) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|document| document.get_element_by_id("plot-widgets-log"))
    {
        status.set_text_content(Some(&log(counters)));
    }
}

#[wasm_bindgen]
pub fn plot_widgets_mount() -> std::result::Result<(), JsValue> {
    plot_widgets_stop()?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|document| document.get_element_by_id("plot-widgets-app"))
        .ok_or_else(|| js("missing #plot-widgets-app"))?;
    let counters = Rc::new(Counters::default());
    COUNTERS.with(|slot| *slot.borrow_mut() = counters.clone());
    let mut sources = None;
    let web = glory::web::launch_with(root, |scope| {
        let (scene, layout) = model(0);
        let scene = scope.cage(scene);
        let layout = scope.cage(layout);
        sources = Some((scene, layout));
        let selects = counters.clone();
        let rejects = counters.clone();
        let factory = counters.clone();
        chart(scene)
            .on_select(move |_| {
                selects.selections.set(selects.selections.get() + 1);
                publish(&selects);
            })
            .on_reject(move |error| {
                rejects.rejects.borrow_mut().push(error.0);
                publish(&rejects);
            })
            .plot_widgets(layout, move |ctx| {
                factory.factories.set(factory.factories.get() + 1);
                publish(&factory);
                if ctx.revision == 1 {
                    return Err(ChartError("fixture slot factory refused revision 1".into()));
                }
                let owned = factory.clone();
                Ok(glory::ui::widget(move |scope| {
                    let cleanup = owned.clone();
                    scope.on_cleanup(move || {
                        cleanup.cleanups.set(cleanup.cleanups.get() + 1);
                        publish(&cleanup);
                    })?;
                    let presses = scope.cage(0_u32);
                    let frame = ctx.frame;
                    let revision = ctx.revision;
                    glory::ui::button()
                        .class(format!("plot-{}", ctx.mark_id))
                        .style(
                            Style::new()
                                .width(Length::Percent(100.))
                                .height(Length::Percent(100.)),
                        )
                        .label(scope.bond(move || {
                            frame.with(|frame| {
                                format!(
                                    "{} {} r{} ×{}",
                                    frame.mark.id,
                                    frame.mark.value,
                                    revision,
                                    presses.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| presses.revise(|n| *n += 1))
                        .show_in(scope)?;
                    if revision == 2 {
                        Err(
                            Error::InvalidProps("fixture slot child build refused revision 2")
                                .into(),
                        )
                    } else {
                        Ok(())
                    }
                }))
            })
    })
    .map_err(js)?;
    let (scene, layout) = sources.ok_or_else(|| js("fixture sources were not constructed"))?;
    ACTIVE.with(|slot| *slot.borrow_mut() = Some(Fixture { web, scene, layout }));
    Ok(())
}

#[wasm_bindgen]
pub fn plot_widgets_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 9 {
        return Err(js("unknown plot Widget phase"));
    }
    ACTIVE.with(|slot| {
        let active = slot.borrow();
        let fixture = active.as_ref().ok_or_else(|| js("fixture is stopped"))?;
        let (scene, layout) = model(phase);
        fixture.scene.revise(|old| *old = scene);
        fixture.layout.revise(|old| *old = layout);
        fixture.web.app().flush().map_err(js)
    })
}

#[wasm_bindgen]
pub fn plot_widgets_stop() -> std::result::Result<(), JsValue> {
    if let Some(fixture) = ACTIVE.with(|slot| slot.borrow_mut().take()) {
        fixture.web.app().clear().map_err(js)?;
    }
    Ok(())
}

#[wasm_bindgen]
pub fn plot_widgets_report() -> String {
    COUNTERS.with(|slot| log(&slot.borrow()))
}
