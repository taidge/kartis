use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::{
    stack::StackOptions,
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};
use std::collections::BTreeSet;

const INK: Color = Color(165, 30, 91, 255);
const BORDER: Color = Color(19, 91, 137, 255);

fn config() -> ChartConfig {
    ChartConfig {
        width: 228.,
        height: 192.,
        margin: Margin {
            left: 11.,
            right: 17.,
            top: 13.,
            bottom: 19.,
        },
        ..Default::default()
    }
}

fn categories() -> Vec<CategoryKey> {
    vec![1_f64.into(), "1".into(), "tail".into()]
}

fn options() -> StreamLayoutOptions {
    let mut options = StreamLayoutOptions {
        stack: StackOptions::default(),
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        y_bounds: [Some(0.), Some(40.)],
        y_scale: ContinuousScaleOptions::default(),
        style: StreamLayerStyle {
            fill_opacity: 0.,
            ..Default::default()
        },
        ..Default::default()
    };
    options.axes.x.visible = false;
    options.axes.x.grid = false;
    options.axes.y.visible = false;
    options.axes.y.grid = false;
    options
}

fn source(value: f64) -> Vec<BarSeries> {
    vec![BarSeries::new("A", vec![Some(10.), Some(value), Some(30.)])]
}

fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + width, y)),
        C::Line(P::new(x + width, y + height)),
        C::Line(P::new(x, y + height)),
        C::Close,
    ])
}

fn custom(value: f64) -> StreamLayout {
    stream_layout_with_dot_factory(
        &config(),
        &categories(),
        &source(value),
        &options(),
        |context| {
            let [x, y] = context.position;
            let mut dot = context.inherit(rectangle(x + 12., y - 6., 12., 12.));
            dot.fill = None;
            dot.stroke = Some(GraphicStroke {
                color: INK,
                width: 3.,
                cap: StrokeCap::Butt,
            });
            Ok(dot)
        },
    )
    .unwrap()
}

fn model(layout: &StreamLayout, picking: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(layout.plot),
            picking,
            ..Default::default()
        },
        Some(layout.slice_interaction_binding()),
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
fn factory_receives_raw_nullable_sources_and_preserves_all_dot_anchor_positions() {
    let data = [
        BarSeries::new("A", vec![Some(10.), None, Some(30.)]),
        BarSeries::new("hidden", vec![Some(8.); 3]),
    ];
    for size in [0., 6.] {
        for position in [
            StreamDotPosition::Start,
            StreamDotPosition::Center,
            StreamDotPosition::End,
        ] {
            let mut opts = options();
            opts.hidden = BTreeSet::from(["hidden".into()]);
            opts.dots.as_mut().unwrap().position = position;
            opts.dots.as_mut().unwrap().size = size;
            let baseline = stream_layout(&config(), &categories(), &data, &opts).unwrap();
            let mut seen = vec![];
            let result =
                stream_layout_with_dot_factory(&config(), &categories(), &data, &opts, |context| {
                    assert_eq!(context.series, &data[0]);
                    assert_eq!(context.series_index, 0);
                    assert_eq!(context.layer_label, "A");
                    assert_eq!(
                        context.sample.value,
                        context.series.values[context.sample.source_index]
                    );
                    assert!(context.sample.value.is_some());
                    assert_eq!(context.position, context.sample.position);
                    let expected = match position {
                        StreamDotPosition::Start => context.sample.screen_y[0],
                        StreamDotPosition::Center => {
                            context.sample.screen_y[0] * 0.5 + context.sample.screen_y[1] * 0.5
                        }
                        StreamDotPosition::End => context.sample.screen_y[1],
                    };
                    assert_eq!(context.position[1], expected);
                    seen.push((context.sample.source_index, context.sample.category.clone()));
                    let [x, y] = context.position;
                    Ok(context.inherit(Geometry::Circle {
                        x,
                        y,
                        radius: context.style.size / 2.,
                    }))
                })
                .unwrap();
            assert_eq!(seen, [(0, 1_f64.into()), (2, "tail".into())]);
            assert_eq!(result.scene, baseline.scene);
            assert_eq!(result.layers, baseline.layers);
            assert_eq!(result.interaction_binding(), baseline.interaction_binding());
            assert_eq!(
                result.slice_interaction_binding(),
                baseline.slice_interaction_binding()
            );
            assert_eq!(result.layers[0].samples[1].value, None);
            assert_eq!(result.layers[1].samples.len(), 3);
        }
    }
    let mut opts = options();
    opts.dots = None;
    stream_layout_with_dot_factory(&config(), &categories(), &data, &opts, |_| {
        panic!("disabled dots invoked the point factory")
    })
    .unwrap();
}

