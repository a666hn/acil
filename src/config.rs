use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::{AppError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub active_profile: String,
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub jira: ServiceConfig,
    pub confluence: ServiceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub base_url: String,
    pub email: String,
    pub api_token: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config {
                active_profile: String::new(),
                profiles: BTreeMap::new(),
            });
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Config(format!("Failed to read {:?}: {}", path, e)))?;
        let config: Config =
            serde_yaml::from_str(&content).map_err(|e| AppError::Config(e.to_string()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Config(format!("Failed to create config dir: {}", e)))?;
        }
        let content = serde_yaml::to_string(self).map_err(|e| AppError::Config(e.to_string()))?;
        std::fs::write(&path, content)
            .map_err(|e| AppError::Config(format!("Failed to write {:?}: {}", path, e)))?;
        Ok(())
    }

    pub fn active_profile(&self) -> Result<&Profile> {
        self.profiles
            .get(&self.active_profile)
            .ok_or_else(|| {
                if self.profiles.is_empty() {
                    AppError::Config("No profiles configured. Run 'acil login' first.".into())
                } else {
                    AppError::Config(format!(
                        "Active profile '{}' not found. Run 'acil profile list' to see available profiles.",
                        self.active_profile
                    ))
                }
            })
    }

    pub fn add_profile(&mut self, name: String, profile: Profile) {
        let is_first = self.profiles.is_empty();
        self.profiles.insert(name.clone(), profile);
        if is_first {
            self.active_profile = name;
        }
    }

    pub fn switch_profile(&mut self, name: &str) -> Result<()> {
        if !self.profiles.contains_key(name) {
            return Err(AppError::NotFound(format!("Profile '{}' not found", name)));
        }
        self.active_profile = name.to_string();
        Ok(())
    }

    pub fn remove_profile(&mut self, name: &str) -> Result<()> {
        if !self.profiles.contains_key(name) {
            return Err(AppError::NotFound(format!("Profile '{}' not found", name)));
        }
        self.profiles.remove(name);
        if self.active_profile == name {
            self.active_profile = self.profiles.keys().next().cloned().unwrap_or_default();
        }
        Ok(())
    }
}

pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Config("Cannot find home dir".into()))?;
    Ok(home.join(".config").join("acil").join("config.yaml"))
}
