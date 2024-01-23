#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod config;

mod config_loader;

mod token_expander;

mod error;
pub use error::Error;

use serde::{de::DeserializeOwned, Serialize};

pub use {config::GraftonConfig, config_loader::load_config_from_dir};

pub trait GraftonConfigProvider:
    'static + Send + Sync + DeserializeOwned + Serialize + std::fmt::Debug
{
    fn get_grafton_config(&self) -> &GraftonConfig;
}
