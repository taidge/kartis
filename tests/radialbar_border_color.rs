use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 320.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn categories() -> Vec<String> {
    ["low", "high", "zero", "missing"]
        .map(str::to_string)
        .to_vec()
}
fn data() -> Vec<BarSeries> {
    vec![
        BarSeries::new("north", vec![Some(2.), Some(4.), Some(0.), None]),
        BarSeries::new("south", vec![Some(3.), None, Some(0.), None]),
    ]
}
fn border(alpha: u8) -> GraphicStroke {
    GraphicStroke {
        color: Color(70, 80, 90, alpha),
        width: 3.,
        cap: StrokeCap::Round,
    }
}
fn options() -> StackedRadialBarOptions {
    StackedRadialBarOptions {
        colors: [
            ("low".into(), Color(20, 40, 60, 160)),
            ("high".into(), Color(200, 100, 50, 101)),
        ]
        .into(),
        border: Some(border(137)),
        labels: Some(RadialBarLabels {
            text: RadialBarLabelText::Category,
            color: Some(Color(4, 5, 6, 101)),
            ..Default::default()
        }),
        axes: PolarAxesOptions {
            radial_start: None,
            circular_outer: None,
            grid: PolarGridStyle {
                radial: false,
                circular: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
fn build(
    c: &ChartConfig,
    o: &StackedRadialBarOptions,
    styled: bool,
) -> Result<StackedRadialBarLayout> {
    if styled {
        layout_stacked_radial_bar_with_arc_style(
            c,
            &categories(),
            &data(),
            o,
            &StackedRadialArcStyle {
                corner_radius: 8.,
                ..Default::default()
            },
        )
    } else {
        layout_stacked_radial_bar(c, &categories(), &data(), o)
    }
}
fn segments(scene: &Scene) -> impl Iterator<Item = &Mark> {
    scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("radial-segment:"))
}
fn contrast() -> InheritedColor {
    InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    }
}

#[test]
fn theme_none_and_borderless_profiles_keep_the_original_complete_layout() -> Result<()> {
    assert!(
        StackedRadialBarOptions::default()
            .border_color_profile
            .is_none()
    );
    for styled in [false, true] {
        for reverse in [false, true] {
            let mut o = options();
            if reverse {
                o.start_angle = 2.7;
                o.end_angle = -0.3;
            }
            let old = build(&config(), &o, styled)?;
            o.border_color_profile = Some(InheritedColor::Theme);
            let same = build(&config(), &o, styled)?;
            assert_eq!(same, old);
            assert_eq!(render::to_svg(&same.scene)?, render::to_svg(&old.scene)?);
            assert_eq!(
                render::to_canvas_script(&same.scene, "c")?,
                render::to_canvas_script(&old.scene, "c")?
            );
            o.border = None;
            o.border_color_profile = None;
            let before = build(&config(), &o, styled)?;
            o.border_color_profile = Some(InheritedColor::Fixed(Color(1, 2, 3, 255)));
            assert_eq!(build(&config(), &o, styled)?, before);
        }
    }
    Ok(())
}

#[test]
fn category_palette_precedes_custom_paints_and_does_not_premultiply_opacity_or_tracks() -> Result<()>
{
    for styled in [false, true] {
        for opacity in [0.25, 0.75, 1.] {
            let mut o = options();
            o.opacity = opacity;
            o.paints.insert(
                "low".into(),
                FillPaint::Linear {
                    start: [0., 0.],
                    end: [1., 0.],
                    stops: vec![
                        GradientStop::new(0., Color(240, 30, 180, 255)),
                        GradientStop::new(1., Color(240, 30, 180, 255)),
                    ],
                },
            );
            let old = build(&config(), &o, styled)?;
            o.border_color_profile = Some(InheritedColor::FromContext(vec![
                ColorModifier::Darker(1.),
                ColorModifier::Opacity(0.5),
            ]));
            let actual = build(&config(), &o, styled)?;
            assert_eq!(actual.segments, old.segments);
            assert_eq!(actual.rings, old.rings);
            let mut normalized = actual.clone();
            for m in &mut normalized.scene.marks {
                if m.id.starts_with("radial-track:") {
                    assert_eq!(&*m, old.scene.marks.iter().find(|x| x.id == m.id).unwrap());
                    continue;
                }
                if m.id.starts_with("radial-segment:") {
                    let expected = if m.series == "low" {
                        Color(14, 28, 42, 128)
                    } else {
                        Color(140, 70, 35, 128)
                    };
                    assert_eq!(
                        m.stroke,
                        Some(GraphicStroke {
                            color: expected,
                            ..border(137)
                        })
                    );
                    assert_eq!(m.opacity, opacity);
                    m.stroke = Some(border(137));
                }
            }
            assert_eq!(normalized, old);
            assert!(
                actual
                    .scene
                    .labels
                    .iter()
                    .all(|l| l.color == Color(4, 5, 6, 101))
            );
            let binding = actual.interaction_binding()?;
            assert!(
                binding
                    .sites
                    .iter()
                    .filter(|s| s.series == "low")
                    .all(|s| s.color == Color(240, 30, 180, 255))
            );
            assert_eq!(binding.sites.len(), 3);
        }
    }
    Ok(())
}

#[test]
fn resolved_border_alpha_controls_styled_collapsed_ink_without_synthesizing_zero_or_none()
-> Result<()> {
    let c = config();
    let cats = categories();
    let rows = [BarSeries::new(
        "ring",
        vec![Some(1.), Some(2.), Some(0.), None],
    )];
    let mut o = options();
    o.tracks = false;
    o.padding = 0.;
    o.inner_radius = 0.4;
    o.start_angle = 0.;
    o.end_angle = 0.14;
    o.pad_angle = 0.135;
    let style = StackedRadialArcStyle {
        corner_radius: 20.,
        ..Default::default()
    };
    o.border = Some(border(0));
    let empty = layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?;
    assert!(empty.scene.marks.is_empty());
    assert_eq!(empty.segments.len(), 3);
    o.border_color_profile = Some(InheritedColor::Fixed(Color(30, 70, 90, 255)));
    let stroked = layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?;
    assert_eq!(stroked.scene.marks.len(), 2);
    assert!(stroked.scene.labels.is_empty());
    assert!(stroked.segments[..2].iter().all(|s| s.visible));
    assert!(!stroked.segments[2].visible);
    assert!(stroked.segments.iter().all(|s| s.category != "missing"));
    for m in segments(&stroked.scene) {
        assert!(!m.has_fill());
        assert_eq!(m.stroke.unwrap().color, Color(30, 70, 90, 255));
    }
    let binding = stroked.interaction_binding()?;
    assert_eq!(binding.sites.len(), 2);
    assert!(
        binding
            .sites
            .iter()
            .all(|s| s.color == Color(30, 70, 90, 255) && s.value.is_some_and(|v| v > 0.))
    );
    o.border = Some(border(255));
    o.border_color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
        0.,
    )]));
    let transparent = layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?;
    assert!(transparent.scene.marks.is_empty());
    assert!(transparent.interaction_binding()?.sites.is_empty());
    assert_eq!(
        transparent
            .segments
            .iter()
            .map(|s| s.cumulative)
            .collect::<Vec<_>>(),
        stroked
            .segments
            .iter()
            .map(|s| s.cumulative)
            .collect::<Vec<_>>()
    );
    o.border_color_profile = Some(InheritedColor::Fixed(Color(1, 2, 3, 255)));
    o.opacity = 0.;
    assert!(
        layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?
            .scene
            .marks
            .is_empty()
    );
    o.opacity = 1.;
    o.border = None;
    assert!(
        layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?
            .scene
            .marks
            .is_empty()
    );
    // The older unstyled path keeps its original angular-pad cull contract.
    o.border = Some(border(255));
    assert!(
        layout_stacked_radial_bar(&c, &cats, &rows, &o)?
            .scene
            .marks
            .is_empty()
    );
    Ok(())
}

