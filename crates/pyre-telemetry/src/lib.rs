use std::time::Duration;

use compound::{MetricsExporter, TracesExporter};
use config::{Config, Endpoint, Layers, Mode, Temporality};
use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_resource_detectors::K8sResourceDetector;
use opentelemetry_sdk::{
    logs::{SdkLogger, SdkLoggerProvider},
    metrics::SdkMeterProvider,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider, Tracer},
    Resource,
};
use suspendable::{Suspendable, SuspendableLayer};
use tracing::{
    info,
    subscriber::{DefaultGuard, SetGlobalDefaultError},
    Subscriber,
};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::{SubscriberInitExt, TryInitError},
    EnvFilter,
    Registry,
};

mod compound;
pub mod config;
pub mod suspendable;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to initialize telemetry: {0}")]
    Initialization(#[from] TryInitError),

    #[error("failed to initialize telemetry: {0}")]
    InitializationSetGlobal(#[from] SetGlobalDefaultError),
}

#[derive(Clone, Debug)]
pub struct Info {
    pub id: String,
    pub domain: String,
    pub meta: Option<Vec<KeyValue>>,
}

pub type LayerStack<S, T> = (
    EnvFilter,
    Option<OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>>,
    Option<MetricsLayer<S>>,
    Option<OpenTelemetryLayer<T, Tracer>>,
);

/// Telemetry configuration for the application.
///
/// This struct provides methods to initialize telemetry with either stdout exporters
/// (for local development) or OTLP exporters (for production environments).
pub struct Telemetry {
    tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    meter_provider: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
    logger_provider: Option<opentelemetry_sdk::logs::SdkLoggerProvider>,
    config: Config,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new_with_stdout(&Config::default())
    }
}

impl Telemetry {
    /// Create a new telemetry instance with the given unique ID and configuration.
    /// Usually the unique Id is the public key of the user.
    #[must_use]
    pub fn new(config: &Config, info: Info) -> Self {
        match config.mode {
            Mode::Stdout => Self::new_with_stdout(config),
            Mode::Otlp | Mode::OtlpAlt | Mode::Dual | Mode::Custom(_) => {
                let resource = Self::get_resource(info);
                Self::new_with_otlp(config, &resource)
            }
        }
    }

    /// Initialize telemetry with stdout exporters for local development.
    fn new_with_stdout(config: &Config) -> Self {
        let layers: Layers = config.layers.clone().into();

        let tracer_provider = layers.contains(Layers::Traces).then(|| {
            opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(opentelemetry_stdout::SpanExporter::default())
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .build()
        });

        let meter_provider = layers.contains(Layers::Metrics).then(|| {
            opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_periodic_exporter(opentelemetry_stdout::MetricExporter::default())
                .build()
        });

        Self {
            tracer_provider,
            meter_provider,
            logger_provider: None,
            config: config.clone(),
        }
    }

    /// Initialize telemetry with OTLP exporters for production environments.
    fn new_with_otlp(config: &Config, resource: &Resource) -> Self {
        let layers: Layers = config.layers.clone().into();
        let endpoints: Vec<Endpoint> = config.mode.clone().into();

        Self {
            tracer_provider: Self::get_traces(&layers, endpoints.clone(), resource.clone()),
            meter_provider: Self::get_metrics(
                &layers,
                endpoints.clone(),
                Duration::from_secs(config.interval),
                resource.clone(),
                config.temporality.clone(),
            ),
            logger_provider: Self::get_logs(&layers, resource.clone()),
            config: config.clone(),
        }
    }

