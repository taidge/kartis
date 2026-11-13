use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

const GOLD: Color = Color(240, 190, 65, 255);
const NAVY: Color = Color(32, 45, 80, 255);
const GREEN: Color = Color(45, 158, 122, 255);
const DARK: Color = Color(0, 0, 0, 255);
const LIGHT: Color = Color(255, 255, 255, 255);

fn palette() -> OrdinalColorScale {
    OrdinalColorOptions {
        palette: OrdinalColorPalette::Custom(vec![NAVY, GOLD, GREEN]),
        domain: Some(
            ["North", "South", "Central"]
                .map(CategoryKey::from)
                .to_vec(),
        ),
        ..Default::default()
    }
    .resolve([])
    .unwrap()
}

fn context(inherited: Color, surface: Color, backdrop: Color) -> InheritedColorContext {
    InheritedColorContext {
        inherited,
        theme: GREEN,
        surface,
        backdrop,
    }
}

#[test]
fn ordered_d3_rgb_modifiers_keep_intermediate_channels_before_saturation() {
    let profile = |modifiers| InheritedColor::FromContext(modifiers);
    let values = [
        (GOLD, Color(168, 133, 46, 255)),
        (NAVY, Color(22, 31, 56, 255)),
        (Color(210, 150, 90, 255), Color(147, 105, 63, 255)),
    ];
    // Independent d3-color 3.1.0 RGB darker(1) reference values.
    for (input, darker) in values {
        let sample = context(input, input, LIGHT);
        assert_eq!(
            profile(vec![ColorModifier::Darker(1.)])
                .resolve(sample)
                .unwrap(),
            darker
        );
        assert_eq!(
            profile(vec![
                ColorModifier::Brighter(2.),
                ColorModifier::Opacity(0.42),
                ColorModifier::Darker(2.),
            ])
            .resolve(sample)
            .unwrap(),
            Color(input.0, input.1, input.2, 107)
        );
    }
    let transparent = context(Color(9, 8, 7, 0), DARK, LIGHT);
    assert_eq!(
        profile(vec![ColorModifier::Opacity(1.)])
            .resolve(transparent)
            .unwrap(),
        Color(9, 8, 7, 255)
    );
    assert_eq!(InheritedColor::Theme.resolve(transparent).unwrap(), GREEN);
    assert!(
        profile(vec![ColorModifier::Opacity(f64::NAN)])
            .resolve(transparent)
            .is_err()
    );
    assert!(
        profile(vec![ColorModifier::Darker(1.); 33])
            .resolve(transparent)
            .is_err()
    );
}

#[test]
fn contrast_uses_composited_background_and_candidate_alpha() {
    let choose = InheritedColor::Contrast {
        dark: DARK,
        light: LIGHT,
    };
    let translucent_ink = Color(0, 0, 0, 64);
    let transparent = Color(0, 0, 0, 0);
    assert_eq!(choose.resolve(context(GOLD, NAVY, LIGHT)).unwrap(), LIGHT);
    assert_eq!(
        choose
            .resolve(context(NAVY, translucent_ink, LIGHT))
            .unwrap(),
        DARK
    );
    assert!(
        choose
            .resolve(context(NAVY, translucent_ink, transparent))
            .is_err()
    );
    assert_eq!(
        choose.resolve(context(GOLD, DARK, transparent)).unwrap(),
        LIGHT
    );
    assert_eq!(
        InheritedColor::Contrast {
            dark: GREEN,
            light: transparent,
        }
        .resolve(context(NAVY, DARK, LIGHT))
        .unwrap(),
        GREEN
    );
}

