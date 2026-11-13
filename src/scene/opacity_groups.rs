use std::collections::{BTreeMap, BTreeSet};

use super::*;

/// One source-over compositing group. Membership follows the Scene's actual
/// paint order and cannot overlap another group or include labels.
/// Canvas backends reject a fractional group containing a mark that itself
/// requires an isolated blend layer, before starting a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkOpacityGroup {
    pub id: String,
    pub members: Vec<String>,
    pub opacity: f64,
}

impl Scene {
    /// Build one sparse index for a traversal of an accepted frame. No cache
    /// is retained on the publicly mutable Scene.
    pub(crate) fn mark_group_opacities(&self) -> BTreeMap<String, f64> {
        self.opacity_groups
            .iter()
            .flat_map(|g| g.members.iter().map(move |id| (id.clone(), g.opacity)))
            .collect()
    }
    /// Effective scalar visibility for picking and controls. Renderers must
    /// composite the group, rather than multiplying this into member paints.
    pub fn effective_mark_opacity(&self, mark: &Mark) -> f64 {
        mark.opacity
            * self
                .opacity_groups
                .iter()
                .find(|g| g.members.iter().any(|id| id == &mark.id))
                .map_or(1., |g| g.opacity)
    }

    pub(crate) fn validate_opacity_groups(&self) -> Result<()> {
        ensure(
            self.opacity_groups.len() <= MAX_MARKS,
            "opacity group budget exceeded",
        )?;
        if self.opacity_groups.is_empty() {
            return Ok(());
        }
        let order = self.draw_items();
        let positions: BTreeMap<_, _> = order
            .iter()
            .enumerate()
            .filter_map(|(p, item)| match item {
                DrawItem::Mark(i) => self.marks.get(*i).map(|m| (m.id.as_str(), p)),
                _ => None,
            })
            .collect();
        let mut ids = BTreeSet::new();
        let mut members = BTreeSet::new();
        let mut bytes = 0usize;
        for group in &self.opacity_groups {
            ensure(
                !group.id.is_empty() && group.id.len() <= MAX_LABEL_BYTES && ids.insert(&group.id),
                "invalid or duplicate opacity group ID",
            )?;
            ensure(
                group.opacity.is_finite() && (0. ..=1.).contains(&group.opacity),
                "invalid group opacity",
            )?;
            ensure(
                !group.members.is_empty() && group.members.len() <= self.marks.len(),
                "invalid opacity group membership",
            )?;
            let mut last = None;
            for id in &group.members {
                bytes = bytes
                    .checked_add(id.len())
                    .ok_or_else(|| ChartError("opacity group ID byte overflow".into()))?;
                ensure(
                    bytes <= 4_000_000 && members.len() < MAX_MARKS,
                    "opacity group membership budget exceeded",
                )?;
                ensure(
                    members.insert(id.as_str()),
                    "overlapping or duplicate opacity group member",
                )?;
                let p = *positions
                    .get(id.as_str())
                    .ok_or_else(|| ChartError("opacity group references unknown mark".into()))?;
                ensure(
                    last.is_none_or(|last| p == last + 1),
                    "opacity group members must be consecutive in paint order",
                )?;
                last = Some(p);
            }
        }
        Ok(())
    }

    /// Endpoints may omit an opacity-one group. Any other membership/topology
    /// change needs an immediate replacement, rather than a per-member fade.
    pub(crate) fn interpolate_opacity_groups(
        &self,
        next: &Self,
        t: f64,
    ) -> Result<Vec<MarkOpacityGroup>> {
        let mut groups: BTreeMap<&str, (&MarkOpacityGroup, f64, f64)> = BTreeMap::new();
        for g in &self.opacity_groups {
            groups.insert(&g.id, (g, g.opacity, 1.));
        }
        for g in &next.opacity_groups {
            if let Some((old, _, b)) = groups.get_mut(g.id.as_str()) {
                ensure(
                    old.members == g.members,
                    "animation opacity group membership changed",
                )?;
                *b = g.opacity;
            } else {
                groups.insert(&g.id, (g, 1., g.opacity));
            }
        }
        if t == 0. {
            return Ok(self.opacity_groups.clone());
        }
        if t == 1. {
            return Ok(next.opacity_groups.clone());
        }
        let mut result = Vec::new();
        for (_, (g, a, b)) in groups {
            let opacity = super::lerp(a, b, t);
            if opacity != 1. {
                result.push(MarkOpacityGroup {
                    opacity,
                    ..g.clone()
                });
            }
        }
        Ok(result)
    }

    /// Start/end events indexed by actual draw-item position. Validation
    /// guarantees one unambiguous group per painted interval.
    pub(crate) fn opacity_group_events(&self) -> (BTreeMap<usize, usize>, BTreeMap<usize, usize>) {
        if self.opacity_groups.is_empty() {
            return (BTreeMap::new(), BTreeMap::new());
        }
        let positions: BTreeMap<_, _> = self
            .draw_items()
            .into_iter()
            .enumerate()
            .filter_map(|(p, item)| match item {
                DrawItem::Mark(i) => Some((self.marks[i].id.as_str(), p)),
                _ => None,
            })
            .collect();
        let mut starts = BTreeMap::new();
        let mut ends = BTreeMap::new();
        for (i, g) in self.opacity_groups.iter().enumerate() {
            if let (Some(first), Some(last)) = (g.members.first(), g.members.last())
                && let (Some(&start), Some(&end)) =
                    (positions.get(first.as_str()), positions.get(last.as_str()))
            {
                starts.insert(start, i);
                ends.insert(end, i);
            }
        }
        (starts, ends)
    }
}
