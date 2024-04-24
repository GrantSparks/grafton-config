#![allow(clippy::module_name_repetitions)]

use {
    derivative::Derivative,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct GraftonConfig {
    #[derivative(Default(value = "\"dev\".into()"))]
    pub run_mode: String,
}
