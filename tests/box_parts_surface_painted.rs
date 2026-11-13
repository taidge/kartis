use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::{surface::*, *};

fn controller() -> (
    ChartConfig,
    Vec<BoxObservation>,
    BoxPlotLayoutOptions,
    BoxPlotInteraction,
) {
    let config = ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    };
    let data = [0., 2., 4., 6., 8., 10.]
        .into_iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(format!("sample-{i}"), "A", "x", Some(v)))
        .collect::<Vec<_>>();
    let stroke = GraphicStroke {
        color: Color(20, 40, 60, 255),
        width: 6.,
        cap: StrokeCap::Butt,
    };
    let options = BoxPlotLayoutOptions {
        domain: Some([-2., 12.]),
        border: Some(stroke),
        median: stroke,
        whisker: stroke,
        cap_size: 0.5,
        outlier_radius: 5.,
        legend: false,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let controller =
        BoxPlotInteraction::new(&config, &data, &options, &Default::default()).unwrap();
    (config, data, options, controller)
}
fn surface(frame: SceneInteraction) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        frame.scene,
        SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction,
    )
    .unwrap()
}
fn move_to(model: &mut SurfaceModel, p: (f64, f64)) -> Vec<SurfaceEvent> {
    model.apply(SurfaceInput::Move { id: 1, point: p }).unwrap()
}
fn first_segment(mark: &Mark, t: f64) -> (f64, f64) {
    let Geometry::Path(commands) = &mark.geometry else {
        panic!("actual path required")
    };
    let (C::Move(a), C::Line(b)) = (&commands[0], &commands[1]) else {
        panic!("actual straight segment required")
    };
    (a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
#[test]
fn actual_before_witness_three_outside_body_parts_now_reach_one_site() {
    let (_, _, _, controller) = controller();
    let body = controller.layout.boxes[0].id.clone();
    let mut legacy = surface(controller.scene_interaction().unwrap());
    let mut painted = surface(controller.whole_parts_scene_interaction().unwrap());
    for (suffix, p) in [
        (":whiskers", (240., 208.5)),
        (":caps", (199.0909090909091, 222.)),
        (":outlier:8:sample-0", (240., 240.)),
    ] {
        let physical = format!("{body}{suffix}");
        assert_eq!(
            controller.layout.scene.hit_test(p.0, p.1).unwrap().id,
            physical
        );
        assert_eq!(
            controller.hit_test(p.0, p.1).unwrap(),
            Some(&BoxPlotKey::new("A", "x"))
        );
        assert!(move_to(&mut legacy, p).is_empty());
        assert!(legacy.hovered().is_empty());
        let events = move_to(&mut painted, p);
        assert_eq!(painted.hovered(), std::slice::from_ref(&body));
        let hit = painted.hovered_physical_hit().unwrap();
        assert_eq!(hit.physical_mark_id, physical);
        assert_eq!(hit.logical_site_id, body);
        assert_eq!(hit.scene_point, [p.0, p.1]);
        assert!(painted.is_current_hit(hit));
        assert!(matches!(
            events.last(),
            Some(SurfaceEvent::PhysicalHover(Some(_)))
        ));
    }
    painted.apply(SurfaceInput::First).unwrap();
    assert_eq!(painted.selected(), Some(body.as_str()));
    painted.apply(SurfaceInput::Next).unwrap();
    assert_eq!(painted.selected(), Some(body.as_str()));
}
#[test]
fn same_owner_part_changes_emit_physical_events_after_logical_events() {
    let (_, _, _, controller) = controller();
    let frame = controller.whole_parts_scene_interaction().unwrap();
    let median = frame
        .scene
        .marks
        .iter()
        .find(|m| m.id.ends_with(":median"))
        .unwrap();
    let cap = frame
        .scene
        .marks
        .iter()
        .find(|m| m.id.ends_with(":caps"))
        .unwrap();
    let a = first_segment(median, 0.25);
    let b = first_segment(cap, 0.25);
    let mut model = surface(frame);
    let first = move_to(&mut model, a);
    assert!(matches!(first.first(), Some(SurfaceEvent::Hover(_))));
    assert!(matches!(
        first.last(),
        Some(SurfaceEvent::PhysicalHover(Some(_)))
    ));
    let second = move_to(&mut model, b);
    assert_eq!(second.len(), 1);
    assert!(matches!(second[0], SurfaceEvent::PhysicalHover(Some(_))));
    let clear = model.apply(SurfaceInput::Leave).unwrap();
    assert!(matches!(clear.first(),Some(SurfaceEvent::Hover(ids)) if ids.is_empty()));
    assert!(matches!(
        clear.last(),
        Some(SurfaceEvent::PhysicalHover(None))
    ));
}
fn line_frame(cap: StrokeCap) -> SceneInteraction {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut body = Mark::new(
        "body",
        Geometry::Rect {
            x: 40.,
            y: 60.,
            width: 40.,
            height: 40.,
        },
        Color(0, 0, 0, 0),
    );
    body.series = "x".into();
    body.value = 5.;
    let mut part = Mark::new(
        "line",
        Geometry::Path(vec![C::Move(P::new(20., 20.)), C::Line(P::new(80., 20.))]),
        Color(0, 0, 0, 0),
    );
    part.fill = None;
    part.label.clear();
    part.series = "x".into();
    part.stroke = Some(GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 4.,
        cap,
    });
    scene.marks = vec![body, part];
    let binding = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "body".into(),
            series: "x".into(),
            label: "body".into(),
            value: Some(5.),
            formatted_value: None,
            color: Color(0, 0, 0, 0),
            position: [60., 80.],
        }],
        ink_owners: [("line".into(), "body".into())].into(),
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    SceneInteraction {
        scene,
        interaction: Some(binding),
        plot: None,
    }
}
#[test]
fn real_half_width_caps_transparent_fill_and_body_opacity_zero() {
    let frame = line_frame(StrokeCap::Butt);
    assert!(
        frame.scene.hit_test(50., 22.5).is_some(),
        "old +2 tolerance really admits this point"
    );
    let mut model = surface(frame.clone());
    assert!(model.physical_hit_at((50., 21.9)).unwrap().is_some());
    assert!(model.physical_hit_at((50., 22.5)).unwrap().is_none());
    assert!(model.physical_hit_at((19.5, 20.)).unwrap().is_none());
    assert!(
        model.physical_hit_at((60., 80.)).unwrap().is_none(),
        "transparent body does not paint"
    );
    let mut zero_body = frame.clone();
    zero_body.scene.marks[0].opacity = 0.;
    model.set_layout(zero_body).unwrap();
    let events = move_to(&mut model, (50., 20.));
    assert_eq!(model.hovered(), &["body".to_string()]);
    assert!(model.active_interaction(model.hovered()).is_some());
    assert!(matches!(
        events.last(),
        Some(SurfaceEvent::PhysicalHover(Some(_)))
    ));
    let square = surface(line_frame(StrokeCap::Square));
    assert!(square.physical_hit_at((18.5, 20.)).unwrap().is_some());
    let round = surface(line_frame(StrokeCap::Round));
    assert!(round.physical_hit_at((18.5, 20.)).unwrap().is_some());
    assert!(round.physical_hit_at((18.5, 21.9)).unwrap().is_none());
}
#[test]
fn dash_gaps_zero_dots_and_actual_clips_remain_unpainted() {
    let mut frame = line_frame(StrokeCap::Butt);
    frame.scene.marks[1].stroke_dash = Some(StrokeDash {
        segments: vec![10., 10.],
        offset: 0.,
    });
    let mut model = surface(frame.clone());
    assert!(model.physical_hit_at((25., 20.)).unwrap().is_some());
    assert!(model.physical_hit_at((35., 20.)).unwrap().is_none());
    frame.scene.marks[1].stroke = Some(GraphicStroke {
        cap: StrokeCap::Round,
        ..frame.scene.marks[1].stroke.unwrap()
    });
    frame.scene.marks[1].stroke_dash = Some(StrokeDash {
        segments: vec![0., 10.],
        offset: 0.,
    });
    model.set_layout(frame.clone()).unwrap();
    assert!(model.physical_hit_at((20., 20.)).unwrap().is_some());
    assert!(model.physical_hit_at((25., 20.)).unwrap().is_none());
    frame.scene.marks[1].clip = Some(ClipRect {
        x: 20.,
        y: 0.,
        width: 0.,
        height: 40.,
    });
    model.set_layout(frame.clone()).unwrap();
    assert!(model.physical_hit_at((20., 20.)).unwrap().is_none());
    frame.scene.marks[1].clip = None;
    frame.scene.marks[1].clip_path = Some(Box::new(ClipPath {
        geometry: Geometry::Circle {
            x: 20.,
            y: 20.,
            radius: 0.5,
        },
        rule: ClipRule::NonZero,
    }));
    model.set_layout(frame).unwrap();
    assert!(model.physical_hit_at((20., 20.)).unwrap().is_some());
    assert!(model.physical_hit_at((21., 20.)).unwrap().is_none());
}
#[test]
fn alpha_group_zero_hidden_and_live_frame_replacements_preserve_tokens_on_error() {
    let mut frame = line_frame(StrokeCap::Butt);
    let mut model = surface(frame.clone());
    let hit = model.physical_hit_at((50., 20.)).unwrap().unwrap();
    let token = model.frame_token();
    let mut bad = frame.clone();
    bad.scene.marks[1].series = "foreign".into();
    assert!(model.set_layout(bad).is_err());
    assert_eq!(model.frame_token(), token);
    assert!(model.is_current_hit(&hit));
    model.set_hidden(["x".into()].into()).unwrap();
    assert!(!model.is_current_hit(&hit));
    assert!(model.physical_hit_at((50., 20.)).unwrap().is_none());
    model.set_hidden(Default::default()).unwrap();
    assert!(model.physical_hit_at((50., 20.)).unwrap().is_some());
    frame.scene.opacity_groups = vec![MarkOpacityGroup {
        id: "whole".into(),
        members: vec!["body".into(), "line".into()],
        opacity: 0.,
    }];
    model.set_layout(frame.clone()).unwrap();
    assert!(model.physical_hit_at((50., 20.)).unwrap().is_none());
    frame.scene.opacity_groups.clear();
    frame.scene.marks[1].stroke.as_mut().unwrap().color.3 = 0;
    model.set_layout(frame).unwrap();
    assert!(model.physical_hit_at((50., 20.)).unwrap().is_none());
}
#[test]
fn explicit_strict_admission_rejects_other_picking_and_non_solid_atomically() {
    let frame = line_frame(StrokeCap::Butt);
    for picking in [
        Picking::Nearest { distance: 100. },
        Picking::SliceX { distance: 100. },
        Picking::SliceY { distance: 100. },
    ] {
        assert!(
            SurfaceModel::new_with_interaction(
                frame.scene.clone(),
                SurfaceOptions {
                    picking,
                    ..Default::default()
                },
                frame.interaction.clone()
            )
            .is_err()
        );
    }
    let mut model = surface(frame.clone());
    move_to(&mut model, (50., 20.));
    let before = model.scene().clone();
    let token = model.frame_token();
    let hit = model.hovered_physical_hit().cloned();
    let mut bad = frame;
    bad.scene.marks[0].fill_paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [100., 0.],
        stops: vec![
            GradientStop::new(0., Color(0, 0, 0, 0)),
            GradientStop::new(1., Color(0, 0, 0, 255)),
        ],
    });
    assert!(model.set_layout(bad).is_err());
    assert_eq!(model.scene(), &before);
    assert_eq!(model.frame_token(), token);
    assert_eq!(model.hovered_physical_hit(), hit.as_ref());
}
#[test]
fn original_outlier_snapshot_survives_motion_and_same_id_source_replacement() {
    let (config, mut data, options, mut controller) = controller();
    let frame = controller.whole_parts_scene_interaction().unwrap();
    let physical = frame
        .scene
        .marks
        .iter()
        .find(|m| m.id.ends_with(":outlier:8:sample-0"))
        .unwrap()
        .id
        .clone();
    let mut model = surface(frame.clone());
    let old = model.physical_hit_at((240., 240.)).unwrap().unwrap();
    assert_eq!(
        old.source_data,
        Some(InteractionPartDatum {
            source_id: "sample-0".into(),
            value: 0.
        })
    );
    let mut animated = frame.scene.clone();
    animated
        .marks
        .iter_mut()
        .find(|m| m.id == physical)
        .unwrap()
        .value = 999.;
    model
        .set_interaction_frame(&frame.scene, frame.interaction.clone(), animated)
        .unwrap();
    assert_eq!(
        model
            .physical_hit_at((240., 240.))
            .unwrap()
            .unwrap()
            .source_data,
        old.source_data
    );
    assert!(!model.is_current_hit(&old));
    data[0].value = Some(-10.);
    controller.replace_source(&config, &data, &options).unwrap();
    let next = controller.whole_parts_scene_interaction().unwrap();
    assert_eq!(
        next.interaction.as_ref().unwrap().part_data[&physical].value,
        -10.
    );
    model.set_layout(next).unwrap();
    assert_eq!(old.source_data.unwrap().value, 0.);
}
#[test]
fn reverse_draw_order_and_clone_divergence_have_actual_physical_identity() {
    let (_, _, _, controller) = controller();
    let frame = controller.whole_parts_scene_interaction().unwrap();
    let median = frame
        .scene
        .marks
        .iter()
        .find(|m| m.id.ends_with(":median"))
        .unwrap();
    let p = first_segment(median, 0.25);
    let mut a = surface(frame.clone());
    let original = a.physical_hit_at(p).unwrap().unwrap();
    assert!(original.physical_mark_id.ends_with(":median"));
    let mut b = a.clone();
    assert_eq!(a.frame_token(), b.frame_token());
    let mut reverse = frame;
    reverse.scene.draw_order = reverse.scene.draw_items();
    reverse.scene.draw_order.reverse();
    a.set_layout(reverse.clone()).unwrap();
    b.set_layout(reverse).unwrap();
    assert_ne!(a.frame_token(), b.frame_token());
    assert!(!a.is_current_hit(&original));
    assert_eq!(
        a.physical_hit_at(p).unwrap().unwrap().physical_mark_id,
        controller.layout.boxes[0].id
    );
}

