use glory::{
    App,
    render::{Event, NodeId, Props},
    style::{Display, FlexDirection, FlexWrap, Justify, Length, WhiteSpace},
};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    for (index, id) in ["alpha", "beta"].iter().enumerate() {
        let mut mark = Mark::new(
            *id,
            Geometry::Circle {
                x: 20. + index as f64 * 40.,
                y: 30.,
                radius: 10.,
            },
            Color(10, 20, 30, 255),
        );
        mark.series = (*id).into();
        mark.label = format!("{id} value");
        mark.value = 1.25 + index as f64;
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id: (*id).into(),
            label: format!("raw {id}"),
            color: Color(10, 20, 30, 255),
        });
    }
    scene
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut found = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        pending.extend(node.children.iter().rev().copied());
    }
    found
}
fn text(app: &App, class: &str) -> String {
    let id = nodes(app, class)[0];
    let tree = app.tree();
    match tree.node(id).unwrap().descriptor.view().props {
        Props::Text(p) => p.content.clone(),
        Props::Button(p) => p.label.clone(),
        _ => panic!("not text"),
    }
}
fn options() -> TooltipOptions {
    TooltipOptions {
        heading: Some("Details <safe>".into()),
        value_format: ValueFormat::Fixed(1),
        lines: vec![
            vec![
                TooltipPart::Series,
                TooltipPart::Text(" = ".into()),
                TooltipPart::Value,
            ],
            vec![
                TooltipPart::Id,
                TooltipPart::Text(" / ".into()),
                TooltipPart::Label,
            ],
        ],
        series_labels: BTreeMap::from([("alpha".into(), "Series A".into())]),
        ..Default::default()
    }
}

#[test]
fn pure_slice_templates_format_current_values_and_multiline_safe_text() {
    let s = scene();
    let opts = options();
    assert_eq!(
        tooltip_text(&opts, &[&s.marks[1], &s.marks[0]]).unwrap(),
        "Details <safe>\nbeta = 2.3\nbeta / beta value\nSeries A = 1.3\nalpha / alpha value"
    );
    assert_eq!(tooltip_text(&opts, &[]).unwrap(), "");
    let mut limited = opts.clone();
    limited.max_rows = 1;
    limited.overflow_text = Some("more <rows>".into());
    assert_eq!(
        tooltip_text(&limited, &[&s.marks[1], &s.marks[0]]).unwrap(),
        "Details <safe>\nbeta = 2.3\nbeta / beta value\nmore <rows>"
    );
    limited.overflow_text = None;
    assert!(
        !tooltip_text(&limited, &[&s.marks[1], &s.marks[0]])
            .unwrap()
            .contains("more")
    );
}

