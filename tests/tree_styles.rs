use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 300.,
        margin: Margin {
            top: 24.,
            right: 24.,
            bottom: 24.,
            left: 24.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 0.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
        HierarchyDatum::new("a1", Some("a".into()), 2.),
        HierarchyDatum::new("a2", Some("a".into()), 4.),
    ]
}
fn node<'a>(scene: &'a Scene, id: &str) -> &'a Mark {
    scene
        .marks
        .iter()
        .find(|mark| mark.id == format!("node:{id}"))
        .unwrap()
}
fn center(mark: &Mark) -> P {
    match &mark.geometry {
        Geometry::Circle { x, y, .. } => P::new(*x, *y),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => P::new(x + width / 2., y + height / 2.),
        Geometry::Path(path) => {
            let points: Vec<_> = path
                .iter()
                .filter_map(|command| match command {
                    C::Move(p) | C::Line(p) => Some(*p),
                    _ => None,
                })
                .collect();
            P::new(
                points.iter().map(|p| p.x).sum::<f64>() / points.len() as f64,
                points.iter().map(|p| p.y).sum::<f64>() / points.len() as f64,
            )
        }
    }
}
fn orientations() -> [TreeOrientation; 5] {
    [
        TreeOrientation::LeftToRight,
        TreeOrientation::RightToLeft,
        TreeOrientation::TopToBottom,
        TreeOrientation::BottomToTop,
        TreeOrientation::Radial,
    ]
}

#[test]
fn defaults_keep_the_earlier_tree_geometry_palette_labels_and_identity() {
    let c = config();
    let data = data();
    let scene = tree_with_options(&c, &data, &TreeOptions::default()).unwrap();
    assert_eq!(scene.marks.len(), 9);
    assert_eq!(scene.labels.len(), 5);
    assert_eq!(scene.label_series.len(), 5);
    assert!(scene.label_clips.is_empty());
    for (index, datum) in data.iter().enumerate() {
        let mark = node(&scene, &datum.id);
        assert_eq!(mark.fill, Some(c.theme.color(index)));
        assert!(mark.stroke.is_none());
        let Geometry::Circle { x, y, radius } = mark.geometry else {
            panic!("default node changed shape")
        };
        assert_eq!(radius, 4.);
        let label = scene
            .labels
            .iter()
            .find(|label| label.text == datum.label)
            .unwrap();
        assert_eq!(
            (label.x, label.y, label.rotation, label.anchor),
            (x, y - 4. - 5., 0., TextAnchor::Middle)
        );
        if datum.parent.is_some() {
            let link = scene
                .marks
                .iter()
                .find(|m| m.id == format!("link:{}", datum.id))
                .unwrap();
            assert_eq!(link.stroke.unwrap().width, 1.5);
            assert_eq!(link.stroke.unwrap().color, c.theme.grid);
            assert!(link.stroke_dash.is_none());
            assert!(
                matches!(&link.geometry,Geometry::Path(path) if path.len()==2&&matches!(path[1],C::Cubic{..}))
            );
        }
    }
}

