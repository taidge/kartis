//! Independent source/geometry qualification for actual Swarm Widget roles.
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 340.,
        height: 230.,
        margin: Margin {
            left: 28.,
            right: 22.,
            top: 24.,
            bottom: 26.,
        },
        ..Default::default()
    }
}
fn source() -> Vec<SwarmPoint> {
    let mut zero = SwarmPoint::new("zero", "A", 2.);
    zero.size = Some(0.);
    vec![
        SwarmPoint::new("third", "B", 4.),
        SwarmPoint::new("selected", "A", -0.),
        SwarmPoint::new("hidden", "H", 3.),
        zero,
        SwarmPoint::new("culled", "B", 50.),
    ]
}
fn options() -> SwarmLayoutOptions {
    SwarmLayoutOptions {
        orientation: SwarmOrientation::Horizontal,
        value_fit: SwarmValueFit::Clip,
        value_domain: Some([-1., 5.]),
        value_scale: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        value_axis: None,
        group_axis: None,
        groups: Some(vec![
            SwarmGroup::new("A"),
            SwarmGroup::new("H"),
            SwarmGroup::new("B"),
        ]),
        hidden: ["H".into()].into(),
        ..Default::default()
    }
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-4., -3.],
        measured: TextSize {
            width: 8.,
            height: 6.,
        },
    }
}
fn build() -> SwarmLayout {
    swarm_layout(&config(), &source(), &options()).unwrap()
}

