use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::{
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};

fn frame(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    }
}

fn bare() -> SwarmLayoutOptions {
    SwarmLayoutOptions {
        orientation: SwarmOrientation::Horizontal,
        value_axis: None,
        group_axis: None,
        ..Default::default()
    }
}

fn pick(layout: &SwarmLayout, p: [f64; 2], picking: Picking) -> Vec<String> {
    let mut model = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            picking,
            plot: Some(layout.plot),
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (p[0], p[1]),
        })
        .unwrap();
    model.hovered().to_vec()
}

#[test]
fn exact_tangent_slots_and_stable_ties_do_not_jitter_the_value_coordinate() {
    let cfg = frame(80., 30.);
    let opts = SwarmLayoutOptions {
        spacing: 0.,
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::Circle { diameter: 10. },
            ..Default::default()
        },
        ..bare()
    };
    let mut points = ["c", "a", "b"].map(|id| SwarmPoint::new(id, "lane", 5.));
    let first = swarm_layout(&cfg, &points, &opts).unwrap();
    assert_eq!(first.value_scale.source_domain(), [5., 5.]);
    assert_eq!(first.value_scale.range(), [5., 75.]);
    assert_eq!(
        first
            .sites
            .iter()
            .map(|p| (p.node_id.as_str(), p.position))
            .collect::<Vec<_>>(),
        [("a", [40., 15.]), ("b", [40., 5.]), ("c", [40., 25.]),]
    );
    assert!(first.sites.iter().all(|p| p.ink_radius == 5.));
    points.reverse();
    let second = swarm_layout(&cfg, &points, &opts).unwrap();
    assert_eq!(first.scene, second.scene);
    for (i, point) in points.iter().enumerate() {
        let index = second.metadata[i].site_index.unwrap();
        assert_eq!(second.sites[index].node_id, point.id);
        assert_eq!(second.sites[index].source_index, i);
    }
    assert!(swarm_layout(&frame(80., 29.), &points, &opts).is_err());
}

#[test]
fn offset_at_fixed_value_uses_euclidean_collision_distance() {
    let cfg = frame(100., 50.);
    let opts = SwarmLayoutOptions {
        value_domain: Some([0., 100.]),
        value_fit: SwarmValueFit::Clip,
        spacing: 0.,
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::Circle { diameter: 10. },
            ..Default::default()
        },
        ..bare()
    };
    let out = swarm_layout(
        &cfg,
        &[
            SwarmPoint::new("a", "G", 40.),
            SwarmPoint::new("b", "G", 46.),
        ],
        &opts,
    )
    .unwrap();
    // 6-8-10 right triangle. The nearest legal cross slot is tangent.
    assert_eq!(out.sites[0].position, [40., 25.]);
    assert_eq!(out.sites[1].position, [46., 17.]);
}

#[test]
fn groups_have_explicit_order_empty_lanes_and_hidden_color_stability() {
    let cfg = frame(220., 110.);
    let opts = SwarmLayoutOptions {
        groups: Some(vec![
            SwarmGroup::new("B"),
            SwarmGroup::new("empty"),
            SwarmGroup::new("A"),
        ]),
        group_gap: 10.,
        group_axis: Some(CategoryAxisOptions {
            grid: true,
            ..Default::default()
        }),
        ..bare()
    };
    let points = [
        SwarmPoint::new("same", "A", 2.),
        SwarmPoint::new("same", "B", 19.),
    ];
    let full = swarm_layout(&cfg, &points, &opts).unwrap();
    assert_eq!(
        full.groups
            .iter()
            .map(|g| (g.id.as_str(), g.band, g.center))
            .collect::<Vec<_>>(),
        [
            ("B", [0., 30.], 15.),
            ("empty", [40., 70.], 55.),
            ("A", [80., 110.], 95.),
        ]
    );
    assert_eq!(
        full.scene
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["B", "empty", "A"]
    );
    assert_ne!(full.sites[0].mark_id, full.sites[1].mark_id);
    assert_eq!(full.sites[0].group_id, "B");
    assert_eq!(full.scene.marks[full.sites[1].mark_index].series, "A");
    assert_eq!(full.scene.legend[2].color, full.sites[1].color);

    let hidden = swarm_layout(
        &cfg,
        &points,
        &SwarmLayoutOptions {
            hidden: ["B".into()].into(),
            ..opts
        },
    )
    .unwrap();
    assert_eq!(hidden.scene.legend, full.scene.legend);
    assert_eq!(hidden.scene.labels, full.scene.labels);
    assert_eq!(
        hidden.value_scale.display_domain(),
        full.value_scale.display_domain()
    );
    assert_eq!(hidden.value_scale.range(), full.value_scale.range());
    assert_eq!(hidden.sites[0].position, full.sites[1].position);
    assert!(hidden.metadata[1].hidden && hidden.metadata[1].site_index.is_none());
    SceneInteraction::from(hidden).validate().unwrap();
}

