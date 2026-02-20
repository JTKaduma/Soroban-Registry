use axum::{http::StatusCode, response::IntoResponse};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace};
use prometheus::Encoder;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::metrics::REGISTRY;

pub fn init(otlp_endpoint: &str) {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint);

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(sdktrace::Config::default().with_resource(
            opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                "soroban-registry",
            )]),
        ))
        .install_batch(runtime::Tokio)
        .expect("failed to install OTLP tracer");

    let tracer = tracer_provider.tracer("soroban-registry");

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "api=debug,tower_http=debug".into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(OpenTelemetryLayer::new(tracer))
        .init();
}

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let mut buf = Vec::new();

    if encoder.encode(&REGISTRY.gather(), &mut buf).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "encoding error").into_response();
    }

    let content_type = encoder.format_type().to_string();
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_str(&content_type)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("text/plain")),
        )],
        buf,
    )
        .into_response()
use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime::Tokio;
use prometheus::Registry;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::metrics;

pub struct Observability {
    pub registry: Registry,
}

impl Observability {
    pub fn init() -> Result<Self> {
        let registry = Registry::new_custom(Some("soroban".into()), None)?;
        metrics::register_all(&registry)?;

        let otel_endpoint =
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".into());

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&otel_endpoint)
            .build()?;

        let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter, Tokio)
            .with_resource(opentelemetry_sdk::Resource::new(vec![
                KeyValue::new("service.name", "soroban-registry-api"),
            ]))
            .build();

        let tracer = tracer_provider.tracer("soroban-registry");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "api=debug,tower_http=debug".into());

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .with(otel_layer)
            .init();

        tracing::info!("Observability stack initialized (Prometheus + OTel → {})", otel_endpoint);
        Ok(Self { registry })
    }

    pub fn shutdown() {
        opentelemetry::global::shutdown_tracer_provider();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = Registry::new_custom(Some("test".into()), None).unwrap();
        metrics::register_all(&registry).unwrap();
        let families = registry.gather();
        assert!(families.len() >= 20, "expected ≥20 metric families, got {}", families.len());
    }

    #[test]
    fn test_metric_names_prefixed() {
        let registry = Registry::new_custom(Some("test".into()), None).unwrap();
        metrics::register_all(&registry).unwrap();
        let families = registry.gather();
        for fam in &families {
            assert!(
                fam.get_name().starts_with("test_"),
                "metric {} missing prefix",
                fam.get_name()
            );
        }
    }
}
