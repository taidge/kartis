//! Run the same reactive chart through two Glory desktop hosts.
use glory::{
    Widget,
    ui::{button, events::activate, text, view, widget},
};
use kartis::render::raster::{FontRegistry, RasterOptions};
use kartis::{ChartConfig, Geometry, Label, LegendItem, Mark, Scene, TextAnchor, chart};
use serde_json::{Value, json};
use std::{cell::RefCell, path::Path, rc::Rc};

type Report = Rc<RefCell<Vec<Value>>>;
type AnyResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CSS: &str = "
body { margin: 0; background: #eef2f7; color: #19253b; font: 16px sans-serif; }
.demo { margin: 18px; padding: 18px; background: white; }
.heading { display: block; font-size: 22px; font-weight: bold; margin-bottom: 10px; }
.status { display: block; margin: 12px 0; }
button { padding: 6px 12px; margin: 2px; }
button:focus { outline: 2px solid #1858a5; }
.kartis-legend, .kartis-data { margin: 8px 0; }
";

fn sales(values: [f64; 2]) -> kartis::Result<Scene> {
    let mut scene = ChartConfig {
        width: 520.,
        height: 172.,
        title: "Regional sales".into(),
        ..Default::default()
    }
    .scene()?;
    for (row, (region, value)) in ["North", "South"].into_iter().zip(values).enumerate() {
        let y = 34. + 70. * row as f64;
        let id = region.to_ascii_lowercase();
        let color = [
            kartis::Color(26, 101, 169, 255),
            kartis::Color(204, 84, 31, 255),
        ][row];
        let mut mark = Mark::new(
            &id,
            Geometry::Rect {
                x: 92.,
                y,
                width: value * 6.,
                height: 40.,
            },
            color,
        );
        mark.series = id.clone();
        mark.label = region.into();
        mark.value = value;
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id,
            label: region.into(),
            color,
        });
        scene.labels.push(Label {
            x: 18.,
            y: y + 25.,
            rotation: 0.,
            text: region.into(),
            color: kartis::Color(25, 37, 59, 255),
            size: 16.,
            anchor: TextAnchor::Start,
        });
    }
    scene.validate()?;
    Ok(scene)
}

fn app(initial: Scene, updated: Scene, report: Report, fonts: Option<FontRegistry>) -> impl Widget {
    widget(move |scope| {
        let scene = scope.cage(initial.clone());
        let status = scope.cage(String::from("Ready"));
        let select = report.clone();
        let hide = report.clone();
        let focus = report.clone();
        let reject = report.clone();
        let update = report.clone();
        let updated = updated.clone();
        let mut invalid = updated.clone();
        invalid.width = 0.;
        let widget = chart(scene);
        let widget = match &fonts {
            Some(fonts) => widget
                .raster(fonts.clone(), RasterOptions::default())
                .basic_data_controls(),
            None => widget,
        };

        view()
            .class("demo")
            .fill((
                text()
                    .class("heading")
                    .content("Regional sales · native chart"),
                button().label("Update values").on(activate, move |_| {
                    update.borrow_mut().push(json!({"kind":"update"}));
                    scene.revise(|value| *value = updated.clone());
                    status.revise(|value| *value = "Updated values".into());
                }),
                button()
                    .label("Submit invalid frame")
                    .on(activate, move |_| {
                        scene.revise(|value| *value = invalid.clone());
                    }),
                text().class("status").content(status),
                widget
                    .on_select(move |mark| {
                        status.revise(|value| {
                            *value = format!("Selected {} ({})", mark.label, mark.value)
                        });
                        select
                            .borrow_mut()
                            .push(json!({"kind":"select","id":mark.id,"value":mark.value}));
                    })
                    .on_hidden_change(move |hidden| {
                        status.revise(|value| {
                            *value = if hidden.is_empty() {
                                "All regions visible".into()
                            } else {
                                format!(
                                    "Hidden: {}",
                                    hidden.iter().cloned().collect::<Vec<_>>().join(", ")
                                )
                            }
                        });
                        hide.borrow_mut()
                            .push(json!({"kind":"hidden","ids":hidden}));
                    })
                    .on_focus(move |mark| {
                        focus
                            .borrow_mut()
                            .push(json!({"kind":"focus","id":mark.map(|mark| mark.id)}));
                    })
                    .on_reject(move |error| {
                        status.revise(|value| *value = "Invalid frame rejected".into());
                        reject
                            .borrow_mut()
                            .push(json!({"kind":"reject","message":error.to_string()}));
                    }),
            ))
            .show_in(scope)
    })
}

fn main() -> AnyResult {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [backend, output] = args.as_slice() else {
        return Err("usage: kartis-native-window <apace|wry> <report.json>".into());
    };
    let report = Report::default();
    let initial = sales([18., 32.])?;
    let updated = sales([38., 54.])?;
    match backend.to_str() {
        Some("apace") => {
            use glory_apace::shell::{Shell, WindowApp, WindowAttributes};
            let app = WindowApp::new();
            app.renderer().set_css(CSS);
            app.mount(self::app(initial, updated, report.clone(), None))?;
            let mut shell = Shell::new()?;
            shell.add_window(
                app,
                WindowAttributes::default().with_title("Kartis chart · Apace"),
            );
            shell.run()?;
        }
        #[cfg(feature = "wry-window")]
        Some("wry") => {
            use glory::wry::shell::{Config, Shell, WindowApp};
            let mut fonts = FontRegistry::new();
            fonts.register(
                include_bytes!("../../../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
                0,
            )?;
            let app = WindowApp::new();
            app.mount(self::app(initial, updated, report.clone(), Some(fonts)))?;
            Shell::new().run(
                app,
                Config {
                    title: "Kartis chart · Wry".into(),
                    width: 650.,
                    height: 530.,
                    css: CSS.into(),
                    ..Config::default()
                },
            )?;
        }
        #[cfg(not(feature = "wry-window"))]
        Some("wry") => return Err("enable the wry-window feature to use Wry".into()),
        _ => return Err("choose apace or wry".into()),
    }
    std::fs::write(
        Path::new(output),
        serde_json::to_vec_pretty(&json!({
            "backend":backend.to_str(), "events":&*report.borrow(),
        }))?,
    )?;
    Ok(())
}
