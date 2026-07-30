use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use openbrief_agent::AuthMethodInfo;
use openbrief_protocol::{RuntimeDescriptor, RuntimeSource};

const CODEX: AcpRuntimeSpec = AcpRuntimeSpec {
    id: "codex",
    label: "Codex",
    packaged_path: "libexec/openbrief/codex-acp",
    args: &[],
    minimum_version: Some((1, 1, 7)),
    preferred_auth_method_hint: Some("chat"),
    manual_auth_instructions: None,
    openbrief_mcp: true,
};
const PI: AcpRuntimeSpec = AcpRuntimeSpec {
    id: "pi",
    label: "Pi",
    packaged_path: "libexec/openbrief/pi-acp",
    args: &[],
    // pi-acp does not implement --version. ACP initialize negotiates v1.
    minimum_version: None,
    preferred_auth_method_hint: Some("pi_terminal_login"),
    manual_auth_instructions: Some(
        "Piのmodel providerをterminalで設定してからopenbriefdを再起動してください",
    ),
    // pi-acp accepts ACP MCP configuration but does not forward it to Pi.
    openbrief_mcp: false,
};
const RUNTIMES: &[AcpRuntimeSpec] = &[CODEX, PI];

#[derive(Debug, Clone, Copy)]
pub(crate) struct AcpRuntimeSpec {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) packaged_path: &'static str,
    pub(crate) args: &'static [&'static str],
    /// `None` delegates compatibility to the ACP initialize handshake.
    minimum_version: Option<(u64, u64, u64)>,
    preferred_auth_method_hint: Option<&'static str>,
    pub(crate) manual_auth_instructions: Option<&'static str>,
    pub(crate) openbrief_mcp: bool,
}

