use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::stack::{StackOffset, StackOptions, StackOrder, stack};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

const LEFT: f64 = 11.;
const TOP: f64 = 13.;

fn config() -> ChartConfig {
    ChartConfig {
        width: 228.,
        height: 192.,
        margin: Margin {
            left: LEFT,
            right: 17.,
            top: TOP,
            bottom: 19.,
        },
        ..Default::default()
    }
}

fn axes_off(o: &mut StreamLayoutOptions) {
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
}

fn categories() -> Vec<CategoryKey> {
    (0..4).map(|i| CategoryKey::Number(f64::from(i))).collect()
}

fn data(signed: bool) -> Vec<BarSeries> {
    vec![
        BarSeries::new("A", vec![Some(10.), Some(20.), Some(30.), Some(16.)]),
        BarSeries::new(
            "B",
            vec![
                Some(16.),
                None,
                Some(if signed { -14. } else { 14. }),
                Some(22.),
            ],
        ),
        BarSeries::new(
            "C",
            vec![
                Some(3.),
                Some(5.),
                Some(8.),
                Some(if signed { -6. } else { 6. }),
            ],
        ),
    ]
}

fn order(source: &str) -> StackOrder {
    match source {
        "none" => StackOrder::None,
        "reverse" => StackOrder::Reverse,
        "ascending" => StackOrder::Ascending,
        "descending" => StackOrder::Descending,
        "insideOut" => StackOrder::InsideOut,
        _ => panic!("unknown order"),
    }
}

fn offset(source: &str) -> StackOffset {
    match source {
        "none" => StackOffset::Zero,
        "diverging" => StackOffset::Diverging,
        "expand" => StackOffset::Expand,
        "silhouette" => StackOffset::Silhouette,
        "wiggle" => StackOffset::Wiggle,
        _ => panic!("unknown offset"),
    }
}

fn curve(source: &str) -> Curve {
    match source {
        "linear" => Curve::Linear,
        "basis" => Curve::Basis,
        "cardinal" => Curve::Cardinal,
        "catmullRom" => Curve::CatmullRom { alpha: 0.5 },
        "monotoneX" => Curve::MonotoneX,
        "monotoneY" => Curve::MonotoneY,
        "natural" => Curve::Natural,
        "step" => Curve::Step,
        "stepBefore" => Curve::StepBefore,
        "stepAfter" => Curve::StepAfter,
        _ => panic!("unknown curve"),
    }
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

fn numbers(source: &str) -> impl Iterator<Item = f64> {
    source.split(',').map(|value| value.parse().unwrap())
}

fn commands(actual: &[C], expected: &str) {
    let expected = expected.split('|').collect::<Vec<_>>();
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        let (name, points) = match actual {
            C::Move(p) => ("M", vec![p]),
            C::Line(p) => ("L", vec![p]),
            C::Cubic { first, second, end } => ("C", vec![first, second, end]),
            C::Close => ("Z", vec![]),
        };
        let (kind, coords) = expected.split_once(',').unwrap_or((expected, ""));
        assert_eq!(name, kind);
        if !points.is_empty() {
            let mut coords = numbers(coords);
            for P { x, y } in points {
                near(*x, coords.next().unwrap() + LEFT);
                near(*y, coords.next().unwrap() + TOP);
            }
            assert!(coords.next().is_none());
        }
    }
}

