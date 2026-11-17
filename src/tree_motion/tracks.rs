use std::collections::{BTreeMap, BTreeSet};

use super::{TreeMotionOptions, TreeMotionPhase, advance};
use crate::motion::{PropertyState, SpringConfig, SpringValue};
use crate::scene::ensure;
use crate::{ChartError, Result};

pub(super) struct Input<K, D, const N: usize> {
    pub key: K,
    pub data: D,
    pub values: [f64; N],
}
#[derive(Clone, Debug)]
pub(super) struct Track<D, const N: usize> {
    pub data: D,
    pub phase: TreeMotionPhase,
    pub instance: u64,
    values: [SpringValue; N],
}
impl<D, const N: usize> Track<D, N> {
    fn settled(&self) -> bool {
        self.values.iter().all(SpringValue::is_settled)
    }
    pub fn states(&self) -> [PropertyState; N] {
        std::array::from_fn(|i| PropertyState {
            value: self.values[i].value(),
            target: self.values[i].target(),
            velocity: self.values[i].velocity(),
        })
    }
}
#[derive(Clone, Debug)]
pub(super) struct Family<K, D, const N: usize> {
    tracks: BTreeMap<K, Track<D, N>>,
    order: Vec<K>,
}
impl<K, D, const N: usize> Default for Family<K, D, N> {
    fn default() -> Self {
        Self {
            tracks: BTreeMap::new(),
            order: Vec::new(),
        }
    }
}
impl<K: Ord + Clone, D, const N: usize> Family<K, D, N> {
    pub fn ordered(&self) -> impl Iterator<Item = (&K, &Track<D, N>)> {
        self.order
            .iter()
            .map(|key| (key, self.tracks.get(key).expect("validated motion order")))
    }
    pub fn is_settled(&self) -> bool {
        self.tracks
            .values()
            .all(|track| track.phase == TreeMotionPhase::Live && track.settled())
    }
    pub fn set_config(&mut self, config: SpringConfig) -> Result<()> {
        for track in self.tracks.values_mut() {
            for spring in &mut track.values {
                spring.set_config(config)?;
            }
        }
        Ok(())
    }
    pub fn retarget(
        &mut self,
        input: Vec<Input<K, D, N>>,
        exit_channel: Option<usize>,
        options: &TreeMotionOptions,
        budget: usize,
        reduced: bool,
        next_instance: &mut u64,
    ) -> Result<()> {
        ensure(input.len() <= budget, "Tree motion target budget exceeded")?;
        let mut live = BTreeSet::new();
        let mut target_order = Vec::with_capacity(input.len());
        for input in input {
            ensure(
                live.insert(input.key.clone()),
                "Tree motion duplicate role owner",
            )?;
            target_order.push(input.key.clone());
            if let Some(track) = self.tracks.get_mut(&input.key) {
                // A revived record keeps both instance and physical velocity;
                // data always changes to the new cohort, even for equal values.
                for (spring, target) in track.values.iter_mut().zip(input.values) {
                    spring.retarget(target)?;
                }
                track.data = input.data;
                track.phase = TreeMotionPhase::Live;
            } else {
                *next_instance = advance(*next_instance, "instance")?;
                let values = input
                    .values
                    .into_iter()
                    .map(|value| {
                        let mut spring = SpringValue::new(value, options.spring)?;
                        spring.set_reduced_motion(reduced);
                        Ok(spring)
                    })
                    .collect::<Result<Vec<_>>>()?
                    .try_into()
                    .map_err(|_| ChartError("Tree motion channel count differs".into()))?;
                self.tracks.insert(
                    input.key,
                    Track {
                        data: input.data,
                        phase: TreeMotionPhase::Live,
                        instance: *next_instance,
                        values,
                    },
                );
            }
        }
        for (key, track) in &mut self.tracks {
            if !live.contains(key) && track.phase == TreeMotionPhase::Live {
                track.phase = TreeMotionPhase::Leaving;
                if let Some(channel) = exit_channel {
                    track.values[channel].retarget(0.)?;
                }
            }
        }
        self.prune();
        self.order = merge(&target_order, &self.order, &self.tracks);
        self.validate(budget)
    }
    fn prune(&mut self) {
        self.tracks
            .retain(|_, track| track.phase == TreeMotionPhase::Live || !track.settled());
        self.order.retain(|key| self.tracks.contains_key(key));
    }
    pub fn tick(&mut self, dt: f64) -> Result<()> {
        for track in self.tracks.values_mut() {
            for spring in &mut track.values {
                spring.tick(dt)?;
            }
        }
        self.prune();
        Ok(())
    }
    pub fn finish(&mut self) {
        for track in self.tracks.values_mut() {
            for spring in &mut track.values {
                spring.finish();
            }
        }
        self.prune();
    }
    pub fn set_reduced(&mut self, reduced: bool) {
        for track in self.tracks.values_mut() {
            for spring in &mut track.values {
                spring.set_reduced_motion(reduced);
            }
        }
        self.prune();
    }
    pub fn validate(&self, budget: usize) -> Result<()> {
        ensure(
            self.tracks.len() <= budget,
            "Tree motion retained union budget exceeded",
        )?;
        let keys = self.order.iter().collect::<BTreeSet<_>>();
        ensure(
            self.order.len() == self.tracks.len()
                && keys.len() == self.order.len()
                && self.tracks.keys().all(|key| keys.contains(key)),
            "Tree motion retained order differs",
        )?;
        ensure(
            self.tracks.values().all(|track| {
                track.values.iter().all(|spring| {
                    [spring.value(), spring.target(), spring.velocity()]
                        .into_iter()
                        .all(f64::is_finite)
                })
            }),
            "Tree motion physical state is nonfinite",
        )
    }
}

// O(n log n), including reorders. Each retained old record is scanned once;
// buckets preserve old exit order without searching/splicing the new vector.
fn merge<K: Ord + Clone, D, const N: usize>(
    target: &[K],
    previous: &[K],
    tracks: &BTreeMap<K, Track<D, N>>,
) -> Vec<K> {
    let mut before: BTreeMap<K, Vec<K>> = BTreeMap::new();
    let mut after: BTreeMap<K, Vec<K>> = BTreeMap::new();
    let mut leading = Vec::new();
    let mut left = None;
    for key in previous {
        let Some(track) = tracks.get(key) else {
            continue;
        };
        if track.phase == TreeMotionPhase::Live {
            if left.is_none() && !leading.is_empty() {
                before.insert(key.clone(), std::mem::take(&mut leading));
            }
            left = Some(key.clone());
        } else if let Some(left) = &left {
            after.entry(left.clone()).or_default().push(key.clone());
        } else {
            leading.push(key.clone());
        }
    }
    let mut order = Vec::with_capacity(tracks.len());
    for key in target {
        if let Some(prefix) = before.remove(key) {
            order.extend(prefix);
        }
        order.push(key.clone());
        if let Some(suffix) = after.remove(key) {
            order.extend(suffix);
        }
    }
    order.extend(leading);
    order
}
