use glory::render::PathCommand;
use kartis::{
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};
use std::f64::consts::{PI, TAU};

fn config() -> ChartConfig {
    ChartConfig {
        width: 260.,
        height: 245.,
        margin: Margin {
            left: 19.,
            right: 13.,
            top: 12.,
            bottom: 23.,
        },
        ..Default::default()
    }
}

fn tree() -> Vec<HierarchyDatum> {
    [
        ("A", Some("west"), 7.),
        ("west", Some("root"), 3.),
        ("A-child", Some("A"), 1.),
        ("root", None, 2.),
        ("east", Some("root"), 5.),
        ("B", Some("west"), 4.),
        ("zero", Some("west"), 0.),
        ("C", Some("east"), 8.),
    ]
    .map(|(id, parent, weight)| HierarchyDatum::new(id, parent.map(str::to_owned), weight))
    .to_vec()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} differs from {expected}"
    );
}

#[test]
fn matches_d3_partition_and_rounded_arc_reference_with_internal_weights() {
    // d3-hierarchy 3.1.2 hierarchy.sum + partition.size([2*PI, 105**2])
    // and d3-shape 3.2.0 arc().cornerRadius(9).padAngle(.08).digits(12).
    let options = SunburstLayoutOptions {
        corner_radius: 9.,
        pad_angle: 0.08,
        ..Default::default()
    };
    let data = tree();
    let layout = sunburst_layout(&config(), &data, &options).unwrap();
    assert_eq!(layout.frame.center, [133., 117.]);
    assert_eq!(layout.frame.outer_radius, 105.);
    assert_eq!(layout.total, 30.);
    assert_eq!(layout.root_id.as_deref(), Some("root"));
    let oracle = [
        ("root", [0., TAU], [0., 52.5], 30., 0, 3, "100.00%", None),
        (
            "west",
            [0., PI],
            [52.5, 74.24621202458749],
            15.,
            1,
            2,
            "50.00%",
            Some([3.636336825035, -64.010867633564]),
        ),
        (
            "east",
            [PI, 5.864306286700947],
            [52.5, 74.24621202458749],
            13.,
            1,
            1,
            "43.33%",
            Some([-3.636336825035, 64.010867633564]),
        ),
        (
            "A",
            [0., 1.6755160819145563],
            [74.24621202458749, 90.93266739736606],
            8.,
            2,
            1,
            "26.67%",
            Some([4.694490654854, -81.553868407498]),
        ),
        (
            "B",
            [1.6755160819145563, 2.5132741228718345],
            [74.24621202458749, 90.93266739736606],
            4.,
            2,
            0,
            "13.33%",
            Some([80.616399889263, 13.193474282022]),
        ),
        (
            "zero",
            [2.5132741228718345, 2.5132741228718345],
            [74.24621202458749, 90.93266739736606],
            0.,
            2,
            0,
            "0.00%",
            None,
        ),
        (
            "C",
            [PI, 4.817108735504349],
            [74.24621202458749, 90.93266739736606],
            8.,
            2,
            0,
            "26.67%",
            Some([-4.694490654854, 81.553868407498]),
        ),
        (
            "A-child",
            [0., 0.20943951023931953],
            [90.93266739736606, 105.],
            1.,
            3,
            0,
            "3.33%",
            Some([5.554596251027, -99.543805580139]),
        ),
    ];
    assert_eq!(layout.nodes.len(), oracle.len());
    for (node, (id, angles, radii, value, depth, height, formatted, start)) in
        layout.nodes.iter().zip(oracle)
    {
        assert_eq!(&node.data, &data[node.source_index]);
        assert_eq!(node.data.id, id);
        assert_eq!(
            (node.value, node.depth, node.height),
            (value, depth, height)
        );
        assert_eq!(node.formatted_value, formatted);
        close(node.percentage, value / 30. * 100.);
        close(node.arc.angles[0], angles[0]);
        close(node.arc.angles[1], angles[1]);
        close(node.arc.inner_radius, radii[0]);
        close(node.arc.outer_radius, radii[1]);
        match start {
            Some([x, y]) => {
                let mark = &layout.scene.marks[node.mark_index.unwrap()];
                let Geometry::Path(path) = &mark.geometry else {
                    panic!("missing arc path")
                };
                let PathCommand::Move(point) = path[0] else {
                    panic!("missing arc start")
                };
                close(point.x - layout.frame.center[0], x);
                close(point.y - layout.frame.center[1], y);
            }
            None => assert!(node.mark_index.is_none()),
        }
    }
    assert_eq!(
        layout
            .path("A-child")
            .unwrap()
            .iter()
            .map(|node| node.data.id.as_str())
            .collect::<Vec<_>>(),
        ["A-child", "A", "west", "root"]
    );
    assert_eq!(layout.node("west").unwrap().data.value, 3.);
    assert_eq!(
        layout.node("west").unwrap().color,
        Color(232, 193, 160, 255)
    );
    assert_eq!(layout.node("east").unwrap().color, Color(244, 117, 96, 255));
    assert_eq!(
        layout.node("A-child").unwrap().color,
        layout.node("west").unwrap().color
    );
    assert_eq!(
        layout.node("C").unwrap().color,
        layout.node("east").unwrap().color
    );
    assert!(layout.scene.labels.is_empty());
    assert!(sunburst(&config(), &data, &HierarchyOptions::default()).is_err());
}

