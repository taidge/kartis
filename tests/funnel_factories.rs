use std::cell::RefCell;
use std::rc::Rc;

use kartis::*;

fn data() -> Vec<FunnelDatum> {
    vec![FunnelDatum::new("a", 8.), FunnelDatum::new(0., -0.)]
}

#[test]
fn unused_border_color_keeps_legacy_transparent_scene_admission() {
    let mut c = ChartConfig::default();
    c.theme.background = Color(0, 0, 0, 0);
    let mut source = data();
    for d in &mut source {
        d.color = Some(Color(20, 30, 40, 0));
    }
    let o = FunnelLayoutOptions {
        border_width: 0.,
        labels: None,
        before: None,
        after: None,
        border_color: InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        },
        ..Default::default()
    };
    assert!(
        funnel_layout(&c, &source, &o).is_ok(),
        "an unused contrast role must not require an opaque effective background"
    );
}
#[test]
fn default_and_inherited_functions_preserve_every_field() {
    for direction in [FunnelDirection::Vertical, FunnelDirection::Horizontal] {
        let o = FunnelLayoutOptions {
            direction,
            ..Default::default()
        };
        let c = ChartConfig::default();
        let data = data();
        let old = funnel_layout(&c, &data, &o).unwrap();
        let mut color = |ctx: &FunnelColorContext<'_>| Ok(ctx.default_color);
        let mut border = |ctx: &FunnelPartStyleContext<'_>| Ok(ctx.default_color);
        let mut label = |ctx: &FunnelPartStyleContext<'_>| Ok(ctx.default_color);
        let actual = funnel_layout_with_factories(
            &c,
            &data,
            &o,
            &mut FunnelFactories {
                color: Some(&mut color),
                border_color: Some(&mut border),
                label_color: Some(&mut label),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(actual, old);
        assert_eq!(
            render::to_svg(&actual.scene).unwrap(),
            render::to_svg(&old.scene).unwrap()
        );
    }
}
#[test]
fn functions_follow_raw_color_format_border_label_dependencies_and_reach_all_consumers() {
    let trace = Rc::new(RefCell::new(vec![]));
    let a = trace.clone();
    let mut color = move |ctx: &FunnelColorContext<'_>| {
        a.borrow_mut().push(format!("color:{}", ctx.source_index));
        assert_eq!(
            ctx.datum.value.to_bits(),
            if ctx.source_index == 0 {
                8f64.to_bits()
            } else {
                (-0f64).to_bits()
            }
        );
        Ok(Color(25, 140, 80, 255))
    };
    let a = trace.clone();
    let mut format = move |v: f64| {
        a.borrow_mut().push(format!("format:{}", v.to_bits()));
        Ok(format!("value={}", v.to_bits()))
    };
    let a = trace.clone();
    let mut border = move |ctx: &FunnelPartStyleContext<'_>| {
        a.borrow_mut()
            .push(format!("border:{}", ctx.part.source_index));
        assert_eq!(ctx.part.color, Color(25, 140, 80, 255));
        assert!(ctx.part.formatted_value.starts_with("value="));
        assert_eq!(ctx.border_width, if ctx.part.is_current { 9. } else { 6. });
        Ok(Color(230, 110, 20, 255))
    };
    let a = trace.clone();
    let mut label = move |ctx: &FunnelPartStyleContext<'_>| {
        a.borrow_mut()
            .push(format!("label:{}", ctx.part.source_index));
        assert_eq!(ctx.border_color, Color(230, 110, 20, 255));
        Ok(Color(100, 20, 180, 255))
    };
    let source = data();
    let layout = funnel_layout_with_factories(
        &ChartConfig::default(),
        &source,
        &FunnelLayoutOptions {
            current: Some("a".into()),
            current_border_width: Some(9.),
            ..Default::default()
        },
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(
        &*trace.borrow(),
        &[
            "color:0",
            &format!("format:{}", 8f64.to_bits()),
            "border:0",
            "label:0",
            "color:1",
            &format!("format:{}", (-0f64).to_bits()),
            "border:1",
            "label:1"
        ]
    );
    assert_eq!(
        layout.parts[1].datum.value.to_bits(),
        source[1].value.to_bits()
    );
    assert!(
        layout
            .parts
            .iter()
            .all(|p| p.color == Color(25, 140, 80, 255))
    );
    assert!(
        layout
            .scene
            .legend
            .iter()
            .all(|l| l.color == Color(25, 140, 80, 255))
    );
    assert!(
        layout
            .scene
            .labels
            .iter()
            .all(|l| l.color == Color(100, 20, 180, 255) && l.text.starts_with("value="))
    );
    assert!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("funnel:border:"))
            .all(|m| m.stroke.unwrap().color == Color(230, 110, 20, 255))
    );
    for site in layout.interaction_binding().sites {
        assert_eq!(site.color, Color(25, 140, 80, 255));
        assert_eq!(
            site.formatted_value.as_deref(),
            Some(
                layout
                    .part_for_mark(&site.mark_id)
                    .unwrap()
                    .formatted_value
                    .as_str()
            )
        );
    }
    assert!(funnel_widgets(&layout, &source, 0, |_| Ok(None)).is_ok());
}
#[test]
fn roles_run_for_unpainted_stages_without_creating_marks_or_labels() {
    let mut source = data();
    for d in &mut source {
        d.size = Some(0.);
    }
    let calls = Rc::new(RefCell::new([0; 4]));
    let c = calls.clone();
    let mut color = move |_: &FunnelColorContext<'_>| {
        c.borrow_mut()[0] += 1;
        Ok(Color(1, 2, 3, 255))
    };
    let c = calls.clone();
    let mut format = move |_: f64| {
        c.borrow_mut()[1] += 1;
        Ok("zero".into())
    };
    let c = calls.clone();
    let mut border = move |_: &FunnelPartStyleContext<'_>| {
        c.borrow_mut()[2] += 1;
        Ok(Color(3, 2, 1, 255))
    };
    let c = calls.clone();
    let mut label = move |_: &FunnelPartStyleContext<'_>| {
        c.borrow_mut()[3] += 1;
        Ok(Color(8, 9, 10, 255))
    };
    let layout = funnel_layout_with_factories(
        &ChartConfig::default(),
        &source,
        &FunnelLayoutOptions {
            size: FunnelSize::DatumPixels,
            border_width: 0.,
            labels: None,
            before: None,
            after: None,
            ..Default::default()
        },
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(*calls.borrow(), [2; 4]);
    assert_eq!(layout.parts.len(), 2);
    assert!(layout.scene.marks.is_empty() && layout.scene.labels.is_empty());
    assert!(layout.interaction_binding().sites.is_empty());
}
#[test]
fn whole_input_validation_precedes_callbacks_and_formatter_budget_is_enforced() {
    let mut source = data();
    source[1].value = f64::NAN;
    let mut calls = 0;
    {
        let mut color = |_: &FunnelColorContext<'_>| {
            calls += 1;
            Ok(Color(1, 2, 3, 255))
        };
        assert!(
            funnel_layout_with_factories(
                &ChartConfig::default(),
                &source,
                &FunnelLayoutOptions::default(),
                &mut FunnelFactories {
                    color: Some(&mut color),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert_eq!(calls, 0);
    let mut format = |_: f64| Ok("x".repeat(100_000));
    assert!(
        funnel_layout_with_factories(
            &ChartConfig::default(),
            &data(),
            &FunnelLayoutOptions::default(),
            &mut FunnelFactories {
                formatted_value: Some(&mut format),
                ..Default::default()
            }
        )
        .unwrap_err()
        .0
        .contains("label budget")
    );
    let mut format = |_: f64| Err(ChartError("formatter failure".into()));
    assert_eq!(
        funnel_layout_with_factories(
            &ChartConfig::default(),
            &data(),
            &FunnelLayoutOptions::default(),
            &mut FunnelFactories {
                formatted_value: Some(&mut format),
                ..Default::default()
            }
        )
        .unwrap_err()
        .0,
        "formatter failure"
    );
}
#[test]
fn custom_ink_retains_explicit_label_factory_without_replaying_color_callbacks() {
    let mut calls = 0;
    let mut label = |_: &FunnelPartStyleContext<'_>| {
        calls += 1;
        Ok(Color(200, 10, 30, 255))
    };
    let source = data();
    let layout = funnel_layout_with_factories_and_stage_factory(
        &ChartConfig::default(),
        &source,
        &FunnelLayoutOptions::default(),
        &mut FunnelFactories {
            label_color: Some(&mut label),
            ..Default::default()
        },
        &mut |ctx| {
            let mut geometry = ctx.inherit(ctx.geometry.clone());
            geometry.body.fill = Some(Color(10, 30, 200, 255));
            geometry.body.fill_paint = None;
            Ok(geometry)
        },
    )
    .unwrap();
    assert_eq!(calls, 2);
    assert!(
        layout
            .scene
            .labels
            .iter()
            .all(|l| l.color == Color(200, 10, 30, 255))
    );
    assert_eq!(layout.parts[0].color, Color(10, 30, 200, 255));
    assert!(funnel_widgets(&layout, &source, 0, |_| Ok(None)).is_ok());
}
