//! Nivo's rect(width=0|height=0) and circle(r=0) have no shape ink.
//! Generic stroked-path point/line semantics must not change these primitives.
use kartis::*;
fn source(direction: FunnelDirection, size: f64) -> FunnelSvgFrame {
    let config = ChartConfig {
        width: 100.,
        height: 100.,
        margin: Margin {
            top: 25.,
            right: 25.,
            bottom: 25.,
            left: 25.,
        },
        title: "Degenerate annotation shapes".into(),
        ..Default::default()
    };
    let mut datum = FunnelDatum::new("stage", 1.);
    datum.size = Some(size);
    FunnelSvgFrame::new(
        &config,
        &[datum],
        &FunnelLayoutOptions {
            direction,
            size: FunnelSize::DatumPixels,
            labels: None,
            before: None,
            after: None,
            legend: false,
            border_width: 0.,
            ..Default::default()
        },
    )
    .unwrap()
}
fn shape_only() -> FunnelAnnotationTheme {
    FunnelAnnotationTheme {
        link: FunnelAnnotationPaint {
            fill: None,
            stroke: None,
            stroke_width: 0.,
            outline_width: 0.,
            ..Default::default()
        },
        text: FunnelAnnotationTextStyle {
            color: Color(0, 0, 0, 0),
            outline_width: 0.,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn recipe(kind: FunnelAnnotationKind, offset: f64) -> FunnelAnnotationSpec {
    let mut spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::All,
        kind,
        FunnelAnnotationNote::Text(String::new()),
        FunnelAnnotationPosition::Relative(18.),
        FunnelAnnotationPosition::Relative(-17.),
    );
    spec.offset = offset;
    spec
}
fn build(
    frame: &FunnelSvgFrame,
    spec: FunnelAnnotationSpec,
    theme: &FunnelAnnotationTheme,
) -> FunnelAnnotations {
    funnel_annotations(frame, &[spec], &mut Default::default(), theme).unwrap()
}
fn check_shape_slots_and_svg(notes: &FunnelAnnotations) {
    assert_eq!(
        notes.bindings().len(),
        1,
        "a no-ink primitive still binds its complete source"
    );
    assert_eq!(
        notes.scene().marks.len(),
        3,
        "shape halo/main and the independent main connector retain their slots"
    );
    assert_eq!(
        notes.scene().draw_order[..3],
        [DrawItem::Mark(0), DrawItem::Mark(1), DrawItem::Mark(2)]
    );
    assert!(notes.scene().marks[0].stroke.is_some());
    assert!(notes.scene().marks[1].stroke.is_some());
    assert!(
        notes.scene().marks[2].stroke.is_none(),
        "connector paint is disabled for the isolation control"
    );
    let svg = render::to_svg(notes.scene()).unwrap();
    for shape in &notes.scene().marks[..2] {
        let needle = format!("data-id=\"{}\"", shape.id);
        let tag = svg
            .split(needle.as_str())
            .nth(1)
            .expect("real annotation shape slot is exported")
            .split('>')
            .next()
            .unwrap();
        let path = tag
            .split(" d=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(
            path.is_empty(),
            "degenerate SVG rect/circle must not be exported as stroked line/point ink: {path}"
        );
    }
}
#[test]
fn zero_rect_dimensions_in_both_funnel_directions_export_no_shape_ink() {
    for direction in [FunnelDirection::Vertical, FunnelDirection::Horizontal] {
        let frame = source(direction, 0.);
        let notes = build(
            &frame,
            recipe(FunnelAnnotationKind::Rect, 0.),
            &shape_only(),
        );
        let bound = &notes.bindings()[0];
        assert_eq!(bound.center, frame.parts()[0].center);
        assert!(bound.width == 0. || bound.height == 0.);
        assert_eq!(bound.width.max(bound.height), 50.);
        check_shape_slots_and_svg(&notes);
    }
}
#[test]
fn square_part_negative_offset_can_legally_collapse_circle_without_point_ink() {
    let frame = source(FunnelDirection::Vertical, 50.);
    assert_eq!(frame.parts()[0].bounds.width, 50.);
    assert_eq!(frame.parts()[0].bounds.height, 50.);
    let notes = build(
        &frame,
        recipe(FunnelAnnotationKind::Circle, -25.),
        &shape_only(),
    );
    let bound = &notes.bindings()[0];
    assert_eq!([bound.size, bound.width, bound.height], [0., 0., 0.]);
    check_shape_slots_and_svg(&notes);
}
#[test]
fn degenerate_shape_does_not_suppress_its_independent_three_point_connector() {
    let frame = source(FunnelDirection::Vertical, 50.);
    let theme = FunnelAnnotationTheme {
        text: FunnelAnnotationTextStyle {
            outline_width: 0.,
            ..Default::default()
        },
        ..Default::default()
    };
    let notes = build(&frame, recipe(FunnelAnnotationKind::Circle, -25.), &theme);
    assert_eq!(notes.bindings()[0].size, 0.);
    assert_eq!(notes.scene().marks.len(), 4);
    assert!(matches!(&notes.scene().marks[0].geometry,Geometry::Path(points)if points.len()==3));
    assert!(notes.scene().marks[0].stroke.is_some());
    assert!(matches!(&notes.scene().marks[3].geometry,Geometry::Path(points)if points.len()==3));
    assert!(notes.scene().marks[3].stroke.is_some());
}
#[cfg(feature = "raster")]
mod pixels {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};

    use super::*;
    fn alpha_pixels(notes: &FunnelAnnotations) -> usize {
        // Even the empty note is processed with a real repository font registry.
        let mut fonts = FontRegistry::new();
        fonts
            .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
            .unwrap();
        render_raster(notes.scene(), &fonts, &RasterOptions::default())
            .unwrap()
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[3] != 0)
            .count()
    }
    #[test]
    fn zero_rect_dimensions_do_not_generate_cpu_stroke_lines() {
        for direction in [FunnelDirection::Vertical, FunnelDirection::Horizontal] {
            let absent = build(
                &source(direction, 0.),
                recipe(FunnelAnnotationKind::Rect, 0.),
                &shape_only(),
            );
            let control = build(
                &source(direction, 50.),
                recipe(FunnelAnnotationKind::Rect, 0.),
                &shape_only(),
            );
            assert!(
                alpha_pixels(&control) > 100,
                "positive-area control must actually draw shape strokes"
            );
            assert_eq!(
                alpha_pixels(&absent),
                0,
                "native zero-width/height rect semantics must not turn into a stroked line"
            );
        }
    }
    #[test]
    fn collapsed_circle_does_not_generate_cpu_square_cap_point_footprints() {
        let frame = source(FunnelDirection::Vertical, 50.);
        let control = build(
            &frame,
            recipe(FunnelAnnotationKind::Circle, 0.),
            &shape_only(),
        );
        assert!(alpha_pixels(&control) > 100);
        let collapsed = build(
            &frame,
            recipe(FunnelAnnotationKind::Circle, -25.),
            &shape_only(),
        );
        assert_eq!(
            alpha_pixels(&collapsed),
            0,
            "native radius0 circle must not acquire a square point-stroke footprint"
        );
    }
}
