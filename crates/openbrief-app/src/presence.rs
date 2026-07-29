use std::fs;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Active,
    Idle,
    Locked,
    Unavailable,
}

pub fn current_presence() -> Presence {
    let Some(uid) = current_uid() else {
        return Presence::Unavailable;
    };
    let Ok(output) = Command::new("loginctl")
        .args(["--json=short", "list-sessions"])
        .output()
    else {
        return Presence::Unavailable;
    };
    if !output.status.success() {
        return Presence::Unavailable;
    }
    let Ok(sessions) = serde_json::from_slice::<Vec<LoginSession>>(&output.stdout) else {
        return Presence::Unavailable;
    };
    let Some(session) = sessions
        .into_iter()
        .find(|session| session.uid == uid && session.class == "user")
    else {
        return Presence::Unavailable;
    };

    let Ok(output) = Command::new("loginctl")
        .args([
            "show-session",
            &session.session,
            "-p",
            "LockedHint",
            "-p",
            "IdleHint",
        ])
        .output()
    else {
        return Presence::Unavailable;
    };
    if !output.status.success() {
        return Presence::Unavailable;
    }

    let properties = String::from_utf8_lossy(&output.stdout);
    if property_is_yes(&properties, "LockedHint") {
        Presence::Locked
    } else if property_is_yes(&properties, "IdleHint") {
        Presence::Idle
    } else {
        Presence::Active
    }
}

#[derive(Debug, Deserialize)]
struct LoginSession {
    session: String,
    uid: u32,
    class: String,
}

fn current_uid() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("Uid:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

fn property_is_yes(properties: &str, name: &str) -> bool {
    properties.lines().any(|line| line == format!("{name}=yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loginctl_boolean_properties() {
        let properties = "IdleHint=no\nLockedHint=yes\n";
        assert!(property_is_yes(properties, "LockedHint"));
        assert!(!property_is_yes(properties, "IdleHint"));
    }
}
