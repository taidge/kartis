//! Headless Apace PNG publication, decoded pixels and genuine host input.
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use apace_dom::Document;
use apace_traits::events::UiEvent;
use glory::Widget;
use glory::render::Props;
use glory_apace::assets::{Bytes, Url};
use glory_apace::shell::WindowApp;
use glory_apace::{Config, Raster};
use kartis::render::raster::*;
use kartis::*;
use keyboard_types::NamedKey;
use serde_json::{Value, json};

use super::conic_model::{Settings, Source, build};
use super::{AnyResult, Provider, bounds, host_pointer, key, nodes, pointer, sample, save, settle};

fn retained(
    handle: &RasterSurfaceHandle,
    before: &RasterSurfaceSnapshot,
    binding: &Option<InteractionBinding>,
    image: &Rc<RenderedImage>,
) -> AnyResult {
    assert_eq!(&handle.snapshot()?, before);
    assert_eq!(&handle.interaction_data()?, binding);
    assert!(Rc::ptr_eq(image, &handle.frame()?));
    Ok(())
}
fn station(source: &Source, missing: bool) -> &InteractionSite {
    source
        .published
        .interaction
        .as_ref()
        .unwrap()
        .sites
        .iter()
        .find(|s| s.mark_id.starts_with("geo-symbol:") && s.value.is_none() == missing)
        .expect("visible source station")
}
fn hover(
    app: &mut WindowApp,
    provider: &Provider,
    handle: &RasterSurfaceHandle,
    source: &Source,
    missing: bool,
) -> AnyResult<Value> {
    let site = station(source, missing);
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, image)?;
    let snapshot = handle.snapshot()?;
    let host = [
        (b.x + site.position[0] / snapshot.width * b.width) as f32 as f64,
        (b.y + site.position[1] / snapshot.height * b.height) as f32 as f64,
    ];
    host_pointer(app, handle, "move", host[0], host[1])?;
    settle(app, provider)?;
    let active = handle.active_interaction()?.ok_or("missing typed hover")?;
    assert_eq!(active.rows[0].mark_id, site.mark_id);
    assert_eq!(active.rows[0].value, site.value);
    // Inversion uses the real f32 Apace event coordinate, converted exactly once.
    let logical = [
        (host[0] as f32 as f64 - b.x) / b.width * snapshot.width,
        (host[1] as f32 as f64 - b.y) / b.height * snapshot.height,
    ];
    let inverse = source
        .context
        .invert(logical)?
        .ok_or("native inverse absent")?;
    let projected = source
        .context
        .project(inverse)?
        .ok_or("reprojection absent")?;
    let residual = (projected[0] - logical[0]).hypot(projected[1] - logical[1]);
    assert!(residual <= 1e-8, "actual native pointer reprojection");
    Ok(
        json!({"id":site.mark_id,"value":site.value,"host":host,"actualF32Logical":logical,
        "inverse":inverse,"reprojectionResidual":residual}),
    )
}
fn pixels(
    app: &WindowApp,
    handle: &RasterSurfaceHandle,
    source: &Source,
    dpr: f64,
    path: &Path,
) -> AnyResult<Value> {
    let id = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, id)?;
    let physical = app.renderer().engine_id(id, glory_html::Part::Main)?;
    let decoded = app.renderer().inspect(|doc| {
        doc.get_node(physical)
            .and_then(|n| n.element_data())
            .and_then(|e| e.raster_image_data())
            .map(|i| (i.width, i.height))
    });
    let frame = handle.frame()?;
    assert_eq!(decoded, Some((frame.width, frame.height)));
    assert_eq!(frame.width, (source.settings.width * dpr) as u32);
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    let mut witnesses = Vec::new();
    // Symbols have a solid interior and a source-backed shared color scale.
    for missing in [false, true] {
        let site = station(source, missing);
        let x = ((b.x + site.position[0]) * dpr).floor() as u32;
        let y = ((b.y + site.position[1]) * dpr).floor() as u32;
        let color = site.color;
        let expected = [color.0, color.1, color.2, color.3];
        let actual = sample(&raster, x, y);
        // Labels from nearby stations can cover the exact center after rotation.
        // Require solid pixels in a bounded interior neighborhood, with no color tolerance.
        let mut solid = Vec::new();
        for dy in -3_i32..=3 {
            for dx in -3_i32..=3 {
                if dx * dx + dy * dy > 9 {
                    continue;
                }
                let xx = (x as i32 + dx) as u32;
                let yy = (y as i32 + dy) as u32;
                let pixel = sample(&raster, xx, yy);
                if pixel == expected {
                    solid.push(json!({"xy":[xx,yy],"actual":pixel}));
                }
            }
        }
        assert!(
            solid.len() >= 3,
            "solid source-colored interior {}: {actual:?}",
            site.mark_id
        );
        witnesses.push(
            json!({"id":site.mark_id,"value":site.value,"centerXY":[x,y],
            "expected":expected,"centerActual":actual,"solidInterior":solid}),
        );
    }
    let masked = source
        .published
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("geo-symbol:") && m.clip_path.is_some())
        .count();
    assert!(masked >= 2);
    save(&raster, path)?;
    Ok(
        json!({"png":path,"decoded":decoded,"bounds":{"x":b.x,"y":b.y,"width":b.width,
        "height":b.height},"witnesses":witnesses,"maskedSymbols":masked}),
    )
}
pub(super) fn verify(output: &Path) -> AnyResult<Value> {
    let mut cases = Vec::new();
    for family in 0..4 {
        for fit_data in [false, true] {
            for dpr in [1., 2.] {
                let settings = Settings {
                    family,
                    fit_data,
                    parallels: match family {
                        2 => [30., 30.],
                        3 => [29.5, 45.5],
                        _ => [0., 60.],
                    },
                    ..Default::default()
                };
                let mut accepted = build(&settings)?;
                let provider = Arc::new(Provider::default());
                provider.0.lock().unwrap().images.insert(
                    "asset://raster/conic-font.ttf".into(),
                    Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
                );
                let mut app = WindowApp::with_config(Config {
                    provider: Some(provider.clone()),
                    base_url: Url::parse("asset://raster/")?,
                    ..Default::default()
                });
                app.renderer().set_viewport(
                    (850. * dpr) as u32,
                    (1000. * dpr) as u32,
                    dpr as f32,
                )?;
                app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/conic-font.ttf')}html,body{margin:0;background:white;font-family:Fixture}.conic-case{padding:24px}.kartis-data{display:flex;gap:8px;flex-wrap:wrap}button{padding:8px;border:1px solid #aaa;background:#eee}");
                let guard = Rc::new(RefCell::new(None::<RasterSurface>));
                let installed = guard.clone();
                let reject = Rc::new(Cell::new(false));
                let fail = reject.clone();
                let publish = provider.clone();
                let initial = accepted.published.clone();
                let mut fonts = FontRegistry::new();
                fonts.register(
                    include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
                    0,
                )?;
                app.mount(
                    glory::ui::view()
                        .class("conic-case")
                        .fill(glory::ui::widget(move |scope| {
                            let publish = publish.clone();
                            let fail = fail.clone();
                            let surface = RasterSurface::mount_layout_published(
                                scope,
                                initial.clone(),
                                fonts.clone(),
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
                                        Err(ChartError("conic publication rejected".into()))
                                    } else {
                                        publish.publish(image)
                                    }
                                },
                            )
                            .map_err(|error| {
                                eprintln!("conic native mount: {error}");
                                glory::render::Error::InvalidProps("conic native mount")
                            })?;
                            let widget = surface.widget();
                            *installed.borrow_mut() = Some(surface);
                            widget.show_in(scope)
                        })),
                )?;
                settle(&mut app, &provider)?;
                let handle = guard.borrow().as_ref().unwrap().handle();
                let events = Rc::new(RefCell::new(Vec::new()));
                let observe = events.clone();
                guard
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .on_event(move |batch| observe.borrow_mut().extend(batch))?;
                let prefix = format!(
                    "conic-{family}-{}-dpr{dpr}",
                    if fit_data { "data" } else { "world" }
                );
                let first_pixels = pixels(
                    &app,
                    &handle,
                    &accepted,
                    dpr,
                    &output.with_file_name(format!("{prefix}.png")),
                )?;
                let observed = hover(&mut app, &provider, &handle, &accepted, false)?;
                let missing = hover(&mut app, &provider, &handle, &accepted, true)?;
                let site = station(&accepted, false);
                let label = &accepted
                    .published
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == site.mark_id)
                    .unwrap()
                    .label;
                let button = nodes(app.app(), "kartis-datum")
                    .into_iter()
                    .find(|id| {
                        matches!(&app.app().tree().node(*id).unwrap().descriptor.view().props,
                Props::Button(b) if b.label.starts_with(label))
                    })
                    .ok_or("conic datum button absent")?;
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
                assert!(events.borrow()[before_enter..].iter().any(
                    |e| matches!(e,surface::SurfaceEvent::Activated(id) if id==&site.mark_id)
                ));
                let before = handle.snapshot()?;
                let image = handle.frame()?;
                let binding = handle.interaction_data()?;
                let old_inverse = accepted
                    .context
                    .invert(station(&accepted, false).position)?;
                let mut invalid_settings = settings.clone();
                invalid_settings.parallels = [90., 0.];
                assert!(build(&invalid_settings).is_err());
                retained(&handle, &before, &binding, &image)?;
                let mut invalid = accepted.published.clone();
                invalid.scene.width = f64::NAN;
                assert!(handle.set_layout(invalid).is_err());
                retained(&handle, &before, &binding, &image)?;
                let mut invalid = accepted.published.clone();
                invalid.interaction.as_mut().unwrap().sites[0].value = Some(f64::NAN);
                assert!(handle.set_layout(invalid).is_err());
                retained(&handle, &before, &binding, &image)?;
                let mut changed = settings.clone();
                changed.width = 700.;
                changed.center = [15., -5.];
                changed.roll = 18.;
                let next = build(&changed)?;
                reject.set(true);
                assert!(handle.set_layout(next.published.clone()).is_err());
                settle(&mut app, &provider)?;
                retained(&handle, &before, &binding, &image)?;
                assert_eq!(
                    accepted
                        .context
                        .invert(station(&accepted, false).position)?,
                    old_inverse
                );
                reject.set(false);
                handle.set_layout(next.published.clone())?;
                accepted = next;
                settle(&mut app, &provider)?;
                assert_eq!(handle.interaction_data()?, accepted.published.interaction);
                let resized_pixels = pixels(
                    &app,
                    &handle,
                    &accepted,
                    dpr,
                    &output.with_file_name(format!("{prefix}-resized.png")),
                )?;
                let resized_hover = hover(&mut app, &provider, &handle, &accepted, true)?;
                let mut hidden_settings = changed;
                hidden_settings.hidden = true;
                let hidden = build(&hidden_settings)?;
                handle.set_layout(hidden.published.clone())?;
                accepted = hidden;
                settle(&mut app, &provider)?;
                assert_eq!(handle.interaction_data()?, accepted.published.interaction);
                assert!(!accepted.symbols.iter().any(|s| s.id == "station-region-1"));
                let hidden_hover = hover(&mut app, &provider, &handle, &accepted, true)?;
                guard.borrow_mut().take();
                settle(&mut app, &provider)?;
                assert!(!handle.is_active());
                assert!(nodes(app.app(), "kartis-raster-surface-image").is_empty());
                app.app().clear()?;
                settle(&mut app, &provider)?;
                drop(app);
                assert!(provider.0.lock().unwrap().images.is_empty());
                cases.push(
                    json!({"family":family,"fitData":fit_data,"dpr":dpr,"initial":first_pixels,
            "resized":resized_pixels,"pointerObserved":observed,"pointerMissing":missing,
            "resizedPointer":resized_hover,"hiddenPointer":hidden_hover,"intrinsicEnter":true,
            "invalidAndResizePublicationRetainFrameBindingAndContext":true,"cleanup":true}),
                );
            }
        }
    }
    let report = json!({"passed":true,"headlessApaceDocument":true,"physicalWindowOrDevice":false,
        "pointerBridge":"real Apace UiEvent + hit tree + explicit image bounds",
        "cases":cases});
    std::fs::write(
        output.with_file_name("geo-conic-native.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "Requires explicit KARTIS_CONIC_NATIVE_ARTIFACTS directory"]
    fn actual_conic() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("KARTIS_CONIC_NATIVE_ARTIFACTS").expect("explicit artifact directory"),
        );
        std::fs::create_dir_all(&directory).unwrap();
        super::verify(&directory.join("geo-conic.png")).unwrap();
    }
}
