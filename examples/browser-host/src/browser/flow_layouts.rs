use super::*;

pub(super) fn source(kind: &str, phase: u8, c: &ChartConfig) -> Result<(SceneInteraction, Value)> {
    let rgba = |Color(r, g, b, a)| [r, g, b, a];
    match kind {
        "hierarchy-labels" => {
            let mut data = vec![HierarchyDatum::new("root", None, 0.)];
            for (id, value) in [
                ("a", 10. + f64::from(phase) * 8.),
                ("b", 14.),
                ("c", 12.),
                ("d", 9.),
            ] {
                let mut datum = HierarchyDatum::new(id, Some("root".into()), value);
                datum.label = format!("Measured label {id}");
                data.push(datum);
            }
            let layout = circle_packing_layout(
                c,
                &data,
                None,
                &CirclePackingLayoutOptions {
                    style: HierarchyStyleOptions {
                        leaf_labels: HierarchyLabelStyle {
                            clip_to_node: false,
                            rotation: 12.,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )?;
            let measured = measure_hierarchy_labels(&layout)?;
            let first = &layout.label_candidates()[0];
            let original = &measured[0].bounds;
            let mut options = HierarchyLabelPlacementOptions::for_layout(&layout);
            options.offsets = vec![(0., -40.), (0., 40.), (-60., 0.), (60., 0.)];
            options.collision.forbidden.push(LayoutRect {
                x: first.label.x + original.x,
                y: first.label.y + original.y,
                ..*original
            });
            options.priorities.insert(first.node_id.clone(), 100);
            let hidden = layout.label_candidates().last().unwrap().node_id.clone();
            options.hidden.insert(hidden.clone());
            let layout = layout.place_labels(&measured, &options)?;
            let placements = layout.label_placements();
            let boxes = placements
                .iter()
                .filter_map(|p| p.bounds)
                .map(|r| [r.x, r.y, r.width, r.height])
                .collect::<Vec<_>>();
            let moved = placements
                .iter()
                .find(|p| p.id == first.id)
                .and_then(|p| p.bounds)
                .is_some_and(|b| {
                    (b.x - first.label.x - original.x).abs() > 1e-6
                        || (b.y - first.label.y - original.y).abs() > 1e-6
                });
            let node = layout.node("a").unwrap();
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == node.mark_id)
                .unwrap();
            let Geometry::Circle { x, y, .. } = mark.geometry else {
                unreachable!()
            };
            let target = json!({"id":mark.id,"value":mark.value,"point":[x,y+18.],"color":rgba(mark.fill.unwrap()),
                "control":format!("{}: {}",mark.label,mark.value),"metadata":{"boxes":boxes,"moved":moved,
                    "hidden":hidden,"hiddenOmitted":placements.iter().find(|p|p.node_id==hidden).unwrap().bounds.is_none(),
                    "candidates":measured.len(),"placed":boxes.len()}});
            Ok((layout.into_scene_interaction()?, target))
        }
        "chord-ribbon" | "chord-rim" => {
            let value = if phase == 0 { 4. } else { 12. };
            let layout = chord_layout(
                c,
                &["A".into(), "B".into()],
                &[vec![0., value], vec![9., 0.]],
                &ChordLayoutOptions {
                    pad_angle: 0.06,
                    inner_radius_ratio: 0.82,
                    inner_radius_offset: 0.08,
                    ribbon_style: ChordPaintOptions {
                        opacity: 1.,
                        ..Default::default()
                    },
                    colors_by_id: BTreeMap::from([
                        ("A".into(), Color(34, 109, 154, 255)),
                        ("B".into(), Color(204, 91, 41, 255)),
                    ]),
                    labels: None,
                    ..Default::default()
                },
            )?;
            let ribbon = &layout.ribbons[0];
            let (id, point, value, color) = if kind == "chord-ribbon" {
                (
                    ribbon.mark_id.clone(),
                    ribbon.anchor,
                    ribbon.value,
                    ribbon.source.color,
                )
            } else {
                let group = layout.group("A").unwrap();
                (
                    group.mark_id.clone(),
                    group.anchor,
                    group.value,
                    group.color,
                )
            };
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == id)
                .unwrap();
            let target = json!({"id":id,"value":value,"point":point,"color":rgba(color),
                "control":format!("{}: {}",mark.label,mark.value),
                "metadata":{"source":ribbon.source.id,"target":ribbon.target.id,
                    "sourceValue":ribbon.source.value,"targetValue":ribbon.target.value,
                    "tooltip":layout.tooltip_text(&ribbon.mark_id,&TooltipOptions::default())?}});
            Ok((layout.into_scene_interaction()?, target))
        }
        "bump" | "bump-nearest" => {
            let rank = if phase == 0 { 4. } else { 10. };
            let data = [
                BumpSeries::new(
                    "Alpha",
                    vec![
                        BumpDatum::new(1., 1.),
                        BumpDatum::new("1", rank),
                        BumpDatum::new("Q3", 10.),
                        BumpDatum::new("Q4", 4.),
                    ],
                ),
                BumpSeries::new(
                    "Beta",
                    vec![
                        BumpDatum::new(1., 10.),
                        BumpDatum::new("1", 1.),
                        BumpDatum::gap("Q3"),
                        BumpDatum::new("Q4", 10.),
                    ],
                ),
            ];
            let layout = bump_layout_with_point_factory(
                c,
                &data,
                &BumpLayoutOptions::default(),
                |context| {
                    let [x, y] = context.position;
                    let p = GraphicPoint::new;
                    Ok(context.inherit(Geometry::Path(vec![
                        PathCommand::Move(p(x + 12., y - 8.)),
                        PathCommand::Line(p(x + 20., y)),
                        PathCommand::Line(p(x + 12., y + 8.)),
                        PathCommand::Line(p(x + 4., y)),
                        PathCommand::Close,
                    ])))
                },
            )?;
            let point = &layout.series[0].points[1];
            let id = point.mark_id.clone().unwrap();
            let origin = point.position.unwrap();
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == id)
                .unwrap();
            let x = layout.x_scale.domain();
            let target = json!({"id":id,"value":rank,
                "point":if kind=="bump-nearest"{origin}else{[origin[0]+12.,origin[1]]},
                "samplePoint":[origin[0]+12.,origin[1]],"pick":if kind=="bump-nearest"{"nearest"}else{"geometry"},"color":rgba(mark.fill.unwrap()),
                "control":format!("{}: {}",mark.label,mark.value),
                "metadata":{"rank":point.rank,"ranks":layout.rank_scale.domain().iter().map(|key| match key { CategoryKey::Number(v)=>*v,_=>0. }).collect::<Vec<_>>(),
                    "typedX":x.contains(&CategoryKey::Number(1.))&&x.contains(&CategoryKey::String("1".into())),
                    "shoulders":layout.series[0].line_points.len()>layout.series[0].points.len()*2,
                    "missing":layout.series.iter().flat_map(|series| &series.points).filter(|point|point.rank.is_none()).count(),
                    "logical":origin}});
            Ok((layout.into_scene_interaction(), target))
        }
        "stream-area" | "stream-slice" | "stream-diverging" | "stream-dot"
        | "stream-dot-nearest" => {
            let delta = f64::from(phase) * 8.;
            let signed = kind == "stream-diverging";
            let custom = kind.starts_with("stream-dot");
            let nearest = kind == "stream-dot-nearest";
            let source = if signed {
                vec![Some(10.), Some(-20. - delta), Some(30.), Some(-16. - delta)]
            } else {
                vec![Some(10.), Some(20. + delta), Some(30.), Some(16. + delta)]
            };
            let categories = ["Q1", "Q2", "Q3", "Q4"].map(CategoryKey::from);
            let data = [
                BarSeries::new("A", source),
                BarSeries::new(
                    "B",
                    vec![
                        Some(16.),
                        None,
                        Some(if signed { -14. } else { 14. }),
                        Some(22.),
                    ],
                ),
            ];
            let options = StreamLayoutOptions {
                stack: stack::StackOptions {
                    order: stack::StackOrder::InsideOut,
                    offset: if signed {
                        stack::StackOffset::Diverging
                    } else {
                        stack::StackOffset::Wiggle
                    },
                },
                curve: if signed {
                    Curve::Linear
                } else {
                    Curve::CatmullRom { alpha: 0.5 }
                },
                dots: custom.then_some(StreamDotOptions::default()),
                ..Default::default()
            };
            let layout = if custom {
                stream_layout_with_dot_factory(c, &categories, &data, &options, |context| {
                    let [x, y] = context.position;
                    let p = GraphicPoint::new;
                    let mut dot = context.inherit(Geometry::Path(vec![
                        PathCommand::Move(p(x + 12., y - 8.)),
                        PathCommand::Line(p(x + 20., y)),
                        PathCommand::Line(p(x + 12., y + 8.)),
                        PathCommand::Line(p(x + 4., y)),
                        PathCommand::Close,
                    ]));
                    dot.fill = Some(Color(120, 45, 170, 255));
                    dot.stroke = None;
                    Ok(dot)
                })?
            } else {
                stream_layout(c, &categories, &data, &options)?
            };
            let layer = &layout.layers[0];
            let slice = kind == "stream-slice";
            let sample = &layer.samples[if slice || custom { 1 } else { 2 }];
            let id = if slice || custom {
                sample.mark_id.clone()
            } else {
                layer.area_mark_id.clone().unwrap()
            };
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == id)
                .unwrap();
            let missing = layout
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == layout.layers[1].samples[1].mark_id)
                .unwrap();
            let pixel_point = if custom {
                [sample.position[0] + 12., sample.position[1]]
            } else {
                sample.position
            };
            let target = json!({"id":id,"value":mark.value,"point":if nearest {sample.position}else{pixel_point},"samplePoint":pixel_point,"color":rgba(mark.fill.unwrap()),
                "control":format!("{}: {}",mark.label,mark.value),"pick":if slice{"slice-x"}else if nearest{"nearest"}else{"geometry"},
                "metadata":{"rawMissing":layout.layers[1].samples[1].value.is_none(),
                    "logical":sample.position,
                    "samples":layout.layers.iter().map(|layer|layer.samples.len()).sum::<usize>(),
                    "sourceValue":layer.samples[3].value,"domain":layout.y_scale.display_domain(),
                    "stack":layout.stack.order,"slice":slice,"missingControl":format!("{}: missing",missing.label),
                    "sliceOrder":layout.slices[1].layer_indices.iter().map(|&i|layout.layers[i].id.clone()).collect::<Vec<_>>()}});
            Ok((
                if slice {
                    layout.into_slice_interaction()?
                } else {
                    layout.into_scene_interaction()?
                },
                target,
            ))
        }
        _ => super::radial_labels::source(kind, phase, c),
    }
}

