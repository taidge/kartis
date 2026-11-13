//! Ordinary Apace Text measured from its actual Parley line baseline.
//! No font-size heuristic or glyph Graphic supplies the baseline.
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use glory::Cage;
use glory::render::{NodeId, Props};
use glory::style::{Align, Display, Length, Position, Style, WhiteSpace};
use glory_apace::assets::{Bytes, Url};
use glory_apace::shell::WindowApp;
use glory_apace::{Config, Raster};
use kartis::render::raster::*;
use kartis::*;

use super::{AnyResult, Provider, bounds, nodes, sample, save, settle};

const FAMILY: &str = "KartisNativeBaseline";
const BEE: &str = "KartisNativeBaselineBee";
const REGULAR: &[u8] = include_bytes!("../../../tests/fonts/KartisFixtureTypography-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../../tests/fonts/KartisFixtureTypography-Bold.ttf");
const ABEEZEE: &[u8] = include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf");
const CSS: &str = "@font-face{font-family:KartisNativeBaseline;font-weight:400;src:url('asset://raster/baseline-400.ttf')}@font-face{font-family:KartisNativeBaseline;font-weight:700;src:url('asset://raster/baseline-700.ttf')}@font-face{font-family:KartisNativeBaselineBee;font-weight:400;src:url('asset://raster/baseline-bee.ttf')}html,body{margin:0;background:white}";

fn family(name: &str) -> FontFamily {
    FontFamily::Named(name.into())
}
fn provider() -> Arc<Provider> {
    let provider = Arc::new(Provider::default());
    for (path, bytes) in [
        ("baseline-400.ttf", REGULAR),
        ("baseline-700.ttf", BOLD),
        ("baseline-bee.ttf", ABEEZEE),
    ] {
        provider.0.lock().unwrap().images.insert(
            format!("asset://raster/{path}"),
            Bytes::copy_from_slice(bytes),
        );
    }
    provider
}
fn application(provider: &Arc<Provider>, ratio: f64) -> AnyResult<WindowApp> {
    let app = WindowApp::with_config(Config {
        provider: Some(provider.clone()),
        base_url: Url::parse("asset://raster/")?,
        ..Default::default()
    });
    app.renderer()
        .set_viewport((600. * ratio) as u32, (340. * ratio) as u32, ratio as f32)?;
    app.renderer().set_css(CSS);
    Ok(app)
}
fn fonts() -> Result<FontRegistry> {
    let mut fonts = FontRegistry::new();
    for (bytes, name, weight) in [
        (REGULAR, FAMILY, 400.),
        (BOLD, FAMILY, 700.),
        (ABEEZEE, BEE, 400.),
    ] {
        let id = fonts.register(bytes, 0)?;
        fonts.bind_face(
            id,
            FaceDescriptor {
                family: family(name),
                weight,
            },
        )?;
    }
    Ok(fonts)
}

#[derive(Clone, Debug, PartialEq)]
struct ProbeInput {
    text: String,
    size: f64,
    font: ResolvedFont,
    line_height: f64,
}

struct HostProbe {
    app: WindowApp,
    provider: Arc<Provider>,
    input: Cage<ProbeInput>,
    text: NodeId,
    ratio: f64,
}

