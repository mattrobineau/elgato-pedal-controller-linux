use std::time::Instant;
use std::sync::{Arc, Mutex};
use crate::button_state_machine::{ButtonStateMachine, StateMachineLogic, StateTransition};
use crate::button_types::{ButtonState, ButtonEventType, ButtonEvent, ButtonInput};
use crate::token_based_config::{PhysicalButtonName, TokenBasedParser};

// Configuration for button behavior
#[derive(Debug, Clone)]
pub struct ButtonConfig {
    pub has_pressed_action: bool,
    pub has_held_action: bool,
    pub threshold_ms: u64,
}

/// Hold intent detection logic
pub struct HoldIntentLogic {
    evaluation_window_ms: u64,
    quick_release_threshold_ms: u64,
    config_parser: Arc<Mutex<TokenBasedParser>>,
}

impl HoldIntentLogic {
    pub fn new(evaluation_window_ms: u64, quick_release_threshold_ms: u64, config_parser: Arc<Mutex<TokenBasedParser>>) -> Self {
        Self {
            evaluation_window_ms,
            quick_release_threshold_ms,
            config_parser,
        }
    }

    pub fn get_button_config(&self, button_name: &PhysicalButtonName) -> ButtonConfig {
        // Use the cached config parser instead of creating a new one each time
        let config_parser = self.config_parser.lock().unwrap();
        let has_pressed_action = config_parser.get_actions_for_button_event(*button_name, "PRESSED").is_some();
        let has_held_action = config_parser.get_actions_for_button_event(*button_name, "HELD").is_some();
        let threshold_ms = 1000; // Default 1 second threshold
        
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
        println!("[{}] 🔄 Button {} signal detected - starting intent evaluation (state: IDLE->EVALUATING)", 
                 timestamp, input.button_name.as_str());
        
        state_machine.transition_to(ButtonState::EVALUATING);
        state_machine.record_signal(now);
        
        println!("[{}] 🔍 Button {} config: has_pressed={}, has_held={}, threshold={}ms, evaluation_window={}ms", 
                 timestamp, input.button_name.as_str(), config.has_pressed_action, config.has_held_action, 
                 config.threshold_ms, self.evaluation_window_ms);
        
        if config.threshold_ms > 0 {
            println!("[{}] ⏱️  Hold threshold timer started - will fire HELD at {}", 
                     timestamp, 
                     chrono::Local::now().checked_add_signed(chrono::Duration::milliseconds(config.threshold_ms as i64))
                         .unwrap_or_else(chrono::Local::now).format("%H:%M:%S%.3f"));
        }
        
        if config.has_pressed_action && !config.has_held_action {
            // PRESSED-only button: Fire immediately
            println!("[{}] ⚡ Immediate PRESSED for {} (PRESSED-only button)", timestamp, input.button_name.as_str());
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
        println!("[{}] 🔄 Button {} additional signal #{} detected", 
                 timestamp, input.button_name.as_str(), state_machine.signal_count());
        
        // If we get multiple signals on PRESSED+HELD button, fire HELD
        if state_machine.signal_count() >= 2 && !state_machine.action_fired() {
            if config.has_held_action && config.has_pressed_action {
                println!("[{}] 🔥 HELD event for {} (multiple signals on PRESSED+HELD button)", 
                         timestamp, input.button_name.as_str());
                state_machine.mark_action_fired();
                // Don't change state - let HID data drive state transitions
                return StateTransition::EmitEvents(vec![ButtonEvent {
                    button_name: input.button_name,
                    event_type: ButtonEventType::HELD,
                }]);
            }
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
                if let Some(time_since_first) = state_machine.time_since_first_signal(now) {
                    if (time_since_first.as_millis() as u64) >= config.threshold_ms {
                        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                        println!("[{}] 🔄 Transitioning to HELD state for {} (threshold reached, button still pressed)", 
                                 timestamp, input.button_name.as_str());
                        state_machine.transition_to(ButtonState::HELD);
                    }
                }
                
                self.handle_evaluating_with_signal(state_machine, &input, &config, now)
            }
            (ButtonState::EVALUATING, false) => {
                // CRITICAL: Physical button release during EVALUATING - cancel hold threshold timer!
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                
                if let Some(time_since_first) = state_machine.time_since_first_signal(now) {
                    let time_elapsed_ms = time_since_first.as_millis() as u64;
                    
                    println!("[{}] 🛑 Physical button release detected during EVALUATING for {} ({}ms elapsed < {}ms threshold)", 
                             timestamp, input.button_name.as_str(), time_elapsed_ms, config.threshold_ms);
                    println!("[{}] ❌ Cancelling hold threshold timer - HELD state now impossible", 
                             timestamp);
                    
                    // Handle different button configurations for early release
                    let mut events_to_emit = vec![];
                    
                    if config.has_pressed_action && config.has_held_action {
                        // PRESSED+HELD button: Check for quick release
                        if time_elapsed_ms < self.quick_release_threshold_ms && !state_machine.action_fired() {
                            println!("[{}] ⚡ Quick release detected for {} ({}ms elapsed < {}ms quick-release threshold) - firing PRESSED", 
                                     timestamp, input.button_name.as_str(), time_elapsed_ms, self.quick_release_threshold_ms);
                            state_machine.mark_action_fired();
                            events_to_emit.push(ButtonEvent {
                                button_name: input.button_name,
                                event_type: ButtonEventType::PRESSED,
                            });
                        } else if !state_machine.action_fired() {
                            // Released too late for PRESSED, too early for HELD - no action
                            println!("[{}] 🔄 Button {} released too late for PRESSED ({}ms > {}ms), too early for HELD ({}ms < {}ms) - no action fired", 
                                     timestamp, input.button_name.as_str(), time_elapsed_ms, self.quick_release_threshold_ms, time_elapsed_ms, config.threshold_ms);
                        }
                    } else if config.has_held_action && !config.has_pressed_action {
                        // HELD-only button: No action since threshold wasn't reached
                        if !state_machine.action_fired() {
                            println!("[{}] 🔄 HELD-only button {} released before threshold ({}ms < {}ms) - no action fired", 
                                     timestamp, input.button_name.as_str(), time_elapsed_ms, config.threshold_ms);
                        }
                    } else if config.has_pressed_action && !config.has_held_action {
                        // PRESSED-only button: Should have fired immediately on press, but handle edge case
                        if !state_machine.action_fired() {
                            println!("[{}] ⚡ Late PRESSED action for {} (PRESSED-only button released)", 
                                     timestamp, input.button_name.as_str());
                            state_machine.mark_action_fired();
                            events_to_emit.push(ButtonEvent {
                                button_name: input.button_name,
                                event_type: ButtonEventType::PRESSED,
                            });
                        }
                    }
                    
                    // Check if we should transition to RELEASING state
                    let config_parser = self.config_parser.lock().unwrap();
                    let has_releasing_action = config_parser.get_actions_for_button_event(input.button_name, "RELEASING").is_some();
                    drop(config_parser);
                    
                    if (state_machine.action_fired() || has_releasing_action) && has_releasing_action {
                        // Transition to RELEASING state to allow RELEASING event to fire
                        println!("[{}] 🔄 Transitioning {} to RELEASING state (action was fired: {}, RELEASING configured: {})", 
                                 timestamp, input.button_name.as_str(), state_machine.action_fired(), has_releasing_action);
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
                        state_machine.reset(ButtonState::IDLE);
                        
                        if !events_to_emit.is_empty() {
                            return StateTransition::EmitEvents(events_to_emit);
                        }
                    }
                }
                
                // Always reset to IDLE when physically released during EVALUATING
                println!("[{}] 🔄 Resetting button {} state: EVALUATING->IDLE (physical release, timer cancelled)", 
                         timestamp, input.button_name.as_str());
                state_machine.reset(ButtonState::IDLE);
                StateTransition::Continue
            }
            (ButtonState::HELD, false) => {
                // Button was released from HELD state - check if RELEASING is configured
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                
                let config_parser = self.config_parser.lock().unwrap();
                let has_releasing_action = config_parser.get_actions_for_button_event(input.button_name, "RELEASING").is_some();
                drop(config_parser);
                
                if has_releasing_action {
                    println!("[{}] 🔄 Button {} released from HELD state - transitioning to RELEASING", 
                             timestamp, input.button_name.as_str());
                    state_machine.transition_to(ButtonState::RELEASING);
                    StateTransition::EmitEvents(vec![ButtonEvent {
                        button_name: input.button_name,
                        event_type: ButtonEventType::RELEASING,
                    }])
                } else {
                    println!("[{}] 🔄 Button {} released from HELD state - no RELEASING action, going to IDLE", 
                             timestamp, input.button_name.as_str());
                    state_machine.reset(ButtonState::IDLE);
                    StateTransition::Continue
                }
            }
            (ButtonState::RELEASING, false) => {
                // Button continues to be released - transition to IDLE (fully released)
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] 🔄 Button {} fully released - transitioning to IDLE", 
                         timestamp, input.button_name.as_str());
                state_machine.reset(ButtonState::IDLE);
                StateTransition::Continue
            }
            (ButtonState::RELEASING, true) => {
                // Button was pressed again during release - go back to EVALUATING
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] 🔄 Button {} pressed again during release - transitioning to EVALUATING", 
                         timestamp, input.button_name.as_str());
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
