use std::f64::consts::{FRAC_PI_2, TAU};

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 248.,
        height: 226.,
        margin: Margin {
            left: 23.,
            right: 25.,
            top: 17.,
            bottom: 29.,
        },
        ..Default::default()
    }
}

fn options() -> ChordLayoutOptions {
    ChordLayoutOptions {
        labels: None,
        value_format: ValueFormat::Fixed(2),
        labels_by_id: [
            ("g0".into(), "label-g0".into()),
            ("g1".into(), "label-g1".into()),
            ("g2".into(), "label-g2".into()),
        ]
        .into(),
        ..Default::default()
    }
}

fn profile(name: &str) -> Vec<Vec<f64>> {
    match name {
        "asymmetric" => vec![vec![2., 4., 0.], vec![9., 0., 1.], vec![0., 0., 0.]],
        "dominance-reversed" => vec![vec![2., 12., 0.], vec![9., 0., 1.], vec![0., 0., 0.]],
        "symmetric" => vec![vec![0., 1.], vec![1., 0.]],
        "self" => vec![vec![7.]],
        "oneway" => vec![vec![0., 5.], vec![0., 0.]],
        _ => panic!("unknown oracle profile"),
    }
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} differs from {expected}"
    );
}

fn number(text: &str) -> f64 {
    text.parse().unwrap()
}

fn xy(point: P) -> [f64; 2] {
    [point.x, point.y]
}

fn cubic(start: [f64; 2], a: [f64; 2], b: [f64; 2], end: [f64; 2], t: f64) -> [f64; 2] {
    std::array::from_fn(|i| {
        (1. - t).powi(3) * start[i]
            + 3. * (1. - t).powi(2) * t * a[i]
            + 3. * (1. - t) * t * t * b[i]
            + t.powi(3) * end[i]
    })
}

fn verify_ribbon_path(commands: &[C], records: &str, center: [f64; 2]) {
    let mut index = 0;
    let mut start = [0.; 2];
    for record in records.split('~') {
        let parts = record.split(',').collect::<Vec<_>>();
        match parts[0] {
            "M" => {
                let C::Move(point) = commands[index] else {
                    panic!("missing ribbon move")
                };
                start = [center[0] + number(parts[1]), center[1] + number(parts[2])];
                close(point.x, start[0]);
                close(point.y, start[1]);
                index += 1;
            }
            "A" => {
                let r = number(parts[3]);
                let a = number(parts[4]);
                let b = number(parts[5]);
                let count = if a == b {
                    0
                } else {
                    ((b - a).abs() / FRAC_PI_2).ceil().max(1.) as usize
                };
                for segment in 0..count {
                    let C::Cubic { first, second, end } = commands[index] else {
                        panic!("missing circular boundary")
                    };
                    let angle = a + (b - a) * (segment + 1) as f64 / count as f64;
                    close(end.x, center[0] + r * angle.cos());
                    close(end.y, center[1] + r * angle.sin());
                    for t in [0.25, 0.5, 0.75] {
                        let sample = cubic(start, xy(first), xy(second), xy(end), t);
                        assert!(
                            ((sample[0] - center[0]).hypot(sample[1] - center[1]) - r).abs()
                                < r * 0.000273 + 1e-9
                        );
                    }
                    start = xy(end);
                    index += 1;
                }
            }
            "Q" => {
                let control = [center[0] + number(parts[1]), center[1] + number(parts[2])];
                let expected = [center[0] + number(parts[3]), center[1] + number(parts[4])];
                let C::Cubic { first, second, end } = commands[index] else {
                    panic!("missing quadratic ribbon side")
                };
                close(end.x, expected[0]);
                close(end.y, expected[1]);
                for t in [0.25, 0.5, 0.75] {
                    let sample = cubic(start, xy(first), xy(second), xy(end), t);
                    for axis in 0..2 {
                        let actual = (1. - t).powi(2) * start[axis]
                            + 2. * (1. - t) * t * control[axis]
                            + t * t * expected[axis];
                        close(sample[axis], actual);
                    }
                }
                start = expected;
                index += 1;
            }
            "Z" => {
                assert!(matches!(commands[index], C::Close));
                index += 1;
            }
            other => panic!("unexpected oracle command {other}"),
        }
    }
    assert_eq!(index, commands.len());
}

