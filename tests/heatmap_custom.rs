use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 180.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        labels: false,
        ..Default::default()
    }
}
fn data() -> Vec<HeatCell> {
    vec![
        HeatCell {
            row: "r".into(),
            column: "null".into(),
            value: None,
        },
        HeatCell::new("r", "zero", 0.),
        HeatCell::new("r", "value", 7.),
    ]
}
fn stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(10, 20, 30, 255),
        width: 2.,
        cap: StrokeCap::Butt,
    }
}
fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + w, y)),
        C::Line(P::new(x + w, y + h)),
        C::Line(P::new(x, y + h)),
        C::Close,
    ])
}
fn diamond(x: f64, y: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y - 20.)),
        C::Line(P::new(x + 20., y)),
        C::Line(P::new(x, y + 20.)),
        C::Line(P::new(x - 20., y)),
        C::Close,
    ])
}

#[test]
fn original_rect_circle_rounding_nullable_and_size_options_keep_default_geometry() {
    for shape in [HeatmapShape::Rect, HeatmapShape::Circle] {
        for corner_radius in [0., 8.] {
            let o = HeatmapLayoutOptions {
                shape,
                corner_radius,
                size_variation: Some(HeatmapSizeVariation::default()),
                ..options()
            };
            let default = heatmap_layout(&config(), &data(), &o).unwrap();
            let custom = heatmap_layout_with_cell_factory(&config(), &data(), &o, |c| {
                Ok(c.inherit(c.geometry.clone()))
            })
            .unwrap();
            assert_eq!(default.cells, custom.cells);
            let mut expected = default.scene.clone();
            // Custom cells explicitly crop to the plot; inherited ink remains identical.
            for m in &mut expected.marks {
                m.clip = Some(custom.plot);
            }
            assert_eq!(expected, custom.scene);
            let binding = custom.interaction_binding().unwrap();
            assert_eq!(
                binding.sites.iter().map(|s| s.value).collect::<Vec<_>>(),
                vec![None, Some(7.), Some(0.)]
            );
        }
    }
}

#[test]
fn callback_order_retains_source_indices_null_zero_and_sparse_hidden_domains() {
    let cells = vec![
        HeatCell::new("z", "two", 100.),
        HeatCell::new("a", "two", 0.),
        HeatCell {
            row: "a".into(),
            column: "one".into(),
            value: None,
        },
    ];
    let o = HeatmapLayoutOptions {
        row_order: Some(vec!["a".into(), "z".into()]),
        column_order: Some(vec!["one".into(), "two".into()]),
        hidden_rows: ["z".into()].into(),
        ..options()
    };
    let expected = heatmap_layout(&config(), &cells, &o).unwrap();
    let mut called = vec![];
    let custom = heatmap_layout_with_color_profile_and_cell_factory(
        &config(),
        &cells,
        &o,
        ColorScheme::Viridis,
        |ctx| {
            called.push((ctx.source_index, ctx.datum.value, ctx.key.clone()));
            Ok(ctx.inherit(ctx.geometry.clone()))
        },
    )
    .unwrap();
    assert_eq!(called.iter().map(|r| r.0).collect::<Vec<_>>(), vec![2, 1]);
    assert_eq!(called[0].1, None);
    assert_eq!(called[1].1, Some(0.));
    assert_eq!(custom.cells.len(), 3);
    assert_eq!(custom.cells.iter().filter(|c| c.visible).count(), 2);
    assert_eq!(custom.color_scale.domain(), expected.color_scale.domain());
    assert!(custom.cell(&HeatmapCellKey::new("z", "one")).is_none());
    assert_eq!(custom.interaction_binding().unwrap().sites.len(), 2);
}

