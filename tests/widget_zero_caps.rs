use glory::App;
use glory::render::{GraphicProps, GraphicStroke, Props, StrokeCap};
use glory::style::Display;
use kartis::*;

#[test]
fn semantic_widget_keeps_one_translucent_cap_and_svg2_dash_endpoint_without_losing_data() {
    for cap in [StrokeCap::Round, StrokeCap::Square] {
        for (pattern, offset, visible) in [
            (vec![3., 3.], 0., true),
            (vec![3., 3.], 3., true),
            (vec![3., 3.], 4., false),
            (vec![0., 3.], 0., true),
            (vec![0., 3.], 1., false),
        ] {
            let mut scene = ChartConfig {
                width: 128.,
                height: 128.,
                ..Default::default()
            }
            .scene()
            .unwrap();
            let mut point = Mark::new(
                "point",
                Geometry::Circle {
                    x: 64.,
                    y: 64.,
                    radius: 0.,
                },
                Color(0, 0, 0, 0),
            );
            point.fill = None;
            point.label = "current zero-size datum".into();
            point.value = 7.;
            point.opacity = 0.5;
            point.stroke = Some(GraphicStroke {
                color: Color(220, 30, 20, 128),
                width: 100.,
                cap,
            });
            point.stroke_dash = Some(StrokeDash {
                segments: pattern,
                offset,
            });
            scene.marks.push(point);
            let app = App::memory();
            app.mount(chart(scene)).unwrap();
            let (parts, original_hidden, group_opacity, controls) = {
                let tree = app.tree();
                let mut pending = tree.roots().to_vec();
                let mut parts: Vec<GraphicProps> = vec![];
                let mut original_hidden = false;
                let mut group_opacity = None;
                let mut controls = 0;
                while let Some(id) = pending.pop() {
                    let node = tree.node(id).unwrap();
                    let view = node.descriptor.view();
                    let class = |name| view.classes.iter().any(|c| c == name);
                    if class("kartis-dash-part") {
                        let Props::Graphic(props) = &view.props else {
                            panic!("cap must be a real Graphic");
                        };
                        assert!(view.style.opacity.is_none(), "opacity belongs to the group");
                        parts.push(props.clone());
                    } else if class("kartis-mark") {
                        original_hidden |= view.style.display == Some(Display::None);
                    }
                    if class("kartis-mark-parts") {
                        group_opacity = view.style.opacity;
                    }
                    if class("kartis-datum") {
                        controls += 1;
                    }
                    pending.extend(node.children.iter().copied());
                }
                (parts, original_hidden, group_opacity, controls)
            };
            assert!(original_hidden);
            assert_eq!(group_opacity, Some(0.5));
            assert_eq!(controls, 1, "a nonpainted datum still has its data control");
            assert_eq!(parts.len(), usize::from(visible));
            if let Some(paint) = parts.first() {
                assert_eq!(paint.fill, Some(Color(220, 30, 20, 128)));
                assert!(
                    paint.stroke.is_none(),
                    "the cap must not be painted a second time"
                );
                let mut probe = ChartConfig {
                    width: 128.,
                    height: 128.,
                    ..Default::default()
                }
                .scene()
                .unwrap();
                let mark = Mark::new(
                    "ink",
                    Geometry::Path(paint.commands.clone()),
                    paint.fill.unwrap(),
                );
                probe.marks.push(mark);
                assert!(probe.hit_test(64., 64.).is_some());
                assert_eq!(
                    probe.hit_test(109., 109.).is_some(),
                    cap == StrokeCap::Square
                );
            }
        }
    }
}
