use std::time::Instant;
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
    config_parser: TokenBasedParser,
}

impl HoldIntentLogic {
    pub fn new(evaluation_window_ms: u64, quick_release_threshold_ms: u64, config_parser: TokenBasedParser) -> Self {
        Self {
            evaluation_window_ms,
            quick_release_threshold_ms,
            config_parser,
        }
    }

    fn get_button_config(&self, button_name: &PhysicalButtonName) -> ButtonConfig {
        // Use the cached config parser instead of creating a new one each time
        let has_pressed_action = self.config_parser.get_actions_for_button_event(*button_name, "PRESSED").is_some();
        let has_held_action = self.config_parser.get_actions_for_button_event(*button_name, "HELD").is_some();
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

    fn handle_evaluating_without_signal(
        &self,
        state_machine: &mut ButtonStateMachine<ButtonState>,
        input: &ButtonInput,
        config: &ButtonConfig,
        now: Instant,
    ) -> StateTransition<ButtonEvent> {
        if let Some(time_since_first) = state_machine.time_since_first_signal(now) {
            // Check for quick release on PRESSED+HELD buttons
            if !state_machine.action_fired() && config.has_pressed_action && config.has_held_action && 
               (time_since_first.as_millis() as u64) < self.quick_release_threshold_ms &&
               (time_since_first.as_millis() as u64) < config.threshold_ms {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] ⚡ Quick release detected for {} ({}ms elapsed < {}ms quick-release threshold) - firing PRESSED", 
                         timestamp, input.button_name.as_str(), time_since_first.as_millis(), self.quick_release_threshold_ms);
                state_machine.mark_action_fired();
                return StateTransition::EmitEvents(vec![ButtonEvent {
                    button_name: input.button_name,
                    event_type: ButtonEventType::PRESSED,
                }]);
            }
            
            // Check for hold threshold - fire action but don't change state here
            if !state_machine.action_fired() && config.has_held_action && 
               (time_since_first.as_millis() as u64) >= config.threshold_ms {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] ⏰ Hold threshold reached for {} ({}ms elapsed >= {}ms threshold, action_fired={})", 
                         timestamp, input.button_name.as_str(), time_since_first.as_millis(), config.threshold_ms, state_machine.action_fired());
                
                if config.has_held_action && !config.has_pressed_action {
                    // HELD-only button: Fire HELD after threshold
                    println!("[{}] 🔥 HELD action for {} (HELD-only button - threshold reached)", 
                             timestamp, input.button_name.as_str());
                } else if config.has_held_action && config.has_pressed_action {
                    // PRESSED+HELD button: Fire HELD after threshold
                    println!("[{}] 🔥 HELD action for {} (PRESSED+HELD button - threshold reached)", 
                             timestamp, input.button_name.as_str());
                }
                
                // Mark that we've fired the action, but DON'T change state
                // State transitions should be driven by actual button release (HID data)
                state_machine.mark_action_fired();
                return StateTransition::EmitEvents(vec![ButtonEvent {
                    button_name: input.button_name,
                    event_type: ButtonEventType::HELD,
                }]);
            }
            
            // Check if evaluation window has expired
            let effective_window = std::cmp::max(self.evaluation_window_ms, config.threshold_ms + 100);
            if (time_since_first.as_millis() as u64) >= effective_window {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                
                println!("[{}] 🔄 Button {} evaluation complete - signals: {}, window: {}ms (effective: {}ms)", 
                         timestamp, input.button_name.as_str(), state_machine.signal_count(), 
                         time_since_first.as_millis(), effective_window);
                
                if state_machine.action_fired() {
                    // Action was already fired - check if we should transition to HELD state
                    if (time_since_first.as_millis() as u64) >= config.threshold_ms {
                        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                        println!("[{}] 🔄 Transitioning to HELD state for {} (action fired, threshold passed)", 
                                 timestamp, input.button_name.as_str());
                        state_machine.transition_to(ButtonState::HELD);
                        return StateTransition::Continue;
                    } else {
                        println!("[{}] ⚠️  Action already fired for {} - evaluation window expired but button still held", 
                                 timestamp, input.button_name.as_str());
                        println!("[{}] 🔄 Staying in current state - no RELEASING event needed", 
                                 timestamp);
                        return StateTransition::Continue;
                    }
                } else {
                    // No action was fired during evaluation - this means button was released quickly
                    println!("[{}] 🔄 No action fired for {} - treating as quick release", 
                             timestamp, input.button_name.as_str());
                    println!("[{}] 🔄 Resetting button {} state: EVALUATING->IDLE", 
                             timestamp, input.button_name.as_str());
                    println!("[{}] ⏹️  Hold threshold timer ended for {}", timestamp, input.button_name.as_str());
                    
                    let events = vec![ButtonEvent {
                        button_name: input.button_name,
                        event_type: ButtonEventType::RELEASING,
                    }];
                    
                    return StateTransition::EmitEvents(events);
                }
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
                self.handle_evaluating_without_signal(state_machine, &input, &config, now)
            }
            (ButtonState::HELD, false) => {
                // Button was released from HELD state - transition to RELEASING
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] 🔄 Button {} released from HELD state - transitioning to RELEASING", 
                         timestamp, input.button_name.as_str());
                state_machine.transition_to(ButtonState::RELEASING);
                StateTransition::EmitEvents(vec![ButtonEvent {
                    button_name: input.button_name,
                    event_type: ButtonEventType::RELEASING,
                }])
            }
            (ButtonState::RELEASING, false) => {
                // Button continues to be released - transition to IDLE (fully released)
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] 🔄 Button {} fully released - transitioning to IDLE", 
                         timestamp, input.button_name.as_str());
                state_machine.reset(ButtonState::IDLE);
                StateTransition::EmitEvents(vec![ButtonEvent {
                    button_name: input.button_name,
                    event_type: ButtonEventType::RELEASED,
                }])
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