#[test]
fn custom_ink_keeps_complete_resolved_data_scale_and_actual_color_legend() {
    let o = HeatmapLayoutOptions {
        colors: ColorScaleOptions {
            kind: ColorScaleKind::SequentialQuantile,
            ..Default::default()
        },
        ..options()
    };
    let mut source = data();
    source.push(HeatCell::new("r", "outlier", 200.));
    let standard =
        heatmap_layout_with_color_profile(&config(), &source, &o, ColorScheme::Viridis).unwrap();
    let custom = heatmap_layout_with_color_profile_and_cell_factory(
        &config(),
        &source,
        &o,
        ColorScheme::Viridis,
        |ctx| {
            let mut cell = ctx.inherit(diamond(ctx.position[0], ctx.position[1]));
            cell.fill = Some(Color(200, 40, 20, 255));
            Ok(cell)
        },
    )
    .unwrap();
    assert_eq!(standard.color_scale, custom.color_scale);
    let mut original_legend = config().scene().unwrap();
    let mut custom_legend = config().scene().unwrap();
    let options = ColorLegendOptions {
        position: [40., 40.],
        ..Default::default()
    };
    let a = append_color_legend(&mut original_legend, &standard.color_scale, &options).unwrap();
    let b = append_color_legend(&mut custom_legend, &custom.color_scale, &options).unwrap();
    assert_eq!(a, b);
    assert_eq!(original_legend, custom_legend);
    assert_eq!(
        custom.cells.iter().map(|c| c.value).collect::<Vec<_>>(),
        standard.cells.iter().map(|c| c.value).collect::<Vec<_>>()
    );
}

#[test]
fn factory_paints_and_actual_ink_are_distinct_from_logical_origins() {
    let c = config();
    let cells = [HeatCell::new("r", "a", 2.)];
    let custom = heatmap_layout_with_cell_factory(&c, &cells, &options(), |ctx| {
        let mut cell = ctx.inherit(rectangle(280., 80., 50., 20.));
        cell.fill = Some(Color(20, 130, 70, 255));
        Ok(cell)
    })
    .unwrap();
    let cell = &custom.cells[0];
    assert_eq!(cell.center, Some([160., 90.]));
    assert!(custom.hit_test(160., 90.).is_none());
    assert_eq!(custom.hit_test(290., 90.).unwrap().key, cell.key);
    assert!(
        custom.hit_test(310., 90.).is_none(),
        "plot clips custom ink at300"
    );
    let binding = custom.interaction_binding().unwrap();
    assert_eq!(binding.sites[0].position, [160., 90.]);
    assert_eq!(
        binding.anchor_for(&custom.scene.marks[0]).unwrap(),
        Some([160., 90.])
    );
    assert_eq!(
        custom.scene.marks[0].geometry,
        rectangle(280., 80., 50., 20.)
    );
}

#[test]
fn controlled_hover_preserves_geometry_paint_nullable_data_and_cell_opacity_factors() {
    let o = HeatmapLayoutOptions {
        labels: true,
        opacity: 0.8,
        active_opacity: 1.,
        inactive_opacity: 0.1,
        hover_target: HeatmapHoverTarget::Cell,
        ..options()
    };
    let mut layout = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut cell = ctx.inherit(diamond(ctx.position[0], ctx.position[1]));
        cell.opacity = 0.5;
        cell.fill_paint = Some(FillPaint::Linear {
            start: [20., 90.],
            end: [300., 90.],
            stops: vec![
                GradientStop::new(0., Color(0, 80, 200, 255)),
                GradientStop::new(1., Color(0, 200, 80, 255)),
            ],
        });
        Ok(cell)
    })
    .unwrap();
    let before = layout.scene.clone();
    layout
        .set_active(Some(HeatmapCellKey::new("r", "null")))
        .unwrap();
    for (old, mark) in before.marks.iter().zip(&layout.scene.marks) {
        assert_eq!(old.geometry, mark.geometry);
        assert_eq!(old.fill_paint, mark.fill_paint);
        assert_eq!(old.clip, mark.clip);
        let expected = if mark.id == HeatmapCellKey::new("r", "null").mark_id() {
            0.5
        } else {
            0.05
        };
        assert_eq!(mark.opacity, expected);
    }
    for cell in &layout.cells {
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == cell.mark_id)
            .unwrap();
        assert_eq!(cell.opacity, mark.opacity);
        let index = layout
            .scene
            .labels
            .iter()
            .position(|l| l.text == cell.formatted_value.as_deref().unwrap_or("missing"))
            .unwrap();
        assert_eq!(
            layout.scene.label_series.get(&index),
            Some(&"r".to_string())
        );
        assert_eq!(
            layout.scene.labels[index].color.3,
            (255. * cell.opacity).round() as u8
        );
    }
    layout.interaction_binding().unwrap();
    layout.set_active(None).unwrap();
    assert_eq!(layout.scene, before);
}

