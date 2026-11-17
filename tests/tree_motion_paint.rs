use std::cell::Cell;
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as Point, GraphicStroke, PathCommand as Command, StrokeCap};
use kartis::*;

fn source() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("a", Some("branch".into()), -0.),
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("branch", Some("r".into()), 0.),
        HierarchyDatum::new("b", Some("r".into()), 3.),
        HierarchyDatum::new("c", Some("branch".into()), 5.),
    ]
}

fn layout(
    rows: &[HierarchyDatum],
    shape: TreeNodeShape,
    orientation: TreeOrientation,
    curve: TreeLinkCurve,
    next: bool,
) -> (ChartConfig, TreeLayout) {
    let config = ChartConfig {
        width: 520.,
        height: 360.,
        margin: Margin {
            left: if next { 43. } else { 20. },
            top: 31.,
            right: 27.,
            bottom: 39.,
        },
        ..Default::default()
    };
    let mut options = TreeOptions {
        orientation,
        ..Default::default()
    };
    options.hierarchy.labels = true;
    options.style.nodes.shape = shape;
    options.style.nodes.border = Some(GraphicStroke {
        color: Color(7, 17, 27, 255),
        width: 1.25,
        cap: StrokeCap::Square,
    });
    options.style.links.curve = curve;
    options.style.labels.rotation = TreeLabelRotation::Fixed(if next { 70. } else { 0. });
    options.style.clip_plot = true;
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(if next { 22. } else { 14. });
    let mut color = |_: &TreeNodeFunctionContext<'_>| {
        Ok(if next {
            Color(110, 40, 80, 200)
        } else {
            Color(20, 100, 150, 255)
        })
    };
    let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(if next { 6. } else { 2. });
    let mut link_color = |_: &TreeLinkFunctionContext<'_>| {
        Ok(if next {
            Color(40, 90, 170, 220)
        } else {
            Color(150, 70, 20, 255)
        })
    };
    let layout = tree_layout_with_factories(
        &config,
        rows,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut width),
            link_color: Some(&mut link_color),
            ..Default::default()
        },
    )
    .unwrap();
    (config, layout)
}

fn capture(rows: &[HierarchyDatum], next: bool) -> TreeEventFrame {
    let (config, layout) = layout(
        rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        next,
    );
    TreeEventFrame::new(config, layout).unwrap()
}

