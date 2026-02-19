use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub api: ApiConfig,
    pub proxy: ProxyConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub addr: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:3000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    pub addr: String,
    pub target: String,
    pub capture_mode: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:3001".to_string(),
            target: "http://localhost:11434".to_string(),
            capture_mode: "full".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub db_path: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self { db_path: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}

impl Config {
    /// Load config from `~/.llmtrace/config.toml`, returning defaults if file is missing.
    pub fn load() -> Self {
        let path = Self::default_path();
        Self::load_from(&path)
    }

    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".llmtrace")
            .join("config.toml")
    }

    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    tracing::info!(path = %path.display(), "loaded config");
                    config
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "invalid config file, using defaults");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".llmtrace")
    }

    pub fn db_path(&self) -> PathBuf {
        self.storage
            .db_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| Self::data_dir().join("traces.db"))
    }

    pub fn log_dir() -> PathBuf {
        Self::data_dir().join("logs")
    }

    pub fn pid_path() -> PathBuf {
        Self::data_dir().join("daemon.pid")
    }

    /// Write config to a TOML file.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml_str)
    }

    /// Save config to the default path.
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&Self::default_path())
    }
}
