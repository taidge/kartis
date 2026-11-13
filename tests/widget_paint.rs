use std::cell::Cell;
use std::rc::Rc;

use glory::App;
use glory::render::{
    Event, GraphicPoint as P, GraphicProps, GraphicStroke, NodeId, PathCommand as C, Props,
    StrokeCap,
};
use glory::style::Display;
use kartis::*;
fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "datum",
        Geometry::Rect {
            x: 20.,
            y: 20.,
            width: 80.,
            height: 40.,
        },
        Color(210, 50, 30, 255),
    );
    mark.series = "series".into();
    mark.label = "Revenue".into();
    mark.value = 12.;
    mark.opacity = 0.5;
    mark.stroke = Some(GraphicStroke {
        color: Color(20, 40, 150, 255),
        width: 6.,
        cap: StrokeCap::Round,
    });
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![8., 4.],
        offset: 1.,
    });
    scene.marks.push(mark);
    scene.labels.push(Label {
        x: 30.,
        y: 90.,
        text: "Label".into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut output = vec![];
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            output.push(id);
        }
        stack.extend(node.children.iter().rev().copied());
    }
    output
}
fn parts(app: &App) -> Vec<GraphicProps> {
    nodes(app, "kartis-dash-part")
        .into_iter()
        .map(|id| {
            let tree = app.tree();
            let Props::Graphic(props) = tree.node(id).unwrap().descriptor.view().props else {
                panic!()
            };
            props.clone()
        })
        .collect()
}

#[test]
fn dashed_semantic_graphics_keep_original_fill_separate_from_stroke_and_group_opacity() {
    let scene = scene();
    let app = App::memory();
    app.mount(chart(scene.clone())).unwrap();
    let parts = parts(&app);
    assert!(parts.len() >= 2);
    assert_eq!(parts[0].commands, scene.marks[0].geometry.commands());
    assert_eq!(parts[0].fill, scene.marks[0].fill);
    assert!(parts[0].stroke.is_none());
    let detailed = stroke_dash::dash_geometry_detailed(
        &scene.marks[0].geometry,
        scene.marks[0].stroke_dash.as_ref().unwrap(),
        &StrokeDashLimits::default(),
    )
    .unwrap();
    let expected = detailed.stroke_geometry().commands();
    let actual = parts
        .iter()
        .filter(|p| p.stroke.is_some())
        .flat_map(|p| p.commands.clone())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert!(parts.iter().skip(1).all(|p| p.fill.is_none()));
    let group = nodes(&app, "kartis-mark-parts")[0];
    assert_eq!(
        app.tree()
            .node(group)
            .unwrap()
            .descriptor
            .view()
            .style
            .opacity,
        Some(0.5)
    );
    for id in nodes(&app, "kartis-dash-part") {
        assert!(
            app.tree()
                .node(id)
                .unwrap()
                .descriptor
                .view()
                .style
                .opacity
                .is_none()
        );
    }
    let normal = nodes(&app, "kartis-mark")[0];
    assert_eq!(
        app.tree()
            .node(normal)
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    let html = glory::ssr::render(chart(scene)).unwrap();
    assert!(html.contains("kartis-dash-part") && html.contains("Revenue: 12"));
}

#[test]
fn square_zero_dashes_use_tangent_footprints_without_zero_length_stroke_duplicates() {
    let mut scene = scene();
    let mark = &mut scene.marks[0];
    mark.fill = None;
    mark.geometry = Geometry::Path(vec![C::Move(P::new(20., 20.)), C::Line(P::new(80., 80.))]);
    mark.stroke.as_mut().unwrap().cap = StrokeCap::Square;
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![0., 10.],
        offset: 0.,
    });
    let detailed = stroke_dash::dash_geometry_detailed(
        &mark.geometry,
        mark.stroke_dash.as_ref().unwrap(),
        &StrokeDashLimits::default(),
    )
    .unwrap();
    assert!(!detailed.dots.is_empty());
    let expected = detailed
        .dots
        .iter()
        .flat_map(|dot| {
            stroke_dash::dash_dot_geometry(dot, 6., StrokeCap::Square)
                .unwrap()
                .commands()
        })
        .collect::<Vec<_>>();
    let app = App::memory();
    app.mount(chart(scene)).unwrap();
    let parts = parts(&app);
    assert!(
        parts
            .iter()
            .all(|p| p.stroke.is_none() && p.fill == Some(Color(20, 40, 150, 255)))
    );
    assert_eq!(
        parts
            .into_iter()
            .flat_map(|p| p.commands)
            .collect::<Vec<_>>(),
        expected
    );
    let C::Move(first) = expected[0] else {
        panic!()
    };
    assert!((first.x - (20. - 3. / 2_f64.sqrt())).abs() < 1e-8);
    assert!((first.y - (20. + 3. / 2_f64.sqrt())).abs() < 1e-8);
}