// Independent gentle underdamped step; no production solver or generator used.
fn sampled(start: f64, target: f64, seconds: f64) -> f64 {
    let beta = 71_f64.sqrt();
    let y = start - target;
    target
        + (-7. * seconds).exp() * y * ((beta * seconds).cos() + 7. / beta * (beta * seconds).sin())
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

fn point(actual: Point, expected: [f64; 2]) {
    close(actual.x, expected[0]);
    close(actual.y, expected[1]);
}

fn color(start: Color, target: Color, time: f64) -> Color {
    let channel = |a: u8, b: u8| sampled(a.into(), b.into(), time).clamp(0., 255.).round() as u8;
    Color(
        channel(start.0, target.0),
        channel(start.1, target.1),
        channel(start.2, target.2),
        channel(start.3, target.3),
    )
}

fn geometry_node(geometry: &Geometry, shape: TreeNodeShape, center: [f64; 2], radius: f64) {
    match (shape, geometry) {
        (
            TreeNodeShape::Circle,
            Geometry::Circle {
                x,
                y,
                radius: actual,
            },
        ) => {
            close(*x, center[0]);
            close(*y, center[1]);
            close(*actual, radius);
        }
        (
            TreeNodeShape::Square,
            Geometry::Rect {
                x,
                y,
                width,
                height,
            },
        ) => {
            close(*x + *width / 2., center[0]);
            close(*y + *height / 2., center[1]);
            close(*width, radius * 2.);
            close(*height, radius * 2.);
        }
        (TreeNodeShape::Diamond, Geometry::Path(commands)) => {
            assert_eq!(commands.len(), 5);
            let expected = [
                [center[0], center[1] - radius],
                [center[0] + radius, center[1]],
                [center[0], center[1] + radius],
                [center[0] - radius, center[1]],
            ];
            for (index, command) in commands[..4].iter().enumerate() {
                match command {
                    Command::Move(p) if index == 0 => point(*p, expected[index]),
                    Command::Line(p) if index > 0 => point(*p, expected[index]),
                    _ => panic!("not the authored diamond topology: {commands:?}"),
                }
            }
            assert!(matches!(commands[4], Command::Close));
        }
        _ => panic!("wrong shape: {shape:?}, {geometry:?}"),
    }
}

fn geometry_link(
    geometry: &Geometry,
    orientation: TreeOrientation,
    curve: TreeLinkCurve,
    source: [f64; 2],
    target: [f64; 2],
) {
    let Geometry::Path(commands) = geometry else {
        panic!("Link is not a Path")
    };
    let Command::Move(start) = commands[0] else {
        panic!("Link has no initial Move")
    };
    point(start, source);
    let horizontal = matches!(
        orientation,
        TreeOrientation::LeftToRight | TreeOrientation::RightToLeft
    );
    let axis = usize::from(!horizontal);
    match curve {
        TreeLinkCurve::Straight => {
            assert_eq!(commands.len(), 2);
            let Command::Line(end) = commands[1] else {
                panic!("Straight curve not a line")
            };
            point(end, target);
        }
        TreeLinkCurve::Bump => {
            assert_eq!(commands.len(), 2);
            let Command::Cubic { first, second, end } = commands[1] else {
                panic!("Bump not cubic")
            };
            let mut first_expected = source;
            let mut second_expected = target;
            first_expected[axis] = (source[axis] + target[axis]) / 2.;
            second_expected[axis] = first_expected[axis];
            point(first, first_expected);
            point(second, second_expected);
            point(end, target);
        }
        _ => {
            assert_eq!(commands.len(), 4);
            let elbow = match curve {
                TreeLinkCurve::StepBefore => source[axis],
                TreeLinkCurve::StepAfter => target[axis],
                _ => (source[axis] + target[axis]) / 2.,
            };
            let mut first = source;
            first[axis] = elbow;
            let mut second = target;
            second[axis] = elbow;
            for (command, expected) in commands[1..].iter().zip([first, second, target]) {
                let Command::Line(actual) = command else {
                    panic!("Step not linear")
                };
                point(*actual, expected);
            }
        }
    }
}

#[test]
fn sampled_shapes_and_cartesian_curves_use_centers_and_physical_properties() {
    let rows = source();
    for shape in [
        TreeNodeShape::Circle,
        TreeNodeShape::Square,
        TreeNodeShape::Diamond,
    ] {
        for orientation in [
            TreeOrientation::LeftToRight,
            TreeOrientation::RightToLeft,
            TreeOrientation::TopToBottom,
            TreeOrientation::BottomToTop,
        ] {
            for curve in [
                TreeLinkCurve::Straight,
                TreeLinkCurve::Bump,
                TreeLinkCurve::Step,
                TreeLinkCurve::StepBefore,
                TreeLinkCurve::StepAfter,
            ] {
                let (config, old) = layout(&rows, shape, orientation, curve, false);
                let old = TreeEventFrame::new(config, old).unwrap();
                let (config, target) = layout(&rows, shape, orientation, curve, true);
                let target = TreeEventFrame::new(config, target).unwrap();
                let mut motion =
                    TreeMotion::new(old.clone(), TreeMotionOptions::default()).unwrap();
                motion.retarget(target.clone()).unwrap();
                motion.tick(0.1).unwrap();
                let paint = tree_motion_paint(motion.frame()).unwrap();
                assert_eq!(paint.motion(), motion.frame());
                assert!(std::ptr::eq(paint.target().source(), target.source()));
                paint.scene().validate().unwrap();
                for row in motion.frame().nodes() {
                    let index = row.data().source_index();
                    let from = &old.nodes()[index];
                    let to = &target.nodes()[index];
                    let center = [
                        sampled(from.position[0], to.position[0], 0.1),
                        sampled(from.position[1], to.position[1], 0.1),
                    ];
                    let mark =
                        &paint.scene().marks[paint.mark_for_instance(row.instance()).unwrap()];
                    geometry_node(&mark.geometry, shape, center, sampled(7., 11., 0.1));
                    assert_eq!(
                        mark.fill,
                        Some(color(from.color.unwrap(), to.color.unwrap(), 0.1))
                    );
                    assert_eq!(mark.stroke, row.data().mark().unwrap().stroke);
                    assert_eq!(mark.clip, row.data().mark().unwrap().clip);
                }
                for row in motion.frame().links() {
                    let index = row.data().link_index();
                    let from = &old.links()[index];
                    let to = &target.links()[index];
                    let source = [
                        sampled(from.source_position[0], to.source_position[0], 0.1),
                        sampled(from.source_position[1], to.source_position[1], 0.1),
                    ];
                    let target_position = [
                        sampled(from.target_position[0], to.target_position[0], 0.1),
                        sampled(from.target_position[1], to.target_position[1], 0.1),
                    ];
                    let mark =
                        &paint.scene().marks[paint.mark_for_instance(row.instance()).unwrap()];
                    geometry_link(&mark.geometry, orientation, curve, source, target_position);
                    let stroke = mark.stroke.unwrap();
                    close(stroke.width, sampled(2., 6., 0.1));
                    assert_eq!(
                        stroke.color,
                        color(
                            old.computed_link_styles()[index].color,
                            row.data().style().color,
                            0.1
                        )
                    );
                    assert!(mark.fill.is_none());
                }
            }
        }
    }
}

#[test]
fn zero_target_channels_do_not_erase_still_visible_sampled_ink() {
    let rows = source();
    let old = capture(&rows, false);
    let (config, _) = layout(
        &rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        true,
    );
    let options = old.layout().options().clone();
    let mut node_size = |_: &TreeNodeFunctionContext<'_>| Ok(0.);
    let mut link_width = |_: &TreeLinkFunctionContext<'_>| Ok(0.);
    let layout = tree_layout_with_factories(
        &config,
        &rows,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut node_size),
            link_thickness: Some(&mut link_width),
            ..Default::default()
        },
    )
    .unwrap();
    let target = TreeEventFrame::new(config, layout).unwrap();
    let mut motion = TreeMotion::new(old, TreeMotionOptions::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    motion.tick(0.1).unwrap();
    let paint = tree_motion_paint(motion.frame()).unwrap();
    for row in motion.frame().nodes() {
        assert_eq!(row.data().node().radius, 0.);
        let mark = &paint.scene().marks[paint.mark_for_instance(row.instance()).unwrap()];
        let Geometry::Circle { radius, .. } = mark.geometry else {
            panic!("expected circle")
        };
        close(radius, sampled(7., 0., 0.1));
        assert!(radius > 0.);
    }
    for row in motion.frame().links() {
        assert_eq!(row.data().style().thickness, 0.);
        let mark = &paint.scene().marks[paint.mark_for_instance(row.instance()).unwrap()];
        close(mark.stroke.unwrap().width, sampled(2., 0., 0.1));
    }
}

