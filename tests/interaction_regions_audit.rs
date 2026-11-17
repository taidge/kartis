//! Business regions deliberately differ from physical paint. Nivo's Canvas
//! rectangle helper includes both edges and takes its first cell; this opt-in
//! Rust contract instead uses half-open bounds and last-region precedence.
use std::collections::BTreeSet;

use kartis::surface::{Picking, SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

#[path = "../examples/shared/waffle_regions.rs"]
mod waffle_source;

fn rect(x: f64, y: f64, width: f64, height: f64) -> ClipRect {
    ClipRect {
        x,
        y,
        width,
        height,
    }
}
fn fixture() -> SceneInteraction {
    let mut scene = ChartConfig {
        width: 200.,
        height: 140.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, series, x, radius, opacity) in [
        ("alpha", "A", 40., 4., 0.),
        ("alpha-alias", "A", 90., 15., 1.),
        ("beta", "B", 55., 3., 0.),
        ("ordinary-top", "top", 55., 2., 1.),
        ("ordinary-outside", "outside", 120., 5., 1.),
    ] {
        let mut mark = Mark::new(
            id,
            Geometry::Circle { x, y: 40., radius },
            Color(20, 80, 160, 255),
        );
        mark.series = series.into();
        mark.label = format!("label {id}");
        mark.value = if id == "alpha" || id == "alpha-alias" {
            -913.
        } else {
            42.
        };
        mark.opacity = opacity;
        scene.marks.push(mark);
    }
    let sites = scene
        .marks
        .iter()
        .filter(|m| m.id != "alpha-alias")
        .map(|m| {
            let Geometry::Circle { x, y, .. } = m.geometry else {
                unreachable!()
            };
            InteractionSite {
                mark_id: m.id.clone(),
                series: m.series.clone(),
                label: m.label.clone(),
                value: if m.id == "alpha" { None } else { Some(42.) },
                formatted_value: if m.id == "alpha" {
                    None
                } else {
                    Some("source 42".into())
                },
                color: Color(20, 80, 160, 255),
                position: [x, y],
            }
        })
        .collect();
    SceneInteraction {
        scene,
        plot: None,
        interaction: Some(InteractionBinding {
            sites,
            picking_profile: PickingProfile::PaintedGeometry,
            ink_owners: [("alpha-alias".into(), "alpha".into())].into(),
            regions: vec![
                InteractionRegion {
                    mark_id: "alpha".into(),
                    bounds: vec![rect(20., 20., 40., 40.), rect(70., 20., 30., 40.)],
                },
                InteractionRegion {
                    mark_id: "beta".into(),
                    bounds: vec![rect(50., 30., 20., 30.)],
                },
            ],
            ..Default::default()
        }),
    }
}
fn model() -> SurfaceModel {
    let frame = fixture();
    SurfaceModel::new_with_interaction(frame.scene, SurfaceOptions::default(), frame.interaction)
        .unwrap()
}
fn move_to(model: &mut SurfaceModel, p: (f64, f64)) -> Vec<SurfaceEvent> {
    model.apply(SurfaceInput::Move { id: 7, point: p }).unwrap()
}
fn click(model: &mut SurfaceModel, p: (f64, f64)) -> Vec<SurfaceEvent> {
    model
        .apply(SurfaceInput::Down {
            id: 7,
            point: p,
            brush: false,
        })
        .unwrap();
    model.apply(SurfaceInput::Up { id: 7, point: p }).unwrap()
}

#[test]
fn disjoint_bounds_and_last_region_priority_do_not_invent_physical_ink() {
    let mut m = model();
    for (point, owner, physical) in [
        ((25., 25.), "alpha", None),
        ((75., 25.), "alpha", None),
        ((55., 40.), "beta", Some("ordinary-top")),
        ((120., 40.), "ordinary-outside", Some("ordinary-outside")),
    ] {
        move_to(&mut m, point);
        assert_eq!(m.hovered(), [owner]);
        assert_eq!(
            m.physical_hit_at(point)
                .unwrap()
                .as_ref()
                .map(|h| h.physical_mark_id.as_str()),
            physical
        );
        assert_eq!(
            m.hovered_physical_hit()
                .map(|h| h.physical_mark_id.as_str()),
            physical
        );
    }
    move_to(&mut m, (25., 25.));
    let active = m.active_interaction(m.hovered()).unwrap();
    assert_eq!(active.ids, ["alpha"]);
    assert_eq!(active.rows[0].value, None);
    assert_eq!(active.rows[0].formatted_value, None);
    assert_eq!(active.anchor, [40., 40.]);
    assert_eq!(
        m.scene().marks[0].value,
        -913.,
        "raw source must not be replaced by live paint scalar"
    );
    assert!(click(&mut m, (25., 25.)).contains(&SurfaceEvent::Activated("alpha".into())));
    assert_eq!(m.selected(), Some("alpha"));
    let mut reversed = fixture();
    reversed.interaction.as_mut().unwrap().regions.reverse();
    let mut reversed = SurfaceModel::new_with_interaction(
        reversed.scene,
        SurfaceOptions::default(),
        reversed.interaction,
    )
    .unwrap();
    move_to(&mut reversed, (55., 40.));
    assert_eq!(
        reversed.hovered(),
        ["alpha"],
        "precedence follows region declaration order, not site or paint order"
    );
}

#[test]
fn half_open_edges_and_region_owned_alias_overflow_preserve_independent_physical_hits() {
    let mut m = model();
    for (point, expected) in [
        ((20., 20.), Some("alpha")),
        ((19.999, 20.), None),
        ((49.999, 30.), Some("alpha")),
        ((50., 30.), Some("beta")),
        ((60., 40.), Some("beta")),
        ((70., 40.), Some("alpha")),
        ((99.999, 59.999), Some("alpha")),
        ((100., 60.), None),
        ((40., 60.), None),
        ((100., 40.), None),
        ((104., 40.), None),
    ] {
        move_to(&mut m, point);
        assert_eq!(
            m.hovered().first().map(String::as_str),
            expected,
            "point={point:?}"
        );
    }
    let physical = m.physical_hit_at((104., 40.)).unwrap().unwrap();
    assert_eq!(physical.physical_mark_id, "alpha-alias");
    assert_eq!(physical.logical_site_id, "alpha");
    assert_eq!(m.hovered_physical_hit(), Some(&physical));
    assert!(
        click(&mut m, (104., 40.))
            .iter()
            .all(|e| !matches!(e, SurfaceEvent::Activated(_)))
    );
    move_to(&mut m, (120., 40.));
    assert_eq!(
        m.hovered(),
        ["ordinary-outside"],
        "non-region physical sites keep ordinary fallback"
    );
}

#[test]
fn explicit_leave_touch_cancel_reset_clear_and_pan_retire_region_overflow_physical_hover() {
    fn overflow(m: &mut SurfaceModel) {
        move_to(m, (104., 40.));
        assert!(m.hovered().is_empty());
        assert_eq!(
            m.hovered_physical_hit().unwrap().physical_mark_id,
            "alpha-alias"
        );
    }
    fn cleared(m: &SurfaceModel, events: &[SurfaceEvent]) {
        assert!(m.hovered().is_empty());
        assert!(m.hovered_physical_hit().is_none());
        assert!(
            events.contains(&SurfaceEvent::PhysicalHover(None)),
            "explicit clear publishes physical retirement"
        );
    }
    for input in [
        SurfaceInput::Leave,
        SurfaceInput::Reset,
        SurfaceInput::Clear,
    ] {
        let mut m = model();
        overflow(&mut m);
        let events = m.apply(input).unwrap();
        cleared(&m, &events);
        assert_eq!(
            m.physical_hit_at((104., 40.))
                .unwrap()
                .unwrap()
                .physical_mark_id,
            "alpha-alias",
            "clearing hover does not erase actual source ink"
        );
    }
    let mut m = model();
    overflow(&mut m);
    let events = m
        .apply(SurfaceInput::TouchStart {
            id: 12,
            point: (104., 40.),
        })
        .unwrap();
    cleared(&m, &events);
    // Independent input devices can interleave. Retire the pointer gesture,
    // then accept a mouse Move while the touch record awaits TouchCancel.
    m.apply(SurfaceInput::Cancel { id: 12 }).unwrap();
    overflow(&mut m);
    let events = m.apply(SurfaceInput::TouchCancel { id: 12 }).unwrap();
    cleared(&m, &events);
    assert_eq!(m.touch_count(), 0);
    let mut m = model();
    overflow(&mut m);
    m.apply(SurfaceInput::Down {
        id: 7,
        point: (104., 40.),
        brush: false,
    })
    .unwrap();
    let events = m
        .apply(SurfaceInput::Move {
            id: 7,
            point: (108., 40.),
        })
        .unwrap();
    cleared(&m, &events);
}

#[test]
fn source_opacity_hidden_series_and_touch_are_business_rules_not_alpha_hit_tests() {
    let mut m = model();
    move_to(&mut m, (25., 25.));
    assert_eq!(m.hovered(), ["alpha"]);
    assert!(m.physical_hit_at((40., 40.)).unwrap().is_none());
    let token = m.frame_token();
    let mut next = fixture();
    next.scene.marks[0].opacity = 1.;
    m.set_layout(next).unwrap();
    assert_ne!(m.frame_token(), token);
    assert_eq!(m.hovered(), ["alpha"]);
    assert_eq!(
        m.physical_hit_at((40., 40.))
            .unwrap()
            .unwrap()
            .physical_mark_id,
        "alpha"
    );
    m.set_hidden(BTreeSet::from(["B".into()])).unwrap();
    move_to(&mut m, (55., 40.));
    assert_eq!(m.hovered(), ["alpha"]);
    m.set_hidden(BTreeSet::from(["A".into(), "B".into()]))
        .unwrap();
    move_to(&mut m, (55., 40.));
    assert_eq!(m.hovered(), ["ordinary-top"]);
    assert!(m.physical_hit_at((90., 40.)).unwrap().is_none());
    m.set_hidden(BTreeSet::new()).unwrap();
    m.apply(SurfaceInput::TouchStart {
        id: 12,
        point: (25., 25.),
    })
    .unwrap();
    let events = m
        .apply(SurfaceInput::TouchEnd {
            id: 12,
            point: (25., 25.),
        })
        .unwrap();
    assert!(events.contains(&SurfaceEvent::Activated("alpha".into())));
    m.apply(SurfaceInput::Leave).unwrap();
    m.apply(SurfaceInput::First).unwrap();
    assert_eq!(m.selected(), Some("alpha"));
    assert!(
        m.apply(SurfaceInput::Activate)
            .unwrap()
            .contains(&SurfaceEvent::Activated("alpha".into()))
    );
}

#[test]
fn camera_repicks_stationary_region_and_new_revision_replaces_bounds_as_one_frame() {
    let mut m = model();
    move_to(&mut m, (25., 25.));
    let token = m.frame_token();
    m.apply(SurfaceInput::Wheel {
        point: (25., 25.),
        delta: -2_f64.ln() / 0.002,
    })
    .unwrap();
    assert_eq!(m.view().scale, 2.);
    assert_ne!(m.frame_token(), token);
    assert_eq!(
        m.hovered(),
        ["alpha"],
        "zoom anchor keeps its region without another Move"
    );
    move_to(&mut m, (100., 40.));
    assert_eq!(
        m.hovered(),
        ["beta"],
        "visual point maps once to world (62.5,32.5)"
    );
    let world = (75., 25.);
    let visual = m.view().project(world).unwrap();
    move_to(&mut m, visual);
    assert_eq!(m.hovered(), ["alpha"]);
    assert!(m.physical_hit_at(visual).unwrap().is_none());
    assert!(click(&mut m, visual).contains(&SurfaceEvent::Activated("alpha".into())));
    m.apply(SurfaceInput::Reset).unwrap();
    move_to(&mut m, (25., 25.));
    let old_token = m.frame_token();
    let mut next = fixture();
    let binding = next.interaction.as_mut().unwrap();
    binding.regions[0].bounds = vec![rect(140., 80., 20., 20.)];
    binding.regions[1].bounds = vec![rect(20., 20., 40., 40.)];
    binding
        .sites
        .iter_mut()
        .find(|s| s.mark_id == "beta")
        .unwrap()
        .value = Some(84.);
    next.scene
        .marks
        .iter_mut()
        .find(|mark| mark.id == "beta")
        .unwrap()
        .value = 84.;
    m.set_layout(next).unwrap();
    assert_ne!(m.frame_token(), old_token);
    assert_eq!(m.hovered(), ["beta"]);
    assert_eq!(
        m.active_interaction(m.hovered()).unwrap().rows[0].value,
        Some(84.)
    );
}

#[test]
fn invalid_region_candidates_keep_old_scene_hover_physical_token_and_active_down() {
    type Change = fn(&mut SceneInteraction);
    let mutations: &[(&str, Change)] = &[
        ("missing owner site", |f| {
            f.interaction.as_mut().unwrap().regions[0].mark_id = "unknown".into()
        }),
        ("alias is not a canonical site", |f| {
            f.interaction.as_mut().unwrap().regions[0].mark_id = "alpha-alias".into()
        }),
        ("duplicate owner", |f| {
            let r = f.interaction.as_ref().unwrap().regions[0].clone();
            f.interaction.as_mut().unwrap().regions.push(r);
        }),
        ("empty bounds", |f| {
            f.interaction.as_mut().unwrap().regions[0].bounds.clear()
        }),
        ("NaN x", |f| {
            f.interaction.as_mut().unwrap().regions[1].bounds[0].x = f64::NAN
        }),
        ("infinite height", |f| {
            f.interaction.as_mut().unwrap().regions[1].bounds[0].height = f64::INFINITY
        }),
        ("negative width", |f| {
            f.interaction.as_mut().unwrap().regions[1].bounds[0].width = -1.
        }),
        ("zero height", |f| {
            f.interaction.as_mut().unwrap().regions[1].bounds[0].height = 0.
        }),
        ("extent above cap", |f| {
            f.interaction.as_mut().unwrap().regions[1].bounds[0].width = 1_000_001.
        }),
        ("endpoint above cap", |f| {
            let r = &mut f.interaction.as_mut().unwrap().regions[1].bounds[0];
            r.x = 999_999.;
            r.width = 2.;
        }),
        ("floating collapsed endpoint", |f| {
            let r = &mut f.interaction.as_mut().unwrap().regions[1].bounds[0];
            r.x = 1_000_000.;
            r.width = f64::MIN_POSITIVE;
        }),
        ("legacy profile", |f| {
            f.interaction.as_mut().unwrap().picking_profile = PickingProfile::LegacyGeometry
        }),
        ("rectangle budget", |f| {
            f.interaction.as_mut().unwrap().regions[1].bounds =
                vec![rect(1., 1., 1., 1.); MAX_MARKS]
        }),
        ("owner budget", |f| {
            let r = f.interaction.as_ref().unwrap().regions[0].clone();
            f.interaction.as_mut().unwrap().regions = vec![r; MAX_MARKS + 1];
        }),
    ];
    for (name, change) in mutations {
        let mut m = model();
        move_to(&mut m, (90., 40.));
        m.apply(SurfaceInput::Down {
            id: 7,
            point: (90., 40.),
            brush: false,
        })
        .unwrap();
        let scene = m.scene().clone();
        let binding = m.interaction_data().unwrap().clone();
        let hover = m.hovered().to_vec();
        let token = m.frame_token();
        let physical = m.hovered_physical_hit().cloned();
        let mut candidate = fixture();
        candidate.scene.background = Color(1, 2, 3, 255);
        change(&mut candidate);
        assert!(m.set_layout(candidate).is_err(), "{name} accepted");
        assert_eq!(m.scene(), &scene, "{name}");
        assert_eq!(m.interaction_data(), Some(&binding), "{name}");
        assert_eq!(m.hovered(), hover, "{name}");
        assert_eq!(m.frame_token(), token, "{name}");
        assert_eq!(m.hovered_physical_hit(), physical.as_ref(), "{name}");
        assert!(m.dragging(), "{name} canceled accepted active gesture");
        assert!(
            m.apply(SurfaceInput::Up {
                id: 7,
                point: (90., 40.)
            })
            .unwrap()
            .contains(&SurfaceEvent::Activated("alpha".into())),
            "{name}"
        );
    }
}

#[test]
fn rectangle_budget_is_cumulative_exact_and_strict_profile_rejects_nearest_and_slices() {
    let mut frame = fixture();
    frame.interaction.as_mut().unwrap().regions[0].bounds =
        vec![rect(20., 20., 1., 1.); MAX_MARKS - 1];
    frame.validate().unwrap();
    let mut m = SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        SurfaceOptions::default(),
        frame.interaction.clone(),
    )
    .unwrap();
    move_to(&mut m, (20., 20.));
    assert_eq!(m.hovered(), ["alpha"]);
    frame.interaction.as_mut().unwrap().regions[1]
        .bounds
        .push(rect(1., 1., 1., 1.));
    assert!(frame.validate().is_err());
    for picking in [
        Picking::Nearest { distance: 50. },
        Picking::SliceX { distance: 50. },
        Picking::SliceY { distance: 50. },
    ] {
        let frame = fixture();
        assert!(
            SurfaceModel::new_with_interaction(
                frame.scene,
                SurfaceOptions {
                    picking,
                    ..Default::default()
                },
                frame.interaction
            )
            .is_err()
        );
    }
}

