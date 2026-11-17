use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn data() -> Vec<BulletDatum> {
    vec![
        BulletDatum {
            id: "A <&> Å".into(),
            ranges: vec![7., -3., 0., 7.],
            measures: vec![5., -2.],
            markers: vec![4., 20.],
        },
        BulletDatum {
            id: "B".into(),
            ranges: vec![2., 8.],
            measures: vec![3.],
            markers: vec![0.],
        },
    ]
}
fn options() -> BulletLayoutOptions {
    BulletLayoutOptions {
        min: Some(-5.),
        max: Some(10.),
        ..Default::default()
    }
}
fn offset(ctx: &BulletPartContext<'_>, dx: f64) -> ScatterNodeGeometry {
    let [x, y] = ctx.position;
    let mut value = ctx.inherit(Geometry::Path(vec![
        C::Move(P::new(x + dx - 4., y - 4.)),
        C::Line(P::new(x + dx + 4., y - 4.)),
        C::Line(P::new(x + dx + 4., y + 4.)),
        C::Line(P::new(x + dx - 4., y + 4.)),
        C::Close,
    ]));
    value.fill = Some(Color(250, 10, 40, 255));
    value.stroke = None;
    value
}
fn hovered(out: &CustomBulletLayout, picking: Picking, point: [f64; 2]) -> Vec<String> {
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
    surface.hovered().to_vec()
}

#[test]
fn inheritance_preserves_whole_layout_and_visits_original_snapshots() {
    let c = ChartConfig::default();
    let data = data();
    for orientation in [
        StatisticsOrientation::Horizontal,
        StatisticsOrientation::Vertical,
    ] {
        for reverse in [false, true] {
            for domain_mode in [BulletDomainMode::Shared, BulletDomainMode::PerRow] {
                let mut o = options();
                o.orientation = orientation;
                o.reverse = reverse;
                o.domain_mode = domain_mode;
                o.order = Some(vec!["B".into(), "A <&> Å".into()]);
                let base = bullet_layout(&c, &data, &o).unwrap();
                let mut visited = vec![];
                let out = bullet_layout_with_part_factory(&c, &data, &o, &mut |ctx| {
                    assert_eq!(ctx.row, &base.rows[ctx.row_index]);
                    assert_eq!(ctx.datum, &data[ctx.source_row_index]);
                    assert_eq!(ctx.datum.id, ctx.row.id);
                    assert_eq!(ctx.geometry, &ctx.part.geometry);
                    let same_kind: Vec<_> = ctx
                        .row
                        .parts
                        .iter()
                        .filter(|p| p.kind == ctx.part.kind)
                        .collect();
                    assert_eq!(same_kind[ctx.rank].id, ctx.part.id);
                    visited.push(ctx.part.id.clone());
                    Ok(ctx.inherit(ctx.geometry.clone()))
                })
                .unwrap();
                assert_eq!(out.layout, base);
                let expected: Vec<_> = base
                    .rows
                    .iter()
                    .flat_map(|row| &row.parts)
                    .filter(|p| base.scene.marks.iter().any(|m| m.id == p.id))
                    .map(|p| p.id.clone())
                    .collect();
                assert_eq!(visited, expected);
                assert_eq!(
                    out.interaction
                        .sites
                        .iter()
                        .map(|s| s.mark_id.clone())
                        .collect::<Vec<_>>(),
                    expected
                );
                assert!(out.layout.scene.marks.len() > out.interaction.sites.len()); // Axes are decoration.
                out.interaction.validate(&out.layout.scene).unwrap();
            }
        }
    }
}