#[test]
fn exit_ids_avoid_real_extra_collisions_and_keep_old_data_out_of_target_maps() {
    let original = capture(&source(), false);
    let mut motion = TreeMotion::new(original.clone(), TreeMotionOptions::default()).unwrap();
    let instance = motion
        .frame()
        .nodes()
        .iter()
        .find(|n| n.key() == "a")
        .unwrap()
        .instance();
    let collision = format!("kartis-tree-motion-exit-{instance}-0");
    let remaining = source()
        .into_iter()
        .filter(|r| r.id != "a")
        .collect::<Vec<_>>();
    let (config, mut layout) = layout(
        &remaining,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        false,
    );
    let extra = Mark::new(
        &collision,
        Geometry::Rect {
            x: 5.,
            y: 5.,
            width: 4.,
            height: 4.,
        },
        Color(1, 2, 3, 255),
    );
    let extra_index = layout.scene.marks.len();
    layout.scene.marks.push(extra.clone());
    if !layout.scene.draw_order.is_empty() {
        layout.scene.draw_order.push(DrawItem::Mark(extra_index));
    }
    motion
        .retarget(TreeEventFrame::new(config, layout).unwrap())
        .unwrap();
    motion.tick(0.1).unwrap();
    let paint = tree_motion_paint(motion.frame()).unwrap();
    let ghost_index = paint.mark_for_instance(instance).unwrap();
    let ghost = &paint.scene().marks[ghost_index];
    assert_ne!(ghost.id, collision);
    assert!(paint.ghost_mark_ids().contains(&ghost.id));
    assert!(!paint.live_mark_indices().contains(&ghost_index));
    assert_eq!(paint.target_item(DrawItem::Mark(ghost_index)), None);
    assert_eq!(ghost.label, "");
    assert_eq!(ghost.value, 0.);
    let row = paint
        .motion()
        .nodes()
        .iter()
        .find(|n| n.instance() == instance)
        .unwrap();
    assert!(std::ptr::eq(row.data().frame().source(), original.source()));
    assert_eq!(row.data().datum().value.to_bits(), (-0_f64).to_bits());
    let item = paint.source_item(DrawItem::Mark(extra_index)).unwrap();
    let DrawItem::Mark(index) = item else {
        panic!("extra not a Mark")
    };
    assert_eq!(paint.scene().marks[index], extra);
    assert_eq!(paint.instance_for_item(item), None);
    assert_eq!(paint.target_item(item), Some(DrawItem::Mark(extra_index)));
    assert_eq!(
        paint
            .scene()
            .marks
            .iter()
            .map(|m| &m.id)
            .collect::<BTreeSet<_>>()
            .len(),
        paint.scene().marks.len()
    );
}

