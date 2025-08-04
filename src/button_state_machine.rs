use std::time::Instant;

/// Generic state machine for button press detection
#[derive(Debug, Clone)]
pub struct ButtonStateMachine<S> {
    current_state: S,
    first_signal_time: Option<Instant>,
    last_signal_time: Option<Instant>,
    signal_count: u32,
    action_fired: bool,
}

impl<S> ButtonStateMachine<S>
where
    S: Clone + Copy + PartialEq,
{
    pub fn new(initial_state: S) -> Self {
        Self {
            current_state: initial_state,
            first_signal_time: None,
            last_signal_time: None,
            signal_count: 0,
            action_fired: false,
        }
    }

    /// Get the current state
    pub fn state(&self) -> S {
        self.current_state
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: S) {
        self.current_state = new_state;
    }

    /// Record a signal at the given time
    pub fn record_signal(&mut self, now: Instant) {
        if self.first_signal_time.is_none() {
            self.first_signal_time = Some(now);
            self.signal_count = 1;
        } else {
            self.signal_count += 1;
        }
        self.last_signal_time = Some(now);
    }

    /// Get the time elapsed since the first signal
    pub fn time_since_first_signal(&self, now: Instant) -> Option<std::time::Duration> {
        self.first_signal_time
            .map(|first| now.duration_since(first))
    }

    /// Get the number of signals recorded
    pub fn signal_count(&self) -> u32 {
        self.signal_count
    }

    /// Check if an action has been fired
    pub fn action_fired(&self) -> bool {
        self.action_fired
    }

    /// Mark that an action has been fired
    pub fn mark_action_fired(&mut self) {
        self.action_fired = true;
    }

    /// Reset the state machine to initial state
    pub fn reset(&mut self, initial_state: S) {
        self.current_state = initial_state;
        self.first_signal_time = None;
        self.last_signal_time = None;
        self.signal_count = 0;
        self.action_fired = false;
    }
}

/// State machine transition result
#[derive(Debug)]
pub enum StateTransition<E> {
    /// Continue processing, no events generated
    Continue,
    /// Generate events and optionally transition state
    EmitEvents(Vec<E>),
    /// Reset the state machine
    Reset,
}

/// Trait for defining state machine behavior
pub trait StateMachineLogic<S, E, Input> {
    /// Process input and return transition result
    fn process_input(
        &self,
        state_machine: &mut ButtonStateMachine<S>,
        input: Input,
        now: Instant,
    ) -> StateTransition<E>;

    /// Get the initial state
    fn initial_state(&self) -> S;
}