#[test]
fn original_nivo_and_d3_oracle_matches_groups_directional_endpoints_and_paths() {
    let mut layout = None;
    let mut counts = [0; 4];
    for line in include_str!("fixtures/chord-nivo.csv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields = line.split('|').collect::<Vec<_>>();
        match fields[1] {
            "case" => {
                let matrix = profile(fields[0]);
                let ids = (0..matrix.len())
                    .map(|i| format!("g{i}"))
                    .collect::<Vec<_>>();
                layout = Some(
                    chord_layout(
                        &config(),
                        &ids,
                        &matrix,
                        &ChordLayoutOptions {
                            pad_angle: number(fields[2]),
                            inner_radius_ratio: number(fields[3]),
                            inner_radius_offset: number(fields[4]),
                            ..options()
                        },
                    )
                    .unwrap(),
                );
                let current = layout.as_ref().unwrap();
                assert_eq!(current.center, [123., 107.]);
                close(current.radius, number(fields[6]));
                close(current.inner_radius, number(fields[7]));
                counts[0] += 1;
            }
            "arc" => {
                let current = layout.as_ref().unwrap();
                let index = fields[2].parse::<usize>().unwrap();
                let group = &current.groups[index];
                assert_eq!(group.id, fields[3]);
                close(group.value, number(fields[4]));
                close(group.angles[0], number(fields[5]));
                close(group.angles[1], number(fields[6]));
                assert_eq!(group.formatted_value, fields[7]);
                let expected = match index {
                    0 => Color(232, 193, 160, 255),
                    1 => Color(244, 117, 96, 255),
                    2 => Color(241, 225, 91, 255),
                    _ => unreachable!(),
                };
                assert_eq!(group.color, expected);
                counts[1] += 1;
            }
            "arc-path" => {
                let current = layout.as_ref().unwrap();
                let group = &current.groups[fields[2].parse::<usize>().unwrap()];
                let Geometry::Path(commands) =
                    &current.scene.marks[group.mark_index.unwrap()].geometry
                else {
                    panic!("missing rim")
                };
                let parts = fields[3]
                    .split('~')
                    .next()
                    .unwrap()
                    .split(',')
                    .collect::<Vec<_>>();
                let C::Move(p) = commands[0] else {
                    panic!("missing rim start")
                };
                close(p.x, current.center[0] + number(parts[1]));
                close(p.y, current.center[1] + number(parts[2]));
            }
            "ribbon" => {
                let current = layout.as_ref().unwrap();
                let source = fields[2].parse::<usize>().unwrap();
                let target = fields[3].parse::<usize>().unwrap();
                let ribbon = current
                    .ribbon(&chord_relation_edge_id(
                        &format!("g{source}"),
                        &format!("g{target}"),
                    ))
                    .unwrap();
                assert_eq!((ribbon.source.index, ribbon.target.index), (source, target));
                close(ribbon.source.value, number(fields[4]));
                close(ribbon.target.value, number(fields[5]));
                close(ribbon.source.angles[0], number(fields[6]));
                close(ribbon.source.angles[1], number(fields[7]));
                close(ribbon.target.angles[0], number(fields[8]));
                close(ribbon.target.angles[1], number(fields[9]));
                assert_eq!(ribbon.source, current.subgroups[source][target]);
                assert_eq!(ribbon.target, current.subgroups[target][source]);
                let mark = &current.scene.marks[ribbon.mark_index.unwrap()];
                assert_eq!(mark.fill, Some(ribbon.source.color));
                assert_eq!(mark.stroke.unwrap().width, 1.);
                counts[2] += 1;
            }
            "ribbon-path" => {
                let current = layout.as_ref().unwrap();
                let source = fields[2].parse::<usize>().unwrap();
                let target = fields[3].parse::<usize>().unwrap();
                let ribbon = current
                    .ribbon(&chord_relation_edge_id(
                        &format!("g{source}"),
                        &format!("g{target}"),
                    ))
                    .unwrap();
                let Geometry::Path(commands) =
                    &current.scene.marks[ribbon.mark_index.unwrap()].geometry
                else {
                    panic!("missing ribbon")
                };
                verify_ribbon_path(commands, fields[4], current.center);
                counts[3] += 1;
            }
            other => panic!("unknown oracle line {other}"),
        }
    }
    assert_eq!(counts, [5, 11, 9, 9]);
}