#[test]
fn every_curve_and_layout_has_real_link_geometry_with_exact_node_endpoints() {
    let c = config();
    let data = data();
    for orientation in orientations() {
        for curve in [
            TreeLinkCurve::Bump,
            TreeLinkCurve::Straight,
            TreeLinkCurve::Step,
            TreeLinkCurve::StepBefore,
            TreeLinkCurve::StepAfter,
        ] {
            let mut options = TreeOptions {
                orientation,
                ..Default::default()
            };
            options.style.links.curve = curve;
            options.style.links.dash = Some(StrokeDash {
                segments: vec![3., 2.],
                offset: 1.,
            });
            let scene = tree_with_options(&c, &data, &options).unwrap();
            scene.validate().unwrap();
            for datum in data.iter().filter(|datum| datum.parent.is_some()) {
                let link = scene
                    .marks
                    .iter()
                    .find(|m| m.id == format!("link:{}", datum.id))
                    .unwrap();
                let Geometry::Path(path) = &link.geometry else {
                    panic!("link missing real path")
                };
                let C::Move(start) = path[0] else {
                    panic!("link has no Move")
                };
                let end = match path.last().unwrap() {
                    C::Line(p) | C::Cubic { end: p, .. } => *p,
                    _ => panic!("link has no endpoint"),
                };
                assert_eq!(start, center(node(&scene, datum.parent.as_ref().unwrap())));
                assert_eq!(end, center(node(&scene, &datum.id)));
                if curve == TreeLinkCurve::Straight {
                    assert_eq!(path.len(), 2);
                    assert!(matches!(path[1], C::Line(_)));
                } else if orientation != TreeOrientation::Radial && curve != TreeLinkCurve::Bump {
                    assert_eq!(path.len(), 4);
                    assert!(path[1..].iter().all(|p| matches!(p, C::Line(_))));
                }
            }
            if orientation == TreeOrientation::Radial
                && matches!(
                    curve,
                    TreeLinkCurve::Step | TreeLinkCurve::StepBefore | TreeLinkCurve::StepAfter
                )
            {
                assert!(
                    scene
                        .marks
                        .iter()
                        .filter(|m| m.id.starts_with("link:"))
                        .any(|m| m
                            .geometry
                            .commands()
                            .iter()
                            .any(|c| matches!(c, C::Cubic { .. }))),
                    "polar elbows require an actual circular arc"
                );
            }
        }
    }
}

#[test]
fn class_shapes_borders_link_sources_and_plot_clips_are_applied_to_shared_scene() {
    let c = config();
    let data = data();
    let internal = TreeNodeStyle {
        shape: TreeNodeShape::Square,
        radius: Some(10.),
        fill: TreeNodeFill::Fixed(Color(30, 120, 80, 255)),
        border: Some(GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 3.,
            cap: StrokeCap::Butt,
        }),
        opacity: 0.8,
    };
    let leaf = TreeNodeStyle {
        shape: TreeNodeShape::Diamond,
        radius: Some(7.),
        fill: TreeNodeFill::Fixed(Color(200, 70, 60, 255)),
        ..Default::default()
    };
    for orientation in orientations() {
        let mut options = TreeOptions {
            orientation,
            ..Default::default()
        };
        options.style.internal = Some(internal.clone());
        options.style.leaves = Some(leaf.clone());
        options.style.clip_plot = true;
        options.style.links = TreeLinkOptions {
            curve: TreeLinkCurve::Step,
            color: TreeLinkColor::Target,
            width: 4.,
            opacity: 0.65,
            cap: StrokeCap::Butt,
            dash: Some(StrokeDash {
                segments: vec![5., 3.],
                offset: -1.,
            }),
            ..Default::default()
        };
        let scene = tree_with_options(&c, &data, &options).unwrap();
        for datum in &data {
            let is_leaf = !data.iter().any(|d| d.parent.as_ref() == Some(&datum.id));
            let mark = node(&scene, &datum.id);
            assert_eq!(
                mark.fill,
                Some(if is_leaf {
                    Color(200, 70, 60, 255)
                } else {
                    Color(30, 120, 80, 255)
                })
            );
            assert_eq!(mark.opacity, if is_leaf { 1. } else { 0.8 });
            assert_eq!(mark.series, datum.id);
            assert_eq!(mark.clip, Some(ClipRect::plot(&c)));
            assert_eq!(matches!(mark.geometry, Geometry::Path(_)), is_leaf);
            if datum.parent.is_some() {
                let link = scene
                    .marks
                    .iter()
                    .find(|m| m.id == format!("link:{}", datum.id))
                    .unwrap();
                assert_eq!(link.stroke.unwrap().color, mark.fill.unwrap());
                assert_eq!(link.stroke.unwrap().width, 4.);
                assert_eq!(link.stroke_dash, options.style.links.dash);
            }
        }
        assert_eq!(scene.label_clips.len(), scene.labels.len());
        for (slot, label) in scene.labels.iter().enumerate() {
            assert_eq!(scene.label_series.get(&slot).unwrap(), &label.text);
            assert!(scene.label_clips.contains_key(&slot));
        }
        let mut reversed = data.clone();
        reversed.reverse();
        let other = tree_with_options(&c, &reversed, &options).unwrap();
        let ids = |s: &Scene| {
            s.marks
                .iter()
                .map(|m| m.id.clone())
                .collect::<std::collections::BTreeSet<_>>()
        };
        assert_eq!(ids(&scene), ids(&other));
    }
}

