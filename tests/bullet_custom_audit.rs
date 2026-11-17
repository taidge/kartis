use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}

fn options() -> BulletLayoutOptions {
    BulletLayoutOptions {
        min: Some(0.),
        max: Some(10.),
        axis: AxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn datum(id: &str) -> BulletDatum {
    BulletDatum {
        id: id.into(),
        ranges: vec![0., 5., 5.],
        measures: vec![0., 3., 3., 12.],
        markers: vec![-1., 0., 5., 5., 11.],
    }
}

fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + width, y)),
        C::Line(P::new(x + width, y + height)),
        C::Line(P::new(x, y + height)),
        C::Close,
    ])
}

#[test]
fn callbacks_match_real_marks_keep_duplicate_sources_and_include_synthetic_range() {
    let source = [datum("hidden"), datum("visible")];
    let mut o = options();
    o.hidden.insert("hidden".into());
    o.order = Some(vec!["visible".into(), "hidden".into()]);
    for orientation in [
        StatisticsOrientation::Horizontal,
        StatisticsOrientation::Vertical,
    ] {
        for reverse in [false, true] {
            o.orientation = orientation;
            o.reverse = reverse;
            let base = bullet_layout(&config(), &source, &o).unwrap();
            let mut seen = Vec::new();
            let out = bullet_layout_with_part_factory(&config(), &source, &o, &mut |ctx| {
                assert_eq!(ctx.source_row_index, 1);
                assert_eq!(ctx.row_index, 0);
                assert_eq!(ctx.datum, &source[1]);
                assert_eq!(ctx.row.id, "visible");
                assert_eq!(ctx.geometry, &ctx.part.geometry);
                seen.push((
                    ctx.part.id.clone(),
                    ctx.part.kind,
                    ctx.part.source_index,
                    ctx.rank,
                ));
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
            .unwrap();
            assert_eq!(out.layout, base);
            let ids: Vec<_> = base.rows[0]
                .parts
                .iter()
                .filter(|part| base.scene.marks.iter().any(|mark| mark.id == part.id))
                .map(|part| part.id.as_str())
                .collect();
            assert_eq!(
                seen.iter()
                    .map(|entry| entry.0.as_str())
                    .collect::<Vec<_>>(),
                ids
            );
            let details: Vec<_> = seen
                .iter()
                .map(|entry| (entry.1, entry.2, entry.3))
                .collect();
            assert_eq!(
                details,
                vec![
                    (BulletPartKind::Range, Some(1), 1),
                    (BulletPartKind::Range, None, 3),
                    (BulletPartKind::Measure, Some(1), 1),
                    (BulletPartKind::Measure, Some(3), 3),
                    (BulletPartKind::Marker, Some(1), 1),
                    (BulletPartKind::Marker, Some(2), 2),
                    (BulletPartKind::Marker, Some(3), 3),
                ]
            );
            assert_eq!(out.interaction.sites.len(), ids.len());
            out.interaction.validate(&out.layout.scene).unwrap();
        }
    }
}

#[test]
fn offset_paths_keep_logical_centers_and_typed_source_but_exact_follows_ink() {
    let source = [BulletDatum {
        id: "row".into(),
        ranges: vec![10.],
        measures: vec![],
        markers: vec![5.],
    }];
    let c = config();
    let mut original_centers = Vec::new();
    let out = bullet_layout_with_part_factory(&c, &source, &options(), &mut |ctx| {
        original_centers.push((ctx.part.id.clone(), ctx.position, ctx.part.raw_value));
        let mut value = ctx.inherit(rectangle(
            ctx.position[0] + 20.,
            ctx.position[1] - 5.,
            10.,
            10.,
        ));
        value.fill = Some(Color(230, 20, 60, 255));
        value.stroke = None;
        Ok(value)
    })
    .unwrap();
    for (id, point, value) in &original_centers {
        let site = out
            .interaction
            .sites
            .iter()
            .find(|site| &site.mark_id == id)
            .unwrap();
        assert_eq!(site.position, *point);
        assert_eq!(site.value, Some(*value));
        assert_eq!(site.series, "row");
        assert_eq!(site.color, Color(230, 20, 60, 255));
        let part = out.layout.rows[0]
            .parts
            .iter()
            .find(|part| &part.id == id)
            .unwrap();
        let mark = out
            .layout
            .scene
            .marks
            .iter()
            .find(|mark| &mark.id == id)
            .unwrap();
        assert_eq!(part.geometry, mark.geometry);
        assert_eq!(mark.clip, Some(out.layout.rows[0].track));
    }
    let p = original_centers[0].1;
    for (picking, point, hit) in [
        (Picking::Geometry, p, false),
        (Picking::Geometry, [p[0] + 25., p[1]], true),
        (Picking::Nearest { distance: 1. }, p, true),
        (Picking::Nearest { distance: 1. }, [p[0] + 25., p[1]], false),
    ] {
        let mut surface = SurfaceModel::new_with_interaction(
            out.layout.scene.clone(),
            SurfaceOptions {
                plot: Some(out.plot),
                picking,
                ..Default::default()
            },
            Some(out.interaction.clone()),
        )
        .unwrap();
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (point[0], point[1]),
            })
            .unwrap();
        assert_eq!(!surface.hovered().is_empty(), hit);
    }
}

