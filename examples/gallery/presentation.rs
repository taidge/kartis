//! Public presentation APIs: actual geometry, no SVG-only decoration overrides.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::mesh::PointMesh;
use kartis::*;

fn caption(scene: &mut Scene, c: &ChartConfig, text: &str) {
    scene.labels.push(Label {
        x: c.margin.left,
        y: 20.,
        text: text.into(),
        color: Color(82, 105, 125, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
}
fn center_text(scene: &mut Scene, c: &ChartConfig, heading: &str, detail: &str) {
    let x = c.margin.left + c.inner_width() / 2.;
    let y = c.margin.top + c.inner_height() / 2.;
    for (text, dy, size, color) in [
        (heading, -2., 16., Color(40, 60, 80, 255)),
        (detail, 19., 11., Color(105, 120, 135, 255)),
    ] {
        scene.labels.push(Label {
            x,
            y: y + dy,
            text: text.into(),
            color,
            size,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
    }
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut output = Vec::with_capacity(11);
    let pie_frame = ChartConfig {
        margin: Margin {
            left: 155.,
            right: 155.,
            top: 55.,
            bottom: 55.,
        },
        ..c.clone()
    };
    let traffic = [
        ("Search", 42.),
        ("Direct", 26.),
        ("Partners", 18.),
        ("Social", 10.),
        ("Email", 4.),
    ]
    .map(|(id, value)| RadialDatum::new(id, value));
    let pie_style = PieStyleOptions {
        corner_radius: 10.,
        border: Some(GraphicStroke {
            color: Color(255, 255, 255, 255),
            width: 1.5,
            cap: StrokeCap::Round,
        }),
        arc_labels: PieArcLabels {
            text: PieLabelText::Percent(ValueFormat::Percent(0)),
            skip_angle: 0.35,
            legacy_fit: false,
            style: PieTextStyle {
                color: Some(Color(255, 255, 255, 255)),
                size: Some(13.),
                ..Default::default()
            },
            ..Default::default()
        },
        link_labels: PieLinkLabels {
            enabled: true,
            text: PieLabelText::Label,
            skip_angle: 0.3,
            offset: 4.,
            diagonal_length: 14.,
            straight_length: 18.,
            color: PieLinkColor::Slice,
            thickness: 1.5,
            style: PieTextStyle {
                size: Some(12.),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut options = PieOptions {
        inner_radius: 0.42,
        pad_angle: 0.04,
        style: pie_style,
        ..Default::default()
    };
    let mut rounded = pie(&pie_frame, &traffic, &options)?;
    center_text(&mut rounded, &pie_frame, "Traffic", "rounded slice corners");
    output.push(("Rounded pie with link labels", rounded));
    options.inner_radius = 0.56;
    options.style.active_ids = BTreeSet::from(["Partners".into()]);
    options.style.active_outer_radius_offset = 16.;
    options.style.active_inner_radius_offset = 10.;
    options.style.arc_labels.text = PieLabelText::Value(ValueFormat::Fixed(0));
    let mut active = pie(&pie_frame, &traffic, &options)?;
    center_text(&mut active, &pie_frame, "Partners", "active ring expands");
    output.push(("Active donut radius offsets", active));
    options.style.active_ids.clear();
    options.style.corner_radius = 8.;
    options.inner_radius = 0.62;
    options.start_angle = PI;
    options.end_angle = -PI;
    options.style.arc_labels.text = PieLabelText::Percent(ValueFormat::Percent(0));
    let mut counterclockwise = pie(&pie_frame, &traffic, &options)?;
    center_text(
        &mut counterclockwise,
        &pie_frame,
        "Counterclockwise",
        "start π · end −π",
    );
    output.push(("Counterclockwise rounded pie", counterclockwise));

    let radar_frame = ChartConfig {
        margin: Margin {
            left: 95.,
            right: 95.,
            top: 55.,
            bottom: 50.,
        },
        ..c.clone()
    };
    let axes = ["Speed", "Quality", "Coverage", "Efficiency", "Uptime"].map(str::to_string);
    let radar_data = [
        RadarSeries::new("Product A", vec![80., 65., 90., 55., 75.]),
        RadarSeries::new("Product B", vec![50., 85., 65., 80., 60.]),
    ];
    let mut radar = RadarLayoutOptions {
        domain: Some([0., 100.]),
        curve: Curve::CatmullRomClosed { alpha: 0.5 },
        ..Default::default()
    };
    radar.grid.shape = RadarGridShape::Polygon;
    radar.grid.values = Some(vec![25., 50., 75., 100.]);
    radar.axis_label_style.size = Some(13.);
    radar.axis_label_offset = 19.;
    radar.series.fill_opacity = 0.16;
    radar.series.border.width = 3.;
    radar.points.size = 9.;
    radar.points.shape = RadarPointShape::Diamond;
    radar.points.border.enabled = true;
    radar.points.border.width = 1.5;
    radar.points.fill = Some(Color(255, 255, 255, 255));
    radar.series_styles.insert(
        "Product A".into(),
        RadarSeriesStyle {
            fill_opacity: 0.16,
            border: RadarLineStyle {
                width: 3.,
                ..Default::default()
            },
            points: Some(RadarPointOptions {
                labels: true,
                value_format: ValueFormat::Fixed(0),
                label_offset: [0., -12.],
                size: 8.,
                shape: RadarPointShape::Diamond,
                label_style: RadarTextStyle {
                    size: Some(11.),
                    color: Some(c.theme.color(0)),
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    let mut smooth = radar_with_options(&radar_frame, &axes, &radar_data, &radar)?;
    caption(
        &mut smooth,
        &radar_frame,
        "Periodic Catmull–Rom · independent fill and border alpha",
    );
    output.push(("Smooth radar with point labels", smooth));
    radar.curve = Curve::BasisClosed;
    radar.grid.shape = RadarGridShape::Circle;
    radar.grid.tick_labels = true;
    radar.grid.tick_label_axis = 2;
    radar.grid.tick_style.size = Some(10.);
    radar.grid.tick_style.color = Some(Color(110, 125, 140, 255));
    radar.series_styles.clear();
    radar.points.shape = RadarPointShape::Cross;
    radar.points.size = 9.;
    radar.series.border.dash = Some(StrokeDash {
        segments: vec![9., 4.],
        offset: 0.,
    });
    radar.rotation = TAU / 20.;
    let mut circular = radar_with_options(&radar_frame, &axes, &radar_data, &radar)?;
    caption(
        &mut circular,
        &radar_frame,
        "Periodic basis · circular grid · equal-area cross symbols",
    );
    output.push(("Radar circular grid and symbols", circular));

    let flow_frame = ChartConfig {
        margin: Margin {
            left: 95.,
            right: 95.,
            top: 55.,
            bottom: 55.,
        },
        ..c.clone()
    };
    let nodes = ["Incoming", "Design", "Build", "Validate", "Release"].map(FlowNode::new);
    let base_links = [
        ("plan", "Incoming", "Design", 60.),
        ("execute", "Incoming", "Build", 40.),
        ("planned", "Design", "Validate", 60.),
        ("built", "Build", "Validate", 40.),
        ("release", "Validate", "Release", 100.),
    ]
    .map(|(id, source, target, value)| SankeyStyleLink::new(id, source, target, value))
    .to_vec();
    let cyclic_links = base_links
        .iter()
        .cloned()
        .chain([
            SankeyStyleLink::new("feedback", "Release", "Incoming", 10.),
            SankeyStyleLink::new("revision", "Validate", "Design", 8.),
            SankeyStyleLink::new("retry", "Build", "Build", 5.),
        ])
        .collect::<Vec<_>>();
    for (vertical, dag_name, circular_name) in [
        (
            false,
            "Styled Sankey horizontal",
            "Styled circular Sankey horizontal",
        ),
        (
            true,
            "Styled Sankey vertical",
            "Styled circular Sankey vertical",
        ),
    ] {
        let mut style = SankeyStyleOptions {
            orientation: if vertical {
                SankeyOrientation::Vertical
            } else {
                SankeyOrientation::Horizontal
            },
            node: SankeyNodeStyle {
                corner_radius: 5.,
                border: Some(GraphicStroke {
                    color: Color(255, 255, 255, 255),
                    width: 1.,
                    cap: StrokeCap::Butt,
                }),
                ..Default::default()
            },
            link: SankeyLinkStyle {
                color: SankeyLinkColor::Gradient,
                opacity: Some(0.45),
            },
            label: SankeyLabelStyle {
                position: if vertical {
                    SankeyLabelPosition::Center
                } else {
                    SankeyLabelPosition::Outside
                },
                color: if vertical {
                    Some(Color(255, 255, 255, 255))
                } else {
                    None
                },
                size: Some(12.),
                padding: 10.,
                ..Default::default()
            },
            forward_curvature: 0.65,
            ..Default::default()
        };
        let layout = SankeyLayoutOptions {
            appearance: SankeyOptions {
                node_width: 19.,
                node_padding: 20.,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut dag = sankey_with_style(&flow_frame, &nodes, &base_links, &layout, &style)?.scene;
        caption(
            &mut dag,
            &flow_frame,
            "Source → target gradients · rounded node bodies",
        );
        output.push((dag_name, dag));
        style.links = BTreeMap::from([
            (
                "feedback".into(),
                SankeyLinkStyle {
                    color: SankeyLinkColor::Fixed(Color(200, 75, 85, 255)),
                    opacity: Some(0.8),
                },
            ),
            (
                "retry".into(),
                SankeyLinkStyle {
                    color: SankeyLinkColor::Fixed(Color(215, 145, 50, 255)),
                    opacity: Some(0.8),
                },
            ),
        ]);
        let circular_layout = CircularSankeyOptions {
            appearance: SankeyOptions {
                node_width: 19.,
                node_padding: 18.,
                ..Default::default()
            },
            route_padding: 5.,
            ..Default::default()
        };
        let mut cycles = circular_sankey_with_style(
            &flow_frame,
            &nodes,
            &cyclic_links,
            &circular_layout,
            &style,
        )?
        .scene;
        caption(
            &mut cycles,
            &flow_frame,
            "Feedback and self-link ribbons retain true flow widths",
        );
        output.push((circular_name, cycles));
    }

    let observed = [
        Some(28.),
        Some(48.),
        Some(40.),
        Some(76.),
        None,
        Some(52.),
        Some(68.),
        Some(60.),
        Some(84.),
    ];
    let data = [
        Series::new(
            "Observed",
            observed
                .into_iter()
                .enumerate()
                .map(|(i, y)| Point { x: i as f64, y })
                .collect(),
        ),
        Series::new(
            "Reference",
            [20., 30., 35., 32., 40., 45., 42., 50., 55.]
                .into_iter()
                .enumerate()
                .map(|(i, y)| Point::new(i as f64, y))
                .collect(),
        ),
    ];
    let line_frame = ChartConfig {
        margin: Margin {
            left: 64.,
            right: 35.,
            top: 48.,
            bottom: 50.,
        },
        ..c.clone()
    };
    let mut axes = CartesianAxes::default();
    axes.x.tick_count = 9;
    axes.x.format = ValueFormat::Fixed(0);
    axes.y.format = ValueFormat::Fixed(0);
    let layout = LineOptions {
        curve: Curve::CatmullRom { alpha: 0.5 },
        area: true,
        x_domain: Some([0., 8.]),
        y_domain: Some([0., 100.]),
        axes: axes.clone(),
        ..Default::default()
    };
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            width: 3.5,
            point_radius: 4.5,
            point_color: Some(Color(255, 255, 255, 255)),
            point_border: Some(GraphicStroke {
                color: c.theme.color(0),
                width: 2.,
                cap: StrokeCap::Round,
            }),
            point_labels: Some(LinePointLabels {
                text: LinePointLabelText::Value(ValueFormat::Fixed(0)),
                offset: [0., -13.],
                color: Some(c.theme.color(0)),
                ..Default::default()
            }),
            area_opacity: 0.6,
            area_paint: Some(FillPaint::Linear {
                start: [0., line_frame.margin.top],
                end: [0., line_frame.height - line_frame.margin.bottom],
                stops: vec![
                    GradientStop::new(0., Color(78, 121, 167, 210)),
                    GradientStop::new(1., Color(78, 121, 167, 5)),
                ],
            }),
            ..Default::default()
        },
        series_styles: BTreeMap::from([(
            "Reference".into(),
            LineSeriesStyle {
                color: Some(c.theme.color(1)),
                width: 2.5,
                dash: Some(StrokeDash {
                    segments: vec![10., 5.],
                    offset: 0.,
                }),
                points: Some(false),
                area_opacity: 0.,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    let mut area = line_with_style(&line_frame, &data, &layout, &style)?;
    caption(
        &mut area,
        &line_frame,
        "Gradient area · missing interval preserved · dashed reference",
    );
    output.push(("Styled line and area", area));
    let mut invisible = line_with_style(
        &line_frame,
        &data,
        &LineOptions {
            area: false,
            points: false,
            curve: Curve::MonotoneX,
            ..layout
        },
        &LineStyleOptions {
            interactive_sites: true,
            connect_missing: true,
            series: LineSeriesStyle {
                points: Some(false),
                width: 3.,
                ..Default::default()
            },
            series_styles: BTreeMap::from([(
                "Reference".into(),
                LineSeriesStyle {
                    color: Some(c.theme.color(1)),
                    points: Some(false),
                    dash: Some(StrokeDash {
                        segments: vec![7., 5.],
                        offset: 0.,
                    }),
                    ..Default::default()
                },
            )]),
        },
    )?;
    let query = [
        line_frame.margin.left + line_frame.inner_width() * 0.44,
        line_frame.margin.top + line_frame.inner_height() * 0.48,
    ];
    let mesh = PointMesh::from_scene(&invisible)?;
    if let Some(nearest) = mesh.nearest(query[0], query[1], 1000.) {
        let id = nearest.id.clone();
        let site = [nearest.x, nearest.y];
        let datum = invisible
            .marks
            .iter()
            .find(|m| m.id == id)
            .expect("mesh IDs belong to the source Scene");
        let text = format!("Nearest: {} ({:.0})", datum.series, datum.value);
        let mut connector = Mark::new(
            "gallery-nearest-connector",
            Geometry::Path(vec![
                C::Move(P::new(query[0], query[1])),
                C::Line(P::new(site[0], site[1])),
            ]),
            c.theme.text,
        );
        connector.fill = None;
        connector.label.clear();
        connector.stroke = Some(GraphicStroke {
            color: Color(120, 140, 155, 255),
            width: 1.,
            cap: StrokeCap::Butt,
        });
        connector.stroke_dash = Some(StrokeDash {
            segments: vec![3., 3.],
            offset: 0.,
        });
        invisible.marks.push(connector);
        for (id, p, r, color) in [
            ("gallery-nearest-query", query, 4., Color(65, 80, 95, 255)),
            ("gallery-nearest-found", site, 8., Color(60, 150, 115, 255)),
        ] {
            let mut m = Mark::new(
                id,
                Geometry::Circle {
                    x: p[0],
                    y: p[1],
                    radius: r,
                },
                Color(255, 255, 255, 255),
            );
            m.label.clear();
            m.stroke = Some(GraphicStroke {
                color,
                width: 2.,
                cap: StrokeCap::Round,
            });
            invisible.marks.push(m);
        }
        invisible.labels.push(Label {
            x: query[0] - 10.,
            y: query[1] - 15.,
            text,
            color: Color(45, 110, 85, 255),
            size: 12.,
            anchor: TextAnchor::End,
            rotation: 0.,
        });
    }
    caption(
        &mut invisible,
        &line_frame,
        "Hidden dots retain Rust nearest/slice sites · sample query shown",
    );
    output.push(("Line invisible interaction sites", invisible));
    for (_, scene) in &output {
        scene.validate()?;
    }
    Ok(output)
}
