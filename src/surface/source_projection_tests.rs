use super::*;
use crate::{ChartConfig, Color, Geometry, InteractionBinding, InteractionSite, PickingProfile};

fn model() -> SurfaceModel {
    let mut scene = ChartConfig {
        width: 120.,
        height: 100.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut node = Mark::new(
        "node",
        Geometry::Circle {
            x: 50.,
            y: 50.,
            radius: 10.,
        },
        Color(0, 100, 255, 255),
    );
    node.series = "original".into();
    node.label = "original datum".into();
    scene.marks.push(node);
    scene.marks.push(Mark::new(
        "decoration",
        Geometry::Rect {
            x: 44.,
            y: 44.,
            width: 12.,
            height: 12.,
        },
        Color(230, 110, 20, 255),
    ));
    let binding = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "node".into(),
            series: "original".into(),
            label: "original datum".into(),
            value: Some(0.),
            formatted_value: None,
            color: Color(0, 100, 255, 255),
            position: [50., 50.],
        }],
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    SurfaceModel::new_with_interaction(scene, SurfaceOptions::default(), Some(binding)).unwrap()
}

#[test]
fn literal_omissions_repeats_and_decorative_blockers_preserve_business_source() {
    let mut model = model();
    let source = model.scene().clone();
    let binding = model.interaction_data().cloned();
    assert_eq!(
        model
            .physical_hit_at((50., 50.))
            .unwrap()
            .unwrap()
            .logical_site_id,
        "node"
    );
    model.set_source_paint_order(vec![0, 1]).unwrap();
    assert!(model.physical_hit_at((50., 50.)).unwrap().is_none());
    model.set_source_paint_order(vec![1, 0]).unwrap();
    assert_eq!(
        model
            .physical_hit_at((50., 50.))
            .unwrap()
            .unwrap()
            .logical_site_id,
        "node"
    );
    model.set_source_paint_order(vec![0, 1, 0]).unwrap();
    assert_eq!(
        model
            .physical_hit_at((50., 50.))
            .unwrap()
            .unwrap()
            .physical_mark_id,
        "node"
    );
    model.set_source_paint_order(Vec::new()).unwrap();
    assert!(model.physical_hit_at((50., 50.)).unwrap().is_none());
    assert!(model.logical_datum("node").is_some());
    assert_eq!(model.scene(), &source);
    assert_eq!(model.interaction_data(), binding.as_ref());
}

#[test]
fn invalid_projection_is_atomic_and_plain_layout_retires_the_projection() {
    let mut model = model();
    model.set_source_paint_order(vec![0]).unwrap();
    let hit = model.physical_hit_at((50., 50.)).unwrap().unwrap();
    assert!(model.set_source_paint_order(vec![usize::MAX]).is_err());
    assert!(model.is_current_hit(&hit));
    assert_eq!(model.physical_hit_at((50., 50.)).unwrap(), Some(hit));
    model.set_source_paint_order(Vec::new()).unwrap();
    model
        .set_layout(crate::SceneInteraction {
            scene: model.scene().clone(),
            plot: None,
            interaction: model.interaction_data().cloned(),
        })
        .unwrap();
    assert_eq!(
        model
            .physical_hit_at((50., 50.))
            .unwrap()
            .unwrap()
            .logical_site_id,
        "node"
    );
}

#[test]
fn mesh_input_keeps_real_source_query_but_does_not_create_physical_hover() {
    let mut model = model();
    model.set_source_paint_order(vec![0]).unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (50., 50.),
        })
        .unwrap();
    assert_eq!(model.hovered(), &["node"]);
    model
        .apply_without_source_pick(SurfaceInput::Move {
            id: 1,
            point: (50., 50.),
        })
        .unwrap();
    assert!(model.hovered().is_empty());
    assert!(model.hovered_physical_hit().is_none());
    assert!(model.physical_hit_at((50., 50.)).unwrap().is_some());
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (50., 50.),
        })
        .unwrap();
    assert_eq!(model.hovered(), &["node"]);
}

