//! Independent real-Mark eligibility and complete Funnel custom-layer inputs.
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as Cmd};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 410.,
        height: 290.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 19.,
            bottom: 27.,
        },
        ..Default::default()
    }
}
fn raw() -> Vec<FunnelDatum> {
    let mut d = vec![
        FunnelDatum::new(-0., 10.),
        FunnelDatum::new("middle", 4.),
        FunnelDatum::new("zero", -0.),
    ];
    d[0].label = "Original zero key".into();
    d[0].size = Some(-0.);
    d[2].size = Some(-0.);
    d
}
fn options() -> FunnelLayoutOptions {
    FunnelLayoutOptions {
        before: Some(FunnelSeparatorOptions {
            length: 18.,
            offset: 4.,
            ..Default::default()
        }),
        after: Some(FunnelSeparatorOptions {
            length: 14.,
            offset: 5.,
            ..Default::default()
        }),
        legend: false,
        ..Default::default()
    }
}
fn slot() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-5., -3.],
        measured: TextSize {
            width: 10.,
            height: 6.,
        },
    }
}

#[test]
fn physical_roles_share_complete_pools_and_terminal_separator_has_no_fake_datum() {
    let raw = raw();
    let l = funnel_layout(&config(), &raw, &options()).unwrap();
    let original = l.clone();
    let mut order = vec![];
    let mut terminal = 0;
    let p = funnel_widgets(&l, &raw, 7, |c| {
        order.push(c.mark.id.clone());
        assert!(std::ptr::eq(c.source, raw.as_slice()));
        assert_eq!(c.plot, l.plot());
        if let Some(i) = c.source_index {
            assert!(std::ptr::eq(c.datum.unwrap(), &raw[i]));
            assert_eq!(c.part_index, Some(i));
        } else {
            terminal += 1;
            assert!(c.datum.is_none() && c.part.is_none() && c.part_index.is_none());
            assert!(c.separator.unwrap().part_id.is_none());
        }
        if let Some(s) = c.separator {
            assert_eq!(
                c.position,
                [
                    s.from[0] * 0.5 + s.to[0] * 0.5,
                    s.from[1] * 0.5 + s.to[1] * 0.5
                ]
            );
        } else {
            assert_eq!(c.position, c.part.unwrap().center);
        }
        Ok(Some(slot()))
    })
    .unwrap();
    assert_eq!(terminal, 2);
    assert_eq!(
        order,
        l.scene
            .paint_marks()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>()
    );
    let first = p.data.values().next().unwrap();
    for s in &p.layout.slots {
        let d = &p.data[&s.mark_id];
        assert!(
            Arc::ptr_eq(&d.source, &first.source)
                && Arc::ptr_eq(&d.parts, &first.parts)
                && Arc::ptr_eq(&d.separators, &first.separators)
                && Arc::ptr_eq(&d.labels, &first.labels)
        );
        assert_eq!(d.source.len(), 3);
        assert_eq!(d.source[2].value.to_bits(), (-0_f64).to_bits());
        let PlotWidgetPlacement::Rect(r) = s.placement else {
            panic!("original role anchor must resolve explicitly")
        };
        assert_eq!([r.x + 5., r.y + 3.], d.position);
        assert_eq!(d.labels.labels, l.scene.labels);
        assert_eq!(d.labels.series, l.scene.label_series);
    }
    let f = funnel_widget_frame(&l).unwrap();
    assert_eq!(f.scene, l.scene);
    assert_eq!(f.plot, Some(l.plot()));
    assert!(
        f.interaction
            .unwrap()
            .sites
            .iter()
            .all(|s| l.parts.iter().any(|p| p.mark_id == s.mark_id))
    );
    assert_eq!(l, original);
}

#[test]
fn zero_body_parts_keep_only_actual_border_and_separator_roles_even_at_zero_opacity() {
    let raw = raw();
    let mut o = options();
    o.size = FunnelSize::DatumPixels;
    let l = funnel_layout_with_size(&config(), &raw, &o, |_| Ok(0.)).unwrap();
    assert!(
        !l.scene
            .marks
            .iter()
            .any(|m| l.parts.iter().any(|p| p.mark_id == m.id))
    );
    let mut roles = vec![];
    let p = funnel_widgets(&l, &raw, 0, |c| {
        roles.push(c.role);
        Ok(Some(slot()))
    })
    .unwrap();
    assert!(!roles.contains(&FunnelWidgetRole::Body));
    assert!(
        roles.contains(&FunnelWidgetRole::Border)
            && roles.contains(&FunnelWidgetRole::BeforeSeparator)
            && roles.contains(&FunnelWidgetRole::AfterSeparator)
    );
    assert_eq!(p.data.len(), l.scene.marks.len());
    let mut transparent = l.clone();
    for m in &mut transparent.scene.marks {
        m.opacity = 0.;
    }
    let mut calls = 0;
    assert_eq!(
        funnel_widgets(&transparent, &raw, 0, |_| {
            calls += 1;
            Ok(Some(slot()))
        })
        .unwrap()
        .data
        .len(),
        p.data.len()
    );
    assert_eq!(calls, p.data.len());
}