fn measure_hierarchy_labels(layout: &HierarchyLayout) -> Result<Vec<HierarchyMeasuredLabel>> {
    let candidates = layout.label_candidates();
    let labels = candidates
        .iter()
        .map(|candidate| candidate.label.clone())
        .collect::<Vec<_>>();
    let bounds = measure_browser_labels(&labels)?;
    Ok(candidates
        .iter()
        .zip(bounds)
        .map(|(candidate, bounds)| candidate.measured(bounds))
        .collect())
}

pub(super) fn measure_browser_labels(labels: &[Label]) -> Result<Vec<LayoutRect>> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| ChartError("missing document".into()))?;
    let canvas = document
        .create_element("canvas")
        .map_err(|_| ChartError("measure canvas".into()))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ChartError("measure canvas type".into()))?;
    let context = canvas
        .get_context("2d")
        .map_err(|_| ChartError("measure context".into()))?
        .ok_or_else(|| ChartError("measure context unavailable".into()))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| ChartError("measure context type".into()))?;
    labels
        .iter()
        .map(|label| {
            context.set_font(&format!("{}px sans-serif", label.size));
            context.set_text_baseline("alphabetic");
            context.set_text_align(match label.anchor {
                TextAnchor::Start => "left",
                TextAnchor::Middle => "center",
                TextAnchor::End => "right",
            });
            let metrics = context
                .measure_text(&label.text)
                .map_err(|_| ChartError("measure label".into()))?;
            let x0 = -metrics.actual_bounding_box_left();
            let x1 = metrics.actual_bounding_box_right();
            let y0 = -metrics.actual_bounding_box_ascent();
            let y1 = metrics.actual_bounding_box_descent();
            let (sin, cos) = label.rotation.to_radians().sin_cos();
            let points = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
                .map(|(x, y)| (x * cos - y * sin, x * sin + y * cos));
            let x = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let y = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let maxx = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let maxy = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            Ok(LayoutRect {
                x,
                y,
                width: maxx - x,
                height: maxy - y,
            })
        })
        .collect()
}