#[test]
fn styles_keep_hidden_slots_raw_values_and_palette_keys() {
    let data = tree();
    let green = Color(20, 160, 80, 255);
    let gradient = FillPaint::Linear {
        start: [90., 90.],
        end: [150., 150.],
        stops: vec![
            GradientStop::new(0., Color(250, 0, 0, 255)),
            GradientStop::new(1., Color(0, 0, 250, 255)),
        ],
    };
    let mut options = SunburstLayoutOptions {
        colors_by_id: [("west".to_owned(), green)].into(),
        paints_by_id: [("A".to_owned(), gradient.clone())].into(),
        child_color: InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
        border_color: InheritedColor::FromContext(vec![ColorModifier::Opacity(0.5)]),
        labels: Some(SunburstLabelOptions {
            text: SunburstLabelText::Label,
            skip_angle: 0.3,
            skip_radius: 15.,
            radius_offset: 0.25,
            size: Some(14.),
            color: InheritedColor::Fixed(Color(1, 2, 3, 255)),
        }),
        ..Default::default()
    };
    let visible = sunburst_layout(&config(), &data, &options).unwrap();
    options.hidden.insert("west".into());
    let hidden = sunburst_layout(&config(), &data, &options).unwrap();
    assert_eq!(visible.color_scale, hidden.color_scale);
    assert_eq!(
        visible.node("west").unwrap().arc,
        hidden.node("west").unwrap().arc
    );
    assert!(hidden.node("west").unwrap().mark_index.is_none());
    assert!(hidden.node("west").unwrap().label_index.is_none());
    let a = hidden.node("A").unwrap();
    assert_eq!(a.color, Color(14, 112, 56, 255));
    let mark = &hidden.scene.marks[a.mark_index.unwrap()];
    assert_eq!(mark.fill_paint.as_ref(), Some(&gradient));
    assert_eq!(mark.stroke.unwrap().color, Color(14, 112, 56, 128));
    let label_slot = a.label_index.unwrap();
    let label = &hidden.scene.labels[label_slot];
    let [x, y] = hidden.frame.point(
        a.arc.inner_radius + (a.arc.outer_radius - a.arc.inner_radius) * 0.25,
        (a.arc.angles[0] + a.arc.angles[1]) / 2.,
    );
    close(label.x, x);
    close(label.y, y + 14. * 0.35);
    assert_eq!(label.color, Color(1, 2, 3, 255));
    assert_eq!(hidden.scene.label_series[&label_slot], "A");
    assert!(hidden.node("A-child").unwrap().label_index.is_none());
    assert!(
        render::to_svg(&hidden.scene)
            .unwrap()
            .contains("<linearGradient")
    );
    assert!(
        render::to_canvas_script(&hidden.scene, "sunburst")
            .unwrap()
            .contains("createLinearGradient")
    );

    options.inherit_color_from_parent = false;
    options.color_by = SunburstColorBy::Depth;
    options.value_format = Some(ValueFormat::Fixed(1));
    let by_depth = sunburst_layout(&config(), &data, &options).unwrap();
    assert_eq!(
        by_depth.color_scale.domain(),
        &[
            CategoryKey::Number(1.),
            CategoryKey::Number(2.),
            CategoryKey::Number(3.)
        ]
    );
    assert_eq!(
        by_depth.node("A").unwrap().color,
        by_depth.node("C").unwrap().color
    );
    assert_eq!(by_depth.node("A").unwrap().formatted_value, "8.0");
}