#[test]
fn label_visibility_outward_anchors_rotation_and_custom_font_are_real_label_fields() {
    let c = config();
    let data = data();
    for orientation in orientations() {
        let mut options = TreeOptions {
            orientation,
            ..Default::default()
        };
        options.style.labels = TreeLabelOptions {
            visibility: TreeLabels::Leaves,
            position: TreeLabelPosition::Outward,
            offset: 11.,
            rotation: TreeLabelRotation::AlongLayout,
            color: Some(Color(10, 50, 120, 255)),
            size: Some(18.),
        };
        let scene = tree_with_options(&c, &data, &options).unwrap();
        assert_eq!(scene.labels.len(), 3);
        for (slot, label) in scene.labels.iter().enumerate() {
            let id = scene.label_series.get(&slot).unwrap();
            assert!(matches!(id.as_str(), "a1" | "a2" | "b"));
            assert_eq!((label.size, label.color), (18., Color(10, 50, 120, 255)));
            assert!(label.rotation.abs() <= 90.);
            let p = center(node(&scene, id));
            if orientation == TreeOrientation::LeftToRight {
                assert!(label.x > p.x + 14.);
                assert_eq!(label.anchor, TextAnchor::Start);
            }
            if orientation == TreeOrientation::RightToLeft {
                assert!(label.x < p.x - 14.);
                assert_eq!(label.anchor, TextAnchor::End);
            }
            if matches!(
                orientation,
                TreeOrientation::TopToBottom | TreeOrientation::BottomToTop
            ) {
                assert_eq!(label.rotation.abs(), 90.);
            }
        }
        options.style.labels.visibility = TreeLabels::Internal;
        assert_eq!(
            tree_with_options(&c, &data, &options).unwrap().labels.len(),
            2
        );
        options.style.labels.visibility = TreeLabels::None;
        assert!(
            tree_with_options(&c, &data, &options)
                .unwrap()
                .labels
                .is_empty()
        );
        options.style.links.enabled = false;
        assert!(
            tree_with_options(&c, &data, &options)
                .unwrap()
                .marks
                .iter()
                .all(|m| m.id.starts_with("node:"))
        );
    }
}