#[test]
fn offset_hollow_paths_use_painted_geometry_and_the_original_nearest_and_slice_sites() {
    let result = custom(20.);
    let sample = &result.layers[0].samples[1];
    let [x, y] = sample.position;
    assert_eq!([x, y], [111., 133.]);
    let mut exact = model(&result, Picking::Geometry);
    assert!(hover(&mut exact, [x, y]).is_empty());
    assert!(hover(&mut exact, [x + 18., y]).is_empty());
    assert_eq!(
        hover(&mut exact, [x + 24., y]).as_slice(),
        std::slice::from_ref(&sample.mark_id)
    );
    let mut nearest = model(&result, Picking::Nearest { distance: 1. });
    assert_eq!(
        hover(&mut nearest, [x, y]).as_slice(),
        std::slice::from_ref(&sample.mark_id)
    );
    assert!(hover(&mut nearest, [x + 24., y]).is_empty());
    let mut slice = model(&result, Picking::SliceX { distance: 1. });
    assert_eq!(
        hover(&mut slice, [x, 14.]).as_slice(),
        std::slice::from_ref(&sample.mark_id)
    );
    let active = slice.active_interaction(slice.hovered()).unwrap();
    assert_eq!(active.rows[0].position, sample.position);
    assert_eq!(active.rows[0].color, INK);
    assert_eq!(active.rows[0].value, Some(20.));
    assert_eq!(result.slice_interaction_binding().anchors.len(), 3);
    assert_eq!(result.interaction_binding().anchors.len(), 4);
    let last = &result.layers[0].samples[2];
    assert!(hover(&mut exact, last.position).is_empty());
    assert!(hover(&mut exact, [last.position[0] + 24., last.position[1]]).is_empty());
    assert_eq!(
        hover(&mut nearest, last.position).as_slice(),
        std::slice::from_ref(&last.mark_id)
    );
}

#[test]
fn shared_filled_and_stroked_glyphs_keep_raw_start_center_and_end_origins() {
    let shapes = [
        GlyphShape::Filled(FilledGlyph::Circle),
        GlyphShape::Filled(FilledGlyph::Cross),
        GlyphShape::Filled(FilledGlyph::Diamond),
        GlyphShape::Filled(FilledGlyph::Square),
        GlyphShape::Filled(FilledGlyph::Star),
        GlyphShape::Filled(FilledGlyph::Triangle),
        GlyphShape::Filled(FilledGlyph::Wye),
        GlyphShape::Stroked(StrokedGlyph::Circle),
        GlyphShape::Stroked(StrokedGlyph::Plus),
        GlyphShape::Stroked(StrokedGlyph::Times),
        GlyphShape::Stroked(StrokedGlyph::Triangle),
        GlyphShape::Stroked(StrokedGlyph::Asterisk),
        GlyphShape::Stroked(StrokedGlyph::Square),
        GlyphShape::Stroked(StrokedGlyph::Diamond),
    ];
    for shape in shapes {
        for position in [
            StreamDotPosition::Start,
            StreamDotPosition::Center,
            StreamDotPosition::End,
        ] {
            let glyph = GlyphOptions {
                shape,
                size: 96.,
                rotation: 0.27,
            };
            let mut opts = options();
            opts.dots = Some(StreamDotOptions {
                position,
                glyph: Some(glyph.clone()),
                color: InheritedColor::Fixed(INK),
                border_width: 2.,
                border_color: InheritedColor::Fixed(BORDER),
                ..Default::default()
            });
            let result = stream_layout(&config(), &categories(), &source(20.), &opts).unwrap();
            let sample = &result.layers[0].samples[1];
            let mark = result
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == sample.mark_id)
                .unwrap();
            let expected = glyph_geometry(sample.position, &glyph).unwrap();
            assert_eq!(mark.geometry, expected.geometry);
            assert_eq!(
                mark.fill,
                if matches!(shape, GlyphShape::Filled(_)) {
                    Some(INK)
                } else {
                    None
                }
            );
            assert_eq!(mark.stroke.unwrap().color, BORDER);
            assert_eq!(
                result.slice_interaction_binding().anchor_for(mark).unwrap(),
                Some(sample.position)
            );
            assert_eq!(
                result.interaction_binding().anchor_for(mark).unwrap(),
                Some(sample.position)
            );
        }
    }
    let mut invalid = options();
    invalid.dots.as_mut().unwrap().glyph = Some(GlyphOptions {
        shape: GlyphShape::Stroked(StrokedGlyph::Plus),
        ..Default::default()
    });
    assert!(stream_layout(&config(), &categories(), &source(20.), &invalid).is_err());
}