/// A separate native document prevents the measuring callback from reading or
/// committing partially accepted chart state. It uses actual supplied faces.
impl HostProbe {
    fn new() -> AnyResult<Self> {
        let provider = provider();
        let mut app = application(&provider, 1.)?;
        app.renderer().set_viewport(1200, 512, 1.)?;
        let input = app.app().scope().cage(ProbeInput {
            text: "B".into(),
            size: 100.,
            font: ResolvedFont {
                family: vec![family(FAMILY)],
                weight: 400.,
            },
            line_height: 100.,
        });
        let text = glory::ui::text()
            .class("native-baseline-probe")
            .content(
                app.app()
                    .scope()
                    .bond(move || input.with(|input| input.text.clone())),
            )
            .style(app.app().scope().bond(move || {
                input.with(|input| Style {
                    display: Some(Display::Block),
                    color: Some(Color(0, 0, 0, 255)),
                    font_size: Some(input.size),
                    font_family: Some(input.font.family.clone()),
                    font_weight: Some(input.font.weight),
                    line_height: Some(input.line_height),
                    letter_spacing: Some(0.),
                    white_space: Some(WhiteSpace::NoWrap),
                    shrink: Some(0.),
                    ..Style::new()
                })
            }));
        app.mount(
            glory::ui::view()
                .style(Style {
                    align: Some(Align::Start),
                    ..Style::new()
                        .display(Display::Flex)
                        .position(Position::Absolute)
                        .left(Length::Px(64.))
                        .top(Length::Px(128.))
                        .width(Length::Px(1000.))
                })
                .fill(text),
        )?;
        settle(&mut app, &provider)?;
        let text = nodes(app.app(), "native-baseline-probe")[0];
        Ok(Self {
            app,
            provider,
            input,
            text,
            ratio: 1.,
        })
    }
    fn set_ratio(&mut self, ratio: f64) -> AnyResult {
        self.app.renderer().set_viewport(
            (1200. * ratio) as u32,
            (512. * ratio) as u32,
            ratio as f32,
        )?;
        self.ratio = ratio;
        Ok(())
    }
    fn measure(&mut self, request: &SemanticTextRequest) -> AnyResult<SemanticTextMeasurement> {
        request.validate()?;
        self.input.revise(|input| {
            *input = ProbeInput {
                text: request.text.clone(),
                size: request.size,
                font: request.font.clone(),
                line_height: request.line_height,
            }
        });
        settle(&mut self.app, &self.provider)?;
        let text_rect = bounds(&self.app, self.text)?;
        let baseline = actual_baseline(&self.app, self.text);
        let mut raster = Raster::new();
        raster.draw(self.app.renderer())?;
        let ink = ink_bounds(&raster, self.ratio, |pixel| {
            pixel[..3].iter().any(|c| *c != 255)
        });
        let baseline = match baseline {
            Ok(baseline) => baseline,
            // A truly empty host box has no observable line baseline. The
            // zero offset follows its measured zero box and absent pixels;
            // it is not an estimate derived from the registered font's em.
            Err(_) if text_rect.width == 0. && text_rect.height == 0. && ink.is_none() => {
                text_rect.y
            }
            Err(error) => return Err(error),
        };
        let metrics = SemanticTextMetrics {
            advance: text_rect.width,
            line_height: text_rect.height,
            baseline_from_top: baseline - text_rect.y,
            ink_bounds: ink.map(|ink| LayoutRect {
                x: ink.x - text_rect.x,
                y: ink.y - baseline,
                ..ink
            }),
        };
        Ok(request.measured(metrics)?)
    }
}

fn actual_baseline(app: &WindowApp, text: NodeId) -> AnyResult<f64> {
    let id = app.renderer().engine_id(text, glory_html::Part::Main)?;
    app.renderer()
        .inspect(|document| {
            let node = document.get_node(id)?;
            let root = node.inline_root_ancestor()?;
            let inline = root.element_data()?.inline_layout_data.as_ref()?;
            let line = inline.layout.lines().next()?;
            let root_rect = document.get_client_bounding_rect(root.id)?;
            let layout = root.final_layout();
            Some(
                root_rect.y
                    + f64::from(layout.padding.top + layout.border.top + inline.block_offset)
                    + f64::from(line.metrics().baseline) / f64::from(document.viewport().scale()),
            )
        })
        .ok_or_else(|| "ordinary Text has no actual host line baseline".into())
}

fn ink_bounds(raster: &Raster, ratio: f64, accept: impl Fn([u8; 4]) -> bool) -> Option<LayoutRect> {
    let frame = raster.frame()?;
    let mut extents: Option<(u32, u32, u32, u32)> = None;
    for y in 0..frame.height {
        for x in 0..frame.width {
            if accept(sample(raster, x, y)) {
                extents = Some(match extents {
                    None => (x, y, x + 1, y + 1),
                    Some((left, top, right, bottom)) => {
                        (left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1))
                    }
                });
            }
        }
    }
    extents.map(|(x, y, right, bottom)| LayoutRect {
        x: f64::from(x) / ratio,
        y: f64::from(y) / ratio,
        width: f64::from(right - x) / ratio,
        height: f64::from(bottom - y) / ratio,
    })
}

