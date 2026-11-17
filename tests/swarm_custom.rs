use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 300.,
        height: 200.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}
fn options() -> SwarmLayoutOptions {
    SwarmLayoutOptions {
        orientation: SwarmOrientation::Horizontal,
        value_axis: None,
        group_axis: None,
        ..Default::default()
    }
}
fn points() -> [SwarmPoint; 3] {
    [
        SwarmPoint::new("c", "B", 1.),
        SwarmPoint::new("b", "A", 2.),
        SwarmPoint::new("a", "A", 2.),
    ]
}
fn pick(layout: &SwarmLayout, point: [f64; 2], picking: Picking) -> Vec<String> {
    let mut host = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(layout.plot),
            picking,
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap();
    host.apply(SurfaceInput::Move {
        id: 1,
        point: (point[0], point[1]),
    })
    .unwrap();
    host.hovered().to_vec()
}
fn rectangle(x: f64, y: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + 8., y)),
        C::Line(P::new(x + 8., y + 8.)),
        C::Line(P::new(x, y + 8.)),
        C::Close,
    ])
}

#[test]
fn exact_inheritance_retains_entire_default_frame_and_packed_callback_order() {
    for shape in [
        SwarmNodeShape::Circle { diameter: 10. },
        SwarmNodeShape::FilledGlyph {
            shape: FilledGlyph::Triangle,
            area: 64.,
            rotation: 0.3,
        },
        SwarmNodeShape::StrokedGlyph {
            shape: StrokedGlyph::Plus,
            size: 8.,
            rotation: 0.2,
        },
    ] {
        let mut o = options();
        o.node.shape = shape;
        if matches!(shape, SwarmNodeShape::StrokedGlyph { .. }) {
            o.node.fill = None;
            o.node.stroke = Some(GraphicStroke {
                color: Color(10, 20, 30, 255),
                width: 2.,
                cap: StrokeCap::Round,
            });
        }
        let d = points();
        let original = swarm_layout(&frame(), &d, &o).unwrap();
        let mut order = vec![];
        let custom = swarm_layout_with_node_factory(&frame(), &d, &o, |ctx| {
            order.push(ctx.source_index);
            assert_eq!(ctx.datum, &d[ctx.source_index]);
            assert_eq!(ctx.site, &original.sites[ctx.site_index]);
            assert_eq!(ctx.group, &original.groups[ctx.group_index]);
            assert_eq!(ctx.position, ctx.site.position);
            assert_eq!(ctx.value_scale.map(ctx.datum.value), ctx.position[0]);
            assert_eq!(
                ctx.opacity,
                original.scene.marks[ctx.site.mark_index].opacity
            );
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .unwrap();
        assert_eq!(order, vec![2, 1, 0]);
        assert_eq!(custom.scene, original.scene);
        assert_eq!(custom.sites, original.sites);
        assert_eq!(custom.metadata, original.metadata);
        assert_eq!(custom.groups, original.groups);
        assert_eq!(
            custom.value_scale.source_domain(),
            original.value_scale.source_domain()
        );
        assert_eq!(custom.value_scale.range(), original.value_scale.range());
        assert_eq!(custom.anchors, original.anchors);
        assert_eq!(custom.interaction_binding(), original.interaction_binding());
    }
}

#[test]
fn source_eligibility_retains_zero_and_transparent_sites_but_skips_hidden_and_culled() {
    let mut d = vec![
        SwarmPoint::new("hidden", "hidden", 0.),
        SwarmPoint::new("culled", "shown", 100.),
        SwarmPoint::new("zero", "shown", 5.),
        SwarmPoint::new("alpha", "shown", 6.),
    ];
    d[2].size = Some(0.);
    d[3].color = Some(Color(10, 20, 30, 0));
    let o = SwarmLayoutOptions {
        value_fit: SwarmValueFit::Clip,
        value_domain: Some([0., 10.]),
        hidden: BTreeSet::from(["hidden".into()]),
        ..options()
    };
    for opacity in [0., 1.] {
        let mut o = o.clone();
        o.node.opacity = opacity;
        let mut calls = vec![];
        let out = swarm_layout_with_node_factory(&frame(), &d, &o, |ctx| {
            calls.push(ctx.source_index);
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .unwrap();
        assert_eq!(calls, [2, 3]);
        assert!(out.metadata[0].hidden);
        assert!(out.metadata[0].site_index.is_none());
        assert!(out.metadata[1].site_index.is_none());
        assert_eq!(out.sites[0].size, 0.);
        assert_eq!(out.sites[1].ink_radius, 0.);
    }
}

#[test]
fn custom_ink_has_exact_picking_while_nearest_and_collision_remain_at_original_sites() {
    let d = [SwarmPoint::new("one", "G", 5.)];
    let old = swarm_layout(&frame(), &d, &options()).unwrap();
    let out = swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
        let mut custom = ctx.inherit(rectangle(ctx.position[0] + 30., ctx.position[1]));
        custom.fill = Some(Color(200, 20, 30, 255));
        Ok(custom)
    })
    .unwrap();
    let a = &old.sites[0];
    let b = &out.sites[0];
    assert_eq!(b.position, a.position);
    assert_eq!(b.ink_radius, a.ink_radius);
    assert_eq!(out.metadata, old.metadata);
    assert_eq!(out.groups, old.groups);
    assert_eq!(out.scene.legend, old.scene.legend);
    assert_eq!(b.color, Color(200, 20, 30, 255));
    assert_eq!(
        out.anchors[&b.mark_id]
            .resolve(&out.scene.marks[b.mark_index].geometry)
            .unwrap(),
        a.position
    );
    assert!(pick(&out, b.position, Picking::Geometry).is_empty());
    assert_eq!(
        pick(
            &out,
            [b.position[0] + 34., b.position[1] + 4.],
            Picking::Geometry
        ),
        std::slice::from_ref(&b.mark_id)
    );
    assert_eq!(
        pick(&out, b.position, Picking::Nearest { distance: 1. }),
        std::slice::from_ref(&b.mark_id)
    );
    assert!(
        pick(
            &out,
            [b.position[0] + 34., b.position[1] + 4.],
            Picking::Nearest { distance: 1. }
        )
        .is_empty()
    );
    let before = &old.scene.marks[a.mark_index];
    let after = &out.scene.marks[b.mark_index];
    assert_eq!(
        (
            &after.id,
            &after.series,
            &after.label,
            after.value,
            after.clip
        ),
        (
            &before.id,
            &before.series,
            &before.label,
            before.value,
            before.clip
        )
    );
}

#[test]
fn custom_paint_dash_opacity_and_zero_ink_preserve_logical_interaction() {
    let d = [SwarmPoint::new("one", "G", 5.)];
    let out = swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
        let mut custom = ctx.inherit(ctx.geometry.clone());
        custom.fill = None;
        custom.fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [10., 0.],
            stops: vec![
                GradientStop::new(0., Color(220, 10, 20, 255)),
                GradientStop::new(1., Color(10, 200, 20, 255)),
            ],
        });
        custom.stroke = Some(GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 2.,
            cap: StrokeCap::Square,
        });
        custom.stroke_dash = Some(StrokeDash {
            segments: vec![3., 2.],
            offset: 1.,
        });
        custom.opacity = 0.4;
        Ok(custom)
    })
    .unwrap();
    let mark = &out.scene.marks[out.sites[0].mark_index];
    assert!(mark.fill_paint.is_some());
    assert!(mark.stroke_dash.is_some());
    assert_eq!(mark.opacity, 0.4);
    let zero = swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
        let mut custom = ctx.inherit(Geometry::Path(vec![C::Move(P::new(
            ctx.position[0],
            ctx.position[1],
        ))]));
        custom.stroke = None;
        Ok(custom)
    })
    .unwrap();
    assert!(pick(&zero, zero.sites[0].position, Picking::Geometry).is_empty());
    assert_eq!(
        pick(
            &zero,
            zero.sites[0].position,
            Picking::Nearest { distance: 1. }
        ),
        std::slice::from_ref(&zero.sites[0].mark_id)
    );
}

