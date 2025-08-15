//! # Initializer
//!
//! Initialize the OpenTelemetry collectors and exporters.

use std::env;

use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde::Deserialize;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Telemetry resource.
#[derive(Debug, Default)]
pub struct Otel {}

/// Telemetry initializer.
pub struct OtelBuilder {
    /// The name of the application to for the purposes of identifying the
    /// service in telemetry data.
    app_name: String,

    /// The name of the environment, e.g. "production", "staging", "development".
    env_name: Option<String>,

    /// The OpenTelemetry metrics collection endpoint.
    endpoint: Option<String>,
}

impl Otel {
    /// Create a new telemetry builder with the default application name.
    pub fn builder() -> OtelBuilder {
        OtelBuilder::new("credibil")
    }
}

impl OtelBuilder {
    /// Create a new telemetry resource.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            app_name: name.into(),
            env_name: None,
            endpoint: None,
        }
    }

    /// Override the default app name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Set the environment name.
    #[must_use]
    pub fn env(mut self, env_name: impl Into<String>) -> Self {
        self.env_name = Some(env_name.into());
        self
    }

    /// Set the OpenTelemetry endpoint.
    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Initialize telemetry with the provided configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the telemetry system fails to initialize, such as if
    /// the OpenTelemetry exporter cannot be created or if setting the global
    /// subscriber fails.
    pub fn build(self) -> Result<Otel> {
        let config = &Config {
            app_name: self.app_name,
            env_name: self.env_name,
            endpoint: self.endpoint,
        };

        let resource = Resource::from(config.clone());

        // metrics
        let meter_provider = init_metrics(config, resource.clone())?;
        global::set_meter_provider(meter_provider);

        // tracer
        let tracer_provider = init_traces(config, resource)?;
        global::set_tracer_provider(tracer_provider.clone());

        // tracing
        let env_filter = EnvFilter::from_default_env();
        let fmt_layer =
            tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);
        let tracer = tracer_provider.tracer(config.app_name.clone());
        let tracing_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let subscriber = Registry::default().with(env_filter).with(fmt_layer).with(tracing_layer);
        tracing::subscriber::set_global_default(subscriber)?;

        Ok(Otel {})
    }
}

fn init_traces(config: &Config, resource: Resource) -> Result<SdkTracerProvider> {
    let mut builder = SpanExporter::builder().with_tonic();
    if let Some(endpoint) = &config.endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    let exporter = builder.build()?;

    Ok(SdkTracerProvider::builder().with_resource(resource).with_batch_exporter(exporter).build())
}

fn init_metrics(config: &Config, resource: Resource) -> Result<SdkMeterProvider> {
    let mut builder = MetricExporter::builder().with_tonic();
    if let Some(endpoint) = &config.endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    let exporter = builder.build()?;
    Ok(SdkMeterProvider::builder().with_resource(resource).with_periodic_exporter(exporter).build())
}

/// Telemetry configuration.
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// The name of the application to for the purposes of identifying the
    /// service in telemetry data.
    pub app_name: String,

    /// The name of the environment, e.g. "production", "staging", "development".
    pub env_name: Option<String>,

    /// The OpenTelemetry metrics collection endpoint.
    pub endpoint: Option<String>,
}

impl From<Config> for Resource {
    fn from(config: Config) -> Self {
        Self::builder()
            .with_service_name(config.app_name.clone())
            .with_attributes(vec![
                KeyValue::new(
                    "deployment.environment",
                    config.env_name.clone().unwrap_or_else(|| "unknown".to_string()),
                ),
                KeyValue::new("service.namespace", config.app_name),
                KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                KeyValue::new(
                    "service.instance.id",
                    env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
                ),
                KeyValue::new("telemetry.sdk.name", "opentelemetry"),
                KeyValue::new("instrumentation.provider", "opentelemetry"),
            ])
            .build()
    }
}