#[test]
fn many_dash_subpaths_pack_within_graphic_budget_without_splitting_runs() {
    let mut scene = scene();
    scene.width = 2400.;
    let mark = &mut scene.marks[0];
    mark.fill = None;
    mark.geometry = Geometry::Path(vec![C::Move(P::new(20., 50.)), C::Line(P::new(2020., 50.))]);
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![4., 4.],
        offset: 0.,
    });
    let expected = stroke_dash::dash_geometry_detailed(
        &mark.geometry,
        mark.stroke_dash.as_ref().unwrap(),
        &StrokeDashLimits::default(),
    )
    .unwrap()
    .stroke_geometry()
    .commands();
    assert!(expected.len() > 256);
    let app = App::memory();
    app.mount(chart(scene)).unwrap();
    let parts = parts(&app);
    assert!(parts.len() > 1);
    assert!(
        parts
            .iter()
            .all(|p| p.commands.len() <= 256 && matches!(p.commands.first(), Some(C::Move(_))))
    );
    assert_eq!(
        parts
            .into_iter()
            .flat_map(|p| p.commands)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn dash_offset_and_solid_transitions_update_current_controls_and_retire_parts() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let selected = Rc::new(Cell::new(0.));
    let capture = selected.clone();
    app.mount(chart(source).on_select(move |m| capture.set(m.value)))
        .unwrap();
    let before = parts(&app);
    let button = nodes(&app, "kartis-datum")[0];
    let wrapper = nodes(&app, "kartis-mark-clip")[0];
    source.revise(|s| {
        s.marks[0].stroke_dash.as_mut().unwrap().offset = 4.;
        s.marks[0].value = 55.;
    });
    app.flush().unwrap();
    assert_ne!(before, parts(&app));
    assert_eq!(wrapper, nodes(&app, "kartis-mark-clip")[0]);
    app.dispatch(button, Event::Activate).unwrap();
    assert_eq!(selected.get(), 55.);
    source.revise(|s| s.marks[0].stroke_dash = None);
    app.flush().unwrap();
    assert!(parts(&app).is_empty());
    let normal = nodes(&app, "kartis-mark")[0];
    assert_ne!(
        app.tree()
            .node(normal)
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    source.revise(|s| {
        s.marks[0].stroke_dash = Some(StrokeDash {
            segments: vec![0., 8.],
            offset: 0.,
        })
    });
    app.flush().unwrap();
    assert!(!parts(&app).is_empty());
    app.clear().unwrap();
    assert_eq!((app.stats().nodes, app.stats().handlers), (0, 0));
    source.revise(|s| s.marks.clear());
    app.flush().unwrap();
    assert_eq!((app.stats().nodes, app.stats().handlers), (0, 0));
}

#[test]
fn unsupported_masks_and_patterns_reject_atomically_but_external_controls_allow_them() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let rejected = Rc::new(Cell::new(0));
    let captured = rejected.clone();
    app.mount(chart(source).on_reject(move |_| captured.set(captured.get() + 1)))
        .unwrap();
    let accepted = parts(&app);
    let button = nodes(&app, "kartis-datum")[0];
    source.revise(|s| {
        s.marks[0].clip_path = Some(Box::new(ClipPath::circle(50., 40., 20.)));
        s.marks[0].value = 999.;
    });
    app.flush().unwrap();
    assert_eq!(rejected.get(), 1);
    assert_eq!(parts(&app), accepted);
    let tree = app.tree();
    let Props::Button(props) = tree.node(button).unwrap().descriptor.view().props else {
        panic!()
    };
    assert!(!props.label.contains("999"));
    drop(tree);
    let mut masked = scene();
    masked.marks[0].clip_path = Some(Box::new(ClipPath::circle(50., 40., 20.)));
    assert!(glory::ssr::render(chart(masked.clone())).is_err());
    assert!(glory::ssr::render(chart(masked).plot(false)).is_ok());
    let mut labels = scene();
    labels
        .label_clips
        .insert(0, ClipPath::circle(50., 40., 20.));
    assert!(glory::ssr::render(chart(labels.clone())).is_err());
    assert!(glory::ssr::render(chart(labels).plot(false)).is_ok());
    let mut pattern = scene();
    pattern.marks[0].fill_paint = Some(FillPaint::Pattern(PatternFill::default()));
    assert!(glory::ssr::render(chart(pattern.clone())).is_err());
    assert!(glory::ssr::render(chart(pattern).plot(false)).is_ok());
}

