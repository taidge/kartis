use std::cell::RefCell;
use std::f64::consts::TAU;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 340.,
        height: 260.,
        margin: Margin {
            left: 12.,
            right: 12.,
            top: 12.,
            bottom: 12.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("leaf", Some("sales".into()), 4.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("sales", Some("root".into()), 0.),
    ]
}
fn options() -> SunburstLayoutOptions {
    SunburstLayoutOptions {
        value_format: Some(ValueFormat::Fixed(0)),
        labels: Some(SunburstLabelOptions::default()),
        ..Default::default()
    }
}
fn metrics(layout: &SunburstLayout) -> Vec<SunburstMeasuredLabel> {
    layout
        .label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: -4.,
                y: -4.,
                width: 8.,
                height: 8.,
            })
        })
        .collect()
}
#[test]
fn context_borrows_original_source_and_captures_final_text_before_measurements() -> Result<()> {
    let input = data();
    let before = input.clone();
    let c = config();
    let o = options();
    let plain = sunburst_layout(&c, &input, &o)?;
    let events = RefCell::new(Vec::new());
    let custom = {
        let mut text = |ctx: &SunburstLabelContext<'_>| {
            assert!(std::ptr::eq(ctx.datum, &input[ctx.node.source_index]));
            assert_eq!(ctx.datum, &ctx.node.data);
            assert!(ctx.node.label_index.is_none() && ctx.node.mark_index.is_some());
            let old = plain.node(&ctx.datum.id).unwrap();
            let label = &plain.scene.labels[old.label_index.unwrap()];
            assert_eq!(ctx.label_position, [label.x, label.y]);
            assert_eq!(ctx.plot, plain.plot());
            assert_eq!(ctx.default_text, "4");
            assert_eq!(ctx.node.formatted_value, "4");
            events.borrow_mut().push((
                ctx.datum.id.clone(),
                ctx.node.source_index,
                ctx.datum.value,
                ctx.node.value,
                ctx.node.depth,
                ctx.node.height,
            ));
            Ok(format!(
                "{}@{}:{}",
                ctx.datum.id, ctx.node.depth, ctx.node.formatted_value
            ))
        };
        sunburst_layout_with_label_factory(&c, &input, &o, &mut text)?
    };
    assert_eq!(
        *events.borrow(),
        vec![
            ("sales".into(), 2, 0., 4., 1, 1),
            ("leaf".into(), 0, 4., 4., 2, 0)
        ]
    );
    assert_eq!(
        custom
            .scene
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["sales@1:4", "leaf@2:4"]
    );
    for candidate in custom.label_candidates() {
        assert!(candidate.label.text.contains('@'));
    }
    assert_eq!(plain.scene.marks, custom.scene.marks);
    assert_eq!(plain.nodes, custom.nodes);
    assert_eq!(plain.interaction_binding()?, custom.interaction_binding()?);
    assert!(
        custom
            .place_labels(
                &metrics(&plain),
                &SunburstLabelPlacementOptions::for_layout(&custom)
            )
            .is_err()
    );
    let supplied = metrics(&custom);
    let placed = custom.place_labels(
        &supplied,
        &SunburstLabelPlacementOptions::for_layout(&custom),
    )?;
    assert_eq!(placed.interaction_binding()?, custom.interaction_binding()?);
    assert_eq!(events.borrow().len(), 2);
    assert_eq!(input, before);
    Ok(())
}

