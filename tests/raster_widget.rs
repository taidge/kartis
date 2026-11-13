#![cfg(feature = "raster")]
use base64::Engine;
use glory::{
    App,
    render::{Event, NodeId, Props},
    style::Length,
};
use kartis::{render::raster::*, *};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    io::Cursor,
    rc::Rc,
};

fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
}
fn scene() -> Scene {
    let mut s = ChartConfig {
        width: 200.,
        height: 120.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        title: "CPU PNG chart".into(),
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, x, color) in [
        ("alpha", 20., Color(220, 30, 40, 255)),
        ("beta", 100., Color(20, 160, 70, 255)),
    ] {
        let mut m = Mark::new(
            id,
            Geometry::Rect {
                x,
                y: 20.,
                width: 50.,
                height: 45.,
            },
            color,
        );
        m.series = id.into();
        m.label = id.into();
        m.value = 1.25;
        s.marks.push(m);
        s.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color,
        });
        s.label_series.insert(s.labels.len(), id.into());
        s.labels.push(Label {
            x: x + 25.,
            y: 95.,
            text: id.into(),
            color: Color(20, 30, 40, 255),
            size: 12.,
            anchor: TextAnchor::Middle,
            rotation: 15.,
        });
    }
    s
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut found = Vec::new();
    while let Some(id) = stack.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        stack.extend(n.children.iter().rev().copied());
    }
    found
}
fn source(app: &App) -> String {
    let tree = app.tree();
    let node = nodes(app, "kartis-raster-image")[0];
    let Props::Image(image) = tree.node(node).unwrap().descriptor.view().props else {
        panic!("real Glory image missing")
    };
    image.source.clone()
}
fn decode(source: &str) -> (u32, u32, Vec<u8>) {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(
            source
                .strip_prefix("data:image/png;base64,")
                .expect("PNG data URL"),
        )
        .unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut output = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut output).unwrap();
    output.truncate(info.buffer_size());
    (info.width, info.height, output)
}
fn pixel(source: &str, x: usize, y: usize) -> [u8; 4] {
    let (w, _, pixels) = decode(source);
    pixels[((y * w as usize + x) * 4)..][..4]
        .try_into()
        .unwrap()
}

#[test]
fn raster_is_real_png_in_a_glory_image_with_existing_controls_and_rotated_text() {
    let app = App::memory();
    app.mount(raster_chart(scene(), fonts(), RasterOptions::default()))
        .unwrap();
    let image = source(&app);
    assert_eq!(decode(&image).0, 200);
    assert_eq!(pixel(&image, 30, 30), [220, 30, 40, 255]);
    assert_eq!(pixel(&image, 110, 30), [20, 160, 70, 255]);
    assert_eq!(nodes(&app, "kartis-raster-image").len(), 1);
    assert!(nodes(&app, "kartis-mark").is_empty());
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    assert_eq!(nodes(&app, "kartis-legend-item").len(), 2);
    let html =
        glory::ssr::render(raster_chart(scene(), fonts(), RasterOptions::default())).unwrap();
    assert!(html.contains("<img"));
    assert!(html.contains("data:image/png;base64,"));
    assert!(!html.contains("<svg"));
    assert!(html.contains("CPU PNG chart"));
}

#[test]
fn reactive_scene_dimensions_and_dpr_change_encoded_pixels_and_logical_image_size() {
    let app = App::memory();
    let scene = app.scope().cage(scene());
    let options = app.scope().cage(RasterOptions::default());
    app.mount(raster_chart(scene, fonts(), options)).unwrap();
    let node = nodes(&app, "kartis-raster-image")[0];
    let old = source(&app);
    scene.revise(|s| {
        s.width = 240.;
        s.marks[0].fill = Some(Color(30, 60, 210, 255));
        s.marks[0].value = 99.;
    });
    options.revise(|o| o.pixel_ratio = 2.);
    app.flush().unwrap();
    let image = source(&app);
    assert_ne!(image, old);
    let (w, h, _) = decode(&image);
    assert_eq!((w, h), (480, 240));
    assert_eq!(pixel(&image, 60, 60), [30, 60, 210, 255]);
    assert_eq!(nodes(&app, "kartis-raster-image")[0], node);
    let tree = app.tree();
    let style = tree.node(node).unwrap().descriptor.view().style;
    assert_eq!(style.width, Some(Length::Px(240.)));
    assert_eq!(style.height, Some(Length::Px(120.)));
    let data = tree
        .node(nodes(&app, "kartis-datum")[0])
        .unwrap()
        .descriptor
        .view();
    let Props::Button(button) = data.props else {
        panic!("datum")
    };
    assert_eq!(button.label, "alpha: 99");
}

