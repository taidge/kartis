use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::render::Props;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
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

fn source(id: &str) -> BumpSeries {
    BumpSeries::new(id, vec![BumpDatum::new("Q1", 3.), BumpDatum::new("Q2", 1.)])
}

fn point_ink(ctx: &BumpPointContext<'_>) -> LinePointGeometry {
    ctx.inherit(Geometry::Circle {
        x: ctx.position[0],
        y: ctx.position[1],
        radius: ctx.style.size / 2.,
    })
}

#[test]
fn endpoint_context_uses_original_source_boundaries_not_global_category_order() {
    let seed = BumpSeries::new(
        "seed",
        vec![
            BumpDatum::new("early", 2.),
            BumpDatum::new("middle", 2.),
            BumpDatum::new("late", 2.),
        ],
    );
    let mut row = BumpSeries::new(
        "row",
        vec![
            BumpDatum::new("late", 4.),
            BumpDatum::new("early", 1.),
            BumpDatum::new("middle", 3.),
        ],
    );
    row.label = "Authored title".into();
    let data = [seed, row];
    for reverse in [false, true] {
        let mut o = options();
        o.hidden.insert("seed".into());
        o.x_scale.reverse = reverse;
        o.style.opacity = 0.5;
        o.labels.size = Some(17.);
        o.labels.start_style = Some(BumpEndpointLabelStyle {
            padding: Some(11.),
            color: Some(InheritedColor::Fixed(Color(30, 60, 90, 200))),
        });
        o.labels.end_style = Some(BumpEndpointLabelStyle {
            padding: Some(23.),
            color: Some(InheritedColor::Fixed(Color(80, 40, 160, 128))),
        });
        let baseline = bump_layout(&config(), &data, &o).unwrap();
        let mut visits = Vec::new();
        let out = bump_layout_with_label_factory(&config(), &data, &o, |ctx| {
            assert_eq!(ctx.series_index, 1);
            assert_eq!(ctx.series, &data[1]);
            assert_eq!(ctx.computed, &baseline.series[1]);
            assert_eq!(ctx.default_text, "Authored title");
            let index = if ctx.endpoint == BumpEndpoint::Start {
                0
            } else {
                1
            };
            assert_eq!(
                ctx.position,
                [
                    baseline.scene.labels[index].x,
                    baseline.scene.labels[index].y
                ]
            );
            let line = if index == 0 {
                ctx.computed.line_points.first()
            } else {
                ctx.computed.line_points.last()
            };
            assert_eq!(Some(Some(ctx.line_position)), line.copied());
            let datum = if index == 0 {
                ctx.series.data.first()
            } else {
                ctx.series.data.last()
            }
            .unwrap();
            assert_eq!(
                ctx.line_position[1],
                ctx.computed.points[if index == 0 { 0 } else { 2 }]
                    .position
                    .unwrap()[1]
            );
            visits.push(ctx.endpoint);
            Ok(format!(
                "{} rank {}",
                if index == 0 { "Opening" } else { "Closing" },
                datum.rank.unwrap()
            ))
        })
        .unwrap();
        assert_eq!(visits, [BumpEndpoint::Start, BumpEndpoint::End]);
        let mut expected = baseline.clone();
        expected.scene.labels[0].text = "Opening rank 4".into();
        expected.scene.labels[1].text = "Closing rank 3".into();
        assert_eq!(out, expected);
        assert_eq!(out.scene.labels[0].anchor, TextAnchor::End);
        assert_eq!(out.scene.labels[1].anchor, TextAnchor::Start);
        assert_eq!(out.scene.labels[0].color, Color(30, 60, 90, 100));
        assert_eq!(out.scene.labels[1].color, Color(80, 40, 160, 64));
        assert_eq!(out.scene.labels[0].size, 17.);
        assert_eq!(
            out.scene.label_series.get(&0).map(String::as_str),
            Some("row")
        );
        assert_eq!(
            out.scene.label_series.get(&1).map(String::as_str),
            Some("row")
        );
    }
}