#[test]
fn identity_factory_preserves_layout_signature_and_old_error_order() -> Result<()> {
    for root in [false, true] {
        for reverse in [false, true] {
            for text in [
                SunburstLabelText::FormattedValue,
                SunburstLabelText::Label,
                SunburstLabelText::Id,
            ] {
                let mut o = options();
                o.include_root = root;
                o.labels.as_mut().unwrap().text = text;
                if reverse {
                    o.start_angle = TAU;
                    o.end_angle = 0.;
                }
                let old = sunburst_layout(&config(), &data(), &o)?;
                let new = sunburst_layout_with_label_factory(&config(), &data(), &o, |c| {
                    Ok(c.default_text.to_string())
                })?;
                assert_eq!(old, new);
                assert_eq!(
                    old.place_labels(
                        &metrics(&old),
                        &SunburstLabelPlacementOptions::for_layout(&old)
                    )?,
                    new.place_labels(
                        &metrics(&old),
                        &SunburstLabelPlacementOptions::for_layout(&new)
                    )?
                );
            }
        }
    }
    for kind in 0..4 {
        let mut cfg = config();
        let mut o = options();
        let mut input = data();
        match kind {
            0 => cfg.width = 0.,
            1 => o.corner_radius = f64::NAN,
            2 => input[0].parent = Some("missing".into()),
            _ => {
                o.labels.as_mut().unwrap().color =
                    InheritedColor::FromContext(vec![ColorModifier::Darker(f64::NAN)])
            }
        }
        let mut calls = 0;
        let old = sunburst_layout(&cfg, &input, &o).unwrap_err();
        let new = sunburst_layout_with_label_factory(&cfg, &input, &o, |c| {
            calls += 1;
            Ok(c.default_text.to_string())
        })
        .unwrap_err();
        assert_eq!(old, new);
        assert_eq!(calls, 0);
    }
    Ok(())
}

#[test]
fn callback_eligibility_retains_exact_root_hidden_zero_and_skip_rules() -> Result<()> {
    let mut source = data();
    source.push(HierarchyDatum::new("zero", Some("root".into()), 0.));
    for kind in 0..8 {
        let mut o = options();
        let expected = match kind {
            0 => {
                o.labels = None;
                0
            }
            1 => {
                o.hidden.insert("sales".into());
                1
            }
            2 => {
                o.include_root = true;
                3
            }
            3 => {
                o.opacity = 0.;
                0
            }
            4 => {
                o.labels.as_mut().unwrap().skip_radius = 16384.;
                0
            }
            5 => {
                o.pad_angle = TAU;
                2
            }
            6 => {
                o.border_width = 0.;
                for d in &source {
                    o.colors_by_id.insert(d.id.clone(), Color(0, 0, 0, 0));
                }
                0
            }
            _ => 2,
        };
        let mut calls = 0;
        let layout = sunburst_layout_with_label_factory(&config(), &source, &o, |c| {
            calls += 1;
            assert_ne!(c.datum.id, "zero");
            Ok(c.default_text.to_string())
        })?;
        assert_eq!(calls, expected, "kind {kind}");
        assert_eq!(layout.label_candidates().len(), expected);
    }
    let mut o = options();
    // A real partial fan with strong PI padding collapses; full turns above
    // retain their existing arc behavior instead of being treated as empty.
    let mut partial = data();
    partial.push(HierarchyDatum::new("outside", Some("root".into()), 4.));
    o.pad_angle = std::f64::consts::PI;
    let mut calls = 0;
    sunburst_layout_with_label_factory(&config(), &partial, &o, |c| {
        calls += 1;
        Ok(c.default_text.to_string())
    })?;
    assert_eq!(calls, 0);
    let mut o = options();
    o.labels.as_mut().unwrap().skip_angle = TAU;
    // Both positive non-root nodes occupy a full turn in this single branch.
    assert_eq!(
        sunburst_layout_with_label_factory(&config(), &data(), &o, |c| Ok(c
            .default_text
            .to_string()))?
        .scene
        .labels
        .len(),
        2
    );
    let empty = sunburst_layout_with_label_factory(&config(), &[], &options(), |_| {
        panic!("empty source callback")
    })?;
    assert!(empty.nodes.is_empty());
    Ok(())
}

#[test]
fn empty_slots_unicode_byte_budget_and_fallible_errors_are_transactional() -> Result<()> {
    let input = data();
    let before = input.clone();
    let accepted = sunburst_layout(&config(), &input, &options())?;
    let snapshot = accepted.clone();
    let empty =
        sunburst_layout_with_label_factory(&config(), &input, &options(), |_| Ok(String::new()))?;
    assert_eq!(empty.scene.labels.len(), 2);
    assert_eq!(empty.label_candidates().len(), 2);
    assert!(empty.scene.labels.iter().all(|l| l.text.is_empty()));
    assert_eq!(empty.with_label_rotation(0.)?, empty);
    for (text, valid) in [("é".repeat(512), true), ("é".repeat(513), false)] {
        let mut calls = 0;
        let result = sunburst_layout_with_label_factory(&config(), &input, &options(), |_| {
            calls += 1;
            Ok(text.clone())
        });
        assert_eq!(result.is_ok(), valid);
        assert_eq!(calls, if valid { 2 } else { 1 });
    }
    let effects = RefCell::new(Vec::new());
    let error = sunburst_layout_with_label_factory(&config(), &input, &options(), |c| {
        effects.borrow_mut().push(c.datum.id.clone());
        if c.datum.id == "leaf" {
            Err(ChartError("application rejected leaf".into()))
        } else {
            Ok(c.default_text.to_string())
        }
    });
    assert!(error.is_err());
    assert_eq!(*effects.borrow(), ["sales", "leaf"]);
    assert_eq!(accepted, snapshot);
    assert_eq!(input, before);
    Ok(())
}