    /// Temporary function to use stdout telemetry before the full telemetry is set up.
    #[must_use]
    pub fn stdout() -> DefaultGuard {
        tracing::subscriber::set_default(
            Registry::default()
                .with(Self::default_fmt_layer())
                .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into())),
        )
    }

    /// Initialize the telemetry provider with a suspendable layer.
    ///
    /// # Errors
    /// If a global default subscriber has already been set, this function will return an error.
    pub fn init_suspendable<S>(self, layer: S) -> Result<Self, Error>
    where
        S: Suspendable + Send + Sync + 'static,
    {
        info!(config = %self.config, "initializing suspendable telemetry");

        let layer: SuspendableLayer<Registry, fmt::Layer<Registry>, S> =
            SuspendableLayer::new(Self::default_fmt_layer(), layer);

        tracing::subscriber::set_global_default(
            Registry::default().with(layer).with(self.get_filter()),
        )?;

        Ok(Self {
            tracer_provider: self.tracer_provider,
            meter_provider: self.meter_provider,
            logger_provider: self.logger_provider,
            config: self.config,
        })
    }

    fn build_layers<S, T>(&self) -> LayerStack<S, T>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
        T: Subscriber + for<'span> LookupSpan<'span>,
    {
        let filter = self.get_filter();

        let logs_layer = self
            .logger_provider
            .as_ref()
            .map(opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new);

        let metrics_layer = self.meter_provider.as_ref().map(|meter_provider| {
            tracing_opentelemetry::MetricsLayer::<S>::new(meter_provider.clone())
        });

        let tracer_layer = self.tracer_provider.as_ref().map(|tracer_provider| {
            tracing_opentelemetry::layer::<T>()
                .with_tracer(tracer_provider.tracer(env!("CARGO_PKG_NAME").to_string()))
        });

        (filter, logs_layer, metrics_layer, tracer_layer)
    }

    /// Initialize the global telemetry providers and set up tracing subscribers.
    ///
    /// # Errors
    /// If a global default subscriber has already been set, this function will return an error.
    pub fn init(self) -> Result<Self, Error> {
        info!(config = %self.config, "initializing global telemetry");

        let (filter, logs_layer, metrics_layer, tracer_layer) = self.build_layers();

        // Prevent the meter provider from being dropped when the telemetry instance is dropped.
        // Please don't remove this, as it should be safe to drop the telemetry instance
        std::mem::forget(self.meter_provider);

        Registry::default()
            .with(Self::default_fmt_layer())
            .with(filter)
            .with(logs_layer)
            .with(metrics_layer)
            .with(tracer_layer)
            .try_init()?;

        Ok(Self {
            tracer_provider: self.tracer_provider,
            meter_provider: None,
            logger_provider: self.logger_provider,
            config: self.config,
        })
    }

    /// Initialize the global telemetry providers and set up tracing subscribers.
    ///
    /// # Errors
    /// If a global default subscriber has already been set, this function will return an error.
    pub fn init_scoped(self) -> Result<(Self, DefaultGuard), Error> {
        info!(config = %self.config, "initializing scoped telemetry");

        let (filter, logs_layer, metrics_layer, tracer_layer) = self.build_layers();

        // Prevent the meter provider from being dropped when the telemetry instance is dropped.
        // Please don't remove this, as it should be safe to drop the telemetry instance
        std::mem::forget(self.meter_provider);

        let guard = tracing::subscriber::set_default(
            Registry::default()
                .with(Self::default_fmt_layer())
                .with(filter)
                .with(logs_layer)
                .with(metrics_layer)
                .with(tracer_layer),
        );

        Ok((
            Self {
                tracer_provider: self.tracer_provider,
                meter_provider: None,
                logger_provider: self.logger_provider,
                config: self.config,
            },
            guard,
        ))
    }

    fn get_metrics(
        layers: &Layers,
        endpoints: Vec<Endpoint>,
        interval: Duration,
        resource: Resource,
        temporality: Temporality,
    ) -> Option<SdkMeterProvider> {
        if !layers.contains(Layers::Metrics) {
            return None;
        }

        let exporters: Vec<opentelemetry_otlp::MetricExporter> = endpoints
            .into_iter()
            .map(|endpoint| {
                opentelemetry_otlp::MetricExporter::builder()
                    .with_tonic()
                    .with_compression(opentelemetry_otlp::Compression::Zstd)
                    .with_export_config(endpoint.into())
                    .build()
                    .expect("Failed to create metric exporter")
            })
            .collect();

        Some(
            opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_reader(
                    opentelemetry_sdk::metrics::PeriodicReader::builder(MetricsExporter {
                        exporters,
                        temporality: temporality.into(),
                    })
                    .with_interval(interval)
                    .build(),
                )
                .with_resource(resource)
                .build(),
        )
    }

    fn get_traces(
        layers: &Layers,
        endpoints: Vec<Endpoint>,
        resource: Resource,
    ) -> Option<SdkTracerProvider> {
        if !layers.contains(Layers::Traces) {
            return None;
        }

        let exporters: Vec<opentelemetry_otlp::SpanExporter> = endpoints
            .into_iter()
            .map(|endpoint| {
                opentelemetry_otlp::SpanExporter::builder()
                    .with_tonic()
                    .with_compression(opentelemetry_otlp::Compression::Zstd)
                    .with_export_config(endpoint.into())
                    .build()
                    .expect("Failed to create span exporter")
            })
            .collect();

        Some(
            opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(TracesExporter { exporters })
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(16)
                .with_resource(resource)
                .build(),
        )
    }

    fn get_logs(layers: &Layers, resource: Resource) -> Option<SdkLoggerProvider> {
        if !layers.contains(Layers::Logs) {
            return None;
        }

        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_compression(opentelemetry_otlp::Compression::Zstd)
            .with_export_config(Endpoint::LocalOtlp.into())
            .build()
            .expect("Failed to create log exporter");

        Some(
            opentelemetry_sdk::logs::SdkLoggerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource)
                .build(),
        )
    }

    fn get_resource(info: Info) -> Resource {
        Resource::builder_empty()
            .with_detector(Box::new(K8sResourceDetector))
            .with_attribute(opentelemetry::KeyValue::new("service.id", info.id))
            .with_attribute(opentelemetry::KeyValue::new("service.domain", info.domain))
            .with_attributes(info.meta.unwrap_or_default())
            .with_service_name(env!("CARGO_PKG_NAME").to_string())
            .build()
    }

    fn get_filter(&self) -> EnvFilter {
        let mut filter = self.config.filter.clone();
        filter.push(self.config.level.clone());

        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter.join(",")))
    }

    fn default_fmt_layer() -> fmt::Layer<Registry> {
        fmt::layer()
            .with_ansi(cfg!(debug_assertions))
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_thread_names(true)
            .with_span_events(FmtSpan::CLOSE)
    }
}