#[test]
fn hidden_slots_source_binding_and_domains_are_preserved() -> Result<()> {
    let mut o = options();
    o.border_color_profile = Some(InheritedColor::Fixed(Color(12, 34, 56, 160)));
    let full = build(&config(), &o, true)?;
    o.hidden.insert("low".into());
    let hidden = build(&config(), &o, true)?;
    assert_eq!(full.domain, hidden.domain);
    assert_eq!(full.rings, hidden.rings);
    for (a, b) in full.segments.iter().zip(&hidden.segments) {
        assert_eq!(a.value, b.value);
        assert_eq!(a.cumulative, b.cumulative);
        assert_eq!(a.angles, b.angles);
        if b.category == "low" {
            assert!(!b.visible);
        }
    }
    assert!(
        hidden
            .interaction_binding()?
            .sites
            .iter()
            .all(|s| s.series != "low")
    );
    assert!(
        hidden
            .segments
            .iter()
            .filter(|s| s.category == "zero")
            .all(|s| s.value == 0. && !s.visible)
    );
    for mutation in 0..3 {
        let mut bad = full.clone();
        match mutation {
            0 => bad.segments[0].cumulative[0] = 1.,
            1 => bad.segments[0].category = "forged".into(),
            _ => {
                let id = bad.segments[0].mark_id.clone();
                let m = bad.scene.marks.iter_mut().find(|m| m.id == id).unwrap();
                m.geometry = Geometry::Rect {
                    x: 0.,
                    y: 0.,
                    width: 1.,
                    height: 1.,
                };
            }
        }
        assert!(bad.interaction_binding().is_err());
    }
    Ok(())
}