#[test]
fn reparented_edge_and_its_retiring_predecessor_have_distinct_physical_identity() {
    let rows = source();
    let moving = capture(&rows, true);
    let mut motion = TreeMotion::new(capture(&rows, false), TreeMotionOptions::default()).unwrap();
    motion.retarget(moving.clone()).unwrap();
    motion.tick(0.1).unwrap();
    let old = motion
        .frame()
        .links()
        .iter()
        .find(|l| l.key().target_id == "a")
        .unwrap()
        .instance();
    let mut reparented = rows;
    reparented[0].parent = Some("b".into());
    reparented
        .iter_mut()
        .find(|row| row.id == "b")
        .unwrap()
        .value = 0.;
    motion.retarget(capture(&reparented, true)).unwrap();
    let paint = tree_motion_paint(motion.frame()).unwrap();
    let old_index = paint.mark_for_instance(old).unwrap();
    let current = motion
        .frame()
        .links()
        .iter()
        .find(|l| l.phase() == TreeMotionPhase::Live && l.key().target_id == "a")
        .unwrap();
    let current_index = paint.mark_for_instance(current.instance()).unwrap();
    assert_ne!(old_index, current_index);
    assert_eq!(paint.scene().marks[current_index].id, "link:a");
    assert_ne!(paint.scene().marks[old_index].id, "link:a");
    assert_eq!(paint.target_item(DrawItem::Mark(old_index)), None);
    assert!(paint.target_item(DrawItem::Mark(current_index)).is_some());
    assert_eq!(
        paint.role_for_item(DrawItem::Mark(old_index)),
        Some(TreeMotionPaintRole::Link)
    );
    let old_row = motion
        .frame()
        .links()
        .iter()
        .find(|l| l.instance() == old)
        .unwrap();
    assert_eq!(old_row.key().source_id, "branch");
    assert!(std::ptr::eq(
        old_row.data().frame().source(),
        moving.source()
    ));
    assert_eq!(current.key().source_id, "b");
}

