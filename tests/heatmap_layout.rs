use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 20.,
            bottom: 20.,
        },
        title: "HeatMap <safe>".into(),
        ..Default::default()
    }
}
fn data() -> Vec<HeatCell> {
    vec![
        HeatCell::new("a", "x", 0.),
        HeatCell {
            row: "a".into(),
            column: "y".into(),
            value: None,
        },
        HeatCell::new("a", "z", 10.),
        HeatCell::new("b", "x", 2.),
        HeatCell::new("b", "y", 5.),
        HeatCell::new("b", "z", 8.),
    ]
}
fn options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(220, 20, 30, 255), Color(20, 40, 220, 255)],
            unknown: Color(120, 130, 140, 255),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn key(row: &str, col: &str) -> HeatmapCellKey {
    HeatmapCellKey::new(row, col)
}
#[test]
fn padding_matches_independent_d3_band_inner_outer_math() {
    let o = HeatmapLayoutOptions {
        x_padding: HeatmapBandPadding {
            inner: 0.2,
            outer: 0.3,
        },
        y_padding: HeatmapBandPadding {
            inner: 0.5,
            outer: 0.1,
        },
        ..options()
    };
    let s = heatmap_layout(&config(), &data(), &o).unwrap();
    let a = s.cell(&key("a", "x")).unwrap().band.unwrap();
    let sx = 300. / 3.4;
    let sy = 200. / 1.7;
    close(a.x, 30. + 0.3 * sx);
    close(a.y, 20. + 0.1 * sy);
    close(a.width, sx * 0.8);
    close(a.height, sy * 0.5);
    let b = s.cell(&key("b", "y")).unwrap().band.unwrap();
    close(b.x - a.x, sx);
    close(b.y - a.y, sy);
    assert!(
        s.hit_test(a.x + a.width + (sx - a.width) / 2., a.y + a.height / 2.)
            .is_none()
    );
}
#[test]
fn actual_circle_and_rounded_rect_boundaries_control_hit_testing() {
    for shape in [HeatmapShape::Circle, HeatmapShape::Rect] {
        let s = heatmap_layout(
            &config(),
            &data(),
            &HeatmapLayoutOptions {
                shape,
                corner_radius: 12.,
                x_padding: HeatmapBandPadding {
                    inner: 0.15,
                    outer: 0.1,
                },
                y_padding: HeatmapBandPadding {
                    inner: 0.2,
                    outer: 0.1,
                },
                border: Some(GraphicStroke {
                    color: Color(0, 0, 0, 255),
                    width: 1.,
                    cap: StrokeCap::Butt,
                }),
                ..options()
            },
        )
        .unwrap();
        let c = s.cell(&key("a", "x")).unwrap();
        let r = c.band.unwrap();
        let p = c.center.unwrap();
        assert_eq!(s.hit_test(p[0], p[1]).unwrap().key, c.key);
        assert!(s.hit_test(r.x + 0.1, r.y + 0.1).is_none());
        assert!(s.hit_test(r.x + r.width + 5., p[1]).is_none());
        assert!(s.hit_test(f64::NAN, p[1]).is_none());
    }
}
#[test]
fn explicit_orders_hidden_rows_columns_and_sparse_pairs_keep_exact_identities() {
    let input = data();
    let o = HeatmapLayoutOptions {
        row_order: Some(vec!["b".into(), "a".into()]),
        column_order: Some(vec!["z".into(), "x".into(), "y".into()]),
        hidden_rows: ["b".into()].into(),
        hidden_columns: ["y".into()].into(),
        row_legend: true,
        ..options()
    };
    let a = heatmap_layout(&config(), &input, &o).unwrap();
    let mut reversed = input.clone();
    reversed.reverse();
    let b = heatmap_layout(&config(), &reversed, &o).unwrap();
    assert_eq!(a.scene, b.scene);
    assert_eq!(a.cells, b.cells);
    assert_eq!(a.rows, ["a"]);
    assert_eq!(a.columns, ["z", "x"]);
    assert_eq!(a.cells.len(), 6);
    assert_eq!(a.scene.marks.len(), 2);
    assert_eq!(a.scene.legend.len(), 2);
    assert!(a.cell(&key("b", "x")).unwrap().geometry.is_none());
    assert!(!a.cell(&key("a", "y")).unwrap().visible);
    assert_eq!(
        a.scene
            .label_series
            .values()
            .filter(|r| r.as_str() == "a")
            .count(),
        2
    );
    let sparse = heatmap_layout(
        &config(),
        &[HeatCell::new("a", "x", 1.), HeatCell::new("b", "y", 2.)],
        &options(),
    )
    .unwrap();
    assert_eq!(sparse.cells.len(), 2);
    assert!(sparse.cell(&key("a", "y")).is_none());
    let weird = heatmap_layout(
        &config(),
        &[HeatCell::new("a:b", "c", 1.), HeatCell::new("a", "b:c", 1.)],
        &options(),
    )
    .unwrap();
    assert_ne!(weird.cells[0].mark_id, weird.cells[1].mark_id);
}
#[test]
fn null_zero_and_size_variation_are_independent_and_nullable_tooltips_are_exact() {
    let mut o = options();
    o.value_format = ValueFormat::Fixed(1);
    o.size_variation = Some(HeatmapSizeVariation {
        domain: Some([0., 10.]),
        sizes: [0., 1.],
    });
    let s = heatmap_layout(&config(), &data(), &o).unwrap();
    let zero = s.cell(&key("a", "x")).unwrap();
    let null = s.cell(&key("a", "y")).unwrap();
    assert_eq!(zero.value, Some(0.));
    assert_eq!(zero.formatted_value.as_deref(), Some("0.0"));
    assert_eq!(null.value, None);
    assert_eq!(null.formatted_value, None);
    assert_ne!(zero.color, null.color);
    assert!(
        s.hit_test(zero.center.unwrap()[0], zero.center.unwrap()[1])
            .is_none()
    );
    assert_eq!(
        s.hit_test(null.center.unwrap()[0], null.center.unwrap()[1])
            .unwrap()
            .key,
        null.key
    );
    assert_eq!(
        null.geometry,
        Some(Geometry::Rect {
            x: null.band.unwrap().x,
            y: null.band.unwrap().y,
            width: null.band.unwrap().width,
            height: null.band.unwrap().height
        })
    );
    let tooltip = TooltipOptions {
        missing: MissingValue::Placeholder("NULL".into()),
        ..Default::default()
    };
    assert!(
        tooltip_data_text(&tooltip, &[null.tooltip_datum()])
            .unwrap()
            .ends_with(": NULL")
    );
    assert!(
        tooltip_data_text(&tooltip, &[zero.tooltip_datum()])
            .unwrap()
            .ends_with(": 0")
    );
}
#[test]
fn all_hover_targets_use_exact_rows_columns_and_include_explicit_nulls() {
    for (target, count) in [
        (HeatmapHoverTarget::Cell, 1),
        (HeatmapHoverTarget::Row, 3),
        (HeatmapHoverTarget::Column, 2),
        (HeatmapHoverTarget::RowColumn, 4),
        (HeatmapHoverTarget::All, 6),
    ] {
        let mut s = heatmap_layout(
            &config(),
            &data(),
            &HeatmapLayoutOptions {
                hover_target: target,
                opacity: 0.8,
                active_opacity: 0.95,
                inactive_opacity: 0.12,
                ..options()
            },
        )
        .unwrap();
        assert_eq!(s.related(&key("a", "x"), target).unwrap().len(), count);
        let original = s.scene.clone();
        s.set_active(Some(key("a", "x"))).unwrap();
        assert_eq!(s.active_ids.len(), count);
        assert_eq!(s.cells.iter().filter(|c| c.opacity == 0.95).count(), count);
        assert!(
            s.cells
                .iter()
                .all(|c| c.opacity == 0.95 || c.opacity == 0.12)
        );
        for (a, b) in original.marks.iter().zip(&s.scene.marks) {
            assert_eq!(a.geometry, b.geometry);
            assert_eq!(a.id, b.id);
        }
        let before = s.scene.clone();
        assert!(s.set_active(Some(key("unknown", "x"))).is_err());
        assert_eq!(s.scene, before);
        s.set_active(None).unwrap();
        assert!(s.cells.iter().all(|c| c.opacity == 0.8));
        s.scene.validate().unwrap();
    }
}
#[test]
fn hidden_controlled_key_has_no_related_cells_and_restores_base_opacity() {
    let s = heatmap_layout(
        &config(),
        &data(),
        &HeatmapLayoutOptions {
            hidden_rows: ["a".into()].into(),
            active: Some(key("a", "x")),
            opacity: 0.8,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(s.active, Some(key("a", "x")));
    assert!(s.active_ids.is_empty());
    assert!(s.cells.iter().all(|c| c.opacity == 0.8));
    assert!(
        s.related(&key("a", "x"), HeatmapHoverTarget::All)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn visible_domain_resolution_and_related_prefix_collisions_are_explicit() {
    let cells = [
        HeatCell::new("a", "x", 0.),
        HeatCell::new("ab", "x", 10.),
        HeatCell::new("a", "xy", 5.),
        HeatCell::new("high", "x", 100.),
    ];
    let o = HeatmapLayoutOptions {
        hidden_rows: ["high".into()].into(),
        colors: ColorScaleOptions::default(),
        ..options()
    };
    let s = heatmap_layout(&config(), &cells, &o).unwrap();
    assert_eq!(s.color_scale.domain(), [0., 10.]);
    assert_eq!(
        s.related(&key("a", "x"), HeatmapHoverTarget::Row)
            .unwrap()
            .iter()
            .map(|c| &c.key)
            .collect::<Vec<_>>(),
        vec![&key("a", "x"), &key("a", "xy")]
    );
    assert_eq!(
        s.related(&key("a", "x"), HeatmapHoverTarget::Column)
            .unwrap()
            .len(),
        2
    );
    let explicit = heatmap_layout(
        &config(),
        &cells,
        &HeatmapLayoutOptions {
            colors: ColorScaleOptions {
                domain: Some([100., 0.]),
                ..Default::default()
            },
            ..o
        },
    )
    .unwrap();
    assert_eq!(explicit.color_scale.domain(), [100., 0.]);
}
#[test]
fn category_axes_apply_ticks_grid_rotation_display_labels_and_text_styles() {
    let mut axis = HeatmapAxis {
        text: HeatmapTextStyle {
            color: Some(Color(15, 65, 140, 255)),
            size: Some(17.),
        },
        ..Default::default()
    };
    axis.options.grid = true;
    axis.options.tick_values = Some(vec!["z".into(), "x".into()]);
    axis.options.tick_rotation = -35.;
    axis.options.legend = Some("Columns".into());
    let o = HeatmapLayoutOptions {
        column_labels: [("x".into(), "Col <X>".into())].into(),
        axes: HeatmapAxes {
            top: Some(axis),
            left: Some(HeatmapAxis::default()),
            right: Some(HeatmapAxis::default()),
            bottom: None,
        },
        ..options()
    };
    let s = heatmap_layout(&config(), &data(), &o).unwrap();
    let label = s.scene.labels.iter().find(|l| l.text == "Col <X>").unwrap();
    assert_eq!(label.size, 17.);
    assert_eq!(label.rotation, -35.);
    assert_eq!(label.color, Color(15, 65, 140, 255));
    assert!(s.scene.marks.iter().any(|m| m.id == "heatmap-grid:top"));
    assert!(s.scene.marks.iter().any(|m| m.id == "heatmap-axis:right"));
    for (i, l) in s.scene.labels.iter().enumerate() {
        if ["Col <X>", "z", "a", "b", "Columns"].contains(&l.text.as_str()) {
            assert!(!s.scene.label_series.contains_key(&i));
        }
    }
    assert!(render::to_svg(&s.scene).unwrap().contains("Col &lt;X&gt;"));
}
#[test]
fn square_pitch_and_null_or_constant_size_domains_have_finite_geometry() {
    let o = HeatmapLayoutOptions {
        force_square: true,
        size_variation: Some(HeatmapSizeVariation {
            domain: None,
            sizes: [0.2, 0.8],
        }),
        ..options()
    };
    let s = heatmap_layout(
        &config(),
        &[HeatCell::new("a", "x", 5.), HeatCell::new("a", "y", 5.)],
        &o,
    )
    .unwrap();
    let c = &s.cells[0];
    let r = c.band.unwrap();
    close(r.width, r.height);
    if let Some(Geometry::Rect { width, height, .. }) = c.geometry {
        close(width, r.width * 0.5);
        close(height, r.height * 0.5);
    } else {
        panic!()
    }
    let null = heatmap_layout(
        &config(),
        &[HeatCell {
            row: "a".into(),
            column: "x".into(),
            value: None,
        }],
        &o,
    )
    .unwrap();
    assert_eq!(null.color_scale.domain(), [0., 10.]);
    let automatic = heatmap_layout(
        &config(),
        &[HeatCell {
            row: "a".into(),
            column: "x".into(),
            value: None,
        }],
        &HeatmapLayoutOptions {
            colors: ColorScaleOptions::default(),
            ..o.clone()
        },
    )
    .unwrap();
    assert_eq!(automatic.color_scale.domain(), [0., 1.]);
    null.scene.validate().unwrap();
    for value in [f64::from_bits(1), -f64::MAX, f64::MAX] {
        let s = heatmap_layout(
            &config(),
            &[HeatCell::new("a", "x", value)],
            &HeatmapLayoutOptions {
                size_variation: Some(HeatmapSizeVariation {
                    domain: Some([-f64::MAX, f64::MAX]),
                    sizes: [0., 1.],
                }),
                ..options()
            },
        )
        .unwrap();
        s.scene.validate().unwrap();
    }
}
#[test]
fn invalid_options_data_and_cache_corruption_reject_transactionally() {
    let input = data();
    let before = input.clone();
    for invalid in [f64::NAN, -0.1, 1.1] {
        let o = HeatmapLayoutOptions {
            x_padding: HeatmapBandPadding {
                inner: invalid,
                outer: 0.,
            },
            ..options()
        };
        assert!(heatmap_layout(&config(), &input, &o).is_err());
    }
    for invalid in [f64::NAN, f64::INFINITY, -1., 2.] {
        let o = HeatmapLayoutOptions {
            inactive_opacity: invalid,
            ..options()
        };
        assert!(heatmap_layout(&config(), &input, &o).is_err());
    }
    assert!(
        heatmap_layout(
            &config(),
            &input,
            &HeatmapLayoutOptions {
                row_order: Some(vec!["a".into(), "a".into()]),
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        heatmap_layout(
            &config(),
            &input,
            &HeatmapLayoutOptions {
                hidden_columns: ["unknown".into()].into(),
                ..options()
            }
        )
        .is_err()
    );
    let mut duplicate = input.clone();
    duplicate.push(input[0].clone());
    assert!(heatmap_layout(&config(), &duplicate, &options()).is_err());
    let mut invalid = input.clone();
    invalid[0].value = Some(f64::NAN);
    assert!(heatmap_layout(&config(), &invalid, &options()).is_err());
    assert_eq!(input, before);
    let mut s = heatmap_layout(&config(), &input, &options()).unwrap();
    s.scene.marks.clear();
    let corrupted = s.scene.clone();
    assert!(s.set_active(Some(key("a", "x"))).is_err());
    assert_eq!(s.scene, corrupted);
    let mut s = heatmap_layout(&config(), &input, &options()).unwrap();
    s.cells.reverse();
    assert!(s.set_active(None).is_err());
}
#[test]
fn sparse_layout_work_tracks_source_cells_instead_of_cartesian_product() {
    let input: Vec<_> = (0..5000)
        .map(|i| {
            HeatCell::new(
                format!("row{i:05}"),
                format!("column{i:05}"),
                (i % 10) as f64,
            )
        })
        .collect();
    let s = heatmap_layout(
        &config(),
        &input,
        &HeatmapLayoutOptions {
            labels: false,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(s.cells.len(), 5000);
    assert_eq!(s.scene.marks.len(), 5000);
    let over = vec![HeatCell::new("a", "x", 1.); MAX_HEATMAP_CELLS + 1];
    assert!(heatmap_layout(&config(), &over, &options()).is_err());
    let empty = heatmap_layout(&config(), &[], &HeatmapLayoutOptions::default()).unwrap();
    assert!(empty.cells.is_empty());
    assert!(empty.scene.marks.is_empty());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_show_distinct_null_zero_circle_border_and_real_padding() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    for shape in [HeatmapShape::Rect, HeatmapShape::Circle] {
        let s = heatmap_layout(
            &config(),
            &data(),
            &HeatmapLayoutOptions {
                shape,
                labels: false,
                corner_radius: 10.,
                border: Some(GraphicStroke {
                    color: Color(0, 0, 0, 255),
                    width: 2.,
                    cap: StrokeCap::Butt,
                }),
                x_padding: HeatmapBandPadding {
                    inner: 0.2,
                    outer: 0.1,
                },
                y_padding: HeatmapBandPadding {
                    inner: 0.2,
                    outer: 0.1,
                },
                ..options()
            },
        )
        .unwrap();
        let frame = render_raster(&s.scene, &fonts, &RasterOptions::default()).unwrap();
        let pixel = |x: f64, y: f64| {
            let i = (y.floor() as usize * frame.width as usize + x.floor() as usize) * 4;
            &frame.rgba[i..i + 4]
        };
        let zero = s.cell(&key("a", "x")).unwrap();
        let null = s.cell(&key("a", "y")).unwrap();
        let z = zero.center.unwrap();
        let n = null.center.unwrap();
        assert_eq!(pixel(z[0], z[1]), &[220, 20, 30, 255]);
        assert_eq!(pixel(n[0], n[1]), &[120, 130, 140, 255]);
        let b = zero.band.unwrap();
        assert_eq!(pixel(b.x + 0.1, b.y + 0.1), &[255, 255, 255, 255]);
        assert_eq!(pixel((z[0] + n[0]) / 2., z[1]), &[255, 255, 255, 255]);
        let border = match zero.geometry.as_ref().unwrap() {
            Geometry::Circle { x, y, radius } => [*x, *y - *radius],
            _ => [z[0], b.y],
        };
        let p = pixel(border[0], border[1]);
        assert!(
            p[0] < 80 && p[1] < 80 && p[2] < 80,
            "missing real border ink {p:?}"
        );
    }
}
