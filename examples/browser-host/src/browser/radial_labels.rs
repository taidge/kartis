use super::*;
pub(super) fn source(kind: &str, phase: u8, c: &ChartConfig) -> Result<(SceneInteraction, Value)> {
    let rgba = |Color(r, g, b, a)| [r, g, b, a];
    match kind {
        "pie-radius-labels" => {
            let data = [
                RadialDatum::new("Alpha", 40. + f64::from(phase) * 8.),
                RadialDatum::new("Beta", 60.),
            ];
            let mut options = PieLayoutOptions::default();
            options.pie.inner_radius = if phase == 0 { 0.92 } else { 0.5 };
            options.pie.pad_angle = 0.05;
            options.pie.style.corner_radius = 5.;
            options.pie.style.arc_labels.radius_measure = ArcLabelRadiusMeasure::RingThickness;
            options.pie.style.arc_labels.skip_radius = 32.;
            options.pie.style.arc_labels.legacy_fit = false;
            let layout = pie_layout(c, &data, &options)?;
            let mut legacy = options.clone();
            legacy.pie.style.arc_labels.radius_measure = ArcLabelRadiusMeasure::OuterRadius;
            let legacy = pie_layout(c, &data, &legacy)?;
            let slice = &layout.slices[0];
            let mark = &layout.scene.marks[slice.mark_index.unwrap()];
            let angle = slice.start_angle + (slice.end_angle - slice.start_angle) * 0.3;
            let r = (slice.inner_radius + slice.outer_radius) * 0.5;
            let point = [
                layout.center[0] + r * angle.sin(),
                layout.center[1] - r * angle.cos(),
            ];
            let target = json!({"id":mark.id,"value":mark.value,"point":point,"color":rgba(mark.fill.unwrap()),"control":format!("{}: {}",mark.label,mark.value),
                "metadata":{"labels":layout.scene.labels.len(),"legacyLabels":legacy.scene.labels.len(),"thickness":slice.outer_radius-slice.inner_radius,"skip":32.,"beforeLabels":0,"afterLabels":2}});
            Ok((layout.into_scene_interaction()?, target))
        }
        "radial-radius-labels" => {
            let categories = ["Q1".into(), "Q2".into()];
            let data = [
                BarSeries::new("North", vec![Some(30. + f64::from(phase) * 8.), Some(30.)]),
                BarSeries::new("South", vec![Some(25.), Some(35.)]),
            ];
            let options = StackedRadialBarOptions {
                inner_radius: if phase == 0 { 0.88 } else { 0.1 },
                max_value: Some(100.),
                pad_angle: 0.04,
                labels: Some(RadialBarLabels {
                    radius_measure: ArcLabelRadiusMeasure::RingThickness,
                    skip_radius: 20.,
                    ..Default::default()
                }),
                axes: PolarAxesOptions {
                    radial_start: None,
                    circular_outer: None,
                    ..Default::default()
                },
                ..Default::default()
            };
            let style = StackedRadialArcStyle {
                corner_radius: 5.,
                ..Default::default()
            };
            let layout =
                layout_stacked_radial_bar_with_arc_style(c, &categories, &data, &options, &style)?;
            let mut legacy = options.clone();
            legacy.labels.as_mut().unwrap().radius_measure = ArcLabelRadiusMeasure::OuterRadius;
            let legacy =
                layout_stacked_radial_bar_with_arc_style(c, &categories, &data, &legacy, &style)?;
            let segment = &layout.segments[0];
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == segment.mark_id)
                .unwrap();
            let p = layout.frame.point(
                (segment.inner_radius + segment.outer_radius) * 0.5,
                segment.angles[0] + (segment.angles[1] - segment.angles[0]) * 0.3,
            );
            let count = |scene: &Scene| {
                scene
                    .label_series
                    .values()
                    .filter(|id| categories.contains(id))
                    .count()
            };
            let target = json!({"id":mark.id,"value":mark.value,"point":[p[0],p[1]],"color":rgba(mark.fill.unwrap()),"control":format!("{}: {}",mark.label,mark.value),
                "metadata":{"labels":count(&layout.scene),"legacyLabels":count(&legacy.scene),"thickness":segment.outer_radius-segment.inner_radius,"skip":20.,"beforeLabels":0,"afterLabels":4}});
            Ok((layout.into_scene_interaction()?, target))
        }
        "sunburst-labels" | "sunburst-masked-labels" => {
            let masked = kind == "sunburst-masked-labels";
            let mut data = vec![HierarchyDatum::new("root", None, 0.)];
            for (id, value) in [
                ("a", 24. + f64::from(phase) * 8.),
                ("b", 18.),
                ("c", 16.),
                ("d", 14.),
            ] {
                let mut datum = HierarchyDatum::new(id, Some("root".into()), value);
                datum.label = if masked {
                    format!("A long measured label for {id}")
                } else {
                    format!("Label {id}")
                };
                data.push(datum);
            }
            let layout = sunburst_layout(
                c,
                &data,
                &SunburstLayoutOptions {
                    pad_angle: 0.04,
                    corner_radius: 5.,
                    border_width: 0.,
                    labels: Some(SunburstLabelOptions {
                        text: SunburstLabelText::Label,
                        size: Some(18.),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )?
            .with_label_rotation(18.)?;
            let candidates = layout.label_candidates();
            let labels = candidates
                .iter()
                .map(|c| c.label.clone())
                .collect::<Vec<_>>();
            let bounds = super::flow_layouts::measure_browser_labels(&labels)?;
            let measurements = candidates
                .iter()
                .zip(&bounds)
                .map(|(c, b)| c.measured(*b))
                .collect::<Vec<_>>();
            let first = candidates.first().unwrap();
            let first_id = first.id.clone();
            let mut options = SunburstLabelPlacementOptions::for_layout(&layout);
            options.clip = if masked {
                SunburstLabelClip::ClipToArc
            } else {
                SunburstLabelClip::Plot
            };
            if !masked {
                options.offsets = vec![(0., 40.), (0., -40.), (60., 0.), (-60., 0.)];
                options.collision.forbidden.push(LayoutRect {
                    x: first.label.x + bounds[0].x,
                    y: first.label.y + bounds[0].y,
                    ..bounds[0]
                });
                options.priorities.insert(first.node_id.clone(), 100);
                options
                    .hidden
                    .insert(candidates.last().unwrap().node_id.clone());
            }
            let first_bounds = LayoutRect {
                x: first.label.x + bounds[0].x,
                y: first.label.y + bounds[0].y,
                ..bounds[0]
            };
            let count = candidates.len();
            let layout = layout.place_labels(&measurements, &options)?;
            let placements = layout.label_placements();
            let boxes = placements
                .iter()
                .filter_map(|p| p.bounds)
                .map(|b| [b.x, b.y, b.width, b.height])
                .collect::<Vec<_>>();
            let moved = placements
                .iter()
                .find(|p| p.id == first_id)
                .and_then(|p| p.bounds)
                .is_some_and(|b| {
                    (b.x - first_bounds.x).abs() > 1e-6 || (b.y - first_bounds.y).abs() > 1e-6
                });
            let node = layout.node("a").unwrap();
            let arc = node.arc;
            let r = (arc.inner_radius + arc.outer_radius) * 0.5;
            let point = layout
                .frame
                .point(r, arc.angles[0] + (arc.angles[1] - arc.angles[0]) * 0.25);
            let mark = &layout.scene.marks[node.mark_index.unwrap()];
            let target = json!({"id":mark.id,"value":mark.value,"point":point,"color":rgba(mark.fill.unwrap()),"control":format!("{}: {}",mark.label,mark.value),
                "metadata":{"boxes":boxes,"candidates":count,"placed":boxes.len(),"moved":moved,"masked":layout.scene.label_clips.len(),"maskQueries":layout.label_placement_stats().mask_queries}});
            Ok((layout.into_scene_interaction()?, target))
        }
        _ => Err(ChartError(format!("unknown layout model fixture {kind}"))),
    }
}