fn decorate_labels(layout: &mut TreeLayout) {
    for index in 0..layout.scene.labels.len() {
        layout.scene.typography.labels.insert(
            index,
            FontOptions {
                family: Some(vec![FontFamily::Mono]),
                weight: Some(500. + index as f64),
            },
        );
        layout
            .scene
            .typography
            .label_whitespace
            .insert(index, LabelWhitespace::Preserve);
    }
    let extra = layout.scene.labels.len();
    let mut label = layout.scene.labels[0].clone();
    label.text = "  static\tannotation  ".into();
    label.x = 17.;
    label.y = 23.;
    layout.scene.labels.push(label);
    layout.scene.typography.labels.insert(
        extra,
        FontOptions {
            family: Some(vec![FontFamily::Serif]),
            weight: Some(720.),
        },
    );
    layout
        .scene
        .typography
        .label_whitespace
        .insert(extra, LabelWhitespace::Preserve);
    layout.scene.label_clips.insert(
        extra,
        ClipPath {
            geometry: Geometry::Rect {
                x: 0.,
                y: 0.,
                width: 100.,
                height: 100.,
            },
            rule: ClipRule::EvenOdd,
        },
    );
    layout.scene.marks.push(Mark::new(
        "independent-extra",
        Geometry::Circle {
            x: 12.,
            y: 18.,
            radius: 3.,
        },
        Color(3, 4, 5, 255),
    ));
    let mut order = Vec::new();
    for index in 0..layout.scene.marks.len().max(layout.scene.labels.len()) {
        if index < layout.scene.labels.len() {
            order.push(DrawItem::Label(index));
        }
        if index < layout.scene.marks.len() {
            order.push(DrawItem::Mark(index));
        }
    }
    layout.scene.draw_order = order;
}

#[test]
fn labels_and_extras_keep_real_fonts_clips_whitespace_and_current_mixed_order() {
    let rows = source();
    let (config, mut old) = layout(
        &rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        false,
    );
    decorate_labels(&mut old);
    let mut motion = TreeMotion::new(
        TreeEventFrame::new(config, old).unwrap(),
        TreeMotionOptions::default(),
    )
    .unwrap();
    let (config, mut next) = layout(
        &rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        true,
    );
    decorate_labels(&mut next);
    let moving = TreeEventFrame::new(config, next).unwrap();
    motion.retarget(moving.clone()).unwrap();
    motion.tick(0.1).unwrap();
    let remaining = rows.into_iter().filter(|r| r.id != "a").collect::<Vec<_>>();
    let (config, mut target) = layout(
        &remaining,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        true,
    );
    decorate_labels(&mut target);
    let target = TreeEventFrame::new(config, target).unwrap();
    motion.retarget(target.clone()).unwrap();
    let paint = tree_motion_paint(motion.frame()).unwrap();
    let restored_order = paint
        .scene()
        .draw_items()
        .into_iter()
        .filter_map(|item| paint.target_item(item))
        .collect::<Vec<_>>();
    assert_eq!(restored_order, target.scene().draw_items());
    for source_item in target.scene().draw_items() {
        let item = paint.source_item(source_item).unwrap();
        assert_eq!(paint.target_item(item), Some(source_item));
    }
    let mut departing = 0;
    for row in motion.frame().labels() {
        let index = paint.label_for_instance(row.instance()).unwrap();
        let source = row.data().label();
        let actual = &paint.scene().labels[index];
        assert_eq!(actual.text, source.text);
        assert_eq!(actual.color, source.color);
        assert_eq!(actual.size, source.size);
        assert_eq!(actual.anchor, source.anchor);
        close(actual.x, row.props().x);
        close(actual.y, row.props().y);
        close(actual.rotation, row.props().rotation);
        assert_eq!(paint.scene().resolved_label_font(index), row.data().font());
        assert_eq!(
            paint.scene().typography.label_whitespace(index),
            row.data().whitespace()
        );
        assert_eq!(paint.scene().label_clips.get(&index), row.data().clip());
        assert_eq!(
            paint.instance_for_item(DrawItem::Label(index)),
            Some(row.instance())
        );
        if row.phase() == TreeMotionPhase::Leaving {
            departing += 1;
            assert!(paint.ghost_label_indices().contains(&index));
            assert_eq!(paint.target_item(DrawItem::Label(index)), None);
            assert!(std::ptr::eq(row.data().frame().source(), moving.source()));
        }
    }
    assert_eq!(departing, 1);
    assert_eq!(paint.extras().len(), 2);
    for &item in paint.extras() {
        assert_eq!(paint.role_for_item(item), None);
        let source = paint.target_item(item).unwrap();
        match (source, item) {
            (DrawItem::Mark(a), DrawItem::Mark(b)) => {
                assert_eq!(target.scene().marks[a], paint.scene().marks[b])
            }
            (DrawItem::Label(a), DrawItem::Label(b)) => {
                assert_eq!(target.scene().labels[a], paint.scene().labels[b]);
                assert_eq!(
                    target.scene().resolved_label_font(a),
                    paint.scene().resolved_label_font(b)
                );
                assert_eq!(
                    target.scene().label_clips.get(&a),
                    paint.scene().label_clips.get(&b)
                );
                assert_eq!(
                    paint.scene().typography.label_whitespace(b),
                    LabelWhitespace::Preserve
                );
            }
            _ => panic!("extra role changed"),
        }
    }
}

