use std::cell::{Cell, RefCell};

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left: 24.,
            right: 24.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn source() -> Vec<HierarchyDatum> {
    let mut data = vec![
        HierarchyDatum::new("a", Some("branch".into()), 2.),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), -0.),
        HierarchyDatum::new("zero", Some("branch".into()), -0.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    data[3].label.clear();
    data
}
fn color(i: usize) -> Color {
    Color(i as u8 * 30, 70, 90, 255)
}

#[test]
fn empty_bundle_preserves_complete_layout_scene_and_labels() {
    for orientation in [
        TreeOrientation::LeftToRight,
        TreeOrientation::RightToLeft,
        TreeOrientation::TopToBottom,
        TreeOrientation::BottomToTop,
        TreeOrientation::Radial,
    ] {
        for mode in [TreeMode::Tidy, TreeMode::Cluster] {
            for data in [
                vec![],
                vec![HierarchyDatum::new("root", None, -0.)],
                source(),
            ] {
                let options = TreeOptions {
                    orientation,
                    mode,
                    ..Default::default()
                };
                let old = tree_layout(&config(), &data, &options).unwrap();
                let new = tree_layout_with_factories(
                    &config(),
                    &data,
                    &options,
                    &mut TreeFactories::default(),
                )
                .unwrap();
                assert_eq!(old, new);
                assert_eq!(format!("{old:?}"), format!("{new:?}"));
                assert_eq!(
                    render::to_svg(&old.scene).unwrap(),
                    render::to_svg(&new.scene).unwrap()
                );
            }
        }
    }
}

#[test]
fn all_five_functions_receive_correct_stages_in_breadth_first_order() {
    let data = source();
    let mut options = TreeOptions::default();
    options.style.nodes.fill = TreeNodeFill::None;
    options.style.nodes.opacity = 0.;
    let base = tree_layout(&config(), &data, &options).unwrap();
    let log = RefCell::new(Vec::new());
    let mut size = |c: &TreeNodeFunctionContext<'_>| {
        assert_eq!(c.source, data);
        assert_eq!(c.datum, &data[c.source_index]);
        assert_eq!(c.position, base.nodes[c.source_index].position);
        assert_eq!(c.children, base.nodes[c.source_index].children);
        assert_eq!(c.parent_index, base.nodes[c.source_index].parent_index);
        assert_eq!(c.depth, base.nodes[c.source_index].depth);
        assert_eq!(c.height, base.nodes[c.source_index].height);
        assert_eq!(c.value, base.nodes[c.source_index].value);
        assert_eq!(c.plot, base.plot);
        log.borrow_mut().push(format!("size:{}", c.datum.id));
        Ok(if c.source_index == 1 {
            0.
        } else {
            8. + c.source_index as f64 * 2.
        })
    };
    let mut node_color = |c: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("color:{}", c.datum.id));
        Ok(color(c.source_index))
    };
    let mut thickness = |c: &TreeLinkFunctionContext<'_>| {
        assert_eq!(c.source, data);
        assert_eq!(c.source_node.source_index, c.link.source_index);
        assert_eq!(c.target_node.source_index, c.link.target_index);
        assert_eq!(c.source_node.color, Some(color(c.link.source_index)));
        assert_eq!(c.target_node.color, Some(color(c.link.target_index)));
        assert_eq!(
            c.source_node.style.fill,
            TreeNodeFill::Fixed(color(c.link.source_index))
        );
        log.borrow_mut()
            .push(format!("width:{}", data[c.link.target_index].id));
        Ok(if c.link.target_index == 4 {
            0.
        } else {
            c.target_node.radius
        })
    };
    let mut link_color = |c: &TreeLinkFunctionContext<'_>| {
        log.borrow_mut()
            .push(format!("link-color:{}", data[c.link.target_index].id));
        Ok(c.target_node.color.unwrap())
    };
    let mut label = |c: &TreeLabelFunctionContext<'_>| {
        assert_eq!(c.node.color, Some(color(c.source_index)));
        assert_eq!(c.node.style.radius, Some(c.node.radius));
        log.borrow_mut().push(format!("label:{}", c.datum.id));
        Ok(if c.source_index == 3 {
            String::new()
        } else {
            format!("{}:{}", c.datum.id, c.node.radius)
        })
    };
    let layout = tree_layout_with_factories(
        &config(),
        &data,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut node_color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
    )
    .unwrap();
    let order = [1, 2, 4, 0, 3];
    let mut expected = vec![];
    for i in order {
        expected.extend([
            format!("size:{}", data[i].id),
            format!("color:{}", data[i].id),
        ]);
    }
    for i in [2, 4, 0, 3] {
        expected.extend([
            format!("width:{}", data[i].id),
            format!("link-color:{}", data[i].id),
        ]);
    }
    for i in order {
        expected.push(format!("label:{}", data[i].id));
    }
    assert_eq!(*log.borrow(), expected);
    assert_eq!(
        layout.nodes.iter().map(|n| n.position).collect::<Vec<_>>(),
        base.nodes.iter().map(|n| n.position).collect::<Vec<_>>()
    );
    for node in &layout.nodes {
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == node.mark_id.as_ref())
            .unwrap();
        assert_eq!(mark.fill, node.color);
        assert_eq!(mark.label, data[node.source_index].label);
        assert_eq!(mark.value, node.value);
        assert_eq!(mark.opacity, 0.);
    }
    let zero_width = layout
        .scene
        .marks
        .iter()
        .find(|m| m.id == "link:b")
        .unwrap();
    assert_eq!(zero_width.stroke, None);
    assert_eq!(layout.scene.labels.len(), 5);
    let index = *layout
        .scene
        .label_series
        .iter()
        .find(|(_, id)| *id == "zero")
        .unwrap()
        .0;
    assert_eq!(layout.scene.labels[index].text, "");
    let frame = tree_widget_frame(&layout).unwrap();
    assert_eq!(frame.interaction.unwrap().sites.len(), 4);
    let packet = tree_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: 40.,
            y: 40.,
            width: 10.,
            height: 10.,
        })))
    })
    .unwrap();
    assert_eq!(packet.data.len(), 9);
}

