#![allow(clippy::module_name_repetitions)]

use std::{
    env,
    path::{Path, PathBuf},
};

use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde_json::Value;

use crate::{token_expander::expand_tokens, Error, TokenExpandingConfig};

const DEFAULT_CONFIG_FILE: &str = "default.toml";

/// Load configuration from the given directory.
///
/// The configuration is loaded from the following files in the given directory:
/// - `default.toml`
/// - `local.toml`
/// - `{run_mode}.toml`
///
/// # Errors
///
/// This function returns an error if any of the configuration files are not found or if there
/// is an error parsing the configuration.
pub fn load_config_from_dir<C: TokenExpandingConfig>(config_dir: &str) -> Result<C, Error> {
    let run_mode = determine_run_mode();
    let config_paths = setup_config_paths(config_dir, run_mode);

    let mut figment = Figment::new();
    for path in &config_paths {
        if path.exists() {
            let config = load_config_from_file(path)?;
            figment = figment.merge(config);
        } else if path.file_name() == Some(DEFAULT_CONFIG_FILE.as_ref()) {
            let abs_path = path.canonicalize().unwrap_or_else(|_| path.clone());
            eprintln!("Default configuration file not found: {abs_path:?}");
        }
    }

    handle_env_vars();

    let config: C = figment
        .extract()
        .map_err(|e| Error::ConfigError(format!("Error extracting config: {e}")))?;

    let config_value: Value = serde_json::to_value(&config)
        .map_err(|e| Error::SerializationError(format!("Error serializing config: {e}")))?;

    let replaced = expand_tokens(&config_value)?;

    serde_json::from_value(replaced)
        .map_err(|e| Error::DeserializationError(format!("Error deserializing config: {e}")))
}

fn determine_run_mode() -> Option<String> {
    env::var("RUN_MODE").ok()
}

fn setup_config_paths(config_dir: &str, run_mode: Option<String>) -> Vec<PathBuf> {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let absolute_config_dir = current_dir.join(config_dir);

    let mut paths = vec![
        absolute_config_dir.join("default.toml"),
        absolute_config_dir.join("local.toml"),
    ];

    if let Some(run_mode) = run_mode {
        paths.push(absolute_config_dir.join(format!("{run_mode}.toml")));
    }

    paths
}

fn load_config_from_file(path: &Path) -> Result<Figment, Error> {
    if path.exists() {
        Ok(Figment::new().merge(Toml::file(path)))
    } else {
        Err(Error::ConfigError(format!("File not found: {path:?}")))
    }
}

fn handle_env_vars() {
    env::vars().for_each(|(key, value)| {
        env::set_var(map_env_var(&key), value);
    });
}

fn map_env_var(key: &str) -> String {
    match key {
        k if k.starts_with("WEBSITE_") => format!("WEBSITE.{}", &k[8..]),
        k if k.starts_with("SESSION_") => format!("SESSION.{}", &k[8..]),
        k if k.starts_with("LOGGER_") => format!("LOGGER.{}", &k[7..]),
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use derivative::Derivative;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
    #[derivative(Default)]
    #[serde(default)]
    struct TestConfig {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[derivative(Default(value = "None"))]
        pub run_mode: Option<String>,
        pub test_value: Option<String>,
    }

    impl TokenExpandingConfig for TestConfig {}

    fn setup_test_env(config_dir: &std::path::Path) {
        env::set_current_dir(config_dir).unwrap();
    }

    fn create_config_file(path: &std::path::Path, content: &str) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "{content}").unwrap();
    }

    #[test]
    fn test_load_config_with_default_run_mode() {
        let dir = tempdir().unwrap();
        setup_test_env(dir.path());

        create_config_file(
            &dir.path().join("default.toml"),
            r#"
            test_value = "default"
        "#,
        );

        let config: TestConfig = load_config_from_dir(".").unwrap();
        assert_eq!(config.test_value, Some("default".to_string()));
    }

    #[test]
    fn test_load_config_with_specific_run_mode() {
        let dir = tempdir().unwrap();
        setup_test_env(dir.path());

        create_config_file(
            &dir.path().join("default.toml"),
            r#"
            test_value = "default"
        "#,
        );

        create_config_file(
            &dir.path().join("prod.toml"),
            r#"
            test_value = "prod"
        "#,
        );

        env::set_var("RUN_MODE", "prod");
        let config: TestConfig = load_config_from_dir(".").unwrap();
        assert_eq!(config.test_value, Some("prod".to_string()));
        env::remove_var("RUN_MODE");
    }

    #[test]
    fn test_load_config_with_null_run_mode() {
        let dir = tempdir().unwrap();
        setup_test_env(dir.path());

        create_config_file(
            &dir.path().join("default.toml"),
            r#"
            test_value = "default"
        "#,
        );

        let config: TestConfig = load_config_from_dir(".").unwrap();
        assert_eq!(config.test_value, Some("default".to_string()));
    }

    #[test]
    fn test_load_config_with_nonexistent_run_mode_file() {
        let dir = tempdir().unwrap();
        setup_test_env(dir.path());

        create_config_file(
            &dir.path().join("default.toml"),
            r#"
            test_value = "default"
        "#,
        );

        env::set_var("RUN_MODE", "nonexistent");
        let config: TestConfig = load_config_from_dir(".").unwrap();
        assert_eq!(config.test_value, Some("default".to_string()));
        env::remove_var("RUN_MODE");
    }
}
