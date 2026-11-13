//! Actual Apace document/input/image decode/CPU paint; no OS window is opened.
#[path = "../../shared/geo_conic.rs"]
pub mod conic_model;
mod contour_density;
mod editable_brush;
mod geo_conic;
mod sequential_quantile;
mod text_baseline;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use apace_dom::Document;
use apace_traits::events::{
    ApaceKeyEvent, ApacePointerEvent, ApacePointerId, KeyState, MouseEventButton,
    MouseEventButtons, Point as InputPoint, PointerCoords, UiEvent,
};
use glory::Widget;
use glory::render::{NodeId, Props, Query, QueryValue};
use glory_apace::assets::{Bytes, NetHandler, NetProvider, Request, Url};
use glory_apace::shell::WindowApp;
use glory_apace::{Config, Raster};
use kartis::render::raster::{
    BidiBaseDirection, ColorGlyphMode, FaceDescriptor, FontRegistry, render_raster,
};
use kartis::surface::{SurfaceEvent, SurfaceInput};
use kartis::*;
use keyboard_types::{Code, Key, Location, Modifiers, NamedKey};
type AnyResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Job {
    doc: usize,
    request: Request,
    handler: Box<dyn NetHandler>,
}
#[derive(Default)]
struct Storage {
    images: BTreeMap<String, Bytes>,
    jobs: VecDeque<Job>,
    next: u64,
    fetched: usize,
    decoded: usize,
    cancelled: usize,
    closed: usize,
}
#[derive(Default)]
struct Provider(Mutex<Storage>);
impl Provider {
    fn publish(&self, image: &kartis::render::raster::RenderedImage) -> Result<String> {
        let bytes = Bytes::from(image.png()?);
        let mut state = self.0.lock().unwrap();
        if bytes.len() > 8 * 1024 * 1024
            || state.images.len() >= 64
            || state.images.values().map(Bytes::len).sum::<usize>() + bytes.len() > 32 * 1024 * 1024
        {
            return Err(ChartError(
                "explicit PNG provider byte/count budget exceeded".into(),
            ));
        }
        state.next = state
            .next
            .checked_add(1)
            .ok_or_else(|| ChartError("image revision exhausted".into()))?;
        let source = format!("asset://raster/frame-{}.png", state.next);
        state.images.insert(source.clone(), bytes);
        Ok(source)
    }
    fn serve(&self) {
        let jobs = {
            let mut state = self.0.lock().unwrap();
            std::mem::take(&mut state.jobs)
        };
        for mut job in jobs {
            if job.request.signal.as_ref().is_some_and(|s| s.aborted()) {
                continue;
            }
            let url = job.request.url.to_string();
            let bytes = self.0.lock().unwrap().images.get(&url).cloned();
            let Some(bytes) = bytes else {
                job.handler.error(url, "no explicit raster asset".into());
                continue;
            };
            let ready = {
                let mut future = job.handler.prepare(&url, &bytes);
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
            };
            match ready {
                Poll::Ready(Ok(())) => {
                    job.handler.bytes(url, bytes);
                    self.0.lock().unwrap().decoded += 1;
                }
                Poll::Ready(Err(error)) => job.handler.error(url, error),
                Poll::Pending => self.0.lock().unwrap().jobs.push_back(job),
            }
        }
    }
}
impl NetProvider for Provider {
    fn fetch(&self, doc: usize, request: Request, handler: Box<dyn NetHandler>) {
        if request.url.scheme() != "asset"
            || request.url.host_str() != Some("raster")
            || request.method.as_str() != "GET"
        {
            handler.error(
                request.url.to_string(),
                "provider denies non-raster capability".into(),
            );
            return;
        }
        let mut state = self.0.lock().unwrap();
        if state.jobs.len() >= 32 {
            drop(state);
            handler.error(request.url.to_string(), "provider queue full".into());
            return;
        }
        state.fetched += 1;
        state.jobs.push_back(Job {
            doc,
            request,
            handler,
        });
    }
    fn cancel(&self, doc: usize, url: &str) {
        let removed = {
            let mut s = self.0.lock().unwrap();
            let mut kept = VecDeque::new();
            let mut removed = Vec::new();
            for job in std::mem::take(&mut s.jobs) {
                if job.doc == doc && job.request.url.as_str() == url {
                    removed.push(job);
                } else {
                    kept.push_back(job);
                }
            }
            s.jobs = kept;
            s.cancelled += removed.len();
            removed
        };
        drop(removed);
    }
    fn close(&self, doc: usize) {
        let removed = {
            let mut s = self.0.lock().unwrap();
            let mut kept = VecDeque::new();
            let mut removed = Vec::new();
            for job in std::mem::take(&mut s.jobs) {
                if job.doc == doc {
                    removed.push(job);
                } else {
                    kept.push_back(job);
                }
            }
            s.jobs = kept;
            s.images.clear();
            s.closed += 1;
            removed
        };
        drop(removed);
    }
}
fn scene(width: f64, height: f64) -> Scene {
    let mut s = ChartConfig {
        width,
        height,
        title: "Native CPU chart".into(),
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, x, color) in [
        ("alpha", 40., Color(220, 30, 40, 255)),
        ("beta", 220., Color(20, 160, 70, 255)),
    ] {
        let mut mark = Mark::new(
            id,
            Geometry::Rect {
                x,
                y: 30.,
                width: 110.,
                height: 85.,
            },
            color,
        );
        mark.series = id.into();
        mark.label = id.into();
        mark.value = 12.5;
        s.marks.push(mark);
        s.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color,
        });
        s.label_series.insert(s.labels.len(), id.into());
        s.labels.push(Label {
            x: x + 55.,
            y: 155.,
            text: format!("{id} fi"),
            size: 20.,
            color: Color(25, 35, 45, 255),
            anchor: TextAnchor::Middle,
            rotation: 12.,
        });
    }
    s.labels.push(Label {
        x: 200.,
        y: 195.,
        text: "CPU AV fi".into(),
        size: 17.,
        color: Color(25, 35, 45, 255),
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
    s
}
fn nodes(app: &glory::App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut output = Vec::new();
    while let Some(id) = stack.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            output.push(id);
        }
        stack.extend(n.children.iter().rev().copied());
    }
    output
}
fn bounds(app: &WindowApp, node: NodeId) -> AnyResult<RasterSurfaceBounds> {
    let output = Rc::new(RefCell::new(None));
    let result = output.clone();
    app.app().request(Query::Bounds(node), move |value| {
        *result.borrow_mut() = Some(value)
    })?;
    app.app().pump()?;
    let value = output.borrow_mut().take().ok_or("bounds not delivered")??;
    let QueryValue::Bounds(r) = value else {
        return Err("not bounds".into());
    };
    Ok(RasterSurfaceBounds {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    })
}
fn settle(app: &mut WindowApp, provider: &Provider) -> AnyResult {
    for _ in 0..8 {
        provider.serve();
        app.observe(&Context::from_waker(Waker::noop()))?;
    }
    Ok(())
}
fn pointer(x: f64, y: f64, pressed: bool) -> ApacePointerEvent {
    ApacePointerEvent {
        id: ApacePointerId::Mouse,
        is_primary: true,
        coords: PointerCoords {
            page_x: x as f32,
            page_y: y as f32,
            client_x: x as f32,
            client_y: y as f32,
            screen_x: x as f32,
            screen_y: y as f32,
        },
        button: MouseEventButton::Main,
        buttons: if pressed {
            MouseEventButtons::Primary
        } else {
            MouseEventButtons::empty()
        },
        mods: Default::default(),
        details: Default::default(),
        element: InputPoint::default(),
        active_pointers: Default::default(),
    }
}
/// One application-native input entry point. Apace gets the genuine UiEvent;
/// the plot bridge uses Apace's hit tree plus explicit image Bounds. Finite
/// Glory Pointer events have no coordinates or press phase, so the host owns
/// this conversion rather than fabricating semantic events or assuming (0,0).
fn host_pointer(
    app: &mut WindowApp,
    handle: &RasterSurfaceHandle,
    kind: &str,
    x: f64,
    y: f64,
) -> AnyResult {
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, image)?;
    let physical = app.renderer().engine_id(image, glory_html::Part::Main)?;
    let hit = app
        .renderer()
        .inspect(|doc| doc.hit(x as f32, y as f32).map(|hit| hit.node_id));
    let in_image = hit == Some(physical);
    let event = match kind {
        "move" => UiEvent::PointerMove(pointer(x, y, false)),
        "down" => UiEvent::PointerDown(pointer(x, y, true)),
        "up" => UiEvent::PointerUp(pointer(x, y, false)),
        _ => return Err("invalid native pointer event".into()),
    };
    app.handle_ui_event(event);
    // Retire queued control hover/focus exits before the current plot input.
    app.app().pump()?;
    if in_image || handle.snapshot()?.dragging {
        let input = match kind {
            "move" => SurfaceInput::Move {
                id: 1,
                point: (x, y),
            },
            "down" => SurfaceInput::Down {
                id: 1,
                point: (x, y),
                brush: false,
            },
            _ => SurfaceInput::Up {
                id: 1,
                point: (x, y),
            },
        };
        handle.dispatch_host(input, b)?;
    }
    Ok(())
}
fn key(app: &mut WindowApp, key: NamedKey) {
    app.handle_ui_event(UiEvent::KeyDown(ApaceKeyEvent {
        key: Key::Named(key),
        code: Code::Unidentified,
        modifiers: Modifiers::empty(),
        location: Location::Standard,
        is_auto_repeating: false,
        is_composing: false,
        state: KeyState::Pressed,
        text: None,
    }));
}
fn text(app: &WindowApp, class: &str) -> String {
    let tree = app.app().tree();
    let Some(id) = nodes(app.app(), class).first().copied() else {
        return String::new();
    };
    match tree.node(id).unwrap().descriptor.view().props {
        Props::Text(t) => t.content.clone(),
        Props::Button(b) => b.label.clone(),
        _ => String::new(),
    }
}
fn save(raster: &Raster, path: &Path) -> AnyResult {
    let frame = raster.frame().ok_or("no raster frame")?;
    let mut rgba = raster.pixels().to_vec();
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgba)?;
    writer.finish()?;
    Ok(())
}
fn sample(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let frame = raster.frame().unwrap();
    let offset = ((y * frame.width + x) * 4) as usize;
    raster.pixels()[offset..offset + 4].try_into().unwrap()
}
fn verify_direct_chart_raster() -> AnyResult {
    let provider = Arc::new(Provider::default());
    provider.0.lock().unwrap().images.insert(
        "asset://raster/font.ttf".into(),
        Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
    );
    let mut app = WindowApp::with_config(Config {
        provider: Some(provider.clone()),
        base_url: Url::parse("asset://raster/")?,
        ..Default::default()
    });
    app.renderer().set_viewport(560, 420, 1.)?;
    app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/font.ttf')}html,body{margin:0;background:white;font-family:Fixture}.direct{padding:18px}button{padding:6px}");
    let mut fonts = FontRegistry::new();
    fonts.register(
        include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    let publish = provider.clone();
    app.mount(
        glory::ui::view().class("direct").fill(
            raster_chart(
                scene(400., 220.),
                fonts,
                kartis::render::raster::RasterOptions::default(),
            )
            .raster_image_source(move |image| publish.publish(image)),
        ),
    )?;
    settle(&mut app, &provider)?;
    let image = nodes(app.app(), "kartis-raster-image")[0];
    let b = bounds(&app, image)?;
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    assert_eq!(
        sample(&raster, (b.x + 60.) as u32, (b.y + 50.) as u32),
        [220, 30, 40, 255]
    );
    let legend = nodes(app.app(), "kartis-legend-item")[0];
    let l = bounds(&app, legend)?;
    app.handle_ui_event(UiEvent::PointerDown(pointer(
        l.x + l.width / 2.,
        l.y + l.height / 2.,
        true,
    )));
    app.handle_ui_event(UiEvent::PointerUp(pointer(
        l.x + l.width / 2.,
        l.y + l.height / 2.,
        false,
    )));
    settle(&mut app, &provider)?;
    raster.draw(app.renderer())?;
    assert_eq!(
        sample(&raster, (b.x + 60.) as u32, (b.y + 50.) as u32),
        [255, 255, 255, 255]
    );
    assert_eq!(nodes(app.app(), "kartis-datum").len(), 1);
    app.app().clear()?;
    settle(&mut app, &provider)?;
    drop(app);
    assert!(provider.0.lock().unwrap().jobs.is_empty());
    Ok(())
}
fn verify_semantic_outline(output: &Path) -> AnyResult<(usize, usize)> {
    let provider = Arc::new(Provider::default());
    provider.0.lock().unwrap().images.insert(
        "asset://raster/font.ttf".into(),
        Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
    );
    let mut app = WindowApp::with_config(Config {
        provider: Some(provider.clone()),
        base_url: Url::parse("asset://raster/")?,
        ..Default::default()
    });
    app.renderer().set_viewport(560, 420, 1.)?;
    app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/font.ttf')}html,body{margin:0;background:white;font-family:Fixture}.outlined{padding:18px}button{padding:6px}");
    let mut fonts = FontRegistry::new().with_bidi(BidiBaseDirection::Auto);
    for bytes in [
        include_bytes!("../../../tests/fonts/Lato-Regular.ttf").as_slice(),
        include_bytes!("../../../tests/fonts/KartisFixtureHebrew-Regular.ttf").as_slice(),
        include_bytes!("../../../tests/fonts/KartisFixtureArabic-Regular.ttf").as_slice(),
    ] {
        fonts.register(bytes, 0)?;
    }
    let mut frame = scene(400., 220.);
    frame.labels.push(Label {
        x: 35.,
        y: 214.,
        text: "Latin אבג 123 السَّلَام".into(),
        size: 16.,
        color: Color(150, 35, 160, 255),
        anchor: TextAnchor::Start,
        rotation: -1.5,
    });
    app.mount(
        glory::ui::view()
            .class("outlined")
            .fill(chart(frame).outline_labels(fonts, Default::default())),
    )?;
    settle(&mut app, &provider)?;
    let glyphs = nodes(app.app(), "kartis-outline-glyph");
    assert!(glyphs.len() > 5, "actual semantic glyph Graphics absent");
    assert!(
        nodes(app.app(), "kartis-raster-image").is_empty(),
        "outline path must not be raster-image substitution"
    );
    let plot = nodes(app.app(), "kartis-plot")[0];
    let b = bounds(&app, plot)?;
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    let mut ink = 0usize;
    for y in (b.y as u32 + 125)..(b.y as u32 + 180) {
        for x in (b.x as u32 + 30)..(b.x as u32 + 360) {
            let p = sample(&raster, x, y);
            if p[..3].iter().copied().max().unwrap() < 130 && p[3] > 0 {
                ink += 1;
            }
        }
    }
    assert!(
        ink > 200,
        "rotated semantic glyph paths produced no actual Apace CPU pixels"
    );
    let mut bidi_ink = 0;
    for y in (b.y as u32 + 195)..(b.y as u32 + 220) {
        for x in (b.x as u32 + 30)..(b.x as u32 + 360) {
            let [r, g, blue, a] = sample(&raster, x, y);
            if (140..180).contains(&r) && g < 100 && (150..190).contains(&blue) && a == 255 {
                bidi_ink += 1;
            }
        }
    }
    assert!(
        bidi_ink > 120,
        "mixed-direction outline produced insufficient Apace ink: {bidi_ink}"
    );
    save(&raster, &output.with_extension("outline.png"))?;
    let legend = nodes(app.app(), "kartis-legend-item")[0];
    let l = bounds(&app, legend)?;
    app.handle_ui_event(UiEvent::PointerDown(pointer(
        l.x + l.width / 2.,
        l.y + l.height / 2.,
        true,
    )));
    app.handle_ui_event(UiEvent::PointerUp(pointer(
        l.x + l.width / 2.,
        l.y + l.height / 2.,
        false,
    )));
    settle(&mut app, &provider)?;
    let owned = nodes(app.app(), "kartis-outline-label")[0];
    assert_eq!(
        app.app()
            .tree()
            .node(owned)
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(glory::style::Display::None)
    );
    app.app().clear()?;
    settle(&mut app, &provider)?;
    assert!(nodes(app.app(), "kartis-outline-glyph").is_empty());
    Ok((ink, bidi_ink))
}
fn verify_semantic_color_fonts(output: &Path) -> AnyResult {
    for palette in [0, 1] {
        let provider = Arc::new(Provider::default());
        provider.0.lock().unwrap().images.insert(
            "asset://raster/font.ttf".into(),
            Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
        );
        let mut app = WindowApp::with_config(Config {
            provider: Some(provider.clone()),
            base_url: Url::parse("asset://raster/")?,
            ..Default::default()
        });
        app.renderer().set_viewport(240, 200, 1.)?;
        app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/font.ttf')}html,body{margin:0;background:white;font-family:Fixture}");
        let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette });
        fonts.register(
            include_bytes!("../../../tests/fonts/KartisFixtureColrV0.ttf").as_slice(),
            0,
        )?;
        let mut frame = ChartConfig {
            width: 180.,
            height: 180.,
            ..Default::default()
        }
        .scene()?;
        frame.labels.push(Label {
            text: "B".into(),
            x: 40.,
            y: 140.,
            size: 100.,
            color: Color(0, 255, 0, 0),
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        let source = app.app().scope().cage(frame);
        app.mount(
            chart(source)
                .legend(false)
                .data_controls(false)
                .tooltip(false)
                .painted_labels(
                    fonts,
                    PaintedLabelOptions {
                        rotated_only: false,
                        ..Default::default()
                    },
                ),
        )?;
        settle(&mut app, &provider)?;
        let ids = nodes(app.app(), "kartis-painted-glyph");
        assert_eq!(ids.len(), 4);
        assert!(nodes(app.app(), "kartis-raster-image").is_empty());
        let plot = nodes(app.app(), "kartis-plot")[0];
        let b = bounds(&app, plot)?;
        let mut raster = Raster::new();
        raster.draw(app.renderer())?;
        let at = |raster: &Raster, x, y| sample(raster, b.x as u32 + x, b.y as u32 + y);
        let close = |actual: [u8; 4], expected: [u8; 4]| {
            assert!(
                actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1),
                "COLR native palette {palette}: {actual:?} != {expected:?}"
            )
        };
        close(
            at(&raster, 50, 50),
            if palette == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 180, 0, 255]
            },
        );
        close(at(&raster, 70, 80), [255, 255, 255, 255]);
        close(
            at(&raster, 100, 80),
            if palette == 0 {
                [127, 127, 255, 255]
            } else {
                [255, 255, 127, 255]
            },
        );
        close(
            at(&raster, 125, 80),
            if palette == 0 {
                [127, 0, 128, 255]
            } else {
                [128, 218, 0, 255]
            },
        );
        close(
            at(&raster, 50, 125),
            if palette == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 180, 0, 255]
            },
        );
        source.revise(|frame| frame.labels[0].color = Color(0, 255, 0, 128));
        settle(&mut app, &provider)?;
        assert_eq!(nodes(app.app(), "kartis-painted-glyph"), ids);
        raster.draw(app.renderer())?;
        close(
            at(&raster, 50, 125),
            if palette == 0 {
                [127, 128, 0, 255]
            } else {
                [0, 218, 0, 255]
            },
        );
        save(
            &raster,
            &output.with_extension(format!("color-{palette}.png")),
        )?;
        app.app().clear()?;
        settle(&mut app, &provider)?;
        assert!(nodes(app.app(), "kartis-painted-glyph").is_empty());
    }
    Ok(())
}
fn verify_semantic_typography(output: &Path) -> AnyResult {
    let regular: Arc<[u8]> = Arc::from(
        include_bytes!("../../../tests/fonts/KartisFixtureTypography-Regular.ttf").as_slice(),
    );
    let bold: Arc<[u8]> = Arc::from(
        include_bytes!("../../../tests/fonts/KartisFixtureTypography-Bold.ttf").as_slice(),
    );
    let regular_weak = Arc::downgrade(&regular);
    let bold_weak = Arc::downgrade(&bold);
    let family = FontFamily::Named("KartisNativeTypography".into());
    let mut fonts = FontRegistry::new();
    for (bytes, weight) in [(regular, 400.), (bold, 700.)] {
        let id = fonts.register(bytes, 0)?;
        fonts.bind_face(
            id,
            FaceDescriptor {
                family: family.clone(),
                weight,
            },
        )?;
    }
    let provider = Arc::new(Provider::default());
    for (path, bytes) in [
        (
            "asset://raster/typography-400.ttf",
            include_bytes!("../../../tests/fonts/KartisFixtureTypography-Regular.ttf").as_slice(),
        ),
        (
            "asset://raster/typography-700.ttf",
            include_bytes!("../../../tests/fonts/KartisFixtureTypography-Bold.ttf").as_slice(),
        ),
    ] {
        provider
            .0
            .lock()
            .unwrap()
            .images
            .insert(path.into(), Bytes::copy_from_slice(bytes));
    }
    let mut app = WindowApp::with_config(Config {
        provider: Some(provider.clone()),
        base_url: Url::parse("asset://raster/")?,
        ..Default::default()
    });
    app.renderer().set_viewport(300, 230, 1.)?;
    app.renderer().set_css("@font-face{font-family:KartisNativeTypography;font-weight:400;src:url('asset://raster/typography-400.ttf')}@font-face{font-family:KartisNativeTypography;font-weight:700;src:url('asset://raster/typography-700.ttf')}html,body{margin:0;background:white;font-family:KartisNativeTypography}");
    let mut frame = ChartConfig {
        width: 240.,
        height: 180.,
        ..Default::default()
    }
    .scene()?;
    frame.typography.default = FontOptions {
        family: Some(vec![family]),
        weight: Some(400.),
    };
    frame.labels.push(Label {
        x: 40.,
        y: 140.,
        text: "B".into(),
        size: 100.,
        color: Color(0, 0, 0, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    let source = app.app().scope().cage(frame.clone());
    let mut cpu_fonts = Some(fonts.clone());
    let rejections = Rc::new(RefCell::new(Vec::new()));
    let rejected = rejections.clone();
    app.mount(
        chart(source)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .painted_labels(fonts, Default::default())
            .on_reject(move |error| rejected.borrow_mut().push(error.0)),
    )
    .map_err(|error| format!("painted typography mount: {error}"))?;
    settle(&mut app, &provider)?;
    let ids = nodes(app.app(), "kartis-painted-glyph");
    assert_eq!(ids.len(), 1);
    assert!(nodes(app.app(), "kartis-raster-image").is_empty());
    let plot = nodes(app.app(), "kartis-plot")[0];
    let b = bounds(&app, plot)?;
    let mut raster = Raster::new();
    for weight in [400., 700.] {
        frame.typography.default.weight = Some(weight);
        source.revise(|source| *source = frame.clone());
        settle(&mut app, &provider)?;
        assert_eq!(nodes(app.app(), "kartis-painted-glyph"), ids);
        let cpu = render_raster(&frame, cpu_fonts.as_ref().unwrap(), &Default::default())?;
        raster.draw(app.renderer())?;
        // The axis-aligned authored B has integer edges and a real winding hole.
        // Compare every pixel of the actual Apace plot with the CPU selection.
        for y in 0..cpu.height {
            for x in 0..cpu.width {
                let actual = sample(&raster, b.x as u32 + x, b.y as u32 + y);
                let offset = ((y * cpu.width + x) * 4) as usize;
                assert!(
                    actual
                        .iter()
                        .zip(&cpu.rgba[offset..offset + 4])
                        .all(|(a, e)| a.abs_diff(*e) <= 1),
                    "native typography {weight} pixel ({x}, {y}) differs: {actual:?}"
                );
            }
        }
        save(
            &raster,
            &output.with_extension(format!("typography-{weight}.png")),
        )?;
    }
    let accepted_pixels = raster.pixels().to_vec();
    let accepted_props = app
        .app()
        .tree()
        .node(ids[0])
        .unwrap()
        .descriptor
        .view()
        .props
        .clone();
    source.revise(|frame| {
        frame.typography.default.family = Some(vec![FontFamily::Named("Unbound face".into())])
    });
    settle(&mut app, &provider)?;
    assert_eq!(rejections.borrow().len(), 1);
    assert!(rejections.borrow()[0].contains("face binding"));
    assert_eq!(nodes(app.app(), "kartis-painted-glyph"), ids);
    assert_eq!(
        app.app()
            .tree()
            .node(ids[0])
            .unwrap()
            .descriptor
            .view()
            .props,
        &accepted_props
    );
    raster.draw(app.renderer())?;
    for y in 0..180u32 {
        for x in 0..240u32 {
            let offset =
                (((b.y as u32 + y) * raster.frame().unwrap().width + b.x as u32 + x) * 4) as usize;
            assert_eq!(
                sample(&raster, b.x as u32 + x, b.y as u32 + y),
                accepted_pixels[offset..offset + 4]
            );
        }
    }
    source.revise(|source| *source = frame.clone());
    settle(&mut app, &provider)?;
    assert_eq!(nodes(app.app(), "kartis-painted-glyph"), ids);
    cpu_fonts.take();
    app.app().clear()?;
    settle(&mut app, &provider)?;
    assert!(regular_weak.upgrade().is_none());
    assert!(bold_weak.upgrade().is_none());
    assert_eq!(app.app().stats().nodes, 0);
    assert_eq!(app.app().stats().handlers, 0);

    // Verify ordinary semantic Text independently with explicitly served font
    // assets. Its layout uses host text metrics; no glyph Graphic substitutes.
    let plain_source = app.app().scope().cage(frame);
    app.mount(
        chart(plain_source)
            .legend(false)
            .data_controls(false)
            .tooltip(false),
    )
    .map_err(|error| format!("plain typography mount: {error}"))?;
    settle(&mut app, &provider)?;
    let wrappers = nodes(app.app(), "kartis-label");
    assert_eq!(wrappers.len(), 1);
    let text_node = app.app().tree().node(wrappers[0]).unwrap().children[0];
    assert!(matches!(
        app.app()
            .tree()
            .node(text_node)
            .unwrap()
            .descriptor
            .view()
            .props,
        Props::Text(_)
    ));
    assert!(nodes(app.app(), "kartis-painted-glyph").is_empty());
    for (weight, expected_advance, expected_width) in [(400., 60., 40.), (700., 100., 80.)] {
        plain_source.revise(|frame| frame.typography.default.weight = Some(weight));
        settle(&mut app, &provider)?;
        assert_eq!(
            app.app().tree().node(wrappers[0]).unwrap().children[0],
            text_node
        );
        let text_bounds = bounds(&app, text_node)?;
        assert!(
            (text_bounds.width - expected_advance).abs() <= 1.,
            "native Text weight {weight}: {text_bounds:?}"
        );
        raster.draw(app.renderer())?;
        let mut ink = Vec::new();
        for y in 0..180 {
            for x in 0..240 {
                if sample(&raster, x, y)[..3].iter().all(|c| *c < 64) {
                    ink.push((x, y));
                }
            }
        }
        assert!(ink.len() > 500);
        let width = ink.iter().map(|(x, _)| x).max().unwrap()
            - ink.iter().map(|(x, _)| x).min().unwrap()
            + 1;
        assert!((f64::from(width) - expected_width).abs() <= 1.);
    }
    assert!(app.renderer().resource_stats().registered_fonts >= 2);
    save(
        &raster,
        &output.with_extension("typography-native-text.png"),
    )?;
    app.app().clear()?;
    settle(&mut app, &provider)?;
    assert_eq!(app.app().stats().nodes, 0);
    assert_eq!(app.app().stats().handlers, 0);
    drop(app);
    assert!(provider.0.lock().unwrap().jobs.is_empty());
    Ok(())
}
fn main() -> AnyResult {
    let output = std::env::args_os()
        .nth(1)
        .ok_or("pass an explicit output PNG path; no window is opened")?;
    verify_direct_chart_raster().map_err(|error| format!("native direct raster: {error}"))?;
    let (semantic_outline_ink, semantic_bidi_ink) = verify_semantic_outline(Path::new(&output))
        .map_err(|error| format!("native semantic outline: {error}"))?;
    verify_semantic_color_fonts(Path::new(&output))
        .map_err(|error| format!("native semantic color: {error}"))?;
    verify_semantic_typography(Path::new(&output))
        .map_err(|error| format!("native semantic typography: {error}"))?;
    text_baseline::verify(Path::new(&output))
        .map_err(|error| format!("native ordinary text baseline: {error}"))?;
    editable_brush::verify(Path::new(&output))
        .map_err(|error| format!("native editable brush: {error}"))?;
    let contour_density_report = contour_density::verify(Path::new(&output))
        .map_err(|error| format!("native contour/density: {error}"))?;
    let sequential_quantile_report = sequential_quantile::verify(Path::new(&output))
        .map_err(|error| format!("native sequential quantile: {error}"))?;
    let geo_conic_report =
        geo_conic::verify(Path::new(&output)).map_err(|error| format!("native conic: {error}"))?;
    let provider = Arc::new(Provider::default());
    provider.0.lock().unwrap().images.insert(
        "asset://raster/font.ttf".into(),
        Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
    );
    let mut app = WindowApp::with_config(Config {
        provider: Some(provider.clone()),
        base_url: Url::parse("asset://raster/")?,
        ..Default::default()
    });
    app.renderer().set_viewport(760, 560, 1.)?;
    app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/font.ttf')}html,body{margin:0;background:white;font-family:Fixture}.native-root{padding:24px}.kartis-legend,.kartis-data{display:flex;gap:8px;margin-top:12px;flex-wrap:wrap}button{padding:8px 10px;border:1px solid #aaa;background:#eee}.kartis-tooltip{padding:6px;background:#20334e;color:white}");
    let guard = Rc::new(RefCell::new(None));
    let installed = guard.clone();
    let explicit_provider = provider.clone();
    app.mount(
        glory::ui::view()
            .class("native-root")
            .fill(glory::ui::widget(move |scope| {
                let mut fonts = FontRegistry::new();
                fonts
                    .register(
                        include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
                        0,
                    )
                    .map_err(|_| glory::render::Error::InvalidProps("fixture font"))?;
                let surface = RasterSurface::mount_published(
                    scope,
                    scene(400., 220.),
                    fonts,
                    RasterSurfaceOptions::default(),
                    {
                        let provider = explicit_provider.clone();
                        move |image| provider.publish(image)
                    },
                )
                .map_err(|_| glory::render::Error::InvalidProps("fixture surface"))?;
                let widget = surface.widget();
                *installed.borrow_mut() = Some(surface);
                widget.show_in(scope)
            })),
    )?;
    let handle = guard
        .borrow()
        .as_ref()
        .ok_or("surface guard missing")?
        .handle();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observer = events.clone();
    guard
        .borrow()
        .as_ref()
        .unwrap()
        .on_event(move |values| observer.borrow_mut().extend(values))?;
    settle(&mut app, &provider)?;
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(&app, image)?;
    assert!(
        b.x >= 20. && b.y >= 20.,
        "fixture must exercise nonzero host offset"
    );
    let decoded = app.renderer().inspect(|doc| {
        let id = app
            .renderer()
            .engine_id(image, glory_html::Part::Main)
            .unwrap();
        doc.get_node(id)
            .and_then(|n| n.element_data())
            .and_then(|e| e.raster_image_data())
            .map(|image| image.data.as_ref()[0..4].to_vec())
    });
    assert!(decoded.is_some(), "actual Apace image decode absent");
    let registered_fonts = app.renderer().resource_stats().registered_fonts;
    assert!(
        registered_fonts >= 1,
        "explicit provider font was not registered by Apace"
    );
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    assert_eq!(
        sample(&raster, (b.x + 60.) as u32, (b.y + 50.) as u32),
        [220, 30, 40, 255]
    );
    let mut font_ink = 0usize;
    for y in (b.y as u32 + 130)..(b.y as u32 + 205) {
        for x in (b.x as u32 + 30)..(b.x as u32 + 360) {
            let p = sample(&raster, x, y);
            if p[..3].iter().copied().max().unwrap() < 130 && p[3] > 0 {
                font_ink += 1;
            }
        }
    }
    assert!(font_ink > 300, "CPU image omitted supplied font outlines");
    save(&raster, Path::new(&output))?;
    host_pointer(&mut app, &handle, "move", b.x + 70., b.y + 60.)?;
    settle(&mut app, &provider)?;
    assert_eq!(handle.snapshot()?.hovered, ["alpha"]);
    assert!(text(&app, "kartis-tooltip").contains("alpha: 12.5"));
    host_pointer(&mut app, &handle, "down", b.x + 70., b.y + 60.)?;
    host_pointer(&mut app, &handle, "up", b.x + 70., b.y + 60.)?;
    settle(&mut app, &provider)?;
    assert_eq!(handle.snapshot()?.selected.as_deref(), Some("alpha"));
    let legend = nodes(app.app(), "kartis-legend-item")[0];
    let l = bounds(&app, legend)?;
    app.handle_ui_event(UiEvent::PointerDown(pointer(
        l.x + l.width / 2.,
        l.y + l.height / 2.,
        true,
    )));
    app.handle_ui_event(UiEvent::PointerUp(pointer(
        l.x + l.width / 2.,
        l.y + l.height / 2.,
        false,
    )));
    settle(&mut app, &provider)?;
    assert!(handle.snapshot()?.hidden.contains("alpha"));
    assert_eq!(nodes(app.app(), "kartis-datum").len(), 1);
    raster.draw(app.renderer())?;
    assert_eq!(
        sample(&raster, (b.x + 60.) as u32, (b.y + 50.) as u32),
        [255, 255, 255, 255]
    );
    handle.set_hidden(BTreeSet::new())?;
    settle(&mut app, &provider)?;
    let datum = nodes(app.app(), "kartis-datum")[0];
    let d = bounds(&app, datum)?;
    app.handle_ui_event(UiEvent::PointerDown(pointer(
        d.x + d.width / 2.,
        d.y + d.height / 2.,
        true,
    )));
    app.handle_ui_event(UiEvent::PointerUp(pointer(
        d.x + d.width / 2.,
        d.y + d.height / 2.,
        false,
    )));
    settle(&mut app, &provider)?;
    key(&mut app, NamedKey::ArrowDown);
    settle(&mut app, &provider)?;
    key(&mut app, NamedKey::Enter);
    settle(&mut app, &provider)?;
    assert_eq!(handle.snapshot()?.selected.as_deref(), Some("beta"));
    assert!(
        events
            .borrow()
            .iter()
            .any(|e| matches!(e,SurfaceEvent::Activated(id)if id=="beta"))
    );
    handle.set_scene(scene(460., 240.))?;
    settle(&mut app, &provider)?;
    assert_eq!(bounds(&app, image)?.width, 460.);
    assert_eq!(handle.frame()?.width, 460);
    handle.set_raster_options(kartis::render::raster::RasterOptions {
        pixel_ratio: 2.,
        ..Default::default()
    })?;
    settle(&mut app, &provider)?;
    let physical = app.renderer().engine_id(image, glory_html::Part::Main)?;
    let decoded_dimensions = app.renderer().inspect(|doc| {
        doc.get_node(physical)
            .and_then(|n| n.element_data())
            .and_then(|e| e.raster_image_data())
            .map(|image| (image.width, image.height))
    });
    assert_eq!(decoded_dimensions, Some((920, 480)));
    assert_eq!(bounds(&app, image)?.width, 460.);
    let before = handle.snapshot()?;
    let mut invalid = scene(460., 240.);
    invalid.width = f64::NAN;
    assert!(handle.set_scene(invalid).is_err());
    assert_eq!(handle.snapshot()?, before);
    let requests = provider.0.lock().unwrap().fetched;
    let weak = handle.clone();
    guard.borrow_mut().take();
    settle(&mut app, &provider)?;
    assert!(!weak.is_active());
    assert!(nodes(app.app(), "kartis-raster-surface-image").is_empty());
    assert!(nodes(app.app(), "kartis-datum").is_empty());
    assert_eq!(provider.0.lock().unwrap().fetched, requests);
    app.app().clear()?;
    settle(&mut app, &provider)?;
    assert_eq!(app.app().stats().nodes, 0);
    assert_eq!(app.app().stats().handlers, 0);
    drop(app);
    {
        let state = provider.0.lock().unwrap();
        assert!(
            state.closed >= 1 && state.jobs.is_empty() && state.images.is_empty(),
            "document close retained explicit provider assets/jobs"
        );
    }
    let report = serde_json::json!({"passed":true,"headlessApaceDocument":true,"directChartRasterPngAndLegend":true,"actualPngDecode":true,"actualCpuImagePixels":true,"semanticColrV0PaletteLayers":true,"semanticTypographyFaces":true,"semanticTextBaseline":true,"editableBrush":true,"contourDensity":contour_density_report,"sequentialQuantile":sequential_quantile_report,"geoConic":geo_conic_report,"semanticOutlineGraphicsInk":semantic_outline_ink,"semanticBidiOutlinePixels":semantic_bidi_ink,"fontOutlinePixels":font_ink,"explicitApaceFontRegistration":registered_fonts,"offsetHostPointerMapping":true,"actualUiEventLegendToggle":true,"actualUiEventKeyboardSelection":true,"tooltip":true,"resizeAndDprDecode":true,"invalidSceneAtomic":true,"guardAndWeakCleanup":true,"providerCloseCleanup":true,"providerRequests":requests,"output":Path::new(&output).display().to_string(),"osWindowOrDeviceAcceptance":false});
    std::fs::write(
        Path::new(&output).with_extension("json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(())
}
