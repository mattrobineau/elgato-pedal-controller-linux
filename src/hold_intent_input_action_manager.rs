use crate::button_types::{ButtonEvent, ButtonEventType};
use crate::config_manager::ConfigManager;
use crate::hold_intent_parser::HoldIntentParser;
use crate::input_simulator::InputSimulator;
use crate::token_based_config::TokenBasedParser;
use anyhow::{Context, Result, anyhow};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct HoldIntentInputActionManager {
    parser: HoldIntentParser,
    config: Arc<Mutex<TokenBasedParser>>,
    input_simulator: InputSimulator,
}

impl HoldIntentInputActionManager {
    pub fn new(global_default_threshold_ms: u64) -> Result<Self> {
        let parser = HoldIntentParser::new(global_default_threshold_ms)
            .context("Failed to create HoldIntentParser.")?;
        let config_manager = ConfigManager::global();
        let config = config_manager.get_parser();
        let input_simulator = InputSimulator::new().context("Failed to create InputSimulator.")?;

        Ok(HoldIntentInputActionManager {
            parser,
            config,
            input_simulator,
        })
    }

    pub fn process_hid_data(&mut self, data: &[u8]) -> Result<()> {
        let now = Instant::now();

        // Collect events first to avoid borrowing issues
        let mut events = Vec::new();
        self.parser
            .parse_hid_data(data, now, |event| {
                events.push(event);
            })
            .context("Failed to parse HID data.")?;

        // Then process the collected events
        for event in events {
            if let Err(e) = self.handle_button_event(event) {
                eprintln!("Error handling button event: {e}");
            }
        }

        Ok(())
    }

    /// Process any pending timer-based events (scheduled releases, timeouts, etc.)
    /// This should be called regularly even when no HID data is received
    pub fn process_timers(&mut self) -> Result<()> {
        self.input_simulator
            .process_scheduled_releases()
            .context("Failed to process scheduled releases.")?;
        Ok(())
    }

    /// Process button timeout events (evaluation windows, hold thresholds, etc.)
    /// This should be called regularly to handle state machine timeouts
    pub fn process_button_timeouts(&mut self) -> Result<()> {
        let now = std::time::Instant::now();

        let mut events = Vec::new();
        self.parser
            .process_button_timeouts(now, |event| {
                events.push(event);
            })
            .context("Failed to process button timeouts.")?;

        // Then process the collected events
        for event in events {
            if let Err(e) = self.handle_button_event(event) {
                eprintln!("Error handling timeout event: {e}");
            }
        }

        Ok(())
    }

    fn handle_button_event(&mut self, event: ButtonEvent) -> Result<()> {
        println!(
            "Button {} event: {} -> executing actions",
            event.button_name.as_str(),
            event.event_type.as_str()
        );

        let config = self
            .config
            .lock()
            .map_err(|e| anyhow!("Failed to lock config: {}", e))?;

        let actions = match event.event_type {
            ButtonEventType::PRESSED => {
                config.get_actions_for_button_event(event.button_name, "PRESSED")
            }
            ButtonEventType::HELD => config.get_actions_for_button_event(event.button_name, "HELD"),
            ButtonEventType::RELEASING => {
                println!(
                    "Looking for RELEASING actions for {}",
                    event.button_name.as_str()
                );
                let releasing_actions =
                    config.get_actions_for_button_event(event.button_name, "RELEASING");
                if releasing_actions.is_some() {
                    println!("✅ Found RELEASING actions!");
                } else {
                    println!("❌ No RELEASING actions found");
                }
                releasing_actions
            }
        };
        drop(config);

        if let Some(actions) = actions {
            println!(
                " Button {} event: {}",
                event.button_name.as_str(),
                event.event_type.as_str()
            );
            println!("> Executing {} actions", actions.len());

            for (i, action) in actions.iter().enumerate() {
                println!(" Executing action {}: {:?}", i + 1, action);
            }

            match self.input_simulator.execute_actions(&actions) {
                Ok(_) => {}
                Err(e) => eprintln!("Failed to execute actions: {e}"),
            }
        } else {
            println!(
                "No actions configured for button {} event {}",
                event.button_name.as_str(),
                event.event_type.as_str()
            );
        }

        Ok(())
    }
}