#[test]
fn intrinsic_profiles_validate_without_sources_and_contexts_use_only_actual_some_values()
-> Result<()> {
    for invalid in [
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(1.1)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(0.5); 33]),
    ] {
        let o = StackedRadialBarOptions {
            border_color_profile: Some(invalid),
            ..Default::default()
        };
        assert!(layout_stacked_radial_bar(&config(), &[], &[], &o).is_err());
        assert!(
            layout_stacked_radial_bar(
                &config(),
                &["a".into()],
                &[BarSeries::new("r", vec![None])],
                &o
            )
            .is_err()
        );
    }
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    for mode in 0..5 {
        let mut o = options();
        o.colors.insert("low".into(), Color(9, 8, 7, 0));
        o.border_color_profile = Some(contrast());
        o.labels = None;
        match mode {
            0 => {
                o.hidden.insert("low".into());
            }
            1 => o.opacity = 0.,
            2 => o.border = None,
            3 => {
                o.end_angle = 0.14;
                o.pad_angle = 0.135;
            }
            _ => {}
        }
        let source = [BarSeries::new("ring", vec![Some(0.), None, None, None])];
        assert!(
            layout_stacked_radial_bar(&c, &categories(), &source, &o)
                .unwrap_err()
                .0
                .contains("opaque")
        );
        let absent = [BarSeries::new("ring", vec![None, None, None, None])];
        assert!(layout_stacked_radial_bar(&c, &categories(), &absent, &o).is_ok());
        assert!(layout_stacked_radial_bar(&c, &categories(), &[], &o).is_ok());
    }
    // An entirely absent transparent category does not poison an actual
    // opaque source in another category over a transparent backdrop.
    let o = StackedRadialBarOptions {
        border_color_profile: Some(contrast()),
        colors: [
            ("absent".into(), Color(8, 7, 6, 0)),
            ("real".into(), Color(200, 200, 200, 255)),
        ]
        .into(),
        ..Default::default()
    };
    assert!(
        layout_stacked_radial_bar(
            &c,
            &["absent".into(), "real".into()],
            &[BarSeries::new("r", vec![None, Some(1.)])],
            &o
        )
        .is_ok()
    );
    Ok(())
}