#[test]
fn altered_public_custom_metadata_geometry_paint_labels_and_opacity_reject_atomically() {
    let original = heatmap_layout_with_cell_factory(
        &config(),
        &data(),
        &HeatmapLayoutOptions {
            labels: true,
            ..options()
        },
        |ctx| Ok(ctx.inherit(diamond(ctx.position[0], ctx.position[1]))),
    )
    .unwrap();
    for mutation in 0..7 {
        let mut layout = original.clone();
        match mutation {
            0 => layout.cells[0].value = Some(0.),
            1 => layout.cells[0].center = Some([1., 1.]),
            2 => layout.scene.marks[0].geometry = rectangle(40., 40., 20., 20.),
            3 => layout.scene.marks[0].fill = Some(Color(255, 0, 0, 255)),
            4 => layout.scene.labels[0].text = "forged".into(),
            5 => {
                layout.cells[0].opacity = 0.1;
                layout.scene.marks[0].opacity = 0.1;
            }
            _ => layout.rows.reverse(),
        }
        // A single row reverses identically: make the final mutation material.
        if mutation == 6 {
            layout.rows.push("forged".into());
        }
        let scene = layout.scene.clone();
        let cells = layout.cells.clone();
        let active = layout.active.clone();
        assert!(
            layout
                .set_active(Some(HeatmapCellKey::new("r", "zero")))
                .is_err(),
            "mutation{mutation}"
        );
        assert_eq!(layout.scene, scene);
        assert_eq!(layout.cells, cells);
        assert_eq!(layout.active, active);
        assert!(layout.interaction_binding().is_err());
    }
}

#[test]
fn new_binding_api_rejects_a_missing_public_default_position_without_panicking() {
    let mut layout = heatmap_layout(&config(), &data(), &options()).unwrap();
    layout.cells[0].center = None;
    assert!(layout.interaction_binding().is_err());
}

#[test]
fn malformed_factory_outputs_and_callback_errors_preserve_inputs_and_an_accepted_layout() {
    let cells = data();
    let backup = cells.clone();
    let accepted = heatmap_layout(&config(), &cells, &options()).unwrap();
    let scene = accepted.scene.clone();
    for mutation in 0..8 {
        let result = heatmap_layout_with_cell_factory(&config(), &cells, &options(), |ctx| {
            let mut g = ctx.inherit(ctx.geometry.clone());
            match mutation {
                0 => return Err(ChartError("application rejected cell".into())),
                1 => {
                    g.geometry = Geometry::Circle {
                        x: f64::NAN,
                        y: 90.,
                        radius: 4.,
                    }
                }
                2 => g.geometry = Geometry::Path(vec![C::Line(P::new(1., 1.))]),
                3 => g.opacity = 1.1,
                4 => {
                    g.stroke = Some(GraphicStroke {
                        width: -1.,
                        ..stroke()
                    })
                }
                5 => {
                    g.fill_paint = Some(FillPaint::Linear {
                        start: [0., 0.],
                        end: [0., 0.],
                        stops: vec![],
                    })
                }
                6 => {
                    g.stroke_dash = Some(StrokeDash {
                        segments: vec![-1.],
                        offset: 0.,
                    })
                }
                _ => {
                    g.geometry = Geometry::Circle {
                        x: 1.,
                        y: 1.,
                        radius: 4.,
                    }
                }
            }
            Ok(g)
        });
        assert!(result.is_err(), "mutation{mutation}");
        assert_eq!(cells, backup);
        assert_eq!(accepted.scene, scene);
    }
}

