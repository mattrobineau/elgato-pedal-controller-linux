use std::fs;
use std::path::Path;
use std::process::Command;

/// Manages systemd service installation and configuration
pub struct ServiceManager {
    binary_name: String,
    service_name: String,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            binary_name: "elgato-pedal-controller".to_string(),
            service_name: "elgato-pedal-controller".to_string(),
        }
    }

    /// Install the systemd service
    pub fn install_service(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = match self.get_binary_path() {
            Ok(path) => path,
            Err(e) => return Err(format!("Failed to get binary path: {}", e).into()),
        };
        let service_content = self.generate_service_file(&binary_path);
        let service_dir = match self.get_service_directory(system_wide) {
            Ok(dir) => dir,
            Err(e) => return Err(format!("Failed to get service directory: {}", e).into()),
        };
        let service_file = format!("{}/{}.service", service_dir, self.service_name);

        fs::create_dir_all(&service_dir)
            .map_err(|e| format!("Failed to create service directory: {}", e))?;

        fs::write(&service_file, service_content)
            .map_err(|e| format!("Failed to write service file: {}", e))?;

        self.reload_systemd(system_wide)?;
        self.enable_service(system_wide)?;
        self.start_service(system_wide)?;

        println!("✅ Service installed and started successfully!");
        println!("   Service file: {}", service_file);
        
        if system_wide {
            println!("   Status: sudo systemctl status {}", self.service_name);
            println!("   Stop with: sudo systemctl stop {}", self.service_name);
        } else {
            println!("   Status: systemctl --user status {}", self.service_name);
            println!("   Stop with: systemctl --user stop {}", self.service_name);
        }

        Ok(())
    }

    /// Uninstall the systemd service
    pub fn uninstall_service(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.stop_service(system_wide);
        let _ = self.disable_service(system_wide);

        let service_dir = match self.get_service_directory(system_wide) {
            Ok(dir) => dir,
            Err(e) => return Err(format!("Failed to get service directory: {}", e).into()),
        };
        let service_file = format!("{}/{}.service", service_dir, self.service_name);

        if Path::new(&service_file).exists() {
            fs::remove_file(&service_file)
                .map_err(|e| format!("Failed to remove service file: {}", e))?;
            println!("Removed service file: {}", service_file);
        }

        self.reload_systemd(system_wide)?;

        println!("✅ Service uninstalled successfully!");
        Ok(())
    }

    /// Get the path to the installed binary
    fn get_binary_path(&self) -> Result<String, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")
            .map_err(|e| format!("Failed to get HOME environment variable: {}", e))?;
        let binary_path = format!("{}/.local/bin/{}", home, self.binary_name);
        
        if !Path::new(&binary_path).exists() {
            return Err(format!(
                "Binary not found at {}. Please run 'make install' first.", 
                binary_path
            ).into());
        }

        Ok(binary_path)
    }

    /// Get the systemd service directory
    fn get_service_directory(&self, system_wide: bool) -> Result<String, Box<dyn std::error::Error>> {
        if system_wide {
            Ok("/etc/systemd/system".to_string())
        } else {
            let home = std::env::var("HOME")
                .map_err(|e| format!("Failed to get HOME environment variable: {}", e))?;
            Ok(format!("{}/.config/systemd/user", home))
        }
    }

    /// Generate the systemd service file content
    fn generate_service_file(&self, binary_path: &str) -> String {
        format!(
r#"[Unit]
Description=Elgato Stream Deck Pedal Controller
Documentation=https://github.com/funnierinspanish/elgato-pedal-controller-linux
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=simple
ExecStart={} run
Restart=on-failure
RestartSec=5
Environment=DISPLAY=:0

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=false
ReadWritePaths={}/.config

[Install]
WantedBy=graphical-session.target
"#,
            binary_path,
            std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string())
        )
    }

    /// Reload systemd daemon
    fn reload_systemd(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("systemctl");
        
        if !system_wide {
            cmd.arg("--user");
        }
        
        let output = cmd.arg("daemon-reload").output()
            .map_err(|e| format!("Failed to execute systemctl daemon-reload: {}", e))?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to reload systemd: {}", 
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }

        Ok(())
    }

    /// Enable the service
    fn enable_service(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("systemctl");
        
        if !system_wide {
            cmd.arg("--user");
        }
        
        let output = cmd.arg("enable").arg(&self.service_name).output()
            .map_err(|e| format!("Failed to execute systemctl enable: {}", e))?;
        
        if !output.status.success() {
            println!("⚠️  Warning: Could not enable service automatically");
            println!("   You may need to run manually:");
            if system_wide {
                println!("   sudo systemctl enable {}", self.service_name);
            } else {
                println!("   systemctl --user enable {}", self.service_name);
            }
        } else {
            println!("✅ Service enabled for automatic startup");
        }

        Ok(())
    }

    /// Disable the service
    fn disable_service(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("systemctl");
        
        if !system_wide {
            cmd.arg("--user");
        }
        
        let _output = cmd.arg("disable").arg(&self.service_name).output()
            .map_err(|e| format!("Failed to execute systemctl disable: {}", e))?;
        Ok(())
    }

    /// Stop the service
    fn stop_service(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("systemctl");
        
        if !system_wide {
            cmd.arg("--user");
        }
        
        let _output = cmd.arg("stop").arg(&self.service_name).output()
            .map_err(|e| format!("Failed to execute systemctl stop: {}", e))?;
        Ok(())
    }

    /// Start the service
    fn start_service(&self, system_wide: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("systemctl");
        
        if !system_wide {
            cmd.arg("--user");
        }
        
        let output = cmd.arg("start").arg(&self.service_name).output()
            .map_err(|e| format!("Failed to execute systemctl start: {}", e))?;
        
        if !output.status.success() {
            println!("⚠️  Warning: Could not start service automatically");
            println!("   Error: {}", String::from_utf8_lossy(&output.stderr));
            println!("   You may need to start it manually:");
            if system_wide {
                println!("   sudo systemctl start {}", self.service_name);
            } else {
                println!("   systemctl --user start {}", self.service_name);
            }
        } else {
            println!("✅ Service started successfully");
        }

        Ok(())
    }
}