#[test]
fn custom_ink_updates_geometry_but_keeps_raw_metadata_clips_and_original_context_rows() {
    let c = ChartConfig::default();
    let data = data();
    let o = options();
    let base = bullet_layout(&c, &data, &o).unwrap();
    let out = bullet_layout_with_part_factory(&c, &data, &o, &mut |ctx| {
        assert_eq!(ctx.row, &base.rows[ctx.row_index]);
        Ok(offset(ctx, 15.))
    })
    .unwrap();
    assert_eq!(base.shared_domain, out.layout.shared_domain);
    assert_eq!(base.scene.labels, out.layout.scene.labels);
    assert_eq!(base.scene.legend, out.layout.scene.legend);
    for (before, after) in base.rows.iter().zip(&out.layout.rows) {
        assert_eq!(before.domain, after.domain);
        assert_eq!(before.track, after.track);
        for (p, q) in before.parts.iter().zip(&after.parts) {
            assert_eq!(p.id, q.id);
            assert_eq!(p.source_index, q.source_index);
            assert_eq!(p.interval, q.interval);
            assert_eq!(p.visible_interval, q.visible_interval);
            assert_eq!(p.raw_value, q.raw_value);
            if let Some(mark) = out.layout.scene.marks.iter().find(|m| m.id == q.id) {
                let old = base.scene.marks.iter().find(|m| m.id == q.id).unwrap();
                assert_eq!(q.geometry, mark.geometry);
                assert_ne!(q.geometry, p.geometry);
                assert_eq!(old.clip, mark.clip);
                assert_eq!(old.label, mark.label);
                assert_eq!(old.series, mark.series);
                assert_eq!(old.value, mark.value);
            } else {
                assert_eq!(p, q);
            }
        }
    }
    let svg = render::to_svg(&out.layout.scene).unwrap();
    assert!(svg.contains("A &lt;&amp;&gt; Å"));
    assert!(
        render::to_canvas_script(&out.layout.scene, "bullet")
            .unwrap()
            .contains("fillText")
    );
    assert!(
        out.interaction
            .sites
            .iter()
            .all(|s| s.color == Color(250, 10, 40, 255))
    );
}

#[test]
fn offset_marker_exact_uses_new_ink_and_nearest_uses_original_visible_part_center() {
    let c = ChartConfig::default();
    let data = [BulletDatum {
        id: "one".into(),
        ranges: vec![],
        measures: vec![],
        markers: vec![4.],
    }];
    let o = BulletLayoutOptions {
        min: Some(0.),
        max: Some(10.),
        ..Default::default()
    };
    let mut original = [0.; 2];
    let mut id = String::new();
    let out = bullet_layout_with_part_factory(&c, &data, &o, &mut |ctx| {
        if ctx.part.kind == BulletPartKind::Marker {
            original = ctx.position;
            id = ctx.part.id.clone();
            Ok(offset(ctx, 25.))
        } else {
            Ok(ctx.inherit(ctx.geometry.clone()))
        }
    })
    .unwrap();
    let ink = [original[0] + 25., original[1]];
    assert!(!hovered(&out, Picking::Geometry, original).contains(&id));
    assert!(hovered(&out, Picking::Geometry, ink).contains(&id));
    assert!(hovered(&out, Picking::Nearest { distance: 1. }, original).contains(&id));
    assert!(!hovered(&out, Picking::Nearest { distance: 1. }, ink).contains(&id));
    assert_eq!(
        out.interaction
            .sites
            .iter()
            .find(|s| s.mark_id == id)
            .unwrap()
            .position,
        original
    );
}

