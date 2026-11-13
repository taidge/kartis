use glory::render::{GraphicStroke, StrokeCap};
use kartis::render::raster::{ColorGlyphMode, FontRegistry, MissingGlyphPolicy};
use kartis::*;

fn measurements(
    layout: &SunburstLayout,
    fonts: &FontRegistry,
) -> Result<Vec<SunburstMeasuredLabel>> {
    layout
        .label_candidates()
        .iter()
        .map(|candidate| {
            let bounds = fonts
                .measure_label(&candidate.label, MissingGlyphPolicy::Error)?
                .map(|ink| LayoutRect {
                    x: ink.x - candidate.label.x,
                    y: ink.y - candidate.label.y,
                    width: ink.width,
                    height: ink.height,
                })
                .unwrap_or_default();
            Ok(candidate.measured(bounds))
        })
        .collect()
}

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::Reject);
    fonts.register(
        include_bytes!("../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    let options = SunburstLayoutOptions {
        include_root: true,
        corner_radius: 5.,
        pad_angle: 0.025,
        border_width: 1.5,
        labels: Some(SunburstLabelOptions {
            text: SunburstLabelText::Label,
            size: Some(14.),
            ..Default::default()
        }),
        ..Default::default()
    };
    let portfolio = [
        HierarchyDatum::new("Portfolio", None, 0.),
        HierarchyDatum::new("Cloud", Some("Portfolio".into()), 0.),
        HierarchyDatum::new("Devices", Some("Portfolio".into()), 0.),
        HierarchyDatum::new("Compute", Some("Cloud".into()), 24.),
        HierarchyDatum::new("Storage", Some("Cloud".into()), 18.),
        HierarchyDatum::new("Phone", Some("Devices".into()), 20.),
        HierarchyDatum::new("Tablet", Some("Devices".into()), 14.),
    ];
    let layout = sunburst_layout(c, &portfolio, &options)?;
    let measured = measurements(&layout, &fonts)?;
    let root = layout
        .label_candidates()
        .iter()
        .position(|candidate| candidate.node_id == "Portfolio")
        .ok_or_else(|| ChartError("missing portfolio label candidate".into()))?;
    let label = &layout.label_candidates()[root].label;
    let ink = measured[root].bounds;
    let obstacle = LayoutRect {
        x: label.x + ink.x - 6.,
        y: label.y + ink.y - 6.,
        width: ink.width + 12.,
        height: ink.height + 12.,
    };
    let mut placement = SunburstLabelPlacementOptions::for_layout(&layout);
    placement.priorities.insert("Portfolio".into(), 100);
    placement.offsets = vec![(0., -28.), (0., 28.), (0., -56.), (0., 56.)];
    placement.collision.padding = 3.;
    placement.collision.forbidden.push(obstacle);
    let mut plot = layout.place_labels(&measured, &placement)?.scene;
    let mut panel = Mark::new(
        "measured-sunburst-obstacle",
        Geometry::Rect {
            x: obstacle.x,
            y: obstacle.y,
            width: obstacle.width,
            height: obstacle.height,
        },
        Color(244, 247, 250, 255),
    );
    panel.label.clear();
    panel.stroke = Some(GraphicStroke {
        color: Color(120, 137, 155, 255),
        width: 1.,
        cap: StrokeCap::Butt,
    });
    panel.stroke_dash = Some(StrokeDash {
        segments: vec![4., 3.],
        offset: 0.,
    });
    plot.marks.push(panel);

    let studio = [
        HierarchyDatum::new("Studio", None, 0.),
        HierarchyDatum::new("Cloud", Some("Studio".into()), 0.),
        HierarchyDatum::new("Gear", Some("Studio".into()), 0.),
        HierarchyDatum::new("Compute platform", Some("Cloud".into()), 24.),
        HierarchyDatum::new("Files", Some("Cloud".into()), 18.),
        HierarchyDatum::new("Phone", Some("Gear".into()), 20.),
        HierarchyDatum::new("Tablet", Some("Gear".into()), 14.),
    ];
    let mask_options = SunburstLayoutOptions {
        labels: Some(SunburstLabelOptions {
            text: SunburstLabelText::Label,
            size: Some(12.),
            ..Default::default()
        }),
        ..options
    };
    let layout = sunburst_layout(c, &studio, &mask_options)?.with_label_rotation(12.)?;
    let measured = measurements(&layout, &fonts)?;
    let mut placement = SunburstLabelPlacementOptions::for_layout(&layout);
    placement.clip = SunburstLabelClip::ClipToArc;
    placement.collision.padding = 3.;
    let masked = layout.place_labels(&measured, &placement)?.scene;

    let paint = PaintedLabelOptions {
        rotated_only: false,
        missing_glyph: MissingGlyphPolicy::Error,
        ..Default::default()
    };
    Ok(vec![
        (
            "Sunburst · measured label placement",
            scene_with_painted_labels(&plot, &fonts, &paint, "sunburst-measured-plot")?,
        ),
        (
            "Sunburst · measured arc masks",
            scene_with_painted_labels(&masked, &fonts, &paint, "sunburst-measured-masks")?,
        ),
    ])
}
