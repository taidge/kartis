//! Independent thickness, robust triangulations and geographic line streams.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::stack::{StackOffset, StackOptions};
use kartis::*;

fn note(scene: &mut Scene, frame: &ChartConfig, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: frame.margin.left,
        y: 22.,
        text: text.into(),
        color: Color(80, 105, 125, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
}

fn dimensions(signed: bool) -> Vec<BarSeries> {
    let values = if signed {
        vec![
            ("North", vec![-2., 3., -1.]),
            ("South", vec![3., -1., -4.]),
            ("Online", vec![-4., 2., 2.]),
        ]
    } else {
        vec![("North", vec![6., 2., 4.]), ("South", vec![2., 6., 0.])]
    };
    values
        .into_iter()
        .map(|(id, values)| BarSeries::new(id, values.into_iter().map(Some).collect()))
        .collect()
}

fn geographic_scenes(frame: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let land = [
        (
            "West",
            vec![
                [-130., 15.],
                [-65., 15.],
                [-65., 60.],
                [-130., 60.],
                [-130., 15.],
            ],
        ),
        (
            "South",
            vec![
                [-80., -50.],
                [-40., -50.],
                [-40., -10.],
                [-80., -10.],
                [-80., -50.],
            ],
        ),
        (
            "Central",
            vec![
                [-15., -30.],
                [40., -30.],
                [40., 55.],
                [-15., 55.],
                [-15., -30.],
            ],
        ),
        (
            "East",
            vec![[60., 5.], [135., 5.], [135., 55.], [60., 55.], [60., 5.]],
        ),
    ]
    .into_iter()
    .map(|(id, ring)| GeoFeature {
        id: id.into(),
        label: id.into(),
        value: Some(1.),
        polygons: vec![vec![ring]],
    })
    .collect::<Vec<_>>();
    let mut routes = vec![
        GeoLineFeature::new("Atlantic", vec![[-75., 40.], [-3., 51.]]),
        GeoLineFeature::new("Pacific seam", vec![[-170., 15.], [170., 35.]]),
        GeoLineFeature::new("Northern route", vec![[3., 45.], [120., 35.]]),
    ];
    for route in &mut routes {
        route.value = Some(1.);
        route.series = "routes".into();
    }
    let route_style = GeoStreamOptions {
        style: GeoLineStyle {
            stroke: GraphicStroke {
                color: Color(228, 99, 64, 255),
                width: 2.5,
                cap: StrokeCap::Round,
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let grid_style = GeoStreamOptions {
        style: GeoLineStyle {
            stroke: GraphicStroke {
                color: Color(125, 153, 178, 255),
                width: 0.65,
                cap: StrokeCap::Butt,
            },
            ..Default::default()
        },
        behind: true,
        ..Default::default()
    };
    let grid = GeoGraticuleOptions {
        step_minor: [30., 30.],
        ..Default::default()
    };
    let path = GeoPathOptions {
        precision: 0.15,
        ..Default::default()
    };
    let mut output = vec![];
    for (name, projection, center, description) in [
        (
            "Adaptive geographic routes and graticule",
            GeoProjection::Equirectangular,
            [0., 0.],
            "Illustrative regions · great-circle routes split at the seam · shared graticule",
        ),
        (
            "Transverse Mercator shared overlays",
            GeoProjection::TransverseMercator,
            [20., 0.],
            "Transverse projection · finite square clipping · routes and points share one fit",
        ),
        (
            "Orthographic clipped route streams",
            GeoProjection::Orthographic,
            [-20., 15.],
            "Front hemisphere · route strokes clip to the globe disk · no horizon bridges",
        ),
    ] {
        let view = GeoMapOptions {
            projection,
            center,
            fill: Some(Color(209, 226, 231, 255)),
            border: Some(Color(105, 136, 149, 255)),
            border_width: 0.8,
            ..Default::default()
        };
        let mut map = geographic_map_layout_with_path_options(frame, &land, &view, &path)
            .map_err(|e| ChartError(format!("{name}: {e}")))?;
        map.add_graticule(&grid, &grid_style)?;
        map.add_lines(&routes, &route_style)?;
        map.add_symbols(
            &[
                GeoSymbol::new("New York", [-75., 40.], Some(1.)),
                GeoSymbol::new("London", [-3., 51.], Some(1.)),
            ],
            &GeoSymbolOptions {
                radius_range: [3., 4.],
                labels: true,
                clip_to_projection: projection == GeoProjection::Orthographic,
                ..Default::default()
            },
        )?;
        note(&mut map.scene, frame, description);
        output.push((name, map.scene));
    }
    let high_arc = GeoLineFeature::new("High latitude route", vec![[-85., 10.], [85., 10.]]);
    let mut fitted = geographic_lines(
        frame,
        &[high_arc],
        &GeoMapOptions {
            fit: GeoFit::Data,
            padding: 15.,
            ..Default::default()
        },
        &GeoStreamOptions {
            path: Some(path),
            ..route_style
        },
    )?;
    fitted.projection.add_symbols(
        &mut fitted.scene,
        &[
            GeoSymbol::new("Origin", [-85., 10.], Some(1.)),
            GeoSymbol::new("Destination", [85., 10.], Some(1.)),
        ],
        &GeoSymbolOptions {
            radius_range: [3., 4.],
            labels: true,
            ..Default::default()
        },
    )?;
    note(
        &mut fitted.scene,
        frame,
        "Data fit includes the curved route's projected bounds · endpoints reuse the exact fit",
    );
    output.push(("Adaptive great-circle data fit", fitted.scene));
    Ok(output)
}

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut frame = c.clone();
    frame.margin = Margin {
        top: 70.,
        right: 55.,
        bottom: 55.,
        left: 65.,
    };
    let categories = [("Services", 1.), ("Products", 3.), ("Support", 2.)]
        .into_iter()
        .map(|(id, value)| MarimekkoCategory::new(id, value))
        .collect::<Vec<_>>();
    let mut options = MarimekkoLayoutOptions {
        thickness_nice: None,
        dimensions_nice: None,
        outer_padding: 8.,
        inner_padding: 10.,
        category_axis: Some(MarimekkoCategoryAxis {
            side: AxisSide::Bottom,
            options: CategoryAxisOptions::default(),
        }),
        labels: BarLabels {
            enabled: true,
            skip_width: 20.,
            skip_height: 16.,
            ..Default::default()
        },
        style: MarimekkoDimensionStyle {
            border: Some(GraphicStroke {
                color: Color(255, 255, 255, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    options.axes.left = Some(AxisOptions::default());
    let mut independent =
        marimekko_layout(&frame, &categories, &dimensions(false), &options)?.scene;
    note(
        &mut independent,
        &frame,
        "Widths 1:3:2 are independent of dimension totals 8:8:4",
    );
    let mut output = vec![("Marimekko independent thickness", independent)];
    options.horizontal = true;
    options.stack.offset = StackOffset::Expand;
    options.category_axis.as_mut().unwrap().side = AxisSide::Left;
    options.axes.left = None;
    options.axes.bottom = Some(AxisOptions::default());
    options.grid_y.enabled = false;
    options.grid_x.enabled = true;
    options.labels.value = BarLabelValue::Displayed;
    options.labels.format = ValueFormat::Percent(0);
    let mut expanded = marimekko_layout(&frame, &categories, &dimensions(false), &options)?.scene;
    note(
        &mut expanded,
        &frame,
        "Independent row heights · dimension contributions normalized per row",
    );
    output.push(("Marimekko horizontal expanded dimensions", expanded));
    options.horizontal = false;
    options.stack = StackOptions {
        offset: StackOffset::Diverging,
        ..Default::default()
    };
    options.category_axis.as_mut().unwrap().side = AxisSide::Bottom;
    options.axes.bottom = None;
    options.axes.left = Some(AxisOptions::default());
    options.grid_x.enabled = false;
    options.grid_y.enabled = true;
    options.labels.value = BarLabelValue::Raw;
    options.labels.format = ValueFormat::Auto;
    let mut signed = marimekko_layout(&frame, &categories, &dimensions(true), &options)?.scene;
    note(
        &mut signed,
        &frame,
        "Signed contributions keep raw values · positive and negative baselines",
    );
    output.push(("Marimekko signed diverging dimensions", signed));
    options.stack.offset = StackOffset::Zero;
    options.thickness_nice = Some(10);
    let niced_categories = [("Services", 1.1), ("Products", 4.2), ("Support", 8.8)]
        .into_iter()
        .map(|(id, value)| MarimekkoCategory::new(id, value))
        .collect::<Vec<_>>();
    let mut niced =
        marimekko_layout(&frame, &niced_categories, &dimensions(false), &options)?.scene;
    note(
        &mut niced,
        &frame,
        "Total thickness 14.1 · D3 nice domain 0..16 leaves real trailing space",
    );
    output.push(("Marimekko niced thickness scale", niced));

    let data = [
        (-2., -0.2),
        (-1., 1.),
        (0.1, 0.2),
        (1.1, 1.2),
        (2., -0.1),
        (0.8, -1.2),
        (-0.9, -1.),
    ];
    let points = data
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| DelaunaySite::new(format!("P{}", i + 1), x, y))
        .collect();
    let style = DelaunayDiagramOptions {
        labels: true,
        triangle_fill: Some(Color(68, 126, 184, 28)),
        ..Default::default()
    };
    let mut triangulated = Delaunay::new(points)?.diagram(&frame, &style)?.scene;
    note(
        &mut triangulated,
        &frame,
        "Robust triangles and adjacency · a uniform fit preserves the Euclidean metric",
    );
    output.push(("Delaunay triangles and neighbors", triangulated));
    let circle = vec![
        DelaunaySite::new("A", -1., -1.),
        DelaunaySite::new("B", 1., -1.),
        DelaunaySite::new("C", 1., 1.),
        DelaunaySite::new("D", -1., 1.),
        DelaunaySite::new("A duplicate", -1., -1.),
    ];
    let mut duplicates = Delaunay::new(circle)?.diagram(&frame, &style)?.scene;
    note(
        &mut duplicates,
        &frame,
        "Four co-circular sites · duplicate coordinates share owner A · deterministic diagonal",
    );
    output.push(("Delaunay duplicates and cocircular sites", duplicates));
    let chain = (0..6)
        .map(|i| DelaunaySite::new(format!("C{i}"), f64::from(i), f64::from(i) * 0.5))
        .collect();
    let mut collinear = Delaunay::new(chain)?.diagram(&frame, &style)?.scene;
    note(
        &mut collinear,
        &frame,
        "Exact collinear chain · adjacent links remain · no jitter or artificial triangles",
    );
    output.push(("Delaunay exact collinear chain", collinear));
    output.extend(geographic_scenes(&frame)?);
    for (_, scene) in &output {
        scene.validate()?;
    }
    Ok(output)
}
