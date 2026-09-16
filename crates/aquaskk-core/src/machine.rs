//! Hierarchical state machine (port of `GenericStateMachine.h`).
//!
//! The original identified states by member-function pointers; here states
//! are identified by a `Copy + Eq` id type and the caller supplies the
//! handler dispatch through the [`StateHandler`] trait.

use std::collections::HashMap;
use std::hash::Hash;

/// System events delivered to state handlers alongside user events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    /// Query the super state (handlers must not act on this).
    Probe,
    Entry,
    Init,
    Exit,
}

/// What a state handler returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateResult<Id> {
    /// Event not handled here; bubble up to this super state.
    Super(Id),
    /// Top state has no super.
    Top,
    /// Event consumed, no transition.
    Handled,
    /// Initial transition target (from Init handlers).
    Initial(Id),
    Transition(Id),
    /// From Exit handlers: record shallow/deep history for this state.
    SaveHistory,
    /// From Init handlers: enter the saved shallow history, or this default.
    ShallowHistory(Id),
    /// Transition to the deep history recorded for this state.
    DeepHistory(Id),
    /// Transition, then re-dispatch the current event in the new state.
    Forward(Id),
    /// DeepHistory + re-dispatch.
    DeepForward(Id),
}

/// Dispatches events to state handlers.
pub trait StateHandler {
    type Id: Copy + Eq + Hash + std::fmt::Debug;
    type Event;

    /// The top state's initial substate.
    fn initial_state(&self) -> Self::Id;

    /// Run one state's handler for a user event.
    fn handle(&mut self, state: Self::Id, event: &Self::Event) -> StateResult<Self::Id>;

    /// Run one state's handler for a system event.
    fn handle_system(&mut self, state: Self::Id, event: SystemEvent) -> StateResult<Self::Id>;
}

/// The machine: current state plus history bookkeeping.
#[derive(Debug, Clone)]
pub struct StateMachine<Id: Copy + Eq + Hash + std::fmt::Debug> {
    top: Id,
    active: Option<Id>,
    prior: Option<Id>,
    history: HashMap<Id, (Option<Id>, Option<Id>)>, // (shallow, deep)
}

impl<Id: Copy + Eq + Hash + std::fmt::Debug> StateMachine<Id> {
    /// `top` is a dedicated top-state id whose handler returns
    /// `Initial(initial_state())` on Init and `Handled` otherwise.
    pub fn new(top: Id) -> Self {
        Self { top, active: None, prior: None, history: HashMap::new() }
    }

    pub fn current_state(&self) -> Option<Id> {
        self.active
    }

    pub fn is_child_of<H>(&mut self, handler: &mut H, state: Id) -> bool
    where
        H: StateHandler<Id = Id>,
    {
        let mut current = match self.active {
            Some(active) => active,
            None => return false,
        };

        while current != self.top {
            if current == state {
                return true;
            }
            match self.super_state(handler, current) {
                Some(super_state) => current = super_state,
                None => return false,
            }
        }

        false
    }

    fn super_state<H>(&mut self, handler: &mut H, state: Id) -> Option<Id>
    where
        H: StateHandler<Id = Id>,
    {
        match handler.handle_system(state, SystemEvent::Probe) {
            StateResult::Super(id) => Some(id),
            _ => None,
        }
    }

    fn entry_action<H>(&mut self, handler: &mut H, state: Id)
    where
        H: StateHandler<Id = Id>,
    {
        handler.handle_system(state, SystemEvent::Entry);
    }

    fn exit_action<H>(&mut self, handler: &mut H, state: Id)
    where
        H: StateHandler<Id = Id>,
    {
        if let StateResult::SaveHistory = handler.handle_system(state, SystemEvent::Exit) {
            debug_assert!(self.prior.is_some(), "shallow history not found");
            self.history.insert(state, (self.prior, self.active));
        }

        self.prior = Some(state);
    }

