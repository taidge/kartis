use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 960.,
        height: 640.,
        ..Default::default()
    }
}
fn options() -> CalendarLayoutOptions {
    CalendarLayoutOptions {
        mode: CalendarLayoutMode::FullYears,
        start: Some((2023, 1, 1)),
        end: Some((2024, 12, 31)),
        weekday_ticks: vec![],
        month_labels: false,
        year_legend: true,
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn same_layout(a: &CalendarLayout, b: &CalendarLayout) {
    assert_eq!(a.scene, b.scene);
    assert_eq!(a.days, b.days);
    assert_eq!(a.months, b.months);
    assert_eq!(a.panels, b.panels);
    assert_eq!(a.start, b.start);
    assert_eq!(a.end, b.end);
    assert_eq!(
        format!("{:?}", a.color_scale),
        format!("{:?}", b.color_scale)
    );
}

#[test]
fn absent_profile_and_factory_preserve_complete_layout_and_color_profile() {
    let c = config();
    for orientation in [
        CalendarOrientation::Horizontal,
        CalendarOrientation::Vertical,
    ] {
        let mut o = options();
        o.orientation = orientation;
        o.month_labels = true;
        o.day_labels = CalendarDayLabel::DayOfMonth;
        o.month_spacing = 2.;
        same_layout(
            &calendar_layout(&c, &[], &o).unwrap(),
            &calendar_layout_with_year_legend(&c, &[], &o, None, None).unwrap(),
        );
        same_layout(
            &calendar_layout_with_color_profile(&c, &[], &o, ColorInterpolation::Lab).unwrap(),
            &calendar_layout_with_color_profile_and_year_legend(
                &c,
                &[],
                &o,
                ColorInterpolation::Lab,
                None,
                None,
            )
            .unwrap(),
        );
    }
}

#[test]
fn all_four_positions_use_actual_panel_bounds_with_spacing_and_inherited_ink() {
    let c = config();
    for orientation in [
        CalendarOrientation::Horizontal,
        CalendarOrientation::Vertical,
    ] {
        for position in [
            CalendarLegendPosition::Before,
            CalendarLegendPosition::After,
        ] {
            for offset in [-1024., -12., 0., 17., 1024.] {
                let mut o = options();
                o.orientation = orientation;
                o.month_spacing = 3.;
                o.year_style = CalendarTextStyle {
                    size: Some(21.),
                    color: Some(Color(180, 30, 40, 255)),
                    rotation: 30.,
                };
                let old = calendar_layout(&c, &[], &o).unwrap();
                let l = calendar_layout_with_year_legend(
                    &c,
                    &[],
                    &o,
                    Some(&CalendarYearLegendOptions { position, offset }),
                    None,
                )
                .unwrap();
                assert_eq!(old.days, l.days);
                assert_eq!(old.months, l.months);
                assert_eq!(old.panels, l.panels);
                assert_eq!(old.scene.marks, l.scene.marks);
                assert_eq!(old.scene.legend, l.scene.legend);
                for (label, panel) in l.scene.labels.iter().zip(&l.panels) {
                    let g = panel.grid;
                    let before = position == CalendarLegendPosition::Before;
                    let (x, y, rotation) = if orientation == CalendarOrientation::Horizontal {
                        (
                            if before {
                                g.x - offset
                            } else {
                                g.x + g.width + offset
                            },
                            g.y + g.height / 2.,
                            -60_f64,
                        )
                    } else {
                        (
                            g.x + g.width / 2.,
                            if before {
                                g.y - offset
                            } else {
                                g.y + g.height + offset
                            },
                            30_f64,
                        )
                    };
                    // Kartis keeps its rotated size/3 alphabetic-baseline shift.
                    near(label.x, x - rotation.to_radians().sin() * 7.);
                    near(label.y, y + rotation.to_radians().cos() * 7.);
                    assert_eq!(label.rotation, rotation);
                    assert_eq!(label.anchor, TextAnchor::Middle);
                    assert_eq!(label.size, 21.);
                    assert_eq!(label.color, o.year_style.color.unwrap());
                    assert_eq!(label.text, panel.year.unwrap().to_string());
                }
                assert_eq!(l.scene.labels.len(), 2);
            }
        }
    }
}

#[test]
fn borrowed_factory_visits_real_year_labels_and_keeps_empty_slots_and_legends() {
    let c = config();
    let mut o = options();
    o.hidden_years.insert(2023);
    let old = calendar_layout(&c, &[], &o).unwrap();
    let prefix = String::from("Fiscal <&> Å ");
    let mut calls = vec![];
    let mut factory = |context: &CalendarYearLegendContext| {
        calls.push(*context);
        Ok(if context.year == 2023 {
            String::new()
        } else {
            format!("{prefix}{}", context.year)
        })
    };
    let l = calendar_layout_with_year_legend(
        &c,
        &[],
        &o,
        Some(&CalendarYearLegendOptions::default()),
        Some(&mut factory),
    )
    .unwrap();
    assert_eq!(calls.len(), 2); // Hidden data retains shared year decoration.
    for (i, call) in calls.iter().enumerate() {
        assert_eq!(call.index, i);
        assert_eq!(call.year, 2023 + i as i32);
        assert_eq!(call.grid, l.panels[i].grid);
    }
    assert_eq!(l.scene.labels.len(), 2);
    assert_eq!(l.scene.labels[0].text, "");
    assert_eq!(l.scene.labels[1].text, "Fiscal <&> Å 2024");
    assert_eq!(old.scene.legend, l.scene.legend);
    assert_eq!(old.days, l.days);
    assert_eq!(old.scene.label_series, l.scene.label_series);
    let svg = render::to_svg(&l.scene).unwrap();
    assert!(svg.contains("Fiscal &lt;&amp;&gt; Å 2024"));
    assert!(svg.contains("rotate(-90"));
    let canvas = render::to_canvas_script(&l.scene, "calendar").unwrap();
    assert!(canvas.contains("Fiscal \\u003c\\u0026\\u003e Å 2024"));
}

#[test]
fn factory_alone_keeps_old_header_geometry_and_disabled_labels_skip_callbacks() {
    let c = config();
    let mut o = options();
    let old = calendar_layout(&c, &[], &o).unwrap();
    let mut factory = |context: &CalendarYearLegendContext| Ok(format!("Year {}", context.year));
    let l = calendar_layout_with_year_legend(&c, &[], &o, None, Some(&mut factory)).unwrap();
    let mut expected = old.scene.clone();
    for label in &mut expected.labels {
        label.text = format!("Year {}", label.text);
    }
    assert_eq!(expected, l.scene);
    o.year_labels = Some(false);
    let mut calls = 0;
    let mut forbidden = |_: &CalendarYearLegendContext| {
        calls += 1;
        Err(ChartError("must not run".into()))
    };
    let l = calendar_layout_with_year_legend(
        &c,
        &[],
        &o,
        Some(&CalendarYearLegendOptions::default()),
        Some(&mut forbidden),
    )
    .unwrap();
    assert_eq!(calls, 0);
    assert!(l.scene.labels.is_empty());
    assert_eq!(l.scene.legend.len(), 2);
}

#[test]
fn invalid_profiles_and_compact_reject_before_factory() {
    let c = config();
    let mut o = options();
    let mut calls = 0;
    let mut factory = |_: &CalendarYearLegendContext| {
        calls += 1;
        Ok(String::new())
    };
    for offset in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1024.001,
        1024.001,
    ] {
        assert!(
            calendar_layout_with_year_legend(
                &c,
                &[],
                &o,
                Some(&CalendarYearLegendOptions {
                    offset,
                    ..Default::default()
                }),
                Some(&mut factory),
            )
            .is_err()
        );
    }
    o.mode = CalendarLayoutMode::Compact;
    assert!(calendar_layout_with_year_legend(&c, &[], &o, None, Some(&mut factory)).is_err());
    assert!(calendar_layout_with_year_legend(&c, &[], &o, None, None).is_err());
    assert_eq!(calls, 0);
    assert!(calendar_layout(&c, &[], &o).is_ok());
}

#[test]
fn callback_errors_and_utf8_byte_budget_are_atomic() {
    let c = config();
    let o = options();
    let old = calendar_layout(&c, &[], &o).unwrap();
    let mut calls = 0;
    let mut factory = |context: &CalendarYearLegendContext| {
        calls += 1;
        if context.index == 1 {
            Err(ChartError("format failed".into()))
        } else {
            Ok("first".into())
        }
    };
    let error =
        calendar_layout_with_year_legend(&c, &[], &o, None, Some(&mut factory)).unwrap_err();
    assert_eq!(error.0, "format failed");
    assert_eq!(calls, 2);
    for text in ["x".repeat(MAX_LABEL_BYTES), "é".repeat(MAX_LABEL_BYTES / 2)] {
        let mut factory = |_: &CalendarYearLegendContext| Ok(text.clone());
        assert!(calendar_layout_with_year_legend(&c, &[], &o, None, Some(&mut factory)).is_ok());
    }
    let mut too_long = |_: &CalendarYearLegendContext| Ok("é".repeat(MAX_LABEL_BYTES / 2 + 1));
    assert!(calendar_layout_with_year_legend(&c, &[], &o, None, Some(&mut too_long)).is_err());
    same_layout(&old, &calendar_layout(&c, &[], &o).unwrap());
}

#[cfg(feature = "raster")]
#[test]
fn actual_font_raster_paints_relocated_years() {
    use render::raster::{FontRegistry, RasterOptions, render_raster};
    let c = config();
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    for orientation in [
        CalendarOrientation::Horizontal,
        CalendarOrientation::Vertical,
    ] {
        let mut o = options();
        o.orientation = orientation;
        o.year_style.size = Some(25.);
        o.year_style.color = Some(Color(250, 0, 100, 255));
        let l = calendar_layout_with_year_legend(
            &c,
            &[],
            &o,
            Some(&CalendarYearLegendOptions::default()),
            None,
        )
        .unwrap();
        let mut labels_only = l.scene.clone();
        labels_only.marks.clear();
        labels_only.opacity_groups.clear();
        let image = render_raster(&labels_only, &fonts, &RasterOptions::default()).unwrap();
        assert!(
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 200 && p[1] < 50 && p[2] > 70 && p[3] > 0)
        );
    }
}
