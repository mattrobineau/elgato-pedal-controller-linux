use crate::token_based_config::PhysicalButtonName;

/// Generic button states that can be used by any button detection system
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ButtonState {
    IDLE,       // No recent activity
    EVALUATING, // Received first signal, evaluating user intent
    HELD,       // Button is being held down
    RELEASING,  // Button is in the process of being released
}

/// Types of button events that can be generated
#[derive(Debug, Copy, Clone)]
pub enum ButtonEventType {
    PRESSED,
    HELD,
    RELEASING,  // Button is being released (transition event)
    RELEASED,   // Button has been fully released (final state)
}

impl ButtonEventType {
    pub fn as_str(&self) -> &str {
        match self {
            ButtonEventType::PRESSED => "PRESSED",
            ButtonEventType::HELD => "HELD",
            ButtonEventType::RELEASING => "RELEASING",
            ButtonEventType::RELEASED => "RELEASED",
        }
    }
}

/// Generic button event structure
#[derive(Debug, Clone)]
pub struct ButtonEvent {
    pub button_name: PhysicalButtonName,
    pub event_type: ButtonEventType,
}

/// Input data for button processing
#[derive(Debug, Clone)]
pub struct ButtonInput {
    pub button_name: PhysicalButtonName,
    pub is_pressed: bool,
}
