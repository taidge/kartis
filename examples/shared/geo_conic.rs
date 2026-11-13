//! Shared application scenes for Conic maps, symbols, routes and graticules.
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;
#[derive(Clone, Debug)]
pub struct Settings {
    pub family: u8,
    pub diagnostic: bool,
    pub parallels: [f64; 2],
    pub fit_data: bool,
    pub center: [f64; 2],
    pub roll: f64,
    pub hidden: bool,
    pub width: f64,
    pub height: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            family: 0,
            diagnostic: false,
            parallels: [0., 60.],
            fit_data: false,
            center: [0., 0.],
            roll: 0.,
            hidden: false,
            width: 640.,
            height: 420.,
        }
    }
}
impl Settings {
    pub fn projection(&self) -> Result<GeoProjection> {
        match self.family {
            0 => GeoProjection::conic_equal_area(self.parallels),
            1 => GeoProjection::conic_equidistant(self.parallels),
            2 => GeoProjection::conic_conformal(self.parallels),
            3 if self.parallels == [29.5, 45.5] => Ok(GeoProjection::Albers),
            3 => Err(ChartError(
                "Albers is a fixed preset; choose Conic Equal Area to edit its parallels".into(),
            )),
            _ => Err(ChartError("unknown conic family".into())),
        }
    }
}
#[derive(Clone)]
pub struct Source {
    pub settings: Settings,
    pub published: SceneInteraction,
    pub context: GeoProjectionContext,
    pub options: GeoMapOptions,
    pub features: Vec<GeoFeature>,
    pub symbols: Vec<GeoSymbol>,
    pub mask_status: &'static str,
}
pub fn features() -> Vec<GeoFeature> {
    [
        (-110., 25., Some(0.)),
        (-80., 30., Some(25.)),
        (-15., 20., None),
        (35., 10., Some(70.)),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, y, value))| GeoFeature {
        id: format!("region-{i}"),
        label: format!("Region {i}"),
        value,
        polygons: vec![vec![vec![
            [x, y],
            [x, y + 16.],
            [x + 20., y + 16.],
            [x + 20., y],
            [x, y],
        ]]],
    })
    .collect()
}
pub fn build(s: &Settings) -> Result<Source> {
    let config = ChartConfig {
        width: s.width,
        height: s.height,
        margin: Margin {
            left: if s.diagnostic { 70. } else { 36. },
            right: if s.diagnostic { 70. } else { 28. },
            top: if s.diagnostic { 30. } else { 24. },
            bottom: if s.diagnostic { 30. } else { 92. },
        },
        ..Default::default()
    };
    let options = GeoMapOptions {
        projection: s.projection()?,
        center: s.center,
        rotation: s.roll,
        fit: if s.fit_data {
            GeoFit::Data
        } else {
            GeoFit::World
        },
        padding: if s.diagnostic { 0. } else { 12. },
        border: Some(Color(40, 65, 80, 255)),
        border_width: 1.5,
        ..Default::default()
    };
    let features = (if s.diagnostic {
        vec![GeoFeature {
            id: "reference-region".into(),
            label: "Reference exterior and hole".into(),
            value: Some(1.),
            polygons: vec![vec![
                vec![
                    [-125., 25.],
                    [-125., 50.],
                    [-65., 50.],
                    [-65., 25.],
                    [-125., 25.],
                ],
                vec![
                    [-105., 32.],
                    [-95., 32.],
                    [-95., 40.],
                    [-105., 40.],
                    [-105., 32.],
                ],
            ]],
        }]
    } else {
        features()
    })
    .into_iter()
    .filter(|f| !s.hidden || f.id != "region-1")
    .collect::<Vec<_>>();
    let mut map = choropleth_with_color_profile(
        &config,
        &features,
        &options,
        &ColorScaleOptions {
            domain: Some([0., 100.]),
            colors: vec![Color(215, 239, 235, 255), Color(20, 105, 125, 255)],
            unknown: Color(160, 148, 173, 255),
            ..Default::default()
        },
        ColorInterpolation::Srgb,
    )?;
    if s.diagnostic {
        let mut reference = geographic_map_layout_with_path_options(
            &config,
            &features,
            &options,
            &GeoPathOptions {
                precision: 0.0001,
                ..Default::default()
            },
        )?;
        for mark in &mut reference.scene.marks {
            if let Some(raw) = features.iter().find(|f| f.id == mark.series) {
                mark.fill = Some(map.color_scale.map(raw.value));
            }
        }
        map.scene = reference.scene;
        map.projection = reference.projection;
    }
    let context = map.projection;
    let mut scene = map.scene;
    let footprint = context
        .projection_clip_path(0.25, 4096)?
        .ok_or_else(|| ChartError("missing Conic footprint".into()))?;
    let mut outline = Mark::new(
        "conic-footprint",
        footprint.geometry.clone(),
        Color(95, 125, 150, 255),
    );
    outline.fill = None;
    outline.label.clear();
    outline.stroke = Some(GraphicStroke {
        color: Color(95, 125, 150, 255),
        width: 1.,
        cap: StrokeCap::Butt,
    });
    outline.clip = Some(context.plot());
    scene.marks.push(outline);
    let mut sites = Vec::new();
    let mut anchors = std::collections::BTreeMap::new();
    for mark in &scene.marks {
        if let Some(raw) = features.iter().find(|f| f.id == mark.series) {
            let point = [
                raw.polygons[0][0][0][0] + 10.,
                raw.polygons[0][0][0][1] + 8.,
            ];
            if let Some(position) = context.project(point)? {
                let points = mark
                    .geometry
                    .commands()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| match c {
                        PathCommand::Move(_) | PathCommand::Line(_) => Some(i),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let commands = mark.geometry.commands();
                let projected_support = points
                    .iter()
                    .map(|i| match commands[*i] {
                        PathCommand::Move(p) | PathCommand::Line(p) => [p.x, p.y],
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>();
                let position = if projected_support.is_empty() {
                    position
                } else {
                    [
                        projected_support.iter().map(|p| p[0]).sum::<f64>()
                            / projected_support.len() as f64,
                        projected_support.iter().map(|p| p[1]).sum::<f64>()
                            / projected_support.len() as f64,
                    ]
                };
                if !points.is_empty() {
                    anchors.insert(
                        mark.id.clone(),
                        InteractionAnchor::path_points(&mark.geometry, position, &points)?,
                    );
                }
                sites.push(InteractionSite {
                    mark_id: mark.id.clone(),
                    series: raw.id.clone(),
                    label: raw.label.clone(),
                    value: raw.value,
                    formatted_value: raw.value.map(|v| v.to_string()),
                    color: map.color_scale.map(raw.value),
                    position,
                });
            }
        }
    }
    let symbols = features
        .iter()
        .map(|f| {
            let mut p = GeoSymbol::new(
                format!("station-{}", f.id),
                [f.polygons[0][0][0][0] + 8., f.polygons[0][0][0][1] + 6.],
                f.value,
            );
            p.label = format!("Station {}", f.id);
            p
        })
        .collect::<Vec<_>>();
    context.add_graticule(
        &mut scene,
        &GeoGraticuleOptions {
            step_major: [60., 30.],
            step_minor: [30., 30.],
            ..Default::default()
        },
        &GeoStreamOptions {
            interactive: false,
            behind: true,
            style: GeoLineStyle {
                stroke: GraphicStroke {
                    color: Color(170, 185, 195, 255),
                    width: 0.7,
                    cap: StrokeCap::Butt,
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    context.add_lines(
        &mut scene,
        &[GeoLineFeature::new(
            "coast-route",
            vec![[-100., 32.], [-60., 42.], [-5., 28.], [45., 20.]],
        )],
        &GeoStreamOptions {
            interactive: false,
            style: GeoLineStyle {
                stroke: GraphicStroke {
                    color: Color(145, 75, 135, 255),
                    width: 2.,
                    cap: StrokeCap::Round,
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    context.add_symbols(
        &mut scene,
        &symbols,
        &GeoSymbolOptions {
            radius_range: [5., 10.],
            zero_radius: 5.,
            clip_to_projection: true,
            clip_labels_to_projection: true,
            labels: true,
            color: GeoSymbolColor::Fixed(Color(220, 105, 35, 255)),
            missing: GeoMissingSymbol::Draw {
                radius: 7.,
                color: Color(160, 148, 173, 255),
                label: "Missing".into(),
            },
            ..Default::default()
        },
    )?;
    for raw in &symbols {
        let id = format!("geo-symbol:{}:{}", raw.id.len(), raw.id);
        if let Some(mark) = scene.marks.iter_mut().find(|m| m.id == id)
            && let Some(position) = context.project(raw.position)?
        {
            mark.label = raw.label.clone();
            mark.fill = Some(map.color_scale.map(raw.value));
            sites.push(InteractionSite {
                mark_id: id,
                series: raw.series.clone(),
                label: raw.label.clone(),
                value: raw.value,
                formatted_value: raw.value.map(|v| v.to_string()),
                color: mark.fill.unwrap_or(Color(0, 0, 0, 255)),
                position,
            });
        }
    }
    if !s.diagnostic {
        append_color_legend(
            &mut scene,
            &map.color_scale,
            &ColorLegendOptions {
                position: [70., s.height - 55.],
                length: s.width - 220.,
                title: "Original observed value".into(),
                unknown_label: Some("Missing".into()),
                ..Default::default()
            },
        )?;
    }
    let published = SceneInteraction {
        scene,
        interaction: Some(InteractionBinding {
            sites,
            anchors,
            ..Default::default()
        }),
        plot: Some(context.plot()),
    };
    published.validate()?;
    Ok(Source {
        settings: s.clone(),
        published,
        context,
        options,
        features,
        symbols,
        mask_status: "Glyphs use footprint plus plot clip; labels use an independent footprint mask",
    })
}
pub fn gallery_scenes(config: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut a = Settings {
        width: config.width,
        height: config.height,
        fit_data: true,
        ..Default::default()
    };
    let first = build(&a)?.published.scene;
    a.family = 3;
    a.parallels = [29.5, 45.5];
    a.center = [15., -5.];
    a.roll = 18.;
    Ok(vec![
        ("Conic · equal-area observations", first),
        (
            "Albers · shared map and symbols",
            build(&a)?.published.scene,
        ),
    ])
}