#[test]
fn adjacent_lanes_keep_node_spacing_even_with_no_group_gap() {
    let cfg = frame(120., 84.);
    let opts = SwarmLayoutOptions {
        spacing: 4.,
        group_gap: 0.,
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::Circle { diameter: 10. },
            ..Default::default()
        },
        ..bare()
    };
    let points: Vec<_> = ["left", "right"]
        .into_iter()
        .flat_map(|group| ["A", "B", "C"].map(|id| SwarmPoint::new(id, group, 10.)))
        .collect();
    let out = swarm_layout(&cfg, &points, &opts).unwrap();
    assert_eq!(out.groups[0].band[1], out.groups[1].band[0]);
    assert_eq!(
        out.sites.iter().map(|p| p.position[1]).collect::<Vec<_>>(),
        [21., 7., 35., 63., 49., 77.],
    );
    assert_eq!(
        out.sites[4].position[1] - out.sites[2].position[1] - 10.,
        4.
    );
}

#[test]
fn configured_value_scales_drive_axis_and_ink_without_distorting_raw_values() {
    let cfg = frame(240., 120.);
    let mut opts = SwarmLayoutOptions {
        orientation: SwarmOrientation::Vertical,
        value_axis: Some(AxisOptions::default()),
        ..bare()
    };
    opts.value_scale.kind = ScaleKind::Log;
    opts.value_scale.reverse = true;
    opts.value_scale.round = true;
    opts.value_scale.log_base = 2.;
    let points = [
        SwarmPoint::new("a", "G", -7.),
        SwarmPoint::new("b", "G", -2.),
    ];
    let log = swarm_layout(&cfg, &points, &opts).unwrap();
    assert_eq!(log.value_scale.source_domain(), [-7., -2.]);
    assert_eq!(log.value_scale.display_domain(), [-8., -2.]);
    assert_eq!(log.value_scale.range(), [3., 117.]);
    assert_eq!(log.sites[1].position[1], 117.);
    for site in &log.sites {
        assert_eq!(site.position[1], log.value_scale.map(site.raw_value));
        assert_eq!(site.position[1].fract(), 0.);
        assert_eq!(log.scene.marks[site.mark_index].value, site.raw_value);
    }
    assert!(
        log.scene
            .labels
            .iter()
            .any(|l| l.text == "−8" || l.text == "-8")
    );

    opts.value_scale.kind = ScaleKind::SymLog { constant: 2. };
    opts.value_scale.reverse = false;
    opts.value_scale.round = false;
    let symlog = swarm_layout(
        &cfg,
        &[-80., 0., 80.].map(|v| SwarmPoint::new(v.to_string(), "G", v)),
        &opts,
    )
    .unwrap();
    assert_eq!(symlog.value_scale.map(0.), 60.);
    assert!(
        symlog
            .sites
            .iter()
            .all(|site| site.position[1] == symlog.value_scale.map(site.raw_value))
    );

    opts.value_scale.kind = ScaleKind::Utc;
    opts.value_domain = Some([0., 86_400_000.]);
    let utc = swarm_layout(&cfg, &[SwarmPoint::new("d", "G", 0.)], &opts).unwrap();
    assert_eq!(utc.sites[0].raw_value, 0.);
    assert!(utc.sites[0].formatted_value.contains("1970"));
    assert_eq!(utc.sites[0].position[1], 117.);
    assert!(swarm_layout(&cfg, &[SwarmPoint::new("fraction", "G", 0.25)], &opts).is_err());
}

#[test]
fn rotated_hollow_glyphs_use_real_geometry_and_conservative_stroke_envelopes() {
    let cfg = frame(320., 260.);
    let opts = SwarmLayoutOptions {
        spacing: 4.,
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::StrokedGlyph {
                shape: StrokedGlyph::Triangle,
                size: 460.,
                rotation: 0.63,
            },
            stroke: Some(GraphicStroke {
                color: Color(37, 110, 82, 255),
                width: 4.,
                cap: StrokeCap::Square,
            }),
            ..Default::default()
        },
        ..bare()
    };
    let mut points = ["A", "B", "C"].map(|id| SwarmPoint::new(id, "G", 5.));
    points[2].size = Some(160.);
    let out = swarm_layout(&cfg, &points, &opts).unwrap();
    for (i, a) in out.sites.iter().enumerate() {
        let mark = &out.scene.marks[a.mark_index];
        assert_eq!(mark.fill, None);
        assert_eq!(mark.stroke.unwrap().color, a.color);
        assert_eq!(a.position[0], out.value_scale.map(5.));
        let glyph = glyph_geometry(
            a.position,
            &GlyphOptions {
                shape: GlyphShape::Stroked(StrokedGlyph::Triangle),
                size: a.size,
                rotation: 0.63,
            },
        )
        .unwrap();
        assert_eq!(mark.geometry, glyph.geometry);
        assert_eq!(
            out.anchors[&a.mark_id].resolve(&mark.geometry).unwrap(),
            a.position
        );
        let Geometry::Path(commands) = &mark.geometry else {
            panic!("triangle must be a path")
        };
        let supports = commands.iter().filter_map(|c| match c {
            PathCommand::Move(p) | PathCommand::Line(p) => Some(p),
            _ => None,
        });
        for p in supports {
            assert!((p.x - a.position[0]).hypot(p.y - a.position[1]) + 8. <= a.ink_radius);
        }
        assert!(pick(&out, a.position, Picking::Geometry).is_empty());
        assert_eq!(
            pick(&out, a.position, Picking::Nearest { distance: 1. }),
            std::slice::from_ref(&a.mark_id)
        );
        for b in &out.sites[i + 1..] {
            assert!(
                (a.position[0] - b.position[0]).hypot(a.position[1] - b.position[1])
                    >= a.ink_radius + b.ink_radius + opts.spacing
            );
        }
    }
}

