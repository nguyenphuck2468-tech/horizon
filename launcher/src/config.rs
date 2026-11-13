//! Reads optional launcher configuration from `horizon-launcher.toml` located
//! next to the executable.
//!
//! Error handling: missing file, invalid TOML, empty values, and wrong types
//! all degrade gracefully to `None` and never block later config sources.
//!
//! The parsed config is cached with `OnceLock` so the file is read at most once
//! per process lifetime. Individual fields are cloned out before returning,
//! so the cached struct is never moved out of the lock.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LauncherConfig {
    pub horizon_msa_client_id: Option<String>,
    pub horizon_ely_client_id: Option<String>,
}

/// Directory containing the running executable, if it can be determined.
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Parse a TOML string into `LauncherConfig`. Returns `Default::default()` on
/// any parse error. Each field is extracted independently so that a wrong type
/// in one field does not cause the other field to be lost.
/// feed arbitrary TOML strings directly.
pub fn parse_config_text(text: &str) -> LauncherConfig {
    let value: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(_) => return LauncherConfig::default(),
    };
    LauncherConfig {
        horizon_msa_client_id: value
            .get("HORIZON_MSA_CLIENT_ID")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        horizon_ely_client_id: value
            .get("HORIZON_ELY_CLIENT_ID")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
    }
}

/// Load and parse the sidecar config file next to the executable.
/// Missing file, invalid TOML, or wrong types all return the default config.
/// Result is cached in a `OnceLock`; the lock itself is never moved out.
fn load_config_from_file() -> &'static LauncherConfig {
    static CONFIG: OnceLock<LauncherConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let dir = match exe_dir() {
            Some(d) => d,
            None => return LauncherConfig::default(),
        };
        let path = dir.join("horizon-launcher.toml");
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return LauncherConfig::default(),
        };
        parse_config_text(&text)
    })
}

/// Microsoft client ID from the sidecar config file only.
pub fn msa_client_id_from_config() -> Option<String> {
    load_config_from_file().horizon_msa_client_id.clone()
}

/// Ely.by client ID from the sidecar config file only.
pub fn ely_client_id_from_config() -> Option<String> {
    load_config_from_file().horizon_ely_client_id.clone()
}

/// Resolve the final client ID from ordered sources. Empty strings (including
/// whitespace-only) are treated as absent and do not block later sources.
pub fn resolve_client_id(config: Option<&str>, env: Option<&str>, default: &str) -> Option<String> {
    config
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
        .or_else(|| env.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()))
        .or_else(|| {
            if default.trim().is_empty() {
                None
            } else {
                Some(default.trim().to_owned())
            }
        })
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn parse_config_text_uppercase_keys() {
        let text = r#"HORIZON_MSA_CLIENT_ID = "msa-id"
HORIZON_ELY_CLIENT_ID = "ely-id"
"#;
        let cfg = parse_config_text(text);
        assert_eq!(cfg.horizon_msa_client_id, Some("msa-id".to_owned()));
        assert_eq!(cfg.horizon_ely_client_id, Some("ely-id".to_owned()));
    }

    #[test]
    fn parse_config_text_valid_toml() {
        let text = r#"HORIZON_MSA_CLIENT_ID = "abc"
"#;
        let cfg = parse_config_text(text);
        assert_eq!(cfg.horizon_msa_client_id, Some("abc".to_owned()));
        assert_eq!(cfg.horizon_ely_client_id, None);
    }

    #[test]
    fn parse_config_text_invalid_syntax_returns_default() {
        let text = "this is not valid toml {{{";
        let cfg = parse_config_text(text);
        assert_eq!(cfg, LauncherConfig::default());
    }

    #[test]
    fn parse_config_text_wrong_type_returns_default() {
        let text = "HORIZON_MSA_CLIENT_ID = 123\n";
        let cfg = parse_config_text(text);
        assert_eq!(cfg, LauncherConfig::default());
    }

    #[test]
    fn parse_config_text_wrong_type_one_field_does_not_lose_other() {
        let text = r#"HORIZON_MSA_CLIENT_ID = "valid-msa-id"
HORIZON_ELY_CLIENT_ID = 123
"#;
        let cfg = parse_config_text(text);
        assert_eq!(cfg.horizon_msa_client_id, Some("valid-msa-id".to_owned()));
        assert_eq!(cfg.horizon_ely_client_id, None);
    }

    #[test]
    fn parse_config_text_empty_value() {
        let text = r#"HORIZON_MSA_CLIENT_ID = ""
"#;
        let cfg = parse_config_text(text);
        assert_eq!(cfg.horizon_msa_client_id, Some("".to_owned()));
    }

    #[test]
    fn resolve_client_id_toml_wins() {
        let id = resolve_client_id(Some("toml-id"), Some("env-id"), "default-id");
        assert_eq!(id, Some("toml-id".to_owned()));
    }

    #[test]
    fn resolve_client_id_env_when_no_toml() {
        let id = resolve_client_id(None, Some("env-id"), "default-id");
        assert_eq!(id, Some("env-id".to_owned()));
    }

    #[test]
    fn resolve_client_id_default_when_none() {
        let id = resolve_client_id(None, None, "default-id");
        assert_eq!(id, Some("default-id".to_owned()));
    }

    #[test]
    fn resolve_client_id_empty_toml_falls_back_to_env() {
        let id = resolve_client_id(Some(""), Some("env-id"), "default-id");
        assert_eq!(id, Some("env-id".to_owned()));
    }

    #[test]
    fn resolve_client_id_whitespace_toml_falls_back_to_env() {
        let id = resolve_client_id(Some("   "), Some("env-id"), "default-id");
        assert_eq!(id, Some("env-id".to_owned()));
    }

    #[test]
    fn resolve_client_id_empty_env_falls_back_to_default() {
        let id = resolve_client_id(None, Some("   "), "default-id");
        assert_eq!(id, Some("default-id".to_owned()));
    }

    #[test]
    fn resolve_client_id_empty_default_returns_none() {
        let id = resolve_client_id(None, None, "");
        assert_eq!(id, None);
    }

    #[test]
    fn resolve_client_id_whitespace_default_returns_none() {
        let id = resolve_client_id(None, None, "   ");
        assert_eq!(id, None);
    }

    #[test]
    fn resolve_client_id_mixed_whitespace() {
        let id = resolve_client_id(Some("\t \n"), Some("\t env \n"), "\t default \n");
        assert_eq!(id, Some("env".to_owned()));
    }
}
