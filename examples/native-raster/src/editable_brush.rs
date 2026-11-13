//! Actual Apace drag-capture and intrinsic-button keyboard bridge. These tests
//! send real host UiEvents; no caller manually dispatches brush pointer input.
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use apace_dom::Document;
use apace_traits::events::UiEvent;
use glory::Widget;
use glory::render::{DragCapture, NodeId};
use glory_apace::assets::{Bytes, Url};
use glory_apace::shell::WindowApp;
use glory_apace::{Config, Raster};
use kartis::render::raster::*;
use kartis::surface::{SurfaceEvent, SurfaceInput};
use kartis::*;
use keyboard_types::NamedKey;

use super::{AnyResult, Provider, bounds, key, nodes, pointer, save, settle};

fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 360.,
        height: 260.,
        ..Default::default()
    }
    .scene()
    .expect("fixed native brush scene");
    for (id, x, y) in [("a", 35., 35.), ("b", 250., 170.)] {
        let mut m = Mark::new(
            id,
            Geometry::Rect {
                x,
                y,
                width: 30.,
                height: 30.,
            },
            Color(210, 40, 40, 255),
        );
        m.label = id.into();
        m.series = id.into();
        scene.marks.push(m);
    }
    scene.draw_order = vec![DrawItem::Mark(1), DrawItem::Mark(0)];
    scene
}
fn editor() -> EditableBrushOptions {
    EditableBrushOptions {
        extent: Brush {
            x0: 10.,
            y0: 20.,
            x1: 310.,
            y1: 220.,
        },
        handle_size: 10.,
        ..Default::default()
    }
}
fn base_selection() -> Brush {
    Brush {
        x0: 80.,
        y0: 70.,
        x1: 220.,
        y1: 170.,
    }
}
fn event(
    app: &mut WindowApp,
    provider: &Arc<Provider>,
    image: NodeId,
    phase: &str,
    point: (f64, f64),
) -> AnyResult {
    let b = bounds(app, image)?;
    let (x, y) = (
        b.x + point.0 / 360. * b.width,
        b.y + point.1 / 260. * b.height,
    );
    app.handle_ui_event(match phase {
        "down" => UiEvent::PointerDown(pointer(x, y, true)),
        "move" => UiEvent::PointerMove(pointer(x, y, true)),
        "up" => UiEvent::PointerUp(pointer(x, y, false)),
        _ => return Err("invalid actual brush pointer phase".into()),
    });
    settle(app, provider)
}
fn selection(handle: &RasterSurfaceHandle) -> AnyResult<Option<Brush>> {
    Ok(handle
        .editable_brush()?
        .ok_or("native brush editor missing")?
        .selection)
}
fn click(app: &mut WindowApp, provider: &Arc<Provider>, node: NodeId) -> AnyResult {
    let b = bounds(app, node)?;
    let (x, y) = (b.x + b.width / 2., b.y + b.height / 2.);
    app.handle_ui_event(UiEvent::PointerDown(pointer(x, y, true)));
    app.handle_ui_event(UiEvent::PointerUp(pointer(x, y, false)));
    settle(app, provider)
}
fn tab_to(app: &mut WindowApp, provider: &Arc<Provider>, node: NodeId) -> AnyResult {
    let physical = app.renderer().engine_id(node, glory_html::Part::Main)?;
    for _ in 0..80 {
        if app.renderer().inspect(|doc| doc.get_focussed_node_id()) == Some(physical) {
            return Ok(());
        }
        key(app, NamedKey::Tab);
        settle(app, provider)?;
    }
    Err("actual Tab navigation did not reach brush button".into())
}
fn blue(image: &RenderedImage) -> usize {
    image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[2] > p[0].saturating_add(70) && p[2] > p[1].saturating_add(50))
        .count()
}
fn css(padding: usize) -> String {
    format!(
        "@font-face{{font-family:BrushFixture;src:url('asset://raster/brush-font.ttf')}}html,body{{margin:0;background:white;font-family:BrushFixture}}.brush-case{{padding:{padding}px}}button{{padding:2px;font-family:BrushFixture;font-size:12px}}"
    )
}