#[test]
fn one_over_budget_painted_run_rejects_before_replacing_scene_or_controls() {
    let mut large = scene();
    large.width = 10000.;
    large.height = 10000.;
    let mark = &mut large.marks[0];
    mark.fill = None;
    mark.geometry = Geometry::Circle {
        x: 5000.,
        y: 5000.,
        radius: 4000.,
    };
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![100000., 1.],
        offset: 0.,
    });
    large.validate().unwrap();
    assert!(glory::ssr::render(chart(large.clone())).is_err());
    let app = App::memory();
    let source = app.scope().cage(scene());
    let rejected = Rc::new(Cell::new(0));
    let captured = rejected.clone();
    app.mount(chart(source).on_reject(move |_| captured.set(captured.get() + 1)))
        .unwrap();
    let accepted = parts(&app);
    source.revise(|s| *s = large);
    app.flush().unwrap();
    assert_eq!(rejected.get(), 1);
    assert_eq!(parts(&app), accepted);
}

#[test]
fn partial_alpha_stroke_colors_reject_multi_graphic_ink_but_mark_opacity_is_grouped() {
    let mut scene = scene();
    let mark = &mut scene.marks[0];
    mark.fill = None;
    mark.geometry = Geometry::Path(vec![C::Move(P::new(20., 20.)), C::Line(P::new(120., 20.))]);
    mark.stroke.as_mut().unwrap().color = Color(255, 0, 0, 128);
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![5., 0., 0., 5.],
        offset: 0.,
    });
    scene.validate().unwrap();
    assert!(glory::ssr::render(chart(scene.clone())).is_err());
    assert!(glory::ssr::render(chart(scene.clone()).plot(false)).is_ok());
    scene.marks[0].stroke.as_mut().unwrap().color = Color(255, 0, 0, 255);
    scene.marks[0].opacity = 0.5;
    assert!(glory::ssr::render(chart(scene.clone())).is_ok());
    scene.marks[0].stroke.as_mut().unwrap().color = Color(255, 0, 0, 128);
    scene.marks[0].stroke_dash = Some(StrokeDash {
        segments: vec![0., 10.],
        offset: 0.,
    });
    assert!(
        glory::ssr::render(chart(scene)).is_ok(),
        "all dots in one filled Graphic form one alpha union"
    );
}
