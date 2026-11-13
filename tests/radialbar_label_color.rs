use kartis::surface::{SurfaceInput, SurfaceModel};
use kartis::*;
use std::collections::BTreeMap;
use std::f64::consts::TAU;

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
    vec!["low".into(), "high".into()]
}
fn rows() -> Vec<BarSeries> {
    vec![BarSeries::new("ring", vec![Some(2.), Some(4.)])]
}
fn options(profile: Option<InheritedColor>) -> StackedRadialBarOptions {
    StackedRadialBarOptions {
        colors: [
            ("low".into(), Color(20, 40, 60, 160)),
            ("high".into(), Color(200, 100, 50, 101)),
        ]
        .into(),
        labels: Some(RadialBarLabels {
            text: RadialBarLabelText::Category,
            color_profile: profile,
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn label_colors(layout: &StackedRadialBarLayout) -> BTreeMap<String, Color> {
    layout
        .scene
        .label_series
        .iter()
        .map(|(index, series)| (series.clone(), layout.scene.labels[*index].color))
        .collect()
}
fn contrast() -> InheritedColor {
    InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    }
}

#[test]
fn inherited_palette_modifiers_are_per_category_and_independent_of_mark_opacity_or_paint() {
    for opacity in [0., 0.2, 1.] {
        let mut o = options(Some(InheritedColor::FromContext(vec![
            ColorModifier::Darker(1.),
        ])));
        o.opacity = opacity;
        o.paints.insert(
            "low".into(),
            FillPaint::Linear {
                start: [0., 0.],
                end: [320., 0.],
                stops: vec![
                    GradientStop::new(0., Color(255, 0, 255, 255)),
                    GradientStop::new(1., Color(0, 255, 0, 255)),
                ],
            },
        );
        let l = layout_stacked_radial_bar(&config(), &categories(), &rows(), &o).unwrap();
        assert_eq!(
            label_colors(&l),
            [
                ("low".into(), Color(14, 28, 42, 160)),
                ("high".into(), Color(140, 70, 35, 101))
            ]
            .into()
        );
        assert!(
            l.scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("radial-segment:"))
                .all(|m| m.opacity == opacity)
        );
        assert!(l.scene.marks.iter().any(|m| m.fill_paint.is_some()));
    }
}

#[test]
fn theme_reuses_existing_fixed_fallback_and_explicit_fixed_overrides_it() {
    let mut c = config();
    c.theme.text = Color(12, 23, 34, 77);
    for fixed in [None, Some(Color(4, 5, 6, 99))] {
        let mut o = options(Some(InheritedColor::Theme));
        o.labels.as_mut().unwrap().color = fixed;
        let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
        assert!(
            label_colors(&l)
                .values()
                .all(|color| *color == fixed.unwrap_or(c.theme.text))
        );
        o.labels.as_mut().unwrap().color_profile =
            Some(InheritedColor::Fixed(Color(40, 50, 60, 101)));
        o.opacity = 0.1;
        let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
        assert!(
            label_colors(&l)
                .values()
                .all(|color| *color == Color(40, 50, 60, 101))
        );
    }
}

#[test]
fn ordered_modifiers_replace_alpha_and_typed_transparent_rgb_is_retained() {
    let mut o = options(Some(InheritedColor::FromContext(vec![
        ColorModifier::Darker(1.),
        ColorModifier::Brighter(1.),
        ColorModifier::Opacity(0.4),
    ])));
    o.opacity = 0.01;
    let l = layout_stacked_radial_bar(&config(), &categories(), &rows(), &o).unwrap();
    assert_eq!(
        label_colors(&l),
        [
            ("low".into(), Color(20, 40, 60, 102)),
            ("high".into(), Color(200, 100, 50, 102))
        ]
        .into()
    );
    o.labels.as_mut().unwrap().color_profile = Some(InheritedColor::Fixed(Color(40, 50, 60, 0)));
    let l = layout_stacked_radial_bar(&config(), &categories(), &rows(), &o).unwrap();
    assert!(
        label_colors(&l)
            .values()
            .all(|color| *color == Color(40, 50, 60, 0))
    );
}

#[test]
fn contrast_uses_original_palette_surface_and_allows_opaque_surface_with_transparent_backdrop() {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    let mut o = options(Some(contrast()));
    o.colors = [
        ("low".into(), Color(0, 0, 0, 255)),
        ("high".into(), Color(255, 255, 255, 255)),
    ]
    .into();
    o.paints
        .insert("low".into(), FillPaint::Pattern(PatternFill::default()));
    let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
    assert_eq!(
        label_colors(&l),
        [
            ("low".into(), Color(255, 255, 255, 255)),
            ("high".into(), Color(0, 0, 0, 255))
        ]
        .into()
    );
}

#[test]
fn intrinsic_invalid_profiles_reject_empty_hidden_skipped_and_zero_sources() {
    for profile in [
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::INFINITY)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(0.5); 33]),
    ] {
        for kind in 0..4 {
            let mut o = options(Some(profile.clone()));
            let mut data = rows();
            let mut cats = categories();
            match kind {
                0 => {
                    cats.clear();
                    data.clear();
                }
                1 => {
                    o.hidden = cats.iter().cloned().collect();
                }
                2 => {
                    o.labels.as_mut().unwrap().skip_angle = TAU;
                }
                _ => {
                    data[0].values = vec![Some(0.), None];
                }
            }
            if kind == 0 {
                o.colors.clear();
            }
            let error = layout_stacked_radial_bar(&config(), &cats, &data, &o).unwrap_err();
            assert!(
                error.0.contains("color") || error.0.contains("modifiers"),
                "{error}"
            );
        }
    }
}

