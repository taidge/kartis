use std::sync::Arc;

use glory::render::GraphicStroke;
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 260.,
        ..Default::default()
    }
}
fn simulation() -> ForceSimulation {
    let mut nodes = vec![
        NetworkNode::new("a"),
        NetworkNode::new("empty"),
        NetworkNode::new("other"),
    ];
    nodes[1].label.clear();
    let mut s = ForceSimulation::new(
        &nodes,
        &[NetworkLink::new("a", "other", 2.)],
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    s.set_position("a", [-40., -0.]).unwrap();
    s.set_position("empty", [30., 20.]).unwrap();
    s.set_position("other", [90., -30.]).unwrap();
    s
}
fn placement(_: &NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-16., -10.],
        measured: TextSize {
            width: 32.,
            height: 20.,
        },
    }))
}
#[test]
fn all_actual_nodes_share_current_force_snapshot_and_resolve_unlabeled_path_origin() {
    let s = simulation();
    for (glyph, size) in [
        (GlyphShape::Filled(FilledGlyph::Triangle), 80.),
        (GlyphShape::Filled(FilledGlyph::Diamond), 0.),
        (GlyphShape::Stroked(StrokedGlyph::Plus), 10.),
    ] {
        let g = NetworkGlyphOptions {
            nodes: [(
                "empty".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: glyph,
                        size,
                        rotation: 0.3,
                    },
                    fill: None,
                    stroke: Some(GraphicStroke {
                        color: Color(40, 90, 180, 255),
                        width: 2.,
                        cap: glory::render::StrokeCap::Round,
                    }),
                },
            )]
            .into(),
        };
        let l = s.scene_with_glyphs(&config(), &g).unwrap();
        let old = l.scene().clone();
        let canonical = l.interaction_binding().sites.len();
        let packet = network_node_widgets(&l, &s, 7, placement).unwrap();
        assert_eq!(packet.data.len(), 3);
        assert_eq!(l.scene(), &old);
        assert_eq!(network_widget_frame(&l).unwrap().scene, old);
        assert_eq!(
            network_widget_frame(&l)
                .unwrap()
                .interaction
                .unwrap()
                .sites
                .len(),
            canonical
        );
        let first = packet.data.values().next().unwrap();
        for (id, d) in &packet.data {
            assert!(Arc::ptr_eq(&first.source, &d.source));
            assert_eq!(&d.source[d.source_index].id, &d.site.node_id);
            let slot = packet
                .layout
                .slots
                .iter()
                .find(|v| &v.mark_id == id)
                .unwrap();
            assert_eq!(slot.revision, 7);
            let PlotWidgetPlacement::Rect(rect) = slot.placement else {
                panic!("explicit node origin not resolved")
            };
            assert_eq!(
                [rect.x, rect.y],
                [d.site.position[0] - 16., d.site.position[1] - 10.]
            );
            assert_eq!(d.ticks, s.total_ticks());
        }
    }
}
#[test]
fn full_snapshot_bits_and_simulation_only_changes_are_observable_with_equal_scene() {
    let mut a = simulation();
    let la = a.scene_with_glyphs(&config(), &Default::default()).unwrap();
    let pa = network_node_widgets(&la, &a, 0, placement).unwrap();
    a.set_velocity("other", [-0., 0.]).unwrap();
    a.set_position("a", [-40., 0.]).unwrap();
    let lb = a.scene_with_glyphs(&config(), &Default::default()).unwrap();
    assert_eq!(la.scene(), lb.scene());
    let pb = network_node_widgets(&lb, &a, 0, placement).unwrap();
    assert_eq!(pa.layout, pb.layout);
    assert_ne!(pa, pb);
    let mut clone = pb.data.values().next().unwrap().clone();
    clone.source = Arc::new(clone.source.as_ref().clone());
    assert_eq!(&clone, pb.data.values().next().unwrap());
    let calls = std::cell::Cell::new(0);
    assert!(
        network_node_widgets(&la, &a, 0, |ctx| {
            calls.set(calls.get() + 1);
            placement(ctx)
        })
        .is_err()
    );
    assert_eq!(
        calls.get(),
        0,
        "raw world-position mismatch precedes callbacks"
    );
}
#[test]
fn every_source_conflict_precedes_callbacks_and_selection_errors_return_no_packet() {
    let s = simulation();
    let l = s.scene_with_glyphs(&config(), &Default::default()).unwrap();
    let mut moved = s.clone();
    moved.set_position("other", [95., -30.]).unwrap();
    let mut nodes = vec![
        NetworkNode::new("a"),
        NetworkNode::new("empty"),
        NetworkNode::new("other"),
    ];
    nodes[2].label = "changed".into();
    let renamed = ForceSimulation::new(&nodes, &[], Default::default()).unwrap();
    for other in [moved, renamed] {
        let calls = std::cell::Cell::new(0);
        assert!(
            network_node_widgets(&l, &other, 0, |ctx| {
                calls.set(calls.get() + 1);
                placement(ctx)
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    let mut calls = 0;
    let packet = network_node_widgets(&l, &s, 0, |c| {
        calls += 1;
        Ok((c.node.id == "empty").then_some({
            PlotWidgetPlacement::Rect(LayoutRect {
                x: 20.,
                y: 20.,
                width: 30.,
                height: 20.,
            })
        }))
    })
    .unwrap();
    assert_eq!(calls, 3);
    assert_eq!(packet.data.len(), 1);
    let mut calls = 0;
    assert!(
        network_node_widgets(&l, &s, 0, |c| {
            calls += 1;
            if calls == 3 {
                Err(ChartError("placement failed".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 3);
}
#[test]
fn empty_scene_and_callback_selection_budget_keep_normal_network_contract() {
    let s = ForceSimulation::new(&[], &[], Default::default()).unwrap();
    let l = s.scene_with_glyphs(&config(), &Default::default()).unwrap();
    let p = network_node_widgets(&l, &s, 0, |_| panic!("no node")).unwrap();
    assert!(p.data.is_empty());
    assert_eq!(p.layout.plot, None);
    let nodes: Vec<_> = (0..129)
        .map(|i| NetworkNode::new(format!("n{i}")))
        .collect();
    let s = ForceSimulation::new(&nodes, &[], Default::default()).unwrap();
    let l = s.scene_with_glyphs(&config(), &Default::default()).unwrap();
    let mut n = 0;
    assert!(
        network_node_widgets(&l, &s, 0, |c| {
            n += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(n, 129);
    let p = network_node_widgets(&l, &s, 0, |c| {
        if c.source_index < 128 {
            placement(c)
        } else {
            Ok(None)
        }
    })
    .unwrap();
    assert_eq!(p.data.len(), 128);
}
