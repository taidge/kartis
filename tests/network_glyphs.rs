use glory::render::{GraphicStroke, StrokeCap};
use kartis::{surface::*, *};
use std::collections::{BTreeMap, BTreeSet};

fn frame() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            left: 47.,
            right: 19.,
            top: 35.,
            bottom: 21.,
        },
        ..Default::default()
    }
}

fn simulation() -> ForceSimulation {
    let mut simulation = ForceSimulation::new(
        &[
            NetworkNode {
                id: "node:a".into(),
                label: "Node A".into(),
                radius: 3.,
            },
            NetworkNode {
                id: "node:b".into(),
                label: "Node B".into(),
                radius: 7.,
            },
            NetworkNode {
                id: "other".into(),
                label: "Other".into(),
                radius: 6.,
            },
        ],
        &[NetworkLink::new("node:a", "node:b", 9.)],
        ForceOptions {
            labels: true,
            ..Default::default()
        },
    )
    .unwrap();
    simulation.set_position("node:a", [-30., 8.]).unwrap();
    simulation.set_position("node:b", [50., -13.]).unwrap();
    simulation.set_position("other", [95., 34.]).unwrap();
    simulation
}

fn stroke(cap: StrokeCap, width: f64) -> GraphicStroke {
    GraphicStroke {
        color: Color(150, 38, 75, 255),
        width,
        cap,
    }
}

fn styles() -> NetworkGlyphOptions {
    NetworkGlyphOptions {
        nodes: BTreeMap::from([
            (
                "node:a".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Filled(FilledGlyph::Star),
                        size: 1600.,
                        rotation: 0.,
                    },
                    fill: Some(Color(20, 75, 150, 255)),
                    stroke: None,
                },
            ),
            (
                "node:b".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Stroked(StrokedGlyph::Triangle),
                        size: 1600.,
                        rotation: 0.31,
                    },
                    fill: None,
                    stroke: Some(stroke(StrokeCap::Round, 3.)),
                },
            ),
        ]),
    }
}

fn mapping(c: &ChartConfig, view: ZoomTransform) -> NetworkDragMapping {
    NetworkDragMapping::from_config(
        c,
        ClipRect {
            x: 100.,
            y: 40.,
            width: 630.,
            height: 165.,
        },
        view,
    )
    .unwrap()
}

