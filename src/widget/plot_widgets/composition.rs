//! Transactional, hidden child staging for the opt-in composition API.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use glory::render::NodeId;

use super::*;

#[derive(Clone, PartialEq)]
struct Snapshot {
    row: PreparedSlot,
    hidden: BTreeSet<String>,
}
struct Record {
    host: Weak<Inner>,
    revision: u64,
    is_label: bool,
    snapshot: Cage<Snapshot>,
    active: Cage<bool>,
    mounted: RefCell<Option<glory::ui::Mounted>>,
    placement: RefCell<BTreeMap<glory::render::NodeId, Placement>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placement {
    parent: Option<glory::render::NodeId>,
    before: Option<glory::render::NodeId>,
    suspended: bool,
}
impl Record {
    fn roots(&self) -> Result<Vec<glory::render::NodeId>> {
        self.mounted
            .borrow()
            .as_ref()
            .map(|mounted| mounted.roots().to_vec())
            .ok_or_else(|| ChartError("composition child retired".into()))
    }
    fn retire(&self) {
        let mounted = self.mounted.borrow_mut().take();
        let removed = self.placement.replace(BTreeMap::new());
        if let Some(host) = self.host.upgrade() {
            for record in host.live_records() {
                for link in record.placement.borrow_mut().values_mut() {
                    while let Some(next) = link.before.and_then(|id| removed.get(&id)) {
                        link.before = next.before;
                    }
                }
            }
        }
        drop(mounted);
    }
}

struct Inner {
    parent: glory::ui::WeakScope,
    root: RefCell<Option<glory::ui::Mounted>>,
    records: RefCell<BTreeMap<String, Rc<Record>>>,
    all_records: RefCell<Vec<Weak<Record>>>,
    generation: Cell<u64>,
    alive: Cell<bool>,
    busy: Cell<bool>,
    order: RefCell<Vec<String>>,
    pending: Cell<usize>,
    profile: Profile,
    factory: WidgetRenderer<PlotWidgetBuildContext>,
    mask_owner: Option<Rc<str>>,
}
impl Inner {
    fn live_records(&self) -> Vec<Rc<Record>> {
        let mut records = self.all_records.borrow_mut();
        records.retain(|record| {
            record
                .upgrade()
                .is_some_and(|record| record.mounted.borrow().is_some())
        });
        records.iter().filter_map(Weak::upgrade).collect()
    }
}
struct Busy<'a>(&'a Cell<bool>);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl<'a> Busy<'a> {
    fn enter(value: &'a Cell<bool>) -> Result<Self> {
        ensure(
            !value.replace(true),
            "reentrant composition preparation or commit",
        )?;
        Ok(Self(value))
    }
}

/// A clone shares the same scoped composition host, never an independent model.
#[derive(Clone)]
pub(crate) struct CompositionManager(Rc<Inner>);

