use std::cell::Cell;

use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 480.,
        height: 320.,
        ..Default::default()
    }
}

fn source() -> Vec<HierarchyDatum> {
    // Child-before-parent source order differs from complete BFS link order.
    vec![
        HierarchyDatum::new("leaf", Some("branch".into()), 2.),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("other", Some("root".into()), 3.),
    ]
}

#[test]
fn static_styles_include_disabled_links_and_use_the_computed_endpoint_color() {
    let c = config();
    let raw = source();
    let border = Color(70, 80, 90, 0);
    for enabled in [false, true] {
        for policy in [
            TreeLinkColor::Theme,
            TreeLinkColor::Fixed(Color(12, 34, 56, 0)),
            TreeLinkColor::Source,
            TreeLinkColor::Target,
        ] {
            let mut o = TreeOptions::default();
            o.style.links.enabled = enabled;
            o.style.links.color = policy;
            o.style.links.width = 6.25;
            o.style.links.opacity = 0.;
            o.style.nodes.fill = TreeNodeFill::None;
            o.style.nodes.border = Some(GraphicStroke {
                color: border,
                width: 2.,
                cap: StrokeCap::Butt,
            });
            let l = tree_layout(&c, &raw, &o).unwrap();
            assert_eq!(l.computed_link_styles().len(), 3);
            assert_eq!(
                l.links
                    .iter()
                    .map(|link| (link.source_index, link.target_index))
                    .collect::<Vec<_>>(),
                vec![(1, 2), (1, 3), (2, 0)]
            );
            let wanted = match policy {
                TreeLinkColor::Theme => c.theme.grid,
                TreeLinkColor::Fixed(color) => color,
                TreeLinkColor::Source | TreeLinkColor::Target => border,
            };
            for (link, style) in l.links.iter().zip(l.computed_link_styles()) {
                assert_eq!(style.thickness, 6.25);
                assert_eq!(style.color, wanted);
                if enabled {
                    let mark = l
                        .scene
                        .marks
                        .iter()
                        .find(|mark| Some(&mark.id) == link.mark_id.as_ref())
                        .unwrap();
                    assert_eq!(mark.opacity, 0.);
                    assert_eq!(mark.stroke.unwrap().color, style.color);
                    assert_eq!(mark.stroke.unwrap().width, style.thickness);
                } else {
                    assert!(link.mark_id.is_none());
                    assert!(!l.scene.marks.iter().any(|m| m.id.starts_with("link:")));
                }
            }
        }
    }
}

