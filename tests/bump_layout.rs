use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::{
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};
use std::collections::BTreeSet;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 240.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn plain() -> BumpLayoutOptions {
    BumpLayoutOptions {
        axes: BumpAxes::none(),
        legend: false,
        labels: BumpEndpointLabels {
            end: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn series(id: &str, points: Vec<BumpDatum>) -> BumpSeries {
    BumpSeries::new(id, points)
}
fn profiles() -> Vec<Vec<BumpSeries>> {
    vec![
        vec![
            series(
                "A",
                vec![
                    BumpDatum::new("Q1", 1.),
                    BumpDatum::new(1., 4.),
                    BumpDatum::new("1", 10.),
                    BumpDatum::new("Q4", 4.),
                ],
            ),
            series(
                "B",
                vec![
                    BumpDatum::new("Q1", 4.),
                    BumpDatum::new(1., 1.),
                    BumpDatum::gap("1"),
                    BumpDatum::new("Q4", 10.),
                ],
            ),
        ],
        vec![
            series(
                "A",
                vec![
                    BumpDatum::gap("Q1"),
                    BumpDatum::new("Q2", 4.),
                    BumpDatum::gap("Q3"),
                    BumpDatum::new("Q4", 10.),
                    BumpDatum::gap("Q5"),
                ],
            ),
            series(
                "B",
                vec![
                    BumpDatum::new("Q1", 1.),
                    BumpDatum::new("Q2", 10.),
                    BumpDatum::new("Q3", 4.),
                    BumpDatum::new("Q4", 1.),
                    BumpDatum::new("Q5", 4.),
                ],
            ),
        ],
        vec![
            series("A", vec![BumpDatum::new("only", 4.)]),
            series("B", vec![BumpDatum::gap("only")]),
        ],
        vec![
            series(
                "A",
                vec![
                    BumpDatum::new("Q3", -1.5),
                    BumpDatum::new("Q1", 0.),
                    BumpDatum::new("Q2", 2.25),
                ],
            ),
            series(
                "B",
                vec![BumpDatum::new("Q2", 10.), BumpDatum::new("Q3", 2.25)],
            ),
        ],
        vec![series(
            "A",
            vec![BumpDatum::gap("Q1"), BumpDatum::gap("Q2")],
        )],
        vec![
            series(
                "A",
                vec![BumpDatum::new("Q1", 1.), BumpDatum::new("Q2", 2.)],
            ),
            series(
                "B",
                vec![BumpDatum::new("Q2", 2.), BumpDatum::new("Q1", 1.)],
            ),
        ],
    ]
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 2e-10,
        "actual {actual}, expected {expected}"
    );
}
fn coordinates(command: &C) -> Vec<f64> {
    match command {
        C::Move(p) | C::Line(p) => vec![p.x, p.y],
        C::Cubic { first, second, end } => vec![first.x, first.y, second.x, second.y, end.x, end.y],
        C::Close => vec![],
    }
}
fn kind(command: &C) -> &str {
    match command {
        C::Move(_) => "M",
        C::Line(_) => "L",
        C::Cubic { .. } => "C",
        C::Close => "Z",
    }
}
fn key(encoded: &str) -> CategoryKey {
    match encoded.split_once(':').unwrap() {
        ("n", value) => CategoryKey::Number(value.parse().unwrap()),
        (_, value) => CategoryKey::from(value),
    }
}

#[test]
fn bump_matches_original_nivo_point_lattices_shoulders_and_d3_curves() {
    let fixture = include_str!("fixtures/bump-nivo.txt");
    let source = profiles();
    let mut layouts = vec![];
    for header in fixture.lines().filter(|line| line.starts_with("H|")) {
        let fields = header.split('|').collect::<Vec<_>>();
        let index: usize = fields[1].parse().unwrap();
        let mut options = plain();
        options.x_padding = fields[2].parse().unwrap();
        options.x_scale.padding = fields[3].parse().unwrap();
        options.rank_scale.padding = fields[4].parse().unwrap();
        options.interpolation = if fields[5] == "1" {
            BumpInterpolation::Smooth
        } else {
            BumpInterpolation::Linear
        };
        layouts.push(bump_layout(&frame(), &source[index], &options).unwrap());
    }
    assert_eq!(layouts.len(), 6);
    let mut checked = [0; 5];
    let mut paths = std::collections::BTreeMap::<(usize, usize), Vec<&C>>::new();
    for (i, layout) in layouts.iter().enumerate() {
        layout.clone().into_scene_interaction().validate().unwrap();
        for (s, series) in layout.series.iter().enumerate() {
            let commands = series
                .line_mark_ids
                .iter()
                .flat_map(|id| {
                    let mark = layout
                        .scene
                        .marks
                        .iter()
                        .find(|mark| &mark.id == id)
                        .unwrap();
                    let Geometry::Path(commands) = &mark.geometry else {
                        panic!("line lost path");
                    };
                    commands.iter()
                })
                .collect();
            paths.insert((i, s), commands);
            assert_eq!(series.source, source[i][s]);
            assert_eq!(series.points.len(), source[i][s].data.len());
        }
    }
    let mut consumed = std::collections::BTreeMap::<(usize, usize), usize>::new();
    for row in fixture
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty() && !line.starts_with("H|"))
    {
        let fields = row.split('|').collect::<Vec<_>>();
        let index: usize = fields[1].parse().unwrap();
        let layout = &layouts[index];
        match fields[0] {
            "X" => {
                close(
                    layout.x_scale.map(&key(fields[2])).unwrap(),
                    fields[3].parse().unwrap(),
                );
                checked[0] += 1;
            }
            "Y" => {
                close(
                    layout
                        .rank_scale
                        .map(&CategoryKey::Number(fields[2].parse().unwrap()))
                        .unwrap(),
                    fields[3].parse().unwrap(),
                );
                checked[1] += 1;
            }
            "P" => {
                let s: usize = fields[2].parse().unwrap();
                let p: usize = fields[3].parse().unwrap();
                let point = &layout.series[s].points[p];
                assert_eq!(point.raw_x, key(fields[4]));
                assert_eq!(point.rank, Some(fields[5].parse().unwrap()));
                let [x, y] = point.position.unwrap();
                close(x, fields[6].parse().unwrap());
                close(y, fields[7].parse().unwrap());
                assert!(point.mark_id.is_some());
                checked[2] += 1;
            }
            "L" => {
                let s: usize = fields[2].parse().unwrap();
                let p: usize = fields[3].parse().unwrap();
                if fields[4] == "null" {
                    assert!(layout.series[s].line_points[p].is_none());
                } else {
                    let [x, y] = layout.series[s].line_points[p].unwrap();
                    close(x, fields[4].parse().unwrap());
                    close(y, fields[5].parse().unwrap());
                }
                checked[3] += 1;
            }
            "G" => {
                let s: usize = fields[2].parse().unwrap();
                let cursor = consumed.entry((index, s)).or_default();
                let command = paths[&(index, s)][*cursor];
                *cursor += 1;
                assert_eq!(kind(command), fields[3]);
                let values = coordinates(command);
                assert_eq!(values.len(), fields.len() - 4);
                for (actual, expected) in values.iter().zip(&fields[4..]) {
                    close(*actual, expected.parse().unwrap());
                }
                checked[4] += 1;
            }
            _ => panic!("unknown fixture record"),
        }
    }
    for (key, commands) in paths {
        assert_eq!(commands.len(), consumed.get(&key).copied().unwrap_or(0));
    }
    assert_eq!(checked, [17, 13, 24, 79, 80]);
}

fn surface(layout: &BumpLayout, picking: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(layout.plot),
            picking,
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap()
}

#[test]
fn bump_custom_hollow_points_preserve_raw_origin_and_exact_painted_geometry() {
    let data = [series(
        "A",
        vec![BumpDatum::new(1., 1.), BumpDatum::new("1", 10.)],
    )];
    let mut options = plain();
    options.style.width = 0.;
    let mut calls = vec![];
    let layout = bump_layout_with_point_factory(&frame(), &data, &options, |context| {
        calls.push((
            context.datum.x.clone(),
            context.datum.rank,
            context.source_index,
            context.position,
        ));
        let [x, y] = context.position;
        let mut point = context.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 10., y - 6.)),
            C::Line(P::new(x + 22., y - 6.)),
            C::Line(P::new(x + 22., y + 6.)),
            C::Line(P::new(x + 10., y + 6.)),
            C::Close,
        ]));
        point.fill = None;
        point.stroke = Some(GraphicStroke {
            color: Color(177, 40, 90, 255),
            width: 3.,
            cap: StrokeCap::Butt,
        });
        Ok(point)
    })
    .unwrap();
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0].0, calls[1].0);
    let [x, y] = calls[0].3;
    let id = layout.series[0].points[0].mark_id.as_deref().unwrap();
    let mut exact = surface(&layout, Picking::Geometry);
    exact
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x + 22., y),
        })
        .unwrap();
    assert_eq!(exact.hovered(), [id]);
    let actual = exact.active_interaction(exact.hovered()).unwrap();
    assert_eq!(actual.rows[0].position, [x, y]);
    assert_eq!(actual.rows[0].value, Some(1.));
    exact
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x + 16., y),
        })
        .unwrap();
    assert!(exact.hovered().is_empty());
    let mut nearest = surface(&layout, Picking::Nearest { distance: 4. });
    nearest
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert_eq!(nearest.hovered(), [id]);
    nearest
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x + 16., y),
        })
        .unwrap();
    assert!(nearest.hovered().is_empty());
    assert!(
        render::to_svg(&layout.scene)
            .unwrap()
            .contains("stroke-width=\"3\"")
    );
    let prior = layout.scene.clone();
    assert!(
        bump_layout_with_point_factory(&frame(), &data, &options, |context| {
            Ok(context.inherit(Geometry::Circle {
                x: context.position[0] + 1.,
                y: context.position[1],
                radius: 3.,
            }))
        })
        .is_err()
    );
    assert_eq!(layout.scene, prior);
}

