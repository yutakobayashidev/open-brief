use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub capture: CaptureConfig,
    pub retention_days: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            capture: CaptureConfig::default(),
            retention_days: 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub excluded_apps: Vec<String>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            excluded_apps: vec![
                "1password".into(),
                "com.1password.1password".into(),
                "signal".into(),
                "org.signal.signal".into(),
                "discord".into(),
                "com.discordapp.discord".into(),
            ],
        }
    }
}

impl Config {
    pub fn load_or_create(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            return Self::load(path);
        }

        let config = Self::default();
        config.save(path)?;
        Ok(config)
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let raw = fs::read_to_string(path).map_err(ConfigError::Read)?;
        let mut config: Self = toml::from_str(&raw).map_err(ConfigError::Parse)?;
        config.validate()?;
        config.capture.excluded_apps = normalize_apps(config.capture.excluded_apps);
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        self.validate()?;
        let Some(parent) = path.parent() else {
            return Err(ConfigError::InvalidPath);
        };
        fs::create_dir_all(parent).map_err(ConfigError::Write)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(ConfigError::Write)?;
        let encoded = toml::to_string_pretty(self).map_err(ConfigError::Encode)?;
        fs::write(path, encoded).map_err(ConfigError::Write)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(ConfigError::Write)?;
        Ok(())
    }

    #[must_use]
    pub fn is_excluded(&self, app_id: &str) -> bool {
        let normalized = app_id.trim().to_ascii_lowercase();
        self.capture
            .excluded_apps
            .iter()
            .any(|candidate| candidate == &normalized)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.retention_days == 0 || self.retention_days > 30 {
            return Err(ConfigError::InvalidRetention(self.retention_days));
        }
        Ok(())
    }
}

fn normalize_apps(apps: Vec<String>) -> Vec<String> {
    let mut apps: Vec<_> = apps
        .into_iter()
        .map(|app| app.trim().to_ascii_lowercase())
        .filter(|app| !app.is_empty())
        .collect();
    apps.sort_unstable();
    apps.dedup();
    apps
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read config: {0}")]
    Read(io::Error),
    #[error("could not write config: {0}")]
    Write(io::Error),
    #[error("invalid TOML config: {0}")]
    Parse(toml::de::Error),
    #[error("could not encode TOML config: {0}")]
    Encode(toml::ser::Error),
    #[error("config path has no parent directory")]
    InvalidPath,
    #[error("retention_days must be between 1 and 30, got {0}")]
    InvalidRetention(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusion_matching_is_case_insensitive() {
        let config = Config::default();
        assert!(config.is_excluded("ORG.SIGNAL.SIGNAL"));
        assert!(config.is_excluded("Discord"));
        assert!(!config.is_excluded("firefox"));
    }
}