#[test]
fn original_order_packed_order_full_source_and_all_actual_zero_roles_are_distinct() {
    let raw = source();
    let layout = swarm_layout_with_node_factory(&config(), &raw, &options(), |c| {
        let mut n = c.inherit(c.geometry.clone());
        if c.datum.id == "zero" {
            n.opacity = 0.;
        }
        Ok(n)
    })
    .unwrap();
    let before = layout.scene.clone();
    let mut calls = vec![];
    let packet = swarm_node_widgets(&layout, &raw, 7, |c| {
        assert!(std::ptr::eq(c.datum, &raw[c.source_index]));
        assert!(std::ptr::eq(c.site, &layout.sites[c.site_index]));
        assert!(std::ptr::eq(c.mark, &layout.scene.marks[c.site.mark_index]));
        assert_eq!(c.datum.id, c.site.node_id);
        assert_eq!(c.datum.group, c.group.id);
        assert_eq!(c.datum.value.to_bits(), c.site.raw_value.to_bits());
        calls.push((c.source_index, c.site_index, c.datum.id.clone()));
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(
        calls,
        [
            (1, 0, "selected".into()),
            (3, 1, "zero".into()),
            (0, 2, "third".into())
        ]
    );
    assert_eq!(layout.metadata[2].site_index, None);
    assert!(layout.metadata[2].hidden);
    assert_eq!(layout.metadata[4].site_index, None);
    assert!(!layout.metadata[4].hidden);
    assert_eq!(packet.data.len(), 3);
    let all = packet.data.values().collect::<Vec<_>>();
    assert!(
        all.windows(2)
            .all(|p| Arc::ptr_eq(&p[0].source, &p[1].source))
    );
    assert_eq!(all[0].source.as_slice(), raw.as_slice());
    assert_eq!(before, layout.scene);
    let zero = packet
        .data
        .values()
        .find(|d| d.site.node_id == "zero")
        .unwrap();
    assert_eq!(zero.site.size, 0.);
    assert_eq!(layout.scene.marks[zero.site.mark_index].opacity, 0.);
    let frame = swarm_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, layout.scene);
    assert_eq!(frame.plot, Some(layout.plot));
    assert_eq!(frame.interaction.unwrap().sites.len(), 3);
}

#[test]
fn all_source_and_late_public_conflicts_reject_before_any_callback() {
    type Mutation = (&'static str, fn(&mut SwarmLayout, &mut Vec<SwarmPoint>));
    let cases: &[Mutation] = &[
        ("late source value", |_, d| d[4].value = 49.),
        ("source id", |_, d| d[0].id = "invented".into()),
        ("source group", |_, d| d[0].group = "A".into()),
        ("source order", |_, d| d.swap(0, 1)),
        ("source count", |_, d| {
            d.pop();
        }),
        ("source sign", |_, d| d[1].value = 0.),
        ("late NaN source", |_, d| d[4].value = f64::NAN),
        ("source size", |_, d| d[3].size = Some(1.)),
        ("source label", |_, d| d[1].label = Some("forged".into())),
        ("meta source index", |l, _| l.metadata[4].source_index = 0),
        ("meta id", |l, _| l.metadata[4].node_id = "other".into()),
        ("meta raw", |l, _| l.metadata[1].raw_value = 0.),
        ("meta hidden", |l, _| l.metadata[2].hidden = false),
        ("meta site", |l, _| l.metadata[0].site_index = Some(0)),
        ("site source", |l, _| l.sites[0].source_index = 0),
        ("site raw", |l, _| l.sites[0].raw_value = 0.),
        ("site group", |l, _| l.sites[0].group_id = "B".into()),
        ("site mark index", |l, _| l.sites[0].mark_index = 999),
        ("site position", |l, _| l.sites[0].position[0] += 1.),
        ("site ink radius", |l, _| l.sites[0].ink_radius = -1.),
        ("site order", |l, _| l.sites.swap(0, 1)),
        ("group center", |l, _| l.groups[0].center += 1.),
        ("group band", |l, _| l.groups[0].band[0] = f64::NAN),
        ("group duplicate", |l, _| {
            l.groups[1].id = l.groups[0].id.clone()
        }),
        ("mark scalar", |l, _| {
            let i = l.sites[0].mark_index;
            l.scene.marks[i].value = 1.;
        }),
        ("mark label", |l, _| {
            let i = l.sites[0].mark_index;
            l.scene.marks[i].label = "fake".into();
        }),
        ("mark group", |l, _| {
            let i = l.sites[0].mark_index;
            l.scene.marks[i].series = "B".into();
        }),
        ("mark clip", |l, _| {
            let i = l.sites[0].mark_index;
            l.scene.marks[i].clip = None;
        }),
        ("mark anchor", |l, _| {
            let i = l.sites[0].mark_index;
            if let Geometry::Circle { x, .. } = &mut l.scene.marks[i].geometry {
                *x += 1.;
            }
        }),
        ("orphan mark", |l, _| {
            let mut m = l.scene.marks[l.sites[0].mark_index].clone();
            m.id = "swarm-node:orphan".into();
            l.scene.marks.push(m);
        }),
    ];
    for (name, mutate) in cases {
        let mut l = build();
        let mut d = source();
        mutate(&mut l, &mut d);
        let mut count = 0;
        let result = swarm_node_widgets(&l, &d, 7, |_| {
            count += 1;
            Ok(None)
        });
        assert!(result.is_err(), "accepted {name}");
        assert_eq!(count, 0, "callback before full validation: {name}");
    }
}

#[test]
fn coherent_label_color_and_paint_edits_are_current_data_not_immutable_snapshots() {
    let mut l = build();
    let mut d = source();
    d[1].label = Some(String::new());
    let s = &mut l.sites[0];
    s.label = format!(
        "{} · : {}",
        l.groups[s.group_index].label, s.formatted_value
    );
    s.color = Color(1, 2, 3, 255);
    let m = &mut l.scene.marks[s.mark_index];
    m.label = s.label.clone();
    m.fill = Some(Color(200, 30, 80, 255));
    let packet = swarm_node_widgets(&l, &d, 1, |_| Ok(Some(placement()))).unwrap();
    let data = packet
        .data
        .values()
        .find(|p| p.site.node_id == "selected")
        .unwrap();
    assert_eq!(data.source[data.source_index].label.as_deref(), Some(""));
    assert_eq!(data.site.color, Color(1, 2, 3, 255));
    assert_eq!(swarm_widget_frame(&l).unwrap().scene, l.scene);
}

#[test]
fn offset_path_anchor_remains_the_packed_datum_and_stale_binding_rejects_early() {
    let d = source();
    let mut l = swarm_layout_with_node_factory(&config(), &d, &options(), |c| {
        let [x, y] = c.position;
        Ok(c.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 12., y + 8.)),
            C::Line(P::new(x + 20., y + 8.)),
            C::Line(P::new(x + 20., y + 16.)),
            C::Line(P::new(x + 12., y + 16.)),
            C::Close,
        ])))
    })
    .unwrap();
    let frame = swarm_widget_frame(&l).unwrap();
    for s in &l.sites {
        assert_eq!(
            frame
                .interaction
                .as_ref()
                .unwrap()
                .anchor_for(&l.scene.marks[s.mark_index])
                .unwrap(),
            Some(s.position)
        );
    }
    let packet = swarm_node_widgets(&l, &d, 1, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(packet.data.len(), 3);
    let i = l.sites.last().unwrap().mark_index;
    if let Geometry::Path(path) = &mut l.scene.marks[i].geometry
        && let C::Move(p) = &mut path[0]
    {
        p.x += 3.;
    }
    let mut calls = 0;
    assert!(
        swarm_node_widgets(&l, &d, 1, |_| {
            calls += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    assert!(swarm_widget_frame(&l).is_err());
}

#[test]
fn full_source_signed_zero_equality_includes_unselected_hidden_siblings() {
    let mut a = source();
    a[2].value = -0.;
    a[2].size = Some(-0.);
    let mut b = a.clone();
    b[2].value = 0.;
    b[2].size = Some(0.);
    let l = swarm_layout(&config(), &a, &options()).unwrap();
    let next = swarm_layout(&config(), &b, &options()).unwrap();
    assert_eq!(l.scene, next.scene);
    let p = |l: &SwarmLayout, d: &[SwarmPoint]| {
        swarm_node_widgets(l, d, 0, |c| Ok((c.datum.id == "selected").then(placement))).unwrap()
    };
    let x = p(&l, &a);
    let y = p(&next, &b);
    assert_ne!(x, y);
    assert_eq!(x, p(&l, &a));
    assert_eq!(
        x.data.values().next().unwrap().source[2]
            .size
            .unwrap()
            .to_bits(),
        (-0_f64).to_bits()
    );
}

#[test]
fn bounded_selection_skip_and_late_callback_error_publish_no_partial_packet() {
    let d = (0..129)
        .map(|i| {
            let mut p = SwarmPoint::new(format!("n{i:03}"), "A", i as f64);
            p.size = Some(0.);
            p
        })
        .collect::<Vec<_>>();
    let o = SwarmLayoutOptions {
        orientation: SwarmOrientation::Horizontal,
        value_domain: Some([0., 128.]),
        spacing: 0.,
        value_axis: None,
        group_axis: None,
        ..Default::default()
    };
    let l = swarm_layout(&config(), &d, &o).unwrap();
    let mut n = 0;
    assert!(
        swarm_node_widgets(&l, &d, 0, |_| {
            n += 1;
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(n, 129);
    assert_eq!(
        swarm_node_widgets(&l, &d, 0, |c| Ok((c.source_index % 2 == 0).then(placement)))
            .unwrap()
            .data
            .len(),
        65
    );
    let mut n = 0;
    assert!(
        swarm_node_widgets(&l, &d, 0, |_| {
            n += 1;
            if n == 3 {
                Err(ChartError("stop placement".into()))
            } else {
                Ok(Some(placement()))
            }
        })
        .is_err()
    );
    assert_eq!(n, 3);
}