#[test]
fn contrast_preflights_real_hidden_zero_skipped_contexts_but_not_absent_or_empty_sources() {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    for kind in 0..3 {
        let mut o = options(Some(contrast()));
        if kind == 0 {
            o.hidden = categories().into_iter().collect();
        }
        if kind == 1 {
            o.labels.as_mut().unwrap().skip_angle = TAU;
        }
        let mut data = rows();
        if kind == 2 {
            data[0].values = vec![Some(0.), None];
        }
        let error = layout_stacked_radial_bar(&c, &categories(), &data, &o).unwrap_err();
        assert!(error.0.contains("opaque effective background"));
    }
    let o = options(Some(contrast()));
    let l = layout_stacked_radial_bar(
        &c,
        &categories(),
        &[BarSeries::new("ring", vec![None, None])],
        &o,
    )
    .unwrap();
    assert!(l.scene.label_series.is_empty());
    let empty = StackedRadialBarOptions {
        labels: o.labels,
        ..Default::default()
    };
    assert!(layout_stacked_radial_bar(&c, &[], &[], &empty).is_ok());
}

#[test]
fn none_role_does_not_add_context_validation_and_labels_none_keeps_disabled_path() {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    let mut o = options(None);
    o.colors.insert("low".into(), Color(10, 20, 30, 0));
    assert!(layout_stacked_radial_bar(&c, &categories(), &rows(), &o).is_ok());
    o.labels = None;
    let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
    assert!(l.scene.label_series.is_empty());
}

#[test]
fn color_role_keeps_data_bindings_hover_and_original_source_rejection_contract() {
    let old = layout_stacked_radial_bar(&config(), &categories(), &rows(), &options(None)).unwrap();
    let role = options(Some(InheritedColor::FromContext(vec![
        ColorModifier::Darker(1.),
    ])));
    let l = layout_stacked_radial_bar(&config(), &categories(), &rows(), &role).unwrap();
    assert_eq!(l.frame, old.frame);
    assert_eq!(l.rings, old.rings);
    assert_eq!(l.segments, old.segments);
    assert_eq!(l.domain, old.domain);
    let mut restored = l.scene.clone();
    for index in restored.label_series.keys() {
        restored.labels[*index].color = old.scene.labels[*index].color;
    }
    assert_eq!(restored, old.scene);
    assert_eq!(
        l.interaction_binding().unwrap(),
        old.interaction_binding().unwrap()
    );
    let site = l.interaction_binding().unwrap().sites[0].clone();
    let mut surface = SurfaceModel::new(l.scene.clone(), Default::default()).unwrap();
    surface
        .set_layout(l.clone().into_scene_interaction().unwrap())
        .unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (site.position[0], site.position[1]),
        })
        .unwrap();
    assert_eq!(surface.hovered(), [site.mark_id]);
    surface.apply(SurfaceInput::Leave).unwrap();
    assert!(surface.hovered().is_empty());
    assert_eq!(surface.scene(), &l.scene);
    let mut bad = l.clone();
    bad.segments[0].value = 99.;
    assert!(bad.interaction_binding().is_err());
    let mut bad = l.clone();
    bad.scene
        .marks
        .iter_mut()
        .find(|m| m.id.starts_with("radial-segment:"))
        .unwrap()
        .label = "forged".into();
    assert!(bad.interaction_binding().is_err());
    // Labels remain ordinary decorations: this color role does not introduce
    // a new frozen-label protocol or silently change the binding contract.
    let mut decorated = l;
    let slot = *decorated.scene.label_series.keys().next().unwrap();
    decorated.scene.labels[slot].color = Color(1, 2, 3, 255);
    decorated.interaction_binding().unwrap();
}