#[test]
fn compatible_motion_keeps_offset_path_anchors_and_rejects_topology_changes_atomically() {
    let old = custom(20.);
    let target = custom(24.);
    let live = old.scene.interpolate(&target.scene, 0.5).unwrap();
    let binding = target.slice_interaction_binding();
    let sample = &target.layers[0].samples[1];
    let mut model = model(&old, Picking::Nearest { distance: 1. });
    model
        .set_interaction_frame(&target.scene, Some(binding.clone()), live.clone())
        .unwrap();
    assert_eq!(
        hover(&mut model, [111., 129.]).as_slice(),
        std::slice::from_ref(&sample.mark_id)
    );
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(active.rows[0].position, [111., 129.]);
    assert_eq!(active.rows[0].value, Some(24.));
    let mut bad = live.clone();
    let dot = bad
        .marks
        .iter_mut()
        .find(|mark| mark.id == sample.mark_id)
        .unwrap();
    if let Geometry::Path(path) = &mut dot.geometry {
        path.pop();
    }
    assert!(
        model
            .set_interaction_frame(&target.scene, Some(binding), bad)
            .is_err()
    );
    assert_eq!(model.scene(), &live);
    assert_eq!(
        hover(&mut model, [111., 129.]).as_slice(),
        std::slice::from_ref(&sample.mark_id)
    );
}

#[test]
fn factory_validation_rejects_offset_primitives_bad_supports_and_aggregate_command_overflow() {
    let data = source(20.);
    assert!(
        stream_layout_with_dot_factory(&config(), &categories(), &data, &options(), |context| {
            let [x, y] = context.position;
            Ok(context.inherit(Geometry::Circle {
                x: x + 1.,
                y,
                radius: 3.,
            }))
        })
        .is_err()
    );
    assert!(
        stream_layout_with_dot_factory(&config(), &categories(), &data, &options(), |context| {
            let [x, y] = context.position;
            let mut dot = context.inherit(rectangle(x + 12., y - 6., 12., 12.));
            dot.anchor_commands = Some(vec![0, 0]);
            Ok(dot)
        })
        .is_err()
    );
    assert!(
        stream_layout_with_dot_factory(&config(), &categories(), &data, &options(), |context| {
            let [x, y] = context.position;
            Ok(context.inherit(Geometry::Path(vec![C::Move(P::new(x, y)); 120_000])))
        })
        .is_err()
    );
    assert!(
        stream_layout_with_dot_factory(&config(), &categories(), &data, &options(), |_| {
            Err(ChartError("rejected Stream dot".into()))
        })
        .is_err_and(|error| error.0 == "rejected Stream dot")
    );
    assert!(stream_layout(&config(), &categories(), &data, &options()).is_ok());
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_and_svg_preserve_the_custom_hole_and_plot_clip() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let result = custom(20.);
    let image = render_raster(
        &result.scene,
        &FontRegistry::new(),
        &RasterOptions::default(),
    )
    .unwrap();
    let pixel = |x: u32, y: u32| -> Vec<u8> {
        let start = ((y * image.width + x) * 4) as usize;
        image.rgba[start..start + 4].to_vec()
    };
    assert_eq!(pixel(135, 133), [165, 30, 91, 255]);
    assert_eq!(pixel(129, 133), [255, 255, 255, 255]);
    assert_eq!(pixel(111, 133), [255, 255, 255, 255]);
    assert_eq!(pixel(213, 113), [255, 255, 255, 255]);
    let svg = render::to_svg(&result.scene).unwrap();
    assert!(
        svg.contains("stroke-width=\"3\"")
            && svg.contains("viewBox=\"11 13 200 160\" overflow=\"hidden\"")
    );
}
