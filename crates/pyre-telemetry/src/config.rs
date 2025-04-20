use std::{fmt::Display, time::Duration};

use bitflags::bitflags;
use garde::Validate;
use opentelemetry_otlp::ExportConfig;
use serde::{Deserialize, Serialize};

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[garde(skip)]
    pub mode: Mode,
    #[garde(ascii)]
    pub layers: String,
    #[garde(ascii, length(min = 1))]
    pub filter: String,
    #[garde(range(min = 5, max = 60))]
    pub interval: u64,
    #[garde(skip)]
    pub temporality: Temporality,
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ mode: {}, layers: {}, filter: {}, interval: {}, temporality: {} }}",
            self.mode, self.layers, self.filter, self.interval, self.temporality
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layers: "metrics".to_string(),
            filter: "info".to_string(),
            interval: 30,
            temporality: Temporality::default(),
            mode: Mode::default(),
        }
    }
}

#[derive(Debug, strum::Display, Clone, Default, Serialize, Deserialize)]
pub enum Temporality {
    #[default]
    Cumulative,
    Delta,
}

impl From<Temporality> for opentelemetry_sdk::metrics::Temporality {
    fn from(value: Temporality) -> Self {
        match value {
            Temporality::Cumulative => opentelemetry_sdk::metrics::Temporality::Cumulative,
            Temporality::Delta => opentelemetry_sdk::metrics::Temporality::Delta,
        }
    }
}

#[derive(Debug, strum::Display, Clone, Default, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Stdout,
    Alloy,
    Otlp,
    Dual,
    Custom(Vec<String>),
}

impl From<Mode> for Vec<Endpoint> {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Stdout => vec![],
            Mode::Alloy => vec![Endpoint::LocalAlloy],
            Mode::Otlp => vec![Endpoint::LocalOtlp],
            Mode::Dual => {
                vec![Endpoint::LocalAlloy, Endpoint::LocalOtlp]
            }
            Mode::Custom(endpoints) => endpoints.into_iter().map(Endpoint::Other).collect(),
        }
    }
}

#[derive(Debug, strum::Display, Clone, Serialize, Deserialize)]
pub enum Endpoint {
    #[strum(to_string = "http://localhost:4317")]
    LocalAlloy,
    #[strum(to_string = "http://localhost:4318")]
    LocalOtlp,
    #[strum(to_string = "{0}")]
    Other(String),
}

impl From<Endpoint> for ExportConfig {
    fn from(value: Endpoint) -> Self {
        ExportConfig {
            endpoint: Some(value.to_string()),
            timeout: Some(Duration::from_secs(5)),
            protocol: opentelemetry_otlp::Protocol::Grpc,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Layers(u8);
bitflags! {
    impl Layers: u8 {
        const Logs = 0b0000_0001;
        const Metrics = 0b0000_0010;
        const Traces = 0b0000_0100;
    }
}

impl From<String> for Layers {
    fn from(value: String) -> Self {
        value
            .replace(' ', "")
            .split(',')
            .filter_map(|layer| {
                match layer {
                    "logs" => Some(Layers::Logs),
                    "metrics" => Some(Layers::Metrics),
                    "traces" => Some(Layers::Traces),
                    _ => None,
                }
            })
            .fold(Layers(0), |acc, layer| acc | layer)
    }
}