#[cfg(test)]
mod tests {
    #[tracing::instrument]
    fn sum(a: i32, b: i32) -> i32 {
        a + b
    }

    #[test]
    fn test_default_init() {
        let telemetry = super::Telemetry::default().init_scoped().inspect_err(|e| {
            tracing::error!("Failed to initialize telemetry: {}", e);
        });
        assert!(telemetry.is_ok());

        tracing::info!(counter.foo = 1, "Test counter");
        tracing::info!(histogram.foo = 1, "Test histogram");

        sum(1, 1);
    }

    #[test]
    fn test_stdout_scoped() {
        let _guard = super::Telemetry::stdout();

        tracing::info!(counter.foo = 1, "Test counter");
        tracing::info!(histogram.foo = 1, "Test histogram");

        sum(1, 1);
    }

    #[tokio::test]
    async fn test_otlp_scoped() {
        let info = super::Info {
            id: "test_id".to_string(),
            domain: "test_domain".to_string(),
            meta: None,
        };

        let config = super::config::Config {
            mode: super::config::Mode::OtlpAlt,
            layers: "metrics,traces,logs".to_string(),
            level: "info".to_string(),
            filter: vec![],
            interval: 30,
            temporality: super::config::Temporality::Cumulative,
        };

        let telemetry = super::Telemetry::new(&config, info)
            .init_scoped()
            .inspect_err(|e| {
                tracing::error!("Failed to initialize telemetry: {}", e);
            });
        assert!(telemetry.is_ok());

        let (_telemetry, _guard) = telemetry.unwrap();

        tracing::info!(counter.foo = 1, "Test counter");
        tracing::info!(histogram.foo = 1, "Test histogram");

        sum(1, 1);
    }
}