#[test]
fn cubic_true_centerline_and_curved_fill_do_not_become_bounding_boxes() {
    let mut frame = line_frame(StrokeCap::Butt);
    let curve = Geometry::Path(vec![
        C::Move(P::new(20., 20.)),
        C::Cubic {
            first: P::new(20., 80.),
            second: P::new(80., 80.),
            end: P::new(80., 20.),
        },
    ]);
    frame.scene.marks[1].geometry = curve;
    let model = surface(frame.clone());
    // Exact Bernstein evaluation at t=1/2: (50,65), with horizontal tangent.
    assert!(model.physical_hit_at((50., 65.)).unwrap().is_some());
    assert!(model.physical_hit_at((50., 67.5)).unwrap().is_none());
    assert!(model.physical_hit_at((50., 40.)).unwrap().is_none());
    frame.scene.marks[1].stroke = None;
    frame.scene.marks[1].fill = Some(Color(0, 0, 0, 255));
    frame.scene.marks[1].geometry = Geometry::Path(vec![
        C::Move(P::new(20., 10.)),
        C::Line(P::new(80., 10.)),
        C::Cubic {
            first: P::new(85.52284749830794, 10.),
            second: P::new(90., 14.477152501692066),
            end: P::new(90., 20.),
        },
        C::Line(P::new(90., 60.)),
        C::Line(P::new(20., 60.)),
        C::Close,
    ]);
    let model = surface(frame);
    assert!(model.physical_hit_at((85., 20.)).unwrap().is_some());
    assert!(model.physical_hit_at((89.5, 10.5)).unwrap().is_none());
}

