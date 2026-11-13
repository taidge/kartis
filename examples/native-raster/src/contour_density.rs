//! Headless Apace document input, explicit PNG decode and CPU paint.
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use apace_dom::Document;
use apace_traits::events::UiEvent;
use glory::Widget;
use glory::render::{NodeId, Props};
use glory_apace::assets::{Bytes, Url};
use glory_apace::shell::WindowApp;
use glory_apace::{Config, Raster};
use kartis::render::raster::*;
use kartis::*;
use keyboard_types::NamedKey;
use serde_json::json;

use super::{AnyResult, Provider, bounds, host_pointer, key, nodes, pointer, sample, save, settle};

struct Fixture {
    layout: SceneInteraction,
    ink: [f64; 2],
    blank: [f64; 2],
    threshold: String,
    raw: Option<(String, [f64; 2], [u8; 4])>,
}
fn fixture(density: bool, width: f64, value: f64) -> AnyResult<Fixture> {
    let config = ChartConfig {
        width,
        height: if density { 260. } else { 340. },
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let style = ContourLayoutOptions {
        colors: ColorScaleOptions {
            colors: vec![Color(10, 130, 140, 255); 2],
            ..Default::default()
        },
        ..Default::default()
    };
    if !density {
        let values = (0usize..225)
            .map(|i| {
                let (x, y) = (i % 15, i / 15);
                Some(if x.min(y).min(14 - x).min(14 - y) / 2 % 2 == 0 {
                    2.
                } else {
                    0.
                })
            })
            .collect::<Vec<_>>();
        let output = contour_layout(
            &config,
            ContourGrid {
                width: 15,
                height: 15,
                values: &values,
            },
            &ContourOptions {
                thresholds: ContourThresholds::Values(vec![value]),
                ..Default::default()
            },
            &style,
        )?;
        assert_eq!((output.levels[0].polygons, output.levels[0].holes), (2, 2));
        let plot = output.plot;
        Ok(Fixture {
            layout: output.scene_interaction(),
            ink: [plot.x + plot.width / 30., plot.y + plot.height / 2.],
            blank: [plot.x + plot.width / 6., plot.y + plot.height / 2.],
            threshold: output.binding.sites[0].mark_id.clone(),
            raw: None,
        })
    } else {
        let mut a = ScatterDatum::new("a", 35., 60.);
        a.color = Some(Color(220, 40, 30, 255));
        a.size_value = Some(999.);
        let mut b = ScatterDatum::new("b", 75., 30.);
        b.color = Some(Color(30, 80, 190, 255));
        let axis = || ScatterAxisScale::Continuous {
            options: ContinuousScaleOptions::default(),
            min: ScatterBound::Fixed(0.),
            max: ScatterBound::Fixed(100.),
        };
        let scatter = scatter_layout(
            &config,
            &[ScatterSeries::new("observations", vec![a, b])],
            &ScatterLayoutOptions {
                scales: ScatterScaleOptions {
                    x: axis(),
                    y: axis(),
                },
                axes: ScatterAxes {
                    top: None,
                    right: None,
                    bottom: None,
                    left: None,
                },
                node: ScatterNodeStyle {
                    shape: ScatterNodeShape::Circle {
                        diameter: ScatterSize::Fixed(6.),
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;
        let weights = [
            (("observations".into(), "a".into()), 2.),
            (("observations".into(), "b".into()), 0.65),
        ]
        .into();
        let output = scatter_density_layout(
            &config,
            &scatter,
            &weights,
            ScatterDensityOptions {
                bandwidth: 12.,
                cell_size: 4.,
                ..Default::default()
            },
            &DensityThresholds::Values(vec![value]),
            &style,
        )?;
        assert_eq!(output.observations[0].weight, 2.);
        let plot = output.plot;
        let position = [plot.x + plot.width * 0.35, plot.y + plot.height * 0.4];
        assert_eq!(scatter.sites[0].position, position);
        Ok(Fixture {
            layout: output.scene_interaction(),
            ink: [position[0] + 10., position[1]],
            blank: [plot.x + 4., plot.y + plot.height - 4.],
            threshold: output.levels[0]
                .mark_id
                .clone()
                .ok_or("density threshold is empty")?,
            raw: Some((
                scatter.sites[0].mark_id.clone(),
                position,
                [220, 40, 30, 255],
            )),
        })
    }
}
fn move_to(
    app: &mut WindowApp,
    provider: &Provider,
    handle: &RasterSurfaceHandle,
    point: [f64; 2],
) -> AnyResult {
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, image)?;
    let s = handle.snapshot()?;
    host_pointer(
        app,
        handle,
        "move",
        b.x + point[0] / s.width * b.width,
        b.y + point[1] / s.height * b.height,
    )?;
    settle(app, provider)?;
    Ok(())
}
fn control(app: &WindowApp, label: &str) -> AnyResult<NodeId> {
    nodes(app.app(),"kartis-datum").into_iter().find(|id|matches!(&app.app().tree().node(*id).unwrap().descriptor.view().props,Props::Button(button) if button.label.starts_with(label))).ok_or_else(||format!("missing typed datum control: {label}").into())
}
fn pixels(
    app: &WindowApp,
    handle: &RasterSurfaceHandle,
    fixture: &Fixture,
    dpr: f64,
) -> AnyResult<Raster> {
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, image)?;
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    let s = handle.snapshot()?;
    let at = |point: [f64; 2]| {
        sample(
            &raster,
            ((b.x + point[0] / s.width * b.width) * dpr).floor() as u32,
            ((b.y + point[1] / s.height * b.height) * dpr).floor() as u32,
        )
    };
    assert_eq!(
        at(fixture.ink),
        [10, 130, 140, 255],
        "actual Apace contour ink"
    );
    assert_eq!(
        at(fixture.blank),
        [255, 255, 255, 255],
        "actual Apace hole/background"
    );
    if let Some((_, point, color)) = &fixture.raw {
        assert_eq!(
            at(*point),
            *color,
            "source glyph must paint in front of density"
        );
    }
    Ok(raster)
}
pub(super) fn verify(output: &Path) -> AnyResult<serde_json::Value> {
    let mut cases = Vec::new();
    for density in [false, true] {
        for dpr in [1., 2.] {
            let first = fixture(density, 340., if density { 0.0005 } else { 1. })?;
            let provider = Arc::new(Provider::default());
            provider.0.lock().unwrap().images.insert(
                "asset://raster/contour-font.ttf".into(),
                Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
            );
            let mut app = WindowApp::with_config(Config {
                provider: Some(provider.clone()),
                base_url: Url::parse("asset://raster/")?,
                ..Default::default()
            });
            app.renderer()
                .set_viewport((740. * dpr) as u32, (900. * dpr) as u32, dpr as f32)?;
            app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/contour-font.ttf')}html,body{margin:0;background:white;font-family:Fixture}.contour-case{padding:24px}.kartis-data{display:flex;gap:8px;flex-wrap:wrap}button{padding:8px;border:1px solid #aaa;background:#eee}");
            let guard = Rc::new(RefCell::new(None::<RasterSurface>));
            let installed = guard.clone();
            let publish = provider.clone();
            let reject = Rc::new(Cell::new(false));
            let fail = reject.clone();
            let initial = first.layout.clone();
            app.mount(
                glory::ui::view()
                    .class("contour-case")
                    .fill(glory::ui::widget(move |scope| {
                        let publish = publish.clone();
                        let fail = fail.clone();
                        let surface = RasterSurface::mount_layout_published(
                            scope,
                            initial.clone(),
                            FontRegistry::new(),
                            RasterSurfaceOptions {
                                highlight: false,
                                raster: RasterOptions {
                                    pixel_ratio: dpr,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            move |image| {
                                if fail.get() {
                                    Err(ChartError("contour publisher rejected".into()))
                                } else {
                                    publish.publish(image)
                                }
                            },
                        )
                        .map_err(|_| glory::render::Error::InvalidProps("contour native mount"))?;
                        let widget = surface.widget();
                        *installed.borrow_mut() = Some(surface);
                        widget.show_in(scope)
                    })),
            )?;
            settle(&mut app, &provider)?;
            let handle = guard.borrow().as_ref().unwrap().handle();
            let events = Rc::new(RefCell::new(Vec::new()));
            let observed = events.clone();
            let token = Rc::new(());
            let weak = Rc::downgrade(&token);
            guard.borrow().as_ref().unwrap().on_event(move |batch| {
                let _keep = &token;
                observed.borrow_mut().extend(batch);
            })?;
            let image = nodes(app.app(), "kartis-raster-surface-image")[0];
            let b = bounds(&app, image)?;
            assert!(b.x >= 20. && b.y >= 20.);
            let physical = app.renderer().engine_id(image, glory_html::Part::Main)?;
            let decoded = app.renderer().inspect(|doc| {
                doc.get_node(physical)
                    .and_then(|n| n.element_data())
                    .and_then(|e| e.raster_image_data())
                    .map(|i| (i.width, i.height))
            });
            assert_eq!(
                decoded,
                Some((handle.frame()?.width, handle.frame()?.height))
            );
            let raster = pixels(&app, &handle, &first, dpr)?;
            let path = output.with_file_name(format!(
                "contour-density-{}-dpr{dpr}.png",
                if density { "density" } else { "grid" }
            ));
            save(&raster, &path)?;
            move_to(&mut app, &provider, &handle, first.blank)?;
            assert!(handle.snapshot()?.hovered.is_empty());
            move_to(&mut app, &provider, &handle, first.ink)?;
            assert_eq!(
                handle.snapshot()?.hovered,
                std::slice::from_ref(&first.threshold)
            );
            assert_eq!(
                handle.active_interaction()?.unwrap().rows[0].value,
                Some(if density { 0.0005 } else { 1. })
            );
            if let Some((id, point, _)) = &first.raw {
                move_to(&mut app, &provider, &handle, *point)?;
                assert_eq!(handle.snapshot()?.hovered, std::slice::from_ref(id));
                assert_eq!(
                    handle.active_interaction()?.unwrap().rows[0].value,
                    Some(60.)
                );
            }
            let site = first
                .layout
                .interaction
                .as_ref()
                .unwrap()
                .sites
                .iter()
                .find(|s| s.mark_id == first.threshold)
                .unwrap();
            let button = control(&app, &site.label)?;
            let b = bounds(&app, button)?;
            app.handle_ui_event(UiEvent::PointerDown(pointer(
                b.x + b.width / 2.,
                b.y + b.height / 2.,
                true,
            )));
            app.handle_ui_event(UiEvent::PointerUp(pointer(
                b.x + b.width / 2.,
                b.y + b.height / 2.,
                false,
            )));
            settle(&mut app, &provider)?;
            let before_enter = events.borrow().len();
            key(&mut app, NamedKey::Enter);
            settle(&mut app, &provider)?;
            assert_eq!(
                handle.snapshot()?.selected.as_deref(),
                Some(first.threshold.as_str())
            );
            assert!(events.borrow()[before_enter..].iter().any(
                |e| matches!(e,kartis::surface::SurfaceEvent::Activated(id) if id==&first.threshold)
            ));
            let next = fixture(density, 340., if density { 0.0008 } else { 1.5 })?;
            let before = handle.snapshot()?;
            let frame = handle.frame()?;
            let binding = handle.interaction_data()?;
            let requests = provider.0.lock().unwrap().fetched;
            let mut invalid = next.layout.clone();
            invalid.interaction.as_mut().unwrap().sites[0].value = Some(f64::NAN);
            assert!(handle.set_layout(invalid).is_err());
            assert_eq!(handle.snapshot()?, before);
            reject.set(true);
            assert!(handle.set_layout(next.layout.clone()).is_err());
            settle(&mut app, &provider)?;
            assert_eq!(handle.snapshot()?, before);
            assert!(Rc::ptr_eq(&frame, &handle.frame()?));
            assert_eq!(handle.interaction_data()?, binding);
            assert_eq!(provider.0.lock().unwrap().fetched, requests);
            reject.set(false);
            handle.set_layout(next.layout.clone())?;
            settle(&mut app, &provider)?;
            assert!(!nodes(app.app(), "kartis-datum").contains(&button));
            move_to(&mut app, &provider, &handle, next.ink)?;
            assert_eq!(
                handle.snapshot()?.hovered,
                std::slice::from_ref(&next.threshold)
            );
            let resized = fixture(density, 380., if density { 0.0008 } else { 1.5 })?;
            handle.set_layout(resized.layout.clone())?;
            settle(&mut app, &provider)?;
            assert_eq!(bounds(&app, image)?.width, 380.);
            assert_eq!(handle.frame()?.width, (380. * dpr) as u32);
            pixels(&app, &handle, &resized, dpr)?;
            move_to(&mut app, &provider, &handle, resized.ink)?;
            assert_eq!(
                handle.snapshot()?.hovered,
                std::slice::from_ref(&resized.threshold)
            );
            if let Some((id, point, _)) = &resized.raw {
                move_to(&mut app, &provider, &handle, *point)?;
                assert_eq!(handle.snapshot()?.hovered, std::slice::from_ref(id));
            }
            let fetched = provider.0.lock().unwrap().fetched;
            guard.borrow_mut().take();
            settle(&mut app, &provider)?;
            assert!(!handle.is_active());
            assert!(weak.upgrade().is_none());
            assert!(nodes(app.app(), "kartis-raster-surface-image").is_empty());
            assert!(nodes(app.app(), "kartis-datum").is_empty());
            assert_eq!(provider.0.lock().unwrap().fetched, fetched);
            app.app().clear()?;
            settle(&mut app, &provider)?;
            assert_eq!(app.app().stats().nodes, 0);
            assert_eq!(app.app().stats().handlers, 0);
            drop(app);
            let storage = provider.0.lock().unwrap();
            assert!(storage.closed >= 1 && storage.jobs.is_empty() && storage.images.is_empty());
            cases.push(json!({"kind":if density{"density"}else{"contour"},"dpr":dpr,"actualPngDecode":true,"actualCpuInkAndBlank":true,"sourceGlyphForeground":density,"typedHostPointer":true,"intrinsicKeyboardEnter":true,"invalidBindingRetained":true,"rejectedPublicationRetained":true,"acceptedUpdate":true,"resizeCurrentBinding":true,"guardCallbacksNodesAssetsClean":true,"decoded":decoded,"png":path.display().to_string()}));
        }
    }
    let report = json!({"passed":true,"headlessApaceDocument":true,"pointerBridge":"Apace UiEvent + image hit tree + Bounds + explicit dispatch_host","keyboardBridge":"intrinsic datum button and Apace KeyDown Enter","osWindowOrPhysicalDevice":false,"cases":cases});
    std::fs::write(
        output.with_file_name("contour-density-native.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(report)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "Provide KARTIS_CONTOUR_NATIVE_ARTIFACTS for real Apace output"]
    fn actual_contour_density() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("KARTIS_CONTOUR_NATIVE_ARTIFACTS")
                .expect("explicit native artifact directory"),
        );
        std::fs::create_dir_all(&directory).unwrap();
        super::verify(&directory.join("contour-density-focused.png")).unwrap();
    }
}
