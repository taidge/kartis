//! Full occupied-slot group interaction, independent of painted cell padding.
use super::*;
use crate::{InteractionRegion, PickingProfile};

/// Bind one logical datum per occupied, non-hidden Waffle group without
/// changing any Scene paint. Every occupied cell contributes its entire bounds,
/// including padding, to the group's half-open logical region. Regions remain
/// eligible when their source marks are transparent; host-hidden series do not.
///
/// Cells mode keeps the first occupied cell's actual site and custom Path
/// anchor as the canonical datum. Other occupied cells become physical aliases
/// of that site. Areas mode uses the actual union mark and its existing anchor.
/// Empty cells, zero allocations and groups beyond capacity create no sites.
///
/// Complete existing layout/source consistency checks run before conversion.
/// Already adapted regions, aliases, part metadata and polar bindings are
/// rejected. As with other Waffle adapters, coherently edited public layout
/// metadata is supported; this is not an immutable original-input snapshot.
/// The output requires PaintedGeometry. Exported Scene paint and ordinary
/// `into_scene_interaction` retain their existing behavior.
pub fn waffle_area_interaction(layout: &WaffleLayout) -> Result<SceneInteraction> {
    widgets::validate_layout(layout)?;
    let original = &layout.interaction;
    ensure(
        original.regions.is_empty()
            && original.ink_owners.is_empty()
            && original.part_data.is_empty()
            && original.polar.is_none()
            && original.polar_spoke.is_none(),
        "Waffle group regions require an unadapted layout binding",
    )?;
    let mut binding = InteractionBinding {
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    let sites: BTreeMap<_, _> = original
        .sites
        .iter()
        .map(|site| (site.mark_id.as_str(), site))
        .collect();
    let mut expected_sites = BTreeSet::new();
    for group in &layout.groups {
        if group.hidden || group.occupied.is_empty() {
            continue;
        }
        let cells = &layout.cells[group.occupied.clone()];
        let canonical = if let Some(id) = group.area_mark_id.as_deref() {
            expected_sites.insert(id);
            id
        } else {
            let canonical = cells
                .first()
                .and_then(|cell| cell.mark_id.as_deref())
                .ok_or_else(|| ChartError("Waffle group has no actual canonical cell".into()))?;
            for cell in cells {
                let id = cell
                    .mark_id
                    .as_deref()
                    .ok_or_else(|| ChartError("Waffle group has a missing physical cell".into()))?;
                expected_sites.insert(id);
                if id != canonical {
                    binding.ink_owners.insert(id.into(), canonical.into());
                }
            }
            canonical
        };
        let site = sites
            .get(canonical)
            .ok_or_else(|| ChartError("Waffle canonical group site is missing".into()))?;
        binding.sites.push((*site).clone());
        if let Some(anchor) = original.anchors.get(canonical) {
            binding.anchors.insert(canonical.into(), anchor.clone());
        }
        binding.regions.push(InteractionRegion {
            mark_id: canonical.into(),
            bounds: cells.iter().map(|cell| cell.bounds).collect(),
        });
    }
    ensure(
        expected_sites.len() == sites.len() && sites.keys().all(|id| expected_sites.contains(id)),
        "Waffle group source has extra or missing interaction sites",
    )?;
    let frame = SceneInteraction {
        scene: layout.scene.clone(),
        interaction: Some(binding),
        plot: Some(layout.plot),
    };
    frame.validate()?;
    Ok(frame)
}