#[test]
fn bump_shared_triangle_and_hidden_sites_keep_typed_origins_and_lattices() {
    let data = profiles().remove(0);
    let mut options = plain();
    options.style.point.glyph = Some(GlyphOptions {
        shape: GlyphShape::Filled(FilledGlyph::Triangle),
        size: 100.,
        rotation: 0.4,
    });
    let normal = bump_layout(&frame(), &data, &options).unwrap();
    let id = normal.series[0].points[0].mark_id.as_ref().unwrap();
    let site = normal
        .interaction
        .sites
        .iter()
        .find(|s| &s.mark_id == id)
        .unwrap();
    assert_eq!(
        normal
            .interaction
            .anchor_for(normal.scene.marks.iter().find(|m| &m.id == id).unwrap())
            .unwrap(),
        Some(site.position)
    );
    options.hidden = BTreeSet::from(["A".into()]);
    let hidden = bump_layout(&frame(), &data, &options).unwrap();
    assert_eq!(normal.x_scale, hidden.x_scale);
    assert_eq!(normal.rank_scale, hidden.rank_scale);
    assert!(hidden.series[0].points.iter().all(|p| p.mark_id.is_none()));
    assert_eq!(hidden.series[1].color, normal.series[1].color);
    assert!(hidden.interaction.sites.iter().all(|s| s.series == "B"));
}