#[test]
fn authored_node_border_gradient_and_opacity_remain_target_only_properties() {
    let rows = source();
    let (config, mut layout) = layout(
        &rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        true,
    );
    let mark = layout
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == "node:a")
        .unwrap();
    mark.stroke = Some(GraphicStroke {
        width: 3.25,
        color: Color(90, 80, 70, 255),
        cap: StrokeCap::Butt,
    });
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![4., 2.],
        offset: 1.,
    });
    mark.opacity = 0.35;
    mark.fill_paint = Some(FillPaint::Linear {
        start: [20., 30.],
        end: [90., 70.],
        stops: vec![
            GradientStop::new(0., Color(1, 2, 3, 255)),
            GradientStop::new(1., Color(7, 8, 9, 180)),
        ],
    });
    let authored = mark.clone();
    let target = TreeEventFrame::new(config, layout).unwrap();
    let mut motion = TreeMotion::new(capture(&rows, false), TreeMotionOptions::default()).unwrap();
    motion.retarget(target).unwrap();
    motion.tick(0.1).unwrap();
    let paint = tree_motion_paint(motion.frame()).unwrap();
    let actual = paint
        .scene()
        .marks
        .iter()
        .find(|m| m.id == "node:a")
        .unwrap();
    assert_eq!(actual.stroke, authored.stroke);
    assert_eq!(actual.stroke_dash, authored.stroke_dash);
    assert_eq!(actual.fill_paint, authored.fill_paint);
    assert_eq!(actual.fill, authored.fill);
    assert_eq!(actual.opacity, authored.opacity);
    assert_eq!(actual.clip, authored.clip);
}

#[test]
fn edited_node_path_requires_an_explicit_renderer_and_keeps_its_authored_offset() {
    let rows = source();
    let (config, mut layout) = layout(
        &rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        true,
    );
    let data = &layout.nodes[0];
    let center = data.position;
    let mark = layout
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == "node:a")
        .unwrap();
    mark.geometry = Geometry::Path(vec![
        Command::Move(Point::new(center[0] + 5., center[1] - 4.)),
        Command::Line(Point::new(center[0] + 18., center[1] + 3.)),
        Command::Line(Point::new(center[0] + 2., center[1] + 9.)),
        Command::Close,
    ]);
    let target = TreeEventFrame::new(config, layout).unwrap();
    let mut motion = TreeMotion::new(capture(&rows, false), TreeMotionOptions::default()).unwrap();
    motion.retarget(target).unwrap();
    motion.tick(0.1).unwrap();
    let snapshot = motion.frame().clone();
    assert!(tree_motion_paint(motion.frame()).is_err());
    assert_eq!(motion.frame(), &snapshot);
    let calls = Cell::new(0);
    let mut render = |row: &TreeMotionNode, source: &Mark| -> Result<Mark> {
        calls.set(calls.get() + 1);
        let mut mark = source.clone();
        if let Geometry::Path(commands) = &mut mark.geometry {
            let dx = row.props().x - row.data().node().position[0];
            let dy = row.props().y - row.data().node().position[1];
            for command in commands {
                match command {
                    Command::Move(p) | Command::Line(p) => {
                        p.x += dx;
                        p.y += dy;
                    }
                    _ => {}
                }
            }
        }
        Ok(mark)
    };
    let paint = tree_motion_paint_with_renderers(
        motion.frame(),
        &mut TreeMotionRenderers {
            node: Some(&mut render),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), rows.len());
    let row = motion
        .frame()
        .nodes()
        .iter()
        .find(|n| n.key() == "a")
        .unwrap();
    let mark = &paint.scene().marks[paint.mark_for_instance(row.instance()).unwrap()];
    let Geometry::Path(commands) = &mark.geometry else {
        panic!("edited Path reset")
    };
    let Command::Move(first) = commands[0] else {
        panic!("missing authored Move")
    };
    point(first, [row.props().x + 5., row.props().y - 4.]);
    assert_eq!(mark.id, "node:a");
    assert_eq!(
        mark.value.to_bits(),
        row.data().mark().unwrap().value.to_bits()
    );
}

