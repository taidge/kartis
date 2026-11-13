//! Deterministic animation scheduling without browser, timer or global state.
//! A host queues a ticket, then delivers its monotonic timestamp through
//! advance_frame. Retain the driver in the owning component/resource; Drop
//! cancels outstanding work. Scheduler callbacks should hold only weak owners.
use crate::scene::ensure;
use crate::{
    ChartError, Result, Scene,
    motion::{SpringConfig, SpringMotion},
};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

/// Request only queues work; it must not synchronously reenter the driver.
/// Tickets must distinguish requests while cancelled callbacks can still arrive.
pub trait FrameScheduler {
    type Ticket: Clone + Eq;
    fn request_frame(&mut self) -> Result<Self::Ticket>;
    fn cancel_frame(&mut self, ticket: Self::Ticket);
    /// Optional acknowledgement for manually queued schedulers. Native RAF
    /// schedulers normally consume their ticket before invoking the driver.
    fn frame_delivered(&mut self, _ticket: &Self::Ticket) {}
}

/// A validated scene animation that can be advanced by any host clock.
/// Implementations must leave their accepted scene intact on rejection.
pub trait SceneMotion: Clone {
    fn scene(&self) -> &Scene;
    fn is_settled(&self) -> bool;
    fn retarget(&mut self, target: Scene) -> Result<()>;
    fn tick(&mut self, seconds: f64) -> Result<()>;
    fn set_reduced_motion(&mut self, reduced: bool);
}
impl SceneMotion for SpringMotion {
    fn scene(&self) -> &Scene {
        self.scene()
    }
    fn is_settled(&self) -> bool {
        self.is_settled()
    }
    fn retarget(&mut self, target: Scene) -> Result<()> {
        self.retarget(target)
    }
    fn tick(&mut self, seconds: f64) -> Result<()> {
        self.tick(seconds).map(|_| ())
    }
    fn set_reduced_motion(&mut self, reduced: bool) {
        self.set_reduced_motion(reduced);
    }
}