#[test]
fn callback_errors_invalid_geometry_and_paint_reject_atomic_candidates() {
    let d = points();
    let original = swarm_layout(&frame(), &d, &options()).unwrap();
    let mut calls = 0;
    let result = swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
        calls += 1;
        if calls == 2 {
            return Err(ChartError("application failure".into()));
        }
        Ok(ctx.inherit(ctx.geometry.clone()))
    });
    assert_eq!(result.unwrap_err().0, "application failure");
    assert_eq!(calls, 2);
    for invalid in 0..6 {
        assert!(
            swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
                let mut custom = ctx.inherit(ctx.geometry.clone());
                match invalid {
                    0 => {
                        custom.geometry = Geometry::Circle {
                            x: ctx.position[0] + 1.,
                            y: ctx.position[1],
                            radius: 4.,
                        }
                    }
                    1 => custom.opacity = f64::NAN,
                    2 => {
                        custom.geometry = Geometry::Circle {
                            x: ctx.position[0],
                            y: ctx.position[1],
                            radius: -1.,
                        }
                    }
                    3 => custom.anchor_commands = Some(vec![0]),
                    4 => custom.geometry = Geometry::Path(vec![C::Line(P::new(1., 1.))]),
                    _ => {
                        custom.fill_paint = Some(FillPaint::Linear {
                            start: [0., 0.],
                            end: [0., 0.],
                            stops: vec![],
                        })
                    }
                }
                Ok(custom)
            })
            .is_err()
        );
    }
    assert_eq!(
        swarm_layout(&frame(), &d, &options()).unwrap().scene,
        original.scene
    );
}