impl AcpRuntimeSpec {
    pub(crate) fn prioritize_auth_methods(&self, methods: &mut [AuthMethodInfo]) {
        if let Some(hint) = self.preferred_auth_method_hint {
            methods.sort_by_key(|method| !method.id.to_ascii_lowercase().contains(hint));
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ResolveError {
    #[error("設定された{label} ACP runtimeが見つかりません: {path}")]
    MissingOverride { label: String, path: PathBuf },
    #[error(
        "{label} ACP runtimeがOpenBriefに含まれていません。OpenBriefを再インストールしてください"
    )]
    MissingPackaged { label: String },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProbeError {
    #[error("{label} ACPのversion確認がtimeoutしました")]
    Timeout { label: String },
    #[error("{label} ACPのversionを確認できません: {message}")]
    Failed { label: String, message: String },
    #[error("{label} ACPのversion出力を解釈できません")]
    Unparseable { label: String },
    #[error("{label} ACP {found}は非互換です。{required}以上が必要です")]
    Incompatible {
        label: String,
        found: String,
        required: String,
    },
}

pub(crate) fn runtime_spec(id: &str) -> Option<&'static AcpRuntimeSpec> {
    RUNTIMES.iter().find(|spec| spec.id == id)
}

pub(crate) fn runtime_matches_provider(
    runtime: Option<&RuntimeDescriptor>,
    provider_id: &str,
) -> bool {
    runtime.is_some_and(|runtime| runtime.provider_id == provider_id)
}

pub(crate) fn resolve_runtime(
    spec: &AcpRuntimeSpec,
    configured: Option<&Path>,
    current_executable: &Path,
    resource_dir: Option<&Path>,
) -> Result<RuntimeDescriptor, ResolveError> {
    if let Some(path) = configured {
        if path.is_file() {
            return Ok(descriptor(spec, RuntimeSource::Override, path.to_owned()));
        }
        return Err(ResolveError::MissingOverride {
            label: spec.label.to_owned(),
            path: path.to_owned(),
        });
    }

    let mut candidates = Vec::new();
    if let Some(bin_dir) = current_executable.parent() {
        let source = if current_executable.starts_with("/nix/store/") {
            RuntimeSource::NixStore
        } else {
            RuntimeSource::Packaged
        };
        candidates.push((source, bin_dir.join("../").join(spec.packaged_path)));
    }
    if let Some(resource_dir) = resource_dir {
        candidates.push((
            RuntimeSource::Packaged,
            resource_dir.join(spec.packaged_path),
        ));
    }

    candidates
        .into_iter()
        .find(|(_, path)| path.is_file())
        .map(|(source, path)| descriptor(spec, source, path))
        .ok_or_else(|| ResolveError::MissingPackaged {
            label: spec.label.to_owned(),
        })
}

pub(crate) async fn probe_runtime(
    spec: &AcpRuntimeSpec,
    descriptor: &mut RuntimeDescriptor,
) -> Result<(), ProbeError> {
    let Some(required) = spec.minimum_version else {
        return Ok(());
    };

    let mut command = tokio::process::Command::new(&descriptor.path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .map_err(|_| ProbeError::Timeout {
            label: spec.label.to_owned(),
        })?
        .map_err(|error| ProbeError::Failed {
            label: spec.label.to_owned(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(ProbeError::Failed {
            label: spec.label.to_owned(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    let found = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find(|token| parse_version(token).is_some())
        .map(str::to_owned)
        .ok_or_else(|| ProbeError::Unparseable {
            label: spec.label.to_owned(),
        })?;
    descriptor.version = Some(found.clone());
    if meets_minimum_version(&found, required) {
        return Ok(());
    }
    Err(ProbeError::Incompatible {
        label: spec.label.to_owned(),
        found,
        required: format!("{}.{}.{}", required.0, required.1, required.2),
    })
}

fn descriptor(spec: &AcpRuntimeSpec, source: RuntimeSource, path: PathBuf) -> RuntimeDescriptor {
    RuntimeDescriptor {
        provider_id: spec.id.to_owned(),
        label: spec.label.to_owned(),
        source,
        version: None,
        path,
    }
}

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.trim_start_matches('v').split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

fn meets_minimum_version(value: &str, required: (u64, u64, u64)) -> bool {
    parse_version(value).is_some_and(|version| version >= required)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_exposes_supported_runtimes_without_provider_specific_control_flow() {
        let spec = runtime_spec("codex").expect("codex should be registered");
        assert_eq!(spec.label, "Codex");
        assert_eq!(spec.packaged_path, "libexec/openbrief/codex-acp");
        assert!(spec.openbrief_mcp);

        let spec = runtime_spec("pi").expect("pi should be registered");
        assert_eq!(spec.label, "Pi");
        assert_eq!(spec.packaged_path, "libexec/openbrief/pi-acp");
        assert_eq!(spec.minimum_version, None);
        assert!(spec.manual_auth_instructions.is_some());
        assert!(!spec.openbrief_mcp);
        assert!(runtime_spec("unknown").is_none());
    }

    #[test]
    fn parses_versions_used_by_minimum_policy() {
        assert_eq!(parse_version("1.1.7"), Some((1, 1, 7)));
        assert_eq!(parse_version("v1.2.0"), Some((1, 2, 0)));
        assert_eq!(parse_version("latest"), None);
        assert!(meets_minimum_version("1.1.7", (1, 1, 7)));
        assert!(meets_minimum_version("v1.2.0", (1, 1, 7)));
        assert!(!meets_minimum_version("1.1.6", (1, 1, 7)));
    }

    #[test]
    fn explicit_override_wins_without_provider_specific_resolution() {
        let spec = runtime_spec("codex").expect("codex should be registered");
        let override_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let runtime = resolve_runtime(
            spec,
            Some(&override_path),
            Path::new("/unused/openbrief-desktop"),
            None,
        )
        .expect("existing override should resolve");

        assert!(matches!(runtime.source, RuntimeSource::Override));
        assert_eq!(runtime.provider_id, "codex");
        assert_eq!(runtime.path, override_path);
    }

    #[test]
    fn packaged_path_comes_from_the_catalog() {
        let spec = AcpRuntimeSpec {
            id: "fixture",
            label: "Fixture",
            packaged_path: "Cargo.toml",
            args: &["serve"],
            minimum_version: None,
            preferred_auth_method_hint: None,
            manual_auth_instructions: None,
            openbrief_mcp: false,
        };
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let runtime = resolve_runtime(
            &spec,
            None,
            Path::new("/outside/openbrief-desktop"),
            Some(resource_dir),
        )
        .expect("catalog resource should resolve");

        assert!(matches!(runtime.source, RuntimeSource::Packaged));
        assert_eq!(runtime.provider_id, "fixture");
        assert_eq!(runtime.path, resource_dir.join("Cargo.toml"));
        assert!(!spec.openbrief_mcp);
    }

    #[test]
    fn catalog_controls_auth_preference_and_runtime_identity() {
        let spec = runtime_spec("codex").expect("codex should be registered");
        let mut methods = vec![
            AuthMethodInfo {
                id: "api-key".to_owned(),
                name: "API key".to_owned(),
            },
            AuthMethodInfo {
                id: "chatgpt".to_owned(),
                name: "ChatGPT".to_owned(),
            },
        ];
        spec.prioritize_auth_methods(&mut methods);

        assert_eq!(methods[0].id, "chatgpt");
        let descriptor = descriptor(spec, RuntimeSource::Packaged, "/runtime".into());
        assert!(runtime_matches_provider(Some(&descriptor), "codex"));
        assert!(!runtime_matches_provider(Some(&descriptor), "other"));
    }
}
