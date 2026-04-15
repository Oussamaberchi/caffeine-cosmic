use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::state::TimerSelection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InhibitMode {
    #[default]
    Idle,
    Suspend,
    Both,
}

impl InhibitMode {
    pub fn label(&self) -> &'static str {
        match self {
            InhibitMode::Idle => "Idle only",
            InhibitMode::Suspend => "Suspend only",
            InhibitMode::Both => "Idle + Suspend",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaffeineConfig {
    pub last_timer: TimerSelection,
    pub manual_mins: u32,
    pub manual_hours: u32,
    pub inhibit_mode: InhibitMode,
    pub auto_start: bool,
    pub warning_mins: u32,
    pub show_notifications: bool,
}

impl Default for CaffeineConfig {
    fn default() -> Self {
        Self {
            last_timer: TimerSelection::default(),
            manual_mins: 30,
            manual_hours: 0,
            inhibit_mode: InhibitMode::default(),
            auto_start: false,
            warning_mins: 5,
            show_notifications: true,
        }
    }
}

impl CaffeineConfig {
    pub fn config_path() -> Option<PathBuf> {
        let config_dir = dirs::config_dir()?;
        Some(config_dir.join("cosmic-caffeine").join("config.toml"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&contents) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("No config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let contents = toml::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, contents).map_err(|e| e.to_string())?;
        Ok(())
    }
}