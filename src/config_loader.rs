#![allow(clippy::module_name_repetitions)]

use std::{
    env,
    path::{Path, PathBuf},
};

use {
    figment::{
        providers::{Format, Toml},
        Figment,
    },
    serde_json::Value,
};

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
            let config = load_config_from_file(path)
                .map_err(|e| Error::ConfigError(format!("Error loading config: {e}")))?;
            figment = figment.merge(config);
        } else if path.file_name().unwrap_or_default() == DEFAULT_CONFIG_FILE {
            let parent_dir = path.parent().unwrap_or_else(|| Path::new(""));
            let abs_parent = parent_dir
                .canonicalize()
                .unwrap_or_else(|_| parent_dir.to_path_buf());
            let abs_path = abs_parent.join(DEFAULT_CONFIG_FILE);
            println!("Default configuration file not found: {abs_path:?}");
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
fn determine_run_mode() -> &'static str {
    env::var("RUN_MODE").map_or("dev", |val| Box::leak(val.into_boxed_str()))
}

fn setup_config_paths(config_dir: &str, run_mode: &str) -> Vec<PathBuf> {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let absolute_config_dir = current_dir.join(config_dir);

    vec![
        absolute_config_dir.join("default.toml"),
        absolute_config_dir.join("local.toml"),
        absolute_config_dir.join(format!("{run_mode}.toml")),
    ]
}

fn load_config_from_file(path: &Path) -> Result<Figment, Error> {
    if path.exists() {
        Ok(Figment::new().merge(Toml::file(path)))
    } else {
        Err(Error::ConfigError(format!("File not found: {path:?}")))
    }
}

fn handle_env_vars() {
    let original_env = env::vars().collect::<Vec<_>>();
    for (key, value) in &original_env {
        let new_key = map_env_var(key);
        env::set_var(new_key, value);
    }
}

fn map_env_var(key: &str) -> String {
    let mut new_key = String::with_capacity(key.len());
    for ch in key.chars() {
        match ch {
            'W' if key.starts_with("WEBSITE_") => new_key.push_str("WEBSITE."),
            'S' if key.starts_with("SESSION_") => new_key.push_str("SESSION."),
            'L' if key.starts_with("LOGGER_") => new_key.push_str("LOGGER."),
            _ => new_key.push(ch),
        }
    }
    new_key
}
