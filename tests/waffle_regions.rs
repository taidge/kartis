use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

#[path = "../examples/shared/waffle_regions.rs"]
mod shared;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn model(frame: &SceneInteraction) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction.clone(),
    )
    .unwrap()
}
fn hover(model: &mut SurfaceModel, p: [f64; 2]) -> Vec<String> {
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (p[0], p[1]),
        })
        .unwrap();
    model.hovered().to_vec()
}
fn group_site(frame: &SceneInteraction, id: &str) -> Option<String> {
    frame
        .interaction
        .as_ref()
        .unwrap()
        .sites
        .iter()
        .find(|s| s.series == id)
        .map(|s| s.mark_id.clone())
}

#[test]
fn all_directions_allocations_modes_and_hidden_groups_keep_exact_source_paint() {
    for direction in [
        WaffleFillDirection::Top,
        WaffleFillDirection::Left,
        WaffleFillDirection::Right,
        WaffleFillDirection::Bottom,
    ] {
        for allocation in [
            WaffleAllocation::IndependentRound,
            WaffleAllocation::LargestRemainder,
        ] {
            for mode in [WaffleMode::Cells, WaffleMode::Areas] {
                for hidden in [false, true] {
                    let layout = waffle_layout(
                        &config(),
                        &[
                            RadialDatum::new("alpha", 13.),
                            RadialDatum::new("beta", 13.),
                            RadialDatum::new("gamma", 13.),
                            RadialDatum::new("zero", 0.),
                        ],
                        &WaffleLayoutOptions {
                            rows: 3,
                            columns: 4,
                            total: Some(100.),
                            padding: 8.,
                            direction,
                            allocation,
                            mode,
                            hidden: if hidden {
                                BTreeSet::from(["beta".into()])
                            } else {
                                BTreeSet::new()
                            },
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    let before = layout.clone();
                    let svg = kartis::render::to_svg(&layout.scene).unwrap();
                    let frame = waffle_area_interaction(&layout).unwrap();
                    let binding = frame.interaction.as_ref().unwrap();
                    assert_eq!(layout, before);
                    assert_eq!(frame.scene, layout.scene);
                    assert_eq!(kartis::render::to_svg(&frame.scene).unwrap(), svg);
                    assert_eq!(frame.plot, layout.clone().into_scene_interaction().plot);
                    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
                    let groups = layout
                        .groups
                        .iter()
                        .filter(|g| !g.hidden && !g.occupied.is_empty())
                        .collect::<Vec<_>>();
                    assert_eq!(binding.sites.len(), groups.len());
                    assert_eq!(binding.regions.len(), groups.len());
                    assert!(group_site(&frame, "zero").is_none());
                    if hidden {
                        assert!(group_site(&frame, "beta").is_none());
                    }
                    let mut surface = model(&frame);
                    let controls = surface
                        .controls_scene()
                        .marks
                        .into_iter()
                        .filter(|m| !m.label.is_empty())
                        .collect::<Vec<_>>();
                    assert_eq!(controls.len(), groups.len());
                    for (group, region) in groups.iter().zip(&binding.regions) {
                        let cells = &layout.cells[group.occupied.clone()];
                        let canonical = group
                            .area_mark_id
                            .as_ref()
                            .or(cells[0].mark_id.as_ref())
                            .unwrap();
                        assert_eq!(&region.mark_id, canonical);
                        assert_eq!(
                            region.bounds,
                            cells.iter().map(|c| c.bounds).collect::<Vec<_>>()
                        );
                        let site = binding
                            .sites
                            .iter()
                            .find(|s| &s.mark_id == canonical)
                            .unwrap();
                        assert_eq!(site.value, Some(group.source.value));
                        assert_eq!(
                            site.formatted_value.as_deref(),
                            Some(group.formatted_value.as_str())
                        );
                        for cell in cells {
                            let p = [cell.bounds.x + 0.25, cell.bounds.y + 0.25];
                            assert_eq!(
                                hover(&mut surface, p).as_slice(),
                                std::slice::from_ref(canonical),
                                "padding belongs to the whole group"
                            );
                            let active = surface.active_interaction(surface.hovered()).unwrap();
                            assert_eq!(active.rows.len(), 1);
                            assert_eq!(active.rows[0].series, group.source.id);
                            let physical = surface.physical_hit_at((p[0], p[1])).unwrap();
                            if mode == WaffleMode::Cells {
                                assert!(
                                    physical.is_none(),
                                    "padded cell corner has no painted ink"
                                );
                            } else {
                                assert_eq!(
                                    physical.unwrap().logical_site_id,
                                    *canonical,
                                    "area unions really paint the full occupied slot"
                                );
                            }
                            if mode == WaffleMode::Cells && cell.mark_id.as_ref() != Some(canonical)
                            {
                                assert_eq!(
                                    binding.ink_owners.get(cell.mark_id.as_ref().unwrap()),
                                    Some(canonical)
                                );
                            }
                        }
                    }
                    assert_eq!(
                        binding.ink_owners.len(),
                        if mode == WaffleMode::Cells {
                            groups.iter().map(|g| g.occupied.len() - 1).sum()
                        } else {
                            0
                        }
                    );
                    for cell in &layout.cells {
                        if cell.owner.is_none() {
                            assert!(
                                hover(
                                    &mut surface,
                                    [
                                        cell.bounds.x + cell.bounds.width / 2.,
                                        cell.bounds.y + cell.bounds.height / 2.
                                    ]
                                )
                                .is_empty()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn shared_edges_are_half_open_and_empty_zero_and_capacity_clipped_groups_have_no_site() {
    for phase in [0, 3, 5, 6, 7] {
        let layout = shared::source(phase, 320.).unwrap();
        let frame = waffle_area_interaction(&layout).unwrap();
        let mut surface = model(&frame);
        for cell in &layout.cells {
            for p in [
                [cell.bounds.x, cell.bounds.y],
                [
                    cell.bounds.x + cell.bounds.width,
                    cell.bounds.y + cell.bounds.height / 2.,
                ],
                [
                    cell.bounds.x + cell.bounds.width / 2.,
                    cell.bounds.y + cell.bounds.height,
                ],
            ] {
                let expected = layout
                    .area_at(p[0], p[1])
                    .and_then(|g| group_site(&frame, &g.source.id))
                    .into_iter()
                    .collect::<Vec<_>>();
                assert_eq!(hover(&mut surface, p), expected);
            }
        }
    }
    for allocation in [
        WaffleAllocation::IndependentRound,
        WaffleAllocation::LargestRemainder,
    ] {
        let layout = waffle_layout(
            &config(),
            &[
                RadialDatum::new("first", 34.),
                RadialDatum::new("second", 33.),
                RadialDatum::new("cropped", 33.),
                RadialDatum::new("zero", 0.),
            ],
            &WaffleLayoutOptions {
                rows: 1,
                columns: 2,
                total: Some(100.),
                allocation,
                ..Default::default()
            },
        )
        .unwrap();
        let frame = waffle_area_interaction(&layout).unwrap();
        assert_eq!(frame.interaction.as_ref().unwrap().sites.len(), 2);
        assert!(group_site(&frame, "cropped").is_none());
        assert!(group_site(&frame, "zero").is_none());
    }
    for mode in [WaffleMode::Cells, WaffleMode::Areas] {
        let empty = waffle_layout(
            &config(),
            &[],
            &WaffleLayoutOptions {
                mode,
                ..Default::default()
            },
        )
        .unwrap();
        let frame = waffle_area_interaction(&empty).unwrap();
        assert!(frame.interaction.as_ref().unwrap().sites.is_empty());
        assert!(frame.interaction.as_ref().unwrap().regions.is_empty());
        assert_eq!(frame.scene, empty.scene);
    }
}

#[test]
fn transparent_cells_keep_logical_regions_and_one_keyboard_control_but_never_fake_ink() {
    let layout = shared::source(1, 320.).unwrap();
    assert!(layout.scene.marks.iter().all(|m| m.opacity == 0.));
    let frame = waffle_area_interaction(&layout).unwrap();
    let mut surface = model(&frame);
    let binding = frame.interaction.as_ref().unwrap();
    assert_eq!(binding.sites.len(), 2);
    for (group, region) in layout
        .groups
        .iter()
        .filter(|g| !g.occupied.is_empty())
        .zip(&binding.regions)
    {
        let cell = &layout.cells[group.occupied.start];
        let p = [cell.bounds.x + 1., cell.bounds.y + 1.];
        assert_eq!(
            hover(&mut surface, p).as_slice(),
            std::slice::from_ref(&region.mark_id)
        );
        assert!(surface.physical_hit_at((p[0], p[1])).unwrap().is_none());
    }
    let controls = surface
        .controls_scene()
        .marks
        .into_iter()
        .filter(|m| !m.label.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(controls.len(), 2);
    surface.apply(SurfaceInput::First).unwrap();
    surface.apply(SurfaceInput::Activate).unwrap();
    assert!(
        binding
            .sites
            .iter()
            .any(|s| Some(s.mark_id.as_str()) == surface.selected())
    );
    surface
        .set_hidden(BTreeSet::from(["alpha".into()]))
        .unwrap();
    let cell = &layout.cells[0];
    assert!(hover(&mut surface, [cell.bounds.x + 1., cell.bounds.y + 1.]).is_empty());
    assert!(surface.logical_datum(&binding.sites[0].mark_id).is_none());
    let hidden = shared::source(2, 320.).unwrap();
    let frame = waffle_area_interaction(&hidden).unwrap();
    assert_eq!(frame.interaction.as_ref().unwrap().sites.len(), 1);
    assert_eq!(frame.interaction.as_ref().unwrap().sites[0].series, "beta");
}

#[test]
fn custom_paths_keep_only_canonical_anchor_and_ink_cannot_escape_its_group_region() {
    let base = shared::source(4, 320.).unwrap();
    let frame = waffle_area_interaction(&base).unwrap();
    let binding = frame.interaction.as_ref().unwrap();
    assert_eq!(binding.anchors.len(), 2);
    assert_eq!(binding.sites.len(), 2);
    for site in &binding.sites {
        assert_eq!(
            binding.anchors.get(&site.mark_id),
            base.interaction.anchors.get(&site.mark_id)
        );
        assert_eq!(
            binding
                .anchor_for(
                    frame
                        .scene
                        .marks
                        .iter()
                        .find(|m| m.id == site.mark_id)
                        .unwrap()
                )
                .unwrap(),
            Some(site.position)
        );
    }
    let options = WaffleLayoutOptions {
        rows: 3,
        columns: 5,
        total: Some(15.),
        padding: 8.,
        ..Default::default()
    };
    let layout = waffle_layout_with_cell_factory(
        &config(),
        &[RadialDatum::new("one", 1.)],
        &options,
        &mut |c| {
            let [x, y] = c.position;
            Ok(c.inherit(Geometry::Path(vec![
                C::Move(P::new(x - 6., y - 106.)),
                C::Line(P::new(x + 6., y - 106.)),
                C::Line(P::new(x + 6., y - 94.)),
                C::Line(P::new(x - 6., y - 94.)),
                C::Close,
            ])))
        },
    )
    .unwrap();
    let frame = waffle_area_interaction(&layout).unwrap();
    let mut surface = model(&frame);
    let cell = &layout.cells[0];
    let p = [
        cell.bounds.x + cell.bounds.width / 2.,
        cell.bounds.y + cell.bounds.height / 2. - 100.,
    ];
    assert!(layout.area_at(p[0], p[1]).is_none());
    assert!(surface.physical_hit_at((p[0], p[1])).unwrap().is_some());
    assert!(
        hover(&mut surface, p).is_empty(),
        "outside-region source ink is not an alternate region hit"
    );
}

#[test]
fn adapter_rejects_preexisting_extra_bindings_and_inconsistent_public_metadata() {
    let base = shared::source(0, 320.).unwrap();
    type Mutate = Box<dyn FnOnce(&mut WaffleLayout)>;
    let changes: Vec<Mutate> = vec![
        Box::new(|l| {
            l.interaction.regions.push(InteractionRegion {
                mark_id: l.interaction.sites[0].mark_id.clone(),
                bounds: vec![l.cells[0].bounds],
            })
        }),
        Box::new(|l| {
            let id = l.interaction.sites[1].mark_id.clone();
            let owner = l.interaction.sites[0].mark_id.clone();
            l.interaction.sites.remove(1);
            l.interaction.ink_owners.insert(id, owner);
        }),
        Box::new(|l| {
            let id = l.interaction.sites[0].mark_id.clone();
            l.interaction.part_data.insert(
                id,
                InteractionPartDatum {
                    source_id: "fake".into(),
                    value: 3.,
                },
            );
        }),
        Box::new(|l| l.groups[0].source.value = 11.),
        Box::new(|l| l.groups[0].occupied.end -= 1),
        Box::new(|l| l.groups[0].polygons.clear()),
        Box::new(|l| l.cells[0].owner = None),
        Box::new(|l| l.cells[0].bounds.width = f64::NAN),
        Box::new(|l| l.interaction.sites[0].value = Some(999.)),
        Box::new(|l| {
            l.interaction.sites.pop();
        }),
        Box::new(|l| {
            let mark = &mut l.scene.marks[0];
            mark.series = "fake".into();
        }),
        Box::new(|l| {
            let mut mark = l.scene.marks[0].clone();
            mark.id = "extra-business-mark".into();
            let mut site = l.interaction.sites[0].clone();
            site.mark_id = mark.id.clone();
            l.scene.marks.push(mark);
            l.interaction.sites.push(site);
        }),
    ];
    for change in changes {
        let mut layout = base.clone();
        change(&mut layout);
        assert!(waffle_area_interaction(&layout).is_err());
    }
    let mut path = shared::source(3, 320.).unwrap();
    let id = path.groups[0].area_mark_id.as_ref().unwrap();
    path.scene
        .marks
        .iter_mut()
        .find(|m| &m.id == id)
        .unwrap()
        .geometry = Geometry::Circle {
        x: 30.,
        y: 30.,
        radius: 3.,
    };
    assert!(waffle_area_interaction(&path).is_err());
    let mut coherent = base;
    for g in &mut coherent.groups {
        if g.source.id == "alpha" {
            g.source.label = "Coherent public edit".into();
        }
    }
    for mark in &mut coherent.scene.marks {
        if mark.series == "alpha" {
            mark.label = "Coherent public edit".into();
        }
    }
    for site in &mut coherent.interaction.sites {
        if site.series == "alpha" {
            site.label = "Coherent public edit".into();
        }
    }
    assert!(
        waffle_area_interaction(&coherent).is_ok(),
        "coherently edited public layouts retain their existing contract"
    );
}
