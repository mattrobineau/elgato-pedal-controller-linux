use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::token_based_config::PhysicalButtonName;
use chrono;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ButtonState {
    IDLE,           // No recent activity
    EVALUATING,     // Received first signal, evaluating user intent
}

#[derive(Debug, Copy, Clone)]
pub enum ButtonEventType {
    PRESSED,
    HELD,
    RELEASED,
}

impl ButtonEventType {
    pub fn as_str(&self) -> &str {
        match self {
            ButtonEventType::PRESSED => "PRESSED",
            ButtonEventType::HELD => "HELD", 
            ButtonEventType::RELEASED => "RELEASED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonEvent {
    pub button_name: PhysicalButtonName,
    pub event_type: ButtonEventType,
}

#[derive(Debug)]
struct ButtonTracker {
    state: ButtonState,
    first_signal_time: Option<Instant>,
    last_signal_time: Option<Instant>,
    signal_count: u32,
    action_fired: bool,
}

impl ButtonTracker {
    fn new() -> Self {
        Self {
            state: ButtonState::IDLE,
            first_signal_time: None,
            last_signal_time: None,
            signal_count: 0,
            action_fired: false,
        }
    }
}

pub struct HoldIntentParser {
    buttons: HashMap<PhysicalButtonName, ButtonTracker>,
    evaluation_window_ms: u64, // How long to wait for additional signals to determine intent
}

impl HoldIntentParser {
    pub fn new() -> Self {
        let mut buttons = HashMap::new();
        buttons.insert(PhysicalButtonName::Button1, ButtonTracker::new()); // button_0
        buttons.insert(PhysicalButtonName::Button2, ButtonTracker::new()); // button_1  
        buttons.insert(PhysicalButtonName::Button3, ButtonTracker::new()); // button_2
        
        Self { 
            buttons,
            evaluation_window_ms: 800, // Wait 800ms after first signal to evaluate intent
        }
    }

    pub fn parse_hid_data<F, G, H>(&mut self, 
                                   data: &[u8], 
                                   now: Instant,
                                   has_held_action: F,
                                   has_pressed_action: G, 
                                   get_hold_threshold: H) -> Vec<ButtonEvent>
    where
        F: Fn(&PhysicalButtonName) -> bool,
        G: Fn(&PhysicalButtonName) -> bool,
        H: Fn(&PhysicalButtonName) -> u64,
    {
        let mut events = Vec::new();

        if data.len() < 8 || data[0] != 1 || data[2] != 3 {
            return events;
        }

        // Check each button state from HID data
        let button_data = [
            (PhysicalButtonName::Button1, data[4] != 0), // button_0 (left pedal)
            (PhysicalButtonName::Button2, data[5] != 0), // button_1 (middle pedal)
            (PhysicalButtonName::Button3, data[6] != 0), // button_2 (right pedal)
        ];

        for (button_name, is_pressed) in button_data {
            if let Some(tracker) = self.buttons.get_mut(&button_name) {
                
                match (tracker.state, is_pressed) {
                    (ButtonState::IDLE, true) => {
                        // First signal detected - start evaluation
                        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                        println!("[{}] 🔄 Button {} signal detected - starting intent evaluation (state: IDLE->EVALUATING)", timestamp, button_name.as_str());
                        tracker.state = ButtonState::EVALUATING;
                        tracker.first_signal_time = Some(now);
                        tracker.last_signal_time = Some(now);
                        tracker.signal_count = 1;
                        tracker.action_fired = false;
                        
                        let has_pressed = has_pressed_action(&button_name);
                        let has_held = has_held_action(&button_name);
                        let threshold_ms = get_hold_threshold(&button_name);
                        
                        println!("[{}] 🔍 Button {} config: has_pressed={}, has_held={}, threshold={}ms, evaluation_window={}ms", 
                                 timestamp, button_name.as_str(), has_pressed, has_held, threshold_ms, self.evaluation_window_ms);
                        
                        if threshold_ms > 0 {
                            let _threshold_time = now + Duration::from_millis(threshold_ms);
                            println!("[{}] ⏱️  Hold threshold timer started - will fire HELD at {}", 
                                     timestamp, 
                                     chrono::Local::now().checked_add_signed(chrono::Duration::milliseconds(threshold_ms as i64))
                                         .unwrap_or_else(chrono::Local::now).format("%H:%M:%S%.3f"));
                        }
                        
                        if has_pressed && !has_held {
                            // PRESSED-only button: Fire immediately
                            println!("[{}] ⚡ Immediate PRESSED for {} (PRESSED-only button)", timestamp, button_name.as_str());
                            tracker.action_fired = true;
                            events.push(ButtonEvent {
                                button_name,
                                event_type: ButtonEventType::PRESSED,
                            });
                        }
                        // For HELD-only and PRESSED+HELD buttons, wait for threshold timing
                    },
                    
                    (ButtonState::EVALUATING, true) => {
                        // Additional signal - user might be holding (rare with Elgato)
                        tracker.signal_count += 1;
                        tracker.last_signal_time = Some(now);
                        
                        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                        println!("[{}] 🔄 Button {} additional signal #{} detected", 
                                 timestamp, button_name.as_str(), tracker.signal_count);
                        
                        // If we get multiple signals on PRESSED+HELD button, fire HELD
                        if tracker.signal_count >= 2 && !tracker.action_fired {
                            let has_held = has_held_action(&button_name);
                            let has_pressed = has_pressed_action(&button_name);
                            if has_held && has_pressed {
                                println!("[{}] 🔥 HELD event for {} (multiple signals on PRESSED+HELD button)", 
                                         timestamp, button_name.as_str());
                                tracker.action_fired = true;
                                events.push(ButtonEvent {
                                    button_name,
                                    event_type: ButtonEventType::HELD,
                                });
                            }
                        }
                    },
                    
                    (ButtonState::EVALUATING, false) => {
                        // No HID signal, but still evaluating intent
                        if let Some(first_signal) = tracker.first_signal_time {
                            let time_since_first = now.duration_since(first_signal);
                            let has_pressed = has_pressed_action(&button_name);
                            let has_held = has_held_action(&button_name);
                            let threshold_ms = get_hold_threshold(&button_name);
                            
                            // For PRESSED+HELD buttons: Only fire PRESSED if user releases very quickly (within 300ms)
                            // This accounts for the fact that Elgato device stops sending HID signals after ~100ms
                            // even when user is still physically holding the button
                            let quick_release_threshold = 300; // ms - conservative threshold for actual quick releases
                            if !tracker.action_fired && has_pressed && has_held && 
                               (time_since_first.as_millis() as u64) < quick_release_threshold &&
                               (time_since_first.as_millis() as u64) < threshold_ms {
                                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                println!("[{}] ⚡ Quick release detected for {} ({}ms elapsed < {}ms quick-release threshold) - firing PRESSED", 
                                         timestamp, button_name.as_str(), time_since_first.as_millis(), quick_release_threshold);
                                tracker.action_fired = true;
                                events.push(ButtonEvent {
                                    button_name,
                                    event_type: ButtonEventType::PRESSED,
                                });
                            }
                            
                            // Check if we've reached the hold threshold for HELD actions
                            if !tracker.action_fired && has_held && time_since_first.as_millis() as u64 >= threshold_ms {
                                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                println!("[{}] ⏰ Hold threshold reached for {} ({}ms elapsed >= {}ms threshold, action_fired={})", 
                                         timestamp, button_name.as_str(), time_since_first.as_millis(), threshold_ms, tracker.action_fired);
                                
                                if has_held && !has_pressed {
                                    // HELD-only button: Fire HELD after threshold
                                    println!("[{}] 🔥 HELD event for {} (HELD-only button - threshold reached)", 
                                             timestamp, button_name.as_str());
                                } else if has_held && has_pressed {
                                    // PRESSED+HELD button: Fire HELD after threshold
                                    println!("[{}] 🔥 HELD event for {} (PRESSED+HELD button - threshold reached)", 
                                             timestamp, button_name.as_str());
                                }
                                
                                tracker.action_fired = true;
                                events.push(ButtonEvent {
                                    button_name,
                                    event_type: ButtonEventType::HELD,
                                });
                            } else if has_held && time_since_first.as_millis() as u64 >= threshold_ms {
                                // Threshold reached but action already fired - log this
                                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                println!("[{}] ⚠️  Hold threshold reached for {} but action already fired (action_fired={})", 
                                         timestamp, button_name.as_str(), tracker.action_fired);
                            }
                            
                            // Determine the effective evaluation window - use the longer of evaluation_window or threshold
                            let effective_window = if has_held { 
                                std::cmp::max(self.evaluation_window_ms, threshold_ms + 100) // Add 100ms buffer after threshold
                            } else {
                                self.evaluation_window_ms
                            };
                            
                            // If effective evaluation window has passed, make final decision
                            if time_since_first.as_millis() as u64 >= effective_window {
                                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                println!("[{}] 🔄 Button {} evaluation complete - signals: {}, window: {}ms (effective: {}ms)", 
                                         timestamp, button_name.as_str(), tracker.signal_count, time_since_first.as_millis(), effective_window);
                                
                                // Only fire action if none fired yet (for PRESSED+HELD buttons)
                                if !tracker.action_fired && has_pressed && has_held {
                                    // PRESSED+HELD button with single signal = PRESSED
                                    println!("[{}] 🔥 PRESSED event for {} (single signal on PRESSED+HELD button)", 
                                             timestamp, button_name.as_str());
                                    events.push(ButtonEvent {
                                        button_name,
                                        event_type: ButtonEventType::PRESSED,
                                    });
                                }
                                
                                // Reset state and fire RELEASED
                                let timestamp_reset = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                println!("[{}] 🔄 Resetting button {} state: EVALUATING->IDLE, clearing all tracking data", 
                                         timestamp_reset, button_name.as_str());
                                tracker.state = ButtonState::IDLE;
                                tracker.first_signal_time = None;
                                tracker.last_signal_time = None;
                                tracker.signal_count = 0;
                                tracker.action_fired = false;
                                
                                let timestamp_final = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                println!("[{}] ⏹️  Hold threshold timer ended for {}", timestamp_final, button_name.as_str());
                                
                                events.push(ButtonEvent {
                                    button_name,
                                    event_type: ButtonEventType::RELEASED,
                                });
                            }
                        }
                    },
                    
                    (ButtonState::IDLE, false) => {
                        // Button remains idle - no action needed
                    }
                }
            }
        }

        events
    }
}