#[test]
fn directional_metadata_and_tooltips_survive_dominance_flip_and_scalar_aggregation() {
    let c = config();
    let ids = vec!["A".into(), "B".into()];
    let a = chord_layout(
        &c,
        &ids,
        &[vec![0., 4.], vec![9., 0.]],
        &ChordLayoutOptions {
            labels: None,
            value_format: ValueFormat::Fixed(2),
            ..Default::default()
        },
    )
    .unwrap();
    let b = chord_layout(
        &c,
        &ids,
        &[vec![0., 12.], vec![9., 0.]],
        &ChordLayoutOptions {
            labels: None,
            value_format: ValueFormat::Fixed(2),
            ..Default::default()
        },
    )
    .unwrap();
    let first = &a.ribbons[0];
    let second = &b.ribbons[0];
    assert_eq!(first.mark_id, second.mark_id);
    assert_eq!(first.id, second.id);
    assert_eq!(
        (
            first.source.id.as_str(),
            first.source.value,
            first.target.id.as_str(),
            first.target.value
        ),
        ("B", 9., "A", 4.)
    );
    assert_eq!(
        (
            second.source.id.as_str(),
            second.source.value,
            second.target.id.as_str(),
            second.target.value
        ),
        ("A", 12., "B", 9.)
    );
    assert_eq!((first.value, second.value), (13., 21.));
    assert_eq!(
        a.tooltip_text(&first.mark_id, &TooltipOptions::default())
            .unwrap(),
        "B: 9.00\nA: 4.00"
    );
    assert_eq!(
        b.tooltip_text(&second.mark_id, &TooltipOptions::default())
            .unwrap(),
        "A: 12.00\nB: 9.00"
    );
    assert_eq!(
        a.tooltip_text(
            &first.mark_id,
            &TooltipOptions {
                value_format: ValueFormat::Fixed(1),
                ..Default::default()
            }
        )
        .unwrap(),
        "B: 9.0\nA: 4.0"
    );
    assert_eq!(
        a.tooltip_text(&a.groups[1].mark_id, &TooltipOptions::default())
            .unwrap(),
        "B: 9.00"
    );
    let self_layout = chord_layout(
        &c,
        &["self".into()],
        &[vec![7.]],
        &ChordLayoutOptions {
            labels: None,
            ..Default::default()
        },
    )
    .unwrap();
    let ribbon = &self_layout.ribbons[0];
    assert_eq!(ribbon.value, 7.);
    assert_eq!(
        ribbon.tooltip_rows().map(|row| row.value),
        [Some(7.), Some(7.)]
    );
    let Geometry::Path(commands) = &self_layout.scene.marks[ribbon.mark_index.unwrap()].geometry
    else {
        panic!("self path missing")
    };
    assert_eq!(
        commands.len(),
        7,
        "full self arc is followed by one closure curve"
    );
}