#[must_use = "retain the frame driver while animating; dropping it cancels pending frames"]
pub struct FrameDriver<S: FrameScheduler, M: SceneMotion = SpringMotion> {
    motion: M,
    scheduler: S,
    pending: Option<S::Ticket>,
    last_timestamp: Option<f64>,
    max_frame_delta: f64,
    stopped: bool,
}
impl<S: FrameScheduler, M: SceneMotion> FrameDriver<S, M> {
    /// Schedule one frame immediately if the supplied motion is active.
    pub fn new(motion: M, scheduler: S) -> Result<Self> {
        let mut driver = Self {
            motion,
            scheduler,
            pending: None,
            last_timestamp: None,
            max_frame_delta: 0.25,
            stopped: false,
        };
        driver.next_frame()?;
        Ok(driver)
    }
    pub fn scene(&self) -> &Scene {
        self.motion.scene()
    }
    pub fn motion(&self) -> &M {
        &self.motion
    }
    pub fn pending_ticket(&self) -> Option<&S::Ticket> {
        self.pending.as_ref()
    }
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
    pub fn max_frame_delta(&self) -> f64 {
        self.max_frame_delta
    }
    /// Long host pauses simulate at most this many seconds, preventing an
    /// unintended jump when a background tab resumes. ExplicitClock uses 60.
    pub fn set_max_frame_delta(&mut self, seconds: f64) -> Result<()> {
        ensure(
            seconds.is_finite() && (0.000001..=60.).contains(&seconds),
            "invalid maximum frame delta",
        )?;
        self.max_frame_delta = seconds;
        Ok(())
    }
    /// Establish a known time baseline without advancing any animation.
    pub fn start_at(&mut self, timestamp_seconds: f64) -> Result<()> {
        timestamp(timestamp_seconds)?;
        self.last_timestamp = Some(timestamp_seconds);
        Ok(())
    }
    pub fn retarget(&mut self, target: Scene) -> Result<()> {
        let mut candidate = self.motion.clone();
        candidate.retarget(target)?;
        let next = if !self.stopped && !candidate.is_settled() && self.pending.is_none() {
            Some(self.scheduler.request_frame()?)
        } else {
            None
        };
        if self.motion.is_settled() {
            self.last_timestamp = None;
        }
        self.motion = candidate;
        if let Some(ticket) = next {
            self.pending = Some(ticket);
        }
        if self.motion.is_settled() {
            self.cancel_pending();
            self.last_timestamp = None;
        }
        Ok(())
    }
    /// At most one request may be outstanding. Calling this repeatedly is safe.
    pub fn next_frame(&mut self) -> Result<()> {
        if self.stopped || self.motion.is_settled() {
            self.cancel_pending();
            return Ok(());
        }
        if self.pending.is_none() {
            self.pending = Some(self.scheduler.request_frame()?);
        }
        Ok(())
    }
    /// Deliver the current scheduled ticket. Hosts that retain ticket identity
    /// should prefer advance_frame so stale callbacks cannot consume newer work.
    pub fn advance(&mut self, timestamp_seconds: f64) -> Result<&Scene> {
        if self.stopped || self.motion.is_settled() {
            return Ok(self.scene());
        }
        let ticket = self
            .pending
            .clone()
            .ok_or_else(|| ChartError("no scheduled animation frame".into()))?;
        self.advance_frame(ticket, timestamp_seconds)
    }
    /// Ignore cancelled, duplicate and stale deliveries before touching state.
    /// A scheduling failure retains the prior delivered frame and consumes the
    /// completed ticket; next_frame can retry requesting further work.
    pub fn advance_frame(&mut self, ticket: S::Ticket, timestamp_seconds: f64) -> Result<&Scene> {
        if self.stopped || self.pending.as_ref() != Some(&ticket) {
            return Ok(self.scene());
        }
        timestamp(timestamp_seconds)?;
        let delta = match self.last_timestamp {
            Some(last) => {
                ensure(
                    timestamp_seconds >= last,
                    "animation timestamps must be monotonic",
                )?;
                (timestamp_seconds - last).min(self.max_frame_delta)
            }
            None => 0.,
        };
        let mut candidate = self.motion.clone();
        candidate.tick(delta)?;
        self.scheduler.frame_delivered(&ticket);
        self.pending = None;
        let next = if candidate.is_settled() {
            None
        } else {
            Some(self.scheduler.request_frame()?)
        };
        self.motion = candidate;
        self.pending = next;
        self.last_timestamp = if self.motion.is_settled() {
            None
        } else {
            Some(timestamp_seconds)
        };
        Ok(self.scene())
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> &Scene {
        self.motion.set_reduced_motion(reduced);
        if self.motion.is_settled() {
            self.cancel_pending();
            self.last_timestamp = None;
        }
        self.scene()
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        self.cancel_pending();
        self.last_timestamp = None;
    }
    pub fn resume(&mut self) -> Result<()> {
        if !self.stopped {
            return self.next_frame();
        }
        let next = if self.motion.is_settled() {
            None
        } else {
            Some(self.scheduler.request_frame()?)
        };
        self.pending = next;
        self.last_timestamp = None;
        self.stopped = false;
        Ok(())
    }
    fn cancel_pending(&mut self) {
        if let Some(ticket) = self.pending.take() {
            self.scheduler.cancel_frame(ticket);
        }
    }
}
impl<S: FrameScheduler, M: SceneMotion> Drop for FrameDriver<S, M> {
    fn drop(&mut self) {
        self.stop();
    }
}
fn timestamp(value: f64) -> Result<()> {
    ensure(
        value.is_finite() && value >= 0.,
        "animation timestamp must be finite and nonnegative",
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTicket(pub u64);
#[derive(Default)]
struct ExplicitState {
    next: u64,
    pending: VecDeque<FrameTicket>,
    requests: u64,
    cancellations: u64,
}
/// A bounded, shared manual scheduler. Taking a request simulates host delivery;
/// it does not run callbacks or advance time implicitly.
#[derive(Clone, Default)]
pub struct ExplicitScheduler(Rc<RefCell<ExplicitState>>);
impl ExplicitScheduler {
    pub fn take_requested(&self) -> Option<FrameTicket> {
        self.0.borrow_mut().pending.pop_front()
    }
    pub fn pending_count(&self) -> usize {
        self.0.borrow().pending.len()
    }
    pub fn request_count(&self) -> u64 {
        self.0.borrow().requests
    }
    pub fn cancellation_count(&self) -> u64 {
        self.0.borrow().cancellations
    }
}
impl FrameScheduler for ExplicitScheduler {
    type Ticket = FrameTicket;
    fn request_frame(&mut self) -> Result<FrameTicket> {
        let mut state = self.0.borrow_mut();
        ensure(
            state.pending.len() < 4096,
            "manual frame queue budget exceeded",
        )?;
        let next = state
            .next
            .checked_add(1)
            .ok_or_else(|| ChartError("manual frame identity exhausted".into()))?;
        let ticket = FrameTicket(next);
        state.next = next;
        state.requests = state.requests.saturating_add(1);
        state.pending.push_back(ticket);
        Ok(ticket)
    }
    fn cancel_frame(&mut self, ticket: FrameTicket) {
        let mut state = self.0.borrow_mut();
        state.pending.retain(|pending| *pending != ticket);
        state.cancellations = state.cancellations.saturating_add(1);
    }
    fn frame_delivered(&mut self, ticket: &FrameTicket) {
        self.0
            .borrow_mut()
            .pending
            .retain(|pending| pending != ticket);
    }
}

/// A deterministic explicit-clock adapter for simulations, tests and hosts
/// already owning a clock. Each step delivers one requested frame. Large steps
/// up to 60 seconds are simulated completely instead of using the RAF pause cap.
pub struct ExplicitClock<M: SceneMotion = SpringMotion> {
    driver: FrameDriver<ExplicitScheduler, M>,
    scheduler: ExplicitScheduler,
    now: f64,
}
impl ExplicitClock<SpringMotion> {
    pub fn new(initial: Scene, config: SpringConfig) -> Result<Self> {
        Self::from_motion(SpringMotion::new(initial, config)?)
    }
}
impl<M: SceneMotion> ExplicitClock<M> {
    pub fn from_motion(motion: M) -> Result<Self> {
        let scheduler = ExplicitScheduler::default();
        let mut driver = FrameDriver::new(motion, scheduler.clone())?;
        driver.set_max_frame_delta(60.)?;
        driver.start_at(0.)?;
        Ok(Self {
            driver,
            scheduler,
            now: 0.,
        })
    }
    pub fn scene(&self) -> &Scene {
        self.driver.scene()
    }
    pub fn motion(&self) -> &M {
        self.driver.motion()
    }
    pub fn now(&self) -> f64 {
        self.now
    }
    pub fn pending_frames(&self) -> usize {
        self.scheduler.pending_count()
    }
    pub fn is_settled(&self) -> bool {
        self.driver.motion().is_settled()
    }
    pub fn retarget(&mut self, target: Scene) -> Result<()> {
        self.driver.retarget(target)?;
        self.driver.start_at(self.now)
    }
    pub fn step(&mut self, seconds: f64) -> Result<&Scene> {
        ensure(
            seconds.is_finite() && (0. ..=60.).contains(&seconds),
            "manual clock step must be in 0..=60 seconds",
        )?;
        let next = self.now + seconds;
        timestamp(next)?;
        if let Some(ticket) = self.driver.pending_ticket().cloned() {
            self.driver.advance_frame(ticket, next)?;
        }
        self.now = next;
        Ok(self.scene())
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> &Scene {
        self.driver.set_reduced_motion(reduced)
    }
    pub fn stop(&mut self) {
        self.driver.stop();
    }
    pub fn resume(&mut self) -> Result<()> {
        self.driver.resume()?;
        self.driver.start_at(self.now)
    }
}
