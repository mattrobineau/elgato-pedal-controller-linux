use crate::token_based_config::ExecutableAction;
use anyhow::{Context, Result};
use enigo::Keyboard;
use enigo::{
    Direction, Enigo, Key, Settings,
    agent::{Agent, Token},
};
use std::collections::HashSet;
use std::time::{Duration, Instant};

pub struct InputSimulator {
    enigo: Enigo,
    pressed_keys: HashSet<Key>,
    scheduled_releases: Vec<(Instant, Key)>,
}

impl InputSimulator {
    pub fn new() -> Result<Self> {
        println!("Initializing Input Simulation System");
        println!("{}", "=".repeat(80));

        let session_type =
            std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
        let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let desktop =
            std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".to_string());

        println!("| {:<20} | {:<50} |", "Property", "Value");
        println!("{}", "-".repeat(80));
        println!("| {:<20} | {:<50} |", "Session Type", session_type);

        if let Some(display) = &wayland_display {
            println!("| {:<20} | {:<50} |", "Wayland Display", display);
        }

        println!("| {:<20} | {:<50} |", "Desktop Environment", desktop);

        let (compatibility_status, notes) = match session_type.as_str() {
            "wayland" => {
                let desktop_notes = match desktop.to_lowercase().as_str() {
                    "gnome" => "May require accessibility permissions",
                    "kde" => "Generally compatible",
                    "sway" => "May need additional configuration",
                    _ => "Compatibility varies by compositor",
                };
                (
                    "Limited",
                    format!("Wayland restrictions apply. {}", desktop_notes),
                )
            }
            "x11" => (
                "Full",
                "X11 provides complete input simulation support".to_string(),
            ),
            _ => ("Unknown", "Compatibility cannot be determined".to_string()),
        };

        println!("| {:<20} | {:<50} |", "Compatibility", compatibility_status);
        println!("| {:<20} | {:<50} |", "Notes", notes);

        if session_type == "wayland" {
            println!("{}", "-".repeat(80));
            println!("| Troubleshooting Options (if input simulation fails):");
            println!("| - Run with elevated permissions: sudo cargo run");
            println!("| - Switch to X11 session temporarily");
            println!("| - Check compositor-specific input permissions");
        }

        println!("{}", "=".repeat(80));

        let enigo = Enigo::new(&Settings::default()).context("Failed to create Enigo instance.")?;

        let _test_token = Token::Key(Key::Escape, Direction::Press);
        println!("Input simulation system initialized successfully");

        Ok(InputSimulator {
            enigo,
            pressed_keys: HashSet::new(),
            scheduled_releases: Vec::new(),
        })
    }

    pub fn execute_actions(&mut self, actions: &[ExecutableAction]) -> Result<()> {
        if actions.is_empty() {
            return Ok(());
        }

        println!("Executing Action Sequence");
        println!("{}", "-".repeat(60));
        println!("| {:<3} | {:<50} |", "No.", "Action");
        println!("{}", "-".repeat(60));

        for (i, action) in actions.iter().enumerate() {
            let action_desc = match action {
                ExecutableAction::KeyPress { key, auto_release } => {
                    if *auto_release {
                        format!("Key Press + Auto Release: {:?}", key)
                    } else {
                        format!("Key Press: {:?}", key)
                    }
                }
                ExecutableAction::KeyRelease { key } => format!("Key Release: {:?}", key),
                ExecutableAction::Text { text } => format!("Text Input: \"{}\"", text),
                ExecutableAction::Sleep { duration_ms } => format!("Sleep: {}ms", duration_ms),
                ExecutableAction::ReleaseAfter { duration_ms } => {
                    format!("Release After: {}ms", duration_ms)
                }
                ExecutableAction::ReleaseAll => "Release All Keys".to_string(),
                ExecutableAction::ReleaseAllAfter { duration_ms } => {
                    format!("Release All After: {}ms", duration_ms)
                }
            };

            println!("| {:<3} | {:<50} |", i + 1, action_desc);

            match action {
                ExecutableAction::KeyPress { key, auto_release } => {
                    self.execute_key_press(*key, *auto_release)
                        .context(format!("Failed to execute key press"))?;
                }
                ExecutableAction::KeyRelease { key } => {
                    self.execute_key_release(*key)
                        .context("Failed to execute key release")?;
                }
                ExecutableAction::Text { text } => {
                    self.execute_text(text.clone())
                        .context(format!("Failed to execute text input for {}", text))?;
                }
                ExecutableAction::Sleep { duration_ms } => {
                    self.execute_sleep(*duration_ms)
                        .context("Failed to execute sleep.")?;
                }
                ExecutableAction::ReleaseAfter { duration_ms } => {
                    self.schedule_release_all_after(*duration_ms);
                }
                ExecutableAction::ReleaseAll => {
                    self.schedule_release_all_after(0);
                }
                ExecutableAction::ReleaseAllAfter { duration_ms } => {
                    self.schedule_release_all_after(*duration_ms);
                }
            }

            std::thread::sleep(Duration::from_millis(10));
        }

        println!("{}", "-".repeat(60));
        println!("Action sequence completed successfully");

        Ok(())
    }

    fn execute_key_press(&mut self, key: Key, auto_release: bool) -> Result<()> {
        let press_token = Token::Key(key, Direction::Press);

        self.enigo
            .execute(&press_token)
            .context("Failed to execute key press.")?;

        self.pressed_keys.insert(key);

        if auto_release {
            let release_token = Token::Key(key, Direction::Release);
            self.enigo
                .execute(&release_token)
                .context("Failed to auto-release key.")?;
            self.pressed_keys.remove(&key);
        }

        Ok(())
    }

    fn execute_key_release(&mut self, key: Key) -> Result<()> {
        if self.pressed_keys.contains(&key) {
            let release_token = Token::Key(key, Direction::Release);

            self.enigo
                .execute(&release_token)
                .context("Failed to execute key release.")?;

            self.pressed_keys.remove(&key);
        }

        Ok(())
    }

    fn execute_text(&mut self, text: String) -> Result<()> {
        self.enigo
            .text(&text)
            .context("Failed to execute text input.")?;
        Ok(())
    }

    fn execute_sleep(&self, duration_ms: u64) -> Result<()> {
        std::thread::sleep(Duration::from_millis(duration_ms));
        Ok(())
    }

    fn schedule_release_all_after(&mut self, duration_ms: u64) {
        let release_time = if duration_ms > 0 {
            Instant::now() + Duration::from_millis(duration_ms)
        } else {
            Instant::now()
        };

        for key in &self.pressed_keys {
            self.scheduled_releases.push((release_time, *key));
        }

        if !self.pressed_keys.is_empty() {
            println!(
                "Scheduled {} keys for delayed release ({}ms)",
                self.pressed_keys.len(),
                duration_ms
            );
        }
    }

    pub fn process_scheduled_releases(&mut self) -> Result<()> {
        let now = Instant::now();
        let mut releases_to_process = Vec::new();

        self.scheduled_releases.retain(|(release_time, key)| {
            if *release_time <= now {
                releases_to_process.push(*key);
                false
            } else {
                true
            }
        });

        if !releases_to_process.is_empty() {
            for key in releases_to_process {
                if self.pressed_keys.contains(&key) {
                    let release_token = Token::Key(key, Direction::Release);

                    match self.enigo.execute(&release_token) {
                        Ok(_) => {
                            self.pressed_keys.remove(&key);
                        }
                        Err(e) => {
                            eprintln!(
                                "Error: Failed to execute scheduled release for {:?}: {}",
                                key, e
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