#[test]
fn clip_keeps_partial_ink_and_omits_only_disjoint_envelopes() {
    let cfg = ChartConfig {
        width: 120.,
        height: 100.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 5.,
            bottom: 5.,
        },
        ..Default::default()
    };
    let opts = SwarmLayoutOptions {
        value_domain: Some([0., 100.]),
        value_fit: SwarmValueFit::Clip,
        ..bare()
    };
    let out = swarm_layout(
        &cfg,
        &[
            SwarmPoint::new("partial", "G", -2.),
            SwarmPoint::new("outside", "G", -20.),
        ],
        &opts,
    )
    .unwrap();
    let site = &out.sites[0];
    assert_eq!(out.sites.len(), 1);
    assert_eq!(site.position, [out.value_scale.map(-2.), 50.]);
    assert!((site.position[0] - 8.).abs() < 1e-12);
    assert!(out.metadata[1].site_index.is_none());
    assert_eq!(out.metadata[1].raw_value, -20.);
    assert_eq!(out.scene.marks[site.mark_index].clip, Some(out.plot));
    assert_eq!(
        pick(&out, [10.25, 50.], Picking::Geometry),
        std::slice::from_ref(&site.mark_id)
    );
    assert!(pick(&out, [8., 50.], Picking::Geometry).is_empty());
}

#[test]
fn rejects_bad_input_before_publishing_a_candidate() {
    let cfg = frame(120., 100.);
    let opts = bare();
    let p = SwarmPoint::new("A", "G", 1.);
    assert!(swarm_layout(&cfg, &[p.clone(), p.clone()], &opts).is_err());
    let too_many = vec![p.clone(); 2001];
    assert!(
        swarm_layout(&cfg, &too_many, &opts)
            .unwrap_err()
            .0
            .contains("budget")
    );
    let broken_group = SwarmLayoutOptions {
        groups: Some(vec![SwarmGroup::new("other")]),
        ..opts.clone()
    };
    assert!(swarm_layout(&cfg, std::slice::from_ref(&p), &broken_group).is_err());
    let bad_style = SwarmLayoutOptions {
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::StrokedGlyph {
                shape: StrokedGlyph::Circle,
                size: 10.,
                rotation: 0.,
            },
            ..Default::default()
        },
        ..opts.clone()
    };
    assert!(swarm_layout(&cfg, std::slice::from_ref(&p), &bad_style).is_err());
    let bad_hidden = SwarmLayoutOptions {
        hidden: ["G".into()].into(),
        ..opts
    };
    assert!(
        swarm_layout(
            &cfg,
            &[SwarmPoint {
                value: f64::NAN,
                ..p
            }],
            &bad_hidden
        )
        .is_err()
    );
}

#[cfg(feature = "raster")]
#[test]
fn painted_caps_and_rotated_glyphs_do_not_share_pixels() {
    use kartis::render::raster::*;
    let opts = SwarmLayoutOptions {
        spacing: 3.,
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::StrokedGlyph {
                shape: StrokedGlyph::Diamond,
                size: 140.,
                rotation: 0.47,
            },
            stroke: Some(GraphicStroke {
                color: Color(12, 32, 75, 255),
                width: 5.,
                cap: StrokeCap::Square,
            }),
            ..Default::default()
        },
        ..bare()
    };
    let points = [
        SwarmPoint::new("a", "G", 6.),
        SwarmPoint {
            size: Some(0.),
            ..SwarmPoint::new("b", "G", 6.)
        },
    ];
    let out = swarm_layout(&frame(200., 180.), &points, &opts).unwrap();
    let mask = |mark: &Mark| {
        let mut scene = out.scene.clone();
        scene.marks = vec![mark.clone()];
        render_raster(&scene, &FontRegistry::new(), &Default::default())
            .unwrap()
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[0] < 254)
            .collect::<Vec<_>>()
    };
    let a = mask(&out.scene.marks[out.sites[0].mark_index]);
    let b = mask(&out.scene.marks[out.sites[1].mark_index]);
    assert!(a.iter().filter(|pixel| **pixel).count() > 80);
    assert!(b.iter().filter(|pixel| **pixel).count() >= 25);
    assert!(a.into_iter().zip(b).all(|(a, b)| !(a && b)));
}
