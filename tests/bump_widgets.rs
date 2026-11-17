use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use glory::App;
use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 260.,
        margin: Margin {
            left: 30.,
            right: 20.,
            top: 24.,
            bottom: 26.,
        },
        ..Default::default()
    }
}

fn options() -> BumpLayoutOptions {
    BumpLayoutOptions {
        axes: BumpAxes::none(),
        ..Default::default()
    }
}

fn source() -> Vec<BumpSeries> {
    let mut first = BumpSeries::new(
        "north",
        vec![
            BumpDatum::new(-0., -0.),
            BumpDatum::new("0", 2.),
            BumpDatum::gap("gap"),
            BumpDatum::new("end", -1.),
        ],
    );
    first.label = "North original".into();
    vec![
        first,
        BumpSeries::new(
            "south",
            vec![BumpDatum::new(-0., 3.), BumpDatum::new("0", 1.)],
        ),
    ]
}

fn placement(_: &BumpPointWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-8., -4.],
        measured: TextSize {
            width: 16.,
            height: 8.,
        },
    }))
}

#[test]
fn complete_original_series_are_borrowed_then_shared_and_scene_remains_exact() {
    let mut layout = bump_layout(&config(), &source(), &options()).unwrap();
    let before = layout.clone();
    let mut order = vec![];
    let packet = bump_point_widgets(&layout, 7, |c| {
        assert!(std::ptr::eq(c.layout, &layout));
        assert!(std::ptr::eq(c.series, &c.computed.source));
        assert!(std::ptr::eq(c.datum, &c.series.data[c.point.source_index]));
        assert_eq!(Some(c.mark.id.as_str()), c.point.mark_id.as_deref());
        order.push((c.computed.source_index, c.point.source_index));
        placement(c)
    })
    .unwrap();
    assert_eq!(order, [(0, 0), (0, 1), (0, 3), (1, 0), (1, 1)]);
    assert_eq!(layout, before);
    assert_eq!(packet.layout.plot, Some(layout.plot));
    assert!(packet.layout.slots.iter().all(|s| s.revision == 7));
    let rows = packet
        .data
        .values()
        .filter(|d| d.series_index == 0)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|d| Arc::ptr_eq(&rows[0].series, &d.series)));
    assert_eq!(rows[0].series.data.len(), 4);
    assert_eq!(rows[0].series.data[2].rank, None);
    assert_eq!(
        rows[0].series.data[0].rank.unwrap().to_bits(),
        (-0_f64).to_bits()
    );
    assert!(
        matches!(rows[0].series.data[0].x, CategoryKey::Number(n) if n.to_bits()==(-0_f64).to_bits())
    );
    assert!(matches!(rows[0].series.data[1].x, CategoryKey::String(ref s) if s=="0"));
    let frame = bump_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, layout.scene);
    assert_eq!(frame.plot, Some(layout.plot));
    assert_eq!(
        frame.interaction.as_ref().unwrap().picking_profile,
        PickingProfile::PaintedGeometry
    );
    layout.series[0].source.data[0].rank = Some(99.);
    assert_eq!(
        rows[0].series.data[0].rank.unwrap().to_bits(),
        (-0_f64).to_bits()
    );
    let mut detached = rows[0].series.clone();
    Arc::make_mut(&mut detached).label = "changed".into();
    assert_eq!(rows[0].series.label, "North original");
}

#[test]
fn reverse_round_shoulder_gap_hidden_and_zero_actual_point_profiles_stay_eligible() {
    for reverse in [false, true] {
        for round in [false, true] {
            for interpolation in [BumpInterpolation::Smooth, BumpInterpolation::Linear] {
                for padding in [0., 0.6, 4.] {
                    let mut o = options();
                    o.x_scale.reverse = reverse;
                    o.rank_scale.reverse = !reverse;
                    o.x_scale.round = round;
                    o.rank_scale.round = round;
                    o.x_padding = padding;
                    o.interpolation = interpolation;
                    o.style.point.size = 0.;
                    o.style.opacity = 0.;
                    o.hidden.insert("south".into());
                    let layout = bump_layout(&config(), &source(), &o).unwrap();
                    let packet = bump_point_widgets(&layout, 0, placement).unwrap();
                    assert_eq!(packet.layout.slots.len(), 3);
                    assert!(
                        packet
                            .data
                            .values()
                            .all(|d| d.series_index == 0 && d.point.rank.is_some())
                    );
                    assert!(packet.layout.slots.iter().all(|s| {
                        layout
                            .scene
                            .marks
                            .iter()
                            .find(|m| m.id == s.mark_id)
                            .unwrap()
                            .opacity
                            == 0.
                    }));
                    assert_eq!(bump_widget_frame(&layout).unwrap().scene, layout.scene);
                }
            }
        }
    }
    for data in [
        vec![],
        vec![BumpSeries::new("empty", vec![])],
        vec![BumpSeries::new("gaps", vec![BumpDatum::gap("only")])],
    ] {
        let layout = bump_layout(&config(), &data, &options()).unwrap();
        let packet =
            bump_point_widgets(&layout, 3, |_| panic!("empty/gap has no actual role")).unwrap();
        assert!(packet.layout.slots.is_empty());
        assert_eq!(packet.layout.plot, Some(ClipRect::plot(&config())));
        assert_eq!(bump_widget_frame(&layout).unwrap().plot, packet.layout.plot);
    }
}

