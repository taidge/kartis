use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{GraphicPoint, PathCommand};
use kartis::*;
fn frame() -> FunnelSvgFrame {
    let config = ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            top: 17.,
            right: 23.,
            bottom: 31.,
            left: 41.,
        },
        title: "Bound notes / 注解".into(),
        ..Default::default()
    };
    let mut zero = FunnelDatum::new("zero", -0.);
    zero.size = Some(0.);
    let mut a = FunnelDatum::new("1", 8.);
    a.size = Some(180.);
    let mut b = FunnelDatum::new(1., 4.);
    b.size = Some(90.);
    FunnelSvgFrame::new(
        &config,
        &[a, b, zero],
        &FunnelLayoutOptions {
            size: FunnelSize::DatumPixels,
            labels: None,
            legend: false,
            ..Default::default()
        },
    )
    .unwrap()
}
fn spec(kind: FunnelAnnotationKind) -> FunnelAnnotationSpec {
    FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::All,
        kind,
        FunnelAnnotationNote::Text("<note> & 漏斗".into()),
        FunnelAnnotationPosition::Relative(40.),
        FunnelAnnotationPosition::Relative(-30.),
    )
}
fn build(frame: &FunnelSvgFrame, specs: &[FunnelAnnotationSpec]) -> FunnelAnnotations {
    funnel_annotations(frame, specs, &mut Default::default(), &Default::default()).unwrap()
}
#[test]
fn all_matching_preserves_typed_ids_source_order_and_absent_ink() {
    let frame = frame();
    assert!(frame.body_mark(2).unwrap().is_none());
    let mut selected = spec(FunnelAnnotationKind::Dot);
    selected.matcher = FunnelAnnotationMatch::Id(1.0.into());
    let notes = build(&frame, &[spec(FunnelAnnotationKind::Circle), selected]);
    assert_eq!(
        notes
            .bindings()
            .iter()
            .map(|b| (b.spec_index, b.part_index, b.source_index))
            .collect::<Vec<_>>(),
        vec![(0, 0, 0), (0, 1, 1), (0, 2, 2), (1, 1, 1)]
    );
    assert_eq!(notes.bindings()[0].id, CategoryKey::String("1".into()));
    assert_eq!(notes.bindings()[3].id, CategoryKey::Number(1.));
    assert_eq!(
        notes.source().source()[2].value.to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(notes.source(), &frame);
    assert!(notes.scene().label_series.is_empty());
    assert!(notes.scene().legend.is_empty());
}
#[test]
fn binder_getters_override_spec_xy_rect_dimensions_and_apply_exact_size_truthiness() {
    let frame = frame();
    let part = &frame.parts()[0];
    let mut rect = spec(FunnelAnnotationKind::Rect);
    rect.matcher = FunnelAnnotationMatch::Id("1".into());
    rect.x = Some(999.);
    rect.y = Some(-999.);
    rect.width = Some(1.);
    rect.height = Some(2.);
    rect.offset = 7.;
    rect.size = Some(0.);
    rect.note_x = FunnelAnnotationPosition::Absolute(12.);
    rect.note_y = FunnelAnnotationPosition::Absolute(35.);
    let mut circle = rect.clone();
    circle.kind = FunnelAnnotationKind::Circle;
    circle.size = Some(9.);
    let mut dot = rect.clone();
    dot.kind = FunnelAnnotationKind::Dot;
    let notes = build(&frame, &[rect, circle, dot]);
    let r = &notes.bindings()[0];
    assert_eq!(r.center, part.center);
    assert_eq!(r.width, part.bounds.width + 14.);
    assert_eq!(r.height, part.bounds.height + 14.);
    assert_eq!(r.size, part.bounds.width.max(part.bounds.height) + 14.);
    assert_eq!(
        r.computed.points[1],
        [frame.plot().x + 12., frame.plot().y + 35.]
    );
    assert_eq!(notes.bindings()[1].size, 9.);
    assert_eq!(
        notes.bindings()[2].size,
        part.bounds.width.max(part.bounds.height)
    );
    assert_eq!(notes.bindings()[2].width, part.bounds.width);
    assert_eq!(r.border_radius, 6.);
    assert_eq!(r.note_width, 120.);
    assert_eq!(r.note_text_offset, 8.);
}
#[test]
fn rectangle_uses_eight_discrete_anchors_and_notes_do_not_rotate() {
    let frame = frame();
    let mut specs = Vec::new();
    for (x, y) in [
        (0., -50.),
        (50., -50.),
        (50., 0.),
        (50., 50.),
        (0., 50.),
        (-50., 50.),
        (-50., 0.),
        (-50., -50.),
    ] {
        let mut s = spec(FunnelAnnotationKind::Rect);
        s.matcher = FunnelAnnotationMatch::Id("1".into());
        s.note_x = FunnelAnnotationPosition::Relative(x);
        s.note_y = FunnelAnnotationPosition::Relative(y);
        specs.push(s);
    }
    let notes = build(&frame, &specs);
    let part = &frame.parts()[0];
    for (binding, (dx, dy)) in notes.bindings().iter().zip([
        (0., -1.),
        (1., -1.),
        (1., 0.),
        (1., 1.),
        (0., 1.),
        (-1., 1.),
        (-1., 0.),
        (-1., -1.),
    ]) {
        assert!(
            (binding.computed.points[0][0] - (part.center[0] + dx * part.bounds.width / 2.)).abs()
                < 1e-10
        );
        assert!(
            (binding.computed.points[0][1] - (part.center[1] + dy * part.bounds.height / 2.)).abs()
                < 1e-10
        );
        assert_eq!(binding.computed.points[1][1], binding.computed.points[2][1]);
        assert_eq!(binding.computed.text[1], binding.computed.points[1][1] - 8.);
    }
    assert!(
        notes
            .scene()
            .labels
            .iter()
            .all(|label| label.rotation == 0.)
    );
}
#[test]
fn predicate_reads_complete_collection_before_real_custom_notes_run_once() {
    let frame = frame();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let mut predicate = move |c: FunnelAnnotationMatchContext<'_>| {
        assert_eq!(c.collection, frame_parts(c.frame));
        assert_eq!(c.part, &c.collection[c.part_index]);
        assert_eq!(c.part.source_index, c.source_index);
        seen.borrow_mut()
            .push(format!("match:{}:{}", c.key, c.source_index));
        Ok(c.source_index != 2)
    };
    let seen = events.clone();
    let mut note = move |c: FunnelAnnotationNoteContext<'_>| {
        assert_eq!(c.datum, &c.part.datum);
        assert_eq!(c.x, c.binding.computed.text[0]);
        assert_eq!(c.y, c.binding.computed.text[1]);
        seen.borrow_mut()
            .push(format!("note:{}:{}", c.key, c.binding.source_index));
        let mut s = c.scene()?;
        s.marks.push(Mark::new(
            "same-local-id",
            Geometry::Rect {
                x: c.x,
                y: c.y,
                width: 12.,
                height: 8.,
            },
            Color(90, 20, 170, 255),
        ));
        s.labels.push(Label {
            rotation: 0.,
            x: c.x,
            y: c.y,
            text: c.datum.label.clone(),
            color: Color(0, 0, 0, 255),
            size: 13.,
            anchor: TextAnchor::Start,
        });
        s.typography.label_strokes.insert(
            0,
            LabelStroke {
                color: Color(255, 255, 255, 255),
                width: 3.,
            },
        );
        s.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
        Ok(s)
    };
    let mut first = spec(FunnelAnnotationKind::Dot);
    first.matcher = FunnelAnnotationMatch::Predicate("a".into());
    first.note = FunnelAnnotationNote::Custom("draw".into());
    let mut second = first.clone();
    second.matcher = FunnelAnnotationMatch::Predicate("b".into());
    let result = funnel_annotations(
        &frame,
        &[first, second],
        &mut FunnelAnnotationFactories {
            matcher: Some(&mut predicate),
            note: Some(&mut note),
        },
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        &*events.borrow(),
        &[
            "match:a:0",
            "match:a:1",
            "match:a:2",
            "match:b:0",
            "match:b:1",
            "match:b:2",
            "note:draw:0",
            "note:draw:1",
            "note:draw:0",
            "note:draw:1"
        ]
    );
    let s = result.scene();
    s.validate().unwrap();
    assert_eq!(s.typography.label_strokes.len(), 4);
    for note_index in 0..4 {
        assert_eq!(
            &s.draw_order[note_index * 6 + 4..note_index * 6 + 6],
            &[
                DrawItem::Label(note_index),
                DrawItem::Mark(note_index * 5 + 4)
            ]
        );
    }
    let unique = s
        .marks
        .iter()
        .map(|m| &m.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), s.marks.len());
}
fn frame_parts(frame: &FunnelSvgFrame) -> &[FunnelPart] {
    frame.parts()
}
#[test]
fn svg_paint_order_real_halos_rounded_rect_and_escaped_text_are_retained() {
    let frame = frame();
    let mut rect = spec(FunnelAnnotationKind::Rect);
    rect.matcher = FunnelAnnotationMatch::Id("1".into());
    let result = build(&frame, &[rect]);
    let s = result.scene();
    assert_eq!(
        s.draw_order,
        vec![
            DrawItem::Mark(0),
            DrawItem::Mark(1),
            DrawItem::Mark(2),
            DrawItem::Mark(3),
            DrawItem::Label(0),
            DrawItem::Label(1)
        ]
    );
    assert_eq!(s.marks[0].stroke.unwrap().width, 5.);
    assert_eq!(s.marks[1].stroke.unwrap().width, 6.);
    assert_eq!(s.marks[2].stroke.unwrap().width, 2.);
    assert_eq!(s.marks[3].stroke.unwrap().width, 1.);
    assert!(
        matches!(&s.marks[2].geometry,Geometry::Path(p)if p.iter().any(|c|matches!(c,PathCommand::Cubic{..})))
    );
    assert_eq!(s.typography.label_strokes[&0].width, 4.);
    assert_eq!(
        s.labels[0].color,
        FunnelAnnotationTheme::default().text.color
    );
    let svg = render::to_svg(s).unwrap();
    assert!(svg.contains("stroke-linejoin=\"round\""));
    assert!(svg.contains("&lt;note&gt; &amp; 漏斗"));
    assert!(!svg.contains("<note>"));
}
#[test]
fn invalid_static_input_and_work_budget_run_no_predicate_or_note() {
    let frame = frame();
    let calls = Cell::new(0);
    let mut matcher = |_: FunnelAnnotationMatchContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(true)
    };
    let mut note = |c: FunnelAnnotationNoteContext<'_>| {
        calls.set(calls.get() + 1);
        c.scene()
    };
    let mut s = spec(FunnelAnnotationKind::Dot);
    s.matcher = FunnelAnnotationMatch::Predicate("m".into());
    s.note = FunnelAnnotationNote::Custom("n".into());
    let mut factories = FunnelAnnotationFactories {
        matcher: Some(&mut matcher),
        note: Some(&mut note),
    };
    let mut invalid = s.clone();
    invalid.note_x = FunnelAnnotationPosition::Absolute(f64::NAN);
    assert!(funnel_annotations(&frame, &[invalid], &mut factories, &Default::default()).is_err());
    assert!(
        funnel_annotations_with_budget(
            &frame,
            &[s],
            &mut factories,
            &Default::default(),
            FunnelAnnotationBudget {
                match_work: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}
#[test]
fn custom_note_errors_and_invalid_scene_cannot_change_the_source() {
    let frame = frame();
    let old = render::to_svg(frame.scene()).unwrap();
    let calls = Cell::new(0);
    let mut s = spec(FunnelAnnotationKind::Dot);
    s.note = FunnelAnnotationNote::Custom("n".into());
    let mut note = |c: FunnelAnnotationNoteContext<'_>| {
        calls.set(calls.get() + 1);
        if c.binding.source_index == 1 {
            return Err(ChartError("authored note rejection".into()));
        }
        c.scene()
    };
    let error = funnel_annotations(
        &frame,
        &[s.clone()],
        &mut FunnelAnnotationFactories {
            matcher: None,
            note: Some(&mut note),
        },
        &Default::default(),
    )
    .unwrap_err();
    assert!(error.0.contains("authored note rejection"));
    assert_eq!(calls.get(), 2);
    assert_eq!(render::to_svg(frame.scene()).unwrap(), old);
    let mut invalid = |c: FunnelAnnotationNoteContext<'_>| {
        let mut scene = c.scene()?;
        let mut mark = Mark::new(
            "wrong-business",
            Geometry::Circle {
                x: c.x,
                y: c.y,
                radius: 2.,
            },
            Color(0, 0, 0, 255),
        );
        mark.series = "foreign".into();
        scene.marks.push(mark);
        Ok(scene)
    };
    assert!(
        funnel_annotations(
            &frame,
            &[s],
            &mut FunnelAnnotationFactories {
                matcher: None,
                note: Some(&mut invalid)
            },
            &Default::default()
        )
        .is_err()
    );
    assert_eq!(render::to_svg(frame.scene()).unwrap(), old);
}
#[test]
fn builtin_geometry_budget_is_checked_before_custom_note_rendering() {
    let frame = frame();
    let calls = Cell::new(0);
    let mut note = |c: FunnelAnnotationNoteContext<'_>| {
        calls.set(calls.get() + 1);
        c.scene()
    };
    let mut s = spec(FunnelAnnotationKind::Rect);
    s.note = FunnelAnnotationNote::Custom("draw".into());
    assert!(
        funnel_annotations_with_budget(
            &frame,
            &[s],
            &mut FunnelAnnotationFactories {
                matcher: None,
                note: Some(&mut note)
            },
            &Default::default(),
            FunnelAnnotationBudget {
                marks: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}
#[test]
fn custom_note_scene_can_paint_actual_offset_geometry_not_only_a_formatter() {
    let frame = frame();
    let mut s = spec(FunnelAnnotationKind::Circle);
    s.matcher = FunnelAnnotationMatch::Id("1".into());
    s.note = FunnelAnnotationNote::Custom("shape".into());
    let mut draw = |c: FunnelAnnotationNoteContext<'_>| {
        let mut scene = c.scene()?;
        scene.marks.push(Mark::new(
            "geometry",
            Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(c.x, c.y)),
                PathCommand::Line(GraphicPoint::new(c.x + 20., c.y + 7.)),
            ]),
            Color(40, 90, 170, 255),
        ));
        Ok(scene)
    };
    let result = funnel_annotations(
        &frame,
        &[s],
        &mut FunnelAnnotationFactories {
            matcher: None,
            note: Some(&mut draw),
        },
        &Default::default(),
    )
    .unwrap();
    assert!(result.scene().labels.is_empty());
    assert_eq!(result.scene().marks.len(), 5);
    assert!(matches!(&result.scene().marks[4].geometry,Geometry::Path(p)if p.len()==2));
}