#[test]
fn missing_disabled_and_hidden_endpoints_do_not_call_the_factory_or_shift_source_indices() {
    let data = [
        source("hidden"),
        BumpSeries::new(
            "leading",
            vec![BumpDatum::gap("Q1"), BumpDatum::new("Q2", 2.)],
        ),
        BumpSeries::new(
            "trailing",
            vec![BumpDatum::new("Q1", 1.), BumpDatum::gap("Q2")],
        ),
        BumpSeries::new(
            "interior",
            vec![
                BumpDatum::new("Q1", 3.),
                BumpDatum::gap("Q2"),
                BumpDatum::new("Q3", 1.),
            ],
        ),
        BumpSeries::new("missing", vec![BumpDatum::gap("Q1"), BumpDatum::gap("Q2")]),
        BumpSeries::new("single", vec![BumpDatum::new("Q2", 2.)]),
    ];
    let mut o = options();
    o.hidden.insert("hidden".into());
    for start in [true, false] {
        o.labels.start = start;
        let baseline = bump_layout(&config(), &data, &o).unwrap();
        let mut seen = Vec::new();
        let out = bump_layout_with_label_factory(&config(), &data, &o, |ctx| {
            seen.push((ctx.series_index, ctx.endpoint));
            Ok(ctx.default_text.to_owned())
        })
        .unwrap();
        let expected = if start {
            vec![
                (1, BumpEndpoint::End),
                (2, BumpEndpoint::Start),
                (3, BumpEndpoint::Start),
                (3, BumpEndpoint::End),
                (5, BumpEndpoint::Start),
                (5, BumpEndpoint::End),
            ]
        } else {
            vec![
                (1, BumpEndpoint::End),
                (3, BumpEndpoint::End),
                (5, BumpEndpoint::End),
            ]
        };
        assert_eq!(seen, expected);
        assert_eq!(out, baseline);
    }
    o.hidden = data.iter().map(|row| row.id.clone()).collect();
    let calls = Cell::new(0);
    let all_hidden = bump_layout_with_label_factory(&config(), &data, &o, |_| {
        calls.set(calls.get() + 1);
        Err(ChartError("unreachable hidden callback".into()))
    })
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert!(all_hidden.scene.labels.is_empty());
}

#[test]
fn combined_factories_have_series_local_order_and_stop_before_later_work_on_error() {
    let data = [source("A"), source("B")];
    let o = options();
    let accepted = bump_layout(&config(), &data, &o).unwrap();
    for failure in [0, 1, 2] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let point_log = log.clone();
        let label_log = log.clone();
        let mut points = |ctx: &BumpPointContext<'_>| {
            point_log
                .borrow_mut()
                .push(format!("point:{}:{}", ctx.series.id, ctx.source_index));
            if failure == 1 && ctx.series.id == "A" && ctx.source_index == 1 {
                return Err(ChartError("point stop".into()));
            }
            Ok(point_ink(ctx))
        };
        let mut labels = |ctx: &BumpEndpointLabelContext<'_>| {
            label_log
                .borrow_mut()
                .push(format!("label:{}:{:?}", ctx.series.id, ctx.endpoint));
            assert!(ctx.computed.points.iter().all(|p| p.mark_id.is_some()));
            assert!(!ctx.computed.line_mark_ids.is_empty());
            if failure == 2 && ctx.series.id == "A" && ctx.endpoint == BumpEndpoint::End {
                return Err(ChartError("label stop".into()));
            }
            Ok(ctx.default_text.to_owned())
        };
        let out = bump_layout_with_factories(
            &config(),
            &data,
            &o,
            BumpFactories {
                point: Some(&mut points),
                endpoint_label: Some(&mut labels),
            },
        );
        let expected: Vec<String> = match failure {
            0 => vec![
                "point:A:0",
                "point:A:1",
                "label:A:Start",
                "label:A:End",
                "point:B:0",
                "point:B:1",
                "label:B:Start",
                "label:B:End",
            ],
            1 => vec!["point:A:0", "point:A:1"],
            _ => vec!["point:A:0", "point:A:1", "label:A:Start", "label:A:End"],
        }
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(*log.borrow(), expected);
        if failure == 0 {
            let old_entry =
                bump_layout_with_point_factory(&config(), &data, &o, |ctx| Ok(point_ink(ctx)))
                    .unwrap();
            assert_eq!(out.unwrap(), old_entry);
        } else {
            assert!(out.is_err());
        }
        assert_eq!(accepted, bump_layout(&config(), &data, &o).unwrap());
    }
}