pub(crate) struct CompositionStage {
    owner: Weak<Inner>,
    generation: u64,
    rows: Vec<PreparedSlot>,
    hidden: BTreeSet<String>,
    targets: BTreeMap<String, Rc<Record>>,
    lease: Rc<NewRecords>,
    committed: Cell<bool>,
}
/// Fallible placement has completed, but no accepted data has changed yet.
/// Dropping an unadopted placement restores the previous managed roots.
pub(crate) struct CompositionPlacement {
    manager: CompositionManager,
    stage: Rc<CompositionStage>,
    records: BTreeMap<NodeId, Rc<Record>>,
    previous: BTreeMap<NodeId, Placement>,
    undo: Vec<(NodeId, Placement)>,
    finished: Cell<bool>,
}
/// Keep replaced scopes alive until every coordinated data Cage is current.
pub(crate) struct CompositionRetirement(Vec<Rc<Record>>);
impl Drop for CompositionRetirement {
    fn drop(&mut self) {
        for record in &self.0 {
            record.retire();
        }
    }
}
impl CompositionPlacement {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure(
            !self.finished.get(),
            "composition placement already finalized",
        )?;
        self.manager.validate_stage(&self.stage)
    }
    pub(crate) fn rollback(&self) -> Result<()> {
        if self.finished.replace(true) {
            return Ok(());
        }
        for (root, previous) in self.undo.iter().rev() {
            let record = &self.records[root];
            let mounted = record.mounted.borrow();
            let mounted = mounted
                .as_ref()
                .ok_or_else(|| ChartError("composition rollback child retired".into()))?;
            mounted
                .scope()
                .place(*root, previous.parent, previous.before)
                .map_err(|e| ChartError(e.to_string()))?;
            if previous.suspended {
                mounted.suspend().map_err(|e| ChartError(e.to_string()))?;
            }
        }
        for record in self.manager.0.live_records() {
            *record.placement.borrow_mut() = record
                .roots()?
                .into_iter()
                .filter_map(|root| self.previous.get(&root).copied().map(|p| (root, p)))
                .collect();
        }
        Ok(())
    }
    /// Call only after all coordinated placements have passed validate(). No
    /// callbacks, backend writes, cleanup, or UI pumping occur in this phase.
    pub(crate) fn adopt_with(&self, adopt: &mut dyn FnMut()) -> CompositionRetirement {
        assert!(
            !self.finished.replace(true),
            "composition placement finalized twice"
        );
        self.manager.adopt_placed(&self.stage, adopt)
    }
}
impl Drop for CompositionPlacement {
    fn drop(&mut self) {
        let _ = self.rollback();
    }
}
struct NewRecords {
    records: Vec<Rc<Record>>,
    adopted: Cell<bool>,
    reservation: Rc<Reservation>,
}
const MAX_PENDING_CHILDREN: usize = 2 * MAX_PLOT_WIDGET_SLOTS;
struct Reservation {
    owner: Weak<Inner>,
    count: Cell<usize>,
}
impl Reservation {
    fn reserve(&self) -> Result<()> {
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| ChartError("composition owner retired".into()))?;
        ensure(
            owner.pending.get() < MAX_PENDING_CHILDREN,
            "pending composition Widget budget exceeded",
        )?;
        owner.pending.set(owner.pending.get() + 1);
        self.count.set(self.count.get() + 1);
        Ok(())
    }
    fn release(&self) {
        let count = self.count.replace(0);
        if let Some(owner) = self.owner.upgrade() {
            owner.pending.set(
                owner
                    .pending
                    .get()
                    .checked_sub(count)
                    .expect("balanced composition reservations"),
            );
        }
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.release();
    }
}
impl Drop for NewRecords {
    fn drop(&mut self) {
        if !self.adopted.get() {
            for record in &self.records {
                record.retire();
            }
        }
    }
}
impl CompositionStage {
    /// Retargeted stages share the adoption lease. It flips only after all
    /// placement gates succeed, before publishing any accepted row Cage.
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn was_adopted(&self) -> bool {
        self.lease.adopted.get()
    }
    #[cfg(test)]
    pub(crate) fn record_roots(&self, id: &str) -> Result<Vec<glory::render::NodeId>> {
        self.targets
            .get(id)
            .ok_or_else(|| ChartError("unknown composition Widget datum".into()))?
            .roots()
    }
    pub(crate) fn rows(&self) -> &[PreparedSlot] {
        &self.rows
    }
    pub(crate) fn validate(&self) -> Result<()> {
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| ChartError("composition owner retired".into()))?;
        ensure(
            owner.alive.get() && owner.generation.get() == self.generation && !self.committed.get(),
            "composition stage is stale or retired",
        )
    }
}
impl CompositionManager {
    pub(crate) fn new(
        scope: &Scope,
        profile: Profile,
        factory: PlotWidgetFactory,
        style: Vary<Style>,
        mask_owner: Option<Rc<str>>,
    ) -> Result<Self> {
        let parent = Rc::new(RefCell::new(None));
        let capture = parent.clone();
        let mounted = scope
            .mount_with(|| {
                glory::ui::view()
                    .class("kartis-plot-widgets kartis-composition-widgets")
                    .style(style)
                    .fill(glory::ui::widget(move |scope| {
                        *capture.borrow_mut() = Some(scope.downgrade());
                        Ok(())
                    }))
            })
            .map_err(|error| ChartError(error.to_string()))?;
        let parent = parent
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("composition parent was not mounted".into()))?;
        let manager = Self(Rc::new(Inner {
            parent,
            root: RefCell::new(Some(mounted)),
            records: RefCell::new(BTreeMap::new()),
            all_records: RefCell::new(vec![]),
            generation: Cell::new(0),
            alive: Cell::new(true),
            busy: Cell::new(false),
            order: RefCell::new(vec![]),
            pending: Cell::new(0),
            profile,
            factory: WidgetRenderer::new(Some(factory)),
            mask_owner,
        }));
        let weak = Rc::downgrade(&manager.0);
        scope
            .on_cleanup(move || {
                if let Some(inner) = weak.upgrade() {
                    Self(inner).clear();
                }
            })
            .map_err(|error| ChartError(error.to_string()))?;
        Ok(manager)
    }

    pub(crate) fn prepare(
        &self,
        rows: Vec<PreparedSlot>,
        hidden: BTreeSet<String>,
    ) -> Result<Rc<CompositionStage>> {
        ensure(self.0.alive.get(), "composition owner retired")?;
        let _busy = Busy::enter(&self.0.busy)?;
        ensure(
            rows.len() <= MAX_PLOT_WIDGET_SLOTS,
            "plot Widget slot budget exceeded",
        )?;
        let generation = self.0.generation.get();
        ensure(generation < u64::MAX, "composition generation exhausted")?;
        let previous = self.0.records.borrow().clone();
        // A factory may assume the source variant it received at construction.
        // Reject a known retained kind conflict before constructing any other
        // candidate children, even when the conflicting row appears last.
        for row in &rows {
            if let Some(record) = previous
                .get(&row.key)
                .filter(|record| record.revision == row.revision)
            {
                ensure(
                    record.is_label == row.is_label(),
                    "composition source kind changed without a new revision",
                )?;
            }
        }
        let parent = self
            .0
            .parent
            .upgrade()
            .ok_or_else(|| ChartError("composition parent retired".into()))?;
        let mut targets = BTreeMap::new();
        let mut new_records = vec![];
        let reservation = Rc::new(Reservation {
            owner: Rc::downgrade(&self.0),
            count: Cell::new(0),
        });
        let staged = (|| -> Result<()> {
            for row in &rows {
                let frame = row.frame(&hidden);
                profile_style(self.0.profile, &clip_style(&frame, row.relative_to))?;
                profile_style(
                    self.0.profile,
                    &content_style(&frame, row.relative_to.is_some()),
                )?;
                ensure(
                    !targets.contains_key(&row.key),
                    "duplicate composition Widget datum",
                )?;
                let record = if let Some(record) = previous
                    .get(&row.key)
                    .filter(|record| record.revision == row.revision)
                {
                    ensure(
                        record.is_label == row.is_label(),
                        "composition source kind changed without a new revision",
                    )?;
                    record.clone()
                } else {
                    reservation.reserve()?;
                    let record = self.build_record(&parent, row.clone(), hidden.clone())?;
                    new_records.push(record.clone());
                    record
                };
                targets.insert(row.key.clone(), record);
                ensure(
                    self.0.alive.get() && self.0.generation.get() == generation,
                    "composition changed during child construction",
                )?;
            }
            Ok(())
        })();
        if let Err(error) = staged {
            for record in &new_records {
                record.retire();
            }
            return Err(error);
        }
        Ok(Rc::new(CompositionStage {
            owner: Rc::downgrade(&self.0),
            generation,
            rows,
            hidden,
            targets,
            lease: Rc::new(NewRecords {
                records: new_records,
                adopted: Cell::new(false),
                reservation,
            }),
            committed: Cell::new(false),
        }))
    }

    pub(crate) fn retarget(
        &self,
        stage: &Rc<CompositionStage>,
        rows: Vec<PreparedSlot>,
        hidden: BTreeSet<String>,
    ) -> Result<Rc<CompositionStage>> {
        self.validate_stage(stage)?;
        ensure(
            rows.len() == stage.rows.len()
                && rows.iter().zip(&stage.rows).all(|(a, b)| {
                    a.key == b.key && a.revision == b.revision && a.is_label() == b.is_label()
                }),
            "composition retarget changed keys, revisions or source kinds",
        )?;
        Ok(Rc::new(CompositionStage {
            owner: stage.owner.clone(),
            generation: stage.generation,
            rows,
            hidden,
            targets: stage.targets.clone(),
            lease: stage.lease.clone(),
            committed: Cell::new(false),
        }))
    }

    fn validate_stage(&self, stage: &CompositionStage) -> Result<()> {
        ensure(
            Weak::ptr_eq(&stage.owner, &Rc::downgrade(&self.0)),
            "composition stage belongs to a different host",
        )?;
        stage.validate()
    }

    pub(crate) fn commit(&self, stage: &Rc<CompositionStage>) -> Result<()> {
        self.validate_stage(stage)?;
        let parent = self
            .0
            .parent
            .upgrade()
            .and_then(|scope| scope.construction_parent())
            .ok_or_else(|| ChartError("composition parent retired".into()))?;
        let mut targets = BTreeMap::new();
        let mut next = None;
        for row in stage.rows.iter().rev() {
            let roots = stage.targets[&row.key].roots()?;
            targets.insert(row.key.clone(), (parent, next));
            next = roots.first().copied();
        }
        self.commit_placed(stage, &targets)
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn commit_with_adopt(
        &self,
        stage: &Rc<CompositionStage>,
        adopt: &mut dyn FnMut(),
    ) -> Result<()> {
        self.validate_stage(stage)?;
        let parent = self
            .0
            .parent
            .upgrade()
            .and_then(|scope| scope.construction_parent())
            .ok_or_else(|| ChartError("composition parent retired".into()))?;
        let mut targets = BTreeMap::new();
        let mut next = None;
        for row in stage.rows.iter().rev() {
            let roots = stage.targets[&row.key].roots()?;
            targets.insert(row.key.clone(), (parent, next));
            next = roots.first().copied();
        }
        self.commit_placed_with_adopt(stage, &targets, adopt)
    }

    /// Place records into registered live paint/group parents. Callers retain
    /// old parents and sibling anchors until this transaction succeeds, and
    /// never move managed roots independently of this manager.
    pub(crate) fn commit_placed(
        &self,
        stage: &Rc<CompositionStage>,
        targets: &BTreeMap<String, (NodeId, Option<NodeId>)>,
    ) -> Result<()> {
        self.commit_placed_with_adopt(stage, targets, &mut || {})
    }

    /// The infallible hook only revises another preflighted data Cage; it does
    /// not execute application code or pump UI effects.
    pub(crate) fn commit_placed_with_adopt(
        &self,
        stage: &Rc<CompositionStage>,
        targets: &BTreeMap<String, (NodeId, Option<NodeId>)>,
        adopt: &mut dyn FnMut(),
    ) -> Result<()> {
        let placement = self.place_placed(stage, targets)?;
        // Legacy commits also reject reentry from retiring child callbacks.
        // Coordinated callers defer retirement until their shared barrier.
        self.with_commit_guard(|| {
            placement.validate()?;
            drop(placement.adopt_with(adopt));
            Ok(())
        })
    }

    /// Legacy hosts finalize a single manager under its existing reentry
    /// barrier, including cleanup of children retired by that publication.
    pub(crate) fn with_commit_guard<T>(&self, action: impl FnOnce() -> Result<T>) -> Result<T> {
        let _busy = Busy::enter(&self.0.busy)?;
        action()
    }

    pub(crate) fn place_placed(
        &self,
        stage: &Rc<CompositionStage>,
        targets: &BTreeMap<String, (NodeId, Option<NodeId>)>,
    ) -> Result<CompositionPlacement> {
        let _busy = Busy::enter(&self.0.busy)?;
        self.validate_stage(stage)?;
        ensure(
            targets.len() == stage.targets.len()
                && targets.keys().all(|id| stage.targets.contains_key(id)),
            "composition placement keys do not match stage",
        )?;
        let mut records = BTreeMap::new();
        let mut links = BTreeMap::new();
        for record in self.0.live_records() {
            for (root, placement) in record.placement.borrow().iter() {
                records.insert(*root, record.clone());
                links.insert(*root, *placement);
            }
        }
        let previous_links = links.clone();
        let mut desired = BTreeMap::new();
        let mut row_roots = vec![];
        for row in &stage.rows {
            let record = &stage.targets[&row.key];
            let (parent, mut before) = targets[&row.key];
            let mounted = record.mounted.borrow();
            let mounted = mounted
                .as_ref()
                .ok_or_else(|| ChartError("composition child retired".into()))?;
            let scope = mounted.scope();
            scope
                .contains_node(parent, parent)
                .map_err(|error| ChartError(error.to_string()))?;
            let roots = mounted.roots();
            for root in roots {
                ensure(
                    !scope
                        .contains_node(*root, parent)
                        .map_err(|error| ChartError(error.to_string()))?,
                    "composition placement creates a cycle",
                )?;
            }
            for root in roots.iter().rev() {
                if let Some(anchor) = before {
                    scope
                        .contains_node(anchor, anchor)
                        .map_err(|error| ChartError(error.to_string()))?;
                }
                desired.insert(
                    *root,
                    Placement {
                        parent: Some(parent),
                        before,
                        suspended: false,
                    },
                );
                before = Some(*root);
            }
            row_roots.extend(roots.iter().copied());
        }
        for placement in desired.values() {
            if let Some(before) = placement.before {
                ensure(
                    !records.contains_key(&before) || desired.contains_key(&before),
                    "composition sibling anchor is being retired",
                )?;
                if let Some(anchor) = desired.get(&before) {
                    ensure(
                        anchor.parent == placement.parent,
                        "composition sibling belongs to another target parent",
                    )?;
                }
            }
        }
        // Anchor records must be moved first even when packet order differs
        // from Scene paint order. Detect cycles before touching any live root.
        fn visit(
            root: NodeId,
            desired: &BTreeMap<NodeId, Placement>,
            marks: &mut BTreeMap<NodeId, u8>,
            order: &mut Vec<NodeId>,
        ) -> Result<()> {
            if marks.get(&root) == Some(&2) {
                return Ok(());
            }
            ensure(
                marks.get(&root) != Some(&1),
                "cyclic composition sibling anchors",
            )?;
            marks.insert(root, 1);
            if let Some(before) = desired[&root].before.filter(|id| desired.contains_key(id)) {
                visit(before, desired, marks, order)?;
            }
            marks.insert(root, 2);
            order.push(root);
            Ok(())
        }
        let mut order = vec![];
        let mut marks = BTreeMap::new();
        for root in row_roots {
            visit(root, &desired, &mut marks, &mut order)?;
        }
        // Simulated sibling links capture the position immediately before each
        // move, not merely last frame's anchor (which may already have moved).
        fn detach(links: &mut BTreeMap<NodeId, Placement>, root: NodeId) {
            let previous = links[&root];
            for (id, link) in links.iter_mut() {
                if *id != root && link.parent == previous.parent && link.before == Some(root) {
                    link.before = previous.before;
                }
            }
        }
        fn insert(links: &mut BTreeMap<NodeId, Placement>, root: NodeId, next: Placement) {
            detach(links, root);
            for (id, link) in links.iter_mut() {
                if *id != root && link.parent == next.parent && link.before == next.before {
                    link.before = Some(root);
                }
            }
            links.insert(root, next);
        }
        let mut undo: Vec<(NodeId, Placement)> = vec![];
        for root in order {
            let target = desired[&root];
            let old = links[&root];
            if old == target {
                continue;
            }
            let record = &records[&root];
            let scope = record
                .mounted
                .borrow()
                .as_ref()
                .ok_or_else(|| ChartError("composition child retired".into()))?
                .scope()
                .clone();
            if let Err(error) = scope.place(root, target.parent, target.before) {
                let mut rollback_error = None;
                for (moved, position) in undo.iter().rev().copied() {
                    let record = &records[&moved];
                    let mounted = record.mounted.borrow();
                    let mounted = mounted.as_ref().expect("live rollback record");
                    if let Err(error) =
                        mounted
                            .scope()
                            .place(moved, position.parent, position.before)
                    {
                        rollback_error = Some(error.to_string());
                        break;
                    }
                    if position.suspended
                        && let Err(error) = mounted.suspend()
                    {
                        rollback_error = Some(error.to_string());
                        break;
                    }
                }
                return Err(ChartError(rollback_error.map_or_else(
                    || error.to_string(),
                    |rollback| {
                        format!(
                            "composition placement failed: {error}; rollback failed: {rollback}"
                        )
                    },
                )));
            }
            undo.push((root, old));
            insert(&mut links, root, target);
        }
        // Suspended children still occupy semantic sibling positions. Keep
        // every pending lease's links current, not only this stage's rows.
        for record in self.0.live_records() {
            *record.placement.borrow_mut() = record
                .roots()?
                .into_iter()
                .map(|root| (root, links[&root]))
                .collect();
        }
        Ok(CompositionPlacement {
            manager: self.clone(),
            stage: stage.clone(),
            records,
            previous: previous_links,
            undo,
            finished: Cell::new(false),
        })
    }

    fn adopt_placed(
        &self,
        stage: &Rc<CompositionStage>,
        adopt: &mut dyn FnMut(),
    ) -> CompositionRetirement {
        let previous = self.0.records.replace(stage.targets.clone());
        *self.0.order.borrow_mut() = stage.rows.iter().map(|row| row.key.clone()).collect();
        self.0.generation.set(stage.generation + 1);
        stage.committed.set(true);
        stage.lease.adopted.set(true);
        stage.lease.reservation.release();
        adopt();
        for row in &stage.rows {
            let record = &stage.targets[&row.key];
            let next = Snapshot {
                row: row.clone(),
                hidden: stage.hidden.clone(),
            };
            if record.snapshot.with(|old| old != &next) {
                record.snapshot.revise(|old| *old = next);
            }
            if !record.active.get_untracked() {
                record.active.revise(|value| *value = true);
            }
        }
        // Explicit disposal defeats strong references held by stale queued
        // stages; those references must not keep old controls visible or live.
        let mut retiring = Vec::new();
        for (id, record) in previous {
            if !stage
                .targets
                .get(&id)
                .is_some_and(|next| Rc::ptr_eq(next, &record))
            {
                retiring.push(record);
            }
        }
        CompositionRetirement(retiring)
    }

    pub(crate) fn raise(&self) -> Result<()> {
        ensure(self.0.alive.get(), "composition owner retired")?;
        self.0
            .root
            .borrow()
            .as_ref()
            .ok_or_else(|| ChartError("composition root retired".into()))?
            .place_before(None)
            .map_err(|error| ChartError(error.to_string()))
    }

    pub(crate) fn clear(&self) {
        if !self.0.alive.replace(false) {
            return;
        }
        self.0.factory.clear();
        let records = std::mem::take(&mut *self.0.records.borrow_mut());
        let root = self.0.root.borrow_mut().take();
        for record in records.into_values() {
            record.retire();
        }
        drop(root);
    }

    fn build_record(
        &self,
        parent: &Scope,
        row: PreparedSlot,
        hidden: BTreeSet<String>,
    ) -> Result<Rc<Record>> {
        let capture = Rc::new(RefCell::new(None));
        let observed = capture.clone();
        let factory = self.0.factory.clone();
        let mask_owner = self.0.mask_owner.clone();
        let revision = row.revision;
        let is_label = row.is_label();
        let mounted = parent
            .mount_with(|| {
                glory::ui::widget(move |scope| {
                    let snapshot = scope.cage(Snapshot {
                        row: row.clone(),
                        hidden: hidden.clone(),
                    });
                    let active = scope.cage(false);
                    *observed.borrow_mut() = Some((snapshot, active));
                    let frame = scope.bond(move || {
                        snapshot.with(|snapshot| snapshot.row.frame(&snapshot.hidden))
                    });
                    let payload =
                        scope.bond(move || snapshot.with(|snapshot| snapshot.row.payload.clone()));
                    let owner = mask_owner.clone();
                    let id = row.key.clone();
                    glory::ui::view()
                        .class(scope.bond(move || {
                            snapshot.with(|snapshot| snapshot.row.clip_class(owner.as_deref()))
                        }))
                        .style(scope.bond(move || {
                            snapshot.with(|snapshot| {
                                let mut style = clip_style(
                                    &snapshot.row.frame(&snapshot.hidden),
                                    snapshot.row.relative_to,
                                );
                                if !active.get() {
                                    style.display = Some(Display::None);
                                }
                                style
                            })
                        }))
                        .semantics(scope.bond(move || {
                            frame.with(|frame| {
                                Some(Box::new(Semantics {
                                    hidden: !active.get() || frame.visible().is_none(),
                                    inert: !active.get() || !frame.interactive(),
                                    ..Default::default()
                                }))
                            })
                        }))
                        .fill(
                            glory::ui::view()
                                .class("kartis-plot-widget-slot")
                                .style(scope.bond(move || {
                                    frame.with(|frame| {
                                        snapshot.with(|snapshot| {
                                            content_style(frame, snapshot.row.relative_to.is_some())
                                        })
                                    })
                                }))
                                .fill(FactoryWidget {
                                    factory: factory.clone(),
                                    context: crate::SceneWidgetContext {
                                        key: id,
                                        revision,
                                        frame,
                                    },
                                    payload,
                                    child: None,
                                }),
                        )
                        .show_in(scope)
                })
            })
            .map_err(|error| ChartError(error.to_string()))?;
        mounted
            .suspend()
            .map_err(|error| ChartError(error.to_string()))?;
        let (snapshot, active) = capture
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("composition child did not initialize".into()))?;
        let roots = mounted.roots();
        let placement = roots
            .iter()
            .copied()
            .map(|root| {
                (
                    root,
                    Placement {
                        parent: mounted.scope().construction_parent(),
                        before: roots
                            .iter()
                            .position(|id| *id == root)
                            .and_then(|i| roots.get(i + 1))
                            .copied(),
                        suspended: true,
                    },
                )
            })
            .collect();
        // New wrappers append to the persistent staging parent even while
        // suspended. Update the former last sibling across all pending stages.
        for record in self.0.live_records() {
            for link in record.placement.borrow_mut().values_mut() {
                if link.parent == mounted.scope().construction_parent() && link.before.is_none() {
                    link.before = roots.first().copied();
                }
            }
        }
        let record = Rc::new(Record {
            host: Rc::downgrade(&self.0),
            revision,
            is_label,
            snapshot,
            active,
            mounted: RefCell::new(Some(mounted)),
            placement: RefCell::new(placement),
        });
        self.0.all_records.borrow_mut().push(Rc::downgrade(&record));
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use glory::render::memory::{Memory, Subtree};
    use glory::render::{
        Capabilities, Error as RenderError, Event, EventKind, Input, Mutation, Props, Query,
        QueryToken, Receipt, Render,
    };

    use super::*;

    #[derive(Clone, PartialEq)]
    struct OrderedData(f64);
    fn mark_rows(
        ids: impl IntoIterator<Item = String>,
        value: f64,
        revision: u64,
    ) -> Vec<PreparedSlot> {
        let mut scene = crate::ChartConfig {
            width: 100.,
            height: 100.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        let mut layout = PlotWidgetsLayout::default();
        let mut data = BTreeMap::new();
        for id in ids {
            let mut mark = Mark::new(
                &id,
                Geometry::Circle {
                    x: 50.,
                    y: 50.,
                    radius: 5.,
                },
                crate::Color(20, 40, 80, 255),
            );
            mark.series = "series".into();
            mark.value = value;
            scene.marks.push(mark);
            layout.slots.push(PlotWidgetSlot {
                mark_id: id.clone(),
                revision,
                placement: PlotWidgetPlacement::Rect(LayoutRect {
                    x: 40.,
                    y: 40.,
                    width: 20.,
                    height: 20.,
                }),
            });
            data.insert(id, OrderedData(value));
        }
        prepare_in_space_packet(
            &scene,
            &PlotWidgetsPacket::typed(PlotWidgetsData { layout, data }),
            Profile::HTML,
            SlotSpace {
                plot: None,
                transform: ZoomTransform::default(),
                responsive: false,
                require_current_data: true,
            },
            |mark| Ok(crate::crosshair::mark_center(mark)),
            |mark| Some(mark.value),
        )
        .unwrap()
    }
    fn ordered_rows(value: f64, revision: u64) -> Vec<PreparedSlot> {
        mark_rows(["a", "b", "c"].map(str::to_owned), value, revision)
    }
    fn scene_rows(
        targets: &[(&str, crate::SceneWidgetTarget)],
        value: f64,
        revision: u64,
    ) -> Vec<PreparedSlot> {
        let mut scene = crate::ChartConfig {
            width: 100.,
            height: 100.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        for id in ["a", "b"] {
            let mut mark = Mark::new(
                id,
                Geometry::Circle {
                    x: 50.,
                    y: 50.,
                    radius: 5.,
                },
                crate::Color(20, 40, 80, 255),
            );
            mark.value = value;
            mark.series = "series".into();
            scene.marks.push(mark);
        }
        for index in 0..2 {
            scene.labels.push(crate::Label {
                rotation: 0.,
                x: 40.,
                y: 55.,
                text: format!("label-{index}"),
                color: crate::Color(20, 40, 80, 255),
                size: 14.,
                anchor: crate::TextAnchor::Start,
            });
            scene.label_series.insert(index, "series".into());
        }
        let packet = PlotWidgetsPacket::scene_composition(crate::SceneWidgetsComposition {
            widgets: crate::SceneWidgetsData {
                layout: crate::SceneWidgetsLayout {
                    plot: None,
                    slots: targets
                        .iter()
                        .map(|(key, target)| crate::SceneWidgetSlot {
                            key: (*key).into(),
                            target: target.clone(),
                            revision,
                            placement: PlotWidgetPlacement::Rect(LayoutRect {
                                x: 40.,
                                y: 40.,
                                width: 20.,
                                height: 20.,
                            }),
                        })
                        .collect(),
                },
                data: targets
                    .iter()
                    .map(|(key, _)| ((*key).into(), OrderedData(value)))
                    .collect(),
            },
            replace_ink: BTreeSet::new(),
        });
        prepare_in_space_packet(
            &scene,
            &packet,
            Profile::HTML,
            SlotSpace {
                plot: None,
                transform: ZoomTransform::default(),
                responsive: false,
                require_current_data: true,
            },
            |mark| Ok(crate::crosshair::mark_center(mark)),
            |mark| Some(mark.value),
        )
        .unwrap()
    }
    fn scene_manager(
        app: &glory::App,
        calls: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
    ) -> CompositionManager {
        CompositionManager::new(
            app.scope(),
            app.scope().style_profile().unwrap(),
            scene_composition_factory(
                move |context: crate::SceneWidgetCompositionContext<OrderedData>| {
                    calls.set(calls.get() + 1);
                    if context.key == "label" && context.revision == 4 {
                        return Err(ChartError("mixed Label factory rejected".into()));
                    }
                    let reject = context.key == "label" && context.revision == 5;
                    let frame = context.frame;
                    let key = context.key;
                    let drops = drops.clone();
                    Ok(glory::ui::widget(move |scope| {
                        let drops = drops.clone();
                        scope.on_cleanup(move || drops.set(drops.get() + 1))?;
                        let count = scope.cage(0u32);
                        let focused = scope.cage(false);
                        glory::ui::button()
                            .class(format!("mixed-{key}"))
                            .label(scope.bond(move || {
                                frame.with(|value| {
                                    let source = match &value.frame {
                                        crate::SceneWidgetFrame::Mark(frame) => {
                                            format!("mark:{}", frame.mark.id)
                                        }
                                        crate::SceneWidgetFrame::Label(frame) => {
                                            format!(
                                                "label:{}:{}",
                                                frame.label_index, frame.label.text
                                            )
                                        }
                                    };
                                    format!(
                                        "{source}:{}:{}:{}",
                                        value.data.0,
                                        count.get(),
                                        focused.get()
                                    )
                                })
                            }))
                            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                            .on(EventKind::FocusChanged, move |event| {
                                if let Event::FocusChanged(value) = event {
                                    focused.revise(|v| *v = value);
                                }
                            })
                            .show_in(scope)?;
                        if reject {
                            return Err(
                                RenderError::InvalidProps("mixed Label build rejected").into()
                            );
                        }
                        Ok(())
                    }))
                },
            ),
            Style::new().into(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn mixed_label_retarget_and_failed_candidates_keep_real_children_and_state() {
        let app = glory::App::memory();
        let calls = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let manager = scene_manager(&app, calls.clone(), drops.clone());
        let targets = [
            ("mark", crate::SceneWidgetTarget::Mark("a".into())),
            ("label", crate::SceneWidgetTarget::Label(0)),
        ];
        let first = manager
            .prepare(scene_rows(&targets, 10., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&first).unwrap();
        app.flush().unwrap();
        let mark = find(&app, "mixed-mark");
        let label = find(&app, "mixed-label");
        app.dispatch(label, Event::Activate).unwrap();
        app.dispatch(label, Event::FocusChanged(true)).unwrap();
        let targets = [
            ("mark", crate::SceneWidgetTarget::Mark("a".into())),
            ("label", crate::SceneWidgetTarget::Label(1)),
        ];
        let second = manager
            .prepare(scene_rows(&targets, 20., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&second).unwrap();
        app.flush().unwrap();
        assert_eq!(find(&app, "mixed-mark"), mark);
        assert_eq!(find(&app, "mixed-label"), label);
        assert_eq!(button_label(&app, mark), "mark:a:20:0:false");
        assert_eq!(button_label(&app, label), "label:1:label-1:20:1:true");
        assert_eq!(calls.get(), 2);
        assert_eq!(drops.get(), 0);
        let before = live_tree(&app);
        assert!(
            manager
                .prepare(scene_rows(&targets, 30., 4), BTreeSet::new())
                .is_err()
        );
        app.flush().unwrap();
        assert_eq!(live_tree(&app), before);
        assert_eq!(drops.get(), 1, "the preceding tentative Mark is disposed");
        assert!(
            manager
                .prepare(scene_rows(&targets, 40., 5), BTreeSet::new())
                .is_err()
        );
        app.flush().unwrap();
        assert_eq!(live_tree(&app), before);
        assert_eq!(
            drops.get(),
            3,
            "both real failed-build candidate scopes retire"
        );
        assert_eq!(find(&app, "mixed-label"), label);
        assert_eq!(button_label(&app, label), "label:1:label-1:20:1:true");
        manager.clear();
        app.flush().unwrap();
        assert_eq!(drops.get(), 5, "accepted children retire exactly once");
    }

    #[test]
    fn retained_source_kind_conflict_rejects_before_other_candidate_factories() {
        let app = glory::App::memory();
        let calls = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let manager = scene_manager(&app, calls.clone(), drops.clone());
        let first = manager
            .prepare(
                scene_rows(
                    &[("shared", crate::SceneWidgetTarget::Mark("a".into()))],
                    1.,
                    0,
                ),
                BTreeSet::new(),
            )
            .unwrap();
        manager.commit(&first).unwrap();
        app.flush().unwrap();
        let before = live_tree(&app);
        let mixed = [
            ("new-first", crate::SceneWidgetTarget::Mark("b".into())),
            ("shared", crate::SceneWidgetTarget::Label(0)),
        ];
        assert!(
            manager
                .prepare(scene_rows(&mixed, 2., 0), BTreeSet::new())
                .is_err()
        );
        assert_eq!(
            calls.get(),
            1,
            "late kind conflict is a whole-stage preflight"
        );
        assert_eq!(live_tree(&app), before);
        let old_kind = manager
            .prepare(
                scene_rows(
                    &[("shared", crate::SceneWidgetTarget::Mark("a".into()))],
                    3.,
                    0,
                ),
                BTreeSet::new(),
            )
            .unwrap();
        assert!(
            manager
                .retarget(
                    &old_kind,
                    scene_rows(&[("shared", crate::SceneWidgetTarget::Label(0))], 3., 0),
                    BTreeSet::new(),
                )
                .is_err()
        );
        assert_eq!(live_tree(&app), before);
        let changed = manager
            .prepare(
                scene_rows(&[("shared", crate::SceneWidgetTarget::Label(0))], 4., 1),
                BTreeSet::new(),
            )
            .unwrap();
        manager.commit(&changed).unwrap();
        app.flush().unwrap();
        assert_eq!(calls.get(), 2);
        assert_eq!(drops.get(), 1);
        assert_eq!(
            button_label(&app, find(&app, "mixed-shared")),
            "label:0:label-0:4:0:false"
        );
    }
    fn ordered_manager(
        app: &glory::App,
        calls: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
    ) -> CompositionManager {
        use glory::Widget;
        CompositionManager::new(
            app.scope(),
            app.scope().style_profile().unwrap(),
            composition_factory(move |context: PlotWidgetCompositionContext<OrderedData>| {
                calls.set(calls.get() + 1);
                if context.revision == 4 {
                    return Err(ChartError("ordered factory rejected".into()));
                }
                let frame = context.frame;
                let id = context.mark_id;
                let drops = drops.clone();
                let revision = context.revision;
                Ok(glory::ui::widget(move |scope| {
                    let drops = drops.clone();
                    scope.on_cleanup(move || drops.set(drops.get() + 1))?;
                    let count = scope.cage(0usize);
                    let focused = scope.cage(false);
                    glory::ui::button()
                        .class(format!("ordered-{id}"))
                        .label(scope.bond(move || {
                            frame.with(|frame| {
                                format!(
                                    "{}:{} clicks={} focus={}",
                                    frame.data.0,
                                    frame.frame.mark.value,
                                    count.get(),
                                    focused.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                        .on(EventKind::FocusChanged, move |event| {
                            if let Event::FocusChanged(value) = event {
                                focused.revise(|v| *v = value)
                            }
                        })
                        .show_in(scope)?;
                    if revision == 5 {
                        return Err(
                            RenderError::InvalidProps("ordered child build rejected").into()
                        );
                    }
                    Ok(())
                }))
            }),
            Style::new().into(),
            None,
        )
        .unwrap()
    }
    fn named_parent(app: &glory::App, name: &str, opacity: f64) -> glory::ui::Mounted {
        app.scope()
            .mount_with(|| {
                glory::ui::view()
                    .name(name.to_owned())
                    .style(Style::new().opacity(opacity))
            })
            .unwrap()
    }
    fn find(app: &glory::App, class: &str) -> NodeId {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            if node
                .descriptor
                .view()
                .classes
                .iter()
                .any(|value| value == class)
            {
                return id;
            }
            pending.extend(node.children.iter().copied());
        }
        panic!("missing test node {class}")
    }
    fn button_label(app: &glory::App, id: NodeId) -> String {
        match app.tree().node(id).unwrap().descriptor.view().props {
            Props::Button(value) => value.label.clone(),
            _ => panic!("Button expected"),
        }
    }
    fn live_tree(app: &glory::App) -> String {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        let mut nodes = vec![];
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            nodes.push(format!("{id:?}:{node:?}"));
            pending.extend(node.children.iter().rev().copied());
        }
        format!("{:?}:{nodes:?}", tree.roots())
    }

    #[test]
    fn ordered_same_revision_cross_parent_keeps_owner_state_and_cleanup() {
        let app = glory::App::memory();
        let calls = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let manager = ordered_manager(&app, calls.clone(), drops.clone());
        let initial = manager
            .prepare(ordered_rows(10., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&initial).unwrap();
        app.flush().unwrap();
        let roots: Vec<_> = ["a", "b", "c"]
            .map(|id| initial.record_roots(id).unwrap()[0])
            .to_vec();
        let owners: Vec<_> = ["a", "b", "c"]
            .map(|id| {
                initial.targets[id]
                    .mounted
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .scope()
                    .id()
            })
            .to_vec();
        let a = find(&app, "ordered-a");
        app.dispatch(a, Event::Activate).unwrap();
        app.dispatch(a, Event::FocusChanged(true)).unwrap();
        let first = named_parent(&app, "first group", 0.5);
        let first_id = first.roots()[0];
        let target = BTreeMap::from([
            ("a".into(), (first_id, Some(roots[1]))),
            ("b".into(), (first_id, Some(roots[2]))),
            ("c".into(), (first_id, None)),
        ]);
        let stage = manager
            .prepare(ordered_rows(20., 0), BTreeSet::new())
            .unwrap();
        manager.commit_placed(&stage, &target).unwrap();
        app.flush().unwrap();
        assert_eq!(app.tree().node(first_id).unwrap().children, roots);
        assert_eq!(
            app.tree()
                .node(first_id)
                .unwrap()
                .descriptor
                .view()
                .style
                .opacity,
            Some(0.5)
        );
        assert_eq!(find(&app, "ordered-a"), a);
        assert_eq!(button_label(&app, a), "20:20 clicks=1 focus=true");
        assert_eq!(calls.get(), 3);
        let second = named_parent(&app, "second group", 0.5);
        let second_id = second.roots()[0];
        // Packet order is a/b/c, but actual source order is c/a/b. Anchor
        // dependencies must therefore drive placement, not map or packet order.
        let targets = BTreeMap::from([
            ("a".into(), (second_id, Some(roots[1]))),
            ("b".into(), (second_id, None)),
            ("c".into(), (second_id, Some(roots[0]))),
        ]);
        let stage = manager
            .prepare(ordered_rows(30., 0), BTreeSet::new())
            .unwrap();
        manager.commit_placed(&stage, &targets).unwrap();
        app.flush().unwrap();
        drop(first);
        app.flush().unwrap();
        assert_eq!(
            app.tree().node(second_id).unwrap().children,
            [roots[2], roots[0], roots[1]]
        );
        assert_eq!(find(&app, "ordered-a"), a);
        assert_eq!(button_label(&app, a), "30:30 clicks=1 focus=true");
        for (index, id) in ["a", "b", "c"].into_iter().enumerate() {
            let record = &stage.targets[id];
            let mounted = record.mounted.borrow();
            let scope = mounted.as_ref().unwrap().scope();
            assert_eq!(scope.id(), owners[index]);
            assert!(scope.contains_node(second_id, roots[index]).unwrap());
            assert!(
                !scope.is_within(second_id).unwrap(),
                "construction ancestry must not be used as the live placement"
            );
        }
        assert_eq!(drops.get(), 0);
        manager.clear();
        app.flush().unwrap();
        assert_eq!(drops.get(), 3);
        assert!(app.tree().node(second_id).unwrap().children.is_empty());
        drop(second);
        app.flush().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }

    struct Guard(Memory);
    impl Render for Guard {
        fn preflight_place(
            &self,
            tree: &Memory,
            _child: Subtree<'_>,
            parent: Option<NodeId>,
        ) -> std::result::Result<(), RenderError> {
            if let Some(parent) = parent
                && tree.node(parent)?.descriptor.view().name.as_deref() == Some("blocked group")
            {
                return Err(RenderError::Unsupported("ordered placement test refusal"));
            }
            Ok(())
        }
        fn capabilities(&self) -> Capabilities {
            self.0.capabilities()
        }
        fn style_profile(&self) -> Profile {
            self.0.style_profile()
        }
        fn apply(&mut self, mutations: &[Mutation]) -> std::result::Result<Receipt, RenderError> {
            self.0.apply(mutations)
        }
        fn request(
            &mut self,
            query: Query,
            token: QueryToken,
        ) -> std::result::Result<(), RenderError> {
            self.0.request(query, token)
        }
        fn drain(&mut self, inputs: &mut Vec<Input>) {
            self.0.drain(inputs)
        }
    }

    #[test]
    fn ordered_failed_third_move_restores_siblings_snapshot_and_node_identity() {
        let app = glory::App::new(|id| Guard(Memory::new(id)));
        let calls = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let manager = ordered_manager(&app, calls.clone(), drops.clone());
        let initial = manager
            .prepare(ordered_rows(10., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&initial).unwrap();
        app.flush().unwrap();
        let roots = ["a", "b", "c"].map(|id| initial.record_roots(id).unwrap()[0]);
        let safe = named_parent(&app, "safe group", 0.5);
        let blocked = named_parent(&app, "blocked group", 0.5);
        app.flush().unwrap();
        let before = live_tree(&app);
        let live = app.stats().nodes;
        let a = find(&app, "ordered-a");
        let placements: Vec<_> = ["a", "b", "c"]
            .map(|id| initial.targets[id].placement.borrow().clone())
            .to_vec();
        let target = BTreeMap::from([
            ("a".into(), (safe.roots()[0], Some(roots[1]))),
            ("b".into(), (safe.roots()[0], None)),
            ("c".into(), (blocked.roots()[0], None)),
        ]);
        let stage = manager
            .prepare(ordered_rows(99., 0), BTreeSet::new())
            .unwrap();
        assert!(
            manager
                .commit_placed(&stage, &target)
                .unwrap_err()
                .0
                .contains("ordered placement test refusal")
        );
        app.flush().unwrap();
        assert_eq!(live_tree(&app), before);
        assert_eq!(app.stats().nodes, live);
        assert_eq!(find(&app, "ordered-a"), a);
        assert_eq!(button_label(&app, a), "10:10 clicks=0 focus=false");
        assert_eq!(manager.0.generation.get(), 1);
        assert!(!stage.committed.get());
        for (index, id) in ["a", "b", "c"].into_iter().enumerate() {
            assert_eq!(*initial.targets[id].placement.borrow(), placements[index]);
        }
        // A rejected placement does not consume the stage or commit tentative
        // data. The same stage can be placed into a valid source plan later.
        let fixed = BTreeMap::from([
            ("a".into(), (safe.roots()[0], Some(roots[1]))),
            ("b".into(), (safe.roots()[0], Some(roots[2]))),
            ("c".into(), (safe.roots()[0], None)),
        ]);
        manager.commit_placed(&stage, &fixed).unwrap();
        app.flush().unwrap();
        assert_eq!(button_label(&app, a), "99:99 clicks=0 focus=false");
        assert_eq!(calls.get(), 3);
        let accepted = live_tree(&app);
        let staged = manager
            .prepare(ordered_rows(200., 1), BTreeSet::new())
            .unwrap();
        // A later pending packet also occupies the staging parent. Its roots
        // must remain stable while an earlier pending packet rolls back.
        let later = manager
            .prepare(ordered_rows(300., 2), BTreeSet::new())
            .unwrap();
        app.flush().unwrap();
        let tentative = live_tree(&app);
        let new_roots = ["a", "b", "c"].map(|id| staged.record_roots(id).unwrap()[0]);
        let fail_new = BTreeMap::from([
            ("a".into(), (safe.roots()[0], Some(new_roots[1]))),
            ("b".into(), (safe.roots()[0], None)),
            ("c".into(), (blocked.roots()[0], None)),
        ]);
        assert!(manager.commit_placed(&staged, &fail_new).is_err());
        app.flush().unwrap();
        assert_eq!(
            live_tree(&app),
            tentative,
            "new roots resume then resuspend"
        );
        assert_eq!(button_label(&app, a), "99:99 clicks=0 focus=false");
        assert_eq!(manager.0.pending.get(), 6);
        drop(staged);
        assert_eq!(manager.0.pending.get(), 3);
        drop(later);
        app.flush().unwrap();
        assert_eq!(live_tree(&app), accepted);
        assert_eq!(manager.0.pending.get(), 0);
        assert_eq!(drops.get(), 6);
        manager.clear();
        drop(safe);
        drop(blocked);
        app.flush().unwrap();
        assert_eq!(drops.get(), 9);
        assert_eq!(app.stats().nodes, 0);
    }

    #[test]
    fn ordered_invalid_plan_and_child_stage_failures_leave_old_frame() {
        let app = glory::App::memory();
        let drops = Rc::new(Cell::new(0));
        let manager = ordered_manager(&app, Rc::new(Cell::new(0)), drops.clone());
        let initial = manager
            .prepare(ordered_rows(10., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&initial).unwrap();
        app.flush().unwrap();
        let parent = named_parent(&app, "target", 1.);
        let parent_id = parent.roots()[0];
        app.flush().unwrap();
        let before = live_tree(&app);
        let count = app.stats().nodes;
        for revision in [4, 5] {
            assert!(
                manager
                    .prepare(ordered_rows(99., revision), BTreeSet::new())
                    .is_err()
            );
            app.flush().unwrap();
            assert_eq!(live_tree(&app), before);
            assert_eq!(app.stats().nodes, count);
        }
        let stage = manager
            .prepare(ordered_rows(99., 0), BTreeSet::new())
            .unwrap();
        let roots = ["a", "b", "c"].map(|id| stage.record_roots(id).unwrap()[0]);
        let cycle = BTreeMap::from([
            ("a".into(), (parent_id, Some(roots[1]))),
            ("b".into(), (parent_id, Some(roots[0]))),
            ("c".into(), (parent_id, None)),
        ]);
        assert!(manager.commit_placed(&stage, &cycle).is_err());
        let missing = BTreeMap::from([("a".into(), (parent_id, None))]);
        assert!(manager.commit_placed(&stage, &missing).is_err());
        let inside = BTreeMap::from([
            ("a".into(), (roots[0], None)),
            ("b".into(), (parent_id, None)),
            ("c".into(), (parent_id, None)),
        ]);
        assert!(manager.commit_placed(&stage, &inside).is_err());
        app.flush().unwrap();
        assert_eq!(live_tree(&app), before);
        assert_eq!(app.stats().nodes, count);
        manager.clear();
        drop(parent);
        app.flush().unwrap();
        assert_eq!(drops.get(), 4);
        assert_eq!(app.stats().nodes, 0);
    }

    #[test]
    fn ordered_packet_mode_is_additive_and_keeps_empty_metadata_type() {
        let widgets = PlotWidgetsData::<String> {
            layout: PlotWidgetsLayout::default(),
            data: BTreeMap::new(),
        };
        let composition = PlotWidgetsComposition {
            widgets: widgets.clone(),
            replace_ink: BTreeSet::new(),
        };
        let ordered = PlotWidgetsPacket::in_scene(composition.clone());
        ordered.validate().unwrap();
        assert!(ordered.is_source_ordered());
        assert!(ordered.is_composition());
        let plain = PlotWidgetsPacket::plain(PlotWidgetsLayout::default());
        let typed = PlotWidgetsPacket::typed(widgets);
        let top = PlotWidgetsPacket::composition(composition);
        assert!(
            !plain.is_source_ordered() && !typed.is_source_ordered() && !top.is_source_ordered()
        );
        assert_eq!(ordered.data_type(), top.data_type());
        assert_ne!(ordered, top);
        let mut invalid = ordered;
        invalid.composition_mode = false;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn pending_children_are_metered_before_build_and_released_once() {
        let app = glory::App::memory();
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        let manager = CompositionManager::new(
            app.scope(),
            app.scope().style_profile().unwrap(),
            plain_factory(move |_| {
                seen.set(seen.get() + 1);
                Ok(glory::ui::button().label("staged"))
            }),
            Style::new().into(),
            None,
        )
        .unwrap();
        let rows = |revision| {
            mark_rows(
                (0..MAX_PLOT_WIDGET_SLOTS).map(|i| format!("child{i}")),
                0.,
                revision,
            )
        };
        let first = manager.prepare(rows(0), BTreeSet::new()).unwrap();
        let second = manager.prepare(rows(1), BTreeSet::new()).unwrap();
        assert_eq!(manager.0.pending.get(), MAX_PENDING_CHILDREN);
        assert!(manager.prepare(rows(2), BTreeSet::new()).is_err());
        assert_eq!(
            count.get(),
            MAX_PENDING_CHILDREN,
            "reject before an extra factory call"
        );
        drop(first);
        assert_eq!(manager.0.pending.get(), MAX_PLOT_WIDGET_SLOTS);
        let third = manager.prepare(rows(2), BTreeSet::new()).unwrap();
        let retarget = manager.retarget(&third, rows(2), BTreeSet::new()).unwrap();
        drop(third);
        assert_eq!(
            manager.0.pending.get(),
            MAX_PENDING_CHILDREN,
            "retarget shares one lease"
        );
        manager.commit(&second).unwrap();
        assert_eq!(manager.0.pending.get(), MAX_PLOT_WIDGET_SLOTS);
        let reuse = manager.prepare(rows(1), BTreeSet::new()).unwrap();
        assert_eq!(
            manager.0.pending.get(),
            MAX_PLOT_WIDGET_SLOTS,
            "same revision uses no pending reservation"
        );
        drop(reuse);
        drop(retarget);
        assert_eq!(manager.0.pending.get(), 0);
        let retry = manager.prepare(rows(3), BTreeSet::new()).unwrap();
        assert_eq!(manager.0.pending.get(), MAX_PLOT_WIDGET_SLOTS);
        drop(retry);
        manager.clear();
        app.flush().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
    #[test]
    fn coordinated_adoption_updates_both_data_cages_before_effects() {
        let app = glory::App::memory();
        let manager = scene_manager(&app, Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let targets = [
            ("mark", crate::SceneWidgetTarget::Mark("a".into())),
            ("label", crate::SceneWidgetTarget::Label(0)),
        ];
        let first = manager
            .prepare(scene_rows(&targets, 10., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&first).unwrap();
        app.flush().unwrap();
        let snapshot = first.targets["mark"].snapshot;
        let sidecar = app.scope().cage(0u32);
        let observations = Rc::new(RefCell::new(Vec::new()));
        let observed = observations.clone();
        app.scope().effect(move || {
            let epoch = sidecar.get();
            let x = snapshot.with(|s| s.row.frame(&s.hidden).mark_frame().unwrap().mark.value);
            observed.borrow_mut().push((epoch, x));
        });
        app.flush().unwrap();
        let second = manager
            .prepare(scene_rows(&targets, 20., 0), BTreeSet::new())
            .unwrap();
        let parent = manager
            .0
            .parent
            .upgrade()
            .unwrap()
            .construction_parent()
            .unwrap();
        let mut placements = BTreeMap::new();
        let mut before = None;
        for row in second.rows.iter().rev() {
            let roots = second.targets[&row.key].roots().unwrap();
            placements.insert(row.key.clone(), (parent, before));
            before = roots.first().copied();
        }
        let calls = Cell::new(0);
        manager
            .commit_placed_with_adopt(&second, &placements, &mut || {
                assert!(second.lease.adopted.get());
                assert_eq!(
                    snapshot.with(|s| s.row.frame(&s.hidden).mark_frame().unwrap().mark.value),
                    10.
                );
                sidecar.revise(|v| *v = 1);
                calls.set(calls.get() + 1);
            })
            .unwrap();
        assert_eq!(calls.get(), 1);
        app.flush().unwrap();
        assert_eq!(observations.borrow().last(), Some(&(1, 20.)));
        assert!(
            !observations.borrow().contains(&(1, 10.)),
            "no sidecar-only intermediate effect"
        );
        let rejected = manager
            .prepare(scene_rows(&targets, 30., 0), BTreeSet::new())
            .unwrap();
        let invalid = BTreeMap::from(targets.map(|(key, _)| {
            (
                key.to_owned(),
                (rejected.record_roots(key).unwrap()[0], None),
            )
        }));
        assert!(
            manager
                .commit_placed_with_adopt(&rejected, &invalid, &mut || sidecar.revise(|v| *v = 2))
                .is_err()
        );
        assert!(!rejected.lease.adopted.get());
        app.flush().unwrap();
        assert_eq!(sidecar.get(), 1);
        assert_eq!(
            snapshot.with(|s| s.row.frame(&s.hidden).mark_frame().unwrap().mark.value),
            20.
        );
        assert_eq!(observations.borrow().last(), Some(&(1, 20.)));
    }
    #[test]
    fn legacy_commit_rejects_prepare_from_retiring_child_cleanup() {
        assert_cleanup_reentry_is_rejected(false);
    }
    #[test]
    fn split_legacy_commit_guard_covers_retiring_child_cleanup() {
        assert_cleanup_reentry_is_rejected(true);
    }
    fn assert_cleanup_reentry_is_rejected(split: bool) {
        use glory::Widget;
        let app = glory::App::memory();
        let owner = Rc::new(RefCell::new(None::<Weak<Inner>>));
        let result = Rc::new(RefCell::new(None::<String>));
        let calls = Rc::new(Cell::new(0usize));
        let captured_owner = owner.clone();
        let captured_result = result.clone();
        let captured_calls = calls.clone();
        let manager = CompositionManager::new(
            app.scope(),
            app.scope().style_profile().unwrap(),
            composition_factory(move |context: PlotWidgetCompositionContext<OrderedData>| {
                captured_calls.set(captured_calls.get() + 1);
                let owner = captured_owner.clone();
                let result = captured_result.clone();
                let revision = context.revision;
                Ok(glory::ui::widget(move |scope| {
                    if revision == 0 {
                        let owner = owner.clone();
                        let result = result.clone();
                        scope.on_cleanup(move || {
                            let manager = CompositionManager(
                                owner.borrow().as_ref().unwrap().upgrade().unwrap(),
                            );
                            let attempt =
                                manager.prepare(mark_rows(["a".into()], 3., 99), BTreeSet::new());
                            *result.borrow_mut() = Some(match attempt {
                                Ok(_) => "unexpected reentry".into(),
                                Err(e) => e.0,
                            });
                        })?;
                    }
                    glory::ui::button().label("retained").show_in(scope)
                }))
            }),
            Style::new().into(),
            None,
        )
        .unwrap();
        *owner.borrow_mut() = Some(Rc::downgrade(&manager.0));
        let first = manager
            .prepare(mark_rows(["a".into()], 1., 0), BTreeSet::new())
            .unwrap();
        manager.commit(&first).unwrap();
        app.flush().unwrap();
        let next = manager
            .prepare(mark_rows(["a".into()], 2., 1), BTreeSet::new())
            .unwrap();
        if split {
            let parent = manager
                .0
                .parent
                .upgrade()
                .unwrap()
                .construction_parent()
                .unwrap();
            let targets = BTreeMap::from([("a".into(), (parent, None))]);
            let placed = manager.place_placed(&next, &targets).unwrap();
            manager
                .with_commit_guard(|| {
                    placed.validate()?;
                    drop(placed.adopt_with(&mut || {}));
                    Ok(())
                })
                .unwrap();
        } else {
            manager.commit(&next).unwrap();
        }
        assert_eq!(
            result.borrow().as_deref(),
            Some("reentrant composition preparation or commit")
        );
        assert_eq!(
            calls.get(),
            2,
            "cleanup cannot construct an additional child"
        );
        app.flush().unwrap();
    }
}
