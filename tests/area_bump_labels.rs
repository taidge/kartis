use std::collections::BTreeMap;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use kartis::surface::SurfaceModel;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 460.,
        height: 160.,
        margin: Margin {
            left: 120.,
            right: 120.,
            top: 25.,
            bottom: 25.,
        },
        ..Default::default()
    }
}

fn rows() -> Vec<AreaBumpSeries> {
    [("Giant", 90.), ("Small", 5.), ("Tiny", 5.)]
        .into_iter()
        .map(|(name, value)| {
            AreaBumpSeries::new(
                name,
                vec![
                    AreaBumpDatum::new("end", value),
                    AreaBumpDatum::new("start", value),
                ],
            )
        })
        .collect()
}

fn source_options() -> AreaBumpOptions {
    let mut options = AreaBumpOptions {
        x_order: Some(vec!["start".into(), "end".into()]),
        tie_break: AreaBumpTieBreak::SeriesId,
        axis_top: None,
        axis_bottom: None,
        grid: false,
        ..Default::default()
    };
    options.start_labels.text = AreaBumpLabelText::Endpoint(ValueFormat::Fixed(1));
    options.start_labels.rotation = -20.;
    options.start_labels.offset = [-3., 2.];
    options.end_labels.offset = [3., -2.];
    options
}

fn measure(layout: &AreaBumpLayout) -> Vec<AreaBumpMeasuredLabel> {
    layout
        .endpoint_label_candidates()
        .iter()
        .map(|c| {
            // These known rectangles include horizontal bearings and an
            // asymmetric vertical baseline, unlike a width-only estimate.
            c.measured(LayoutRect {
                x: if c.side == AreaBumpLabelSide::Start {
                    -46.
                } else {
                    2.
                },
                y: -10.,
                width: 44.,
                height: 18.,
            })
        })
        .collect()
}

fn placement(layout: &AreaBumpLayout) -> AreaBumpLabelPlacementOptions {
    let mut options = AreaBumpLabelPlacementOptions::for_scene(&layout.scene);
    options.leader = Some(GraphicStroke {
        color: Color(40, 50, 60, 255),
        width: 1.5,
        cap: StrokeCap::Butt,
    });
    options
}

#[test]
fn endpoint_candidates_use_terminal_cells_and_nivo_text_anchors() {
    let mut options = source_options();
    options.hidden.insert("Tiny".into());
    let raw = layout_area_bump(&config(), &rows(), &options).unwrap();
    assert!(raw.endpoint_label_placements().is_empty());
    assert_eq!(raw.endpoint_label_candidates().len(), 4);
    for candidate in raw.endpoint_label_candidates() {
        let series = raw
            .series
            .iter()
            .find(|s| s.id == candidate.series)
            .unwrap();
        let start = candidate.side == AreaBumpLabelSide::Start;
        let cell = &raw.cells[if start {
            series.cell_indices[0]
        } else {
            *series.cell_indices.last().unwrap()
        }];
        assert_eq!(candidate.endpoint, [cell.x_pixel, (cell.y0 + cell.y1) / 2.]);
        if start {
            assert_eq!(cell.source_index, 1);
            assert_eq!(candidate.label.anchor, TextAnchor::End);
            assert_eq!(candidate.label.rotation, -20.);
            assert_eq!(candidate.label.x, candidate.endpoint[0] - 15.);
            assert_eq!(candidate.label.y, candidate.endpoint[1] + 2.);
            assert_eq!(candidate.label.text, format!("{:.1}", cell.value));
        } else {
            assert_eq!(cell.source_index, 0);
            assert_eq!(candidate.label.anchor, TextAnchor::Start);
            assert_eq!(candidate.label.x, candidate.endpoint[0] + 15.);
            assert_eq!(candidate.label.y, candidate.endpoint[1] - 2.);
            assert_eq!(candidate.label.text, series.label);
        }
    }
    assert!(raw.scene.legend.iter().any(|item| item.id == "Tiny"));
}