#[test]
fn text_slots_remain_owned_filterable_and_real_glory_text_at_the_utf8_byte_limit() {
    let data = [source("A"), source("B")];
    let limit = "é".repeat(MAX_LABEL_BYTES / 2);
    let mut out = bump_layout_with_label_factory(&config(), &data, &options(), |ctx| {
        Ok(match (ctx.series_index, ctx.endpoint) {
            (0, BumpEndpoint::Start) => String::new(),
            (0, BumpEndpoint::End) => limit.clone(),
            (1, BumpEndpoint::Start) => "B <&> début".into(),
            _ => "  B end  ".into(),
        })
    })
    .unwrap();
    assert_eq!(out.scene.labels.len(), 4);
    assert!(out.scene.labels[0].text.is_empty());
    assert_eq!(out.scene.labels[1].text.len(), MAX_LABEL_BYTES);
    assert_eq!(out.scene.legend[0].label, "A");
    assert!(
        out.interaction
            .sites
            .iter()
            .all(|site| !site.label.contains("début"))
    );
    out.scene
        .typography
        .label_whitespace
        .insert(3, LabelWhitespace::Preserve);
    let app = glory::App::memory();
    app.mount(chart(out.scene.clone())).unwrap();
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut actual = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if let Props::Text(text) = node.descriptor.view().props {
            actual.push(text.content.clone());
        }
        pending.extend(node.children.iter().copied());
    }
    assert!(actual.contains(&limit));
    assert!(actual.iter().any(|text| text == "B <&> début"));
    assert!(actual.iter().any(|text| text == "  B end  "));
    let filtered = out.scene.without_series(&BTreeSet::from(["A".into()]));
    filtered.validate().unwrap();
    assert_eq!(filtered.labels.len(), 2);
    assert_eq!(filtered.labels[0].text, "B <&> début");
    assert_eq!(filtered.labels[1].text, "  B end  ");
    assert_eq!(filtered.label_series.get(&0).map(String::as_str), Some("B"));
    assert_eq!(
        filtered.typography.label_whitespace(1),
        LabelWhitespace::Preserve
    );
    assert!(!filtered.typography.label_whitespace.contains_key(&3));
    assert!(
        render::to_svg(&filtered)
            .unwrap()
            .contains("B &lt;&amp;&gt; début")
    );
}

#[test]
fn oversized_utf8_result_stops_before_the_next_series_and_keeps_source_unchanged() {
    let data = [source("A"), source("B")];
    let original = data.clone();
    let accepted = bump_layout(&config(), &data, &options()).unwrap();
    let mut seen = Vec::new();
    let error = bump_layout_with_label_factory(&config(), &data, &options(), |ctx| {
        seen.push((ctx.series_index, ctx.endpoint));
        Ok(if ctx.endpoint == BumpEndpoint::Start {
            String::new()
        } else {
            "é".repeat(MAX_LABEL_BYTES / 2) + "x"
        })
    })
    .unwrap_err();
    assert!(error.0.contains("text"));
    assert_eq!(seen, [(0, BumpEndpoint::Start), (0, BumpEndpoint::End)]);
    assert_eq!(data, original);
    assert_eq!(accepted, bump_layout(&config(), &data, &options()).unwrap());
}

#[test]
fn transparent_owned_endpoints_and_zero_point_size_do_not_suppress_callbacks() {
    let data = [source("A")];
    let mut o = options();
    o.style.width = 0.;
    o.style.opacity = 0.;
    o.style.point.size = 0.;
    o.labels.color = InheritedColor::Fixed(Color(30, 40, 50, 0));
    let baseline = bump_layout(&config(), &data, &o).unwrap();
    let mut calls = Vec::new();
    let out = bump_layout_with_label_factory(&config(), &data, &o, |ctx| {
        calls.push(ctx.endpoint);
        Ok(ctx.default_text.to_owned())
    })
    .unwrap();
    assert_eq!(calls, [BumpEndpoint::Start, BumpEndpoint::End]);
    assert_eq!(out, baseline);
    assert_eq!(out.scene.labels.len(), 2);
    assert!(out.scene.labels.iter().all(|label| label.color.3 == 0));
    assert_eq!(out.interaction.sites.len(), 2);
    assert!(out.series[0].line_mark_ids.is_empty());
}