#[test]
fn disabled_links_compute_both_functions_and_labels_follow_actual_rust_gate() {
    for visibility in [
        TreeLabels::All,
        TreeLabels::Leaves,
        TreeLabels::Internal,
        TreeLabels::None,
    ] {
        for enabled_labels in [true, false] {
            let mut options = TreeOptions::default();
            options.style.links.enabled = false;
            options.hierarchy.labels = enabled_labels;
            options.style.labels.visibility = visibility;
            let width_calls = Cell::new(0);
            let color_calls = Cell::new(0);
            let label_calls = Cell::new(0);
            let mut width = |_: &TreeLinkFunctionContext<'_>| {
                width_calls.set(width_calls.get() + 1);
                Ok(0.)
            };
            let mut color = |_: &TreeLinkFunctionContext<'_>| {
                color_calls.set(color_calls.get() + 1);
                Ok(Color(1, 2, 3, 255))
            };
            let mut label = |_: &TreeLabelFunctionContext<'_>| {
                label_calls.set(label_calls.get() + 1);
                Ok(String::new())
            };
            let layout = tree_layout_with_factories(
                &config(),
                &source(),
                &options,
                &mut TreeFactories {
                    link_thickness: Some(&mut width),
                    link_color: Some(&mut color),
                    label: Some(&mut label),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(width_calls.get(), 4);
            assert_eq!(color_calls.get(), 4);
            let expected = if !enabled_labels {
                0
            } else {
                match visibility {
                    TreeLabels::All => 5,
                    TreeLabels::Leaves => 3,
                    TreeLabels::Internal => 2,
                    TreeLabels::None => 0,
                }
            };
            assert_eq!(label_calls.get(), expected);
            assert_eq!(layout.scene.labels.len(), expected);
            assert!(layout.links.iter().all(|l| l.mark_id.is_none()));
            assert_eq!(layout.scene.marks.len(), 5);
        }
    }
}

#[test]
fn inherited_link_color_and_label_spacing_use_resolved_node_styles() {
    let mut options = TreeOptions::default();
    options.style.links.color = TreeLinkColor::Source;
    options.style.labels.position = TreeLabelPosition::Right;
    let original = tree_layout(&config(), &source(), &options).unwrap();
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(40.);
    let mut node_color = |c: &TreeNodeFunctionContext<'_>| Ok(color(c.source_index));
    let layout = tree_layout_with_factories(
        &config(),
        &source(),
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut node_color),
            ..Default::default()
        },
    )
    .unwrap();
    for link in &layout.links {
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == link.mark_id.as_ref())
            .unwrap();
        assert_eq!(mark.stroke.unwrap().color, color(link.source_index));
        assert_eq!(
            link.source_position,
            original.nodes[link.source_index].position
        );
        assert_eq!(
            link.target_position,
            original.nodes[link.target_index].position
        );
    }
    for (i, label) in layout.scene.labels.iter().enumerate() {
        assert_eq!(label.x - original.scene.labels[i].x, 16.);
    }
}

