use kartis::{motion::*, *};
fn ring(radius: f64, reverse: bool) -> Vec<glory::render::PathCommand> {
    use glory::render::PathCommand as C;
    let path = Geometry::Circle {
        x: 360.,
        y: 200.,
        radius,
    }
    .commands();
    if !reverse {
        return path;
    }
    let C::Move(start) = path[0] else {
        unreachable!()
    };
    let mut previous = start;
    let mut segments = Vec::new();
    for command in &path[1..] {
        if let C::Cubic { first, second, end } = command {
            segments.push((previous, *first, *second));
            previous = *end;
        }
    }
    let mut result = vec![C::Move(start)];
    for (end, second, first) in segments.into_iter().rev() {
        result.push(C::Cubic { first, second, end });
    }
    result.push(C::Close);
    result
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut scenes = Vec::new();
    let mut map = geographic_map_layout(
        c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            clip_angle: Some(70.),
            ..Default::default()
        },
    )?;
    let disk = map.projection.projection_disk().expect("orthographic disk");
    let mut guide = Mark::new(
        "projection-disk",
        Geometry::Circle {
            x: disk.center[0],
            y: disk.center[1],
            radius: disk.radius,
        },
        Color(243, 247, 250, 255),
    );
    guide.label.clear();
    guide.stroke = Some(glory::render::GraphicStroke {
        color: Color(175, 190, 205, 255),
        width: 1.,
        cap: glory::render::StrokeCap::Butt,
    });
    map.scene.marks.push(guide);
    for (i, (shape, position)) in [
        (GeoSymbolShape::Circle, [60., 0.]),
        (GeoSymbolShape::Diamond, [-60., 0.]),
        (GeoSymbolShape::Triangle, [0., 60.]),
        (GeoSymbolShape::Cross, [40., -40.]),
        (GeoSymbolShape::Square, [-40., -40.]),
    ]
    .into_iter()
    .enumerate()
    {
        map.add_symbols(
            &[GeoSymbol::new(format!("edge-{i}"), position, Some(100.))],
            &GeoSymbolOptions {
                shape,
                clip_to_projection: true,
                stroke: None,
                domain: Some([0., 100.]),
                radius_range: [0., 62.],
                color: GeoSymbolColor::Fixed(c.theme.color(i)),
                ..Default::default()
            },
        )?;
    }
    scenes.push(("Symbols clipped to projection", map.scene));
    let mut start = c.scene()?;
    let mut shape = Mark::new(
        "donut",
        Geometry::Path(
            ring(115., false)
                .into_iter()
                .chain(ring(35., true))
                .collect(),
        ),
        Color(28, 135, 155, 255),
    );
    shape.series = "donut".into();
    shape.label = "Explicit outer/hole contour keys".into();
    start.marks.push(shape);
    let mut target = start.clone();
    target.marks[0].geometry = Geometry::Path(
        ring(65., true)
            .into_iter()
            .chain(ring(145., false))
            .collect(),
    );
    let mut options = PropertyMotionOptions::default();
    options.geometry.contour_morphs.insert(
        "donut".into(),
        ContourMorphOptions {
            sampling: PathMorphOptions {
                samples: 96,
                alignment: ClosedAlignment::Rotate,
                ..Default::default()
            },
            contours: vec![
                MorphContour {
                    key: "outer".into(),
                    closed: true,
                    collapse: [360., 200.],
                },
                MorphContour {
                    key: "hole".into(),
                    closed: true,
                    collapse: [360., 200.],
                },
            ],
        },
    );
    let mut motion = PropertyMotion::new_with_contours(
        start.clone(),
        options,
        ContourBindings::from([("donut".into(), vec!["outer".into(), "hole".into()])]),
    )?;
    motion.retarget_with_contours(
        target.clone(),
        ContourBindings::from([("donut".into(), vec!["hole".into(), "outer".into()])]),
    )?;
    motion.tick(0.12)?;
    scenes.push(("Contour morph initial", start));
    scenes.push(("Contour morph keyed holes", motion.scene().clone()));
    scenes.push(("Contour morph exact target", target));
    let mut styled = bar(
        c,
        &[
            "Alpha".into(),
            "Beta".into(),
            "Gamma".into(),
            "Delta".into(),
        ],
        &[BarSeries::new(
            "Volume",
            vec![Some(35.), Some(62.), Some(47.), Some(80.)],
        )],
        &BarOptions {
            corner_radius: 14.,
            border: Some(glory::render::GraphicStroke {
                color: Color(22, 75, 110, 255),
                width: 1.5,
                cap: glory::render::StrokeCap::Round,
            }),
            ..Default::default()
        },
    )?;
    for mark in styled.marks.iter_mut().filter(|m| !m.series.is_empty()) {
        mark.fill_paint = Some(FillPaint::Linear {
            start: [0., c.margin.top],
            end: [0., c.height - c.margin.bottom],
            stops: vec![
                GradientStop::new(0., Color(30, 155, 170, 255)),
                GradientStop::new(1., Color(55, 90, 190, 255)),
            ],
        });
    }
    scenes.push(("Gradient rounded bars", styled));
    let mut s = c.scene()?;
    for i in 0..3 {
        let x = 150. + i as f64 * 210.;
        let mut m = Mark::new(
            format!("gradient-{i}"),
            Geometry::Circle {
                x,
                y: 190.,
                radius: 70.,
            },
            Color(50, 100, 180, 255),
        );
        m.value = (i + 1) as f64;
        m.fill_paint = Some(FillPaint::Radial {
            center: [x - 15., 170.],
            radius: 100.,
            stops: vec![
                GradientStop::new(0., Color(255, 230, 130, 255)),
                GradientStop::new(0.5, Color(230, 115, 70, 255)),
                GradientStop::new(1., Color(100, 45, 140, 255)),
            ],
        });
        s.marks.push(m);
    }
    scenes.push(("Radial gradient fills", s));
    let uv_to_geo = |u: f64, v: f64| {
        [
            1_f64.atan2(-u).to_degrees(),
            v.atan2(u.hypot(1.)).to_degrees(),
        ]
    };
    let exterior = [
        [-2., -2.],
        [2., -2.],
        [2., 2.],
        [-2., 2.],
        [-2., 1.],
        [1., 1.],
        [1., -1.],
        [-2., -1.],
    ]
    .map(|[u, v]| uv_to_geo(u, v))
    .to_vec();
    let hole = [
        [-1.8, -1.8],
        [1.8, -1.8],
        [1.8, 1.8],
        [-1.8, 1.8],
        [-1.8, 1.2],
        [1.3, 1.2],
        [1.3, -1.2],
        [-1.8, -1.2],
    ]
    .map(|[u, v]| uv_to_geo(u, v))
    .to_vec();
    let nonconvex = GeoFeature {
        id: "islands".into(),
        label: "Disconnected visible lobes and clipped holes".into(),
        value: Some(1.),
        polygons: vec![vec![exterior, hole]],
    };
    scenes.push((
        "Nonconvex horizon clipping",
        geographic_map(
            c,
            &[nonconvex],
            &GeoMapOptions {
                projection: GeoProjection::Orthographic,
                fill: Some(Color(35, 125, 160, 255)),
                border: Some(Color(15, 65, 100, 255)),
                ..Default::default()
            },
        )?,
    ));
    let cap_land = GeoFeature {
        id: "cap".into(),
        label: "Gnomonic small-circle cap".into(),
        value: Some(1.),
        polygons: vec![vec![vec![
            [-50., -40.],
            [50., -40.],
            [50., 40.],
            [-50., 40.],
        ]]],
    };
    scenes.push((
        "Gnomonic small-circle clip",
        geographic_map(
            c,
            &[cap_land],
            &GeoMapOptions {
                projection: GeoProjection::Gnomonic,
                clip_angle: Some(35.),
                fill: Some(Color(230, 145, 60, 255)),
                ..Default::default()
            },
        )?,
    ));
    let nodes = ["Source", "Process", "Reuse", "Sink"].map(FlowNode::new);
    let links = [
        ("input", "Source", "Process", 80.),
        ("output", "Process", "Sink", 50.),
        ("waste", "Process", "Reuse", 30.),
        ("recycle", "Reuse", "Process", 20.),
        ("loop", "Reuse", "Reuse", 10.),
    ]
    .map(|(id, s, t, v)| CircularFlowLink::new(id, s, t, v));
    scenes.push((
        "Circular Sankey",
        sankey_circular(c, &nodes, &links, &CircularSankeyOptions::default())?,
    ));
    let categories = (0..12).map(|i| format!("Month {i:02}")).collect::<Vec<_>>();
    let bars = [
        BarSeries::new("North", (0..12).map(|i| Some(15. + i as f64)).collect()),
        BarSeries::new("South", vec![Some(12.); 12]),
    ];
    scenes.push((
        "Category window bars",
        bar(
            c,
            &categories,
            &bars,
            &BarOptions {
                layout: BarLayout::Stacked,
                category_window: Some([0.28, 0.72]),
                category_axis: CategoryAxisOptions {
                    tick_count: 12,
                    ..Default::default()
                },
                ..Default::default()
            },
        )?,
    ));
    let mixed = [
        ComboSeries::new(
            "Bars",
            ComboKind::Bar,
            categories
                .iter()
                .enumerate()
                .map(|(i, k)| ComboPoint::new(k.as_str(), 12. + i as f64))
                .collect(),
        ),
        ComboSeries::new(
            "Line",
            ComboKind::Line,
            categories
                .iter()
                .enumerate()
                .map(|(i, k)| ComboPoint::new(k.as_str(), 20. + (i as f64).sin() * 5.))
                .collect(),
        ),
    ];
    scenes.push((
        "Category window combo",
        combo(
            c,
            &mixed,
            &ComboOptions {
                x: ComboXAxis::Categories {
                    values: categories,
                    padding: 0.15,
                },
                category_window: Some([0.28, 0.72]),
                ..Default::default()
            },
        )?,
    ));
    let land = [GeoFeature {
        id: "region".into(),
        label: "Visible region".into(),
        value: Some(1.),
        polygons: vec![vec![vec![
            [-35., -20.],
            [35., -20.],
            [35., 35.],
            [-35., 35.],
            [-35., -20.],
        ]]],
    }];
    let map_frame = ChartConfig {
        margin: Margin {
            right: 210.,
            ..c.margin
        },
        ..c.clone()
    };
    for (name, projection, shape) in [
        (
            "Azimuthal equal-area symbols",
            GeoProjection::AzimuthalEqualArea,
            GeoSymbolShape::Diamond,
        ),
        (
            "Azimuthal equidistant symbols",
            GeoProjection::AzimuthalEquidistant,
            GeoSymbolShape::Triangle,
        ),
        (
            "Stereographic cross symbols",
            GeoProjection::Stereographic,
            GeoSymbolShape::Cross,
        ),
    ] {
        let mut map = geographic_map_layout(
            &map_frame,
            &land,
            &GeoMapOptions {
                projection,
                fill: Some(Color(231, 239, 246, 255)),
                ..Default::default()
            },
        )?;
        let symbols = [
            ("West", [-25., 0.], 25.),
            ("Central", [0., 20.], 64.),
            ("East", [25., 0.], 100.),
        ]
        .map(|(id, p, v)| GeoSymbol::new(id, p, Some(v)));
        map.add_symbols(
            &symbols,
            &GeoSymbolOptions {
                shape,
                domain: Some([0., 100.]),
                radius_range: [0., 20.],
                labels: true,
                size_legend: Some(GeoSizeLegend {
                    layout: GeoLegendOptions {
                        position: [520., 70.],
                        title: "Equal symbol area".into(),
                        ..Default::default()
                    },
                    values: vec![25., 64., 100.],
                }),
                ..Default::default()
            },
        )?;
        scenes.push((name, map.scene));
    }
    let mut first = c.scene()?;
    let mut shape = Mark::new(
        "morph",
        Geometry::Rect {
            x: 120.,
            y: 110.,
            width: 140.,
            height: 100.,
        },
        Color(30, 142, 158, 255),
    );
    shape.series = "shape".into();
    shape.label = "Explicit 48-point shape correspondence".into();
    first.marks.push(shape);
    let mut target = first.clone();
    target.marks[0].geometry = Geometry::Circle {
        x: 480.,
        y: 210.,
        radius: 72.,
    };
    let mut options = PropertyMotionOptions::default();
    options.geometry.morphs.insert(
        "morph".into(),
        PathMorphOptions {
            samples: 48,
            alignment: ClosedAlignment::Rotate,
            ..Default::default()
        },
    );
    let mut motion = PropertyMotion::new(first.clone(), options)?;
    motion.retarget(target.clone())?;
    motion.tick(0.10)?;
    scenes.push(("Morph initial", first));
    scenes.push(("Morph sampled transition", motion.scene().clone()));
    scenes.push(("Morph exact target", target));
    // The example supplies explicit measurements, as a host would after font
    // layout. No character-count estimate is used as font evidence.
    let mut labels = c.scene()?;
    let candidates = (0..9)
        .map(|i| LabelCandidate {
            id: format!("city{i}"),
            rect: LayoutRect {
                x: 300. + (i % 3) as f64 * 6.,
                y: 160. + (i / 3) as f64 * 8.,
                width: 76.,
                height: 18.,
            },
            priority: 9 - i,
            offsets: vec![
                (-90., 0.),
                (0., -40.),
                (90., 0.),
                (0., 40.),
                (-90., -40.),
                (90., 40.),
            ],
        })
        .collect::<Vec<_>>();
    let placements = layout_labels(
        &candidates,
        &LabelCollisionOptions {
            viewport: LayoutRect {
                x: c.margin.left,
                y: c.margin.top,
                width: c.inner_width(),
                height: c.inner_height(),
            },
            ..Default::default()
        },
    )?;
    for (i, p) in placements.iter().enumerate() {
        if let Some(r) = p.rect {
            labels.marks.push(Mark::new(
                format!("placed-{i}"),
                Geometry::Rect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                },
                Color(226, 238, 247, 255),
            ));
            labels.labels.push(Label {
                x: r.x + 4.,
                y: r.y + 14.,
                text: format!("City {}", i + 1),
                size: 12.,
                color: c.theme.text,
                anchor: TextAnchor::Start,
                rotation: 0.,
            });
        }
    }
    scenes.push(("Measured label collision placement", labels));
    Ok(scenes)
}