#[test]
fn measurement_provenance_rejects_text_font_rotation_and_source_edits() -> Result<()> {
    let custom = sunburst_layout_with_label_factory(&config(), &data(), &options(), |c| {
        Ok(format!("{}:{}", c.datum.id, c.default_text))
    })?;
    let measured = metrics(&custom);
    let placement = SunburstLabelPlacementOptions::for_layout(&custom);
    let before = custom.clone();
    for kind in 0..5 {
        let mut bad = custom.clone();
        let slot = bad.node("sales").unwrap().label_index.unwrap();
        match kind {
            0 => bad.scene.labels[slot].text = "forged".into(),
            1 => {
                bad.scene.label_series.remove(&slot);
            }
            2 => {
                let owner = bad.nodes.iter().position(|n| n.data.id == "sales").unwrap();
                bad.nodes[owner].data.value = 99.;
            }
            3 => bad.scene.labels[slot].color = Color(255, 0, 255, 255),
            _ => bad.nodes[1].arc.outer_radius -= 1.,
        }
        let edited = bad.clone();
        assert!(bad.place_labels(&measured, &placement).is_err());
        assert!(bad.with_label_rotation(10.).is_err());
        assert_eq!(bad, edited);
    }
    // Existing managed-label provenance is not a whole-hierarchy freeze.
    let mut unowned_root = custom.clone();
    assert!(unowned_root.node("root").unwrap().label_index.is_none());
    unowned_root.nodes[0].data.value = 99.;
    assert!(unowned_root.with_label_rotation(0.).is_ok());
    assert!(unowned_root.place_labels(&measured, &placement).is_ok());
    let font = custom.with_label_font(FontOptions {
        weight: Some(500.),
        ..Default::default()
    })?;
    assert!(font.place_labels(&measured, &placement).is_err());
    let rotated = custom.with_label_rotation(15.)?;
    assert!(rotated.place_labels(&measured, &placement).is_err());
    let current = metrics(&rotated);
    let mut hide = SunburstLabelPlacementOptions::for_layout(&rotated);
    hide.hidden = rotated
        .label_candidates()
        .iter()
        .map(|c| c.node_id.clone())
        .collect();
    let hidden = rotated.place_labels(&current, &hide)?;
    assert!(hidden.nodes.iter().all(|n| n.label_index.is_none()));
    hide.hidden.clear();
    let restored = hidden.place_labels(&current, &hide)?;
    assert!(restored.scene.labels.iter().any(|l| l.text == "sales:4"));
    assert_eq!(custom, before);
    Ok(())
}

#[test]
fn custom_text_does_not_replace_raw_hover_format_or_existing_label_alpha() -> Result<()> {
    use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
    let events = RefCell::new(0);
    let mut o = options();
    o.opacity = 0.5;
    o.labels.as_mut().unwrap().color = InheritedColor::Fixed(Color(40, 50, 60, 101));
    let layout = {
        let mut text = |c: &SunburstLabelContext<'_>| {
            *events.borrow_mut() += 1;
            Ok(format!("caption:{}", c.datum.id))
        };
        sunburst_layout_with_label_factory(&config(), &data(), &o, &mut text)?
    };
    assert!(
        layout
            .scene
            .labels
            .iter()
            .all(|l| l.color == Color(40, 50, 60, 101))
    );
    let binding = layout.interaction_binding()?;
    assert!(binding.sites.iter().all(|s| s.value == Some(4.)
        && s.formatted_value.as_deref() == Some("4")
        && !s.label.starts_with("caption:")));
    let mut model = SurfaceModel::new(
        config().scene()?,
        SurfaceOptions {
            picking: Picking::Nearest { distance: 1. },
            ..Default::default()
        },
    )?;
    model.set_layout(layout.clone().into_scene_interaction()?)?;
    let point = layout.node("leaf").unwrap().anchor;
    model.apply(SurfaceInput::Move {
        id: 1,
        point: (point[0], point[1]),
    })?;
    assert!(
        model
            .active_interaction(model.hovered())
            .unwrap()
            .rows
            .iter()
            .all(|r| r.value == Some(4.))
    );
    assert_eq!(*events.borrow(), 2);
    Ok(())
}