#[test]
fn edited_link_geometry_or_animated_stroke_requires_an_explicit_renderer() {
    for geometry_edit in [false, true] {
        let rows = source();
        let (config, mut layout) = layout(
            &rows,
            TreeNodeShape::Circle,
            TreeOrientation::LeftToRight,
            TreeLinkCurve::Bump,
            false,
        );
        let mark = layout
            .scene
            .marks
            .iter_mut()
            .find(|m| m.id == "link:a")
            .unwrap();
        if geometry_edit {
            mark.geometry = Geometry::Path(vec![
                Command::Move(Point::new(100., 100.)),
                Command::Line(Point::new(140., 120.)),
            ]);
        } else {
            mark.stroke.as_mut().unwrap().width = 7.;
        }
        let target = TreeEventFrame::new(config, layout).unwrap();
        let motion = TreeMotion::new(target.clone(), TreeMotionOptions::default()).unwrap();
        assert!(tree_motion_paint(motion.frame()).is_err());
        let mut render = |_row: &TreeMotionLink,
                          source: &Mark,
                          _generator: &TreeSvgLinkGenerator|
         -> Result<Mark> { Ok(source.clone()) };
        let paint = tree_motion_paint_with_renderers(
            motion.frame(),
            &mut TreeMotionRenderers {
                link: Some(&mut render),
                ..Default::default()
            },
        )
        .unwrap();
        let actual = paint
            .scene()
            .marks
            .iter()
            .find(|m| m.id == "link:a")
            .unwrap();
        assert_eq!(
            actual,
            target
                .scene()
                .marks
                .iter()
                .find(|m| m.id == "link:a")
                .unwrap()
        );
    }
}