#[test]
fn typed_binding_uses_arc_ink_for_exact_and_centroid_for_nearest() {
    let c = config();
    let options = SunburstLayoutOptions {
        corner_radius: 5.,
        pad_angle: 0.08,
        border_width: 0.,
        ..Default::default()
    };
    let layout = sunburst_layout(&c, &tree(), &options).unwrap();
    let a = layout.node("A").unwrap();
    let [x, y] = a.anchor;
    assert_eq!(layout.hit_test(x, y).unwrap().data.id, "A");
    let blank = layout
        .frame
        .point((a.arc.inner_radius + a.arc.outer_radius) / 2., 2.8);
    assert!(layout.hit_test(blank[0], blank[1]).is_none());
    assert!(
        layout
            .hit_test(layout.frame.center[0], layout.frame.center[1])
            .is_none()
    );
    #[cfg(feature = "raster")]
    {
        use kartis::render::raster::{FontRegistry, render_raster};
        let raster =
            render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
        let pixel = |x: f64, y: f64| {
            let offset = (y as usize * raster.width as usize + x as usize) * 4;
            &raster.rgba[offset..offset + 4]
        };
        assert_eq!(pixel(x, y), [232, 193, 160, 255]);
        assert_eq!(pixel(blank[0], blank[1]), [255, 255, 255, 255]);
    }
    let interaction = layout.clone().into_scene_interaction().unwrap();
    assert_eq!(interaction.plot, Some(ClipRect::plot(&c)));
    for picking in [Picking::Geometry, Picking::Nearest { distance: 1. }] {
        let mut model = SurfaceModel::new(
            c.scene().unwrap(),
            SurfaceOptions {
                picking,
                ..Default::default()
            },
        )
        .unwrap();
        model.set_layout(interaction.clone()).unwrap();
        model
            .apply(SurfaceInput::Move {
                id: 1,
                point: (x, y),
            })
            .unwrap();
        assert_eq!(model.hovered(), std::slice::from_ref(&a.mark_id));
        let result = model.active_interaction(model.hovered()).unwrap();
        assert_eq!(result.rows[0].value, Some(8.));
        assert_eq!(result.rows[0].formatted_value.as_deref(), Some("26.67%"));
        assert_eq!(result.anchor, a.anchor);
        model
            .apply(SurfaceInput::Move {
                id: 1,
                point: (blank[0], blank[1]),
            })
            .unwrap();
        assert!(model.hovered().is_empty());
    }
}

#[test]
fn reverse_partial_partition_and_root_band_use_squared_radius() {
    let c = config();
    let options = SunburstLayoutOptions {
        start_angle: PI,
        end_angle: -PI / 2.,
        inner_ratio: 0.25,
        include_root: true,
        border_width: 0.,
        ..Default::default()
    };
    let layout = sunburst_layout(&c, &tree(), &options).unwrap();
    let root = layout.node("root").unwrap();
    let west = layout.node("west").unwrap();
    assert_eq!(root.arc.angles, [PI, -PI / 2.]);
    close(root.arc.inner_radius, 26.25);
    close(west.arc.angles[1], PI / 4.);
    close(
        root.arc.outer_radius.powi(2) - root.arc.inner_radius.powi(2),
        west.arc.outer_radius.powi(2) - west.arc.inner_radius.powi(2),
    );
    assert_eq!(
        layout
            .hit_test(root.anchor[0], root.anchor[1])
            .unwrap()
            .data
            .id,
        "root"
    );
    assert!(
        layout
            .hit_test(layout.frame.center[0], layout.frame.center[1])
            .is_none()
    );
    let hidden_root = sunburst_layout(
        &c,
        &tree(),
        &SunburstLayoutOptions {
            include_root: false,
            ..options
        },
    )
    .unwrap();
    assert_eq!(hidden_root.node("west").unwrap().arc, west.arc);
    assert_eq!(hidden_root.node("west").unwrap().color, west.color);
}

#[test]
fn validates_topology_options_and_zero_geometry_without_fabricating_marks() {
    let c = config();
    let default = SunburstLayoutOptions::default();
    let empty = sunburst_layout(&c, &[], &default).unwrap();
    assert_eq!(empty.root_id, None);
    assert!(empty.scene.marks.is_empty());
    assert!(empty.interaction_binding().unwrap().sites.is_empty());
    let mut data = tree();
    data.iter_mut().for_each(|node| node.value = 0.);
    let zero = sunburst_layout(&c, &data, &default).unwrap();
    assert_eq!(zero.total, 0.);
    assert!(zero.scene.marks.is_empty());
    assert!(
        zero.nodes
            .iter()
            .all(|node| node.formatted_value == "0.00%")
    );
    data[0].parent = Some("missing".into());
    assert!(sunburst_layout(&c, &data, &default).is_err());
    assert!(
        sunburst_layout(
            &c,
            &tree(),
            &SunburstLayoutOptions {
                corner_radius: f64::NAN,
                ..default.clone()
            }
        )
        .is_err()
    );
    let mut large = tree();
    large[0].value = f64::MAX;
    large[4].value = f64::MAX;
    assert!(sunburst_layout(&c, &large, &default).is_err());

    let collapsed = SunburstLayoutOptions {
        pad_angle: 0.25,
        border_width: 0.,
        ..default
    };
    let layout = sunburst_layout(&c, &tree(), &collapsed).unwrap();
    let child = layout.node("A-child").unwrap();
    assert_eq!(child.value, 1.);
    assert!(child.mark_index.is_none());
    let bordered = sunburst_layout(
        &c,
        &tree(),
        &SunburstLayoutOptions {
            border_width: 1.,
            ..collapsed
        },
    )
    .unwrap();
    assert!(bordered.node("A-child").unwrap().mark_index.is_some());
}
