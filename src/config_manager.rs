use std::sync::{Arc, Mutex};
use std::sync::OnceLock;
use crate::token_based_config::TokenBasedParser;

/// Shared configuration manager to avoid duplicate config parsing
pub struct ConfigManager {
    parser: Arc<Mutex<TokenBasedParser>>,
}

static CONFIG_MANAGER: OnceLock<ConfigManager> = OnceLock::new();

impl ConfigManager {
    /// Get the global shared config manager instance
    pub fn global() -> &'static ConfigManager {
        CONFIG_MANAGER.get_or_init(|| {
            let parser = TokenBasedParser::new()
                .expect("Failed to initialize config parser");
            ConfigManager {
                parser: Arc::new(Mutex::new(parser)),
            }
        })
    }

    /// Get a clone of the shared parser
    pub fn get_parser(&self) -> Arc<Mutex<TokenBasedParser>> {
        Arc::clone(&self.parser)
    }

    /// Reload config (useful for testing or config changes)
    pub fn reload(&self) -> Result<(), Box<dyn std::error::Error>> {
        let new_parser = TokenBasedParser::new()?;
        let mut parser = self.parser.lock().unwrap();
        *parser = new_parser;
        Ok(())
    }
}
