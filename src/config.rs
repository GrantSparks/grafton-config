#![allow(clippy::module_name_repetitions)]

use {
    derivative::Derivative,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct GraftonConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[derivative(Default(value = "None"))]
    pub run_mode: Option<String>,
}