fn label(text: &str) -> Label {
    Label {
        text: text.into(),
        x: 50.,
        y: 140.,
        size: 100.,
        color: Color(0, 0, 0, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    }
}
fn scene() -> Result<Scene> {
    let mut scene = ChartConfig {
        width: 520.,
        height: 260.,
        ..Default::default()
    }
    .scene()?;
    scene.typography.default = FontOptions {
        family: Some(vec![family(FAMILY)]),
        weight: Some(400.),
    };
    scene.labels.push(label("B"));
    scene.label_series.insert(0, "text".into());
    scene.legend.push(LegendItem {
        id: "text".into(),
        label: "Text".into(),
        color: Color(0, 0, 0, 255),
    });
    Ok(scene)
}
fn ordinary_text(app: &WindowApp) -> AnyResult<NodeId> {
    let wrappers = nodes(app.app(), "kartis-label");
    let tree = app.app().tree();
    let wrapper = tree.node(*wrappers.first().ok_or("missing ordinary label")?)?;
    wrapper
        .children
        .iter()
        .copied()
        .find(|id| {
            matches!(
                tree.node(*id).unwrap().descriptor.view().props,
                Props::Text(_)
            )
        })
        .ok_or_else(|| "label was replaced with a non-Text primitive".into())
}

fn assert_ordinary_primitives(app: &WindowApp) {
    let tree = app.app().tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        assert!(
            !matches!(
                node.descriptor.view().props,
                Props::Graphic(_) | Props::Image(_)
            ),
            "ordinary text acceptance must not substitute Graphic paths or images"
        );
        pending.extend(node.children.iter().copied());
    }
}

fn verify_collision(output: &Path) -> AnyResult {
    let provider = provider();
    let mut app = application(&provider, 1.)?;
    let probe = Rc::new(RefCell::new(HostProbe::new()?));
    let weak = Rc::downgrade(&probe);
    let measured = probe.clone();
    let mut frame = scene()?;
    frame.labels[0] = Label {
        x: 180.,
        y: 180.,
        size: 80.,
        color: Color(180, 20, 20, 255),
        ..label("B")
    };
    frame.labels.push(Label {
        x: 180.,
        y: 180.,
        size: 64.,
        color: Color(20, 20, 180, 255),
        ..label("Ágj")
    });
    frame.typography.labels.insert(
        1,
        FontOptions {
            family: Some(vec![family(BEE)]),
            weight: Some(400.),
        },
    );
    let measured_inputs = Rc::new(RefCell::new(Vec::new()));
    let recorded = measured_inputs.clone();
    let source = app.app().scope().cage(frame);
    app.mount(
        chart(source)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .semantic_text_measurer(SemanticTextProfile::default(), move |request| {
                let result = measured
                    .borrow_mut()
                    .measure(request)
                    .map_err(|error| ChartError(format!("collision host measure: {error}")))?;
                recorded
                    .borrow_mut()
                    .push((request.text.clone(), *result.metrics()));
                Ok(result)
            })
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 520.,
                    height: 260.,
                },
                padding: 3.,
                ..Default::default()
            }),
    )?;
    drop(probe);
    settle(&mut app, &provider)?;
    let ids = nodes(app.app(), "kartis-label");
    assert_ordinary_primitives(&app);
    assert_eq!(ids.len(), 2);
    for wrapper in &ids {
        assert_ne!(
            app.app()
                .tree()
                .node(*wrapper)
                .unwrap()
                .descriptor
                .view()
                .style
                .display,
            Some(Display::None),
            "actual collision host measurements: {:?}",
            measured_inputs.borrow()
        );
        assert!(
            app.app()
                .tree()
                .node(*wrapper)
                .unwrap()
                .children
                .iter()
                .any(|child| matches!(
                    app.app()
                        .tree()
                        .node(*child)
                        .unwrap()
                        .descriptor
                        .view()
                        .props,
                    Props::Text(_)
                ))
        );
    }
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    let red = ink_bounds(&raster, 1., |pixel| {
        pixel[0] > pixel[2].saturating_add(40) && pixel[1] < 180
    })
    .ok_or("collision omitted red actual Text")?;
    let blue = ink_bounds(&raster, 1., |pixel| {
        pixel[2] > pixel[0].saturating_add(40) && pixel[1] < 180
    })
    .ok_or("collision omitted blue actual Text")?;
    assert_eq!(
        red.overlap(blue),
        0.,
        "actual differently ascended Text ink still overlaps: {red:?} / {blue:?}"
    );
    assert!((red.y - blue.y).abs() > 30.);
    // The first baseline remains authored; the second moves using its own
    // measured ink, rather than treating the placed ink box as a line box.
    let first = app.app().tree().node(ids[0]).unwrap().children[0];
    let second = app.app().tree().node(ids[1]).unwrap().children[0];
    assert!((actual_baseline(&app, first)? - 180.).abs() < 1.);
    assert!((actual_baseline(&app, second)? - 180.).abs() > 30.);
    save(
        &raster,
        &output.with_extension("ordinary-baseline-collision.png"),
    )?;
    source.revise(|scene| scene.labels[1].text = "gj".into());
    settle(&mut app, &provider)?;
    assert_eq!(nodes(app.app(), "kartis-label"), ids);
    app.app().clear()?;
    settle(&mut app, &provider)?;
    assert!(weak.upgrade().is_none());
    assert_eq!(app.app().stats().nodes, 0);
    assert_eq!(app.app().stats().handlers, 0);
    Ok(())
}