#[test]
fn scale_entry_preserves_resolved_scales_and_combines_reversal_and_nice() {
    let c = ChartConfig::default();
    let data = [BulletDatum {
        id: "positive".into(),
        ranges: vec![10., 100.],
        measures: vec![40.],
        markers: vec![50.],
    }];
    for domain_mode in [BulletDomainMode::Shared, BulletDomainMode::PerRow] {
        let o = BulletLayoutOptions {
            domain_mode,
            reverse: true,
            min: Some(3.),
            max: Some(760.),
            ..Default::default()
        };
        let scale = ContinuousScaleOptions {
            kind: ScaleKind::Log,
            nice: ScaleNice::Default,
            round: true,
            reverse: true,
            ..Default::default()
        };
        let base = bullet_layout_with_scale_options(&c, &data, &o, &scale).unwrap();
        let out =
            bullet_layout_with_scale_options_and_part_factory(&c, &data, &o, &scale, &mut |ctx| {
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
            .unwrap();
        assert_eq!(out.layout.layout, base.layout);
        assert_eq!(
            format!("{:?}", out.shared_value_scale),
            format!("{:?}", base.shared_value_scale)
        );
        assert_eq!(
            format!("{:?}", out.row_value_scales),
            format!("{:?}", base.row_value_scales)
        );
    }
}

#[test]
fn empty_hidden_and_zero_clipped_parts_do_not_create_phantom_sites() {
    let c = ChartConfig::default();
    let calls = std::cell::Cell::new(0);
    let mut factory = |ctx: &BulletPartContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(ctx.inherit(ctx.geometry.clone()))
    };
    let empty = bullet_layout_with_part_factory(&c, &[], &options(), &mut factory).unwrap();
    assert!(empty.interaction.sites.is_empty());
    let mut o = options();
    o.hidden = data().iter().map(|d| d.id.clone()).collect();
    let hidden = bullet_layout_with_part_factory(&c, &data(), &o, &mut factory).unwrap();
    assert!(hidden.interaction.sites.is_empty());
    assert_eq!(calls.get(), 0);
    let data = [BulletDatum {
        id: "zeros".into(),
        ranges: vec![0., 0.],
        measures: vec![-2., 0., 0.],
        markers: vec![-1., 20.],
    }];
    let o = BulletLayoutOptions {
        min: Some(0.),
        max: Some(10.),
        ..Default::default()
    };
    let out = bullet_layout_with_part_factory(&c, &data, &o, &mut factory).unwrap();
    assert_eq!(out.interaction.sites.len(), 1);
    assert!(out.interaction.sites[0].mark_id.ends_with(":range:domain"));
    assert_eq!(calls.get(), 1);
}

#[test]
fn invalid_output_and_late_callback_failure_return_no_candidate_and_preserve_source() {
    let c = ChartConfig::default();
    let source = data();
    let o = options();
    let base = bullet_layout(&c, &source, &o).unwrap();
    for invalid in [f64::NAN, -1., 1.1] {
        assert!(
            bullet_layout_with_part_factory(&c, &source, &o, &mut |ctx| {
                let mut value = ctx.inherit(ctx.geometry.clone());
                value.opacity = invalid;
                Ok(value)
            })
            .is_err()
        );
    }
    assert!(
        bullet_layout_with_part_factory(&c, &source, &o, &mut |ctx| Ok(ctx.inherit(
            Geometry::Circle {
                x: ctx.position[0] + 1.,
                y: ctx.position[1],
                radius: 3.
            }
        )))
        .is_err()
    );
    assert!(
        bullet_layout_with_part_factory(&c, &source, &o, &mut |ctx| Ok(ctx.inherit(
            Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: f64::NAN
            }
        )))
        .is_err()
    );
    let mut calls = 0;
    let error = bullet_layout_with_part_factory(&c, &source, &o, &mut |ctx| {
        calls += 1;
        if calls == 3 {
            Err(ChartError("late failure".into()))
        } else {
            Ok(offset(ctx, 20.))
        }
    })
    .unwrap_err();
    assert_eq!(error.0, "late failure");
    assert_eq!(calls, 3);
    assert_eq!(base, bullet_layout(&c, &source, &o).unwrap());
}

