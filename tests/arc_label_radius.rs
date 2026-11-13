use std::f64::consts::{PI, TAU};

use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 200.,
        height: 200.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<RadialDatum> {
    vec![
        RadialDatum::new("small", 1.),
        RadialDatum::new("large", 3.),
        RadialDatum::new("zero", 0.),
    ]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn pie_options() -> PieLayoutOptions {
    let mut o = PieLayoutOptions {
        fit: PieFit::Centered,
        ..Default::default()
    };
    o.pie.inner_radius = 0.9;
    o.pie.style.arc_labels.legacy_fit = false;
    o
}

#[test]
fn real_nivo_filter_oracle_includes_equal_boundary_and_absolute_thickness() {
    let mut count = 0;
    for line in include_str!("fixtures/arc-label-radius-nivo.txt")
        .lines()
        .skip(2)
    {
        let f = line
            .split('|')
            .map(|s| s.parse::<f64>().unwrap())
            .collect::<Vec<_>>();
        let radius = ArcLabelRadiusMeasure::RingThickness.measure(f[0], f[1]);
        let admitted = f[2].abs() >= f[3] && radius >= f[4];
        assert_eq!(admitted, f[5] == 1., "{line}");
        count += 1;
    }
    assert_eq!(count, 12);
    assert_eq!(
        ArcLabelRadiusMeasure::default(),
        ArcLabelRadiusMeasure::OuterRadius
    );
    assert_eq!(
        PieArcLabels::default().radius_measure,
        ArcLabelRadiusMeasure::OuterRadius
    );
    assert_eq!(
        RadialBarLabels::default().radius_measure,
        ArcLabelRadiusMeasure::OuterRadius
    );
}

#[test]
fn pie_gate_uses_actual_radii_but_does_not_move_centers_or_reindex_labels() {
    let c = config();
    for inner in [0., 0.9] {
        for padding in [
            PiePadding::LegacyAngularGap,
            PiePadding::default(),
            PiePadding::D3Applied {
                pad_radius: PiePadRadius::Auto,
            },
        ] {
            for measure in [
                ArcLabelRadiusMeasure::OuterRadius,
                ArcLabelRadiusMeasure::RingThickness,
            ] {
                for skip in [0., 10., 11., 100., 101.] {
                    for reversed in [false, true] {
                        let mut o = pie_options();
                        o.padding = padding;
                        o.pie.inner_radius = inner;
                        o.pie.start_angle = 0.2;
                        o.pie.end_angle = if reversed { 0.2 - TAU } else { 0.2 + TAU };
                        o.pie.pad_angle = 0.06;
                        o.pie.sort_by_value = true;
                        o.pie.style.corner_radius = 8.;
                        o.pie.style.active_ids.insert("small".into());
                        o.pie.style.active_inner_radius_offset = 2.;
                        o.pie.style.active_outer_radius_offset = 3.;
                        let a = &mut o.pie.style.arc_labels;
                        a.radius_measure = measure;
                        a.skip_radius = skip;
                        a.skip_angle = PI / 2.;
                        a.radius_offset = 0.25;
                        let layout = pie_layout(&c, &data(), &o).unwrap();
                        let expected = layout
                            .slices
                            .iter()
                            .filter(|s| {
                                s.mark_index.is_some()
                                    && (s.end_angle - s.start_angle).abs() >= PI / 2.
                                    && match measure {
                                        ArcLabelRadiusMeasure::OuterRadius => {
                                            s.outer_radius >= skip
                                        }
                                        ArcLabelRadiusMeasure::RingThickness => {
                                            (s.outer_radius - s.inner_radius).abs() >= skip
                                        }
                                    }
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(layout.scene.labels.len(), expected.len());
                        for (index, slice) in expected.into_iter().enumerate() {
                            assert_eq!(layout.scene.label_series.get(&index), Some(&slice.id));
                            let label = &layout.scene.labels[index];
                            assert_eq!(label.text, slice.id);
                            let r = slice.inner_radius
                                + (slice.outer_radius - slice.inner_radius) * 0.25;
                            let angle = (slice.start_angle + slice.end_angle) / 2.;
                            close(label.x, layout.center[0] + r * angle.sin());
                            close(label.y, layout.center[1] - r * angle.cos());
                        }
                        assert_eq!(
                            pie_with_layout_options(&c, &data(), &o).unwrap(),
                            layout.scene
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn pie_legacy_profile_defaults_and_opt_in_gate_remain_coherent_with_hidden_labels() {
    let c = config();
    let mut o = pie_options();
    o.padding = PiePadding::LegacyAngularGap;
    o.pie.style.arc_labels.skip_radius = 11.;
    o.pie.hidden.insert("large".into());
    let old = pie(&c, &data(), &o.pie).unwrap();
    let typed = pie_layout(&c, &data(), &o).unwrap();
    assert_eq!(old.labels, typed.scene.labels);
    assert_eq!(old.label_series, typed.scene.label_series);
    assert_eq!(old.labels.len(), 1);
    o.pie.style.arc_labels.radius_measure = ArcLabelRadiusMeasure::RingThickness;
    assert!(pie(&c, &data(), &o.pie).unwrap().labels.is_empty());
    let thin = pie_layout(&c, &data(), &o).unwrap();
    assert!(thin.scene.labels.is_empty());
    assert_eq!(thin.scene.marks, typed.scene.marks);
    assert_eq!(thin.slices, typed.slices);
    o.pie.style.arc_labels.skip_radius = 10.;
    assert_eq!(pie_layout(&c, &data(), &o).unwrap().scene.labels.len(), 1);
    o.pie.style.arc_labels.enabled = false;
    assert!(pie_layout(&c, &data(), &o).unwrap().scene.labels.is_empty());
}

#[test]
fn pie_link_placement_indices_follow_only_the_admitted_arc_labels() {
    for measure in [
        ArcLabelRadiusMeasure::OuterRadius,
        ArcLabelRadiusMeasure::RingThickness,
    ] {
        let mut o = pie_options();
        o.pie.style.arc_labels.radius_measure = measure;
        o.pie.style.arc_labels.skip_radius = 11.;
        o.pie.style.arc_labels.text = PieLabelText::Value(ValueFormat::Fixed(0));
        o.pie.style.link_labels.enabled = true;
        let layout = pie_layout(&config(), &data(), &o).unwrap();
        assert_eq!(layout.link_candidates.len(), 2);
        let arc_count = if measure == ArcLabelRadiusMeasure::OuterRadius {
            2
        } else {
            0
        };
        let measurements = layout
            .link_candidates
            .iter()
            .map(|c| {
                c.measured(LayoutRect {
                    x: 0.,
                    y: -4.,
                    width: 12.,
                    height: 8.,
                })
            })
            .collect::<Vec<_>>();
        let placed = layout
            .place_link_labels(
                &measurements,
                &PieLinkPlacementOptions {
                    collision: LabelCollisionOptions {
                        viewport: LayoutRect {
                            x: -1000.,
                            y: -1000.,
                            width: 2000.,
                            height: 2000.,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(placed.scene.labels.len(), arc_count + 2);
        for (i, p) in placed.link_placements.iter().enumerate() {
            let candidate = &placed.link_candidates[i];
            let index = p.label_index.unwrap();
            assert_eq!(index, arc_count + i);
            assert_eq!(placed.scene.label_series[&index], candidate.series);
            assert_eq!(placed.scene.labels[index], candidate.label);
            assert_eq!(
                placed.scene.marks[p.leader_index.unwrap()].series,
                candidate.series
            );
        }
        assert_eq!(
            placed
                .place_link_labels(
                    &measurements,
                    &PieLinkPlacementOptions {
                        collision: LabelCollisionOptions {
                            viewport: LayoutRect {
                                x: -1000.,
                                y: -1000.,
                                width: 2000.,
                                height: 2000.
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                )
                .unwrap(),
            placed
        );
    }
}

fn radial_options() -> StackedRadialBarOptions {
    let mut o = StackedRadialBarOptions {
        inner_radius: 0.8,
        padding: 0.,
        max_value: Some(4.),
        labels: Some(RadialBarLabels {
            text: RadialBarLabelText::Category,
            radius_offset: 0.25,
            ..Default::default()
        }),
        ..Default::default()
    };
    o.axes.radial_start = None;
    o.axes.circular_outer = None;
    o
}

#[test]
fn radial_real_conveniences_share_nivo_gate_without_track_or_null_labels() {
    let c = config();
    let categories = vec![
        "small".into(),
        "large".into(),
        "zero".into(),
        "missing".into(),
    ];
    let series = vec![
        BarSeries::new("inside", vec![Some(1.), Some(3.), Some(0.), None]),
        BarSeries::new("outside", vec![Some(1.), Some(3.), Some(0.), None]),
    ];
    for tracks in [false, true] {
        for measure in [
            ArcLabelRadiusMeasure::OuterRadius,
            ArcLabelRadiusMeasure::RingThickness,
        ] {
            for skip in [0., 10., 11., 90., 100., 101.] {
                for skip_angle in [0., TAU * 0.75 / 4., PI] {
                    for hidden in [false, true] {
                        let mut o = radial_options();
                        o.tracks = tracks;
                        o.pad_angle = 0.04;
                        if hidden {
                            o.hidden.insert("large".into());
                        }
                        let labels = o.labels.as_mut().unwrap();
                        labels.radius_measure = measure;
                        labels.skip_radius = skip;
                        labels.skip_angle = skip_angle;
                        for style in [
                            None,
                            Some(StackedRadialArcStyle {
                                corner_radius: 4.,
                                ..Default::default()
                            }),
                        ] {
                            let l = match style {
                                None => {
                                    layout_stacked_radial_bar(&c, &categories, &series, &o).unwrap()
                                }
                                Some(s) => layout_stacked_radial_bar_with_arc_style(
                                    &c,
                                    &categories,
                                    &series,
                                    &o,
                                    &s,
                                )
                                .unwrap(),
                            };
                            let expected = l
                                .segments
                                .iter()
                                .filter(|s| {
                                    s.visible
                                        && (s.angles[1] - s.angles[0]).abs() >= skip_angle
                                        && match measure {
                                            ArcLabelRadiusMeasure::OuterRadius => {
                                                s.outer_radius >= skip
                                            }
                                            ArcLabelRadiusMeasure::RingThickness => {
                                                (s.outer_radius - s.inner_radius).abs() >= skip
                                            }
                                        }
                                })
                                .collect::<Vec<_>>();
                            assert_eq!(l.scene.labels.len(), expected.len());
                            for (index, s) in expected.into_iter().enumerate() {
                                assert_eq!(l.scene.label_series.get(&index), Some(&s.category));
                                let label = &l.scene.labels[index];
                                assert_eq!(label.text, s.category);
                                let position = l.frame.point(
                                    s.inner_radius + (s.outer_radius - s.inner_radius) * 0.25,
                                    (s.angles[0] + s.angles[1]) / 2.,
                                );
                                close(label.x, position[0]);
                                close(label.y, position[1]);
                            }
                            let scene = match style {
                                None => stacked_radial_bar(&c, &categories, &series, &o).unwrap(),
                                Some(s) => stacked_radial_bar_with_arc_style(
                                    &c,
                                    &categories,
                                    &series,
                                    &o,
                                    &s,
                                )
                                .unwrap(),
                            };
                            assert_eq!(scene, l.scene);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn collapsed_pad_and_zero_width_inputs_keep_existing_geometry_admission() {
    let c = config();
    let mut o = pie_options();
    o.pie.end_angle = 0.1;
    o.pie.pad_angle = 0.1;
    o.pie.style.arc_labels.radius_measure = ArcLabelRadiusMeasure::RingThickness;
    let l = pie_layout(&c, &[RadialDatum::new("one", 1.)], &o).unwrap();
    assert!(l.slices[0].outer_collapsed);
    assert!(l.scene.labels.is_empty());
    o.pie.inner_radius = 1.;
    assert!(pie_layout(&c, &data(), &o).is_err());
    let categories = vec!["one".into()];
    let series = vec![BarSeries::new("ring", vec![Some(1.)])];
    let mut r = radial_options();
    r.end_angle = 0.1;
    r.max_value = Some(1.);
    r.pad_angle = 0.1;
    r.labels.as_mut().unwrap().radius_measure = ArcLabelRadiusMeasure::RingThickness;
    r.border = Some(GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 1.,
        cap: StrokeCap::Butt,
    });
    let l =
        layout_stacked_radial_bar_with_arc_style(&c, &categories, &series, &r, &Default::default())
            .unwrap();
    assert!(l.segments[0].visible); // D3's collapsed radial border remains real ink.
    assert!(l.scene.labels.is_empty());
    r.inner_radius = 1.;
    assert!(layout_stacked_radial_bar(&c, &categories, &series, &r).is_err());
}
