#![cfg(feature = "raster")]
use glory::render::StrokeCap;
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 360.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 30.,
            bottom: 50.,
        },
        ..Default::default()
    }
}
fn dimensions() -> Vec<ParallelDimension> {
    ["A", "B", "C"]
        .into_iter()
        .map(|id| {
            let mut d = ParallelDimension::new(id);
            d.domain = Some([0., 100.]);
            d.axis.visible = false;
            d
        })
        .collect()
}
fn pixel(image: &kartis::render::raster::RenderedImage, x: usize, y: usize) -> [u8; 4] {
    image.rgba[(y * 600 + x) * 4..(y * 600 + x) * 4 + 4]
        .try_into()
        .unwrap()
}
#[test]
fn actual_cpu_strokes_preserve_dash_gaps_both_layouts_and_changed_data() {
    let config = config();
    let dims = dimensions();
    let data = [ParallelDatum::new("row", vec![Some(50.); 3])];
    let mut options = ParallelCoordinatesOptions {
        line: ParallelLineStyle {
            color: Some(Color(230, 30, 40, 255)),
            width: 6.,
            opacity: 1.,
            cap: StrokeCap::Butt,
            dash: Some(StrokeDash {
                segments: vec![16., 16.],
                offset: 0.,
            }),
        },
        ..Default::default()
    };
    let fonts = FontRegistry::new();
    let scene = parallel_coordinates_with_options(&config, &dims, &data, &options).unwrap();
    let image = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
    assert_eq!(pixel(&image, 70, 170), [230, 30, 40, 255]);
    assert_eq!(pixel(&image, 84, 170), [255, 255, 255, 255]);
    let changed = parallel_coordinates_with_options(
        &config,
        &dims,
        &[ParallelDatum::new("row", vec![Some(75.); 3])],
        &options,
    )
    .unwrap();
    let image = render_raster(&changed, &fonts, &RasterOptions::default()).unwrap();
    assert_eq!(pixel(&image, 70, 170), [255, 255, 255, 255]);
    assert_eq!(pixel(&image, 70, 100), [230, 30, 40, 255]);
    options.layout = ParallelLayout::Vertical;
    let scene = parallel_coordinates_with_options(&config, &dims, &data, &options).unwrap();
    let image = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
    assert_eq!(pixel(&image, 300, 300), [230, 30, 40, 255]);
    assert_eq!(pixel(&image, 300, 284), [255, 255, 255, 255]);
}
#[test]
fn actual_shaped_endpoint_labels_obey_plot_mask_and_keep_axis_captions_unowned() {
    let config = config();
    let dims = dimensions();
    let mut row = ParallelDatum::new("row", vec![Some(50.); 3]);
    row.label = "ROW LABEL".into();
    let data = [row];
    let options = ParallelCoordinatesOptions {
        line: ParallelLineStyle {
            color: Some(Color(230, 30, 40, 255)),
            opacity: 1.,
            ..Default::default()
        },
        row_labels: true,
        row_label_style: ParallelTextStyle {
            color: Some(Color(20, 30, 230, 255)),
            size: Some(20.),
        },
        row_label_offset: [-20., -24.],
        ..Default::default()
    };
    let original = parallel_coordinates_with_options(&config, &dims, &data, &options).unwrap();
    let clipped = parallel_coordinates_with_options(
        &config,
        &dims,
        &data,
        &ParallelCoordinatesOptions {
            clip_row_labels: true,
            ..options
        },
    )
    .unwrap();
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let source = render_raster(&original, &fonts, &RasterOptions::default()).unwrap();
    let masked = render_raster(&clipped, &fonts, &RasterOptions::default()).unwrap();
    let blue = |p: [u8; 4]| p[2] > 150 && p[0] < 70 && p[1] < 100;
    let (mut original_outside, mut masked_outside, mut kept) = (0, 0, 0);
    for y in 110..150 {
        for x in 500..600 {
            original_outside += usize::from(x > 542 && blue(pixel(&source, x, y)));
            masked_outside += usize::from(x > 542 && blue(pixel(&masked, x, y)));
            kept += usize::from(x < 538 && blue(pixel(&masked, x, y)));
        }
    }
    assert!(
        original_outside > 30 && kept > 8,
        "must draw real font ink on both sides: {original_outside}/{kept}"
    );
    assert_eq!(masked_outside, 0);
    assert_eq!(clipped.label_series.len(), 1);
    assert_eq!(clipped.label_clips.len(), 1);
}
