//! The bounded native press lifetime, independent of DOM and paint ownership.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PressToken {
    pub id: i32,
    pub generation: u64,
    pub widget: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Down,
    UpTail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sequence {
    token: PressToken,
    phase: Phase,
    cancelled: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PressState {
    generation: u64,
    sequence: Option<Sequence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct StartedPress {
    pub token: PressToken,
    /// A previous Up-tail can finish before another native Down begins.
    pub previous_active: Option<PressToken>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FinishedPress {
    pub token: PressToken,
    pub was_active: bool,
}

impl PressState {
    pub fn held(self) -> bool {
        self.sequence.is_some_and(|s| !s.cancelled)
    }

    pub fn token(self) -> Option<PressToken> {
        self.sequence.map(|s| s.token)
    }

    pub fn cancelled(self) -> bool {
        self.sequence.is_some_and(|s| s.cancelled)
    }

    pub fn begin(&mut self, id: i32, widget: bool) -> Option<StartedPress> {
        if self
            .sequence
            .is_some_and(|s| s.phase == Phase::Down && !s.cancelled)
        {
            return None;
        }
        let previous_active = self.sequence.filter(|s| !s.cancelled).map(|s| s.token);
        self.generation = self.generation.wrapping_add(1);
        let token = PressToken {
            id,
            generation: self.generation,
            widget,
        };
        self.sequence = Some(Sequence {
            token,
            phase: Phase::Down,
            cancelled: false,
        });
        Some(StartedPress {
            token,
            previous_active,
        })
    }

    pub fn up(&mut self, id: i32) -> Option<PressToken> {
        let sequence = self.sequence.as_mut().filter(|s| s.token.id == id)?;
        sequence.phase = Phase::UpTail;
        Some(sequence.token)
    }

    /// Retire painting protection only after the explicit candidate commits.
    /// Keep a cancelled sequence until its old native click can be suppressed.
    pub fn cancel_accepted(&mut self) -> Option<PressToken> {
        let sequence = self.sequence.as_mut().filter(|s| !s.cancelled)?;
        sequence.cancelled = true;
        Some(sequence.token)
    }

    pub fn finish(&mut self, token: PressToken) -> Option<FinishedPress> {
        let sequence = self.sequence.filter(|s| s.token == token)?;
        self.sequence = None;
        Some(FinishedPress {
            token,
            was_active: !sequence.cancelled,
        })
    }

    /// A different real Down must not inherit cancelled click suppression.
    pub fn clear_retired(&mut self) -> Option<FinishedPress> {
        let sequence = self
            .sequence
            .filter(|s| s.cancelled || s.phase == Phase::UpTail)?;
        self.finish(sequence.token)
    }

    pub fn abort(&mut self) -> Option<FinishedPress> {
        let sequence = self.sequence?;
        self.finish(sequence.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_widget_hold_through_up_tail() {
        for widget in [false, true] {
            let mut state = PressState::default();
            let started = state.begin(7, widget).unwrap();
            assert_eq!(started.previous_active, None);
            assert!(state.held());
            assert_eq!(state.up(99), None);
            assert_eq!(state.up(7), Some(started.token));
            assert!(state.held());
            assert_eq!(state.token().unwrap().widget, widget);
            let finished = state.finish(started.token).unwrap();
            assert_eq!(finished.token, started.token);
            assert!(finished.was_active);
            assert!(!state.held());
            assert_eq!(state.token(), None);
        }
    }

    #[test]
    fn duplicate_down_cannot_replace_active_press() {
        let mut state = PressState::default();
        let token = state.begin(1, true).unwrap().token;
        let before = state;
        assert!(!state.cancelled());
        assert_eq!(state.begin(2, false), None);
        assert_eq!(state, before);
        assert_eq!(state.up(1), Some(token));
        assert!(state.finish(token).unwrap().was_active);
    }

    #[test]
    fn accepted_candidate_cancels_but_retains_old_click_identity() {
        let mut state = PressState::default();
        let token = state.begin(1, true).unwrap().token;
        assert_eq!(state.cancel_accepted(), Some(token));
        assert!(!state.held());
        assert!(state.cancelled());
        assert_eq!(state.cancel_accepted(), None);
        assert_eq!(state.token(), Some(token));
        assert_eq!(state.up(1), Some(token));
        assert!(!state.finish(token).unwrap().was_active);
    }

    #[test]
    fn old_timer_cannot_finish_reused_pointer_id() {
        let mut state = PressState::default();
        let old = state.begin(1, false).unwrap().token;
        state.up(1).unwrap();
        let new = state.begin(1, true).unwrap();
        assert_eq!(new.previous_active, Some(old));
        assert_ne!(new.token.generation, old.generation);
        assert_eq!(state.finish(old), None);
        assert!(state.held());
        assert_eq!(state.token(), Some(new.token));
    }

    #[test]
    fn fresh_down_and_abort_remove_old_suppression_without_leaking() {
        let mut state = PressState::default();
        let old = state.begin(1, true).unwrap().token;
        state.cancel_accepted();
        assert!(!state.clear_retired().unwrap().was_active);
        let new = state.begin(1, false).unwrap().token;
        assert!(!state.cancelled());
        assert_eq!(state.finish(old), None);
        assert!(state.abort().unwrap().was_active);
        assert_eq!(state.token(), None);
        assert_eq!(state.finish(new), None);
        assert_eq!(state.abort(), None);
    }
}