#[test]
fn cumulative_gradient_stop_budget_accepts_exact_limit_and_rejects_overflow() {
    let c = ChartConfig::default();
    let data = [BulletDatum {
        id: "many".into(),
        ranges: vec![],
        measures: vec![],
        markers: (1..=32).map(f64::from).collect(),
    }];
    let o = BulletLayoutOptions {
        min: Some(0.),
        max: Some(100.),
        axis: AxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let stops: Vec<_> = (0..512)
        .map(|i| GradientStop::new(f64::from(i) / 511., Color(30, 60, 100, 255)))
        .collect();
    for (count, valid) in [(32, true), (33, false)] {
        let mut visited = 0;
        let out = bullet_layout_with_part_factory(&c, &data, &o, &mut |ctx| {
            visited += 1;
            let mut value = ctx.inherit(ctx.geometry.clone());
            if visited <= count {
                value.fill_paint = Some(FillPaint::Linear {
                    start: [0., 0.],
                    end: [800., 0.],
                    stops: stops.clone(),
                });
            }
            Ok(value)
        });
        assert_eq!(out.is_ok(), valid);
        if let Err(error) = out {
            assert!(error.0.contains("paint budget"));
        }
    }
}

#[cfg(feature = "raster")]
#[test]
fn actual_cpu_pixels_follow_custom_marker_ink() {
    use render::raster::{FontRegistry, RasterOptions, render_raster};
    let c = ChartConfig::default();
    let data = [BulletDatum {
        id: "one".into(),
        ranges: vec![],
        measures: vec![],
        markers: vec![4.],
    }];
    let o = BulletLayoutOptions {
        min: Some(0.),
        max: Some(10.),
        axis: AxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut position = [0.; 2];
    let out = bullet_layout_with_part_factory(&c, &data, &o, &mut |ctx| {
        if ctx.part.kind == BulletPartKind::Marker {
            position = ctx.position;
            Ok(offset(ctx, 25.))
        } else {
            Ok(ctx.inherit(ctx.geometry.clone()))
        }
    })
    .unwrap();
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    let image = render_raster(&out.layout.scene, &fonts, &RasterOptions::default()).unwrap();
    let pixel = |x: f64, y: f64| {
        let i = (y.floor() as usize * image.width as usize + x.floor() as usize) * 4;
        &image.rgba[i..i + 4]
    };
    assert_eq!(pixel(position[0] + 25., position[1]), &[250, 10, 40, 255]);
    assert_ne!(pixel(position[0], position[1]), &[250, 10, 40, 255]);
}

#[test]
fn identity_uses_visible_stroke_color_without_rewriting_transparent_source_fill() {
    use glory::render::{GraphicStroke, StrokeCap};
    let c = ChartConfig::default();
    let source = [BulletDatum {
        id: "transparent".into(),
        ranges: vec![4.],
        measures: vec![],
        markers: vec![],
    }];
    let red = Color(220, 10, 30, 255);
    let o = BulletLayoutOptions {
        min: Some(0.),
        max: Some(10.),
        range_colors: vec![Color(0, 0, 0, 0)],
        range_border: Some(GraphicStroke {
            color: red,
            width: 2.,
            cap: StrokeCap::Butt,
        }),
        ..Default::default()
    };
    let old = bullet_layout(&c, &source, &o).unwrap();
    let out = bullet_layout_with_part_factory(&c, &source, &o, &mut |ctx| {
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    assert_eq!(out.layout, old);
    assert_eq!(out.interaction.sites.len(), 2);
    assert!(out.interaction.sites.iter().all(|site| site.color == red));
    let bound = out.into_scene_interaction();
    bound.validate().unwrap();
    assert_eq!(bound.scene, old.scene);
}

#[test]
fn strict_conversion_mounts_real_glory_graphics_without_changing_legacy_nearest() {
    let c = ChartConfig::default();
    let out = bullet_layout_with_part_factory(&c, &data(), &options(), &mut |ctx| {
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    assert_eq!(
        out.interaction.picking_profile,
        PickingProfile::LegacyGeometry
    );
    let strict = out.clone().into_painted_scene_interaction().unwrap();
    assert_eq!(strict.scene, out.layout.scene);
    assert_eq!(
        strict.interaction.as_ref().unwrap().picking_profile,
        PickingProfile::PaintedGeometry
    );
    assert!(
        SurfaceModel::new_with_interaction(
            strict.scene.clone(),
            SurfaceOptions {
                plot: strict.plot,
                picking: Picking::Nearest { distance: 10. },
                ..Default::default()
            },
            strict.interaction.clone()
        )
        .is_err()
    );
    let app = glory::App::memory();
    app.mount(chart_layout(strict)).unwrap();
    let ordinary = out.into_scene_interaction();
    SurfaceModel::new_with_interaction(
        ordinary.scene,
        SurfaceOptions {
            plot: ordinary.plot,
            picking: Picking::Nearest { distance: 10. },
            ..Default::default()
        },
        ordinary.interaction,
    )
    .unwrap();
}

#[test]
fn strict_conversion_rejects_gradient_ink_that_legacy_exports_support() {
    let c = ChartConfig::default();
    let out = bullet_layout_with_part_factory(&c, &data(), &options(), &mut |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [100., 0.],
            stops: vec![
                GradientStop {
                    offset: 0.,
                    color: Color(10, 20, 30, 255),
                },
                GradientStop {
                    offset: 1.,
                    color: Color(40, 50, 60, 255),
                },
            ],
        });
        Ok(ink)
    })
    .unwrap();
    render::to_svg(&out.layout.scene).unwrap();
    render::to_canvas_script(&out.layout.scene, "gradient").unwrap();
    let old = out.clone();
    let error = out.clone().into_painted_scene_interaction().unwrap_err();
    assert!(error.0.contains("solid eligible fills"));
    assert_eq!(out, old);
}