#[test]
fn bump_rank_axes_use_distinct_values_and_endpoint_labels_follow_original_gaps() {
    let data = profiles().remove(1);
    let mut options = BumpLayoutOptions::default();
    options.labels.start = true;
    let layout = bump_layout(&frame(), &data, &options).unwrap();
    assert_eq!(
        layout.rank_scale.domain(),
        [
            CategoryKey::Number(1.),
            CategoryKey::Number(4.),
            CategoryKey::Number(10.)
        ]
    );
    let rank_labels = layout
        .scene
        .labels
        .iter()
        .filter(|l| l.anchor == TextAnchor::End && ["1", "4", "10"].contains(&l.text.as_str()))
        .count();
    assert_eq!(rank_labels, 3);
    let labels = layout
        .scene
        .labels
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            layout
                .scene
                .label_series
                .get(&i)
                .map(|s| (s.as_str(), l.text.as_str()))
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, [("B", "B"), ("B", "B")]);
    options.x_ticks = Some(vec![CategoryKey::from("unknown")]);
    assert!(bump_layout(&frame(), &data, &options).is_err());
}

#[test]
fn bump_ids_survive_reordering_and_zero_points_remain_logical_sites() {
    let mut data = profiles().remove(0);
    let mut options = plain();
    options.style.point.size = 0.;
    options.style.width = 0.;
    let first = bump_layout(&frame(), &data, &options).unwrap();
    data.reverse();
    data[1].data.reverse();
    let second = bump_layout(&frame(), &data, &options).unwrap();
    let identities = |layout: &BumpLayout| {
        layout
            .interaction
            .sites
            .iter()
            .map(|s| s.mark_id.clone())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(identities(&first), identities(&second));
    let site = &first.interaction.sites[0];
    let [x, y] = site.position;
    let mut exact = surface(&first, Picking::Geometry);
    exact
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert!(exact.hovered().is_empty());
    let mut nearest = surface(&first, Picking::Nearest { distance: 1. });
    nearest
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert_eq!(nearest.hovered(), [site.mark_id.as_str()]);
}

#[test]
fn bump_reverse_point_scale_keeps_shoulders_oriented_and_rejects_invalid_candidates() {
    let data = [series(
        "A",
        vec![BumpDatum::new("first", 1.), BumpDatum::new("last", 10.)],
    )];
    let mut options = plain();
    options.x_scale.reverse = true;
    options.rank_scale.reverse = true;
    let layout = bump_layout(&frame(), &data, &options).unwrap();
    assert_eq!(
        layout.series[0].line_points.first(),
        Some(&Some([400., 180.]))
    );
    assert_eq!(layout.series[0].line_points.last(), Some(&Some([0., 60.])));
    assert_eq!(layout.series[0].points[0].position, Some([300., 180.]));
    for invalid in [
        BumpLayoutOptions {
            x_padding: f64::NAN,
            ..plain()
        },
        BumpLayoutOptions {
            hidden: BTreeSet::from(["missing".into()]),
            ..plain()
        },
        BumpLayoutOptions {
            x_scale: PointScaleOptions {
                padding: -1.,
                ..Default::default()
            },
            ..plain()
        },
    ] {
        assert!(bump_layout(&frame(), &data, &invalid).is_err());
    }
    assert!(
        bump_layout(
            &frame(),
            &[series(
                "A",
                vec![BumpDatum::new("x", 1.), BumpDatum::new("x", 4.)]
            )],
            &plain()
        )
        .is_err()
    );
    assert!(
        bump_layout(
            &frame(),
            &[series("A", vec![BumpDatum::new("x", f64::INFINITY)])],
            &plain()
        )
        .is_err()
    );
    assert!(
        bump_layout(&frame(), &[], &plain())
            .unwrap()
            .scene
            .marks
            .is_empty()
    );
}