#[test]
fn invalid_styles_extreme_values_and_dash_resource_rejection_are_explicit_and_atomic() {
    let c = config();
    let data = data();
    let saved = data.clone();
    let mut invalid = Vec::new();
    let mut o = TreeOptions::default();
    o.style.links.width = f64::NAN;
    invalid.push(o);
    let mut o = TreeOptions::default();
    o.style.links.dash = Some(StrokeDash {
        segments: vec![-1., 2.],
        offset: 0.,
    });
    invalid.push(o);
    let mut o = TreeOptions::default();
    o.style.nodes.radius = Some(f64::INFINITY);
    invalid.push(o);
    let mut o = TreeOptions::default();
    o.style.nodes.opacity = 1.1;
    invalid.push(o);
    let mut o = TreeOptions::default();
    o.style.labels.offset = f64::NAN;
    invalid.push(o);
    let mut o = TreeOptions::default();
    o.style.labels.rotation = TreeLabelRotation::Fixed(361.);
    invalid.push(o);
    for o in invalid {
        assert!(tree_with_options(&c, &data, &o).is_err());
        assert!(tree_with_options(&c, &[], &o).is_err());
    }
    let mut o = TreeOptions::default();
    o.style.internal = Some(TreeNodeStyle {
        radius: Some(1000.),
        ..Default::default()
    });
    assert!(tree_with_options(&c, &data, &o).is_err());
    let mut o = TreeOptions::default();
    o.style.links.dash = Some(StrokeDash {
        segments: vec![1e-12, 1e-12],
        offset: 0.,
    });
    assert!(tree_with_options(&c, &data, &o).is_err());
    assert_eq!(data, saved);
    let extreme = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("large", Some("root".into()), f64::MAX),
    ];
    let mut o = TreeOptions {
        orientation: TreeOrientation::Radial,
        rotation: 1e6,
        ..Default::default()
    };
    o.style.labels.rotation = TreeLabelRotation::Fixed(-360.);
    let scene = tree_with_options(&c, &extreme, &o).unwrap();
    assert_eq!(node(&scene, "large").value, f64::MAX);
    assert!(
        tree_with_options(
            &c,
            &vec![HierarchyDatum::new("n", None, 0.); 8001],
            &TreeOptions::default()
        )
        .is_err()
    );
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_show_real_node_shapes_borders_and_link_dash_gaps() {
    use render::raster::*;
    let c = config();
    let data = data();
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    for shape in [
        TreeNodeShape::Circle,
        TreeNodeShape::Square,
        TreeNodeShape::Diamond,
    ] {
        let mut options = TreeOptions::default();
        options.style.nodes = TreeNodeStyle {
            shape,
            radius: Some(10.),
            fill: TreeNodeFill::Fixed(Color(20, 130, 70, 255)),
            border: Some(GraphicStroke {
                color: Color(180, 30, 40, 255),
                width: 2.,
                cap: StrokeCap::Butt,
            }),
            ..Default::default()
        };
        options.style.labels.rotation = TreeLabelRotation::Fixed(25.);
        let full = tree_with_options(&c, &data, &options).unwrap();
        render_raster(&full, &fonts, &RasterOptions::default()).unwrap();
        let mark = node(&full, "a").clone();
        let p = center(&mark);
        let mut isolated = c.scene().unwrap();
        isolated.marks.push(mark);
        let image =
            render_raster(&isolated, &FontRegistry::new(), &RasterOptions::default()).unwrap();
        let pixel = |dx: f64, dy: f64| {
            let x = (p.x + dx).floor() as u32;
            let y = (p.y + dy).floor() as u32;
            &image.rgba
                [((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
        };
        assert_eq!(pixel(0., 0.), [20, 130, 70, 255]);
        if shape == TreeNodeShape::Square {
            assert_eq!(pixel(7., 7.), [20, 130, 70, 255]);
        } else {
            assert_eq!(pixel(8., 8.), [255, 255, 255, 255]);
        }
        assert!(
            pixel(9.5, 0.)[0] > 100,
            "node border must paint real red pixels"
        );
    }
    let data = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("child", Some("root".into()), 1.),
    ];
    let mut options = TreeOptions::default();
    options.hierarchy.labels = false;
    options.style.nodes.opacity = 0.;
    options.style.links = TreeLinkOptions {
        curve: TreeLinkCurve::Straight,
        width: 4.,
        color: TreeLinkColor::Fixed(Color(0, 0, 0, 255)),
        cap: StrokeCap::Butt,
        dash: Some(StrokeDash {
            segments: vec![10., 20.],
            offset: 0.,
        }),
        ..Default::default()
    };
    let scene = tree_with_options(&c, &data, &options).unwrap();
    let p = center(node(&scene, "root"));
    let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    let pixel = |dx: f64| {
        let x = (p.x + dx).floor() as u32;
        let y = p.y.floor() as u32;
        &image.rgba[((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
    };
    assert_eq!(pixel(5.), [0, 0, 0, 255]);
    assert_eq!(pixel(20.), [255, 255, 255, 255]);
    assert_eq!(pixel(35.), [0, 0, 0, 255]);
    assert!(render::to_svg(&scene).unwrap().contains("stroke-dasharray"));
}