#[test]
fn uncontrolled_legend_hides_raster_pixels_owned_labels_and_data_as_one_model() {
    let app = App::memory();
    app.mount(raster_chart(scene(), fonts(), RasterOptions::default()))
        .unwrap();
    let original = source(&app);
    let legend = nodes(&app, "kartis-legend-item")[0];
    app.dispatch(legend, Event::Activate).unwrap();
    let hidden = source(&app);
    assert_ne!(hidden, original);
    assert_eq!(pixel(&hidden, 30, 30), [255, 255, 255, 255]);
    assert_eq!(pixel(&hidden, 110, 30), [20, 160, 70, 255]);
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(source(&app), original);
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
}

#[test]
fn invalid_scene_options_and_failed_source_publication_retain_the_whole_previous_frame() {
    let app = App::memory();
    let scene = app.scope().cage(scene());
    let options = app.scope().cage(RasterOptions::default());
    let reject = Rc::new(Cell::new(false));
    let flag = reject.clone();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let observed = errors.clone();
    app.mount(
        raster_chart(scene, fonts(), options)
            .raster_image_source(move |frame| {
                if flag.get() {
                    Err(ChartError("publisher denied".into()))
                } else {
                    frame.data_url()
                }
            })
            .on_reject(move |error| observed.borrow_mut().push(error.0)),
    )
    .unwrap();
    let old = source(&app);
    options.revise(|o| o.pixel_ratio = f64::NAN);
    scene.revise(|s| {
        s.width = 240.;
        s.marks[0].value = 99.;
    });
    app.flush().unwrap();
    assert_eq!(source(&app), old);
    let tree = app.tree();
    let Props::Button(button) = tree
        .node(nodes(&app, "kartis-datum")[0])
        .unwrap()
        .descriptor
        .view()
        .props
    else {
        panic!("datum")
    };
    assert_eq!(button.label, "alpha: 1.25");
    drop(tree);
    options.revise(|o| o.pixel_ratio = 1.);
    reject.set(true);
    app.flush().unwrap();
    assert_eq!(source(&app), old);
    assert!(
        errors
            .borrow()
            .iter()
            .any(|e| e.contains("publisher denied"))
    );
    reject.set(false);
    scene.revise(|s| s.title = "Retry".into());
    app.flush().unwrap();
    assert_ne!(source(&app), old);
    assert_eq!(decode(&source(&app)).0, 240);
}

#[test]
fn failed_uncontrolled_legend_redraw_does_not_emit_or_hide_controls() {
    let app = App::memory();
    let fail = Rc::new(Cell::new(false));
    let flag = fail.clone();
    let proposals = Rc::new(Cell::new(0));
    let observed = proposals.clone();
    let errors = Rc::new(Cell::new(0));
    let rejected = errors.clone();
    app.mount(
        raster_chart(scene(), fonts(), RasterOptions::default())
            .raster_image_source(move |frame| {
                if flag.get() {
                    Err(ChartError("publisher failure".into()))
                } else {
                    frame.data_url()
                }
            })
            .on_hidden_change(move |_| observed.set(observed.get() + 1))
            .on_reject(move |_| rejected.set(rejected.get() + 1)),
    )
    .unwrap();
    let old = source(&app);
    fail.set(true);
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert_eq!(source(&app), old);
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    assert_eq!(proposals.get(), 0);
    assert_eq!(errors.get(), 1);
}

#[test]
fn controlled_hidden_proposes_then_commits_and_failures_retain_accepted_visibility() {
    let app = App::memory();
    let hidden = app.scope().cage(BTreeSet::new());
    let proposals = Rc::new(RefCell::new(Vec::new()));
    let observed = proposals.clone();
    let options = app.scope().cage(RasterOptions::default());
    app.mount(
        raster_chart(scene(), fonts(), options)
            .hidden(hidden)
            .on_hidden_change(move |h| observed.borrow_mut().push(h))
            .on_reject(|_| {}),
    )
    .unwrap();
    let old = source(&app);
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert_eq!(source(&app), old);
    assert_eq!(proposals.borrow().len(), 1);
    hidden.revise(|h| *h = proposals.borrow()[0].clone());
    app.flush().unwrap();
    let accepted = source(&app);
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
    options.revise(|o| o.pixel_ratio = f64::INFINITY);
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert_eq!(source(&app), accepted);
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
}

