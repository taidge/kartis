use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;
use std::collections::BTreeSet;

const LEFT: f64 = 11.;
const TOP: f64 = 13.;

fn config() -> ChartConfig {
    ChartConfig {
        width: 228.,
        height: 212.,
        margin: Margin {
            left: LEFT,
            right: 17.,
            top: TOP,
            bottom: 19.,
        },
        ..Default::default()
    }
}

fn sep(length: f64, offset: f64) -> Option<FunnelSeparatorOptions> {
    Some(FunnelSeparatorOptions {
        length,
        offset,
        ..Default::default()
    })
}

fn source_layout(case: &str) -> FunnelLayout {
    let (values, options) = match case {
        "vertical-basis" => (
            vec![80., 40., 20.],
            FunnelLayoutOptions {
                spacing: 12.,
                before: sep(18., 4.),
                after: sep(28., 6.),
                current: Some("s1".into()),
                current_size_extension: 7.,
                current_border_width: Some(2.),
                ..Default::default()
            },
        ),
        "horizontal-linear" => (
            vec![3., 9., 1.],
            FunnelLayoutOptions {
                direction: FunnelDirection::Horizontal,
                interpolation: FunnelInterpolation::Linear,
                spacing: 10.,
                shape_blending: 0.5,
                before: sep(10., 3.),
                after: sep(14., 2.),
                size: FunnelSize::OrdinalPixels(vec![28., 70., 16.]),
                ..Default::default()
            },
        ),
        "zero-domain" => (
            vec![0., 0.],
            FunnelLayoutOptions {
                spacing: 4.,
                shape_blending: 0.3,
                ..Default::default()
            },
        ),
        _ => panic!("unknown fixture {case}"),
    };
    let data = values
        .into_iter()
        .enumerate()
        .map(|(i, value)| FunnelDatum::new(format!("s{i}"), value))
        .collect::<Vec<_>>();
    funnel_layout(&config(), &data, &options).unwrap()
}

fn numbers(source: &str) -> impl Iterator<Item = f64> {
    source.split(',').map(|number| number.parse().unwrap())
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

fn same_commands(actual: &[C], expected: &str) {
    let expected = expected.split('|').collect::<Vec<_>>();
    assert_eq!(actual.len(), expected.len());
    for (command, source) in actual.iter().zip(expected) {
        let (name, points) = match command {
            C::Move(p) => ("M", vec![p]),
            C::Line(p) => ("L", vec![p]),
            C::Cubic { first, second, end } => ("C", vec![first, second, end]),
            C::Close => ("Z", vec![]),
        };
        let (expected_name, coords) = source.split_once(',').unwrap_or((source, ""));
        assert_eq!(name, expected_name);
        if !points.is_empty() {
            let mut expected = numbers(coords);
            for P { x, y } in points {
                close(*x, expected.next().unwrap() + LEFT);
                close(*y, expected.next().unwrap() + TOP);
            }
            assert!(expected.next().is_none());
        }
    }
}

#[test]
fn matches_nivo_hook_geometry_and_d3_path_commands_for_three_profiles() {
    let mut layout = None;
    let mut separator_index = 0;
    for line in include_str!("fixtures/funnel-reference.txt")
        .lines()
        .skip(1)
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["part", id, bounds, area, border] => {
                let layout: &FunnelLayout = layout.as_ref().unwrap();
                let part = layout
                    .parts
                    .iter()
                    .find(|part| part.datum.id == (*id).into())
                    .unwrap();
                let [x, y] = part.center;
                let actual = [
                    part.bounds.x - LEFT,
                    part.bounds.y - TOP,
                    part.bounds.width,
                    part.bounds.height,
                    x - LEFT,
                    y - TOP,
                ];
                for (actual, expected) in actual.into_iter().zip(numbers(bounds)) {
                    close(actual, expected);
                }
                let marks = &layout.scene.marks;
                let area_mark = marks.iter().find(|mark| mark.id == part.mark_id).unwrap();
                let border_mark = marks
                    .iter()
                    .find(|mark| mark.series == part.series && mark.stroke.is_some())
                    .unwrap();
                same_commands(&area_mark.geometry.commands(), area);
                same_commands(&border_mark.geometry.commands(), border);
                assert_eq!(
                    border_mark.stroke.unwrap().width,
                    if part.is_current { 2. } else { 6. }
                );
                assert_eq!(border_mark.opacity, 0.66);
                assert!(border_mark.label.is_empty());
            }
            ["separator", side, id, endpoints] => {
                let separator = &layout.as_ref().unwrap().separators[separator_index];
                separator_index += 1;
                assert_eq!(
                    separator.side,
                    if *side == "0" {
                        FunnelSeparatorSide::Before
                    } else {
                        FunnelSeparatorSide::After
                    }
                );
                assert_eq!(separator.part_id, (*id != "none").then(|| (*id).into()));
                let [x0, y0] = separator.from;
                let [x1, y1] = separator.to;
                for (actual, expected) in [x0 - LEFT, y0 - TOP, x1 - LEFT, y1 - TOP]
                    .into_iter()
                    .zip(numbers(endpoints))
                {
                    close(actual, expected);
                }
            }
            [case, bandwidth, domain_max, range_max] => {
                let next = source_layout(case);
                close(next.bandwidth, bandwidth.parse().unwrap());
                close(next.value_domain[1], domain_max.parse().unwrap());
                close(next.size_range[1], range_max.parse().unwrap());
                layout = Some(next);
                separator_index = 0;
            }
            _ => panic!("unexpected fixture line: {line}"),
        }
    }
}

