use std::collections::{BTreeMap, BTreeSet};

use super::{Label, Scene};

/// A unique stationary label can survive an inserted tick. A uniquely named
/// label within its owning series can also follow a moving datum across slots.
/// Complete the permutation so removed labels can finish fading in displaced slots.
pub(crate) fn shifted_label_slots<'a>(
    previous: impl Iterator<Item = (usize, &'a Label, Option<&'a str>)> + Clone,
    next: &'a Scene,
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
    let current = next.labels.iter().enumerate().map(|(slot, label)| {
        (
            slot,
            label,
            next.label_series.get(&slot).map(String::as_str),
        )
    });
    let mut moves = unique_matches(
        previous
            .clone()
            .map(|(i, label, owner)| (i, key(label, owner))),
        current
            .clone()
            .map(|(i, label, owner)| (i, key(label, owner))),
    );
    let owned = |(i, label, owner): (usize, &'a Label, Option<&'a str>)| {
        owner.map(|id| (i, (id, label.text.as_str(), label.anchor as u8)))
    };
    moves.extend(unique_matches(
        previous.filter_map(owned),
        current.filter_map(owned),
    ));
    moves.retain(|old, slot| old != slot);
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

fn unique_matches<K: Ord>(
    old: impl Iterator<Item = (usize, K)>,
    new: impl Iterator<Item = (usize, K)>,
) -> BTreeMap<usize, usize> {
    let mut old_slots = BTreeMap::new();
    for (slot, key) in old {
        old_slots
            .entry(key)
            .and_modify(|v| *v = None)
            .or_insert(Some(slot));
    }
    let mut matches = BTreeMap::new();
    for (slot, key) in new {
        if let Some(&Some(source)) = old_slots.get(&key) {
            matches
                .entry(source)
                .and_modify(|v| *v = None)
                .or_insert(Some(slot));
        }
    }
    matches
        .into_iter()
        .filter_map(|(old, new)| new.map(|slot| (old, slot)))
        .collect()
}
