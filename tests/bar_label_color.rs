use kartis::*;
use std::collections::BTreeMap;
fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        ..Default::default()
    }
}
fn styles(profile: InheritedColor) -> BarStyleOptions {
    BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            color_profile: Some(profile),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn label_color(layout: &StyledBarLayout) -> Color {
    layout.scene.labels[layout.bars[0].label_index.unwrap()].color
}
#[cfg(feature = "geojson")]
#[test]
fn original_d3_typed_rgba_and_css_boundary_are_both_preserved() -> Result<()> {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bar-label-color/values.json")).unwrap();
    let (mut ordinary, mut boundary) = (0, 0);
    for row in cases.as_array().unwrap() {
        let bytes = |v: &serde_json::Value| {
            Color(
                v[0].as_u64().unwrap() as u8,
                v[1].as_u64().unwrap() as u8,
                v[2].as_u64().unwrap() as u8,
                v[3].as_u64().unwrap() as u8,
            )
        };
        let input = bytes(&row["input"]);
        let modifiers = row["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let amount = v[1].as_f64().unwrap();
                match v[0].as_str().unwrap() {
                    "brighter" => ColorModifier::Brighter(amount),
                    "darker" => ColorModifier::Darker(amount),
                    _ => ColorModifier::Opacity(amount),
                }
            })
            .collect();
        let mut style = styles(InheritedColor::FromContext(modifiers));
        style.colors.insert("s".into(), input);
        let l = bar_layout_with_style(
            &config(),
            &["A".into()],
            &[BarSeries::new("s", vec![Some(2.)])],
            &Default::default(),
            &style,
        )?;
        assert_eq!(label_color(&l), bytes(&row["nativeTypedRGBA"]));
        assert_eq!(l.bars[0].raw_value, 2.);
        assert_eq!(l.bars[0].color, input);
        if row["profile"] == "ordinary-exact" {
            assert_eq!(label_color(&l), bytes(&row["nativeNivoCSS"]));
            ordinary += 1;
        } else {
            assert_ne!(row["nativeTypedRGBA"], row["nativeNivoCSS"]);
            assert!(
                row["nativeCSSRawChannels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == "n:NaN")
            );
            boundary += 1;
        }
    }
    assert_eq!((ordinary, boundary), (36, 6));
    Ok(())
}
#[test]
fn fixed_theme_contrast_and_category_roles_share_numeric_layout() -> Result<()> {
    let cats = ["dark".to_string(), "light".to_string()];
    let source = [BarSeries::new("s", vec![Some(2.), Some(4.)])];
    let mut style = styles(InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    });
    style.color_by = BarColorBy::Category;
    style.colors = BTreeMap::from([
        ("dark".into(), Color(20, 30, 40, 255)),
        ("light".into(), Color(240, 245, 250, 255)),
    ]);
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            let o = BarOptions {
                horizontal,
                layout,
                category_window: Some([0.1, 0.9]),
                ..Default::default()
            };
            let l = bar_layout_with_style(&config(), &cats, &source, &o, &style)?;
            for bar in &l.bars {
                let color = l.scene.labels[bar.label_index.unwrap()].color;
                assert_eq!(
                    color,
                    if bar.category == "dark" {
                        Color(255, 255, 255, 255)
                    } else {
                        Color(0, 0, 0, 255)
                    }
                );
            }
            let mut legacy = style.clone();
            legacy.labels.color_profile = None;
            let old = bar_layout_with_style(&config(), &cats, &source, &o, &legacy)?;
            assert_eq!(l.bars, old.bars);
            assert_eq!(l.value_domain, old.value_domain);
            assert_eq!(l.scene.marks, old.scene.marks);
        }
    }
    let mut theme = styles(InheritedColor::Theme);
    theme.labels.style.color = Some(Color(10, 20, 30, 137));
    assert_eq!(
        label_color(&bar_layout_with_style(
            &config(),
            &cats,
            &source,
            &Default::default(),
            &theme
        )?),
        Color(10, 20, 30, 137)
    );
    theme.labels.color_profile = Some(InheritedColor::Fixed(Color(90, 80, 70, 51)));
    assert_eq!(
        label_color(&bar_layout_with_style(
            &config(),
            &cats,
            &source,
            &Default::default(),
            &theme
        )?),
        Color(90, 80, 70, 51)
    );
    Ok(())
}
#[test]
fn custom_paint_is_independent_and_alpha_hover_source_guards_restore() -> Result<()> {
    let categories = ["A", "B", "C", "D"].map(str::to_string);
    let data = [BarSeries::new(
        "s",
        vec![Some(2.), Some(3.), Some(0.), None],
    )];
    let mut style = styles(InheritedColor::FromContext(vec![
        ColorModifier::Darker(1.),
        ColorModifier::Opacity(0.8),
    ]));
    style.colors.insert("s".into(), Color(20, 30, 40, 255));
    style.labels.style.color = Some(Color(99, 99, 99, 137));
    let mut frame = bar_layout_with_bar_factory(
        &config(),
        &categories,
        &data,
        &Default::default(),
        &style,
        |c| {
            assert_eq!(c.color, Color(20, 30, 40, 255));
            let mut g = c.inherit(c.geometry.clone());
            g.opacity = 0.5;
            g.fill_paint = Some(FillPaint::Linear {
                start: [0., 0.],
                end: [1., 0.],
                stops: vec![
                    GradientStop::new(0., Color(0, 255, 0, 255)),
                    GradientStop::new(1., Color(0, 255, 0, 255)),
                ],
            });
            Ok(g)
        },
    )?;
    let binding = frame.interaction_binding()?;
    assert_eq!(binding.sites.len(), 3);
    assert!(binding.sites.iter().any(|s| s.value == Some(0.)));
    assert!(
        binding
            .sites
            .iter()
            .all(|s| s.color == Color(0, 255, 0, 255))
    );
    assert!(
        frame
            .layout
            .bars
            .iter()
            .filter(|b| b.raw_value == 0.)
            .all(|b| b.label_index.is_none())
    );
    let before = frame.layout.clone();
    for b in &before.bars {
        if let Some(i) = b.label_index {
            assert_eq!(before.scene.labels[i].color, Color(14, 21, 28, 102));
        }
    }
    let active = frame.layout.bars[0].id.clone();
    frame.set_active(Some(&active), 0.25)?;
    for b in &frame.layout.bars {
        if let Some(i) = b.label_index {
            assert_eq!(
                frame.layout.scene.labels[i].color,
                Color(14, 21, 28, if b.id == active { 102 } else { 26 })
            );
        }
    }
    frame.interaction_binding()?;
    frame.set_active(None, 0.25)?;
    assert_eq!(frame.layout, before);
    let label = frame.layout.bars[0].label_index.unwrap();
    for change in 0..3 {
        let mut bad = frame.clone();
        match change {
            0 => bad.layout.scene.labels[label].color = Color(1, 2, 3, 4),
            1 => {
                bad.layout.scene.label_series.remove(&label);
            }
            _ => {
                bad.layout.scene.label_clips.remove(&label);
            }
        }
        let observed = bad.layout.clone();
        assert!(bad.interaction_binding().is_err());
        assert!(bad.set_active(Some(&active), 0.2).is_err());
        assert_eq!(bad.layout, observed);
    }
    let mut hidden = BarOptions::default();
    hidden.hidden.insert("s".into());
    let l = bar_layout_with_style(&config(), &categories, &data, &hidden, &style)?;
    assert!(l.bars.is_empty());
    Ok(())
}
#[test]
fn invalid_profiles_and_contrast_context_fail_before_callbacks_even_without_labels() -> Result<()> {
    let invalid = [
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(1.1)]),
        InheritedColor::FromContext(vec![ColorModifier::Brighter(1.); 33]),
    ];
    for profile in invalid {
        for mode in 0..5 {
            let (cats, data) = if mode == 0 {
                (vec![], vec![])
            } else {
                (
                    vec!["A".into()],
                    vec![BarSeries::new(
                        "s",
                        vec![match mode {
                            2 => None,
                            3 => Some(0.),
                            _ => Some(2.),
                        }],
                    )],
                )
            };
            let mut o = BarOptions::default();
            if mode == 1 {
                o.hidden.insert("s".into());
            }
            let mut style = styles(profile.clone());
            if mode == 4 {
                style.labels.enabled = false;
            }
            let mut calls = 0;
            let result = bar_layout_with_bar_factory(&config(), &cats, &data, &o, &style, |c| {
                calls += 1;
                Ok(c.inherit(c.geometry.clone()))
            });
            let text = result.unwrap_err().to_string();
            assert!(
                text.contains("color") || text.contains("inherited"),
                "{text}"
            );
            assert_eq!(calls, 0);
        }
    }
    let mut cfg = config();
    cfg.theme.background = Color(0, 0, 0, 0);
    let mut style = styles(InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    });
    style.colors.insert("s".into(), Color(9, 8, 7, 0));
    style.labels.enabled = false;
    let mut hidden = BarOptions::default();
    hidden.hidden.insert("s".into());
    let source = [BarSeries::new("s", vec![Some(0.)])];
    let mut calls = 0;
    let result = bar_layout_with_bar_factory(&cfg, &["A".into()], &source, &hidden, &style, |c| {
        calls += 1;
        Ok(c.inherit(c.geometry.clone()))
    });
    assert!(result.unwrap_err().to_string().contains("opaque"));
    assert_eq!(calls, 0);
    // No fabricated surface is introduced for truly absent Bar values.
    assert!(
        bar_layout_with_style(
            &cfg,
            &["A".into()],
            &[BarSeries::new("s", vec![None])],
            &hidden,
            &style
        )
        .is_ok()
    );
    Ok(())
}