#[test]
fn region_extent_cannot_bypass_scene_and_nonzero_plot_viewport_limits() {
    let mut frame = fixture();
    frame.plot = Some(rect(30., 30., 100., 60.));
    let mut m = SurfaceModel::new_with_interaction(
        frame.scene,
        SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction,
    )
    .unwrap();
    move_to(&mut m, (25., 25.));
    assert!(m.hovered().is_empty());
    move_to(&mut m, (30., 30.));
    assert_eq!(m.hovered(), ["alpha"]);
    move_to(&mut m, (130.001, 40.));
    assert!(m.hovered().is_empty());
    let mut frame = fixture();
    frame.interaction.as_mut().unwrap().regions[0].bounds = vec![rect(-100., -100., 400., 400.)];
    let mut m = SurfaceModel::new_with_interaction(
        frame.scene,
        SurfaceOptions::default(),
        frame.interaction,
    )
    .unwrap();
    for p in [(-1., 20.), (201., 20.), (20., -1.), (20., 141.)] {
        move_to(&mut m, p);
        assert!(m.hovered().is_empty(), "{p:?}");
    }
}

#[test]
fn waffle_adapter_keeps_source_ink_and_maps_full_slots_to_one_real_group_site() {
    for phase in 0..=7 {
        let layout = waffle_source::source(phase, 320.).unwrap();
        let before = layout.scene.clone();
        let frame = waffle_area_interaction(&layout).unwrap();
        assert_eq!(layout.scene, before);
        assert_eq!(
            frame.scene, before,
            "adapter cannot draw transparent proxy marks"
        );
        let binding = frame.interaction.as_ref().unwrap();
        let groups = layout
            .groups
            .iter()
            .filter(|g| !g.hidden && !g.occupied.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(binding.sites.len(), groups.len());
        assert_eq!(binding.regions.len(), groups.len());
        assert!(!binding.sites.iter().any(|s| s.series == "zero"));
        assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
        let mut m = SurfaceModel::new_with_interaction(
            frame.scene.clone(),
            SurfaceOptions {
                plot: frame.plot,
                ..Default::default()
            },
            frame.interaction.clone(),
        )
        .unwrap();
        for (i, group) in groups.iter().enumerate() {
            let site = &binding.sites[i];
            let region = &binding.regions[i];
            assert_eq!(site.series, group.source.id);
            assert_eq!(site.value, Some(group.source.value));
            assert_eq!(
                site.formatted_value.as_deref(),
                Some(group.formatted_value.as_str())
            );
            assert_eq!(site.mark_id, region.mark_id);
            let cells = &layout.cells[group.occupied.clone()];
            assert_eq!(
                region.bounds,
                cells.iter().map(|c| c.bounds).collect::<Vec<_>>()
            );
            for cell in cells {
                if let Some(mark_id) = &cell.mark_id
                    && mark_id != &site.mark_id
                {
                    assert_eq!(binding.ink_owners.get(mark_id), Some(&site.mark_id));
                }
                let gap = (cell.bounds.x + 1., cell.bounds.y + cell.bounds.height / 2.);
                move_to(&mut m, gap);
                assert_eq!(
                    m.hovered(),
                    [site.mark_id.as_str()],
                    "phase={phase} slot={}",
                    cell.index
                );
                assert_eq!(
                    m.active_interaction(m.hovered()).unwrap().rows[0].value,
                    Some(group.source.value)
                );
                if phase != 3 {
                    assert!(
                        m.physical_hit_at(gap).unwrap().is_none(),
                        "phase={phase} slot={} gap is not paint",
                        cell.index
                    );
                } else {
                    assert_eq!(
                        m.physical_hit_at(gap).unwrap().unwrap().physical_mark_id,
                        site.mark_id
                    );
                }
                let center = (
                    cell.bounds.x + cell.bounds.width / 2.,
                    cell.bounds.y + cell.bounds.height / 2.,
                );
                if phase == 1 {
                    assert!(m.physical_hit_at(center).unwrap().is_none());
                }
            }
            assert!(
                click(
                    &mut m,
                    (
                        region.bounds[0].x + 1.,
                        region.bounds[0].y + region.bounds[0].height / 2.
                    )
                )
                .contains(&SurfaceEvent::Activated(site.mark_id.clone()))
            );
            if phase == 4 {
                let mark = frame
                    .scene
                    .marks
                    .iter()
                    .find(|mark| mark.id == site.mark_id)
                    .unwrap();
                assert!(matches!(mark.geometry, Geometry::Path(_)));
                assert_eq!(binding.anchor_for(mark).unwrap(), Some(site.position));
                assert_eq!(
                    m.active_interaction(m.hovered()).unwrap().anchor,
                    site.position
                );
            }
        }
        for cell in layout.cells.iter().filter(|c| c.owner.is_none()) {
            move_to(
                &mut m,
                (
                    cell.bounds.x + cell.bounds.width / 2.,
                    cell.bounds.y + cell.bounds.height / 2.,
                ),
            );
            assert!(
                m.hovered().is_empty(),
                "painted empty slots do not become source data"
            );
        }
        if phase == 2 {
            assert!(!binding.sites.iter().any(|s| s.series == "alpha"));
        }
        let mut already_adapted = layout.clone();
        already_adapted.interaction = binding.clone();
        assert!(waffle_area_interaction(&already_adapted).is_err());
    }
}
