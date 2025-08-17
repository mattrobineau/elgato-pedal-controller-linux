use crate::button_state_machine::{ButtonStateMachine, StateMachineLogic, StateTransition};
use crate::button_types::{ButtonEvent, ButtonEventType, ButtonInput, ButtonState};
use crate::token_based_config::{PhysicalButtonName, TokenBasedParser};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Configuration for button behavior
#[derive(Debug, Clone)]
pub struct ButtonConfig {
    pub has_pressed_action: bool,
    pub has_held_action: bool,
    pub threshold_ms: u64,
}

/// Hold intent detection logic
pub struct HoldIntentLogic {
    global_default_threshold_ms: u64,
    config_parser: Arc<Mutex<TokenBasedParser>>,
}

impl HoldIntentLogic {
    pub fn new(
        global_default_threshold_ms: u64,
        config_parser: Arc<Mutex<TokenBasedParser>>,
    ) -> Self {
        Self {
            global_default_threshold_ms,
            config_parser,
        }
    }

    /// Calculate the quick release threshold as 60% of the button's hold threshold (minimum 200ms)
    fn get_quick_release_threshold_ms(&self, button_name: &PhysicalButtonName) -> u64 {
        let hold_threshold = {
            let config_parser = match self.config_parser.lock() {
                Ok(parser) => parser,
                Err(_) => return 200, // Default fallback
            };
            config_parser.get_hold_threshold_ms(*button_name, self.global_default_threshold_ms)
        };
        let calculated = (hold_threshold * 60) / 100;
        calculated.max(200)
    }

    /// Calculate the evaluation window as 120% of the button's hold threshold
    fn get_evaluation_window_ms(&self, button_name: &PhysicalButtonName) -> u64 {
        let hold_threshold = {
            let config_parser = match self.config_parser.lock() {
                Ok(parser) => parser,
                Err(_) => return 1200, // Default fallback (120% of 1000ms)
            };
            config_parser.get_hold_threshold_ms(*button_name, self.global_default_threshold_ms)
        };
        (hold_threshold * 120) / 100
    }

    pub fn get_button_config(&self, button_name: &PhysicalButtonName) -> ButtonConfig {
        let config_parser = match self.config_parser.lock() {
            Ok(parser) => parser,
            Err(_) => return ButtonConfig {
                has_pressed_action: false,
                has_held_action: false,
                threshold_ms: self.global_default_threshold_ms,
            },
        };
        let has_pressed_action = config_parser
            .get_actions_for_button_event(*button_name, "PRESSED")
            .is_some();
        let has_held_action = config_parser
            .get_actions_for_button_event(*button_name, "HELD")
            .is_some();

        // Use hierarchical threshold resolution: per-button > device > global default
        let threshold_ms =
            config_parser.get_hold_threshold_ms(*button_name, self.global_default_threshold_ms);

        ButtonConfig {
            has_pressed_action,
            has_held_action,
            threshold_ms,
        }
    }

    fn handle_idle_to_evaluating(
        &self,
        state_machine: &mut ButtonStateMachine<ButtonState>,
        input: &ButtonInput,
        config: &ButtonConfig,
        now: Instant,
    ) -> StateTransition<ButtonEvent> {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        println!(
            "[{}] 🔄 Button {} signal detected - starting intent evaluation (state: IDLE->EVALUATING)",
            timestamp,
            input.button_name.as_str()
        );

        state_machine.transition_to(ButtonState::EVALUATING);
        state_machine.record_signal(now);

        println!(
            "[{}] 🔍 Button {} config: has_pressed={}, has_held={}, threshold={}ms, evaluation_window={}ms",
            timestamp,
            input.button_name.as_str(),
            config.has_pressed_action,
            config.has_held_action,
            config.threshold_ms,
            self.get_evaluation_window_ms(&input.button_name)
        );

        if config.threshold_ms > 0 {
            println!(
                "[{}] ⏱️  Hold threshold timer started - will fire HELD at {}",
                timestamp,
                chrono::Local::now()
                    .checked_add_signed(chrono::Duration::milliseconds(config.threshold_ms as i64))
                    .unwrap_or_else(chrono::Local::now)
                    .format("%H:%M:%S%.3f")
            );
        }

        if config.has_pressed_action && !config.has_held_action {
            // PRESSED-only button: Fire immediately
            println!(
                "[{}] ⚡ Immediate PRESSED for {} (PRESSED-only button)",
                timestamp,
                input.button_name.as_str()
            );
            state_machine.mark_action_fired();
            StateTransition::EmitEvents(vec![ButtonEvent {
                button_name: input.button_name,
                event_type: ButtonEventType::PRESSED,
            }])
        } else {
            // For HELD-only and PRESSED+HELD buttons, wait for threshold timing
            StateTransition::Continue
        }
    }

