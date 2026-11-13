use glory::render::{Event, GraphicStroke, NodeId, Props};
use kartis::{
    render::{self, Canvas},
    *,
};
use std::{cell::Cell, rc::Rc};

fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "a",
        Geometry::Rect {
            x: 10.,
            y: 10.,
            width: 30.,
            height: 25.,
        },
        Color(78, 121, 167, 255),
    );
    mark.label = "Revenue".into();
    mark.value = 12.;
    scene.marks.push(mark);
    scene.labels.push(Label {
        rotation: 0.,
        x: 25.,
        y: 60.,
        text: "Revenue".into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
    });
    scene.legend.push(LegendItem {
        id: "series".into(),
        label: "Revenue".into(),
        color: Color(78, 121, 167, 255),
    });
    scene
}
#[derive(Default)]
struct Recorder {
    events: Vec<&'static str>,
    fail: bool,
}
impl Canvas for Recorder {
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.events.push("begin");
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        self.events.push("path");
        Ok(())
    }
    fn move_to(&mut self, _: f64, _: f64) -> Result<()> {
        self.events.push("move");
        Ok(())
    }
    fn line_to(&mut self, _: f64, _: f64) -> Result<()> {
        self.events.push("line");
        Ok(())
    }
    fn cubic_to(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: f64) -> Result<()> {
        self.events.push("cubic");
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        self.events.push("close");
        Ok(())
    }
    fn fill(&mut self, _: Color, _: f64) -> Result<()> {
        self.events.push("fill");
        if self.fail {
            Err(ChartError("host fault".into()))
        } else {
            Ok(())
        }
    }
    fn stroke(&mut self, _: GraphicStroke, _: f64) -> Result<()> {
        self.events.push("stroke");
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        self.events.push("text");
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.events.push("finish");
        Ok(())
    }
}
#[test]
fn raster_replay_orders_geometry_then_text_and_releases_on_host_error() {
    let s = scene();
    let mut recorder = Recorder::default();
    render::draw(&s, &mut recorder).unwrap();
    assert_eq!(
        recorder.events,
        vec![
            "begin", "path", "move", "line", "line", "line", "close", "fill", "text", "finish"
        ]
    );
    let mut failing = Recorder {
        fail: true,
        ..Default::default()
    };
    assert!(render::draw(&s, &mut failing).is_err());
    assert_eq!(failing.events.last(), Some(&"finish"));
    assert!(!failing.events.contains(&"text"));
}
#[test]
fn invalid_proposals_do_not_mutate_any_renderer() {
    let mut s = scene();
    s.marks[0].opacity = f64::NAN;
    let mut recorder = Recorder::default();
    assert!(render::draw(&s, &mut recorder).is_err());
    assert!(recorder.events.is_empty());
    assert!(render::to_svg(&s).is_err());
    assert!(render::to_canvas_script(&s, "canvas").is_err());
}
#[test]
fn rotated_text_has_matching_svg_canvas_anchor_and_explicit_host_capability() {
    let mut s = scene();
    s.labels[0].rotation = -45.;
    let svg = render::to_svg(&s).unwrap();
    assert!(svg.contains("transform=\"rotate(-45 25 60)\""));
    let script = render::to_canvas_script(&s, "canvas").unwrap();
    assert!(script.contains("ctx.save();ctx.translate(25,60);ctx.rotate(-0.7853981633974483);"));
    assert!(script.contains("ctx.fillText(\"Revenue\",0,0);ctx.restore();"));
    let mut unsupported = Recorder::default();
    assert!(render::draw(&s, &mut unsupported).is_err());
    assert!(unsupported.events.is_empty());
    assert!(glory::ssr::render(chart(s.clone())).is_err());
    assert!(glory::ssr::render(chart(s.clone()).plot(false)).is_ok());
    for angle in [f64::NAN, f64::INFINITY, 361.] {
        s.labels[0].rotation = angle;
        assert!(s.validate().is_err());
    }
}
#[test]
fn svg_and_canvas_exports_escape_all_untrusted_text() {
    let mut s = scene();
    s.title = "</script><img src=x onerror=alert(1)>\"&".into();
    s.marks[0].id = "\" onload=\"bad".into();
    s.marks[0].label = s.title.clone();
    s.labels[0].text = s.title.clone();
    let svg = render::to_svg(&s).unwrap();
    assert!(!svg.contains("<img"));
    assert!(svg.contains("&lt;/script&gt;"));
    assert!(svg.contains("data-id=\"&quot; onload=&quot;bad\""));
    let script = render::to_canvas_script(&s, "canvas</script>\"\\\n").unwrap();
    assert!(!script.contains('<'));
    assert!(script.contains("\\u003c/script\\u003e"));
    assert!(script.contains("canvas.getContext('2d')"));
}
#[test]
fn glory_ssr_contains_vector_geometry_labels_legend_and_data() {
    let html = glory::ssr::render(chart(scene())).unwrap();
    assert!(html.contains("<svg"));
    assert!(html.contains("kartis-label"));
    assert!(html.contains("Chart legend"));
    assert!(html.contains("Revenue: 12"));
    assert!(!html.contains("<canvas"));
}
fn button(app: &glory::App) -> NodeId {
    let tree = app.tree();
    let mut stack = tree.roots().to_vec();
    while let Some(id) = stack.pop() {
        let n = tree.node(id).unwrap();
        if matches!(n.descriptor.view().props, Props::Button(_)) {
            return id;
        }
        stack.extend(n.children.iter().copied());
    }
    panic!("data button missing")
}
#[test]
fn reactive_scene_retains_previous_on_rejection_then_recovers_and_cleans_up() {
    let app = glory::App::memory();
    let source = app.scope().cage(scene());
    let rejects = Rc::new(Cell::new(0));
    let observed = rejects.clone();
    let selected = Rc::new(Cell::new(0.));
    let captured = selected.clone();
    app.mount(
        chart(source)
            .on_reject(move |_| observed.set(observed.get() + 1))
            .on_select(move |m| captured.set(m.value)),
    )
    .unwrap();
    let node = button(&app);
    app.dispatch(node, Event::Activate).unwrap();
    assert_eq!(selected.get(), 12.);
    source.revise(|s| {
        s.marks[0].value = 23.;
    });
    app.flush().unwrap();
    assert_eq!(button(&app), node);
    app.dispatch(node, Event::Activate).unwrap();
    assert_eq!(selected.get(), 23.);
    source.revise(|s| s.width = f64::NAN);
    app.flush().unwrap();
    assert_eq!(rejects.get(), 1);
    app.dispatch(node, Event::Activate).unwrap();
    assert_eq!(selected.get(), 23.);
    source.revise(|s| {
        s.width = 800.;
        s.marks[0].value = 31.;
    });
    app.flush().unwrap();
    app.dispatch(node, Event::Activate).unwrap();
    assert_eq!(selected.get(), 31.);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
    source.revise(|s| s.marks.clear());
    app.flush().unwrap();
    assert_eq!(rejects.get(), 1);
}
#[test]
fn glory_rejects_over_budget_paths_before_mounting() {
    let mut s = scene();
    s.marks[0].geometry = Geometry::Path(
        std::iter::once(glory::render::PathCommand::Move(
            glory::render::GraphicPoint::new(0., 0.),
        ))
        .chain((0..256).map(|i| {
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(i as f64, 0.))
        }))
        .collect(),
    );
    assert!(render::to_svg(&s).is_ok());
    assert!(glory::ssr::render(chart(s)).is_err());
}

#[test]
fn foreign_runtime_sources_are_rejected_before_installing_bindings() {
    let foreign = glory::App::memory();
    let source = foreign.scope().cage(scene());
    let local = glory::App::memory();
    assert!(local.mount(chart(source)).is_err());
    assert_eq!(local.stats().nodes, 0);
    assert_eq!(local.stats().handlers, 0);
}
