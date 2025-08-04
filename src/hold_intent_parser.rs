use crate::button_state_machine::{ButtonStateMachine, StateMachineLogic, StateTransition};
use crate::button_types::{ButtonEvent, ButtonEventType, ButtonInput, ButtonState};
use crate::config_manager::ConfigManager;
use crate::hold_intent_state_machine::HoldIntentLogic;
use crate::token_based_config::PhysicalButtonName;
use std::collections::HashMap;
use std::time::Instant;

pub struct HoldIntentParser {
    state_machines: HashMap<PhysicalButtonName, ButtonStateMachine<ButtonState>>,
    logic: HoldIntentLogic,
    previous_button_states: HashMap<PhysicalButtonName, bool>, // Track previous states
}

impl HoldIntentParser {
    pub fn new(global_default_threshold_ms: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let config_manager = ConfigManager::global();
        let config_parser = config_manager.get_parser();
        Ok(Self {
            state_machines: HashMap::new(),
            logic: HoldIntentLogic::new(global_default_threshold_ms, config_parser), // Use dynamic thresholds based on button configuration
            previous_button_states: HashMap::new(),
        })
    }

    pub fn parse_hid_data<F>(
        &mut self,
        data: &[u8],
        now: Instant,
        mut event_handler: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(ButtonEvent),
    {
        println!("🔍 Parsing HID data: {data:?}");

        // Parse HID data to extract button states
        let button_states = self.extract_button_states(data);

        println!("🔍 Extracted button states: {button_states:?}");

        for (button_name, is_pressed) in button_states {
            let input = ButtonInput {
                button_name,
                is_pressed,
            };

            println!(
                "🔍 Processing input: button={}, is_pressed={}",
                button_name.as_str(),
                is_pressed
            );

            // Get or create state machine for this button
            let state_machine = self
                .state_machines
                .entry(button_name)
                .or_insert_with(|| ButtonStateMachine::new(self.logic.initial_state()));

            // Process the input through the state machine
            match self.logic.process_input(state_machine, input, now) {
                StateTransition::Continue => {
                    // No events to emit, continue processing
                }
                StateTransition::EmitEvents(events) => {
                    for event in events {
                        // Check if this is a RELEASING event, which should reset the state machine
                        if matches!(event.event_type, ButtonEventType::RELEASING) {
                            state_machine.reset(self.logic.initial_state());
                        }
                        event_handler(event);
                    }
                }
                StateTransition::Reset => {
                    state_machine.reset(self.logic.initial_state());
                }
            }
        }

        // Note: Timeout processing is now handled separately via process_button_timeouts()
        // This avoids conflicts between HID data processing and timeout logic

        Ok(())
    }

    fn extract_button_states(&mut self, data: &[u8]) -> Vec<(PhysicalButtonName, bool)> {
        if data.len() < 8 {
            return vec![];
        }

        let mut button_states = vec![];

        // Extract current button states from HID data
        let current_button_0 = data[4] & 0x01 != 0;
        let current_button_1 = data[5] & 0x01 != 0;
        let current_button_2 = data[6] & 0x01 != 0;

        // Check for state changes and generate events only on transitions
        self.check_button_transition(
            PhysicalButtonName::Button0,
            current_button_0,
            &mut button_states,
        );
        self.check_button_transition(
            PhysicalButtonName::Button1,
            current_button_1,
            &mut button_states,
        );
        self.check_button_transition(
            PhysicalButtonName::Button2,
            current_button_2,
            &mut button_states,
        );

        button_states
    }

    fn check_button_transition(
        &mut self,
        button_name: PhysicalButtonName,
        current_state: bool,
        button_states: &mut Vec<(PhysicalButtonName, bool)>,
    ) {
        let previous_state = self
            .previous_button_states
            .get(&button_name)
            .copied()
            .unwrap_or(false);

        // Only generate events on state transitions
        if current_state != previous_state {
            println!(
                "🔄 Button {} state transition: {} -> {}",
                button_name.as_str(),
                if previous_state {
                    "PRESSED"
                } else {
                    "RELEASED"
                },
                if current_state { "PRESSED" } else { "RELEASED" }
            );
            button_states.push((button_name, current_state));
        }

        // Update the stored state
        self.previous_button_states
            .insert(button_name, current_state);
    }

    pub fn process_button_timeouts<F>(
        &mut self,
        now: Instant,
        mut event_handler: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(ButtonEvent),
    {
        let mut buttons_to_process = vec![];

        // Collect buttons that need timeout processing
        for (&button_name, state_machine) in &self.state_machines {
            if state_machine.state() == ButtonState::EVALUATING {
                buttons_to_process.push(button_name);
            }
        }

        // Process timeouts for evaluating buttons
        for button_name in buttons_to_process {
            if let Some(state_machine) = self.state_machines.get_mut(&button_name) {
                // For timeout processing, we need to check if hold threshold has been reached
                // without simulating a button release
                if let Some(time_since_first) = state_machine.time_since_first_signal(now) {
                    let config = self.logic.get_button_config(&button_name);

                    // Check if hold threshold has been reached and no action has been fired yet
                    if (time_since_first.as_millis() as u64) >= config.threshold_ms
                        && !state_machine.action_fired()
                    {
                        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                        println!(
                            "[{}] ⏰ Hold threshold reached for {} ({}ms elapsed >= {}ms threshold, action_fired={})",
                            timestamp,
                            button_name.as_str(),
                            time_since_first.as_millis(),
                            config.threshold_ms,
                            state_machine.action_fired()
                        );

                        if config.has_held_action {
                            if config.has_held_action && !config.has_pressed_action {
                                // HELD-only button: Fire HELD after threshold
                                println!(
                                    "[{}] 🔥 HELD action for {} (HELD-only button - threshold reached)",
                                    timestamp,
                                    button_name.as_str()
                                );
                            } else if config.has_held_action && config.has_pressed_action {
                                // PRESSED+HELD button: Fire HELD after threshold
                                println!(
                                    "[{}] 🔥 HELD action for {} (PRESSED+HELD button - threshold reached)",
                                    timestamp,
                                    button_name.as_str()
                                );
                            }

                            // Mark that we've fired the action AND transition to HELD state
                            state_machine.mark_action_fired();
                            state_machine.transition_to(ButtonState::HELD);
                            println!(
                                "[{}] 🔄 Transitioning to HELD state for {} (action fired, threshold passed)",
                                timestamp,
                                button_name.as_str()
                            );

                            let event = ButtonEvent {
                                button_name,
                                event_type: ButtonEventType::HELD,
                            };
                            event_handler(event);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// Re-export the types that are still used by the action manager
