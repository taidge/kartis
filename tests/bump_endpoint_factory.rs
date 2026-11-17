use std::cell::RefCell;
use std::rc::Rc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn source() -> Vec<BumpSeries> {
    vec![
        BumpSeries::new(
            "alpha",
            vec![
                BumpDatum::new("a", 3.),
                BumpDatum::new("b", 1.),
                BumpDatum::new("c", 2.),
            ],
        ),
        BumpSeries::new(
            "beta",
            vec![
                BumpDatum::new("a", 1.),
                BumpDatum::new("b", 2.),
                BumpDatum::new("c", 3.),
            ],
        ),
    ]
}

fn options() -> BumpLayoutOptions {
    BumpLayoutOptions {
        axes: BumpAxes::none(),
        labels: BumpEndpointLabels {
            start: true,
            end: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn identity_preserves_complete_layout_across_reverse_style_and_hidden_profiles() {
    let c = ChartConfig::default();
    let data = source();
    for reverse in [false, true] {
        for hidden in [false, true] {
            for interpolation in [BumpInterpolation::Linear, BumpInterpolation::Smooth] {
                let mut o = options();
                o.x_scale.reverse = reverse;
                o.rank_scale.reverse = reverse;
                o.interpolation = interpolation;
                o.style.opacity = 0.4;
                o.labels.start_style = Some(BumpEndpointLabelStyle {
                    padding: Some(2.),
                    color: Some(InheritedColor::FromContext(vec![])),
                });
                if hidden {
                    o.hidden.insert("alpha".into());
                }
                let original = bump_layout(&c, &data, &o).unwrap();
                let custom = bump_layout_with_label_factory(&c, &data, &o, |ctx| {
                    Ok(ctx.default_text.into())
                })
                .unwrap();
                assert_eq!(original, custom);
                assert_eq!(
                    original,
                    bump_layout_with_factories(&c, &data, &o, BumpFactories::default()).unwrap()
                );
            }
        }
    }
}

#[test]
fn endpoint_text_changes_only_owned_strings_and_borrows_original_series() {
    let c = ChartConfig::default();
    let data = source();
    let o = options();
    let original = bump_layout(&c, &data, &o).unwrap();
    let mut expected = original.clone();
    let mut visited = Vec::new();
    let custom = bump_layout_with_label_factory(&c, &data, &o, |ctx| {
        assert!(std::ptr::eq(ctx.series, &data[ctx.series_index]));
        assert_eq!(ctx.computed.source_index, ctx.series_index);
        assert_eq!(ctx.computed.source, *ctx.series);
        let text = format!(
            "{} {:?}: {}",
            ctx.default_text,
            ctx.endpoint,
            if ctx.endpoint == BumpEndpoint::Start {
                ctx.series.data[0].rank.unwrap()
            } else {
                ctx.series.data.last().unwrap().rank.unwrap()
            }
        );
        visited.push((
            ctx.series_index,
            ctx.endpoint,
            ctx.position,
            ctx.line_position,
            text.clone(),
        ));
        Ok(text)
    })
    .unwrap();
    assert_eq!(visited.len(), 4);
    for (label, (_, endpoint, position, line_position, text)) in
        expected.scene.labels.iter_mut().zip(visited)
    {
        assert_eq!([label.x, label.y], position);
        assert_eq!(
            label.anchor,
            if endpoint == BumpEndpoint::Start {
                TextAnchor::End
            } else {
                TextAnchor::Start
            }
        );
        assert_eq!(position[1], line_position[1]);
        label.text = text;
    }
    assert_eq!(expected, custom);
    assert_eq!(data, source());
}

#[test]
fn missing_disabled_hidden_and_empty_series_only_visit_actual_label_slots() {
    let data = vec![
        BumpSeries::new("none", vec![]),
        BumpSeries::new(
            "start-gap",
            vec![BumpDatum::gap("a"), BumpDatum::new("b", 2.)],
        ),
        BumpSeries::new(
            "end-gap",
            vec![BumpDatum::new("a", 1.), BumpDatum::gap("b")],
        ),
        BumpSeries::new("hidden", vec![BumpDatum::new("a", 4.)]),
    ];
    for enabled in [(true, true), (false, true), (true, false), (false, false)] {
        let mut o = options();
        o.hidden.insert("hidden".into());
        o.labels.start = enabled.0;
        o.labels.end = enabled.1;
        let old = bump_layout(&ChartConfig::default(), &data, &o).unwrap();
        let mut calls = Vec::new();
        let out = bump_layout_with_label_factory(&ChartConfig::default(), &data, &o, |ctx| {
            calls.push((ctx.series_index, ctx.endpoint));
            Ok(String::new())
        })
        .unwrap();
        let expected = [(1, BumpEndpoint::End), (2, BumpEndpoint::Start)]
            .into_iter()
            .filter(|(_, end)| {
                if *end == BumpEndpoint::Start {
                    enabled.0
                } else {
                    enabled.1
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(calls, expected);
        assert_eq!(out.scene.label_series, old.scene.label_series);
        assert_eq!(out.scene.labels.len(), calls.len());
        assert!(out.scene.labels.iter().all(|l| l.text.is_empty()));
        assert_eq!(out.interaction, old.interaction);
    }
}

fn custom_point(ctx: &BumpPointContext<'_>) -> Result<LinePointGeometry> {
    let [x, y] = ctx.position;
    Ok(ctx.inherit(Geometry::Path(vec![
        C::Move(P::new(x + 8., y - 3.)),
        C::Line(P::new(x + 14., y - 3.)),
        C::Line(P::new(x + 14., y + 3.)),
        C::Line(P::new(x + 8., y + 3.)),
        C::Close,
    ])))
}

#[test]
fn point_and_label_factories_share_one_deterministic_build_and_original_line_endpoints() {
    let c = ChartConfig::default();
    let data = source();
    let o = options();
    let old = bump_layout_with_point_factory(&c, &data, &o, custom_point).unwrap();
    let events = Rc::new(RefCell::new(vec![]));
    let points = events.clone();
    let labels = events.clone();
    let mut point = |ctx: &BumpPointContext<'_>| {
        points
            .borrow_mut()
            .push(format!("p{}:{}", ctx.series_index, ctx.source_index));
        custom_point(ctx)
    };
    let mut label = |ctx: &BumpEndpointLabelContext<'_>| {
        labels
            .borrow_mut()
            .push(format!("l{}:{:?}", ctx.series_index, ctx.endpoint));
        Ok(ctx.default_text.into())
    };
    let out = bump_layout_with_factories(
        &c,
        &data,
        &o,
        BumpFactories {
            point: Some(&mut point),
            endpoint_label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(out, old);
    assert_eq!(
        *events.borrow(),
        vec![
            "p0:0", "p0:1", "p0:2", "l0:Start", "l0:End", "p1:0", "p1:1", "p1:2", "l1:Start",
            "l1:End"
        ]
    );
}

#[test]
fn utf8_budget_and_late_callback_failure_return_no_candidate() {
    let c = ChartConfig::default();
    let data = source();
    let o = options();
    let old = bump_layout(&c, &data, &o).unwrap();
    let exact = "é".repeat(MAX_LABEL_BYTES / 2);
    let out = bump_layout_with_label_factory(&c, &data, &o, |_| Ok(exact.clone())).unwrap();
    assert!(
        out.scene
            .labels
            .iter()
            .all(|l| l.text.len() == MAX_LABEL_BYTES)
    );
    for text in [format!("{exact}x"), "x".repeat(MAX_LABEL_BYTES + 1)] {
        assert!(bump_layout_with_label_factory(&c, &data, &o, |_| Ok(text.clone())).is_err());
    }
    let mut calls = 0;
    let error = bump_layout_with_label_factory(&c, &data, &o, |ctx| {
        calls += 1;
        if calls == 3 {
            Err(ChartError("late endpoint failure".into()))
        } else {
            Ok(ctx.default_text.into())
        }
    })
    .unwrap_err();
    assert_eq!(calls, 3);
    assert_eq!(error.0, "late endpoint failure");
    assert_eq!(old, bump_layout(&c, &data, &o).unwrap());
    assert_eq!(data, source());
}

#[test]
fn endpoint_text_is_escaped_in_real_svg_and_canvas_serializers() {
    let out =
        bump_layout_with_label_factory(&ChartConfig::default(), &source(), &options(), |ctx| {
            Ok(format!("{:?} <&> \"é\"", ctx.endpoint))
        })
        .unwrap();
    let svg = render::to_svg(&out.scene).unwrap();
    assert!(svg.contains("Start &lt;&amp;&gt; &quot;é&quot;"));
    assert!(svg.contains("End &lt;&amp;&gt; &quot;é&quot;"));
    let canvas = render::to_canvas_script(&out.scene, "bump").unwrap();
    assert!(canvas.contains("\\u003c\\u0026\\u003e"));
    assert!(canvas.contains("Start"));
    assert!(canvas.contains("End"));
}