#[test]
fn typed_parts_keep_data_and_interaction_identity_through_hiding_and_reorder() {
    let mut data = [
        FunnelDatum::new(1., 0.8),
        FunnelDatum::new("1", 0.4),
        FunnelDatum::new("tail", 0.2),
    ];
    data[0].label = "numeric key".into();
    let options = FunnelLayoutOptions {
        direction: FunnelDirection::Horizontal,
        spacing: 10.,
        before: None,
        after: None,
        value_format: ValueFormat::Percent(0),
        colors: OrdinalColorOptions {
            domain: Some(data.iter().map(|d| d.id.clone()).collect()),
            ..Default::default()
        },
        ..Default::default()
    };
    let layout = funnel_layout(&config(), &data, &options).unwrap();
    let first = &layout.parts[0];
    assert_eq!(layout.scene.labels[0].text, "80%");
    assert_eq!(layout.part_for_mark(&first.mark_id).unwrap().datum, data[0]);
    assert_ne!(first.mark_id, layout.parts[1].mark_id);
    assert!(layout.separators.is_empty());
    let candidate = layout.clone().into_scene_interaction().unwrap();
    let mut surface = SurfaceModel::new_with_interaction(
        candidate.scene,
        SurfaceOptions {
            picking: Picking::Geometry,
            plot: candidate.plot,
            ..Default::default()
        },
        candidate.interaction,
    )
    .unwrap();
    let [x, y] = first.center;
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert_eq!(surface.hovered(), std::slice::from_ref(&first.mark_id));
    let active = surface.active_interaction(surface.hovered()).unwrap();
    assert_eq!(active.anchor, first.center);
    assert_eq!(active.rows[0].formatted_value.as_deref(), Some("80%"));
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (LEFT + layout.bandwidth + 5., y),
        })
        .unwrap();
    assert!(surface.hovered().is_empty());
    let hidden = BTreeSet::from([first.series.clone()]);
    surface.set_hidden(hidden.clone()).unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert!(surface.hovered().is_empty());
    let filtered = layout.scene.without_series(&hidden);
    assert!(filtered.marks.iter().all(|m| m.series != first.series));
    assert!(filtered.labels.iter().all(|label| label.text != "80%"));

    data.swap(0, 1);
    let next = funnel_layout(&config(), &data, &options).unwrap();
    assert_eq!(next.parts[1].mark_id, first.mark_id);
    assert_eq!(next.parts[1].color, first.color);
    surface
        .set_layout(next.into_scene_interaction().unwrap())
        .unwrap();
    assert_eq!(surface.hidden(), &hidden);
}

#[test]
fn pixel_sizes_expose_raw_data_and_keep_empty_parts_out_of_picking() {
    let mut first = FunnelDatum::new("a", 80.);
    first.size = Some(24.);
    let data = [first, FunnelDatum::new("b", 40.)];
    let options = FunnelLayoutOptions {
        size: FunnelSize::DatumPixels,
        before: None,
        after: None,
        border_width: 0.,
        ..Default::default()
    };
    let layout = funnel_layout(&config(), &data, &options).unwrap();
    assert_eq!(layout.parts[0].bounds.width, 24.);
    assert_eq!(layout.parts[1].bounds.width, 0.);
    assert_eq!(layout.parts[1].datum.value, 40.);
    assert_eq!(layout.interaction_binding().sites.len(), 1);
    assert_eq!(layout.scene.marks.len(), 1);
    let custom = funnel_layout_with_size(&config(), &data, &options, |c| {
        close(c.available_size, 200.);
        close(c.scaled_size, 2.5 * c.datum.value);
        Ok(c.scaled_size * 0.5 + c.source_index as f64)
    })
    .unwrap();
    assert_eq!(
        [custom.parts[0].bounds.width, custom.parts[1].bounds.width],
        [100., 51.]
    );
    assert_eq!(custom.interaction_binding().sites.len(), 2);
}

#[test]
fn rejects_invalid_ids_bounds_and_sizes_without_a_partial_layout() {
    let options = FunnelLayoutOptions::default();
    let duplicate = [FunnelDatum::new(0., 1.), FunnelDatum::new(-0., 2.)];
    assert!(funnel_layout(&config(), &duplicate, &options).is_err());
    let data = [FunnelDatum::new("first", 1.), FunnelDatum::new("last", 2.)];
    let wide_separator = FunnelLayoutOptions {
        before: sep(200., 0.),
        ..Default::default()
    };
    assert!(funnel_layout(&config(), &data, &wide_separator).is_err());
    let gap = FunnelLayoutOptions {
        spacing: 180.,
        ..Default::default()
    };
    assert!(funnel_layout(&config(), &data, &gap).is_err());
    let ordinal = FunnelLayoutOptions {
        size: FunnelSize::OrdinalPixels(vec![]),
        ..Default::default()
    };
    assert!(funnel_layout(&config(), &data, &ordinal).is_err());
    assert!(funnel_layout_with_size(&config(), &data, &options, |_| Ok(f64::NAN)).is_err());
    let empty = funnel_layout(&config(), &[], &options).unwrap();
    assert!(empty.scene.marks.is_empty() && empty.parts.is_empty() && empty.separators.is_empty());
}