#[test]
fn zero_ink_keeps_one_keyboard_station_without_mutating_physical_scene() {
    let (_, _, _, controller) = controller();
    let mut frame = controller.whole_parts_scene_interaction().unwrap();
    let members = frame.scene.paint_marks().map(|m| m.id.clone()).collect();
    frame.scene.opacity_groups = vec![MarkOpacityGroup {
        id: "hidden-all".into(),
        members,
        opacity: 0.,
    }];
    let physical = frame.scene.clone();
    let mut model = surface(frame);
    assert!(model.physical_hit_at((240., 240.)).unwrap().is_none());
    model.apply(SurfaceInput::First).unwrap();
    let id = controller.layout.boxes[0].id.clone();
    assert_eq!(model.selected(), Some(id.as_str()));
    let controls = model.controls_scene();
    assert!(controls.opacity_groups.is_empty());
    assert_eq!(
        controls
            .marks
            .iter()
            .filter(|m| !m.label.is_empty())
            .map(|m| m.id.clone())
            .collect::<Vec<_>>(),
        vec![id]
    );
    assert_eq!(model.scene(), &physical);
}

#[test]
fn immutable_event_snapshots_keep_send_and_sync_without_address_debug() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<SurfaceFrameToken>();
    send_sync::<SurfacePhysicalHit>();
    send_sync::<SurfaceEvent>();
    let frame = line_frame(StrokeCap::Butt);
    let model = surface(frame);
    assert_eq!(format!("{:?}", model.frame_token()), "SurfaceFrameToken");
}
