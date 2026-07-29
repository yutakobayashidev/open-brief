use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub database_file: PathBuf,
    pub runtime_dir: PathBuf,
    pub control_socket: PathBuf,
    pub systemd_unit: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, PathsError> {
        let project = ProjectDirs::from("dev", "yutakobayashidev", "openbrief")
            .ok_or(PathsError::HomeUnavailable)?;
        let runtime_root = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or(PathsError::RuntimeDirectoryUnavailable)?;
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(PathsError::HomeUnavailable)?;
        let runtime_dir = runtime_root.join("openbrief");

        Ok(Self {
            config_file: project.config_dir().join("config.toml"),
            database_file: project.data_dir().join("openbrief.sqlite3"),
            control_socket: runtime_dir.join("control.sock"),
            runtime_dir,
            systemd_unit: home
                .join(".config")
                .join("systemd")
                .join("user")
                .join("openbrief.service"),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathsError {
    #[error("could not determine the user home directory")]
    HomeUnavailable,
    #[error("XDG_RUNTIME_DIR is not set")]
    RuntimeDirectoryUnavailable,
}
