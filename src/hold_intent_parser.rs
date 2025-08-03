use std::collections::HashMap;
use std::time::Instant;
use crate::button_state_machine::{ButtonStateMachine, StateMachineLogic, StateTransition};
use crate::button_types::{ButtonState, ButtonEvent, ButtonInput, ButtonEventType};
use crate::hold_intent_state_machine::HoldIntentLogic;
use crate::token_based_config::{PhysicalButtonName, TokenBasedParser};

pub struct HoldIntentParser {
    state_machines: HashMap<PhysicalButtonName, ButtonStateMachine<ButtonState>>,
    logic: HoldIntentLogic,
}

impl HoldIntentParser {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_parser = TokenBasedParser::new()?;
        Ok(Self {
            state_machines: HashMap::new(),
            logic: HoldIntentLogic::new(800, 300, config_parser), // 800ms evaluation window, 300ms quick release threshold
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
        // Parse HID data to extract button states
        let button_states = self.extract_button_states(data);
        
        for (button_name, is_pressed) in button_states {
            let input = ButtonInput { button_name, is_pressed };
            
            // Get or create state machine for this button
            let state_machine = self.state_machines
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

    fn extract_button_states(&self, data: &[u8]) -> Vec<(PhysicalButtonName, bool)> {
        if data.len() < 8 {
            return vec![];
        }

        let mut button_states = vec![];
        
        // Extract button states from HID data
        // Based on the original parsing logic
        if data[4] & 0x01 != 0 {
            button_states.push((PhysicalButtonName::Button0, true)); // button_0
        }
        if data[5] & 0x01 != 0 {
            button_states.push((PhysicalButtonName::Button1, true)); // button_1
        }
        if data[6] & 0x01 != 0 {
            button_states.push((PhysicalButtonName::Button2, true)); // button_2
        }
        
        // IMPORTANT: We need to also generate release events for buttons that are no longer pressed
        // but were previously in a pressed state
        for (&button_name, state_machine) in &self.state_machines {
            let button_index = match button_name {
                PhysicalButtonName::Button0 => 4,
                PhysicalButtonName::Button1 => 5,
                PhysicalButtonName::Button2 => 6,
            };
            
            let is_currently_pressed = data[button_index] & 0x01 != 0;
            let was_in_active_state = matches!(state_machine.state(), 
                ButtonState::EVALUATING | ButtonState::HELD);
            
            if !is_currently_pressed && was_in_active_state {
                button_states.push((button_name, false));
            }
        }

        button_states
    }

    pub fn process_button_timeouts<F>(&mut self, now: Instant, mut event_handler: F) -> Result<(), Box<dyn std::error::Error>>
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
            let input = ButtonInput { button_name, is_pressed: false };
            
            if let Some(state_machine) = self.state_machines.get_mut(&button_name) {
                match self.logic.process_input(state_machine, input, now) {
                    StateTransition::Continue => {}
                    StateTransition::EmitEvents(events) => {
                        for event in events {
                            if matches!(event.event_type, ButtonEventType::RELEASED) {
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
        }
        
        Ok(())
    }
}

// Re-export the types that are still used by the action manager