pub(super) fn verify(output: &Path) -> AnyResult {
    let mut cases = Vec::new();
    for dpr in [1., 2.] {
        for scale in [1_f64, 2.] {
            let provider = Arc::new(Provider::default());
            provider.0.lock().unwrap().images.insert(
                "asset://raster/brush-font.ttf".into(),
                Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
            );
            let mut app = WindowApp::with_config(Config {
                provider: Some(provider.clone()),
                base_url: Url::parse("asset://raster/")?,
                ..Default::default()
            });
            app.renderer()
                .set_viewport((780. * dpr) as u32, (1000. * dpr) as u32, dpr as f32)?;
            app.renderer().set_css(&css(24));
            let guard = Rc::new(RefCell::new(None::<RasterSurface>));
            let installed = guard.clone();
            let publish = provider.clone();
            let reject = Rc::new(Cell::new(false));
            let publication = reject.clone();
            app.mount(
                glory::ui::view()
                    .class("brush-case")
                    .fill(glory::ui::widget(move |scope| {
                        let publish = publish.clone();
                        let publication = publication.clone();
                        let surface = RasterSurface::mount_published(
                            scope,
                            scene(),
                            FontRegistry::new(),
                            RasterSurfaceOptions {
                                editable_brush: Some(editor()),
                                raster: RasterOptions {
                                    pixel_ratio: dpr,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            move |image| {
                                if publication.get() {
                                    Err(ChartError("native brush publisher rejected".into()))
                                } else {
                                    publish.publish(image)
                                }
                            },
                        )
                        .map_err(|_| {
                            glory::render::Error::InvalidProps("native brush fixture mount")
                        })?;
                        let widget = surface.widget();
                        *installed.borrow_mut() = Some(surface);
                        widget.show_in(scope)
                    })),
            )?;
            settle(&mut app, &provider)?;
            let handle = guard.borrow().as_ref().unwrap().handle();
            let events = Rc::new(RefCell::new(Vec::<SurfaceEvent>::new()));
            let observed = events.clone();
            let replace = Rc::new(Cell::new(false));
            let replacement = replace.clone();
            let reenter = Rc::new(Cell::new(false));
            let new_gesture = reenter.clone();
            let replacement_session = Rc::new(RefCell::new(None::<BrushSession>));
            let reopened = replacement_session.clone();
            let nested = handle.clone();
            let captured = Rc::new(());
            let weak = Rc::downgrade(&captured);
            guard.borrow().as_ref().unwrap().on_event(move |batch| {
                let _ = &captured;
                let start = batch.iter().any(|e| {
                    matches!(
                        e,
                        SurfaceEvent::EditableBrush {
                            event: EditableBrushEvent {
                                phase: BrushPhase::Start,
                                source: BrushSource::User,
                                ..
                            },
                            ..
                        }
                    )
                });
                observed.borrow_mut().extend(batch);
                if start && replacement.replace(false) {
                    nested
                        .set_brush_selection(Some(Brush {
                            x0: 40.,
                            y0: 40.,
                            x1: 130.,
                            y1: 110.,
                        }))
                        .unwrap();
                    if new_gesture.replace(false) {
                        nested
                            .dispatch_editable_brush(EditableBrushInput::Begin {
                                id: 1,
                                point: (80., 80.),
                                target: BrushTarget::Selection,
                                modifiers: Default::default(),
                            })
                            .unwrap();
                        *reopened.borrow_mut() = nested.editable_brush().unwrap().unwrap().session;
                    }
                }
            })?;
            let failures = Rc::new(RefCell::new(Vec::<String>::new()));
            let errors = failures.clone();
            guard.borrow().as_ref().unwrap().on_error(move |e| {
                errors.borrow_mut().push(e.0);
            })?;
            let image = nodes(app.app(), "kartis-raster-surface-image")[0];
            let b = bounds(&app, image)?;
            assert!(b.x >= 24. && b.y >= 24.);
            assert_eq!((b.width, b.height), (360., 260.));
            if scale > 1. {
                handle.dispatch(SurfaceInput::Wheel {
                    point: (50., 50.),
                    delta: -scale.ln() / 0.002,
                })?;
                settle(&mut app, &provider)?;
            }
            assert!((handle.snapshot()?.view.scale - scale).abs() < 1e-12);
            event(&mut app, &provider, image, "down", (30., 40.))?;
            assert!(
                handle.editable_brush()?.unwrap().active,
                "real native press did not begin controller"
            );
            event(&mut app, &provider, image, "move", (170., 120.))?;
            event(&mut app, &provider, image, "up", (170., 120.))?;
            assert_eq!(
                selection(&handle)?,
                Some(Brush {
                    x0: 30.,
                    y0: 40.,
                    x1: 170.,
                    y1: 120.
                })
            );
            assert!(!handle.editable_brush()?.unwrap().active);
            let ids = handle.editable_brush()?.unwrap().ids;
            assert_eq!(ids, ["a"]);
            handle.set_hidden(BTreeSet::from(["a".into()]))?;
            assert!(handle.editable_brush()?.unwrap().ids.is_empty());
            handle.set_hidden(BTreeSet::new())?;
            assert_eq!(handle.editable_brush()?.unwrap().ids, ["a"]);
            let positions = [
                (BrushHandle::N, (150., 70.), (150., 35.)),
                (BrushHandle::W, (80., 120.), (30., 120.)),
                (BrushHandle::E, (220., 120.), (290., 120.)),
                (BrushHandle::S, (150., 170.), (150., 205.)),
                (BrushHandle::Nw, (80., 70.), (35., 40.)),
                (BrushHandle::Ne, (220., 70.), (285., 40.)),
                (BrushHandle::Sw, (80., 170.), (35., 205.)),
                (BrushHandle::Se, (220., 170.), (285., 205.)),
            ];
            let mut resized = Vec::new();
            for (edge, start, end) in positions {
                handle.set_brush_selection(Some(base_selection()))?;
                settle(&mut app, &provider)?;
                let node = nodes(
                    app.app(),
                    &format!("kartis-raster-brush-handle-{}", edge.name()),
                )[0];
                let tree = app.app().tree();
                let desc = tree.node(node)?.descriptor.view();
                assert_eq!(
                    desc.semantics.as_ref().unwrap().drag_capture,
                    Some(DragCapture::PressFocus)
                );
                assert!(desc.name.as_ref().is_some_and(|s| !s.is_empty()));
                drop(tree);
                event(&mut app, &provider, image, "down", start)?;
                event(&mut app, &provider, image, "move", end)?;
                event(&mut app, &provider, image, "up", end)?;
                let expected = match edge {
                    BrushHandle::N => Brush {
                        y0: 35.,
                        ..base_selection()
                    },
                    BrushHandle::W => Brush {
                        x0: 30.,
                        ..base_selection()
                    },
                    BrushHandle::E => Brush {
                        x1: 290.,
                        ..base_selection()
                    },
                    BrushHandle::S => Brush {
                        y1: 205.,
                        ..base_selection()
                    },
                    BrushHandle::Nw => Brush {
                        x0: 35.,
                        y0: 40.,
                        ..base_selection()
                    },
                    BrushHandle::Ne => Brush {
                        x1: 285.,
                        y0: 40.,
                        ..base_selection()
                    },
                    BrushHandle::Sw => Brush {
                        x0: 35.,
                        y1: 205.,
                        ..base_selection()
                    },
                    BrushHandle::Se => Brush {
                        x1: 285.,
                        y1: 205.,
                        ..base_selection()
                    },
                };
                assert_eq!(
                    selection(&handle)?,
                    Some(expected),
                    "actual native {} handle",
                    edge.name()
                );
                resized.push(edge.name());
            }
            let target = nodes(app.app(), "kartis-raster-brush-target-selection")[0];
            click(&mut app, &provider, target)?;
            let right = nodes(app.app(), "kartis-raster-brush-nudge-right")[0];
            tab_to(&mut app, &provider, right)?;
            let before = selection(&handle)?.unwrap();
            key(&mut app, NamedKey::Enter);
            settle(&mut app, &provider)?;
            assert_eq!(
                selection(&handle)?,
                Some(Brush {
                    x0: before.x0 + 1.,
                    x1: before.x1 + 1.,
                    ..before
                })
            );
            let accepted = selection(&handle)?;
            event(&mut app, &provider, image, "down", (150., 120.))?;
            event(&mut app, &provider, image, "move", (165., 130.))?;
            key(&mut app, NamedKey::Tab);
            settle(&mut app, &provider)?;
            assert_eq!(selection(&handle)?, accepted);
            assert!(!handle.editable_brush()?.unwrap().active);
            event(&mut app, &provider, image, "up", (165., 130.))?;
            // Hidden-series updates retire a currently captured session, not
            // merely the ids of an idle selection.
            event(&mut app, &provider, image, "down", (150., 120.))?;
            event(&mut app, &provider, image, "move", (165., 130.))?;
            assert!(handle.editable_brush()?.unwrap().active);
            handle.set_hidden(BTreeSet::from(["a".into()]))?;
            settle(&mut app, &provider)?;
            assert!(!handle.editable_brush()?.unwrap().active);
            assert_eq!(selection(&handle)?, accepted);
            event(&mut app, &provider, image, "move", (180., 140.))?;
            event(&mut app, &provider, image, "up", (180., 140.))?;
            assert_eq!(selection(&handle)?, accepted);
            handle.set_hidden(BTreeSet::new())?;
            settle(&mut app, &provider)?;
            // A physical image-box change cancels the original baseline instead
            // of mapping an old press into a new origin.
            event(&mut app, &provider, image, "down", (150., 120.))?;
            event(&mut app, &provider, image, "move", (160., 125.))?;
            app.renderer().set_css(&css(36));
            settle(&mut app, &provider)?;
            assert_eq!(bounds(&app, image)?.x, 36.);
            event(&mut app, &provider, image, "move", (170., 130.))?;
            assert!(!handle.editable_brush()?.unwrap().active);
            assert_eq!(selection(&handle)?, accepted);
            event(&mut app, &provider, image, "up", (170., 130.))?;
            app.renderer().set_css(&css(24));
            settle(&mut app, &provider)?;
            // A genuine Start callback replaces the program selection. Subsequent
            // physical capture events must not resurrect its retired baseline.
            replace.set(true);
            event(&mut app, &provider, image, "down", (200., 210.))?;
            let replaced = selection(&handle)?;
            assert_eq!(
                replaced,
                Some(Brush {
                    x0: 40.,
                    y0: 40.,
                    x1: 130.,
                    y1: 110.
                })
            );
            event(&mut app, &provider, image, "move", (270., 210.))?;
            event(&mut app, &provider, image, "up", (270., 210.))?;
            assert_eq!(selection(&handle)?, replaced);
            assert!(!handle.editable_brush()?.unwrap().active);
            // Stronger observer reentry: replace the old session and explicitly
            // open a new one. Old physical Move/Drop may not acquire session 2.
            reenter.set(true);
            replace.set(true);
            event(&mut app, &provider, image, "down", (200., 210.))?;
            let fresh = replacement_session
                .borrow()
                .ok_or("replacement Start did not open a new session")?;
            assert_eq!(handle.editable_brush()?.unwrap().session, Some(fresh));
            let before_old_transport = selection(&handle)?;
            event(&mut app, &provider, image, "move", (270., 210.))?;
            event(&mut app, &provider, image, "up", (270., 210.))?;
            assert_eq!(selection(&handle)?, before_old_transport);
            assert_eq!(handle.editable_brush()?.unwrap().session, Some(fresh));
            handle.dispatch_editable_brush(EditableBrushInput::Cancel {
                session: fresh,
                id: 1,
            })?;
            assert!(blue(handle.frame()?.as_ref()) > 150 * (dpr * dpr) as usize);
            // Explicit publisher rejection is verified against the same native
            // instance; no new source, session, revision or pixels may leak.
            handle.dispatch_editable_brush(EditableBrushInput::Begin {
                id: 9,
                point: (130., 110.),
                target: BrushTarget::Handle(BrushHandle::Se),
                modifiers: Default::default(),
            })?;
            let old = handle.snapshot()?;
            let old_image = handle.frame()?;
            let session = old.editable_brush.as_ref().unwrap().session.unwrap();
            reject.set(true);
            assert!(
                handle
                    .dispatch_editable_brush(EditableBrushInput::Move {
                        session,
                        id: 9,
                        point: (150., 125.)
                    })
                    .is_err()
            );
            assert_eq!(handle.snapshot()?, old);
            assert!(Rc::ptr_eq(&old_image, &handle.frame()?));
            reject.set(false);
            handle.dispatch_editable_brush(EditableBrushInput::Cancel { session, id: 9 })?;
            settle(&mut app, &provider)?;
            let mut raster = Raster::new();
            raster.draw(app.renderer())?;
            if dpr == 2. && scale == 2. {
                save(&raster, &output.with_extension("editable-brush.png"))?;
            }
            let decoded = provider.0.lock().unwrap().decoded;
            assert!(decoded >= 15);
            let starts = events
                .borrow()
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        SurfaceEvent::EditableBrush {
                            event: EditableBrushEvent {
                                phase: BrushPhase::Start,
                                source: BrushSource::User,
                                ..
                            },
                            ..
                        }
                    )
                })
                .count();
            assert!(starts >= 11);
            let clear = nodes(app.app(), "kartis-raster-brush-clear")[0];
            tab_to(&mut app, &provider, clear)?;
            key(&mut app, NamedKey::Enter);
            settle(&mut app, &provider)?;
            assert_eq!(selection(&handle)?, None);
            assert!(
                failures.borrow().is_empty(),
                "native bridge errors: {:?}",
                failures.borrow()
            );
            cases.push(serde_json::json!({"dpr":dpr,"cameraScale":scale,"imageBounds":[b.x,b.y,b.width,b.height],"pngDimensions":[handle.frame()?.width,handle.frame()?.height],"actualHandles":resized,"nativeUserStarts":starts,"decodedImages":decoded,"idsAfterCreation":ids,"keyboardDelta":[1,0],"focusCancellation":true,"callbackRetirement":true,"reopenedSessionProtected":true,"activeHiddenCancellation":true,"hostLayoutCancellation":true,"publisherAtomic":true}));
            guard.borrow_mut().take();
            settle(&mut app, &provider)?;
            assert!(!handle.is_active());
            assert!(weak.upgrade().is_none());
            assert!(nodes(app.app(), "kartis-raster-brush-overlay").is_empty());
            app.app().clear()?;
            settle(&mut app, &provider)?;
            assert_eq!(app.app().stats().nodes, 0);
            assert_eq!(app.app().stats().handlers, 0);
        }
    }
    std::fs::write(
        output.with_extension("editable-brush.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"passed":true,"headlessApaceDocument":true,"actualDragCapture":true,"actualKeyboardButtons":true,"cpuViewportOverlay":true,"transformedOnce":true,"offsetDprMapping":true,"publisherAtomic":true,"programmaticRetiresSession":true,"staleInputIgnored":true,"controlsNamed":true,"currentDataIds":true,"cleanup":true,"osWindowOrDeviceAcceptance":false,"cases":cases}),
        )?,
    )?;
    Ok(())
}