#[test]
fn aggregate_path_and_gradient_stop_budgets_reject_before_accepting_a_frame() {
    let path = heatmap_layout_with_cell_factory(&config(), &data(), &options(), |ctx| {
        let mut commands = vec![C::Move(P::new(ctx.position[0], ctx.position[1]))];
        commands
            .extend((0..70_000).map(|_| C::Line(P::new(ctx.position[0] + 1., ctx.position[1]))));
        Ok(ctx.inherit(Geometry::Path(commands)))
    });
    assert!(path.is_err());
    let cells = (0..33)
        .map(|i| HeatCell::new("r", i.to_string(), i as f64))
        .collect::<Vec<_>>();
    let gradient = heatmap_layout_with_cell_factory(&config(), &cells, &options(), |ctx| {
        let mut cell = ctx.inherit(ctx.geometry.clone());
        cell.fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [320., 0.],
            stops: (0..512)
                .map(|i| GradientStop::new(i as f64 / 511., Color(10, 20, 30, 255)))
                .collect(),
        });
        Ok(cell)
    });
    assert!(gradient.is_err());
}

#[cfg(feature = "raster")]
#[test]
fn actual_custom_geometry_gradient_pattern_and_hollow_stroke_render_in_cpu_svg_canvas() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    use kartis::render::{self};
    let l = heatmap_layout_with_cell_factory(&config(), &data(), &options(), |ctx| {
        let [x, y] = ctx.position;
        let mut cell = ctx.inherit(diamond(x, y));
        match ctx.datum.value {
            None => {
                cell.fill_paint = Some(FillPaint::Pattern(PatternFill {
                    shape: PatternShape::Lines { width: 4. },
                    spacing: 8.,
                    foreground: Color(200, 30, 40, 255),
                    background: Color(20, 80, 150, 255),
                    ..Default::default()
                }))
            }
            Some(0.) => {
                cell.geometry = Geometry::Circle { x, y, radius: 20. };
                cell.fill = None;
                cell.stroke = Some(stroke());
            }
            Some(_) => {
                cell.fill_paint = Some(FillPaint::Linear {
                    start: [x - 20., y],
                    end: [x + 20., y],
                    stops: vec![
                        GradientStop::new(0., Color(0, 200, 0, 255)),
                        GradientStop::new(1., Color(0, 0, 200, 255)),
                    ],
                })
            }
        }
        Ok(cell)
    })
    .unwrap();
    let svg = render::to_svg(&l.scene).unwrap();
    let js = render::to_canvas_script(&l.scene, "fixture").unwrap();
    assert!(svg.contains("<pattern") && svg.contains("<linearGradient") && svg.contains("<path"));
    assert!(
        js.contains("createPattern")
            && js.contains("createLinearGradient")
            && js.contains("lineTo")
    );
    for dpr in [1., 2.] {
        let image = render_raster(
            &l.scene,
            &FontRegistry::new(),
            &RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
        .unwrap();
        let pixel = |x: f64, y: f64| {
            let i = ((y * dpr) as usize * image.width as usize + (x * dpr) as usize) * 4;
            image.rgba[i..i + 4].to_vec()
        };
        let n = l
            .cell(&HeatmapCellKey::new("r", "null"))
            .unwrap()
            .center
            .unwrap();
        let z = l
            .cell(&HeatmapCellKey::new("r", "zero"))
            .unwrap()
            .center
            .unwrap();
        let v = l
            .cell(&HeatmapCellKey::new("r", "value"))
            .unwrap()
            .center
            .unwrap();
        // Pattern lattice is Scene-global, not cell-relative. Pixel centers
        // y88.5 (mod8=.5) and92.5 (mod8=4.5) are inside stripe/background.
        assert_eq!(pixel(n[0], 88.), vec![200, 30, 40, 255]);
        assert_eq!(pixel(n[0], 92.), vec![20, 80, 150, 255]);
        assert_eq!(pixel(z[0], z[1]), vec![255, 255, 255, 255]);
        // Source radius20 and width2: choose the full stroke interior.
        // +19.5 floored into an inner antialias boundary at DPR1.
        assert_eq!(pixel(z[0] + 20., z[1]), vec![10, 20, 30, 255]);
        let midpoint = pixel(v[0], v[1]);
        assert!(midpoint[1] > 80 && midpoint[1] < 120 && midpoint[2] > 80 && midpoint[2] < 120);
    }
}