fn close(actual: [f64; 2], expected: [f64; 2]) {
    assert!(
        (actual[0] - expected[0]).hypot(actual[1] - expected[1]) < 1e-10,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn glyph_frames_keep_force_radius_links_and_stable_ownership() {
    let c = frame();
    let mut simulation = simulation();
    let base = simulation.scene(&c).unwrap();
    let default = simulation
        .scene_with_glyphs(&c, &Default::default())
        .unwrap();
    assert_eq!(default.scene(), &base);
    let mut control = simulation.clone();
    let styled = simulation.scene_with_glyphs(&c, &styles()).unwrap();
    assert_eq!(styled.scene().marks[0], base.marks[0]);
    assert_eq!(styled.scene().marks[3], base.marks[3]);
    assert_eq!(styled.scene().labels[2], base.labels[2]);
    assert_eq!(styled.scene().label_series, base.label_series);
    styled
        .interaction_binding()
        .validate(styled.scene())
        .unwrap();
    for (i, site) in styled.sites().iter().enumerate() {
        let node = &simulation.nodes()[i];
        assert_eq!(site.physics_radius, node.radius);
        assert_eq!(site.world_position, node.position);
        assert_eq!(site.mark_id, format!("force-node:{}", node.id));
        assert_eq!(
            styled.scene().label_series[&site.label_index.unwrap()],
            node.id
        );
        assert!(styled.scene().labels[i].y <= site.bounds.y - 5.);
        assert_eq!(
            styled.interaction_binding().sites[i].position,
            site.position
        );
        assert_eq!(styled.interaction_binding().sites[i].value, None);
    }
    let hollow = &styled.sites()[1];
    assert_ne!(
        hollow.position,
        [
            hollow.bounds.x + hollow.bounds.width / 2.,
            hollow.bounds.y + hollow.bounds.height / 2.
        ]
    );
    assert!(matches!(
        styled.scene().marks[2].geometry,
        Geometry::Path(_)
    ));
    assert!(styled.scene().marks[2].fill.is_none());
    assert_eq!(
        styled.scene().marks[2].stroke,
        Some(stroke(StrokeCap::Round, 3.))
    );
    assert_eq!(simulation.step(6).unwrap(), control.step(6).unwrap());
    assert_eq!(simulation.nodes(), control.nodes());
    let moved = network_glyph_layout(&c, &simulation, &styles()).unwrap();
    assert_eq!(
        moved.scene().marks[0],
        simulation.scene(&c).unwrap().marks[0]
    );
    assert_eq!(moved.sites()[0].mark_id, styled.sites()[0].mark_id);
    assert_ne!(moved.sites()[0].position, styled.sites()[0].position);
}

#[test]
fn paint_contour_pick_respects_camera_holes_draw_order_and_hidden_nodes() {
    let c = frame();
    let mut simulation = simulation();
    let map = mapping(&c, ZoomTransform::new(1.6, -130., -65.).unwrap());
    let empty = BTreeSet::new();
    let layout = simulation.scene_with_glyphs(&c, &styles()).unwrap();
    let wing = [-30., -18.];
    let pointer = map.world_to_client(wing).unwrap();
    assert_eq!(
        layout
            .pick_node(pointer, &map, &empty)
            .unwrap()
            .unwrap()
            .node_id,
        "node:a"
    );
    let physics = NetworkDragController::new(simulation.clone(), Default::default()).unwrap();
    assert!(physics.pick(pointer, &map, &empty).unwrap().is_none());
    let hollow = map.world_to_client([50., -13.]).unwrap();
    assert!(layout.pick_node(hollow, &map, &empty).unwrap().is_none());
    assert!(layout.pick_node([0., 0.], &map, &empty).unwrap().is_none());
    let mut model = SurfaceModel::new_with_interaction(
        layout.scene().clone(),
        Default::default(),
        Some(layout.interaction_binding().clone()),
    )
    .unwrap();
    let site = &layout.sites()[1];
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (site.position[0], site.position[1]),
        })
        .unwrap();
    assert!(model.hovered().is_empty());
    simulation.set_position("other", wing).unwrap();
    let overlap = simulation.scene_with_glyphs(&c, &styles()).unwrap();
    assert_eq!(
        overlap
            .pick_node(pointer, &map, &empty)
            .unwrap()
            .unwrap()
            .node_id,
        "other"
    );
    let hidden = BTreeSet::from(["other".into()]);
    assert_eq!(
        overlap
            .pick_node(pointer, &map, &hidden)
            .unwrap()
            .unwrap()
            .node_id,
        "node:a"
    );

    let map = mapping(&c, ZoomTransform::default());
    let mut options = styles();
    options.nodes.get_mut("node:a").unwrap().glyph = GlyphOptions {
        shape: GlyphShape::Filled(FilledGlyph::Circle),
        size: std::f64::consts::PI * 30_f64.powi(2),
        rotation: 0.,
    };
    let left_edge = -map.world_offset[0];
    simulation
        .set_position("node:a", [left_edge - 10., 8.])
        .unwrap();
    let clipped = simulation.scene_with_glyphs(&c, &options).unwrap();
    let interior = map.world_to_client([left_edge + 3., 8.]).unwrap();
    let exterior = map.world_to_client([left_edge - 3., 8.]).unwrap();
    assert_eq!(
        clipped
            .pick_node(interior, &map, &empty)
            .unwrap()
            .unwrap()
            .node_id,
        "node:a"
    );
    assert!(clipped.pick_node(exterior, &map, &empty).unwrap().is_none());
}

#[test]
fn drag_uses_force_origin_and_keeps_offset_under_rebase_release_and_cancel() {
    let c = frame();
    let mut simulation = simulation();
    simulation.set_fixed("node:a", Some([-30., 8.])).unwrap();
    let mut drag = NetworkDragController::new(simulation, Default::default()).unwrap();
    let options = styles();
    let map = mapping(&c, ZoomTransform::new(1.6, -130., -65.).unwrap());
    let empty = BTreeSet::new();
    let client = map.world_to_client([-30., -18.]).unwrap();
    assert!(
        drag.begin_with_glyphs(&c, &options, 17, client, &map, &empty)
            .unwrap()
    );
    assert_eq!(drag.active().unwrap().node_id, "node:a");
    close(drag.active().unwrap().grab_offset, [0., 26.]);
    assert_eq!(
        drag.simulation().node("node:a").unwrap().position,
        [-30., 8.]
    );
    let next = map.world_to_client([-11., -13.]).unwrap();
    assert!(!drag.move_pointer(18, next, &map).unwrap());
    assert!(drag.move_pointer(17, next, &map).unwrap());
    close(
        drag.simulation().node("node:a").unwrap().position,
        [-11., 13.],
    );
    let resized = ChartConfig {
        width: 500.,
        height: 320.,
        ..c
    };
    let new_map = mapping(&resized, ZoomTransform::new(1.3, -80., -30.).unwrap());
    drag.rebase(&new_map).unwrap();
    assert!(drag.move_pointer(17, next, &new_map).unwrap());
    close(
        drag.simulation().node("node:a").unwrap().position,
        [-11., 13.],
    );
    assert!(drag.cancel().unwrap());
    assert_eq!(
        drag.simulation().node("node:a").unwrap().fixed,
        Some([-30., 8.])
    );
    assert!(
        drag.begin_with_glyphs(
            &resized,
            &options,
            19,
            new_map.world_to_client([-30., -18.]).unwrap(),
            &new_map,
            &empty
        )
        .unwrap()
    );
    assert!(
        drag.end(19, new_map.world_to_client([-25., -16.]).unwrap(), &new_map)
            .unwrap()
    );
    close(
        drag.simulation().node("node:a").unwrap().fixed.unwrap(),
        [-25., 10.],
    );
}