#[test]
fn all_late_source_and_public_structure_conflicts_fail_before_any_callback() {
    type Mutation = fn(&mut FunnelLayout, &mut Vec<FunnelDatum>);
    let mutations: Vec<(&str, Mutation)> = vec![
        ("late raw value", |_, d| d[2].value = 1.),
        ("unused raw size sign", |_, d| d[2].size = Some(0.)),
        ("numeric key sign", |_, d| d[0].id = CategoryKey::Number(0.)),
        ("raw label", |_, d| d[2].label = "forged".into()),
        ("raw color", |_, d| d[2].color = Some(Color(1, 2, 3, 255))),
        ("part source index", |l, _| l.parts[2].source_index = 0),
        ("part raw size", |l, _| l.parts[2].datum.size = Some(0.)),
        ("part center", |l, _| l.parts[2].center[0] += 2.),
        ("part bound", |l, _| l.parts[2].bounds.x += 1.),
        ("part profile", |l, _| l.parts[2].profile[3].after[0] += 2.),
        ("last separator", |l, _| {
            l.separators.last_mut().unwrap().to[0] += 1.
        }),
        ("terminal owner", |l, d| {
            l.separators.last_mut().unwrap().part_id = Some(d[2].id.clone())
        }),
        ("label owner", |l, _| {
            l.scene.label_series.insert(0, "funnel:missing".into());
        }),
        ("actual body scalar", |l, _| {
            let id = l.parts[0].mark_id.clone();
            l.scene.marks.iter_mut().find(|m| m.id == id).unwrap().value = 99.;
        }),
        ("orphan physical mark", |l, _| {
            l.scene.marks.push(Mark::new(
                "orphan",
                Geometry::Circle {
                    x: 60.,
                    y: 60.,
                    radius: 2.,
                },
                Color(1, 2, 3, 255),
            ))
        }),
    ];
    let data = raw();
    let l = funnel_layout(&config(), &data, &options()).unwrap();
    for (name, mutate) in mutations {
        let (mut candidate, mut source) = (l.clone(), data.clone());
        mutate(&mut candidate, &mut source);
        let mut calls = 0;
        assert!(
            funnel_widgets(&candidate, &source, 0, |_| {
                calls += 1;
                Ok(Some(slot()))
            })
            .is_err(),
            "{name}"
        );
        assert_eq!(calls, 0, "{name}");
    }
}

#[test]
fn offset_custom_path_uses_stage_center_and_current_presentation_is_not_frozen() {
    let raw = raw();
    let l = funnel_layout_with_stage_factory(&config(), &raw, &options(), &mut |c| {
        let [x, y] = c.part.center;
        let mut g = c.inherit(Geometry::Path(vec![
            Cmd::Move(P::new(x + 30., y + 10.)),
            Cmd::Line(P::new(x + 42., y + 10.)),
            Cmd::Line(P::new(x + 36., y + 22.)),
            Cmd::Close,
        ]));
        g.retain_profile_border = false;
        Ok(g)
    })
    .unwrap();
    let p = funnel_widgets(&l, &raw, 0, |c| {
        Ok((c.role == FunnelWidgetRole::Body).then(slot))
    })
    .unwrap();
    for d in p.data.values() {
        assert_eq!(d.position, l.parts[d.part_index.unwrap()].center);
    }
    let mut decorated = l.clone();
    let body = l.parts[0].mark_id.clone();
    decorated
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == body)
        .unwrap()
        .fill = Some(Color(200, 20, 60, 255));
    decorated.scene.labels[0].text = "Current owned label".into();
    let p = funnel_widgets(&decorated, &raw, 0, |_| Ok(Some(slot()))).unwrap();
    assert!(
        p.data
            .values()
            .next()
            .unwrap()
            .labels
            .labels
            .iter()
            .any(|l| l.text == "Current owned label")
    );
    assert_eq!(
        funnel_widget_frame(&decorated).unwrap().scene,
        decorated.scene
    );
    let mut calls = 0;
    let e = funnel_widgets(&l, &raw, 0, |_| {
        calls += 1;
        if calls == 3 {
            Err(ChartError("independent Funnel placement stopped".into()))
        } else {
            Ok(Some(slot()))
        }
    })
    .unwrap_err();
    assert_eq!(e.0, "independent Funnel placement stopped");
    assert_eq!(calls, 3);
}