#[test]
fn invalid_entire_source_options_or_original_scene_runs_no_functions() {
    let count = Cell::new(0);
    let mut function = |_: &TreeNodeFunctionContext<'_>| {
        count.set(count.get() + 1);
        Ok(10.)
    };
    let mut invalid_source = source();
    invalid_source.last_mut().unwrap().parent = Some("missing".into());
    let mut invalid_options = TreeOptions::default();
    invalid_options.style.links.opacity = f64::NAN;
    let mut invalid_theme = config();
    invalid_theme.theme.font_size = 0.;
    for (c, data, o) in [
        (config(), invalid_source, TreeOptions::default()),
        (config(), source(), invalid_options),
        (invalid_theme, source(), TreeOptions::default()),
    ] {
        assert!(
            tree_layout_with_factories(
                &c,
                &data,
                &o,
                &mut TreeFactories {
                    node_size: Some(&mut function),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(count.get(), 0);
    }
}

#[test]
fn function_output_bounds_and_errors_stop_without_partial_layout() {
    for value in [f64::NAN, f64::INFINITY, -1., 32768.1] {
        let later = Cell::new(0);
        let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(value);
        let mut color = |_: &TreeNodeFunctionContext<'_>| {
            later.set(later.get() + 1);
            Ok(Color(0, 0, 0, 255))
        };
        assert!(
            tree_layout_with_factories(
                &config(),
                &source(),
                &TreeOptions::default(),
                &mut TreeFactories {
                    node_size: Some(&mut size),
                    node_color: Some(&mut color),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(later.get(), 0);
    }
    for value in [f64::NAN, f64::INFINITY, -1., 1e-7, 10000.1] {
        let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(value);
        assert!(
            tree_layout_with_factories(
                &config(),
                &source(),
                &TreeOptions::default(),
                &mut TreeFactories {
                    link_thickness: Some(&mut width),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    for text in ["x".repeat(1025), "界".repeat(342)] {
        let calls = Cell::new(0);
        let mut label = |_: &TreeLabelFunctionContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(text.clone())
        };
        assert!(
            tree_layout_with_factories(
                &config(),
                &source(),
                &TreeOptions::default(),
                &mut TreeFactories {
                    label: Some(&mut label),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(calls.get(), 1);
    }
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        if calls.get() == 3 {
            Err(ChartError("application failure".into()))
        } else {
            Ok(2.)
        }
    };
    let err = tree_layout_with_factories(
        &config(),
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.0, "application failure");
    assert_eq!(calls.get(), 3);
}

#[test]
fn labels_accept_exact_utf8_limit_and_original_raw_zero_source_survives() {
    let mut label = |_: &TreeLabelFunctionContext<'_>| Ok("界".repeat(341) + "x");
    let layout = tree_layout_with_factories(
        &config(),
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(layout.scene.labels.iter().all(|l| l.text.len() == 1024));
    assert_eq!(layout.source()[3].value.to_bits(), (-0f64).to_bits());
    assert_eq!(layout.nodes[3].raw_value.to_bits(), (-0f64).to_bits());
    let packet = tree_node_widgets(&layout, 0, |c| {
        Ok(
            (c.source_index == 0).then_some(PlotWidgetPlacement::Rect(LayoutRect {
                x: 40.,
                y: 40.,
                width: 10.,
                height: 10.,
            })),
        )
    })
    .unwrap();
    assert_eq!(
        packet.data.values().next().unwrap().source[3]
            .value
            .to_bits(),
        (-0f64).to_bits()
    );
}

#[test]
fn empty_source_has_accurate_plot_and_no_function_calls() {
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(0.)
    };
    let layout = tree_layout_with_factories(
        &config(),
        &[],
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert_eq!(layout.plot, ClipRect::plot(&config()));
    assert!(
        tree_widget_frame(&layout)
            .unwrap()
            .interaction
            .unwrap()
            .sites
            .is_empty()
    );
}