#[test]
fn mesh_pointer_is_not_replayed_after_source_layout_or_projection_changes() {
    let mut model = model();
    model.set_source_paint_order(vec![0]).unwrap();
    model
        .apply_without_source_pick(SurfaceInput::Move {
            id: 1,
            point: (50., 50.),
        })
        .unwrap();
    assert!(model.hover_point.is_none());
    let candidate = crate::SceneInteraction {
        scene: model.scene().clone(),
        plot: None,
        interaction: model.interaction_data().cloned(),
    };
    let changes = model
        .set_projected_layout(candidate.clone(), vec![0])
        .unwrap();
    assert!(model.hovered().is_empty());
    assert!(model.hovered_physical_hit().is_none());
    assert!(!changes.iter().any(|event| matches!(
        event,
        SurfaceEvent::Hover(ids) if !ids.is_empty()
    )));
    model.set_layout(candidate).unwrap();
    model.set_source_paint_order(vec![0]).unwrap();
    assert!(model.hovered().is_empty());
    assert!(model.hovered_physical_hit().is_none());

    // Suppressing source hover does not abandon native view or drag input.
    model.options.zoom = true;
    model.options.pan = true;
    model
        .apply_without_source_pick(SurfaceInput::Wheel {
            point: (50., 50.),
            delta: -100.,
        })
        .unwrap();
    assert!(model.view().scale > 1.);
    model
        .apply_without_source_pick(SurfaceInput::Down {
            id: 1,
            point: (50., 50.),
            brush: false,
        })
        .unwrap();
    model
        .apply_without_source_pick(SurfaceInput::Move {
            id: 1,
            point: (60., 50.),
        })
        .unwrap();
    assert!(model.dragging());
    assert!(model.drag.unwrap().moved);
    assert!(model.hover_point.is_none());
    assert!(model.hovered().is_empty());
}

#[test]
fn projected_layout_publishes_only_final_hover_and_physical_observations() {
    let mut model = model();
    model.set_source_paint_order(vec![0]).unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (50., 50.),
        })
        .unwrap();
    let candidate = crate::SceneInteraction {
        scene: model.scene().clone(),
        plot: None,
        interaction: model.interaction_data().cloned(),
    };
    let changes = model
        .set_projected_layout(candidate.clone(), Vec::new())
        .unwrap();
    assert_eq!(
        changes
            .iter()
            .filter_map(|event| match event {
                SurfaceEvent::Hover(ids) => Some(ids.clone()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![Vec::<String>::new()]
    );
    assert_eq!(
        changes
            .iter()
            .filter_map(|event| match event {
                SurfaceEvent::PhysicalHover(hit) => Some(hit.clone()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![None]
    );
    let changes = model
        .set_projected_layout(candidate.clone(), Vec::new())
        .unwrap();
    assert!(changes.iter().all(|event| !matches!(
        event,
        SurfaceEvent::Hover(_) | SurfaceEvent::PhysicalHover(_)
    )));
    let token = model.frame_token();
    assert!(
        model
            .set_projected_layout(candidate, vec![usize::MAX])
            .is_err()
    );
    assert_eq!(model.frame_token(), token);
    assert!(model.hovered().is_empty());
    assert!(model.physical_hit_at((50., 50.)).unwrap().is_none());
}

#[test]
fn mesh_click_preserves_existing_source_selection_but_plain_blank_click_clears_it() {
    let mut model = model();
    model.set_source_paint_order(vec![0]).unwrap();
    model
        .apply(SurfaceInput::Down {
            id: 1,
            point: (50., 50.),
            brush: false,
        })
        .unwrap();
    let selected = model
        .apply(SurfaceInput::Up {
            id: 1,
            point: (50., 50.),
        })
        .unwrap();
    assert_eq!(model.selected(), Some("node"));
    assert!(selected.iter().any(|event| matches!(
        event,
        SurfaceEvent::Selected(Some(id)) if id == "node"
    )));

    for point in [(50., 50.), (90., 80.)] {
        for input in [
            SurfaceInput::Down {
                id: 1,
                point,
                brush: false,
            },
            SurfaceInput::Up { id: 1, point },
        ] {
            let changes = model.apply_without_source_pick(input).unwrap();
            assert_eq!(model.selected(), Some("node"));
            assert!(
                changes
                    .iter()
                    .all(|event| !matches!(event, SurfaceEvent::Selected(_)))
            );
        }
    }
    model
        .apply(SurfaceInput::Down {
            id: 1,
            point: (90., 80.),
            brush: false,
        })
        .unwrap();
    let cleared = model
        .apply(SurfaceInput::Up {
            id: 1,
            point: (90., 80.),
        })
        .unwrap();
    assert_eq!(model.selected(), None);
    assert!(
        cleared
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Selected(None)))
    );
}