#[test]
fn nivo_stream_scales_stacks_and_all_area_curves_match_independent_reference() {
    let mut layout = None;
    let mut cases = 0;
    for source in include_str!("fixtures/stream-reference.txt")
        .lines()
        .skip(1)
    {
        let fields = source.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["case", order_name, offset_name, curve_name, domain] => {
                let mut opts = StreamLayoutOptions {
                    stack: StackOptions {
                        order: order(order_name),
                        offset: offset(offset_name),
                    },
                    curve: curve(curve_name),
                    ..Default::default()
                };
                axes_off(&mut opts);
                let next = stream_layout(
                    &config(),
                    &categories(),
                    &data(matches!(*offset_name, "none" | "diverging")),
                    &opts,
                )
                .unwrap();
                for (actual, expected) in next
                    .y_scale
                    .display_domain()
                    .into_iter()
                    .zip(numbers(domain))
                {
                    near(actual, expected);
                }
                layout = Some(next);
                cases += 1;
            }
            ["layer", id, index, path] => {
                let layout: &StreamLayout = layout.as_ref().unwrap();
                let layer = layout.layers.iter().find(|layer| layer.id == *id).unwrap();
                assert_eq!(layer.stack_index, index.parse::<usize>().unwrap());
                let mark = layout
                    .scene
                    .marks
                    .iter()
                    .find(|mark| Some(&mark.id) == layer.area_mark_id.as_ref())
                    .unwrap();
                commands(&mark.geometry.commands(), path);
            }
            ["sample", id, index, value, coords] => {
                let layout: &StreamLayout = layout.as_ref().unwrap();
                let sample = &layout
                    .layers
                    .iter()
                    .find(|layer| layer.id == *id)
                    .unwrap()
                    .samples[index.parse::<usize>().unwrap()];
                assert_eq!(
                    sample.value,
                    (*value != "null").then(|| value.parse().unwrap())
                );
                let [x, _] = sample.position;
                for (actual, expected) in [
                    sample.y0,
                    sample.y1,
                    x - LEFT,
                    sample.screen_y[0] - TOP,
                    sample.screen_y[1] - TOP,
                ]
                .into_iter()
                .zip(numbers(coords))
                {
                    near(actual, expected);
                }
            }
            _ => panic!("unexpected fixture line {source}"),
        }
    }
    assert_eq!(cases, 15);
}

fn model(layout: StreamLayout, slices: bool, picking: Picking) -> SurfaceModel {
    let candidate = if slices {
        layout.into_slice_interaction()
    } else {
        layout.into_scene_interaction()
    }
    .unwrap();
    SurfaceModel::new_with_interaction(
        candidate.scene,
        SurfaceOptions {
            picking,
            plot: candidate.plot,
            ..Default::default()
        },
        candidate.interaction,
    )
    .unwrap()
}

fn hover(model: &mut SurfaceModel, point: [f64; 2]) -> Vec<String> {
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (point[0], point[1]),
        })
        .unwrap();
    model.hovered().to_vec()
}

#[test]
fn disabled_dots_offer_nullable_slice_sites_without_exact_proxy_ink() {
    let data = data(false);
    let data = &data[..2];
    let opts = StreamLayoutOptions {
        stack: Default::default(),
        curve: Curve::Linear,
        ..Default::default()
    };
    let layout = stream_layout(&config(), &categories(), data, &opts).unwrap();
    let sample = layout.layers[0].samples[1].clone();
    let missing = &layout.layers[1].samples[1];
    assert!(missing.value.is_none() && missing.formatted_value.is_none());
    assert_eq!(missing.y0, missing.y1);
    assert_eq!(layout.slice_interaction_binding().sites.len(), 8);
    assert_eq!(layout.interaction_binding().sites.len(), 2);
    let mut exact = model(layout.clone(), false, Picking::Geometry);
    assert_eq!(
        hover(&mut exact, sample.position),
        [layout.layers[0].area_mark_id.clone().unwrap()]
    );
    let mut samples_exact = model(layout.clone(), true, Picking::Geometry);
    assert!(hover(&mut samples_exact, sample.position).is_empty());
    let mut nearest = model(layout.clone(), true, Picking::Nearest { distance: 2. });
    assert_eq!(
        hover(&mut nearest, sample.position).as_slice(),
        std::slice::from_ref(&sample.mark_id)
    );
    let mut slice = model(layout.clone(), true, Picking::SliceX { distance: 0.1 });
    let picked = hover(&mut slice, [sample.position[0], TOP + 1.]);
    assert_eq!(picked.len(), 2);
    let active = slice.active_interaction(&picked).unwrap();
    assert!(
        active
            .rows
            .iter()
            .any(|row| row.series == "A" && row.value == Some(20.))
    );
    assert!(
        active
            .rows
            .iter()
            .any(|row| row.series == "B" && row.value.is_none())
    );
    slice.set_hidden(BTreeSet::from(["B".into()])).unwrap();
    assert_eq!(
        hover(&mut slice, [sample.position[0], TOP + 1.]),
        [sample.mark_id]
    );
    assert!(hover(&mut slice, [LEFT - 1., sample.position[1]]).is_empty());
}

