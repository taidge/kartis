//! Real role payloads wrapped without extra phase/bit fields that could bypass their equality.
use kartis::*;
#[derive(Clone, Debug, PartialEq)]
pub enum RawWidgetData {
    Bump(BumpPointWidgetData),
    Scatter(ScatterNodeWidgetData),
    Bar(BarWidgetData),
    Stream(StreamDotWidgetData),
    Heatmap(HeatmapCellWidgetData),
    Bullet(BulletPartWidgetData),
}
fn category_bits(key: &CategoryKey) -> String {
    match key {
        CategoryKey::Number(n) => n.to_bits().to_string(),
        CategoryKey::String(_) => unreachable!(),
    }
}
fn number_bits(n: Option<f64>) -> String {
    n.unwrap().to_bits().to_string()
}
impl RawWidgetData {
    pub fn kind(&self) -> u8 {
        match self {
            Self::Bump(_) => 0,
            Self::Scatter(_) => 1,
            Self::Bar(_) => 2,
            Self::Stream(_) => 3,
            Self::Heatmap(_) => 4,
            Self::Bullet(_) => 5,
        }
    }
    pub fn bits(&self) -> Vec<String> {
        match self {
            Self::Bump(d) => vec![
                category_bits(&d.point.raw_x),
                category_bits(&d.series.data[0].x),
                number_bits(d.series.data[1].rank),
            ],
            Self::Scatter(d) => vec![
                category_bits(&d.site.raw_x),
                category_bits(&d.site.raw_y),
                number_bits(d.site.size_value),
                category_bits(&d.series.data[0].x),
                category_bits(&d.series.data[0].y),
                number_bits(d.series.data[0].size_value),
            ],
            Self::Bar(d) => vec![number_bits(d.series.values[1])],
            Self::Stream(d) => vec![number_bits(d.series.values[1])],
            Self::Heatmap(d) => vec![number_bits(d.datum.value), number_bits(d.cell.value)],
            Self::Bullet(d) => vec![d.part.raw_value.to_bits().to_string()],
        }
    }
}
#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<RawWidgetData>,
}
fn place() -> Option<PlotWidgetPlacement> {
    Some(PlotWidgetPlacement::Rect(LayoutRect {
        x: 70.,
        y: 80.,
        width: 40.,
        height: 24.,
    }))
}
fn pack<T: Clone + PartialEq + 'static>(
    mut frame: SceneInteraction,
    p: PlotWidgetsData<T>,
    wrap: fn(T) -> RawWidgetData,
) -> Result<Source> {
    frame
        .interaction
        .get_or_insert_with(Default::default)
        .picking_profile = PickingProfile::PaintedGeometry;
    // Author one common compatibility scalar for zero. Original typed values
    // stay untouched; +0/-0 have identical numeric/paint meaning here.
    for mark in &mut frame.scene.marks {
        if mark.value == 0. {
            mark.value = 0.;
        }
    }
    // Keep an authored source paint plane in every renderer, even when the
    // selected Widget replaces the role's only mark. This decoration carries
    // no business data and is identical in both raw-value phases.
    frame.scene.marks.insert(
        0,
        Mark::new(
            "raw-reference",
            Geometry::Rect {
                x: 420.,
                y: 200.,
                width: 16.,
                height: 16.,
            },
            glory::style::Color(90, 100, 120, 255),
        ),
    );
    frame.validate()?;
    let widgets = PlotWidgetsData {
        layout: p.layout,
        data: p.data.into_iter().map(|(id, d)| (id, wrap(d))).collect(),
    };
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
pub fn source(kind: u8, phase: u8, revision: u64) -> Result<Source> {
    let raw = if phase.is_multiple_of(2) { -0. } else { 0. };
    let c = ChartConfig {
        width: 600.,
        height: 260.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    };
    match kind {
        0 => {
            let l = bump_layout(
                &c,
                &[BumpSeries::new(
                    "row",
                    vec![BumpDatum::new(raw, 2.), BumpDatum::new("tail", raw)],
                )],
                &BumpLayoutOptions {
                    axes: BumpAxes::none(),
                    legend: false,
                    labels: BumpEndpointLabels {
                        start: false,
                        end: false,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )?;
            let p = bump_point_widgets(&l, revision, |ctx| {
                Ok((ctx.point.source_index == 0).then(place).flatten())
            })?;
            pack(bump_widget_frame(&l)?, p, RawWidgetData::Bump)
        }
        1 => {
            let mut d = ScatterDatum::new("a", raw, raw);
            d.size_value = Some(raw);
            let data = [ScatterSeries::new("row", vec![d])];
            let axis = ScatterAxisScale::Continuous {
                options: ContinuousScaleOptions {
                    nice: ScaleNice::Off,
                    ..Default::default()
                },
                min: ScatterBound::Fixed(-1.),
                max: ScatterBound::Fixed(1.),
            };
            let l = scatter_layout(
                &c,
                &data,
                &ScatterLayoutOptions {
                    axes: ScatterAxes {
                        top: None,
                        right: None,
                        bottom: None,
                        left: None,
                    },
                    scales: ScatterScaleOptions {
                        x: axis.clone(),
                        y: axis,
                    },
                    ..Default::default()
                },
            )?;
            let p = scatter_node_widgets(&l, &data, revision, |_| Ok(place()))?;
            pack(l.into(), p, RawWidgetData::Scatter)
        }
        2 => {
            let mut o = BarOptions {
                value_domain: Some([-1., 4.]),
                ..Default::default()
            };
            o.value_axis.visible = false;
            o.value_axis.grid = false;
            o.category_axis.visible = false;
            o.category_axis.grid = false;
            let l = bar_layout_with_factories(
                &c,
                &["a".into(), "z".into()],
                &[BarSeries::new("row", vec![Some(3.), Some(raw)])],
                &o,
                &BarStyleOptions::default(),
                BarFactories::default(),
            )?;
            let p = bar_widgets(&l, revision, |ctx| {
                Ok((ctx.bar.source_category == 0).then(place).flatten())
            })?;
            pack(bar_widget_frame(&l)?, p, RawWidgetData::Bar)
        }
        3 => {
            let mut o = StreamLayoutOptions {
                curve: Curve::Linear,
                dots: Some(StreamDotOptions::default()),
                stack: kartis::stack::StackOptions {
                    offset: kartis::stack::StackOffset::Zero,
                    ..Default::default()
                },
                y_bounds: [Some(0.), Some(5.)],
                legend: false,
                ..Default::default()
            };
            o.axes.x.visible = false;
            o.axes.x.grid = false;
            o.axes.y.visible = false;
            o.axes.y.grid = false;
            let l = stream_layout(
                &c,
                &["a".into(), "z".into()],
                &[BarSeries::new("row", vec![Some(3.), Some(raw)])],
                &o,
            )?;
            let p = stream_dot_widgets(&l, revision, |ctx| {
                Ok((ctx.sample.source_index == 0).then(place).flatten())
            })?;
            pack(l.into_scene_interaction()?, p, RawWidgetData::Stream)
        }
        4 => {
            let l = heatmap_layout(
                &c,
                &[
                    HeatCell::new("row", "a", raw),
                    HeatCell::new("row", "z", 3.),
                ],
                &HeatmapLayoutOptions {
                    labels: false,
                    axes: HeatmapAxes::none(),
                    ..Default::default()
                },
            )?;
            let p = heatmap_cell_widgets(&l, revision, |ctx| {
                Ok((ctx.cell.key.column == "a").then(place).flatten())
            })?;
            pack(
                l.into_painted_scene_interaction()?,
                p,
                RawWidgetData::Heatmap,
            )
        }
        5 => {
            let mut o = BulletLayoutOptions {
                min: Some(-1.),
                max: Some(1.),
                legend: false,
                ..Default::default()
            };
            o.axis.visible = false;
            o.axis.grid = false;
            o.titles.insert("row".into(), String::new());
            let l = bullet_layout(
                &c,
                &[BulletDatum {
                    id: "row".into(),
                    ranges: vec![raw],
                    measures: vec![],
                    markers: vec![],
                }],
                &o,
            )?;
            let p = bullet_part_widgets(&l, revision, |ctx| {
                Ok(ctx.part.source_index.is_some().then(place).flatten())
            })?;
            pack(
                SceneInteraction {
                    scene: l.scene,
                    interaction: None,
                    plot: Some(ClipRect::plot(&c)),
                },
                p,
                RawWidgetData::Bullet,
            )
        }
        _ => Err(ChartError("unknown raw-bit role".into())),
    }
}
