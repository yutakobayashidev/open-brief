use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ServiceManager {
    unit_path: PathBuf,
}

impl ServiceManager {
    pub fn new(unit_path: impl Into<PathBuf>) -> Self {
        Self {
            unit_path: unit_path.into(),
        }
    }

    pub fn install_and_start(&self, executable: &Path) -> Result<(), ServiceError> {
        let Some(parent) = self.unit_path.parent() else {
            return Err(ServiceError::InvalidUnitPath);
        };
        fs::create_dir_all(parent).map_err(ServiceError::WriteUnit)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(ServiceError::WriteUnit)?;
        fs::write(&self.unit_path, render_unit(executable)?).map_err(ServiceError::WriteUnit)?;
        fs::set_permissions(&self.unit_path, fs::Permissions::from_mode(0o600))
            .map_err(ServiceError::WriteUnit)?;
        import_session_environment()?;
        run_systemctl(["daemon-reload"])?;
        run_systemctl(["enable", "--now", "openbrief.service"])?;
        Ok(())
    }

    pub fn stop_and_disable(&self) -> Result<(), ServiceError> {
        run_systemctl(["disable", "--now", "openbrief.service"])
    }
}

fn import_session_environment() -> Result<(), ServiceError> {
    let names: Vec<_> = [
        "NIRI_SOCKET",
        "WAYLAND_DISPLAY",
        "XDG_SESSION_ID",
        "XDG_RUNTIME_DIR",
    ]
    .into_iter()
    .filter(|name| std::env::var_os(name).is_some())
    .collect();
    if names.is_empty() {
        return Ok(());
    }

    let status = Command::new("systemctl")
        .args(["--user", "import-environment"])
        .args(names)
        .status()
        .map_err(ServiceError::Systemctl)?;
    if status.success() {
        Ok(())
    } else {
        Err(ServiceError::SystemctlExit(status.code()))
    }
}

fn render_unit(executable: &Path) -> Result<String, ServiceError> {
    let executable = executable.to_str().ok_or(ServiceError::NonUtf8Executable)?;
    if executable.contains(['\n', '\r']) {
        return Err(ServiceError::InvalidExecutable);
    }
    let executable = executable.replace('\\', "\\\\").replace('"', "\\\"");

    Ok(format!(
        "[Unit]\nDescription=OpenBrief local daemon\nAfter=graphical-session.target\nPartOf=graphical-session.target\n\n[Service]\nType=simple\nExecStart=\"{executable}\"\nRestart=on-failure\nRestartSec=3\nUMask=0077\nNoNewPrivileges=true\n\n[Install]\nWantedBy=graphical-session.target\n"
    ))
}

fn run_systemctl<const N: usize>(args: [&str; N]) -> Result<(), ServiceError> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .map_err(ServiceError::Systemctl)?;
    if status.success() {
        Ok(())
    } else {
        Err(ServiceError::SystemctlExit(status.code()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("systemd unit path has no parent")]
    InvalidUnitPath,
    #[error("executable path is not valid UTF-8")]
    NonUtf8Executable,
    #[error("executable path contains a newline")]
    InvalidExecutable,
    #[error("could not write systemd unit: {0}")]
    WriteUnit(io::Error),
    #[error("could not execute systemctl: {0}")]
    Systemctl(io::Error),
    #[error("systemctl exited unsuccessfully with code {0:?}")]
    SystemctlExit(Option<i32>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_runs_daemon_with_restrictive_umask() {
        let unit = render_unit(Path::new("/tmp/open brief")).expect("render unit");
        assert!(unit.contains("ExecStart=\"/tmp/open brief\""));
        assert!(!unit.contains(" watch"));
        assert!(unit.contains("UMask=0077"));
    }
}
