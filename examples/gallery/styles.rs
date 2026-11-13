use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut result = Vec::new();
    let mut patterns = c.scene()?;
    for (i, shape) in [
        PatternShape::Dots {
            diameter: 7.,
            stagger: true,
        },
        PatternShape::Squares {
            size: 6.,
            stagger: true,
        },
        PatternShape::Lines { width: 3. },
    ]
    .into_iter()
    .enumerate()
    {
        let mut mark = Mark::new(
            format!("pattern-{i}"),
            Geometry::Rect {
                x: 50. + i as f64 * 220.,
                y: 65.,
                width: 190.,
                height: 265.,
            },
            c.theme.color(i),
        );
        mark.fill_paint = Some(FillPaint::Pattern(PatternFill {
            shape,
            spacing: 13.25,
            rotation: 25.,
            offset: [3.5, -2.25],
            foreground: c.theme.color(i),
            background: Color(240, 244, 248, 255),
        }));
        patterns.marks.push(mark);
    }
    result.push(("Repeating pattern fills", patterns));
    let mut dashed = c.scene()?;
    for (i, cap) in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square]
        .into_iter()
        .enumerate()
    {
        let mut mark = Mark::new(
            format!("dash-{i}"),
            Geometry::Path(vec![
                C::Move(P::new(70., 95. + i as f64 * 100.)),
                C::Cubic {
                    first: P::new(220., 10. + i as f64 * 100.),
                    second: P::new(450., 170. + i as f64 * 100.),
                    end: P::new(650., 75. + i as f64 * 100.),
                },
            ]),
            c.theme.color(i),
        );
        mark.fill = None;
        mark.stroke = Some(GraphicStroke {
            color: c.theme.color(i),
            width: 9.,
            cap,
        });
        mark.stroke_dash = Some(StrokeDash {
            segments: if i == 2 {
                vec![0., 22.]
            } else {
                vec![20., 13., 5.]
            },
            offset: -8.,
        });
        dashed.marks.push(mark);
    }
    result.push(("Dashed curves and cap shapes", dashed));
    let mut masked = c.scene()?;
    let mut path = Geometry::Circle {
        x: 360.,
        y: 200.,
        radius: 145.,
    }
    .commands();
    path.extend(
        Geometry::Circle {
            x: 360.,
            y: 200.,
            radius: 70.,
        }
        .commands(),
    );
    let clip = ClipPath {
        geometry: Geometry::Path(path),
        rule: ClipRule::EvenOdd,
    };
    let mut m = Mark::new(
        "masked-stroke",
        Geometry::Rect {
            x: 100.,
            y: 40.,
            width: 480.,
            height: 270.,
        },
        c.theme.color(0),
    );
    m.clip_path = Some(Box::new(clip.clone()));
    m.stroke = Some(GraphicStroke {
        color: c.theme.color(1),
        width: 35.,
        cap: StrokeCap::Butt,
    });
    masked.marks.push(m);
    masked.label_clips.insert(0, clip);
    masked.labels.push(Label {
        x: 360.,
        y: 195.,
        text: "Clipped text across a true hole".into(),
        color: Color(255, 255, 255, 255),
        size: 32.,
        anchor: TextAnchor::Middle,
        rotation: -20.,
    });
    result.push(("Even-odd path masks", masked));
    let opts = ComboOptions {
        x: ComboXAxis::Categories {
            values: ["A", "B", "C", "D"].map(str::to_owned).to_vec(),
            padding: 0.15,
        },
        ..Default::default()
    };
    let boxes = ComboSeries::new(
        "Distribution",
        ComboKind::BoxPlot,
        ["A", "B", "C", "D"]
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let v = i as f64 * 5.;
                ComboPoint::box_plot(x, [v + 5., v + 15., v + 25., v + 40., v + 60.])
            })
            .collect(),
    );
    let mut line = ComboSeries::new(
        "Target",
        ComboKind::Line,
        ["A", "B", "C", "D"]
            .into_iter()
            .map(|x| ComboPoint::new(x, 45.))
            .collect(),
    );
    line.style.line_width = 4.;
    result.push((
        "Box plot and line combination",
        combo(c, &[boxes, line], &opts)?,
    ));
    let mut range = ComboSeries::new(
        "Interval",
        ComboKind::RangeBand,
        vec![
            ComboPoint::range("A", 10., 40.),
            ComboPoint::range("B", 15., 55.),
            ComboPoint::range("D", 30., 65.),
        ],
    );
    range.style.area_opacity = 0.22;
    let mut errors = ComboSeries::new(
        "Estimate",
        ComboKind::ErrorBars,
        vec![
            ComboPoint::new("A", 25.).with_bounds(12., 35.),
            ComboPoint::new("B", 35.).with_bounds(20., 45.),
            ComboPoint::new("D", 50.).with_bounds(40., 60.),
        ],
    );
    errors.style.error_cap_width = 18.;
    errors.style.point_labels = Some(ComboPointLabels::default());
    result.push((
        "Range band and error bars",
        combo(c, &[range, errors], &opts)?,
    ));
    let mut geo = geographic_map_layout(
        c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            clip_angle: Some(65.),
            ..Default::default()
        },
    )?;
    geo.add_symbols(
        &[GeoSymbol::new("edge", [60., 0.], Some(1.))],
        &GeoSymbolOptions {
            clip_to_projection: true,
            clip_labels_to_projection: true,
            radius_range: [85., 85.],
            stroke: Some(GraphicStroke {
                color: c.theme.color(1),
                width: 16.,
                cap: StrokeCap::Round,
            }),
            ..Default::default()
        },
    )?;
    result.push(("Projection mask with symbol stroke", geo.scene));
    Ok(result)
}