pub fn verify(output: &Path) -> AnyResult {
    let provider = provider();
    let mut app = application(&provider, 1.)?;
    let probe = Rc::new(RefCell::new(HostProbe::new()?));
    let weak_probe = Rc::downgrade(&probe);
    let measured = probe.clone();
    let source = app.app().scope().cage(scene()?);
    let profile = app.app().scope().cage(SemanticTextProfile::default());
    let hidden = app
        .app()
        .scope()
        .cage(std::collections::BTreeSet::<String>::new());
    let calls = Rc::new(Cell::new(0usize));
    let called = calls.clone();
    let stale = Rc::new(Cell::new(false));
    let fail = Rc::new(Cell::new(false));
    let cached = Rc::new(RefCell::new(None::<SemanticTextMeasurement>));
    let stale_callback = stale.clone();
    let fail_callback = fail.clone();
    let last = cached.clone();
    let rejected = Rc::new(RefCell::new(Vec::new()));
    let errors = rejected.clone();
    app.mount(
        chart(source)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .hidden(hidden)
            .semantic_text_measurer(profile, move |request| {
                called.set(called.get() + 1);
                if fail_callback.get() {
                    return Err(ChartError("native host measurement rejected".into()));
                }
                if stale_callback.get() {
                    return Ok(last.borrow().as_ref().unwrap().clone());
                }
                let result = measured
                    .borrow_mut()
                    .measure(request)
                    .map_err(|error| ChartError(format!("native baseline measurement: {error}")))?;
                *last.borrow_mut() = Some(result.clone());
                Ok(result)
            })
            .on_reject(move |error| errors.borrow_mut().push(error.0)),
    )?;
    drop(probe);
    settle(&mut app, &provider)?;
    let text_id = ordinary_text(&app)?;
    let wrapper_ids = nodes(app.app(), "kartis-label");
    let plot = nodes(app.app(), "kartis-plot")[0];
    let mut raster = Raster::new();
    let fonts = fonts()?;
    let mut records = Vec::new();
    let mut revision = 0;
    for (text, families, weight, size) in [
        ("B", vec![family(FAMILY)], 400., 100.),
        ("B", vec![family(FAMILY)], 700., 100.),
        ("Ágj", vec![family(BEE)], 400., 64.),
        ("BC", vec![family(FAMILY), family(BEE)], 400., 64.),
        (" A  B ", vec![family(FAMILY)], 400., 64.),
    ] {
        for (anchor, x) in [
            (TextAnchor::Start, 50.),
            (TextAnchor::Middle, 260.),
            (TextAnchor::End, 460.),
        ] {
            for ratio in [1., 2.] {
                revision += 1;
                app.renderer().set_viewport(
                    (600. * ratio) as u32,
                    (340. * ratio) as u32,
                    ratio as f32,
                )?;
                weak_probe
                    .upgrade()
                    .unwrap()
                    .borrow_mut()
                    .set_ratio(ratio)?;
                profile.revise(|profile| profile.revision = revision);
                source.revise(|scene| {
                    scene.labels[0] = Label {
                        x,
                        anchor,
                        size,
                        ..label(text)
                    };
                    scene.typography.default = FontOptions {
                        family: Some(families.clone()),
                        weight: Some(weight),
                    };
                });
                settle(&mut app, &provider)?;
                assert!(
                    rejected.borrow().is_empty(),
                    "native baseline rejected: {:?}",
                    rejected.borrow()
                );
                assert_eq!(ordinary_text(&app)?, text_id);
                assert_ordinary_primitives(&app);
                assert_eq!(nodes(app.app(), "kartis-label"), wrapper_ids);
                assert!(nodes(app.app(), "kartis-painted-glyph").is_empty());
                assert!(nodes(app.app(), "kartis-raster-image").is_empty());
                let plot_rect = bounds(&app, plot)?;
                let text_rect = bounds(&app, text_id)?;
                let baseline = actual_baseline(&app, text_id)? - plot_rect.y;
                assert!(
                    (baseline - 140.).abs() <= 1. / ratio,
                    "ordinary baseline {text:?}/{anchor:?}/{ratio}: {baseline}"
                );
                let mut oracle_scene = source.get_untracked();
                // white-space:nowrap collapses HTML spaces. The oracle uses
                // the rendered text, while the ordinary Text keeps its source.
                oracle_scene.labels[0].text =
                    text.split_ascii_whitespace().collect::<Vec<_>>().join(" ");
                let advance = fonts
                    .measure_with_font(
                        &oracle_scene.labels[0].text,
                        size,
                        &oracle_scene.resolved_label_font(0),
                        MissingGlyphPolicy::Error,
                    )?
                    .advance;
                assert!((text_rect.width - advance).abs() <= 1. / ratio + 0.1);
                let expected_left = x - match anchor {
                    TextAnchor::Start => 0.,
                    TextAnchor::Middle => advance / 2.,
                    TextAnchor::End => advance,
                };
                assert!((text_rect.x - plot_rect.x - expected_left).abs() <= 1. / ratio + 0.1);
                raster.draw(app.renderer())?;
                let cpu = render_raster(
                    &oracle_scene,
                    &fonts,
                    &RasterOptions {
                        pixel_ratio: ratio,
                        ..Default::default()
                    },
                )?;
                if text == "B" {
                    assert!(
                        plot_rect.x * ratio + f64::from(cpu.width)
                            <= f64::from(raster.frame().unwrap().width)
                    );
                    assert!(
                        plot_rect.y * ratio + f64::from(cpu.height)
                            <= f64::from(raster.frame().unwrap().height)
                    );
                    for y in 0..cpu.height {
                        for x in 0..cpu.width {
                            let actual = sample(
                                &raster,
                                (plot_rect.x * ratio) as u32 + x,
                                (plot_rect.y * ratio) as u32 + y,
                            );
                            let offset = ((y * cpu.width + x) * 4) as usize;
                            assert!(
                                actual
                                    .iter()
                                    .zip(&cpu.rgba[offset..offset + 4])
                                    .all(|(a, e)| a.abs_diff(*e) <= 1),
                                "ordinary B pixel ({x},{y}) differs at {weight}/{anchor:?}/{ratio}"
                            );
                        }
                    }
                }
                records.push(serde_json::json!({"text":text,"weight":weight,"size":size,"anchor":format!("{anchor:?}"),"dpr":ratio,"baseline":baseline,"advance":text_rect.width,"revision":revision}));
            }
        }
    }
    source.revise(|scene| {
        scene.labels[0] = label("B");
        scene.typography.default = FontOptions {
            family: Some(vec![family(FAMILY)]),
            weight: Some(400.),
        };
    });
    for line_height in [0.65, 1.8] {
        revision += 1;
        profile.revise(|profile| {
            profile.line_height = line_height;
            profile.revision = revision;
        });
        settle(&mut app, &provider)?;
        assert_eq!(ordinary_text(&app)?, text_id);
        let actual = actual_baseline(&app, text_id)? - bounds(&app, plot)?.y;
        assert!(
            (actual - 140.).abs() <= 0.5,
            "changed line-height lost baseline: {actual}"
        );
        let text_rect = bounds(&app, text_id)?;
        assert!((text_rect.height - 100. * line_height).abs() <= 0.5);
        records.push(serde_json::json!({"text":"B","lineHeightRatio":line_height,
            "actualHeight":text_rect.height,"dpr":2,"baseline":actual,"revision":revision}));
    }
    raster.draw(app.renderer())?;
    save(&raster, &output.with_extension("ordinary-baseline.png"))?;
    for blank in ["", " \t  \r\n"] {
        source.revise(|scene| scene.labels[0].text = blank.into());
        settle(&mut app, &provider)?;
        assert_eq!(ordinary_text(&app)?, text_id);
        let metrics = *cached.borrow().as_ref().unwrap().metrics();
        assert_eq!(metrics.advance, 0.);
        assert!(metrics.ink_bounds.is_none());
        assert_eq!(bounds(&app, text_id)?.height, metrics.line_height);
        raster.draw(app.renderer())?;
        assert!(ink_bounds(&raster, 2., |pixel| pixel[..3].iter().any(|c| *c != 255)).is_none());
        records.push(serde_json::json!({"text":blank,"emptyHostInk":true,
            "actualHeight":metrics.line_height,"baselineFromTop":metrics.baseline_from_top,"dpr":2,"revision":revision}));
    }
    let accepted = bounds(&app, text_id)?;
    let accepted_text = app
        .app()
        .tree()
        .node(text_id)
        .unwrap()
        .descriptor
        .view()
        .props
        .clone();
    stale.set(true);
    profile.revise(|profile| profile.revision += 1);
    settle(&mut app, &provider)?;
    assert_eq!(rejected.borrow().len(), 1);
    assert_eq!(bounds(&app, text_id)?, accepted);
    assert_eq!(
        app.app()
            .tree()
            .node(text_id)
            .unwrap()
            .descriptor
            .view()
            .props,
        &accepted_text
    );
    stale.set(false);
    fail.set(true);
    source.revise(|scene| scene.labels[0].text = "AB".into());
    settle(&mut app, &provider)?;
    assert_eq!(rejected.borrow().len(), 2);
    assert_eq!(bounds(&app, text_id)?, accepted);
    fail.set(false);
    profile.revise(|profile| profile.revision += 1);
    settle(&mut app, &provider)?;
    assert_eq!(ordinary_text(&app)?, text_id);
    {
        let tree = app.app().tree();
        let Props::Text(text) = tree.node(text_id).unwrap().descriptor.view().props else {
            unreachable!()
        };
        assert_eq!(text.content, "AB");
    }
    hidden.revise(|hidden| {
        hidden.insert("text".into());
    });
    settle(&mut app, &provider)?;
    assert_eq!(
        app.app()
            .tree()
            .node(wrapper_ids[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    hidden.revise(|hidden| hidden.clear());
    settle(&mut app, &provider)?;
    assert_eq!(ordinary_text(&app)?, text_id);
    app.app().clear()?;
    settle(&mut app, &provider)?;
    assert!(weak_probe.upgrade().is_none());
    let calls_at_cleanup = calls.get();
    source.revise(|scene| scene.labels[0].text = "stale callback".into());
    assert_eq!(calls.get(), calls_at_cleanup);
    assert_eq!(app.app().stats().nodes, 0);
    assert_eq!(app.app().stats().handlers, 0);
    verify_collision(output)?;
    std::fs::write(
        output.with_extension("ordinary-baseline.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "passed":true,"actualApaceLineBaseline":true,"ordinaryTextRetained":true,
            "staleMeasurementRejected":true,"measurementFailureAtomic":true,"hiddenRestore":true,
            "callbackCleanup":true,"actualInkCollision":true,"cases":records,
        }))?,
    )?;
    Ok(())
}
