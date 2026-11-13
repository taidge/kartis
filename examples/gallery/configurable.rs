use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
fn hierarchy() -> Vec<HierarchyDatum> {
    let mut data = vec![HierarchyDatum::new("Charts", None, 0.)];
    for (parent, leaves) in [
        ("Cartesian", ["Line", "Bar", "Parallel"]),
        ("Radial", ["Pie", "Tree", "Radar"]),
        ("Graph", ["Force", "Sankey", "Chord"]),
    ] {
        data.push(HierarchyDatum::new(parent, Some("Charts".into()), 0.));
        for leaf in leaves {
            data.push(HierarchyDatum::new(leaf, Some(parent.into()), 1.));
        }
    }
    data
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut result = Vec::new();
    let mut tree_config = c.clone();
    tree_config.margin = Margin {
        left: 70.,
        right: 90.,
        top: 65.,
        bottom: 65.,
    };
    let mut tree_options = TreeOptions {
        hierarchy: HierarchyOptions {
            include_root: true,
            ..Default::default()
        },
        node_radius: 9.,
        ..Default::default()
    };
    tree_options.style.links.curve = TreeLinkCurve::StepAfter;
    tree_options.style.links.width = 3.;
    tree_options.style.links.color = TreeLinkColor::Target;
    tree_options.style.links.dash = Some(StrokeDash {
        segments: vec![8., 5.],
        offset: 2.,
    });
    tree_options.style.internal = Some(TreeNodeStyle {
        shape: TreeNodeShape::Diamond,
        radius: Some(13.),
        border: Some(GraphicStroke {
            color: Color(30, 50, 70, 255),
            width: 2.,
            cap: StrokeCap::Round,
        }),
        ..Default::default()
    });
    tree_options.style.leaves = Some(TreeNodeStyle {
        shape: TreeNodeShape::Square,
        fill: TreeNodeFill::Fixed(Color(50, 155, 115, 255)),
        ..Default::default()
    });
    tree_options.style.labels.visibility = TreeLabels::Leaves;
    tree_options.style.labels.offset = 12.;
    result.push((
        "Tree step links and node styles",
        tree_with_options(&tree_config, &hierarchy(), &tree_options)?,
    ));
    tree_options.orientation = TreeOrientation::Radial;
    tree_options.style.links.curve = TreeLinkCurve::Step;
    tree_options.style.links.dash = None;
    tree_options.style.labels.position = TreeLabelPosition::Outward;
    tree_options.style.labels.rotation = TreeLabelRotation::AlongLayout;
    result.push((
        "Radial tree link styles",
        tree_with_options(&tree_config, &hierarchy(), &tree_options)?,
    ));
    tree_options.orientation = TreeOrientation::TopToBottom;
    tree_options.style.links.curve = TreeLinkCurve::Straight;
    tree_options.style.labels.position = TreeLabelPosition::Below;
    tree_options.style.labels.rotation = TreeLabelRotation::Fixed(20.);
    result.push((
        "Tree straight links",
        tree_with_options(&tree_config, &hierarchy(), &tree_options)?,
    ));
    let mut dimensions: Vec<_> = ["Cost", "Throughput", "Deviation", "Success"]
        .map(ParallelDimension::new)
        .to_vec();
    dimensions[0].domain = Some([0., 100.]);
    dimensions[1].domain = Some([1., 1000.]);
    dimensions[1].scale = ScaleKind::Log;
    dimensions[2].domain = Some([-100., 100.]);
    dimensions[2].scale = ScaleKind::SymLog { constant: 5. };
    dimensions[3].domain = Some([0., 1.]);
    dimensions[3].reverse = true;
    dimensions[3].axis.format = ValueFormat::Percent(0);
    let mut rows = vec![
        ParallelDatum::new("Alpha", vec![Some(20.), Some(10.), Some(-40.), Some(0.90)]),
        ParallelDatum::new("Beta", vec![Some(45.), Some(150.), Some(20.), Some(0.60)]),
        ParallelDatum::new("Gamma", vec![Some(80.), Some(700.), Some(70.), Some(0.25)]),
        ParallelDatum::new("Delta", vec![Some(60.), None, Some(-10.), Some(0.75)]),
    ];
    rows[0].group = Some("Control".into());
    rows[1].group = Some("Control".into());
    rows[2].group = Some("Candidate".into());
    rows[3].group = Some("Candidate".into());
    let mut parallel = ParallelCoordinatesOptions {
        curve: Curve::Natural,
        row_labels: true,
        row_label_offset: [55., 0.],
        line: ParallelLineStyle {
            opacity: 0.8,
            width: 3.,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut parallel_config = c.clone();
    parallel_config.margin = Margin {
        left: 105.,
        right: 110.,
        top: 55.,
        bottom: 65.,
    };
    parallel.series_styles.insert(
        "Control".into(),
        ParallelLineStyle {
            color: Some(Color(45, 105, 180, 255)),
            opacity: 0.8,
            width: 3.,
            ..Default::default()
        },
    );
    parallel.series_styles.insert(
        "Candidate".into(),
        ParallelLineStyle {
            color: Some(Color(210, 95, 40, 255)),
            opacity: 0.85,
            width: 3.,
            dash: Some(StrokeDash {
                segments: vec![8., 5.],
                offset: 0.,
            }),
            ..Default::default()
        },
    );
    result.push((
        "Parallel coordinates independent scales",
        parallel_coordinates_with_options(&parallel_config, &dimensions, &rows, &parallel)?,
    ));
    parallel.layout = ParallelLayout::Vertical;
    parallel.curve = Curve::Linear;
    parallel.missing = ParallelMissing::Connect;
    parallel.row_label_offset = [0., -20.];
    result.push((
        "Parallel coordinates gaps and horizontal axes",
        parallel_coordinates_with_options(&parallel_config, &dimensions, &rows, &parallel)?,
    ));
    let nodes: Vec<_> = ["Glory", "Widgets", "Layouts", "SVG", "Canvas"]
        .map(|id| NetworkNode {
            id: id.into(),
            label: id.into(),
            radius: 18.,
        })
        .to_vec();
    let links = vec![
        NetworkLink::new("Glory", "Widgets", 25.),
        NetworkLink::new("Glory", "Layouts", 25.),
        NetworkLink::new("Widgets", "SVG", 16.),
        NetworkLink::new("Widgets", "Canvas", 16.),
        NetworkLink::new("Layouts", "SVG", 9.),
        NetworkLink::new("Layouts", "Canvas", 9.),
    ];
    let mut simulation = ForceSimulation::new(
        &nodes,
        &links,
        ForceOptions {
            repulsion: 0.,
            center_strength: 0.,
            link_strength: 0.,
            collision_iterations: 0,
            ..Default::default()
        },
    )?;
    for (id, p) in [
        ("Glory", [-180., 0.]),
        ("Widgets", [-25., -80.]),
        ("Layouts", [-25., 80.]),
        ("SVG", [150., -80.]),
        ("Canvas", [150., 80.]),
    ] {
        simulation.set_position(id, p)?;
    }
    result.push(("Network before drag", simulation.scene(c)?));
    let mapping = NetworkDragMapping::from_config(
        c,
        ClipRect {
            x: 0.,
            y: 0.,
            width: c.width,
            height: c.height,
        },
        ZoomTransform::default(),
    )?;
    let mut drag = NetworkDragController::new(
        simulation,
        NetworkDragOptions {
            release: NetworkDragRelease::Pin,
            ..Default::default()
        },
    )?;
    let start = mapping.world_to_client([-180., 0.])?;
    let end = mapping.world_to_client([-100., -120.])?;
    drag.begin(1, "Glory", start, &mapping)?;
    drag.move_pointer(1, end, &mapping)?;
    drag.end(1, end, &mapping)?;
    result.push(("Network after drag", drag.simulation().scene(c)?));
    Ok(result)
}
