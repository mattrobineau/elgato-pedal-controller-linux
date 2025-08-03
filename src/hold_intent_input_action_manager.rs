use std::time::Instant;
use crate::hold_intent_parser::{HoldIntentParser, ButtonEvent, ButtonEventType};
use crate::token_based_config::{TokenBasedParser, PhysicalButtonName};
use crate::input_simulator::InputSimulator;

pub struct HoldIntentInputActionManager {
    parser: HoldIntentParser,
    config: TokenBasedParser,
    input_simulator: InputSimulator,
}

impl HoldIntentInputActionManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let parser = HoldIntentParser::new();
        let config = TokenBasedParser::new()?;
        let input_simulator = InputSimulator::new()?;
        
        Ok(Self {
            parser,
            config,
            input_simulator,
        })
    }

    pub fn process_hid_data(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        
        // Get button events from parser
        let events = self.parser.parse_hid_data(
            data,
            now,
            |button: &PhysicalButtonName| self.config.button_has_held_action(*button),
            |button: &PhysicalButtonName| self.config.button_has_pressed_action(*button),
            |button: &PhysicalButtonName| self.config.get_hold_threshold_ms_for_button(*button),
        );

        // Process each event
        for event in events {
            self.handle_button_event(event)?;
        }

        Ok(())
    }

    fn handle_button_event(&mut self, event: ButtonEvent) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Button {} event: {} -> executing actions", 
                 event.button_name.as_str(), event.event_type.as_str());

        let actions = match event.event_type {
            ButtonEventType::PRESSED => {
                self.config.get_actions_for_button_event(event.button_name, "PRESSED")
            },
            ButtonEventType::HELD => {
                self.config.get_actions_for_button_event(event.button_name, "HELD")
            },
            ButtonEventType::RELEASED => {
                self.config.get_actions_for_button_event(event.button_name, "RELEASED")
            },
        };

        if let Some(actions) = actions {
            println!("Button {} event: {}", event.button_name.as_str(), event.event_type.as_str());
            println!("Executing {} actions", actions.len());
            
            for (i, action) in actions.iter().enumerate() {
                println!("Executing action {}: {:?}", i + 1, action);
            }
            
            match self.input_simulator.execute_actions(&actions) {
                Ok(_) => {},
                Err(e) => eprintln!("Failed to execute actions: {}", e),
            }
        } else {
            println!("No actions configured for button {} event {}", 
                     event.button_name.as_str(), event.event_type.as_str());
        }

        Ok(())
    }
}
