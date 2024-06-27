#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod config;

mod config_loader;

mod token_expander;

mod error;
pub use error::Error;

use serde::{de::DeserializeOwned, Serialize};

pub use {
    config::GraftonConfig, config_loader::load_config_from_dir, token_expander::expand_tokens,
};

pub trait GraftonConfigProvider: TokenExpandingConfig {
    fn get_grafton_config(&self) -> &GraftonConfig;
}

pub trait TokenExpandingConfig:
    'static + Send + Sync + DeserializeOwned + Serialize + std::fmt::Debug
{
}