#[test]
fn measured_text_reflows_vertically_with_obstacles_and_connectors() {
    let raw = layout_area_bump(&config(), &rows(), &source_options()).unwrap();
    let measurements = measure(&raw);
    let mut options = placement(&raw);
    let obstacle = LayoutRect {
        x: 354.,
        y: 113.,
        width: 90.,
        height: 16.,
    };
    options.collision.forbidden.push(obstacle);
    let placed = raw.place_endpoint_labels(&measurements, &options).unwrap();
    assert_eq!(placed.cells, raw.cells);
    assert_eq!(placed.slices, raw.slices);
    let mut previous = Vec::new();
    let mut moved = 0;
    let surface = SurfaceModel::new(placed.scene.clone(), Default::default()).unwrap();
    for (source, result) in raw
        .endpoint_label_candidates()
        .iter()
        .zip(placed.endpoint_label_placements())
    {
        assert_eq!(source.id, result.id);
        let rect = result.bounds.unwrap();
        assert!(options.collision.viewport.contains(rect));
        assert_eq!(rect.overlap(obstacle), 0.);
        assert!(previous.iter().all(|other| rect.overlap(*other) == 0.));
        previous.push(rect);
        let label_index = result.label_index.unwrap();
        let label = &placed.scene.labels[label_index];
        assert_eq!(placed.scene.label_series[&label_index], source.series);
        assert_eq!(label.x, source.label.x);
        assert_eq!(label.anchor, source.label.anchor);
        assert_eq!(label.rotation, source.label.rotation);
        assert_eq!(label.y, rect.y + 10.);
        moved += usize::from(label.y != source.label.y);
        let leader = &placed.scene.marks[result.leader_index.unwrap()];
        assert!(surface.datum(&leader.id).is_none());
        assert_eq!(leader.series, source.series);
        assert!(leader.label.is_empty());
        let Geometry::Path(commands) = &leader.geometry else {
            panic!("expected a path")
        };
        assert_eq!(
            commands[0],
            PathCommand::Move(GraphicPoint::new(source.endpoint[0], source.endpoint[1]))
        );
        let edge = match source.side {
            AreaBumpLabelSide::Start => {
                assert!(rect.x + rect.width <= source.endpoint[0]);
                rect.x + rect.width
            }
            AreaBumpLabelSide::End => {
                assert!(rect.x >= source.endpoint[0]);
                rect.x
            }
        };
        assert_eq!(
            commands[2],
            PathCommand::Line(GraphicPoint::new(edge, rect.y + 9.))
        );
    }
    assert!(moved >= 2);
    let svg = render::to_svg(&placed.scene).unwrap();
    assert!(svg.contains("area-bump-endpoint:5:Small:end:leader"));
    assert!(svg.contains("rotate(-20 "));

    let reverse: Vec<_> = rows().into_iter().rev().collect();
    let reordered = layout_area_bump(&config(), &reverse, &source_options()).unwrap();
    let reordered = reordered
        .place_endpoint_labels(&measure(&reordered), &options)
        .unwrap();
    let positions = |layout: &AreaBumpLayout| {
        layout
            .endpoint_label_placements()
            .iter()
            .map(|entry| (entry.id.clone(), entry.bounds))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(positions(&placed), positions(&reordered));
}

#[test]
fn rejected_measurements_and_collision_limits_preserve_the_layout() {
    let raw = layout_area_bump(&config(), &rows(), &source_options()).unwrap();
    let before = raw.clone();
    let measurements = measure(&raw);
    let options = placement(&raw);
    let mut duplicate = measurements.clone();
    duplicate[0] = duplicate[1].clone();
    assert!(raw.place_endpoint_labels(&duplicate, &options).is_err());
    assert!(
        raw.place_endpoint_labels(&measurements[1..], &options)
            .is_err()
    );

    let mut changed = config();
    changed.width += 5.;
    let relayout = layout_area_bump(&changed, &rows(), &source_options()).unwrap();
    assert!(
        relayout
            .place_endpoint_labels(&measurements, &options)
            .is_err()
    );
    let mut malformed = measurements.clone();
    malformed[0].bounds.height = f64::NAN;
    assert!(raw.place_endpoint_labels(&malformed, &options).is_err());
    let mut candidate = raw.endpoint_label_candidates()[0].clone();
    candidate.label.text.push('x');
    malformed[0] = candidate.measured(measurements[0].bounds);
    assert!(raw.place_endpoint_labels(&malformed, &options).is_err());

    let mut budget = options.clone();
    budget.collision.max_checks = 1;
    budget.collision.forbidden = vec![
        LayoutRect {
            x: 2.,
            y: 2.,
            width: 1.,
            height: 1.
        };
        2
    ];
    assert!(
        raw.place_endpoint_labels(&measurements, &budget)
            .unwrap_err()
            .0
            .contains("work budget")
    );
    let mut edited = raw.clone();
    edited.scene.labels[0].text.push('x');
    assert!(
        edited
            .place_endpoint_labels(&measurements, &options)
            .is_err()
    );
    assert_eq!(raw, before);
}

#[test]
fn hidden_labels_can_return_without_losing_focus_or_unrelated_scene_layers() {
    let mut raw = layout_area_bump(&config(), &rows(), &source_options()).unwrap();
    let extra_mark = Mark::new(
        "overlay",
        Geometry::Circle {
            x: 210.,
            y: 22.,
            radius: 2.,
        },
        Color(10, 20, 30, 255),
    );
    let extra_label = Label {
        x: 216.,
        y: 24.,
        text: "annotation".into(),
        size: 12.,
        color: Color(20, 30, 40, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    };
    let mark_slot = raw.scene.marks.len();
    let label_slot = raw.scene.labels.len();
    raw.scene.marks.push(extra_mark.clone());
    raw.scene.labels.push(extra_label.clone());
    raw.scene
        .label_series
        .insert(label_slot, "annotation-owner".into());
    raw.scene.draw_order = raw.scene.draw_items();
    raw.scene
        .draw_order
        .retain(|item| *item != DrawItem::Label(label_slot) && *item != DrawItem::Mark(mark_slot));
    raw.scene.draw_order.splice(
        0..0,
        [DrawItem::Label(label_slot), DrawItem::Mark(mark_slot)],
    );
    let measurements = measure(&raw);
    let restore = placement(&raw);
    let mut blocked = restore.clone();
    blocked.collision.forbidden = vec![blocked.collision.viewport];
    let mut hidden = raw.place_endpoint_labels(&measurements, &blocked).unwrap();
    assert_eq!(hidden.scene.labels, std::slice::from_ref(&extra_label));
    assert!(
        hidden
            .endpoint_label_placements()
            .iter()
            .all(|p| p.bounds.is_none() && p.leader_index.is_none())
    );
    hidden.set_active(["Tiny".into()].into()).unwrap();
    let mut placed = hidden
        .place_endpoint_labels(&measurements, &restore)
        .unwrap();
    let items: Vec<_> = placed.scene.paint_items().collect();
    assert!(matches!(items[0], PaintItem::Label(l) if l == &extra_label));
    assert!(matches!(items[1], PaintItem::Mark(m) if m == &extra_mark));
    assert_eq!(placed.scene.label_series[&0], "annotation-owner");
    assert!(
        placed
            .endpoint_label_placements()
            .iter()
            .all(|p| p.bounds.is_some())
    );
    for mark in placed
        .scene
        .marks
        .iter()
        .filter(|mark| mark.id.ends_with(":leader"))
    {
        assert_eq!(mark.opacity, if mark.series == "Tiny" { 1. } else { 0.15 });
    }
    placed.set_active(Default::default()).unwrap();
    assert!(
        placed
            .scene
            .marks
            .iter()
            .filter(|mark| mark.id.ends_with(":leader"))
            .all(|mark| mark.opacity == 0.8)
    );
    assert_eq!(
        placed
            .place_endpoint_labels(&measurements, &restore)
            .unwrap(),
        placed
    );
    let visible = placed.scene.without_series(&["Small".into()].into());
    assert!(!visible.marks.iter().any(|mark| mark.series == "Small"));
    assert!(
        !visible
            .label_series
            .values()
            .any(|series| series == "Small")
    );
}

#[cfg(feature = "raster")]
#[test]
fn cpu_rotated_outline_measurements_match_the_reflowed_label_ink() {
    use kartis::render::raster::{FontRegistry, MissingGlyphPolicy, render_raster};

    let mut config = config();
    config.height = 250.;
    let mut options = source_options();
    options.start_labels.enabled = false;
    options.end_labels.rotation = 22.;
    options.end_labels.size = Some(24.);
    options.end_labels.padding = 23.;
    options.end_labels.color = Some(Color(240, 0, 180, 255));
    options.style.fill_opacity = 1.;
    let mut rows = rows();
    for (row, label) in rows.iter_mut().zip(["HIH", "HHH", "EEE"]) {
        row.label = label.into();
    }
    let raw = layout_area_bump(&config, &rows, &options).unwrap();
    let mut fonts = FontRegistry::new();
    fonts
        .register(
            std::sync::Arc::<[u8]>::from(include_bytes!("fonts/Lato-Regular.ttf").as_slice()),
            0,
        )
        .unwrap();
    let measurements = raw
        .endpoint_label_candidates()
        .iter()
        .map(|candidate| {
            let outline = fonts
                .outline_label(&candidate.label, MissingGlyphPolicy::Error)
                .unwrap();
            let points: Vec<_> = outline
                .glyphs
                .iter()
                .flatten()
                .filter_map(|command| match command {
                    PathCommand::Move(p) | PathCommand::Line(p) => Some(*p),
                    PathCommand::Close => None,
                    PathCommand::Cubic { .. } => {
                        panic!("the selected measurement glyphs must use straight outlines")
                    }
                })
                .collect();
            let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            candidate.measured(LayoutRect {
                x: min_x - candidate.label.x,
                y: min_y - candidate.label.y,
                width: max_x - min_x,
                height: max_y - min_y,
            })
        })
        .collect::<Vec<_>>();
    let mut placed = raw
        .place_endpoint_labels(&measurements, &placement(&raw))
        .unwrap();
    placed.scene.legend.clear();
    assert!(
        placed
            .endpoint_label_placements()
            .iter()
            .all(|p| p.bounds.is_some()),
        "measured boxes did not fit: {:?}",
        placed.endpoint_label_placements()
    );
    let bounds: Vec<_> = placed
        .endpoint_label_placements()
        .iter()
        .map(|p| p.bounds.unwrap())
        .collect();
    let moved = raw
        .endpoint_label_candidates()
        .iter()
        .zip(placed.endpoint_label_placements())
        .any(|(candidate, placed_label)| {
            placed.scene.labels[placed_label.label_index.unwrap()].y != candidate.label.y
        });
    assert!(moved);
    let frame = render_raster(&placed.scene, &fonts, &Default::default()).unwrap();
    let mut ink = vec![0; bounds.len()];
    for (i, p) in frame.rgba.as_chunks::<4>().0.iter().enumerate() {
        if p[0] < 215 || p[1] > 70 || p[2] < 140 {
            continue;
        }
        let x = (i % frame.width as usize) as f64 + 0.5;
        let y = (i / frame.width as usize) as f64 + 0.5;
        let index = bounds
            .iter()
            .position(|b| {
                x >= b.x - 0.5
                    && x <= b.x + b.width + 0.5
                    && y >= b.y - 0.5
                    && y <= b.y + b.height + 0.5
            })
            .expect("actual rotated glyph ink must lie in a placed measured box");
        ink[index] += 1;
    }
    assert!(ink.iter().all(|&count| count > 50), "{ink:?}");
}
