//! Authored geographic glyph snapshots. Interaction is exercised by the host.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
fn caption(scene: &mut Scene, c: &ChartConfig, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: c.width / 2.,
        y: c.height - 9.,
        text: text.into(),
        color: Color(70, 90, 110, 255),
        size: 11.,
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
}
fn map(c: &ChartConfig, globe: bool) -> Result<GeoMapLayout> {
    let mut map = choropleth_layout(
        c,
        &[GeoFeature {
            id: "land".into(),
            label: String::new(),
            value: Some(1.),
            polygons: vec![vec![vec![
                [-35., -30.],
                [35., -30.],
                [35., 35.],
                [-35., 35.],
                [-35., -30.],
            ]]],
        }],
        &GeoMapOptions {
            projection: if globe {
                GeoProjection::Orthographic
            } else {
                GeoProjection::EqualEarth
            },
            scale: Some(if globe { 135. } else { 95. }),
            translate: Some([c.width / 2., 205.]),
            fill: Some(Color(228, 237, 244, 255)),
            border: Some(Color(130, 154, 178, 255)),
            ..Default::default()
        },
    )?;
    map.scene.legend.clear();
    for mark in &mut map.scene.marks {
        mark.label.clear();
        mark.series.clear();
    }
    Ok(map)
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut star = map(c, false)?;
    let mut source = vec![
        GeoSymbol::new("main", [0., 0.], Some(81.)),
        GeoSymbol::new("small", [-60., -25.], Some(9.)),
        GeoSymbol::new("missing", [45., -20.], None),
        GeoSymbol::new("zero", [70., 28.], Some(0.)),
        GeoSymbol::new("hidden", [80., 10.], Some(100.)),
    ];
    for p in &mut source {
        p.series = if p.id == "hidden" { "Beta" } else { "Alpha" }.into();
    }
    let mut options = GeoGlyphOptions {
        shape: GlyphShape::Filled(FilledGlyph::Star),
        symbols: GeoSymbolOptions {
            domain: Some([0., 100.]),
            radius_range: [0., 20.],
            labels: true,
            missing: GeoMissingSymbol::Draw {
                radius: 8.,
                color: Color(190, 110, 35, 255),
                label: "missing".into(),
            },
            stroke: Some(GraphicStroke {
                color: Color(255, 255, 255, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions {
                    position: [25., 45.],
                    title: "Equivalent area".into(),
                    ..Default::default()
                },
                values: vec![9., 36., 81.],
            }),
            color_legend: Some(GeoLegendOptions {
                position: [c.width - 135., 45.],
                title: "Full-source colors".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    options.hidden.insert("Beta".into());
    star.add_glyphs(&source, &options)?;
    caption(
        &mut star.scene,
        c,
        "Snapshot: Beta hidden; zero omitted; missing retained; legends use real Star geometry",
    );
    star.scene.validate()?;

    let mut globe = map(c, true)?;
    let mut source = vec![
        GeoSymbol::new("main", [0., 0.], Some(100.)),
        GeoSymbol::new("partial", [85., 0.], Some(64.)),
        GeoSymbol::new("missing", [-25., -35.], None),
        GeoSymbol::new("back", [150., 0.], Some(100.)),
    ];
    for p in &mut source {
        p.series = if p.id == "partial" { "Beta" } else { "Alpha" }.into();
    }
    let options = GeoGlyphOptions {
        shape: GlyphShape::Stroked(StrokedGlyph::Triangle),
        rotation: 0.23,
        symbols: GeoSymbolOptions {
            domain: Some([0., 100.]),
            radius_range: [0., 24.],
            labels: true,
            missing: GeoMissingSymbol::Draw {
                radius: 10.,
                color: Color(190, 110, 35, 255),
                label: "missing".into(),
            },
            stroke: Some(GraphicStroke {
                color: Color(20, 75, 140, 255),
                width: 4.,
                cap: StrokeCap::Round,
            }),
            clip_to_projection: true,
            clip_labels_to_projection: true,
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions {
                    position: [25., 45.],
                    title: "D3 nominal size".into(),
                    ..Default::default()
                },
                values: vec![16., 64.],
            }),
            color_legend: Some(GeoLegendOptions {
                position: [c.width - 130., 45.],
                title: "Stroke colors".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    globe.add_glyphs(&source, &options)?;
    caption(
        &mut globe.scene,
        c,
        "Snapshot: original triangle origins; real hemisphere masks; partial stroke clipped; back face culled",
    );
    globe.scene.validate()?;
    Ok(vec![
        ("Geographic Star glyphs with shared legends", star.scene),
        ("Clipped globe: stroked glyphs", globe.scene),
    ])
}
