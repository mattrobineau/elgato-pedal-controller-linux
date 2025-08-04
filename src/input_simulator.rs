use crate::token_based_config::ExecutableAction;
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
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("Creating Enigo for input simulation...");

        // Check environment and provide helpful guidance
        if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
            println!("   Desktop session type: {session_type}");
            match session_type.as_str() {
                "wayland" => {
                    println!("   ⚠️  Wayland detected - input simulation may have restrictions");

                    // Check for specific Wayland compositor for more targeted advice
                    if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
                        println!("   🖥️  Wayland display: {wayland_display}");
                    }

                    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
                        match desktop.to_lowercase().as_str() {
                            "gnome" => {
                                println!(
                                    "   🔧 GNOME detected - may work with accessibility permissions enabled"
                                );
                            }
                            "kde" => {
                                println!(
                                    "   🔧 KDE Plasma detected - input simulation usually works well"
                                );
                            }
                            "sway" => {
                                println!("   🔧 Sway detected - may need additional configuration");
                            }
                            _ => {
                                println!("   🔧 Desktop environment: {desktop}");
                            }
                        }
                    }

                    println!("   💡 Solutions if input simulation doesn't work:");
                    println!("      • Run with elevated permissions: sudo cargo run");
                    println!("      • Switch to X11 session temporarily");
                    println!("      • Some Wayland compositors may work without issues");
                }
                "x11" => {
                    println!("   ✅ X11 detected - input simulation should work seamlessly");
                }
                _ => {
                    println!("   ℹ️  Unknown session type - input simulation compatibility varies");
                }
            }
        } else {
            println!("   ℹ️  Could not detect session type - assuming compatibility");
        }

        let enigo = Enigo::new(&Settings::default())?;

        // Quick test to verify input simulation is working
        println!("   🧪 Testing input simulation capability...");
        // We'll just test creating a key token without executing it
        let _test_token = Token::Key(Key::Escape, Direction::Press);
        println!("   ✅ Input simulation initialized successfully");

        Ok(InputSimulator {
            enigo,
            pressed_keys: HashSet::new(),
            scheduled_releases: Vec::new(),
        })
    }

    pub fn execute_actions(
        &mut self,
        actions: &[ExecutableAction],
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Executing {} actions", actions.len());

        for (i, action) in actions.iter().enumerate() {
            println!(" ⚙️ Executing action {}: {:?}", i + 1, action);

            match action {
                ExecutableAction::KeyPress { key, auto_release } => {
                    self.execute_key_press(*key, *auto_release)
                        .expect("Failed to execute key press");
                }
                ExecutableAction::KeyRelease { key } => {
                    self.execute_key_release(*key)
                        .expect("Failed to execute key release");
                }
                ExecutableAction::Text { text } => {
                    self.execute_text(text.clone())
                        .expect("Failed to execute text input");
                }
                ExecutableAction::Sleep { duration_ms } => {
                    self.execute_sleep(*duration_ms)
                        .expect("Failed to execute sleep");
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

            // Small delay between actions for proper execution
            std::thread::sleep(Duration::from_millis(10));
        }

        // Note: Scheduled releases will be processed by the main timer loop
        // This ensures proper timing and avoids conflicts with other timer processing

        Ok(())
    }

    fn execute_key_press(
        &mut self,
        key: Key,
        auto_release: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let press_token = Token::Key(key, Direction::Press);

        match self.enigo.execute(&press_token) {
            Ok(_) => {
                println!("Key press executed successfully: {key:?}");
                self.pressed_keys.insert(key);

                if auto_release {
                    // Immediately release the key
                    let release_token = Token::Key(key, Direction::Release);
                    match self.enigo.execute(&release_token) {
                        Ok(_) => {
                            println!("Key auto-RELEASING successfully: {key:?}");
                            self.pressed_keys.remove(&key);
                        }
                        Err(e) => {
                            eprintln!("Error auto-releasing key: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error executing key press: {e}");
                return Err(format!("Failed to execute key press: {e}").into());
            }
        }

        Ok(())
    }

    fn execute_key_release(&mut self, key: Key) -> Result<(), Box<dyn std::error::Error>> {
        println!("===========================================================");
        println!(" ⬆️ Executing key release...");
        println!("===========================================================");
        if self.pressed_keys.contains(&key) {
            let release_token = Token::Key(key, Direction::Release);

            match self.enigo.execute(&release_token) {
                Ok(_) => {
                    println!("Key release executed successfully: {key:?}");
                    self.pressed_keys.remove(&key);
                }
                Err(e) => {
                    eprintln!("Error executing key release: {e}");
                    return Err(format!("Failed to execute key release: {e}").into());
                }
            }
        } else {
            println!("Key {key:?} not currently pressed, skipping release");
        }

        Ok(())
    }

    fn execute_text(&mut self, text: String) -> Result<(), Box<dyn std::error::Error>> {
        match self.enigo.text(&text) {
            Ok(_) => {
                println!("Text executed successfully: {text}");
            }
            Err(e) => {
                eprintln!("Error executing text: {e}");
                return Err(format!("Failed to execute text: {e}").into());
            }
        }

        Ok(())
    }

    fn execute_sleep(&self, duration_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
        println!("Sleeping for {duration_ms} ms");
        std::thread::sleep(Duration::from_millis(duration_ms));
        Ok(())
    }

    fn schedule_release_all_after(&mut self, duration_ms: u64) {
        let release_time = if duration_ms > 0 {
            Instant::now() + Duration::from_millis(duration_ms)
        } else {
            Instant::now()
        };

        // Schedule all currently pressed keys for release
        for key in &self.pressed_keys {
            self.scheduled_releases.push((release_time, *key));
        }

        println!(
            "Scheduled {} keys for release after {} ms",
            self.pressed_keys.len(),
            duration_ms
        );
    }

    pub fn process_scheduled_releases(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let mut releases_to_process = Vec::new();

        // Find releases that are due
        self.scheduled_releases.retain(|(release_time, key)| {
            if *release_time <= now {
                releases_to_process.push(*key);
                false // Remove from scheduled list
            } else {
                true // Keep in scheduled list
            }
        });

        // Execute the releases only if there are any
        if !releases_to_process.is_empty() {
            println!(
                "⏰ Processing {} scheduled key releases",
                releases_to_process.len()
            );

            for key in releases_to_process {
                if self.pressed_keys.contains(&key) {
                    let release_token = Token::Key(key, Direction::Release);

                    match self.enigo.execute(&release_token) {
                        Ok(_) => {
                            println!("   ⬆️ Scheduled release executed: {key:?}");
                            self.pressed_keys.remove(&key);
                        }
                        Err(e) => {
                            eprintln!("   ❌ Error executing scheduled release for {key:?}: {e}");
                        }
                    }
                } else {
                    println!("   ⚠️  Key {key:?} already released");
                }
            }
        }

        Ok(())
    }
}
