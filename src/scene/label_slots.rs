use std::collections::{BTreeMap, BTreeSet};

use super::{Label, Scene};

/// Labels are normally identified by slot. Inserting or removing ticks can
/// shift an otherwise identical label to a new slot. Recognize a unique
/// stationary label by its owner, text, anchor and position; ambiguous labels
/// keep slot semantics. Return a permutation so displaced slots are not lost
/// (in particular, a removed tick may still be fading out).
pub(crate) fn shifted_label_slots<'a>(
    previous: impl Iterator<Item = (usize, &'a Label, Option<&'a str>)> + Clone,
    next: &Scene,
) -> BTreeMap<usize, usize> {
    if previous.clone().count() == next.labels.len()
        && previous.clone().all(|(slot, label, owner)| {
            next.labels.get(slot).is_some_and(|new| {
                label.text == new.text
                    && label.anchor == new.anchor
                    && owner == next.label_series.get(&slot).map(String::as_str)
            })
        })
    {
        return BTreeMap::new();
    }
    fn key<'b>(
        label: &'b Label,
        owner: Option<&'b str>,
    ) -> (&'b str, Option<&'b str>, u8, u64, u64) {
        (
            &label.text,
            owner,
            label.anchor as u8,
            label.x.to_bits(),
            label.y.to_bits(),
        )
    }
    let mut old_slots = BTreeMap::new();
    for (slot, label, owner) in previous {
        old_slots
            .entry(key(label, owner))
            .and_modify(|v| *v = None)
            .or_insert(Some(slot));
    }
    let mut matches = BTreeMap::new();
    for (slot, label) in next.labels.iter().enumerate() {
        let owner = next.label_series.get(&slot).map(String::as_str);
        if let Some(&Some(old)) = old_slots.get(&key(label, owner)) {
            matches
                .entry(old)
                .and_modify(|v| *v = None)
                .or_insert(Some(slot));
        }
    }
    let mut moves: BTreeMap<_, _> = matches
        .into_iter()
        .filter_map(|(old, slot)| slot.filter(|&slot| slot != old).map(|slot| (old, slot)))
        .collect();
    let destinations: BTreeSet<_> = moves.values().copied().collect();
    let starts: Vec<_> = moves
        .keys()
        .filter(|slot| !destinations.contains(slot))
        .copied()
        .collect();
    for start in starts {
        let mut end = start;
        while let Some(next) = moves.get(&end) {
            end = *next;
        }
        moves.insert(end, start);
    }
    moves
}