#[test]
fn typed_binding_uses_real_rim_ribbon_ink_and_keeps_raw_endpoint_ids() {
    let ids = vec!["A".into(), "B".into()];
    let c = config();
    for matrix in [
        vec![vec![0., 4.], vec![9., 0.]],
        vec![vec![0., 12.], vec![9., 0.]],
    ] {
        let layout = chord_layout(
            &c,
            &ids,
            &matrix,
            &ChordLayoutOptions {
                pad_angle: 0.08,
                inner_radius_offset: 0.12,
                labels: None,
                ribbon_style: ChordPaintOptions {
                    opacity: 1.,
                    border_width: 0.,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let ribbon = &layout.ribbons[0];
        assert_eq!(
            layout
                .scene
                .hit_test(ribbon.anchor[0], ribbon.anchor[1])
                .unwrap()
                .id,
            ribbon.mark_id
        );
        let group = &layout.groups[0];
        assert_eq!(
            layout
                .scene
                .hit_test(group.anchor[0], group.anchor[1])
                .unwrap()
                .id,
            group.mark_id
        );
        let candidate = layout.clone().into_scene_interaction().unwrap();
        for picking in [Picking::Geometry, Picking::Nearest { distance: 1. }] {
            let mut surface = SurfaceModel::new(
                c.scene().unwrap(),
                SurfaceOptions {
                    picking,
                    ..Default::default()
                },
            )
            .unwrap();
            surface.set_layout(candidate.clone()).unwrap();
            for (point, id) in [
                (ribbon.anchor, &ribbon.mark_id),
                (group.anchor, &group.mark_id),
            ] {
                surface
                    .apply(SurfaceInput::Move {
                        id: 1,
                        point: (point[0], point[1]),
                    })
                    .unwrap();
                assert_eq!(surface.hovered(), std::slice::from_ref(id));
            }
        }
        let site = candidate
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|site| site.mark_id == ribbon.mark_id)
            .unwrap();
        assert_eq!(site.value, Some(ribbon.value));
        assert!(site.label.contains('A') && site.label.contains('B'));
        assert_eq!(site.series, ribbon.source.id);
        #[cfg(feature = "raster")]
        {
            use render::raster::{FontRegistry, render_raster};
            let raster =
                render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
            let offset =
                (ribbon.anchor[1] as usize * raster.width as usize + ribbon.anchor[0] as usize) * 4;
            let Color(r, g, b, a) = ribbon.source.color;
            assert_eq!(&raster.rgba[offset..offset + 4], &[r, g, b, a]);
        }
    }
}

#[test]
fn relation_and_hidden_layout_use_both_endpoints_while_preserving_domains() {
    let ids = vec!["g0".into(), "g1".into(), "g2".into()];
    let c = config();
    let matrix = profile("asymmetric");
    let original = chord_layout(&c, &ids, &matrix, &options()).unwrap();
    let hidden = chord_layout(
        &c,
        &ids,
        &matrix,
        &ChordLayoutOptions {
            hidden: ["g0".to_owned()].into(),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(original.color_scale, hidden.color_scale);
    for (a, b) in original.groups.iter().zip(&hidden.groups) {
        assert_eq!(a.angles, b.angles);
    }
    assert!(hidden.group("g0").unwrap().mark_index.is_none());
    assert!(
        hidden
            .ribbons
            .iter()
            .filter(|r| r.source.id == "g0" || r.target.id == "g0")
            .all(|r| r.mark_index.is_none())
    );
    assert!(
        hidden
            .ribbon(&chord_relation_edge_id("g1", "g2"))
            .unwrap()
            .mark_index
            .is_some()
    );
    let mut related = original.relation_layout(&RelationOptions::chord()).unwrap();
    let pair = chord_relation_edge_id("g0", "g1");
    related
        .set_focus(Some(RelationFocus::Edge(pair.clone())))
        .unwrap();
    assert_eq!(
        related.selection().nodes,
        ["g0".to_owned(), "g1".to_owned()].into()
    );
    related.set_hidden(["g0".to_owned()].into()).unwrap();
    assert!(related.scene().marks.iter().all(|mark| {
        original
            .ribbon_for_mark(&mark.id)
            .is_none_or(|r| r.source.id != "g0" && r.target.id != "g0")
    }));
    related.set_hidden(Default::default()).unwrap();
    assert_eq!(related.scene().marks.len(), original.scene.marks.len());
}

#[test]
fn labels_radii_styles_and_palette_configuration_reach_shared_output() {
    let c = config();
    let ids = vec!["g0".into(), "g1".into()];
    let matrix = vec![vec![0., 4.], vec![9., 0.]];
    let o = ChordLayoutOptions {
        inner_radius_ratio: 0.8,
        inner_radius_offset: 0.2,
        labels: Some(ChordLabelOptions {
            rotation: 25.,
            offset: 16.,
            ..Default::default()
        }),
        legend: true,
        colors_by_id: [("g1".into(), Color(10, 100, 200, 255))].into(),
        ..options()
    };
    let layout = chord_layout(&c, &ids, &matrix, &o).unwrap();
    close(layout.inner_radius, 72.);
    close(layout.ribbon_radius, 54.);
    assert_eq!(layout.scene.legend.len(), 2);
    for group in &layout.groups {
        let label = &layout.scene.labels[group.label_index.unwrap()];
        let angle = (group.angles[0] + group.angles[1]) / 2.;
        close(label.x, layout.center[0] + 106. * angle.sin());
        close(
            label.y,
            layout.center[1] - 106. * angle.cos() + c.theme.font_size * 0.35,
        );
        assert_eq!(
            layout.scene.label_series[&group.label_index.unwrap()],
            group.id
        );
        assert_eq!(
            label.anchor,
            if angle.to_degrees() > 180. {
                TextAnchor::Start
            } else {
                TextAnchor::End
            }
        );
    }
    let source = &layout.ribbons[0].source;
    assert_eq!(source.color, Color(10, 100, 200, 255));
    let mark = &layout.scene.marks[layout.ribbons[0].mark_index.unwrap()];
    assert_eq!(mark.fill, Some(source.color));
    assert_eq!(mark.opacity, 0.5);
    assert!(render::to_svg(&layout.scene).unwrap().contains("rotate("));
    assert!(
        render::to_canvas_script(&layout.scene, "chord")
            .unwrap()
            .contains("rotate(")
    );
    let horizontal = chord_layout(
        &c,
        &ids,
        &matrix,
        &ChordLayoutOptions {
            labels: Some(ChordLabelOptions {
                orientation: ChordLabelOrientation::Horizontal,
                ..Default::default()
            }),
            ..o
        },
    )
    .unwrap();
    assert!(
        horizontal
            .scene
            .labels
            .iter()
            .all(|label| label.rotation == 0.)
    );
    let app = glory::App::memory();
    app.mount(chart(horizontal.scene)).unwrap();
}

#[test]
fn ids_are_collision_safe_and_invalid_or_extreme_inputs_are_explicit() {
    let c = config();
    let o = ChordLayoutOptions {
        labels: None,
        ..Default::default()
    };
    let ids = vec!["a.b".into(), "c".into(), "a".into(), "b.c".into()];
    let matrix = vec![
        vec![0., 1., 0., 0.],
        vec![2., 0., 0., 0.],
        vec![0., 0., 0., 3.],
        vec![0., 0., 4., 0.],
    ];
    let layout = chord_layout(&c, &ids, &matrix, &o).unwrap();
    assert_eq!(layout.ribbons.len(), 2);
    assert_ne!(layout.ribbons[0].mark_id, layout.ribbons[1].mark_id);
    let empty = chord_layout(&c, &[], &[], &o).unwrap();
    assert!(empty.scene.marks.is_empty());
    let zero = chord_layout(&c, &ids, &vec![vec![0.; 4]; 4], &o).unwrap();
    assert!(zero.scene.marks.is_empty());
    assert!(
        zero.groups
            .iter()
            .enumerate()
            .all(|(i, g)| g.angles == [TAU * i as f64 / 4.; 2])
    );
    assert!(zero.interaction_binding().unwrap().sites.is_empty());
    let tiny = chord_layout(&c, &["tiny".into()], &[vec![f64::from_bits(1)]], &o).unwrap();
    assert_eq!(tiny.groups[0].angles, [0., TAU]);
    for (keys, data) in [
        (vec!["x".into()], vec![vec![-1.]]),
        (vec!["x".into()], vec![vec![f64::NAN]]),
        (vec!["x".into(), "x".into()], vec![vec![1.; 2]; 2]),
        (vec!["x".into()], vec![]),
    ] {
        assert!(chord_layout(&c, &keys, &data, &o).is_err());
    }
    assert!(
        chord_layout(
            &c,
            &ids,
            &matrix,
            &ChordLayoutOptions {
                inner_radius_offset: 1.,
                ..o.clone()
            }
        )
        .is_err()
    );
    assert!(
        chord_layout(
            &c,
            &ids,
            &matrix,
            &ChordLayoutOptions {
                pad_angle: TAU / 4.,
                ..o.clone()
            }
        )
        .is_err()
    );
    assert!(
        chord_layout(
            &c,
            &["x".into(), "y".into()],
            &vec![vec![f64::MAX; 2]; 2],
            &o
        )
        .is_err()
    );
    assert!(
        chord_layout(
            &c,
            &["x".repeat(512), "y".repeat(512)],
            &[vec![0., 1.], vec![1., 0.]],
            &o
        )
        .is_err()
    );
    let inset = chord_layout(
        &c,
        &ids,
        &matrix,
        &ChordLayoutOptions {
            inner_radius_ratio: 0.,
            inner_radius_offset: 0.,
            ..o
        },
    )
    .unwrap();
    assert!(
        inset
            .ribbons
            .iter()
            .all(|ribbon| ribbon.mark_index.is_none())
    );
}