    fn initialize<H>(&mut self, handler: &mut H, target: Id)
    where
        H: StateHandler<Id = Id>,
    {
        let mut active = target;

        loop {
            match handler.handle_system(active, SystemEvent::Init) {
                StateResult::Initial(next) => {
                    debug_assert!(next != active, "infinite loop in Init");
                    active = next;
                }
                StateResult::ShallowHistory(default) => {
                    let shallow = self.history.get(&active).and_then(|(s, _)| *s);
                    match shallow {
                        Some(saved) => active = saved,
                        None => {
                            self.history.insert(active, (Some(default), None));
                            active = default;
                        }
                    }
                }
                _ => break,
            }

            self.entry_action(handler, active);
        }

        self.active = Some(active);
    }

    fn transition<H>(&mut self, handler: &mut H, source: Id, target: Id)
    where
        H: StateHandler<Id = Id>,
    {
        debug_assert!(target != self.top, "transition to top state is not allowed");

        // Exit down to the source
        self.prior = None;
        let mut tmp = self.active.expect("machine not started");
        while tmp != source {
            self.exit_action(handler, tmp);
            tmp = self.super_state(handler, tmp).expect("no super state");
        }

        // Exit to the least common ancestor, recording the entry path
        let mut path = vec![target];

        'plan: {
            // (a) self transition
            if source == target {
                self.exit_action(handler, source);
                break 'plan;
            }

            // (b) go into a direct substate
            let target_super = self.super_state(handler, target);
            if Some(source) == target_super {
                break 'plan;
            }

            // (c) sibling transition
            let source_super = self.super_state(handler, source);
            if source_super == target_super {
                self.exit_action(handler, source);
                break 'plan;
            }

            // (d) leave to the direct super state
            if source_super == Some(target) {
                self.exit_action(handler, source);
                path.clear();
                break 'plan;
            }

            // (e) go into a substate multiple levels down
            if let Some(target_super) = target_super {
                path.push(target_super);
                let mut tmp = self.super_state(handler, target_super);
                while let Some(state) = tmp {
                    if source == state {
                        break 'plan;
                    }
                    path.push(state);
                    tmp = self.super_state(handler, state);
                }
            }

            self.exit_action(handler, source);

            let truncate_at = |path: &mut Vec<Id>, state: Id| -> bool {
                if let Some(pos) = path.iter().position(|&p| p == state) {
                    path.truncate(pos);
                    return true;
                }
                false
            };

            // (f) unbalanced transition
            if let Some(source_super) = source_super {
                if truncate_at(&mut path, source_super) {
                    break 'plan;
                }

                // (g) leave from a substate multiple levels up
                let mut tmp = Some(source_super);
                while let Some(state) = tmp {
                    if truncate_at(&mut path, state) {
                        break 'plan;
                    }
                    self.exit_action(handler, state);
                    tmp = self.super_state(handler, state);
                }
            }

            debug_assert!(false, "invalid state transition form");
        }

        // Enter towards the target
        while let Some(state) = path.pop() {
            self.entry_action(handler, state);
        }
    }

    pub fn start<H>(&mut self, handler: &mut H)
    where
        H: StateHandler<Id = Id>,
    {
        debug_assert!(self.active.is_none(), "start() called twice");
        let top = self.top;
        self.initialize(handler, top);
    }

    pub fn dispatch<H>(&mut self, handler: &mut H, event: &H::Event)
    where
        H: StateHandler<Id = Id>,
    {
        if self.active.is_none() {
            self.start(handler);
        }

        let mut source = self.active.expect("machine not started");

        loop {
            let next = handler.handle(source, event);

            match next {
                StateResult::Transition(_)
                | StateResult::DeepHistory(_)
                | StateResult::Forward(_)
                | StateResult::DeepForward(_) => {
                    let target = match next {
                        StateResult::DeepHistory(id) | StateResult::DeepForward(id) => self
                            .history
                            .get(&id)
                            .and_then(|(_, deep)| *deep)
                            .expect("deep history not found"),
                        StateResult::Transition(id) | StateResult::Forward(id) => id,
                        _ => unreachable!(),
                    };

                    self.transition(handler, source, target);
                    self.initialize(handler, target);

                    if matches!(next, StateResult::Forward(_) | StateResult::DeepForward(_)) {
                        source = target;
                        continue;
                    }

                    break;
                }
                StateResult::Super(super_state) => {
                    source = super_state;
                }
                StateResult::Top | StateResult::Handled => break,
                _ => {
                    debug_assert!(false, "invalid state result from user event");
                    break;
                }
            }
        }
    }
}
