//! Static snapshots of real accepted interaction/layout state; no tooltip or
//! measuring heuristic is substituted for the live BrowserChart consumers.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::surface::*;
use kartis::*;

fn caption(scene: &mut Scene, frame: &ChartConfig, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: frame.width / 2.,
        y: frame.height - 9.,
        text: text.into(),
        color: Color(70, 90, 110, 255),
        size: 11.,
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
}
fn overlay(
    mut scene: Scene,
    model: &SurfaceModel,
    options: Option<&CrosshairOptions>,
) -> Result<Scene> {
    for mark in model.interaction_overlays(model.hovered(), options)? {
        if !scene.draw_order.is_empty() {
            scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
        }
        scene.marks.push(mark);
    }
    scene.validate()?;
    Ok(scene)
}
fn pie_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let data = [
        RadialDatum::new("North", 34.),
        RadialDatum::new("West", 26.),
        RadialDatum::new("Online", 22.),
        RadialDatum::new("South", 18.),
    ];
    let mut options = PieLayoutOptions {
        padding: PiePadding::D3Applied {
            pad_radius: PiePadRadius::Auto,
        },
        ..Default::default()
    };
    options.pie.inner_radius = 0.56;
    options.pie.pad_angle = 0.075;
    options.pie.style.corner_radius = 7.;
    options.pie.style.active_ids.insert("West".into());
    options.pie.style.active_outer_radius_offset = 10.;
    options.pie.style.arc_labels.text = PieLabelText::Percent(ValueFormat::Percent(0));
    let mut donut = pie_layout(c, &data, &options)?.scene;
    caption(
        &mut donut,
        c,
        "State snapshot: D3 asin padding, rounded donut; West explicitly active",
    );
    let mut partial_options = options.clone();
    partial_options.pie.start_angle = -std::f64::consts::FRAC_PI_2;
    partial_options.pie.end_angle = std::f64::consts::PI / 3.;
    partial_options.pie.style.active_ids.clear();
    partial_options.pie.inner_radius = 0.45;
    partial_options.fit = PieFit::NivoSector;
    let mut partial = pie_layout(c, &data, &partial_options)?.scene;
    caption(
        &mut partial,
        c,
        "State snapshot: exact-cardinal sector fit; no text measurement implied",
    );
    let zero = [
        RadialDatum::new("Zero A", 0.),
        RadialDatum::new("Main", 2.),
        RadialDatum::new("Other", 1.),
        RadialDatum::new("Zero B", 0.),
    ];
    options.pie.style.active_ids.clear();
    options.pie.style.corner_radius = 0.;
    options.pie.style.border = Some(GraphicStroke {
        color: Color(32, 56, 86, 255),
        width: 3.,
        cap: StrokeCap::Butt,
    });
    options.pie.style.arc_labels.text = PieLabelText::Label;
    // A padding-only arc has no filled area on which to place text. Preserve
    // its real border and raw zero metadata while omitting those two labels.
    options.pie.style.arc_labels.skip_angle = 0.2;
    options.pie.pad_angle = 0.12;
    let mut borders = pie_layout(c, &zero, &options)?.scene;
    caption(
        &mut borders,
        c,
        "State snapshot: zero-valued slices reserve padding and retain explicit radial borders",
    );
    Ok(vec![
        ("Pie · D3 padded donut snapshot", donut),
        ("Pie · partial sector fit snapshot", partial),
        ("Pie · zero-value border snapshot", borders),
    ])
}
fn line_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let data = [
        Series::new(
            "North",
            vec![Point::new(0., 2.), Point::new(5., 7.), Point::new(10., 4.)],
        ),
        Series::new(
            "South",
            vec![Point::new(0., 8.), Point::new(5., 3.), Point::new(10., 7.)],
        ),
    ];
    let layout = line_layout_with_style(
        c,
        &data,
        &LineOptions {
            points: false,
            x_domain: Some([0., 10.]),
            y_domain: Some([0., 10.]),
            ..Default::default()
        },
        &LineStyleOptions {
            interactive_sites: true,
            ..Default::default()
        },
    )?;
    let site = layout
        .sites
        .iter()
        .find(|s| s.series_id == "North" && s.raw_x == 5.)
        .ok_or_else(|| ChartError("gallery line site missing".into()))?;
    let mut result = vec![];
    for (title, picking, kind, point, note) in [
        (
            "Line · point crosshair snapshot",
            Picking::Nearest { distance: 30. },
            CrosshairType::Cross,
            site.position,
            "State snapshot: one nearest point; explicit Cross crosshair",
        ),
        (
            "Line · SliceX crosshair snapshot",
            Picking::SliceX { distance: 500. },
            CrosshairType::TopRight,
            [site.position[0], c.margin.top + 35.],
            "State snapshot: both series at x=5; SliceX forces a full-height crosshair",
        ),
        (
            "Line · SliceY crosshair snapshot",
            Picking::SliceY { distance: 500. },
            CrosshairType::BottomLeft,
            [c.margin.left + 60., site.position[1]],
            "State snapshot: current y=7 members; SliceY forces a full-width crosshair",
        ),
    ] {
        let mut model = SurfaceModel::new_with_interaction(
            layout.scene.clone(),
            SurfaceOptions {
                plot: Some(layout.plot),
                picking,
                ..Default::default()
            },
            Some(layout.interaction_binding()),
        )?;
        model.apply(SurfaceInput::Move {
            id: 1,
            point: (point[0], point[1]),
        })?;
        if model.hovered().is_empty() {
            return Err(ChartError("gallery line interaction failed".into()));
        }
        let mut scene = overlay(
            layout.scene.clone(),
            &model,
            Some(&CrosshairOptions {
                kind,
                ..Default::default()
            }),
        )?;
        caption(&mut scene, c, note);
        result.push((title, scene));
    }
    Ok(result)
}
fn radar_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let axes = ["Speed", "Quality", "Cost", "Safety", "Uptime"].map(str::to_string);
    let data = [
        RadarSeries::new("North", vec![75., 65., 40., 70., 85.]),
        RadarSeries::new("South", vec![50., 80., 65., 45., 60.]),
        RadarSeries::new("Online", vec![60., 55., 85., 65., 40.]),
    ];
    let mut result = vec![];
    for (index, title, note) in [
        (
            0,
            "Radar · clockwise slice snapshot",
            "State snapshot: clockwise Quality sector; three rows sorted by raw value",
        ),
        (
            1,
            "Radar · counterclockwise slice snapshot",
            "State snapshot: rotated counterclockwise axes; genuine center-to-tip spoke",
        ),
        (
            2,
            "Radar · zero-value hidden slice snapshot",
            "State snapshot: no point ink; all-zero Speed, South hidden, typed slice remains",
        ),
    ] {
        let mut source = data.clone();
        let mut options = RadarLayoutOptions {
            domain: Some([0., 100.]),
            rotation: if index == 1 { 0.55 } else { 0. },
            ..Default::default()
        };
        options.points.enabled = index < 2;
        if index == 2 {
            for d in &mut source {
                d.values[0] = 0.;
            }
            options.hidden.insert("South".into());
        }
        let layout = radar_layout(
            c,
            &axes,
            &source,
            &options,
            &RadarInteractionOptions {
                direction: if index == 1 {
                    PolarSliceDirection::CounterClockwise
                } else {
                    PolarSliceDirection::Clockwise
                },
                ..Default::default()
            },
        )?;
        let slice = &layout.slices[if index == 2 { 0 } else { 1 }];
        let mut model = SurfaceModel::new_with_interaction(
            layout.scene.clone(),
            SurfaceOptions::default(),
            Some(layout.interaction_binding()),
        )?;
        let point = [
            layout.center[0] + layout.radius * 0.65 * slice.angle.sin(),
            layout.center[1] - layout.radius * 0.65 * slice.angle.cos(),
        ];
        model.apply(SurfaceInput::Move {
            id: 1,
            point: (point[0], point[1]),
        })?;
        if model.active_interaction(model.hovered()).is_none() {
            return Err(ChartError("gallery radar sector missing".into()));
        }
        let mut scene = overlay(layout.scene.clone(), &model, None)?;
        caption(&mut scene, c, note);
        result.push((title, scene));
    }
    Ok(result)
}
fn relation_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let tree = [
        HierarchyDatum::new("Product", None, 0.),
        HierarchyDatum::new("Cloud", Some("Product".into()), 0.),
        HierarchyDatum::new("Devices", Some("Product".into()), 0.),
        HierarchyDatum::new("API", Some("Cloud".into()), 3.),
        HierarchyDatum::new("Storage", Some("Cloud".into()), 2.),
        HierarchyDatum::new("Phone", Some("Devices".into()), 4.),
    ];
    let mut tree_options = TreeOptions {
        node_radius: 7.,
        ..Default::default()
    };
    tree_options.hierarchy.include_root = true;
    let mut tree_layout = tree_relation_layout(
        c,
        &tree,
        &tree_options,
        &RelationOptions {
            mode: RelationMode::Tree(TreeRelationOptions {
                ancestor_nodes: true,
                ancestor_links: true,
                descendant_nodes: false,
                descendant_links: false,
            }),
            ..Default::default()
        },
    )?;
    tree_layout.set_focus(Some(RelationFocus::Node("API".into())))?;
    let mut tree_scene = tree_layout.scene().clone();
    caption(
        &mut tree_scene,
        c,
        "State snapshot: API focus; only ancestor nodes and ancestor links active",
    );
    let nodes = ["Core", "Search", "Storage", "Web", "Batch", "Isolated"].map(NetworkNode::new);
    let links = [
        NetworkLink::new("Core", "Search", 1.),
        NetworkLink::new("Core", "Storage", 1.),
        NetworkLink::new("Core", "Web", 1.),
        NetworkLink::new("Search", "Batch", 1.),
        NetworkLink::new("Storage", "Web", 1.),
    ];
    let mut simulation = ForceSimulation::new(&nodes, &links, ForceOptions::default())?;
    for (id, p) in [
        ("Core", [-20., 0.]),
        ("Search", [-160., -90.]),
        ("Storage", [120., -90.]),
        ("Web", [125., 90.]),
        ("Batch", [-170., 95.]),
        ("Isolated", [250., 15.]),
    ] {
        simulation.set_fixed(id, Some(p))?;
    }
    let mut network = simulation.scene_with_relations(
        c,
        &[
            "core-search",
            "core-storage",
            "core-web",
            "search-batch",
            "storage-web",
        ]
        .map(str::to_string),
        &RelationOptions {
            mode: RelationMode::Neighbors,
            ..Default::default()
        },
    )?;
    network.set_focus(Some(RelationFocus::Node("Core".into())))?;
    let mut network_scene = network.scene().clone();
    caption(
        &mut network_scene,
        c,
        "State snapshot: Core and its direct neighbors; only incident links active",
    );
    let ids = ["North", "South", "Online", "Retail"].map(str::to_string);
    let mut chord = chord_relation_layout(
        c,
        &ids,
        &[
            vec![0., 7., 3., 2.],
            vec![5., 0., 4., 1.],
            vec![3., 2., 0., 5.],
            vec![1., 2., 4., 0.],
        ],
        0.04,
        &RelationOptions {
            mode: RelationMode::Chord,
            ..Default::default()
        },
    )?;
    chord.set_focus(Some(RelationFocus::Node("South".into())))?;
    let mut chord_scene = chord.scene().clone();
    caption(
        &mut chord_scene,
        c,
        "State snapshot: South group and incident ribbons; unrelated groups dimmed",
    );
    let flow_nodes = ["Source", "Branch", "Other", "Sink"].map(FlowNode::new);
    let flow_links = [
        SankeyStyleLink::new("main", "Source", "Branch", 8.),
        SankeyStyleLink::new("parallel", "Source", "Branch", 4.),
        SankeyStyleLink::new("other", "Source", "Other", 6.),
        SankeyStyleLink::new("branch-out", "Branch", "Sink", 12.),
        SankeyStyleLink::new("other-out", "Other", "Sink", 6.),
    ];
    let mut sankey = sankey_with_style(
        c,
        &flow_nodes,
        &flow_links,
        &SankeyLayoutOptions::default(),
        &SankeyStyleOptions::default(),
    )?
    .relation_layout(&RelationOptions {
        mode: RelationMode::Sankey,
        ..Default::default()
    })?;
    sankey.set_focus(Some(RelationFocus::Edge("main".into())))?;
    let mut sankey_scene = sankey.scene().clone();
    caption(
        &mut sankey_scene,
        c,
        "State snapshot: Source→Branch edge focus; both parallel ribbons active",
    );
    Ok(vec![
        ("Tree · ancestor relation snapshot", tree_scene),
        ("Network · neighbor relation snapshot", network_scene),
        ("Chord · incident relation snapshot", chord_scene),
        ("Sankey · parallel relation snapshot", sankey_scene),
    ])
}
/// Counts are exposed for a caller's independent rendered-DOM checks; they are
/// observations of actual produced parts, not alternate drawing metadata.
pub fn part_counts(scene: &Scene) -> std::collections::BTreeMap<&'static str, usize> {
    let counts = [
        ("marks", scene.marks.len()),
        ("labels", scene.labels.len()),
        (
            "crosshairs",
            scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("kartis-crosshair:"))
                .count(),
        ),
        (
            "radar_sites",
            scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("radar-point:"))
                .count(),
        ),
        (
            "low_opacity_marks",
            scene.marks.iter().filter(|m| m.opacity < 0.5).count(),
        ),
    ];
    counts.into_iter().collect()
}
pub fn scenes(frame: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut c = frame.clone();
    c.width = 720.;
    c.height = 400.;
    let mut result = pie_scenes(&c)?;
    result.extend(line_scenes(&c)?);
    result.extend(radar_scenes(&c)?);
    result.extend(relation_scenes(&c)?);
    for (_, scene) in &result {
        scene.validate()?;
        let _ = part_counts(scene);
    }
    Ok(result)
}
