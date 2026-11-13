//! Bounded, renderer-independent editing of a persistent logical-pixel brush.
//!
//! Pointer geometry and modifier baselines follow d3-brush 3.0.0. This checked
//! Rust profile additionally retires superseded sessions, makes idle program
//! updates idempotent, and tests only active axes when terminating a selection.
//! Hosts perform their scene/domain coordinate conversion at their boundary.
//!
//! Geometry/modifier rules adapted from d3-brush 3.0.0, Copyright 2010–2021
//! Mike Bostock, ISC; full notice in
//! `tests/fixtures/editable-brush-d3-LICENSE.txt` and
//! `docs/third-party-notices.md`.
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{Brush, ChartError, Result};

static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrushAxis {
    X,
    Y,
    #[default]
    XY,
}
impl BrushAxis {
    fn x(self) -> bool {
        self != Self::Y
    }
    fn y(self) -> bool {
        self != Self::X
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditableBrushOptions {
    pub axis: BrushAxis,
    /// Logical pixels, including an optional zero span on either axis.
    pub extent: Brush,
    /// Full hit-target width, as in d3-brush; rectangles extend half outside.
    pub handle_size: f64,
    pub modifiers: bool,
}
impl Default for EditableBrushOptions {
    fn default() -> Self {
        Self {
            axis: BrushAxis::XY,
            extent: Brush {
                x0: 0.,
                y0: 0.,
                x1: 1.,
                y1: 1.,
            },
            handle_size: 6.,
            modifiers: true,
        }
    }
}
impl EditableBrushOptions {
    pub fn validate(&self) -> Result<()> {
        self.extent.validate()?;
        checked(
            self.handle_size.is_finite() && (0. ..=1_000_000.).contains(&self.handle_size),
            "invalid brush handle size",
        )?;
        let h = self.handle_size / 2.;
        checked(
            [
                self.extent.x0 - h,
                self.extent.y0 - h,
                self.extent.x1 + h,
                self.extent.y1 + h,
            ]
            .iter()
            .all(|n| n.is_finite() && n.abs() <= 1_000_000.),
            "brush handles exceed coordinate budget",
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushHandle {
    N,
    E,
    S,
    W,
    Nw,
    Ne,
    Sw,
    Se,
}
impl BrushHandle {
    pub fn name(self) -> &'static str {
        match self {
            Self::N => "n",
            Self::E => "e",
            Self::S => "s",
            Self::W => "w",
            Self::Nw => "nw",
            Self::Ne => "ne",
            Self::Sw => "sw",
            Self::Se => "se",
        }
    }
    fn signs(self) -> (i8, i8) {
        match self {
            Self::N => (0, -1),
            Self::E => (1, 0),
            Self::S => (0, 1),
            Self::W => (-1, 0),
            Self::Nw => (-1, -1),
            Self::Ne => (1, -1),
            Self::Sw => (-1, 1),
            Self::Se => (1, 1),
        }
    }
    fn flip_x(self) -> Self {
        match self {
            Self::E => Self::W,
            Self::W => Self::E,
            Self::Nw => Self::Ne,
            Self::Ne => Self::Nw,
            Self::Sw => Self::Se,
            Self::Se => Self::Sw,
            other => other,
        }
    }
    fn flip_y(self) -> Self {
        match self {
            Self::N => Self::S,
            Self::S => Self::N,
            Self::Nw => Self::Sw,
            Self::Sw => Self::Nw,
            Self::Ne => Self::Se,
            Self::Se => Self::Ne,
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushTarget {
    Overlay,
    Selection,
    Handle(BrushHandle),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushHandleRect {
    pub handle: BrushHandle,
    pub bounds: Brush,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrushModifiers {
    pub alt: bool,
    pub shift: bool,
    pub space: bool,
    pub meta: bool,
    pub ctrl: bool,
}
/// An unforgeable generation within one controller. Retired input is ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrushSession {
    owner: u64,
    generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushPhase {
    Start,
    Change,
    End,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushSource {
    User,
    Programmatic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushMode {
    Create,
    Move,
    Resize,
    Center,
    Space,
    Programmatic,
    Clear,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditableBrushEvent {
    pub phase: BrushPhase,
    pub source: BrushSource,
    pub mode: BrushMode,
    pub selection: Option<Brush>,
    pub session: Option<BrushSession>,
    pub target: Option<BrushTarget>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditableBrushInput {
    Begin {
        id: i32,
        point: (f64, f64),
        target: BrushTarget,
        modifiers: BrushModifiers,
    },
    Move {
        session: BrushSession,
        id: i32,
        point: (f64, f64),
    },
    End {
        session: BrushSession,
        id: i32,
        point: (f64, f64),
    },
    Cancel {
        session: BrushSession,
        id: i32,
    },
    Modifiers {
        session: BrushSession,
        modifiers: BrushModifiers,
    },
    /// Keyboard editing uses the same geometry without inventing a pointer.
    Nudge {
        target: BrushTarget,
        delta: (f64, f64),
    },
    Clear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushReconfigure {
    Preserve,
    Clamp,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Motion {
    Handle,
    Drag,
    Center,
    Space,
}
#[derive(Clone, Debug, PartialEq)]
struct Gesture {
    session: BrushSession,
    id: i32,
    target: BrushTarget,
    origin: Option<Brush>,
    start: (f64, f64),
    point: (f64, f64),
    // Baseline edges may reverse on crossing, while public rectangles normalize.
    base: [f64; 4],
    current: Brush,
    sx: i8,
    sy: i8,
    motion: Motion,
    dx: f64,
    dy: f64,
    shifting: bool,
    lock_x: bool,
    lock_y: bool,
    modifiers: BrushModifiers,
}
impl Gesture {
    fn mode(&self) -> BrushMode {
        match self.motion {
            Motion::Drag => BrushMode::Move,
            Motion::Center => BrushMode::Center,
            Motion::Space => BrushMode::Space,
            Motion::Handle if self.target == BrushTarget::Overlay => BrushMode::Create,
            Motion::Handle => BrushMode::Resize,
        }
    }
    fn event(&self, phase: BrushPhase, selection: Option<Brush>) -> EditableBrushEvent {
        EditableBrushEvent {
            phase,
            source: BrushSource::User,
            mode: self.mode(),
            selection,
            session: Some(self.session),
            target: Some(self.target),
        }
    }
    fn move_to(&mut self, point: (f64, f64), extent: Brush) -> Brush {
        if self.shifting && !self.lock_x && !self.lock_y {
            if (point.0 - self.point.0).abs() > (point.1 - self.point.1).abs() {
                self.lock_y = true;
            } else {
                self.lock_x = true;
            }
        }
        self.point = point;
        self.calculate(extent)
    }
    fn calculate(&mut self, extent: Brush) -> Brush {
        let [w0, n0, e0, s0] = self.base;
        let mut w1 = self.current.x0;
        let mut n1 = self.current.y0;
        let mut e1 = self.current.x1;
        let mut s1 = self.current.y1;
        self.dx = self.point.0 - self.start.0;
        self.dy = self.point.1 - self.start.1;
        match self.motion {
            Motion::Drag | Motion::Space => {
                if self.sx != 0 {
                    self.dx = self.dx.clamp(extent.x0 - w0, extent.x1 - e0);
                    w1 = w0 + self.dx;
                    e1 = e0 + self.dx;
                }
                if self.sy != 0 {
                    self.dy = self.dy.clamp(extent.y0 - n0, extent.y1 - s0);
                    n1 = n0 + self.dy;
                    s1 = s0 + self.dy;
                }
            }
            Motion::Handle => {
                if self.sx < 0 {
                    self.dx = self.dx.clamp(extent.x0 - w0, extent.x1 - w0);
                    w1 = w0 + self.dx;
                    e1 = e0;
                } else if self.sx > 0 {
                    self.dx = self.dx.clamp(extent.x0 - e0, extent.x1 - e0);
                    w1 = w0;
                    e1 = e0 + self.dx;
                }
                if self.sy < 0 {
                    self.dy = self.dy.clamp(extent.y0 - n0, extent.y1 - n0);
                    n1 = n0 + self.dy;
                    s1 = s0;
                } else if self.sy > 0 {
                    self.dy = self.dy.clamp(extent.y0 - s0, extent.y1 - s0);
                    n1 = n0;
                    s1 = s0 + self.dy;
                }
            }
            Motion::Center => {
                if self.sx != 0 {
                    w1 = (w0 - self.dx * f64::from(self.sx)).clamp(extent.x0, extent.x1);
                    e1 = (e0 + self.dx * f64::from(self.sx)).clamp(extent.x0, extent.x1);
                }
                if self.sy != 0 {
                    n1 = (n0 - self.dy * f64::from(self.sy)).clamp(extent.y0, extent.y1);
                    s1 = (s0 + self.dy * f64::from(self.sy)).clamp(extent.y0, extent.y1);
                }
            }
        }
        if e1 < w1 {
            self.sx *= -1;
            self.base.swap(0, 2);
            std::mem::swap(&mut w1, &mut e1);
            if let BrushTarget::Handle(h) = self.target {
                self.target = BrushTarget::Handle(h.flip_x());
            }
        }
        if s1 < n1 {
            self.sy *= -1;
            self.base.swap(1, 3);
            std::mem::swap(&mut n1, &mut s1);
            if let BrushTarget::Handle(h) = self.target {
                self.target = BrushTarget::Handle(h.flip_y());
            }
        }
        if self.lock_x {
            w1 = self.current.x0;
            e1 = self.current.x1;
        }
        if self.lock_y {
            n1 = self.current.y0;
            s1 = self.current.y1;
        }
        self.current = Brush {
            x0: w1,
            y0: n1,
            x1: e1,
            y1: s1,
        };
        self.current
    }
    fn modifiers(&mut self, modifiers: BrushModifiers, extent: Brush) -> Brush {
        let previous = self.modifiers;
        self.modifiers = modifiers;
        if modifiers.shift && !previous.shift {
            self.shifting = self.sx != 0 && self.sy != 0;
        }
        if !modifiers.shift && previous.shift {
            self.shifting = false;
            self.lock_x = false;
            self.lock_y = false;
            self.calculate(extent);
        }
        // Match physical key transitions. Space overrides Alt while held.
        if modifiers.alt && !previous.alt && self.motion == Motion::Handle {
            self.center_baseline();
            self.motion = Motion::Center;
            self.calculate(extent);
        }
        if !modifiers.alt && previous.alt && self.motion == Motion::Center {
            self.handle_baseline();
            self.motion = Motion::Handle;
            self.calculate(extent);
        }
        if modifiers.space
            && !previous.space
            && matches!(self.motion, Motion::Handle | Motion::Center)
        {
            // Lock the *displayed* span. After centered resize hits an extent,
            // either edge may have been clipped independently. Rebasing only
            // the opposite edge (D3's unchecked path) can create an oversized
            // drag baseline and an inverted translation interval.
            if self.sx != 0 {
                self.base[0] = self.current.x0 - self.dx;
                self.base[2] = self.current.x1 - self.dx;
            }
            if self.sy != 0 {
                self.base[1] = self.current.y0 - self.dy;
                self.base[3] = self.current.y1 - self.dy;
            }
            self.motion = Motion::Space;
            self.calculate(extent);
        }
        if !modifiers.space && previous.space && self.motion == Motion::Space {
            if modifiers.alt {
                self.center_baseline();
                self.motion = Motion::Center;
            } else {
                self.handle_baseline();
                self.motion = Motion::Handle;
            }
            self.calculate(extent);
        }
        self.current
    }
    fn center_baseline(&mut self) {
        if self.sx != 0 {
            self.base[2] = self.current.x1 - self.dx * f64::from(self.sx);
            self.base[0] = self.current.x0 + self.dx * f64::from(self.sx);
        }
        if self.sy != 0 {
            self.base[3] = self.current.y1 - self.dy * f64::from(self.sy);
            self.base[1] = self.current.y0 + self.dy * f64::from(self.sy);
        }
    }
    fn handle_baseline(&mut self) {
        if self.sx < 0 {
            self.base[2] = self.current.x1;
        } else if self.sx > 0 {
            self.base[0] = self.current.x0;
        }
        if self.sy < 0 {
            self.base[3] = self.current.y1;
        } else if self.sy > 0 {
            self.base[1] = self.current.y0;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditableBrush {
    owner: u64,
    options: EditableBrushOptions,
    selection: Option<Brush>,
    gesture: Option<Gesture>,
    generation: u64,
}
impl EditableBrush {
    pub fn new(options: EditableBrushOptions) -> Result<Self> {
        options.validate()?;
        let owner = allocate_owner()?;
        Ok(Self {
            owner,
            options,
            selection: None,
            gesture: None,
            generation: 0,
        })
    }
    pub fn options(&self) -> &EditableBrushOptions {
        &self.options
    }
    pub fn selection(&self) -> Option<Brush> {
        self.selection
    }
    pub fn session(&self) -> Option<BrushSession> {
        self.gesture.as_ref().map(|g| g.session)
    }
    pub fn active(&self) -> bool {
        self.gesture.is_some()
    }
    pub fn mode(&self) -> Option<BrushMode> {
        self.gesture.as_ref().map(Gesture::mode)
    }
    pub fn handles(&self) -> Vec<BrushHandleRect> {
        let Some(b) = self.selection else {
            return vec![];
        };
        let h = self.options.handle_size;
        let half = h / 2.;
        let order = match self.options.axis {
            BrushAxis::X => &[BrushHandle::W, BrushHandle::E][..],
            BrushAxis::Y => &[BrushHandle::N, BrushHandle::S][..],
            BrushAxis::XY => &[
                BrushHandle::N,
                BrushHandle::W,
                BrushHandle::E,
                BrushHandle::S,
                BrushHandle::Nw,
                BrushHandle::Ne,
                BrushHandle::Sw,
                BrushHandle::Se,
            ][..],
        };
        order
            .iter()
            .map(|handle| {
                let (sx, sy) = handle.signs();
                let x0 = if sx > 0 { b.x1 - half } else { b.x0 - half };
                let y0 = if sy > 0 { b.y1 - half } else { b.y0 - half };
                let width = if matches!(handle, BrushHandle::N | BrushHandle::S) {
                    b.x1 - b.x0 + h
                } else {
                    h
                };
                let height = if matches!(handle, BrushHandle::E | BrushHandle::W) {
                    b.y1 - b.y0 + h
                } else {
                    h
                };
                BrushHandleRect {
                    handle: *handle,
                    bounds: Brush {
                        x0,
                        y0,
                        x1: x0 + width,
                        y1: y0 + height,
                    },
                }
            })
            .collect()
    }
    pub fn hit_test(&self, point: (f64, f64)) -> Result<Option<BrushTarget>> {
        finite(point)?;
        for handle in self.handles().iter().rev() {
            if handle.bounds.contains(point) {
                return Ok(Some(BrushTarget::Handle(handle.handle)));
            }
        }
        if self.selection.is_some_and(|b| b.contains(point)) {
            return Ok(Some(BrushTarget::Selection));
        }
        Ok(self
            .options
            .extent
            .contains(point)
            .then_some(BrushTarget::Overlay))
    }
    /// All rejected inputs retain geometry, sessions, and modifier baselines.
    pub fn apply(&mut self, input: EditableBrushInput) -> Result<Vec<EditableBrushEvent>> {
        let mut next = self.clone();
        let events = next.input(input)?;
        next.validate()?;
        *self = next;
        Ok(events)
    }
    /// Roll back an active user gesture and retire its session.
    pub fn cancel(&mut self) -> Result<Vec<EditableBrushEvent>> {
        let Some(g) = self.gesture.as_ref() else {
            return Ok(vec![]);
        };
        self.apply(EditableBrushInput::Cancel {
            session: g.session,
            id: g.id,
        })
    }
    /// Replace the displayed selection, retiring any current gesture. Idle,
    /// identical updates are no-ops, so callbacks can safely echo accepted state.
    pub fn set_selection(&mut self, selection: Option<Brush>) -> Result<Vec<EditableBrushEvent>> {
        let selection = self.admit(selection, false)?;
        let mut next = self.clone();
        let mut events = next.retire();
        if selection != next.selection {
            events.extend(next.replace(
                selection,
                BrushSource::Programmatic,
                BrushMode::Programmatic,
                None,
            ));
        }
        next.validate()?;
        *self = next;
        Ok(events)
    }
    /// A mode/extent change never silently interprets old pixels as data values.
    /// Preserve rejects out-of-extent active coordinates; Clamp is explicit.
    pub fn configure(
        &mut self,
        options: EditableBrushOptions,
        policy: BrushReconfigure,
    ) -> Result<Vec<EditableBrushEvent>> {
        options.validate()?;
        let mut next = self.clone();
        next.options = options;
        let selection = match policy {
            BrushReconfigure::Clear => None,
            BrushReconfigure::Clamp => next.admit(self.selection, true)?,
            BrushReconfigure::Preserve => next.admit(self.selection, false)?,
        };
        if options == self.options && selection == self.selection && !self.active() {
            return Ok(vec![]);
        }
        let mut events = next.retire();
        if options != self.options || selection != self.selection {
            events.extend(next.replace(
                selection,
                BrushSource::Programmatic,
                BrushMode::Programmatic,
                None,
            ));
        }
        next.validate()?;
        *self = next;
        Ok(events)
    }
    fn validate(&self) -> Result<()> {
        self.options.validate()?;
        if let Some(b) = self.selection {
            b.validate()?;
            checked(
                inside(b, self.options.extent),
                "brush selection escaped extent",
            )?;
        }
        if let Some(g) = &self.gesture {
            finite(g.start)?;
            finite(g.point)?;
            checked(
                g.base
                    .iter()
                    .chain([&g.dx, &g.dy])
                    .all(|v| v.is_finite() && v.abs() <= 4_000_000.),
                "brush gesture baseline overflow",
            )?;
        }
        Ok(())
    }
    fn admit(&self, selection: Option<Brush>, clamp: bool) -> Result<Option<Brush>> {
        let Some(mut b) = selection else {
            return Ok(None);
        };
        b.validate()?;
        let e = self.options.extent;
        if !self.options.axis.x() {
            b.x0 = e.x0;
            b.x1 = e.x1;
        }
        if !self.options.axis.y() {
            b.y0 = e.y0;
            b.y1 = e.y1;
        }
        if clamp {
            b.x0 = b.x0.clamp(e.x0, e.x1);
            b.x1 = b.x1.clamp(e.x0, e.x1);
            b.y0 = b.y0.clamp(e.y0, e.y1);
            b.y1 = b.y1.clamp(e.y0, e.y1);
        }
        checked(inside(b, e), "brush selection is outside extent")?;
        Ok((!empty(b, self.options.axis)).then_some(b))
    }
    fn retire(&mut self) -> Vec<EditableBrushEvent> {
        self.gesture
            .take()
            .map(|g| vec![g.event(BrushPhase::Cancel, self.selection)])
            .unwrap_or_default()
    }
    fn replace(
        &mut self,
        selection: Option<Brush>,
        source: BrushSource,
        mode: BrushMode,
        target: Option<BrushTarget>,
    ) -> Vec<EditableBrushEvent> {
        self.selection = selection;
        [BrushPhase::Start, BrushPhase::Change, BrushPhase::End]
            .into_iter()
            .map(|phase| EditableBrushEvent {
                phase,
                source,
                mode,
                selection,
                session: None,
                target,
            })
            .collect()
    }
    fn gesture(
        &self,
        id: i32,
        point: (f64, f64),
        target: BrushTarget,
        modifiers: BrushModifiers,
        session: BrushSession,
    ) -> Result<Gesture> {
        let target = if self.options.modifiers && modifiers.meta {
            BrushTarget::Overlay
        } else {
            target
        };
        let origin = self.selection;
        let selection = match target {
            BrushTarget::Overlay => Brush {
                x0: if self.options.axis.x() {
                    point.0
                } else {
                    self.options.extent.x0
                },
                x1: if self.options.axis.x() {
                    point.0
                } else {
                    self.options.extent.x1
                },
                y0: if self.options.axis.y() {
                    point.1
                } else {
                    self.options.extent.y0
                },
                y1: if self.options.axis.y() {
                    point.1
                } else {
                    self.options.extent.y1
                },
            },
            BrushTarget::Selection => self
                .selection
                .ok_or_else(|| ChartError("cannot move an absent brush".into()))?,
            BrushTarget::Handle(h) => {
                checked(
                    self.handles().iter().any(|r| r.handle == h),
                    "brush handle is unavailable for this axis or selection",
                )?;
                self.selection
                    .ok_or_else(|| ChartError("cannot resize an absent brush".into()))?
            }
        };
        let (mut sx, mut sy) = match target {
            BrushTarget::Handle(h) => h.signs(),
            _ => (1, 1),
        };
        if !self.options.axis.x() {
            sx = 0;
        }
        if !self.options.axis.y() {
            sy = 0;
        }
        let motion = if target == BrushTarget::Selection {
            Motion::Drag
        } else if self.options.modifiers && modifiers.alt {
            Motion::Center
        } else {
            Motion::Handle
        };
        let modifiers = if self.options.modifiers {
            modifiers
        } else {
            BrushModifiers::default()
        };
        Ok(Gesture {
            session,
            id,
            target,
            origin,
            start: point,
            point,
            base: [selection.x0, selection.y0, selection.x1, selection.y1],
            current: selection,
            sx,
            sy,
            motion,
            dx: 0.,
            dy: 0.,
            shifting: sx != 0 && sy != 0 && modifiers.shift,
            lock_x: false,
            lock_y: false,
            modifiers,
        })
    }
    fn input(&mut self, input: EditableBrushInput) -> Result<Vec<EditableBrushEvent>> {
        match input {
            EditableBrushInput::Begin {
                id,
                point,
                target,
                modifiers,
            } => {
                finite(point)?;
                if self.active() || modifiers.ctrl {
                    return Ok(vec![]);
                }
                let target = if self.options.modifiers && modifiers.meta {
                    BrushTarget::Overlay
                } else {
                    target
                };
                if target == BrushTarget::Overlay && !self.options.extent.contains(point) {
                    return Ok(vec![]);
                }
                let generation = self
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| ChartError("brush session generation exhausted".into()))?;
                let g = self.gesture(
                    id,
                    point,
                    target,
                    modifiers,
                    BrushSession {
                        owner: self.owner,
                        generation,
                    },
                )?;
                self.selection = Some(g.current);
                self.generation = generation;
                let event = g.event(BrushPhase::Start, self.selection);
                self.gesture = Some(g);
                Ok(vec![event])
            }
            EditableBrushInput::Move { session, id, point } => {
                if !self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.session == session && g.id == id)
                {
                    return Ok(vec![]);
                }
                finite(point)?;
                let g = self.gesture.as_mut().unwrap();
                let b = g.move_to(point, self.options.extent);
                if self.selection == Some(b) {
                    return Ok(vec![]);
                }
                self.selection = Some(b);
                Ok(vec![g.event(BrushPhase::Change, self.selection)])
            }
            EditableBrushInput::End { session, id, point } => {
                if !self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.session == session && g.id == id)
                {
                    return Ok(vec![]);
                }
                let mut events = self.input(EditableBrushInput::Move { session, id, point })?;
                let g = self.gesture.take().unwrap();
                if self.selection.is_some_and(|b| empty(b, self.options.axis)) {
                    self.selection = None;
                }
                events.push(g.event(BrushPhase::End, self.selection));
                Ok(events)
            }
            EditableBrushInput::Cancel { session, id } => {
                if !self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.session == session && g.id == id)
                {
                    return Ok(vec![]);
                }
                let g = self.gesture.take().unwrap();
                self.selection = g.origin;
                Ok(vec![g.event(BrushPhase::Cancel, self.selection)])
            }
            EditableBrushInput::Modifiers { session, modifiers } => {
                if !self.options.modifiers
                    || !self.gesture.as_ref().is_some_and(|g| g.session == session)
                {
                    return Ok(vec![]);
                }
                let g = self.gesture.as_mut().unwrap();
                let b = g.modifiers(modifiers, self.options.extent);
                if self.selection == Some(b) {
                    return Ok(vec![]);
                }
                self.selection = Some(b);
                Ok(vec![g.event(BrushPhase::Change, self.selection)])
            }
            EditableBrushInput::Nudge { target, delta } => {
                finite(delta)?;
                checked(
                    target != BrushTarget::Overlay,
                    "keyboard editing requires a selection or handle",
                )?;
                checked(
                    !matches!(target, BrushTarget::Handle(h) if (self.options.axis == BrushAxis::X && !matches!(h, BrushHandle::E | BrushHandle::W)) || (self.options.axis == BrushAxis::Y && !matches!(h, BrushHandle::N | BrushHandle::S))),
                    "brush handle is unavailable for this axis",
                )?;
                let Some(b) = self.selection else {
                    return Ok(vec![]);
                };
                let mut g = self.gesture(
                    0,
                    (0., 0.),
                    target,
                    BrushModifiers::default(),
                    BrushSession {
                        owner: self.owner,
                        generation: 0,
                    },
                )?;
                let result = g.move_to(delta, self.options.extent);
                let result = (!empty(result, self.options.axis)).then_some(result);
                let mut events = self.retire();
                if Some(b) != result {
                    events.extend(self.replace(
                        result,
                        BrushSource::User,
                        g.mode(),
                        Some(g.target),
                    ));
                }
                Ok(events)
            }
            EditableBrushInput::Clear => {
                let mut events = self.retire();
                if self.selection.is_some() {
                    events.extend(self.replace(None, BrushSource::User, BrushMode::Clear, None));
                }
                Ok(events)
            }
        }
    }
}

fn checked(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(ChartError(message.into()))
    }
}
fn allocate_owner() -> Result<u64> {
    let mut previous = NEXT_OWNER.load(Ordering::Relaxed);
    loop {
        let next = previous
            .checked_add(1)
            .ok_or_else(|| ChartError("brush owner generation exhausted".into()))?;
        match NEXT_OWNER.compare_exchange_weak(previous, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(owner) => return Ok(owner),
            Err(actual) => previous = actual,
        }
    }
}
fn finite(point: (f64, f64)) -> Result<()> {
    checked(
        [point.0, point.1]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
        "brush input is outside finite coordinate budget",
    )
}
fn inside(b: Brush, e: Brush) -> bool {
    b.x0 >= e.x0 && b.x1 <= e.x1 && b.y0 >= e.y0 && b.y1 <= e.y1
}
fn empty(b: Brush, axis: BrushAxis) -> bool {
    (axis.x() && b.x0 == b.x1) || (axis.y() && b.y0 == b.y1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_exhaustion_rejects_without_touching_accepted_selection() {
        let mut b = EditableBrush::new(EditableBrushOptions {
            extent: Brush {
                x0: 0.,
                y0: 0.,
                x1: 100.,
                y1: 100.,
            },
            ..Default::default()
        })
        .unwrap();
        b.set_selection(Some(Brush {
            x0: 20.,
            y0: 20.,
            x1: 60.,
            y1: 60.,
        }))
        .unwrap();
        b.generation = u64::MAX;
        let before = b.clone();
        assert!(
            b.apply(EditableBrushInput::Begin {
                id: 1,
                point: (30., 30.),
                target: BrushTarget::Selection,
                modifiers: BrushModifiers::default()
            })
            .is_err()
        );
        assert_eq!(b, before);
    }

    #[test]
    fn pure_cancel_needs_no_further_generation_and_cannot_overflow() {
        let mut b = EditableBrush::new(EditableBrushOptions {
            extent: Brush {
                x0: 0.,
                y0: 0.,
                x1: 100.,
                y1: 100.,
            },
            ..Default::default()
        })
        .unwrap();
        let origin = Brush {
            x0: 20.,
            y0: 20.,
            x1: 60.,
            y1: 60.,
        };
        b.set_selection(Some(origin)).unwrap();
        b.apply(EditableBrushInput::Begin {
            id: 1,
            point: (30., 30.),
            target: BrushTarget::Selection,
            modifiers: BrushModifiers::default(),
        })
        .unwrap();
        b.generation = u64::MAX;
        let events = b.cancel().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].phase, BrushPhase::Cancel);
        assert_eq!(b.selection(), Some(origin));
        assert!(!b.active());
    }
}