#[test]
fn publisher_captures_retire_with_owner_and_foreign_options_cannot_mount() {
    struct DropMarker(Rc<Cell<usize>>);
    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let app = App::memory();
    let source = app.scope().cage(scene());
    let drops = Rc::new(Cell::new(0));
    let captured = DropMarker(drops.clone());
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    app.mount(
        raster_chart(source, fonts(), RasterOptions::default()).raster_image_source(move |frame| {
            let _ = &captured;
            count.set(count.get() + 1);
            frame.data_url()
        }),
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    app.clear().unwrap();
    assert_eq!(drops.get(), 1);
    source.revise(|s| s.title = "retired".into());
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
    let other = App::memory();
    assert!(
        app.mount(raster_chart(
            scene(),
            fonts(),
            other.scope().cage(RasterOptions::default())
        ))
        .is_err()
    );
    assert_eq!(app.stats().nodes, 0);
}

#[test]
fn raster_plot_accepts_long_paths_without_allocating_semantic_graphics() {
    use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
    let mut scene = scene();
    let mut path = Mark::new(
        "long",
        Geometry::Path(
            (0..=600)
                .map(|i| {
                    if i == 0 {
                        PathCommand::Move(GraphicPoint::new(0., 110.))
                    } else {
                        PathCommand::Line(GraphicPoint::new(i as f64 / 3., 110.))
                    }
                })
                .collect(),
        ),
        Color(30, 40, 50, 255),
    );
    path.fill = None;
    path.stroke = Some(GraphicStroke {
        color: Color(30, 40, 50, 255),
        width: 1.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(path);
    let app = App::memory();
    app.mount(raster_chart(scene, fonts(), RasterOptions::default()))
        .unwrap();
    assert_eq!(nodes(&app, "kartis-raster-image").len(), 1);
    assert!(nodes(&app, "kartis-mark").is_empty());
    assert_eq!(pixel(&source(&app), 100, 110)[3], 255);
}

#[test]
fn publisher_can_retire_its_owner_without_committing_or_reentering_callbacks() {
    let app = Rc::new(App::memory());
    let scene = app.scope().cage(scene());
    let retire = Rc::new(Cell::new(false));
    let flag = retire.clone();
    let weak = Rc::downgrade(&app);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let rejects = Rc::new(Cell::new(0));
    let errors = rejects.clone();
    app.mount(
        raster_chart(scene, fonts(), RasterOptions::default())
            .raster_image_source(move |frame| {
                counter.set(counter.get() + 1);
                if flag.get() {
                    assert!(matches!(
                        weak.upgrade().unwrap().clear(),
                        Err(glory::ui::Error::Access(
                            glory::core::AccessError::ReentrantFlush
                        ))
                    ));
                }
                frame.data_url()
            })
            .on_reject(move |_| errors.set(errors.get() + 1)),
    )
    .unwrap();
    retire.set(true);
    scene.revise(|s| s.title = "retire while publishing".into());
    app.flush().unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(rejects.get(), 0);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
    scene.revise(|s| s.title = "already retired".into());
    app.flush().unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn controls_only_updates_do_not_repeat_png_publication() {
    let app = App::memory();
    let hovered = app.scope().cage(None);
    let selected = app.scope().cage(None);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    app.mount(
        raster_chart(scene(), fonts(), RasterOptions::default())
            .hovered(hovered)
            .selected(selected)
            .raster_image_source(move |frame| {
                counter.set(counter.get() + 1);
                frame.data_url()
            }),
    )
    .unwrap();
    hovered.revise(|v| *v = Some("alpha".into()));
    selected.revise(|v| *v = Some("beta".into()));
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
}

#[test]
fn initial_publication_can_retire_its_construction_owner_without_panicking() {
    use glory::Widget;
    let app = App::memory();
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let result = app.mount(glory::ui::widget(move |scope| {
        let owner = scope.downgrade();
        let count = counter.clone();
        raster_chart(scene(), fonts(), RasterOptions::default())
            .raster_image_source(move |frame| {
                count.set(count.get() + 1);
                owner.upgrade().unwrap().owner().dispose();
                frame.data_url()
            })
            .show_in(scope)
    }));
    assert!(result.is_err());
    assert_eq!(calls.get(), 1);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