#[test]
fn all_stack_orders_offsets_and_hidden_slots_use_the_shared_stack_result() {
    let data = data(false);
    for order in [
        StackOrder::None,
        StackOrder::Reverse,
        StackOrder::Ascending,
        StackOrder::Descending,
        StackOrder::InsideOut,
    ] {
        for offset in [
            StackOffset::Zero,
            StackOffset::Diverging,
            StackOffset::Expand,
            StackOffset::Silhouette,
            StackOffset::Wiggle,
        ] {
            let opts = StreamLayoutOptions {
                stack: StackOptions { order, offset },
                hidden: BTreeSet::from(["B".into()]),
                ..Default::default()
            };
            let layout = stream_layout(&config(), &categories(), &data, &opts).unwrap();
            assert_eq!(layout.stack, stack(&data, &opts.stack).unwrap());
            assert!(layout.layers[1].hidden && layout.layers[1].area_mark_id.is_none());
            assert_eq!(layout.layers[1].samples.len(), 4);
            assert!(layout.scene.marks.iter().all(|mark| mark.series != "B"));
            assert!(
                layout
                    .slice_interaction_binding()
                    .sites
                    .iter()
                    .all(|site| site.series != "B")
            );
            for reversed in [false, true] {
                let mut slice_options = opts.clone();
                slice_options.y_scale.reverse = reversed;
                let sampled =
                    stream_layout(&config(), &categories(), &data, &slice_options).unwrap();
                let mut surface = model(sampled.clone(), true, Picking::SliceX { distance: 0.1 });
                for slice in &sampled.slices {
                    let selected = hover(&mut surface, [slice.x, TOP + 1.]);
                    let active = surface.active_interaction(&selected).unwrap();
                    assert_eq!(
                        active
                            .rows
                            .iter()
                            .map(|row| row.series.as_str())
                            .collect::<Vec<_>>(),
                        slice
                            .layer_indices
                            .iter()
                            .map(|index| sampled.layers[*index].id.as_str())
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

#[test]
fn dot_positions_and_reversed_rounded_scales_keep_raw_boundary_values() {
    let categories = categories();
    let data = [BarSeries::new(
        "A",
        vec![Some(10.), Some(20.), Some(30.), Some(40.)],
    )];
    for position in [
        StreamDotPosition::Start,
        StreamDotPosition::Center,
        StreamDotPosition::End,
    ] {
        let mut opts = StreamLayoutOptions {
            stack: Default::default(),
            dots: Some(StreamDotOptions {
                position,
                size: 8.,
                ..Default::default()
            }),
            x_scale: PointScaleOptions {
                reverse: true,
                round: true,
                ..Default::default()
            },
            y_scale: ContinuousScaleOptions {
                reverse: true,
                round: true,
                clamp: true,
                ..Default::default()
            },
            y_bounds: [Some(0.), Some(25.)],
            ..Default::default()
        };
        axes_off(&mut opts);
        let layout = stream_layout(&config(), &categories, &data, &opts).unwrap();
        let sample = layout.layers[0].samples[2].clone();
        assert_eq!([sample.y0, sample.y1], [0., 30.]);
        assert_eq!(sample.value, Some(30.));
        let expected = match position {
            StreamDotPosition::Start => TOP,
            StreamDotPosition::Center => TOP + 80.,
            StreamDotPosition::End => TOP + 160.,
        };
        assert_eq!(sample.position, [78., expected]);
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|mark| mark.id == sample.mark_id)
            .unwrap();
        assert_eq!(
            mark.geometry,
            Geometry::Circle {
                x: 78.,
                y: expected,
                radius: 4.
            }
        );
        let mut exact = model(layout, false, Picking::Geometry);
        assert_eq!(hover(&mut exact, sample.position), [sample.mark_id]);
    }
}

#[test]
fn typed_keys_stable_ids_and_rejected_proposals_preserve_accepted_state() {
    let categories = [
        CategoryKey::Number(1.),
        CategoryKey::String("1".into()),
        CategoryKey::String("next".into()),
        CategoryKey::String("last".into()),
    ];
    let data = data(false);
    let opts = StreamLayoutOptions {
        colors: OrdinalColorOptions {
            domain: Some(vec!["A".into()]),
            unknown: OrdinalColorUnknown::Extend,
            ..Default::default()
        },
        styles: std::collections::BTreeMap::from([(
            "B".into(),
            StreamLayerStyle {
                color: Some(Color(0, 128, 0, 255)),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    let layout = stream_layout(&config(), &categories, &data, &opts).unwrap();
    assert_eq!(
        layout.colors.domain(),
        &["A".into(), "B".into(), "C".into()]
    );
    assert_eq!(layout.layers[1].color, Color(0, 128, 0, 255));
    assert_ne!(
        layout.layers[0].samples[0].mark_id,
        layout.layers[0].samples[1].mark_id
    );
    let mut surface = model(layout.clone(), true, Picking::SliceX { distance: 2. });
    let old = surface.scene().clone();
    let mut rejected = layout.into_slice_interaction().unwrap();
    rejected.scene.width = 0.;
    assert!(surface.set_layout(rejected).is_err());
    assert_eq!(surface.scene(), &old);
    let signed = self::data(true);
    assert!(
        stream_layout(
            &config(),
            &categories,
            &signed,
            &StreamLayoutOptions::default()
        )
        .is_err()
    );
    assert!(
        stream_layout(
            &config(),
            &[0., -0.].map(CategoryKey::Number),
            &[],
            &Default::default()
        )
        .is_err()
    );
    let invalid = StreamLayoutOptions {
        curve: Curve::BasisClosed,
        ..Default::default()
    };
    assert!(stream_layout(&config(), &categories, &data, &invalid).is_err());
    let empty = stream_layout(&config(), &[], &[], &Default::default()).unwrap();
    assert!(empty.layers.is_empty() && empty.slices.is_empty());
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_keep_fill_alpha_border_ink_and_clipping_independent() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let mut opts = StreamLayoutOptions {
        stack: Default::default(),
        curve: Curve::Linear,
        y_bounds: [Some(0.), Some(40.)],
        style: StreamLayerStyle {
            color: Some(Color(255, 0, 0, 255)),
            fill_opacity: 0.25,
            border_width: 4.,
            border_color: InheritedColor::Fixed(Color(0, 128, 0, 255)),
            ..Default::default()
        },
        ..Default::default()
    };
    axes_off(&mut opts);
    let data = [BarSeries::new("A", vec![Some(20.); 4])];
    let layout = stream_layout(&config(), &categories(), &data, &opts).unwrap();
    let image = render_raster(
        &layout.scene,
        &FontRegistry::new(),
        &RasterOptions::default(),
    )
    .unwrap();
    let pixel = |x: u32, y: u32| -> Vec<u8> {
        let start = ((y * image.width + x) * 4) as usize;
        image.rgba[start..start + 4].to_vec()
    };
    assert_eq!(pixel(111, 133), [255, 191, 191, 255]);
    assert_eq!(pixel(111, 93), [0, 128, 0, 255]);
    assert_eq!(pixel(9, 133), [255, 255, 255, 255]);
    let mark = layout
        .scene
        .marks
        .iter()
        .find(|mark| Some(&mark.id) == layout.layers[0].area_mark_id.as_ref())
        .unwrap();
    assert_eq!(mark.fill, Some(Color(255, 0, 0, 64)));
    assert_eq!(mark.stroke.unwrap().color, Color(0, 128, 0, 255));
}