#[test]
fn nullable_slice_values_have_explicit_distinct_policies() {
    let mut opts = TooltipOptions {
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    let data = [
        TooltipDatum {
            id: "a",
            label: "A",
            series: "s",
            value: Some(1.25),
        },
        TooltipDatum {
            id: "b",
            label: "B",
            series: "s",
            value: None,
        },
    ];
    assert_eq!(tooltip_data_text(&opts, &data).unwrap(), "");
    opts.missing = MissingValue::OmitRow;
    assert_eq!(tooltip_data_text(&opts, &data).unwrap(), "A: 1.3");
    opts.missing = MissingValue::Placeholder("missing <value>".into());
    assert_eq!(
        tooltip_data_text(&opts, &data).unwrap(),
        "A: 1.3\nB: missing <value>"
    );
    assert!(
        tooltip_data_text(
            &opts,
            &[TooltipDatum {
                value: Some(f64::NAN),
                ..data[0]
            }]
        )
        .is_err()
    );
    opts.max_rows = 1;
    opts.missing = MissingValue::HideTooltip;
    // Missing rows do not silently become valid when a display row limit cuts in.
    assert_eq!(tooltip_data_text(&opts, &data).unwrap(), "");
}

#[test]
fn legend_layout_symbol_and_text_options_reach_real_glory_nodes() {
    let app = App::memory();
    let opts = LegendOptions {
        direction: LegendDirection::Vertical,
        wrap: false,
        gap: 9.,
        item_gap: 3.,
        align: PresentationAlign::End,
        item_width: Some(180.),
        symbol: LegendSymbol::Circle,
        size: 14.,
        symbol_color: Some(Color(220, 30, 40, 255)),
        text: PresentationText {
            color: Some(Color(1, 2, 3, 255)),
            size: 17.,
            weight: 650.,
            line_height: 1.8,
            font: FontOptions::default(),
        },
        series_labels: BTreeMap::from([("alpha".into(), "Friendly <alpha>".into())]),
        label_prefix: "[".into(),
        label_suffix: "]".into(),
    };
    app.mount(chart(scene()).legend_options(opts)).unwrap();
    let tree = app.tree();
    let legend = tree
        .node(nodes(&app, "kartis-legend")[0])
        .unwrap()
        .descriptor
        .view();
    assert_eq!(legend.style.flex_direction, Some(FlexDirection::Column));
    assert_eq!(legend.style.flex_wrap, Some(FlexWrap::NoWrap));
    assert_eq!(legend.style.gap, Some(9.));
    assert_eq!(legend.style.justify, Some(Justify::Start));
    assert_eq!(legend.style.align, Some(glory::style::Align::End));
    let item = tree
        .node(nodes(&app, "kartis-legend-item")[0])
        .unwrap()
        .descriptor
        .view();
    assert_eq!(item.style.width, Some(Length::Px(180.)));
    assert_eq!(item.style.gap, Some(3.));
    assert_eq!(item.style.font_size, Some(17.));
    assert_eq!(item.style.font_weight, Some(650.));
    assert!((item.style.line_height.unwrap() - 30.6).abs() < 1e-9);
    assert_eq!(item.style.color, Some(Color(1, 2, 3, 255)));
    let symbol = tree
        .node(nodes(&app, "kartis-legend-symbol")[0])
        .unwrap()
        .descriptor
        .view();
    assert_eq!(symbol.style.width, Some(Length::Px(14.)));
    assert_eq!(symbol.style.radius, Some(7.));
    assert_eq!(symbol.style.background, Some(Color(220, 30, 40, 255)));
    assert_eq!(text(&app, "kartis-legend-item"), "Hide [Friendly <alpha>]");
    let first = nodes(&app, "kartis-legend-item")[0];
    drop(tree);
    app.dispatch(first, Event::Activate).unwrap();
    assert_eq!(text(&app, "kartis-legend-item"), "Show [Friendly <alpha>]");
    assert_eq!(
        app.tree()
            .node(nodes(&app, "kartis-legend-symbol")[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .opacity,
        Some(0.35)
    );
    for (symbol, height, display) in [
        (LegendSymbol::Line, 2., Display::InlineFlex),
        (LegendSymbol::Square, 12., Display::InlineFlex),
        (LegendSymbol::None, 12., Display::None),
    ] {
        let style = legend_symbol_style(
            &LegendOptions {
                symbol,
                ..Default::default()
            },
            Color(1, 2, 3, 255),
            false,
        );
        assert_eq!(style.height, Some(Length::Px(height)));
        assert_eq!(style.display, Some(display));
    }
}

#[test]
fn controlled_slice_resolves_current_visible_data_and_retains_valid_inputs() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let ids = app.scope().cage(vec![
        "beta".to_owned(),
        "alpha".to_owned(),
        "stale".to_owned(),
    ]);
    let hidden = app.scope().cage(BTreeSet::new());
    let rejects = Rc::new(Cell::new(0));
    let r = rejects.clone();
    app.mount(
        chart(source)
            .tooltip_marks(ids)
            .hidden(hidden)
            .tooltip_options(options())
            .on_reject(move |_| r.set(r.get() + 1)),
    )
    .unwrap();
    assert!(text(&app, "kartis-tooltip").starts_with("Details <safe>\nbeta = 2.3"));
    source.revise(|s| s.marks[0].value = 8.25);
    app.flush().unwrap();
    assert!(text(&app, "kartis-tooltip").contains("Series A = 8.3"));
    hidden.revise(|h| {
        h.insert("beta".into());
    });
    app.flush().unwrap();
    assert_eq!(
        text(&app, "kartis-tooltip"),
        "Details <safe>\nSeries A = 8.3\nalpha / alpha value"
    );
    ids.revise(|ids| *ids = vec!["alpha".into(), "alpha".into()]);
    app.flush().unwrap();
    assert_eq!(rejects.get(), 1);
    assert!(text(&app, "kartis-tooltip").contains("8.3"));
    ids.revise(|ids| *ids = vec!["alpha".into()]);
    source.revise(|s| s.marks.remove(0));
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "");
    assert_eq!(
        app.tree()
            .node(nodes(&app, "kartis-tooltip")[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    app.clear().unwrap();
    ids.revise(Vec::clear);
    app.flush().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

#[test]
fn tooltip_style_and_plain_text_survive_ssr_without_html_injection() {
    let mut s = scene();
    s.marks[0].label = "<script>alert(1)</script>".into();
    let opts = TooltipOptions {
        heading: Some("<b>Summary</b>".into()),
        padding: 8.,
        background: Some(Color(240, 241, 242, 255)),
        text: PresentationText {
            size: 15.,
            ..Default::default()
        },
        ..Default::default()
    };
    let app = App::memory();
    app.mount(
        chart(s.clone())
            .tooltip_marks(vec!["alpha".into()])
            .tooltip_options(opts.clone()),
    )
    .unwrap();
    let tree = app.tree();
    let style = tree
        .node(nodes(&app, "kartis-tooltip")[0])
        .unwrap()
        .descriptor
        .view()
        .style;
    assert_eq!(style.white_space, Some(WhiteSpace::PreWrap));
    assert_eq!(style.padding, Some(glory::style::Edges::all(8.)));
    assert_eq!(style.background, Some(Color(240, 241, 242, 255)));
    assert_eq!(style.font_size, Some(15.));
    let html = glory::ssr::render(
        chart(s)
            .tooltip_marks(vec!["alpha".into()])
            .tooltip_options(opts)
            .legend_options(LegendOptions {
                label_prefix: "<i>".into(),
                label_suffix: "</i>".into(),
                ..Default::default()
            }),
    )
    .unwrap();
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<b>Summary</b>"));
    assert!(html.contains("&lt;b&gt;Summary&lt;/b&gt;"));
    assert!(html.contains("&lt;i&gt;raw alpha&lt;/i&gt;"));
    assert!(html.contains("kartis-legend-symbol"));
}

#[test]
fn formatting_error_hides_tooltip_and_recovers_without_stale_data() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured = errors.clone();
    app.mount(
        chart(source)
            .tooltip_marks(vec!["alpha".into()])
            .tooltip_options(TooltipOptions {
                value_format: ValueFormat::Percent(1),
                ..Default::default()
            })
            .on_reject(move |e| captured.borrow_mut().push(e.0)),
    )
    .unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "alpha value: 125.0%");
    source.revise(|s| s.marks[0].value = f64::MAX);
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "");
    assert_eq!(errors.borrow().len(), 1);
    source.revise(|s| s.marks[0].value = 0.5);
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "alpha value: 50.0%");
}

#[test]
fn formatting_error_callback_can_dispose_the_widget_and_cleans_new_effects() {
    let app = Rc::new(App::memory());
    let source = app.scope().cage(scene());
    let weak = Rc::downgrade(&app);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    app.mount(
        chart(source)
            .tooltip_marks(vec!["alpha".into()])
            .tooltip_options(TooltipOptions {
                value_format: ValueFormat::Percent(1),
                ..Default::default()
            })
            .on_reject(move |_| {
                observed.set(observed.get() + 1);
                // clear retires the owner immediately; Glory rejects only its
                // nested flush while the enclosing structural effect runs.
                assert!(matches!(
                    weak.upgrade().unwrap().clear(),
                    Err(glory::ui::Error::Access(
                        glory::core::AccessError::ReentrantFlush
                    ))
                ));
            }),
    )
    .unwrap();
    source.revise(|s| s.marks[0].value = f64::MAX);
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
    source.revise(|s| s.marks[0].value = 0.5);
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
}

#[test]
fn invalid_options_and_foreign_slice_sources_reject_before_mount() {
    let app = App::memory();
    let other = App::memory();
    let foreign = other.scope().cage(vec!["alpha".to_owned()]);
    assert!(app.mount(chart(scene()).tooltip_marks(foreign)).is_err());
    assert_eq!(app.stats().nodes, 0);
    assert!(
        app.mount(chart(scene()).legend_options(LegendOptions {
            gap: f64::NAN,
            ..Default::default()
        }))
        .is_err()
    );
    assert_eq!(app.stats().nodes, 0);
    assert!(
        app.mount(chart(scene()).tooltip_options(TooltipOptions {
            max_rows: 0,
            ..Default::default()
        }))
        .is_err()
    );
    assert_eq!(app.stats().nodes, 0);
    assert!(
        TooltipOptions {
            lines: vec![vec![TooltipPart::Text("x".repeat(1025))]],
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        TooltipOptions {
            text: PresentationText {
                size: f64::INFINITY,
                ..Default::default()
            },
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        legend_label(
            &scene().legend[0],
            &LegendOptions {
                label_prefix: "x".repeat(1024),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn dynamic_legend_label_overflow_rejects_before_any_view_or_datum_changes() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let errors = Rc::new(Cell::new(0));
    let observed = errors.clone();
    app.mount(
        chart(source)
            .tooltip_marks(vec!["alpha".into()])
            .legend_options(LegendOptions {
                label_prefix: "Series ".into(),
                ..Default::default()
            })
            .on_reject(move |_| observed.set(observed.get() + 1)),
    )
    .unwrap();
    let legend = nodes(&app, "kartis-legend-item")[0];
    source.revise(|s| {
        s.legend[0].label = "x".repeat(1024);
        s.marks[0].value = 99.;
    });
    app.flush().unwrap();
    assert_eq!(errors.get(), 1);
    assert_eq!(nodes(&app, "kartis-legend-item")[0], legend);
    assert_eq!(text(&app, "kartis-legend-item"), "Hide Series raw alpha");
    assert_eq!(text(&app, "kartis-tooltip"), "alpha value: 1.25");
    source.revise(|s| s.legend[0].label = "Updated".into());
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-legend-item"), "Hide Series Updated");
    assert_eq!(text(&app, "kartis-tooltip"), "alpha value: 99");
}
