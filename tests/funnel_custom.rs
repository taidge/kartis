use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn data() -> Vec<FunnelDatum> {
    vec![FunnelDatum::new("a", 8.), FunnelDatum::new(2., 4.)]
}

#[test]
fn exact_inheritance_preserves_every_legacy_field_and_gradient_anchor() {
    let mut source = data();
    source[0].paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [100., 100.],
        stops: vec![
            GradientStop::new(0., Color(200, 10, 30, 255)),
            GradientStop::new(1., Color(10, 80, 200, 0)),
        ],
    });
    for direction in [FunnelDirection::Horizontal, FunnelDirection::Vertical] {
        for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth] {
            let o = FunnelLayoutOptions {
                direction,
                interpolation,
                fill_opacity: 0.35,
                ..Default::default()
            };
            let c = ChartConfig::default();
            let baseline = funnel_layout(&c, &source, &o).unwrap();
            let mut calls = 0;
            let custom = funnel_layout_with_stage_factory(&c, &source, &o, &mut |ctx| {
                assert_eq!(ctx.datum, &source[calls]);
                assert_eq!(ctx.source_index, calls);
                assert_eq!(ctx.part.source_index, calls);
                calls += 1;
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
            .unwrap();
            assert_eq!(calls, 2);
            assert_eq!(custom, baseline);
        }
    }
}

fn offset(layout: &ChartConfig) -> FunnelLayout {
    funnel_layout_with_stage_factory(
        layout,
        &data()[..1],
        &FunnelLayoutOptions::default(),
        &mut |ctx| {
            let [x, y] = ctx.part.center;
            let mut out = ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 20., y - 5.)),
                C::Line(P::new(x + 30., y - 5.)),
                C::Line(P::new(x + 30., y + 5.)),
                C::Line(P::new(x + 20., y + 5.)),
                C::Close,
            ]));
            out.retain_profile_border = false;
            out.body.fill = Some(Color(210, 30, 90, 255));
            out.body.fill_paint = None;
            Ok(out)
        },
    )
    .unwrap()
}

#[test]
fn offset_ink_uses_actual_geometry_while_nearest_keeps_raw_center() {
    let layout = offset(&ChartConfig::default());
    assert!(
        layout
            .scene
            .marks
            .iter()
            .all(|m| !m.id.starts_with("funnel:border:"))
    );
    assert_eq!(layout.parts[0].datum, data()[0]);
    assert_eq!(layout.parts[0].color, Color(210, 30, 90, 255));
    assert_eq!(layout.scene.legend[0].color, layout.parts[0].color);
    let p = layout.parts[0].center;
    for (picking, point, expected) in [
        (Picking::Geometry, p, false),
        (Picking::Geometry, [p[0] + 25., p[1]], true),
        (Picking::Nearest { distance: 1. }, p, true),
        (Picking::Nearest { distance: 1. }, [p[0] + 25., p[1]], false),
    ] {
        let mut surface = SurfaceModel::new_with_interaction(
            layout.scene.clone(),
            SurfaceOptions {
                plot: Some(layout.plot()),
                picking,
                ..Default::default()
            },
            Some(layout.interaction_binding()),
        )
        .unwrap();
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (point[0], point[1]),
            })
            .unwrap();
        assert_eq!(!surface.hovered().is_empty(), expected);
    }
}

#[test]
fn invalid_custom_frames_reject_without_modifying_source_or_accepted_layout() {
    let c = ChartConfig::default();
    let d = data();
    let old = funnel_layout(&c, &d, &Default::default()).unwrap();
    for bad in [f64::NAN, -1., 1.1] {
        assert!(
            funnel_layout_with_stage_factory(&c, &d, &Default::default(), &mut |ctx| {
                let mut out = ctx.inherit(ctx.geometry.clone());
                out.body.opacity = bad;
                Ok(out)
            })
            .is_err()
        );
    }
    assert!(
        funnel_layout_with_stage_factory(&c, &d, &Default::default(), &mut |_| Err(ChartError(
            "application failure".into()
        )))
        .is_err()
    );
    assert_eq!(old, funnel_layout(&c, &d, &Default::default()).unwrap());
}

#[test]
fn centered_primitives_and_path_command_budget_are_enforced() {
    let c = ChartConfig::default();
    assert!(
        funnel_layout_with_stage_factory(&c, &data(), &Default::default(), &mut |ctx| Ok(ctx
            .inherit(Geometry::Circle {
                x: ctx.part.center[0] + 1.,
                y: ctx.part.center[1],
                radius: 5.
            })))
        .is_err()
    );
    assert!(
        funnel_layout_with_stage_factory(&c, &data(), &Default::default(), &mut |ctx| Ok(
            ctx.inherit(Geometry::Path(vec![C::Move(P::new(0., 0.)); 200001]))
        ))
        .is_err()
    );
}

#[test]
fn zero_width_non_drawable_stages_keep_metadata_without_callback_or_fake_sites() {
    let c = ChartConfig::default();
    let mut calls = 0;
    let o = FunnelLayoutOptions {
        size: FunnelSize::OrdinalPixels(vec![0.]),
        ..Default::default()
    };
    let out = funnel_layout_with_stage_factory(&c, &data(), &o, &mut |ctx| {
        calls += 1;
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert_eq!(out.parts.len(), 2);
    assert_eq!(out.scene.legend.len(), 2);
    assert!(out.interaction_binding().sites.is_empty());
}

#[test]
fn final_geometry_budget_excludes_later_replaced_source_bodies() {
    let options = FunnelLayoutOptions {
        before: None,
        after: None,
        border_width: 0.,
        ..Default::default()
    };
    let layout =
        funnel_layout_with_stage_factory(&ChartConfig::default(), &data(), &options, &mut |ctx| {
            let p = P::new(ctx.part.center[0], ctx.part.center[1]);
            let n = if ctx.source_index == 0 { 199_995 } else { 5 };
            let mut path = vec![C::Line(p); n];
            path[0] = C::Move(p);
            let mut out = ctx.inherit(Geometry::Path(path));
            out.body.anchor_commands = Some(vec![0]);
            out.retain_profile_border = false;
            Ok(out)
        })
        .unwrap();
    assert_eq!(
        layout
            .scene
            .marks
            .iter()
            .map(|m| match &m.geometry {
                Geometry::Path(p) => p.len(),
                _ => 0,
            })
            .sum::<usize>(),
        200_000
    );
}