#[test]
fn matrix_budget_remains_bounded_with_zero_actual_sources() -> Result<()> {
    let cats = (0..256).map(|i| format!("c{i}")).collect::<Vec<_>>();
    let rows = (0..32)
        .map(|i| BarSeries::new(format!("r{i}"), vec![Some(0.); 256]))
        .collect::<Vec<_>>();
    let o = StackedRadialBarOptions {
        tracks: false,
        legend: false,
        axes: options().axes,
        border_color_profile: Some(InheritedColor::Fixed(Color(1, 2, 3, 4))),
        ..Default::default()
    };
    let l = layout_stacked_radial_bar(&config(), &cats, &rows, &o)?;
    assert_eq!(l.segments.len(), MAX_RADIAL_BAR_CELLS);
    assert!(l.scene.marks.is_empty());
    let mut too_many = rows;
    too_many.push(BarSeries::new("extra", vec![Some(0.); 256]));
    assert!(
        layout_stacked_radial_bar(&config(), &cats, &too_many, &o)
            .unwrap_err()
            .0
            .contains("matrix budget")
    );
    Ok(())
}

#[cfg(feature = "geojson")]
#[test]
fn existing_original_nivo_colour_fixture_reaches_radial_border_without_rewriting_shared_algorithm()
-> Result<()> {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bar-label-color/values.json")).unwrap();
    let mut ordinary = 0;
    let mut boundary = 0;
    for row in rows.as_array().unwrap() {
        let bytes = |v: &serde_json::Value| {
            Color(
                v[0].as_u64().unwrap() as u8,
                v[1].as_u64().unwrap() as u8,
                v[2].as_u64().unwrap() as u8,
                v[3].as_u64().unwrap() as u8,
            )
        };
        let modifiers = row["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                let a = m[1].as_f64().unwrap();
                match m[0].as_str().unwrap() {
                    "darker" => ColorModifier::Darker(a),
                    "brighter" => ColorModifier::Brighter(a),
                    _ => ColorModifier::Opacity(a),
                }
            })
            .collect();
        let o = StackedRadialBarOptions {
            border: Some(border(100)),
            border_color_profile: Some(InheritedColor::FromContext(modifiers)),
            colors: [("category".into(), bytes(&row["input"]))].into(),
            ..Default::default()
        };
        let l = layout_stacked_radial_bar(
            &config(),
            &["category".into()],
            &[BarSeries::new("ring", vec![Some(1.)])],
            &o,
        )?;
        let m = segments(&l.scene).next().unwrap();
        assert_eq!(m.stroke.unwrap().color, bytes(&row["nativeTypedRGBA"]));
        if row["profile"] == "ordinary-exact" {
            assert_eq!(m.stroke.unwrap().color, bytes(&row["nativeNivoCSS"]));
            ordinary += 1;
        } else {
            assert_ne!(row["nativeTypedRGBA"], row["nativeNivoCSS"]);
            boundary += 1;
        }
    }
    assert_eq!((ordinary, boundary), (36, 6));
    Ok(())
}

#[cfg(feature = "raster")]
#[test]
fn resolved_visible_collapsed_stroke_renders_and_transparent_override_is_empty() -> Result<()> {
    use render::raster::{FontRegistry, RasterOptions, render_raster};
    let cats = categories();
    let rows = [BarSeries::new(
        "ring",
        vec![Some(1.), Some(2.), Some(0.), None],
    )];
    let mut o = options();
    o.tracks = false;
    o.legend = false;
    o.labels = None;
    o.start_angle = 0.;
    o.end_angle = 0.14;
    o.pad_angle = 0.135;
    o.border = Some(border(0));
    o.border_color_profile = Some(InheritedColor::Fixed(Color(200, 20, 60, 255)));
    let style = StackedRadialArcStyle {
        corner_radius: 20.,
        ..Default::default()
    };
    let c = config();
    let visible = layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?;
    let image = render_raster(
        &visible.scene,
        &FontRegistry::new(),
        &RasterOptions::default(),
    )?;
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| u16::from(p[0]) > u16::from(p[1]) + 20
                && u16::from(p[0]) > u16::from(p[2]) + 20)
    );
    o.border_color_profile = Some(InheritedColor::Fixed(Color(200, 20, 60, 0)));
    let empty = layout_stacked_radial_bar_with_arc_style(&c, &cats, &rows, &o, &style)?;
    let image = render_raster(
        &empty.scene,
        &FontRegistry::new(),
        &RasterOptions::default(),
    )?;
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [255, 255, 255, 255])
    );
    Ok(())
}