#[test]
fn path_and_paint_budgets_accumulate_across_nodes_and_preserve_exact_limit() {
    let d = [SwarmPoint::new("a", "G", 0.), SwarmPoint::new("b", "G", 1.)];
    for total in [200_000, 200_001] {
        let out = swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
            let p = P::new(ctx.position[0], ctx.position[1]);
            let count = if ctx.site_index == 0 {
                100_000
            } else {
                total - 100_000
            };
            let mut path = vec![C::Line(p); count];
            path[0] = C::Move(p);
            let mut custom = ctx.inherit(Geometry::Path(path));
            custom.anchor_commands = Some(vec![0]);
            Ok(custom)
        });
        assert_eq!(out.is_ok(), total == 200_000);
    }
    let d: Vec<_> = (0..33)
        .map(|i| SwarmPoint::new(i.to_string(), "G", i as f64))
        .collect();
    let mut calls = 0;
    let out = swarm_layout_with_node_factory(&frame(), &d, &options(), |ctx| {
        calls += 1;
        let mut custom = ctx.inherit(ctx.geometry.clone());
        custom.fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [10., 0.],
            stops: (0..512)
                .map(|i| GradientStop::new(i as f64 / 511., Color(10, 20, 30, 255)))
                .collect(),
        });
        Ok(custom)
    });
    assert!(out.unwrap_err().0.contains("paint budget"));
    assert_eq!(calls, 33);
}

#[test]
fn failed_source_or_packing_never_invokes_factory_and_custom_extent_uses_original_clip() {
    let mut tiny = frame();
    tiny.height = 21.;
    let mut calls = 0;
    assert!(
        swarm_layout_with_node_factory(&tiny, &points(), &options(), |ctx| {
            calls += 1;
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    let mut bad = points();
    bad[2].value = f64::NAN;
    assert!(
        swarm_layout_with_node_factory(&frame(), &bad, &options(), |ctx| {
            calls += 1;
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    let d = [SwarmPoint::new("edge", "G", 0.)];
    let o = SwarmLayoutOptions {
        value_domain: Some([0., 10.]),
        value_fit: SwarmValueFit::Clip,
        ..options()
    };
    let layout = swarm_layout_with_node_factory(&frame(), &d, &o, |ctx| {
        Ok(ctx.inherit(Geometry::Rect {
            x: ctx.position[0] - 20.,
            y: ctx.position[1] - 20.,
            width: 40.,
            height: 40.,
        }))
    })
    .unwrap();
    let site = &layout.sites[0];
    assert_eq!(site.position[0], layout.plot.x);
    assert!(
        pick(
            &layout,
            [site.position[0] - 1., site.position[1]],
            Picking::Geometry
        )
        .is_empty()
    );
    assert_eq!(
        pick(
            &layout,
            [site.position[0] + 1., site.position[1]],
            Picking::Geometry
        ),
        std::slice::from_ref(&site.mark_id)
    );
}

#[test]
fn authored_anchor_support_follows_path_translation_without_moving_business_origin() {
    let layout = swarm_layout_with_node_factory(
        &frame(),
        &[SwarmPoint::new("one", "G", 3.)],
        &options(),
        |ctx| {
            let [x, y] = ctx.position;
            let mut custom = ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 20., y)),
                C::Cubic {
                    first: P::new(x + 30., y - 20.),
                    second: P::new(x + 50., y + 20.),
                    end: P::new(x + 60., y),
                },
            ]));
            custom.anchor_commands = Some(vec![0, 1]);
            Ok(custom)
        },
    )
    .unwrap();
    let site = &layout.sites[0];
    let anchor = &layout.anchors[&site.mark_id];
    let mark = &layout.scene.marks[site.mark_index];
    assert_eq!(anchor.resolve(&mark.geometry).unwrap(), site.position);
    let Geometry::Path(mut path) = mark.geometry.clone() else {
        unreachable!()
    };
    for command in &mut path {
        match command {
            C::Move(p) | C::Line(p) => {
                p.x += 7.;
                p.y -= 3.;
            }
            C::Cubic { first, second, end } => {
                for p in [first, second, end] {
                    p.x += 7.;
                    p.y -= 3.;
                }
            }
            C::Close => {}
        }
    }
    assert_eq!(
        anchor.resolve(&Geometry::Path(path)).unwrap(),
        [site.position[0] + 7., site.position[1] - 3.]
    );
}