#[test]
fn all_source_and_role_conflicts_reject_before_any_callback() {
    type Mutation = (&'static str, fn(&mut BumpLayout));
    let mutations: &[Mutation] = &[
        ("source rank", |l| {
            l.series[0].source.data[0].rank = Some(5.)
        }),
        ("source x", |l| {
            l.series[0].source.data[0].x = "different".into()
        }),
        ("point x variant", |l| {
            l.series[0].points[0].raw_x = "0".into()
        }),
        ("point raw zero bits", |l| {
            l.series[0].points[0].raw_x = 0_f64.into()
        }),
        ("point rank zero bits", |l| {
            l.series[0].points[0].rank = Some(0.)
        }),
        ("point source index", |l| {
            l.series[0].points[0].source_index = 1
        }),
        ("series source index", |l| l.series[0].source_index = 1),
        ("source label", |l| {
            l.series[0].source.label = "altered".into()
        }),
        ("duplicate series", |l| {
            l.series[1].source.id = "north".into()
        }),
        ("point position", |l| {
            l.series[0].points[0].position.as_mut().unwrap()[0] += 1.
        }),
        ("source eligibility", |l| l.series[0].hidden = true),
        ("late point", |l| {
            l.series[1].points[1].rank = Some(f64::NAN)
        }),
        ("missing mark identity", |l| {
            l.series[0].points[0].mark_id = None
        }),
        ("point points to line", |l| {
            l.series[0].points[0].mark_id = Some(l.series[0].line_mark_ids[0].clone())
        }),
        ("point typed label", |l| {
            l.interaction.sites[0].label = "other".into()
        }),
        ("point typed value", |l| {
            l.interaction.sites[0].value = Some(1.)
        }),
        ("point typed position", |l| {
            l.interaction.sites[0].position[0] += 1.
        }),
        ("extra site", |l| {
            l.interaction.sites.push(l.interaction.sites[0].clone())
        }),
        ("missing site", |l| {
            l.interaction.sites.pop();
        }),
        ("shoulder count", |l| {
            l.series[0].line_points.pop();
        }),
        ("shoulder middle", |l| {
            l.series[0].line_points[1].as_mut().unwrap()[0] += 1.
        }),
        ("gap separator", |l| {
            l.series[0].line_points[6] = Some([0., 0.])
        }),
        ("line run id", |l| l.series[0].line_mark_ids[0].push('x')),
        ("missing line metadata", |l| {
            l.series[0].line_mark_ids.clear()
        }),
        ("line shape", |l| {
            let id = &l.series[0].line_mark_ids[0];
            l.scene
                .marks
                .iter_mut()
                .find(|m| &m.id == id)
                .unwrap()
                .geometry = Geometry::Circle {
                x: 0.,
                y: 0.,
                radius: 2.,
            };
        }),
        ("point value", |l| {
            let id = l.series[0].points[0].mark_id.as_ref().unwrap();
            l.scene
                .marks
                .iter_mut()
                .find(|m| &m.id == id)
                .unwrap()
                .value = 123.;
        }),
        ("point clip", |l| {
            let id = l.series[0].points[0].mark_id.as_ref().unwrap();
            l.scene.marks.iter_mut().find(|m| &m.id == id).unwrap().clip = None;
        }),
        ("orphan point", |l| {
            let mut m = l.scene.marks.last().unwrap().clone();
            m.id = "bump-point:fake".into();
            l.scene.marks.push(m);
        }),
        ("plot", |l| l.plot.x += 1.),
        ("scale domain", |l| {
            l.x_scale = PointScale::new(vec!["fake".into()], l.x_scale.range(), &Default::default())
                .unwrap()
        }),
        ("scale position", |l| {
            l.rank_scale = PointScale::new(
                l.rank_scale.domain().to_vec(),
                l.rank_scale.range(),
                &PointScaleOptions {
                    padding: 5.,
                    ..Default::default()
                },
            )
            .unwrap()
        }),
    ];
    for (name, mutate) in mutations {
        let mut layout = bump_layout(&config(), &source(), &options()).unwrap();
        mutate(&mut layout);
        let mut calls = 0;
        assert!(
            bump_point_widgets(&layout, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err(),
            "accepted {name}"
        );
        assert_eq!(calls, 0, "callback escaped preflight: {name}");
        assert!(bump_widget_frame(&layout).is_err(), "frame accepted {name}");
    }
}

#[test]
fn selected_budget_skips_and_late_callback_error_return_no_partial_packet() {
    let data = vec![BumpSeries::new(
        "many",
        (0..129)
            .map(|i| BumpDatum::new(i as f64, i as f64))
            .collect(),
    )];
    let layout = bump_layout(&config(), &data, &options()).unwrap();
    let mut called = 0;
    assert!(
        bump_point_widgets(&layout, 0, |c| {
            called += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(called, 129);
    let packet = bump_point_widgets(&layout, 8, |c| {
        if c.point.source_index == 128 {
            Ok(None)
        } else {
            placement(c)
        }
    })
    .unwrap();
    assert_eq!(packet.layout.slots.len(), 128);
    let before = layout.clone();
    let mut called = vec![];
    assert!(
        bump_point_widgets(&layout, 0, |c| {
            called.push(c.point.source_index);
            if c.point.source_index == 2 {
                Err(ChartError("placement refused".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(called, [0, 1, 2]);
    assert_eq!(layout, before);
    assert!(
        bump_point_widgets(&layout, 0, |_| Ok(None))
            .unwrap()
            .data
            .is_empty()
    );
}

#[test]
fn coherent_public_source_edits_and_representative_colors_do_not_freeze_paint() {
    let mut layout = bump_layout(&config(), &source(), &options()).unwrap();
    layout.series[0].source.label = "Edited current series".into();
    layout.series[0].color = Color(9, 8, 7, 255);
    for p in &layout.series[0].points {
        if let Some(id) = &p.mark_id {
            let label = format!(
                "Edited current series · {}",
                match &p.raw_x {
                    CategoryKey::Number(v) => ValueFormat::Auto.format(*v).unwrap(),
                    CategoryKey::String(s) => s.clone(),
                }
            );
            let mark = layout.scene.marks.iter_mut().find(|m| &m.id == id).unwrap();
            mark.label = label.clone();
            mark.opacity = 0.;
            let site = layout
                .interaction
                .sites
                .iter_mut()
                .find(|s| &s.mark_id == id)
                .unwrap();
            site.label = label;
            site.color = Color(12, 34, 56, 255);
        }
    }
    let packet = bump_point_widgets(&layout, 0, placement).unwrap();
    assert!(packet.data.values().filter(|d| d.series_index==0).all(|d|d.color==Color(12,34,56,255)&&d.series.label=="Edited current series"));
    // Raw provenance is bit-exact; the compatibility Mark scalar remains numeric.
    layout.series[0].source.data[0] = BumpDatum::new(0., 0.);
    layout.series[0].points[0].raw_x = 0_f64.into();
    layout.series[0].points[0].rank = Some(0.);
    layout.interaction.sites[0].value = Some(0.);
    assert!(bump_point_widgets(&layout, 0, placement).is_ok());
}

#[test]
fn off_origin_path_role_mounts_at_original_anchor_with_owned_typed_data() {
    let layout = bump_layout_with_point_factory(&config(), &source(), &options(), |c| {
        let [x, y] = c.position;
        Ok(c.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 20., y - 3.)),
            C::Line(P::new(x + 30., y - 3.)),
            C::Line(P::new(x + 30., y + 3.)),
            C::Close,
        ])))
    })
    .unwrap();
    let packet = bump_point_widgets(&layout, 3, placement).unwrap();
    let frame = bump_widget_frame(&layout).unwrap();
    let original = frame.clone();
    let observed = Rc::new(RefCell::new(vec![]));
    let out = observed.clone();
    let app = App::memory();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .plot_widgets_with_data(packet, move |context| {
                let frame = context.frame;
                out.borrow_mut()
                    .push(frame.with(|f| (f.frame.rect, f.data.clone(), f.frame.mark.clone())));
                Ok(glory::ui::button().label("real Bump role"))
            }),
    )
    .unwrap();
    assert_eq!(observed.borrow().len(), 5);
    for (rect, data, mark) in observed.borrow().iter() {
        let p = data.point.position.unwrap();
        assert_eq!([rect.x, rect.y], [p[0] - 8., p[1] - 4.]);
        assert_eq!(
            original
                .interaction
                .as_ref()
                .unwrap()
                .anchor_for(mark)
                .unwrap(),
            Some(p)
        );
        assert_eq!(
            data.series.data[data.point.source_index].rank,
            data.point.rank
        );
    }
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    let mut forged = layout.clone();
    let id = forged.series[1].points[1].mark_id.as_ref().unwrap().clone();
    let mark = forged.scene.marks.iter().find(|m| m.id == id).unwrap();
    let p = forged.series[1].points[1].position.unwrap();
    forged.interaction.anchors.insert(
        id,
        InteractionAnchor::path_points(&mark.geometry, [p[0] + 20., p[1] - 3.], &[0]).unwrap(),
    );
    let mut calls = 0;
    assert!(
        bump_point_widgets(&forged, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(calls, 0);
}