#[test]
fn svg_translucent_note_halo_and_main_each_retain_their_fill() {
    let frame = frame();
    let mut s = spec(FunnelAnnotationKind::Dot);
    s.matcher = FunnelAnnotationMatch::Id("1".into());
    let theme = FunnelAnnotationTheme {
        text: FunnelAnnotationTextStyle {
            color: Color(20, 70, 130, 100),
            ..Default::default()
        },
        ..Default::default()
    };
    let result = funnel_annotations(&frame, &[s], &mut Default::default(), &theme).unwrap();
    assert_eq!(result.scene().labels.len(), 2);
    assert_eq!(result.scene().labels[0].color, theme.text.color);
    assert_eq!(result.scene().labels[1].color, theme.text.color);
    assert!(result.scene().typography.label_strokes.contains_key(&0));
    assert!(!result.scene().typography.label_strokes.contains_key(&1));
    assert_eq!(
        result.scene().draw_order[result.scene().draw_order.len() - 2..],
        [DrawItem::Label(0), DrawItem::Label(1)]
    );
}

#[test]
fn replacing_annotations_uses_the_original_capture_without_managed_paint_chains() {
    let base = frame();
    let first = base
        .with_annotations(
            &[spec(FunnelAnnotationKind::Circle)],
            &mut Default::default(),
            &Default::default(),
        )
        .unwrap();
    let bundle = funnel_annotations(
        &first,
        &[spec(FunnelAnnotationKind::Dot)],
        &mut Default::default(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(bundle.source(), &base);
    let foreign = frame();
    assert!(foreign.with_annotation_bundle(bundle.clone()).is_err());
    let next = first.with_annotation_bundle(bundle).unwrap();
    assert_eq!(next.annotation_source(), &base);
    assert_eq!(next.scene().marks.len(), first.scene().marks.len());
    assert_eq!(next.scene().labels.len(), first.scene().labels.len());
    assert_eq!(
        next.annotations().unwrap().bindings().len(),
        base.parts().len()
    );
    let cleared = next
        .with_annotations(&[], &mut Default::default(), &Default::default())
        .unwrap();
    assert_eq!(cleared.scene().marks, base.scene().marks);
    assert_eq!(cleared.scene().labels, base.scene().labels);
    assert_eq!(
        render::to_svg(cleared.scene()).unwrap(),
        render::to_svg(base.scene()).unwrap()
    );
}