#[test]
fn renderer_business_identity_and_invalid_geometry_fail_without_hiding_side_effects() {
    let motion = TreeMotion::new(capture(&source(), false), TreeMotionOptions::default()).unwrap();
    let frame = motion.frame().clone();
    for field in 0..4 {
        let mut render = |_row: &TreeMotionLink,
                          source: &Mark,
                          _generator: &TreeSvgLinkGenerator|
         -> Result<Mark> {
            let mut mark = source.clone();
            match field {
                0 => mark.id.push_str("-changed"),
                1 => mark.series = "fake".into(),
                2 => mark.label = "fake".into(),
                _ => {
                    assert_eq!(mark.value.to_bits(), 0_f64.to_bits());
                    mark.value = -0.;
                }
            }
            Ok(mark)
        };
        assert!(
            tree_motion_paint_with_renderers(
                &frame,
                &mut TreeMotionRenderers {
                    link: Some(&mut render),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let calls = Cell::new(0);
    let mut invalid = |_row: &TreeMotionNode, source: &Mark| -> Result<Mark> {
        calls.set(calls.get() + 1);
        let mut mark = source.clone();
        mark.geometry = Geometry::Circle {
            x: 10.,
            y: 10.,
            radius: -1.,
        };
        Ok(mark)
    };
    assert!(
        tree_motion_paint_with_renderers(
            &frame,
            &mut TreeMotionRenderers {
                node: Some(&mut invalid),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        calls.get() > 0,
        "application callback effects are not rolled back"
    );
    assert_eq!(motion.frame(), &frame);
    let mut label = |_row: &TreeMotionLabel, source: &Label| -> Result<Label> {
        let mut result = source.clone();
        result.rotation = 721.;
        Ok(result)
    };
    assert!(
        tree_motion_paint_with_renderers(
            &frame,
            &mut TreeMotionRenderers {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn radial_and_opacity_groups_in_current_or_retiring_cohorts_reject_before_callbacks() {
    for radial in [true, false] {
        for historical in [true, false] {
            let rows = source();
            let (config, mut layout) = layout(
                &rows,
                TreeNodeShape::Circle,
                if radial {
                    TreeOrientation::Radial
                } else {
                    TreeOrientation::LeftToRight
                },
                TreeLinkCurve::Bump,
                false,
            );
            if !radial {
                layout.scene.opacity_groups.push(MarkOpacityGroup {
                    id: "native-paint-group".into(),
                    members: layout.scene.marks[..2]
                        .iter()
                        .map(|m| m.id.clone())
                        .collect(),
                    opacity: 0.5,
                });
            }
            let mut motion = TreeMotion::new(
                TreeEventFrame::new(config, layout).unwrap(),
                TreeMotionOptions::default(),
            )
            .unwrap();
            if historical {
                let remaining = rows.into_iter().filter(|r| r.id != "a").collect::<Vec<_>>();
                motion.retarget(capture(&remaining, false)).unwrap();
                assert_eq!(
                    motion.frame().target().layout().options().orientation,
                    TreeOrientation::LeftToRight
                );
                assert!(motion.frame().target().scene().opacity_groups.is_empty());
                assert!(
                    motion
                        .frame()
                        .nodes()
                        .iter()
                        .any(|r| r.phase() == TreeMotionPhase::Leaving)
                );
            }
            let calls = Cell::new(0);
            let mut render = |_row: &TreeMotionLink,
                              source: &Mark,
                              _generator: &TreeSvgLinkGenerator|
             -> Result<Mark> {
                calls.set(calls.get() + 1);
                Ok(source.clone())
            };
            assert!(
                tree_motion_paint_with_renderers(
                    motion.frame(),
                    &mut TreeMotionRenderers {
                        link: Some(&mut render),
                        ..Default::default()
                    }
                )
                .is_err()
            );
            assert_eq!(calls.get(), 0);
        }
    }
}

#[test]
fn another_ghost_retiring_does_not_change_a_retained_instances_paint_id() {
    let rows = source();
    let (config, template) = layout(
        &rows,
        TreeNodeShape::Circle,
        TreeOrientation::LeftToRight,
        TreeLinkCurve::Bump,
        false,
    );
    let mut size =
        |context: &TreeNodeFunctionContext<'_>| Ok(if context.datum.id == "a" { 1. } else { 14. });
    let original = tree_layout_with_factories(
        &config,
        &rows,
        template.options(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            ..Default::default()
        },
    )
    .unwrap();
    let mut motion = TreeMotion::new(
        TreeEventFrame::new(config, original).unwrap(),
        TreeMotionOptions::default(),
    )
    .unwrap();
    let a = motion
        .frame()
        .nodes()
        .iter()
        .find(|r| r.key() == "a")
        .unwrap()
        .instance();
    let b = motion
        .frame()
        .nodes()
        .iter()
        .find(|r| r.key() == "b")
        .unwrap()
        .instance();
    let remaining = rows
        .into_iter()
        .filter(|r| r.id != "a" && r.id != "b")
        .collect::<Vec<_>>();
    motion.retarget(capture(&remaining, false)).unwrap();
    let initial = tree_motion_paint(motion.frame()).unwrap();
    let b_id = initial.scene().marks[initial.mark_for_instance(b).unwrap()]
        .id
        .clone();
    assert!(initial.mark_for_instance(a).is_some());
    let mut observed_retirement = false;
    for _ in 0..300 {
        motion.tick(0.01).unwrap();
        let has_a = motion.frame().nodes().iter().any(|r| r.instance() == a);
        let has_b = motion.frame().nodes().iter().any(|r| r.instance() == b);
        if !has_a && has_b {
            let next = tree_motion_paint(motion.frame()).unwrap();
            let item = next.item_for_instance(b).unwrap();
            let DrawItem::Mark(index) = item else {
                panic!("retained Node changed kind")
            };
            assert_eq!(next.scene().marks[index].id, b_id);
            assert_eq!(next.target_item(item), None);
            assert!(next.ghost_mark_ids().contains(&b_id));
            observed_retirement = true;
            break;
        }
    }
    assert!(
        observed_retirement,
        "different physical exit sizes must expose independent retirement"
    );
}