    fn handle_evaluating_with_signal(
        &self,
        state_machine: &mut ButtonStateMachine<ButtonState>,
        input: &ButtonInput,
        config: &ButtonConfig,
        now: Instant,
    ) -> StateTransition<ButtonEvent> {
        state_machine.record_signal(now);

        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        println!(
            "[{}] 🔄 Button {} additional signal #{} detected",
            timestamp,
            input.button_name.as_str(),
            state_machine.signal_count()
        );

        // If we get multiple signals on PRESSED+HELD button, fire HELD
        if state_machine.signal_count() >= 2
            && !state_machine.action_fired()
            && config.has_held_action
            && config.has_pressed_action
        {
            println!(
                "[{}] 🔥 HELD event for {} (multiple signals on PRESSED+HELD button)",
                timestamp,
                input.button_name.as_str()
            );
            state_machine.mark_action_fired();
            // Don't change state - let HID data drive state transitions
            return StateTransition::EmitEvents(vec![ButtonEvent {
                button_name: input.button_name,
                event_type: ButtonEventType::HELD,
            }]);
        }

        StateTransition::Continue
    }
}

impl StateMachineLogic<ButtonState, ButtonEvent, ButtonInput> for HoldIntentLogic {
    fn process_input(
        &self,
        state_machine: &mut ButtonStateMachine<ButtonState>,
        input: ButtonInput,
        now: Instant,
    ) -> StateTransition<ButtonEvent> {
        let config = self.get_button_config(&input.button_name);

        match (state_machine.state(), input.is_pressed) {
            (ButtonState::IDLE, true) => {
                self.handle_idle_to_evaluating(state_machine, &input, &config, now)
            }
            (ButtonState::EVALUATING, true) => {
                // Check if we should transition to HELD state based on timing
                if let Some(time_since_first) = state_machine.time_since_first_signal(now)
                    && (time_since_first.as_millis() as u64) >= config.threshold_ms
                {
                    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                    println!(
                        "[{}] 🔄 Transitioning to HELD state for {} (threshold reached, button still pressed)",
                        timestamp,
                        input.button_name.as_str()
                    );
                    state_machine.transition_to(ButtonState::HELD);
                }

                self.handle_evaluating_with_signal(state_machine, &input, &config, now)
            }
            (ButtonState::EVALUATING, false) => {
                // CRITICAL: Physical button release during EVALUATING - cancel hold threshold timer!
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

                if let Some(time_since_first) = state_machine.time_since_first_signal(now) {
                    let time_elapsed_ms = time_since_first.as_millis() as u64;

                    println!(
                        "[{}] 🛑 Physical button release detected during EVALUATING for {} ({}ms elapsed < {}ms threshold)",
                        timestamp,
                        input.button_name.as_str(),
                        time_elapsed_ms,
                        config.threshold_ms
                    );
                    println!(
                        "[{timestamp}] ❌ Cancelling hold threshold timer - HELD state now impossible"
                    );

                    // Handle different button configurations for early release
                    let mut events_to_emit = vec![];

                    if config.has_pressed_action && config.has_held_action {
                        // PRESSED+HELD button: Check for quick release
                        let quick_release_threshold =
                            self.get_quick_release_threshold_ms(&input.button_name);
                        if time_elapsed_ms < quick_release_threshold
                            && !state_machine.action_fired()
                        {
                            println!(
                                "[{}] ⚡ Quick release detected for {} ({}ms elapsed < {}ms quick-release threshold) - firing PRESSED",
                                timestamp,
                                input.button_name.as_str(),
                                time_elapsed_ms,
                                quick_release_threshold
                            );
                            state_machine.mark_action_fired();
                            events_to_emit.push(ButtonEvent {
                                button_name: input.button_name,
                                event_type: ButtonEventType::PRESSED,
                            });
                        } else if !state_machine.action_fired() {
                            // Released too late for PRESSED, too early for HELD - no action
                            println!(
                                "[{}] 🔄 Button {} released too late for PRESSED ({}ms > {}ms), too early for HELD ({}ms < {}ms) - no action fired",
                                timestamp,
                                input.button_name.as_str(),
                                time_elapsed_ms,
                                quick_release_threshold,
                                time_elapsed_ms,
                                config.threshold_ms
                            );
                        }
                    } else if config.has_held_action && !config.has_pressed_action {
                        // HELD-only button: No action since threshold wasn't reached
                        if !state_machine.action_fired() {
                            println!(
                                "[{}] 🔄 HELD-only button {} released before threshold ({}ms < {}ms) - no action fired",
                                timestamp,
                                input.button_name.as_str(),
                                time_elapsed_ms,
                                config.threshold_ms
                            );
                        }
                    } else if config.has_pressed_action && !config.has_held_action {
                        // PRESSED-only button: Should have fired immediately on press, but handle edge case
                        if !state_machine.action_fired() {
                            println!(
                                "[{}] ⚡ Late PRESSED action for {} (PRESSED-only button released)",
                                timestamp,
                                input.button_name.as_str()
                            );
                            state_machine.mark_action_fired();
                            events_to_emit.push(ButtonEvent {
                                button_name: input.button_name,
                                event_type: ButtonEventType::PRESSED,
                            });
                        }
                    }

                    let has_releasing_action = {
                        let config_parser = match self.config_parser.lock() {
                            Ok(parser) => parser,
                            Err(_) => {
                                // On lock failure, emit any collected events and reset
                                return if events_to_emit.is_empty() {
                                    StateTransition::Reset
                                } else {
                                    StateTransition::EmitEvents(events_to_emit)
                                };
                            }
                        };
                        let result = config_parser
                            .get_actions_for_button_event(input.button_name, "RELEASING")
                            .is_some();
                        drop(config_parser);
                        result
                    };

                    if has_releasing_action {
                        // Transition to RELEASING state to allow RELEASING event to fire
                        println!(
                            "[{}] 🔄 Transitioning {} to RELEASING state (action was fired: {}, RELEASING configured: {})",
                            timestamp,
                            input.button_name.as_str(),
                            state_machine.action_fired(),
                            has_releasing_action
                        );
                        state_machine.transition_to(ButtonState::RELEASING);

                        if !events_to_emit.is_empty() {
                            // Emit both PRESSED and RELEASING in sequence
                            events_to_emit.push(ButtonEvent {
                                button_name: input.button_name,
                                event_type: ButtonEventType::RELEASING,
                            });
                            return StateTransition::EmitEvents(events_to_emit);
                        } else {
                            // Only RELEASING event
                            return StateTransition::EmitEvents(vec![ButtonEvent {
                                button_name: input.button_name,
                                event_type: ButtonEventType::RELEASING,
                            }]);
                        }
                    } else {
                        // No RELEASING action, go directly to IDLE
                        if !events_to_emit.is_empty() {
                            return StateTransition::EmitEvents(events_to_emit);
                        } else {
                            return StateTransition::Reset;
                        }
                    }
                }

                // Always reset to IDLE when physically released during EVALUATING
                println!(
                    "[{}] 🔄 Resetting button {} state: EVALUATING->IDLE (physical release, timer cancelled)",
                    timestamp,
                    input.button_name.as_str()
                );
                StateTransition::Reset
            }
            (ButtonState::HELD, false) => {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

                let has_releasing_action = {
                    let config_parser = match self.config_parser.lock() {
                        Ok(parser) => parser,
                        Err(_) => return StateTransition::Reset,
                    };
                    let result = config_parser
                        .get_actions_for_button_event(input.button_name, "RELEASING")
                        .is_some();
                    drop(config_parser);
                    result
                };

                if has_releasing_action {
                    println!(
                        "[{}] 🔄 Button {} released from HELD state - transitioning to RELEASING",
                        timestamp,
                        input.button_name.as_str()
                    );
                    state_machine.transition_to(ButtonState::RELEASING);
                    StateTransition::EmitEvents(vec![ButtonEvent {
                        button_name: input.button_name,
                        event_type: ButtonEventType::RELEASING,
                    }])
                } else {
                    println!(
                        "[{}] 🔄 Button {} released from HELD state - no RELEASING action, going to IDLE",
                        timestamp,
                        input.button_name.as_str()
                    );
                    StateTransition::Reset
                }
            }
            (ButtonState::RELEASING, false) => {
                // Button continues to be released - transition to IDLE (fully released)
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!(
                    "[{}] 🔄 Button {} fully released - transitioning to IDLE",
                    timestamp,
                    input.button_name.as_str()
                );
                StateTransition::Reset
            }
            (ButtonState::RELEASING, true) => {
                // Button was pressed again during release - go back to EVALUATING
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!(
                    "[{}] 🔄 Button {} pressed again during release - transitioning to EVALUATING",
                    timestamp,
                    input.button_name.as_str()
                );
                state_machine.transition_to(ButtonState::EVALUATING);
                state_machine.record_signal(now);
                StateTransition::Continue
            }
            (ButtonState::HELD, true) => {
                // Button continues to be held - stay in HELD state
                StateTransition::Continue
            }
            (ButtonState::IDLE, false) => {
                // No action needed when idle and no signal
                StateTransition::Continue
            }
        }
    }

    fn initial_state(&self) -> ButtonState {
        ButtonState::IDLE
    }
}