#[test]
fn contrast_context_error_keeps_the_documented_external_side_effect_boundary() -> Result<()> {
    let mut cfg = config();
    cfg.theme.background = Color(255, 255, 255, 100);
    let mut o = options();
    o.colors_by_id
        .insert("sales".into(), Color(20, 40, 60, 100));
    o.labels.as_mut().unwrap().color = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    let input = data();
    let before = input.clone();
    let effects = RefCell::new(Vec::new());
    let result = sunburst_layout_with_label_factory(&cfg, &input, &o, |c| {
        effects.borrow_mut().push(c.datum.id.clone());
        Ok(c.default_text.to_string())
    });
    assert!(result.is_err());
    // Intrinsic options are validated first. The existing per-label color
    // context resolves after text construction; no global preflight or rollback
    // of callback-owned external state is promised.
    assert_eq!(*effects.borrow(), ["sales"]);
    assert_eq!(input, before);
    Ok(())
}

#[cfg(feature = "raster")]
#[test]
fn actual_font_measurement_and_all_three_mask_policies_use_custom_text() -> Result<()> {
    use kartis::render::raster::{FontRegistry, MissingGlyphPolicy, render_raster};
    let mut fonts = FontRegistry::new();
    fonts.register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)?;
    let original = sunburst_layout(&config(), &data(), &options())?;
    let custom = sunburst_layout_with_label_factory(&config(), &data(), &options(), |c| {
        Ok(format!(
            "{}@{}:{}",
            c.datum.id, c.node.depth, c.node.formatted_value
        ))
    })?
    .with_label_rotation(12.)?;
    let measurements = custom
        .label_candidates()
        .iter()
        .map(|c| {
            let ink = fonts
                .measure_label(&c.label, MissingGlyphPolicy::Error)
                .unwrap()
                .unwrap();
            c.measured(LayoutRect {
                x: ink.x - c.label.x,
                y: ink.y - c.label.y,
                width: ink.width,
                height: ink.height,
            })
        })
        .collect::<Vec<_>>();
    let old_measurements = original
        .label_candidates()
        .iter()
        .map(|c| {
            let ink = fonts
                .measure_label(&c.label, MissingGlyphPolicy::Error)
                .unwrap()
                .unwrap();
            c.measured(LayoutRect {
                x: ink.x - c.label.x,
                y: ink.y - c.label.y,
                width: ink.width,
                height: ink.height,
            })
        })
        .collect::<Vec<_>>();
    assert!(
        custom
            .place_labels(
                &old_measurements,
                &SunburstLabelPlacementOptions::for_layout(&custom)
            )
            .is_err()
    );
    for clip in [
        SunburstLabelClip::Plot,
        SunburstLabelClip::ClipToArc,
        SunburstLabelClip::RequireFullInkBounds,
    ] {
        let mut placement = SunburstLabelPlacementOptions::for_layout(&custom);
        placement.clip = clip;
        let placed = custom.place_labels(&measurements, &placement)?;
        assert_eq!(placed.interaction_binding()?, custom.interaction_binding()?);
        let raster = render_raster(&placed.scene, &fonts, &Default::default())?;
        assert_eq!(raster.rgba.len(), 340 * 260 * 4);
        assert!(
            render::to_svg(&placed.scene)?.contains("sales@1:4")
                || placed.node("sales").unwrap().label_index.is_none()
        );
    }
    let image = render_raster(&custom.scene, &fonts, &Default::default())?;
    let old_image = render_raster(&original.scene, &fonts, &Default::default())?;
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(old_image.rgba.as_chunks::<4>().0.iter())
            .filter(|(a, b)| a != b)
            .count()
            > 10
    );
    Ok(())
}