#[test]
fn custom_ink_remains_clipped_to_track_across_rows_and_scale_projection() {
    let source = [datum("first"), datum("second")];
    let o = options();
    let scale = ContinuousScaleOptions {
        round: true,
        reverse: true,
        ..Default::default()
    };
    let out = bullet_layout_with_scale_options_and_part_factory(
        &config(),
        &source,
        &o,
        &scale,
        &mut |ctx| {
            Ok(ctx.inherit(rectangle(
                ctx.position[0] - 3.,
                ctx.row.track.y + ctx.row.track.height - 3.,
                6.,
                10.,
            )))
        },
    )
    .unwrap();
    let first = &out.layout.layout.rows[0];
    let id = &out.layout.interaction.sites[0].mark_id;
    let mark = out
        .layout
        .layout
        .scene
        .marks
        .iter()
        .find(|mark| &mark.id == id)
        .unwrap();
    let Geometry::Path(commands) = &mark.geometry else {
        panic!("custom path")
    };
    let C::Move(p) = commands[0] else {
        panic!("path start")
    };
    assert_eq!(mark.clip, Some(first.track));
    assert!(
        out.layout
            .layout
            .scene
            .hit_test(p.x + 1., p.y + 1.)
            .is_some()
    );
    assert!(
        out.layout
            .layout
            .scene
            .hit_test(p.x + 1., p.y + 5.)
            .is_none()
    );
    assert_eq!(out.row_value_scales.len(), 2);
    assert!(out.shared_value_scale.is_some());
}

#[test]
fn final_command_budget_excludes_replaced_marker_paths_and_rejects_the_next_command() {
    let source = [BulletDatum {
        id: "row".into(),
        ranges: vec![],
        measures: vec![],
        markers: vec![2., 8.],
    }];
    for (last, valid) in [(2, true), (3, false)] {
        let result = bullet_layout_with_part_factory(&config(), &source, &options(), &mut |ctx| {
            let count = match (ctx.part.kind, ctx.part.source_index) {
                (BulletPartKind::Range, None) => 199_996,
                (BulletPartKind::Marker, Some(1)) => last,
                _ => 2,
            };
            let p = P::new(ctx.position[0], ctx.position[1]);
            let mut path = vec![C::Line(p); count];
            path[0] = C::Move(p);
            let mut value = ctx.inherit(Geometry::Path(path));
            value.anchor_commands = Some(vec![0]);
            Ok(value)
        });
        assert_eq!(result.is_ok(), valid, "{result:?}");
    }
}

#[test]
fn failed_late_callback_and_invalid_frames_leave_accepted_layout_and_source_unchanged() {
    let source = [datum("row")];
    let saved_source = source.clone();
    let accepted = bullet_layout(&config(), &source, &options()).unwrap();
    for bad in [f64::NAN, -1., 1.01] {
        assert!(
            bullet_layout_with_part_factory(&config(), &source, &options(), &mut |ctx| {
                let mut value = ctx.inherit(ctx.geometry.clone());
                value.opacity = bad;
                Ok(value)
            })
            .is_err()
        );
    }
    let mut calls = 0;
    let failure = bullet_layout_with_part_factory(&config(), &source, &options(), &mut |ctx| {
        calls += 1;
        if calls == 3 {
            return Err(ChartError("late callback rejection".into()));
        }
        Ok(ctx.inherit(rectangle(ctx.position[0], ctx.position[1], 2., 2.)))
    })
    .unwrap_err();
    assert_eq!(calls, 3);
    assert!(failure.0.contains("late callback rejection"));
    assert_eq!(source, saved_source);
    assert_eq!(
        accepted,
        bullet_layout(&config(), &source, &options()).unwrap()
    );
}