#[test]
fn accessor_does_not_replay_functions_or_infer_from_edited_scene_paint() {
    let c = config();
    let raw = source();
    let widths = Cell::new(0);
    let colors = Cell::new(0);
    let mut width = |context: &TreeLinkFunctionContext<'_>| {
        widths.set(widths.get() + 1);
        Ok(if context.link.link_index == 0 {
            -0.
        } else {
            context.link.link_index as f64 + 2.
        })
    };
    let mut color = |context: &TreeLinkFunctionContext<'_>| {
        colors.set(colors.get() + 1);
        Ok(Color(context.link.link_index as u8, 40, 50, 0))
    };
    let mut l = tree_layout_with_factories(
        &c,
        &raw,
        &TreeOptions::default(),
        &mut TreeFactories {
            link_thickness: Some(&mut width),
            link_color: Some(&mut color),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((widths.get(), colors.get()), (3, 3));
    let expected = l.computed_link_styles().to_vec();
    assert_eq!(expected[0].thickness.to_bits(), (-0f64).to_bits());
    assert!(
        l.scene
            .marks
            .iter()
            .find(|m| m.id == "link:branch")
            .unwrap()
            .stroke
            .is_none()
    );
    let clone = l.clone();
    assert_eq!(
        l.computed_link_styles().as_ptr(),
        clone.computed_link_styles().as_ptr()
    );
    for mark in l
        .scene
        .marks
        .iter_mut()
        .filter(|m| m.id.starts_with("link:"))
    {
        mark.stroke = Some(GraphicStroke {
            color: Color(220, 130, 10, 255),
            width: 777.,
            cap: StrokeCap::Round,
        });
    }
    for _ in 0..3 {
        assert_eq!(l.computed_link_styles(), expected);
        assert_eq!(clone.computed_link_styles(), expected);
    }
    assert_eq!((widths.get(), colors.get()), (3, 3));
}

#[test]
fn final_activity_modifiers_are_captured_even_when_no_link_is_painted() {
    let c = config();
    let raw = source();
    let mut o = TreeOptions::default();
    o.style.links.enabled = false;
    let base = Cell::new(0);
    let active = Cell::new(0);
    let inactive = Cell::new(0);
    let mut width = |context: &TreeLinkFunctionContext<'_>| {
        base.set(base.get() + 1);
        Ok(context.link.link_index as f64 + 2.)
    };
    let mut color =
        |context: &TreeLinkFunctionContext<'_>| Ok(Color(context.link.link_index as u8, 30, 80, 0));
    let mut on = |context: &TreeLinkModifierContext<'_>| {
        assert!(context.is_active);
        assert_eq!(context.color.3, 0);
        active.set(active.get() + 1);
        Ok(context.thickness + 10.)
    };
    let mut off = |context: &TreeLinkModifierContext<'_>| {
        assert!(!context.is_active);
        inactive.set(inactive.get() + 1);
        Ok(-0.)
    };
    let l = tree_layout_with_activity(
        &c,
        &raw,
        &o,
        &mut TreeFactories {
            link_thickness: Some(&mut width),
            link_color: Some(&mut color),
            ..Default::default()
        },
        &TreeActivityOptions {
            focus_id: Some("leaf".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_link_thickness: Some(&mut on),
            inactive_link_thickness: Some(&mut off),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.activity.links, vec![Some(true), Some(false), Some(true)]);
    assert_eq!((base.get(), active.get(), inactive.get()), (3, 2, 1));
    assert!(l.layout.links.iter().all(|link| link.mark_id.is_none()));
    let styles = l.layout.computed_link_styles();
    assert_eq!(styles[0].thickness, 12.);
    assert_eq!(styles[1].thickness.to_bits(), (-0f64).to_bits());
    assert_eq!(styles[2].thickness, 14.);
    for (i, style) in styles.iter().enumerate() {
        assert_eq!(style.color, Color(i as u8, 30, 80, 0));
    }
    assert_eq!((base.get(), active.get(), inactive.get()), (3, 2, 1));
}

#[test]
fn zero_and_unpainted_style_bits_do_not_change_legacy_layout_debug_or_equality() {
    let c = config();
    let raw = source();
    let build_zero = |zero| {
        let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(zero);
        tree_layout_with_factories(
            &c,
            &raw,
            &TreeOptions::default(),
            &mut TreeFactories {
                link_thickness: Some(&mut width),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let positive = build_zero(0.);
    let negative = build_zero(-0.);
    assert_eq!(positive, negative);
    assert_eq!(positive.scene, negative.scene);
    assert_eq!(format!("{positive:?}"), format!("{negative:?}"));
    assert_eq!(format!("{positive:#?}"), format!("{negative:#?}"));
    assert_eq!(
        render::to_svg(&positive.scene).unwrap(),
        render::to_svg(&negative.scene).unwrap()
    );
    assert_ne!(
        positive.computed_link_styles(),
        negative.computed_link_styles()
    );
    assert!(!format!("{negative:?}").contains("link_styles"));
    let mut o = TreeOptions::default();
    o.style.links.enabled = false;
    let disabled = |color| {
        let mut resolve = |_: &TreeLinkFunctionContext<'_>| Ok(color);
        tree_layout_with_factories(
            &c,
            &raw,
            &o,
            &mut TreeFactories {
                link_color: Some(&mut resolve),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let a = disabled(Color(1, 2, 3, 0));
    let b = disabled(Color(8, 9, 10, 255));
    assert_eq!(a, b);
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert_ne!(a.computed_link_styles(), b.computed_link_styles());
}

#[test]
fn empty_and_singleton_paths_have_no_link_callbacks_or_styles() {
    let c = config();
    for raw in [vec![], vec![HierarchyDatum::new("root", None, -0.)]] {
        let plain = tree_layout(&c, &raw, &TreeOptions::default()).unwrap();
        assert!(plain.computed_link_styles().is_empty());
        let called = Cell::new(0);
        let mut width = |_: &TreeLinkFunctionContext<'_>| {
            called.set(called.get() + 1);
            Err(ChartError("no source edge".into()))
        };
        let dynamic = tree_layout_with_factories(
            &c,
            &raw,
            &TreeOptions::default(),
            &mut TreeFactories {
                link_thickness: Some(&mut width),
                ..Default::default()
            },
        )
        .unwrap();
        let active = tree_layout_with_activity(
            &c,
            &raw,
            &TreeOptions::default(),
            &mut TreeFactories::default(),
            &TreeActivityOptions::default(),
            &mut TreeActivityFactories::default(),
        )
        .unwrap();
        assert_eq!(called.get(), 0);
        assert!(dynamic.computed_link_styles().is_empty());
        assert!(active.layout.computed_link_styles().is_empty());
        assert_eq!(plain, dynamic);
        assert_eq!(plain, active.layout);
    }
}
