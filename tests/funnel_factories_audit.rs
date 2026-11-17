//! Independent formatting/style consumers, defaults and bounded failure checks.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use kartis::*;

fn data() -> Vec<FunnelDatum> {
    vec![
        FunnelDatum::new("first", 10.),
        FunnelDatum::new("middle", 4.),
        FunnelDatum::new("zero", -0.),
    ]
}

#[test]
fn inherited_callbacks_preserve_whole_default_frames_across_zero_current_and_size_profiles() {
    let c = ChartConfig {
        width: 413.5,
        height: 287.25,
        ..Default::default()
    };
    for direction in [FunnelDirection::Vertical, FunnelDirection::Horizontal] {
        for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth] {
            for all_zero in [false, true] {
                for current in [false, true] {
                    let mut d = data();
                    if all_zero {
                        for x in &mut d {
                            x.value = 0.;
                        }
                    }
                    let o = FunnelLayoutOptions {
                        direction,
                        interpolation,
                        spacing: 3.25,
                        current: current.then_some(CategoryKey::String("zero".into())),
                        current_size_extension: 2.5,
                        current_border_width: Some(4.5),
                        size: FunnelSize::OrdinalPixels(vec![70., 0., 35.]),
                        ..Default::default()
                    };
                    let expected = funnel_layout(&c, &d, &o).unwrap();
                    let mut color = |x: &FunnelColorContext<'_>| Ok(x.default_color);
                    let mut border = |x: &FunnelPartStyleContext<'_>| Ok(x.default_color);
                    let mut label = |x: &FunnelPartStyleContext<'_>| Ok(x.default_color);
                    let mut format = |v| o.value_format.format(v);
                    let actual = funnel_layout_with_factories(
                        &c,
                        &d,
                        &o,
                        &mut FunnelFactories {
                            color: Some(&mut color),
                            formatted_value: Some(&mut format),
                            border_color: Some(&mut border),
                            label_color: Some(&mut label),
                        },
                    )
                    .unwrap();
                    assert_eq!(actual, expected);
                    assert_eq!(
                        render::to_svg(&actual.scene).unwrap(),
                        render::to_svg(&expected.scene).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn explicit_label_and_border_roles_survive_custom_body_ink_without_replaying_factories() {
    let d = data();
    let trace = Rc::new(RefCell::new(vec![]));
    let a = trace.clone();
    let mut color = move |x: &FunnelColorContext<'_>| {
        a.borrow_mut().push(format!("color:{}", x.source_index));
        Ok(Color(10 + x.source_index as u8, 20, 30, 255))
    };
    let a = trace.clone();
    let mut format = move |v: f64| {
        a.borrow_mut().push("format".into());
        Ok(format!("raw:{:016x}", v.to_bits()))
    };
    let a = trace.clone();
    let mut border = move |x: &FunnelPartStyleContext<'_>| {
        a.borrow_mut()
            .push(format!("border:{}", x.part.source_index));
        assert_eq!(
            x.part.color,
            Color(10 + x.part.source_index as u8, 20, 30, 255)
        );
        assert!(x.part.profile.iter().any(|p| p.before != x.part.center));
        Ok(Color(180, 50 + x.part.source_index as u8, 60, 255))
    };
    let a = trace.clone();
    let mut label = move |x: &FunnelPartStyleContext<'_>| {
        a.borrow_mut()
            .push(format!("label:{}", x.part.source_index));
        assert_eq!(
            x.border_color,
            Color(180, 50 + x.part.source_index as u8, 60, 255)
        );
        Ok(Color(20, 220, 100, 255))
    };
    let purple = Color(130, 20, 180, 255);
    let mut stage_calls = 0;
    let l = funnel_layout_with_factories_and_stage_factory(
        &ChartConfig::default(),
        &d,
        &FunnelLayoutOptions::default(),
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
        },
        &mut |ctx| {
            stage_calls += 1;
            let mut g = ctx.inherit(ctx.geometry.clone());
            g.body.fill = Some(purple);
            Ok(g)
        },
    )
    .unwrap();
    assert_eq!(stage_calls, 2);
    assert_eq!(trace.borrow().len(), 12);
    for (i, part) in l.parts.iter().enumerate() {
        assert_eq!(part.datum, d[i]);
        assert_eq!(
            part.formatted_value,
            format!("raw:{:016x}", d[i].value.to_bits())
        );
        let border_id = part.mark_id.replace("funnel:stage:", "funnel:border:");
        let mark = l.scene.marks.iter().find(|m| m.id == border_id).unwrap();
        assert_eq!(
            mark.stroke.unwrap().color,
            Color(180, 50 + i as u8, 60, 255)
        );
        if let Some(body) = l.scene.marks.iter().find(|m| m.id == part.mark_id) {
            assert_eq!(body.fill, Some(purple));
            assert_eq!(part.color, purple);
            assert_eq!(
                l.interaction_binding()
                    .sites
                    .iter()
                    .find(|s| s.mark_id == part.mark_id)
                    .unwrap()
                    .formatted_value
                    .as_deref(),
                Some(part.formatted_value.as_str())
            );
        }
        for (&slot, owner) in &l.scene.label_series {
            if owner == &part.series {
                assert_eq!(l.scene.labels[slot].color, Color(20, 220, 100, 255));
                assert_eq!(l.scene.labels[slot].text, part.formatted_value);
            }
        }
        assert_eq!(
            l.scene
                .legend
                .iter()
                .find(|x| x.id == part.series)
                .unwrap()
                .color,
            part.color
        );
    }
}

#[test]
fn disabled_visual_roles_still_run_and_formatter_utf8_budget_and_late_source_are_checked() {
    let c = ChartConfig::default();
    let d = data();
    let o = FunnelLayoutOptions {
        labels: None,
        border_width: 0.,
        before: None,
        after: None,
        legend: false,
        fill_opacity: 0.,
        ..Default::default()
    };
    let calls = Rc::new(Cell::new(0));
    let a = calls.clone();
    let mut color = move |x: &FunnelColorContext<'_>| {
        a.set(a.get() + 1);
        Ok(x.default_color)
    };
    let a = calls.clone();
    let mut border = move |x: &FunnelPartStyleContext<'_>| {
        a.set(a.get() + 1);
        Ok(x.default_color)
    };
    let a = calls.clone();
    let mut label = move |x: &FunnelPartStyleContext<'_>| {
        a.set(a.get() + 1);
        Ok(x.default_color)
    };
    let boundary = "界".repeat(341) + "a";
    assert_eq!(boundary.len(), 1024);
    let mut format = |_: f64| Ok(boundary.clone());
    let l = funnel_layout_with_factories(
        &c,
        &d,
        &o,
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 9);
    assert!(l.scene.labels.is_empty());
    assert!(l.parts.iter().all(|p| p.formatted_value.len() == 1024));
    let mut too_long = |_: f64| Ok(boundary.clone() + "b");
    assert!(
        funnel_layout_with_factories(
            &c,
            &d,
            &o,
            &mut FunnelFactories {
                formatted_value: Some(&mut too_long),
                ..Default::default()
            }
        )
        .is_err()
    );
    calls.set(0);
    let mut bad = d.clone();
    bad[2].size = Some(f64::NAN);
    assert!(
        funnel_layout_with_factories(
            &c,
            &bad,
            &o,
            &mut FunnelFactories {
                color: Some(&mut color),
                formatted_value: Some(&mut format),
                border_color: Some(&mut border),
                label_color: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut fail = |_: f64| Err(ChartError("independent formatter failure".into()));
    assert_eq!(
        funnel_layout_with_factories(
            &c,
            &d,
            &o,
            &mut FunnelFactories {
                formatted_value: Some(&mut fail),
                ..Default::default()
            }
        )
        .unwrap_err()
        .0,
        "independent formatter failure"
    );
}

#[test]
fn unused_contrast_does_not_activate_for_color_or_format_but_explicit_style_requests_defaults() {
    let mut c = ChartConfig::default();
    c.theme.background = Color(0, 0, 0, 0);
    let mut datum = FunnelDatum::new("only", 8.);
    datum.color = Some(Color(20, 40, 60, 0));
    let contrast = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    for (border_width, border_opacity, current_border_width) in
        [(0., 1., None), (6., 0., None), (6., 1., Some(0.))]
    {
        for labels in [
            None,
            Some(FunnelLabelOptions {
                color: InheritedColor::Fixed(Color(90, 80, 70, 255)),
                ..Default::default()
            }),
        ] {
            let o = FunnelLayoutOptions {
                border_width,
                border_opacity,
                border_color: contrast.clone(),
                current: Some(datum.id.clone()),
                current_border_width,
                labels,
                before: None,
                after: None,
                legend: false,
                ..Default::default()
            };
            let source = std::slice::from_ref(&datum);
            let expected = funnel_layout(&c, source, &o).unwrap();
            assert!(expected.scene.marks.iter().all(|m| m.stroke.is_none()));
            let mut color_calls = 0;
            let mut color = |ctx: &FunnelColorContext<'_>| {
                color_calls += 1;
                Ok(ctx.default_color)
            };
            let mut format_calls = 0;
            let mut format = |value| {
                format_calls += 1;
                o.value_format.format(value)
            };
            let actual = funnel_layout_with_factories(
                &c,
                source,
                &o,
                &mut FunnelFactories {
                    color: Some(&mut color),
                    formatted_value: Some(&mut format),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(color_calls, 1);
            assert_eq!(format_calls, 1);
            let style_calls = Cell::new(0);
            let mut style = |_: &FunnelPartStyleContext<'_>| {
                style_calls.set(style_calls.get() + 1);
                Ok(Color(1, 2, 3, 255))
            };
            assert!(
                funnel_layout_with_factories(
                    &c,
                    source,
                    &o,
                    &mut FunnelFactories {
                        border_color: Some(&mut style),
                        ..Default::default()
                    }
                )
                .unwrap_err()
                .0
                .contains("opaque effective background")
            );
            assert_eq!(
                style_calls.get(),
                0,
                "invalid requested default rejects before callback"
            );
            assert!(
                funnel_layout_with_factories(
                    &c,
                    source,
                    &o,
                    &mut FunnelFactories {
                        label_color: Some(&mut style),
                        ..Default::default()
                    }
                )
                .is_err()
            );
            assert_eq!(style_calls.get(), 0);
        }
    }
    let active = FunnelLayoutOptions {
        labels: None,
        border_color: contrast,
        ..Default::default()
    };
    assert!(funnel_layout(&c, std::slice::from_ref(&datum), &active).is_err());
}