#[test]
fn pie_palette_and_factory_reach_fill_border_labels_and_legend() {
    let config = ChartConfig::default();
    let data = [RadialDatum::new("South", 4.), RadialDatum::new("North", 6.)];
    let mut options = PieLayoutOptions::default();
    options.pie.style.border = Some(GraphicStroke {
        color: GREEN,
        width: 2.,
        cap: StrokeCap::Round,
    });
    options.pie.style.arc_labels.legacy_fit = false;
    options.pie.style.link_labels.enabled = true;
    let styles = PieColorStyles {
        border: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
        arc_label: Some(InheritedColor::Contrast {
            dark: DARK,
            light: LIGHT,
        }),
        link_label: Some(InheritedColor::Contrast {
            dark: DARK,
            light: LIGHT,
        }),
        link_line: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
            0.5,
        )])),
    };
    let scale = palette();
    let layout = pie_layout_with_ordinal_colors(&config, &data, &options, &scale, &styles).unwrap();
    for (name, color, stroke, text) in [
        ("South", GOLD, Color(168, 133, 46, 255), DARK),
        ("North", NAVY, Color(22, 31, 56, 255), LIGHT),
    ] {
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|mark| mark.id == name)
            .unwrap();
        assert_eq!(mark.fill, Some(color));
        assert_eq!(mark.stroke.unwrap().color, stroke);
        assert_eq!(
            layout
                .scene
                .legend
                .iter()
                .find(|item| item.id == name)
                .unwrap()
                .color,
            color
        );
        let labels: Vec<_> = layout
            .scene
            .label_series
            .iter()
            .filter(|(_, series)| series.as_str() == name)
            .map(|(index, _)| layout.scene.labels[*index].color)
            .collect();
        assert_eq!(labels, [text, DARK]);
        let leader = layout
            .link_candidates
            .iter()
            .find(|link| link.series == name)
            .unwrap();
        let color = Color(color.0, color.1, color.2, 128);
        assert_eq!(
            layout
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == leader.id)
                .unwrap()
                .stroke
                .unwrap()
                .color,
            color
        );
    }
    let svg = render::to_svg(&layout.scene).unwrap();
    assert!(svg.contains("rgba(240,190,65,1)") && svg.contains("rgba(22,31,56,1)"));

    let collapsed = [RadialDatum::new("North", 0.), RadialDatum::new("South", 1.)];
    let layout =
        pie_layout_with_ordinal_colors(&config, &collapsed, &options, &scale, &styles).unwrap();
    assert!(
        layout
            .slices
            .iter()
            .any(|slice| slice.id == "North" && slice.outer_collapsed)
    );
    let label = layout
        .scene
        .label_series
        .iter()
        .find(|(_, name)| name.as_str() == "North")
        .unwrap()
        .0;
    assert_eq!(layout.scene.labels[*label].color, DARK);

    options.pie.hidden.insert("North".into());
    let mut seen = Vec::new();
    let layout = pie_layout_with_color_factory(&config, &data, &options, &styles, |datum, _| {
        seen.push(datum.id.clone());
        Ok(GREEN)
    })
    .unwrap();
    assert_eq!(seen, ["South", "North"]);
    assert_eq!(layout.scene.legend[1].color, GREEN);
    assert!(!layout.scene.marks.iter().any(|mark| mark.series == "North"));
    assert!(
        pie_layout_with_color_factory(&config, &data, &options, &styles, |_, _| {
            Err(ChartError("rejected palette".into()))
        })
        .is_err()
    );
}

#[test]
fn bar_and_line_use_color_identity_across_visibility_and_scale_changes() {
    let c = ChartConfig::default();
    let colors = palette();
    let categories = ["South".into(), "North".into()];
    let data = [
        BarSeries::new("Central", vec![Some(5.), Some(8.)]),
        BarSeries::new("North", vec![Some(8.), Some(10.)]),
    ];
    let bars = BarStyleOptions {
        color_by: BarColorBy::Category,
        colors: [("South".into(), GREEN)].into(),
        ..Default::default()
    };
    let options = BarOptions {
        hidden: ["North".into()].into(),
        ..Default::default()
    };
    let style = bars
        .with_ordinal_colors(&categories, &data, &colors)
        .unwrap();
    let output = bar_layout_with_scale_options(
        &c,
        &categories,
        &data,
        &options,
        &style,
        &ContinuousScaleOptions::default(),
    )
    .unwrap()
    .layout;
    assert_eq!(output.bars.len(), 2);
    for bar in output.bars {
        let expected = if bar.category == "South" { GREEN } else { NAVY };
        assert_eq!(bar.color, expected);
        assert_eq!(
            output
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == bar.id)
                .unwrap()
                .fill,
            Some(expected)
        );
    }
    assert_eq!(output.scene.legend[0].color, Color(110, 125, 140, 255));
    assert!(
        BarStyleOptions {
            colors: [("absent".into(), DARK)].into(),
            ..bars
        }
        .with_ordinal_colors(&categories, &data, &colors)
        .is_err()
    );
    let series_layout = bar_layout_with_ordinal_colors(
        &c,
        &categories,
        &data,
        &options,
        &Default::default(),
        &colors,
    )
    .unwrap();
    assert!(series_layout.bars.iter().all(|bar| bar.color == GREEN));
    assert_eq!(
        series_layout
            .scene
            .legend
            .iter()
            .find(|item| item.id == "North")
            .unwrap()
            .color,
        NAVY
    );

    let line_data = [
        Series::new("South", vec![Point::new(1., 4.), Point::new(4., 10.)]),
        Series::new("North", vec![Point::new(1., 2.), Point::new(4., 7.)]),
    ];
    let line_options = LineOptions {
        points: true,
        hidden: ["South".into()].into(),
        ..Default::default()
    };
    let line = line_layout_with_scale_options_and_colors(
        &c,
        &line_data,
        &line_options,
        &Default::default(),
        &Default::default(),
        &colors,
    )
    .unwrap()
    .layout;
    assert_eq!(
        line.scene
            .legend
            .iter()
            .map(|item| item.color)
            .collect::<Vec<_>>(),
        [GOLD, NAVY]
    );
    assert_eq!(line.sites.len(), 2);
    assert!(
        line.interaction_binding()
            .sites
            .iter()
            .all(|site| site.color == NAVY)
    );
    assert!(line.scene.marks.iter().any(
        |mark| mark.series == "North" && mark.stroke.is_some_and(|stroke| stroke.color == NAVY)
    ));
    let explicit = line_layout_with_ordinal_colors(
        &c,
        &line_data,
        &line_options,
        &LineStyleOptions {
            series_styles: [(
                "North".into(),
                LineSeriesStyle {
                    color: Some(GREEN),
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        },
        &colors,
    )
    .unwrap();
    assert_eq!(explicit.scene.legend[1].color, GREEN);
    assert!(explicit.sites.iter().all(|site| site.series_color == GREEN));
}