#[test]
fn relation_selection_keeps_actual_paths_and_canonical_ownership() {
    let c = frame();
    let simulation = simulation();
    let styles = styles();
    let glyph_frame = simulation.scene_with_glyphs(&c, &styles).unwrap();
    let edge_ids = ["explicit:edge".into()];
    let mut relation =
        network_glyph_relation_layout(&c, &simulation, &styles, &edge_ids, &Default::default())
            .unwrap();
    assert_eq!(
        relation.graph().node("node:a").unwrap().mark_ids,
        ["force-node:node:a"]
    );
    assert_eq!(relation.graph().node("node:b").unwrap().label_indices, [1]);
    assert_eq!(
        relation.graph().edge("explicit:edge").unwrap().source,
        "node:a"
    );
    assert_eq!(
        relation.scene().marks[1].geometry,
        glyph_frame.scene().marks[1].geometry
    );
    assert_eq!(
        relation.scene().marks[0].geometry,
        glyph_frame.scene().marks[0].geometry
    );
    relation
        .set_focus(Some(RelationFocus::Node("node:a".into())))
        .unwrap();
    assert_eq!(
        relation.scene().marks[1].geometry,
        glyph_frame.scene().marks[1].geometry
    );
    assert!(relation.scene().marks[3].opacity < 1.);
    relation
        .set_hidden(BTreeSet::from(["node:b".into()]))
        .unwrap();
    assert_eq!(relation.scene().marks.len(), 2);
    assert!(
        !relation
            .scene()
            .label_series
            .values()
            .any(|id| id == "node:b")
    );
}

#[test]
fn zero_caps_follow_actual_ink_and_bad_candidates_are_atomic() {
    let c = frame();
    let simulation = simulation();
    let mut style = styles();
    let map = mapping(&c, ZoomTransform::default());
    let empty = BTreeSet::new();
    let center = simulation.node("node:a").unwrap().position;
    for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
        style.nodes.insert(
            "node:a".into(),
            NetworkNodeGlyph {
                glyph: GlyphOptions {
                    shape: GlyphShape::Stroked(StrokedGlyph::Plus),
                    size: 0.,
                    rotation: 0.4,
                },
                fill: None,
                stroke: Some(stroke(cap, 10.)),
            },
        );
        let frame = simulation.scene_with_glyphs(&c, &style).unwrap();
        let pointer = map.world_to_client(center).unwrap();
        assert_eq!(
            frame.pick_node(pointer, &map, &empty).unwrap().is_some(),
            cap != StrokeCap::Butt
        );
        let corner = map
            .world_to_client([center[0] + 4.5, center[1] + 4.5])
            .unwrap();
        assert_eq!(
            frame.pick_node(corner, &map, &empty).unwrap().is_some(),
            cap == StrokeCap::Square
        );
    }
    let mut drag = NetworkDragController::new(simulation, Default::default()).unwrap();
    let before = drag.simulation().nodes().to_vec();
    style
        .nodes
        .insert("unknown".into(), NetworkNodeGlyph::default());
    let pointer = map.world_to_client(center).unwrap();
    assert!(
        drag.begin_with_glyphs(&c, &style, 7, pointer, &map, &empty)
            .is_err()
    );
    style.nodes.remove("unknown");
    style.nodes.get_mut("node:a").unwrap().stroke = None;
    assert!(
        drag.begin_with_glyphs(&c, &style, 7, pointer, &map, &empty)
            .is_err()
    );
    let mut wrong = map.clone();
    wrong.world_offset[0] += 1.;
    assert!(
        drag.begin_with_glyphs(&c, &styles(), 7, pointer, &wrong, &empty)
            .is_err()
    );
    style.nodes.get_mut("node:a").unwrap().glyph.size = f64::MAX;
    assert!(drag.simulation().scene_with_glyphs(&c, &style).is_err());
    assert!(drag.active().is_none());
    assert_eq!(drag.simulation().nodes(), before);
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_show_actual_star_wing_and_hollow_triangle() {
    use kartis::render::raster::{FontRegistry, render_raster};
    let c = frame();
    let simulation = simulation();
    // Remove the decorative edge at the hollow center for an independent ink
    // assertion.
    let mut layout: SceneInteraction = simulation.scene_with_glyphs(&c, &styles()).unwrap().into();
    layout.scene.labels.clear();
    layout.scene.label_series.clear();
    layout.scene.marks.remove(0);
    layout.validate().unwrap();
    let image = render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let ink = |x: usize, y: usize| {
        let i = (y * c.width as usize + x) * 4;
        image.rgba[i..i + 3].iter().any(|v| *v < 220)
    };
    let sites = &layout.interaction.as_ref().unwrap().sites;
    assert!(ink(
        sites[0].position[0] as usize,
        (sites[0].position[1] - 24.) as usize
    ));
    assert!(!ink(
        sites[1].position[0] as usize,
        sites[1].position[1] as usize
    ));
}
