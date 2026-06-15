use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    runtime,
    trace::{self, RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::resource as semconv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Guard returned by `init` — call `.shutdown()` before process exit so any
/// in-flight spans flush to the collector instead of getting dropped.
pub struct TelemetryGuard {
    provider: Option<TracerProvider>,
}

impl TelemetryGuard {
    pub fn shutdown(mut self) {
        if let Some(p) = self.provider.take() {
            // Best-effort flush. We don't propagate the error: shutting down
            // because the binary is exiting anyway.
            let _ = p.shutdown();
        }
    }
}

/// Initialise tracing/metrics output.
///
/// Selection by env:
/// - `RUST_LOG`                    → EnvFilter directives, default `info`.
/// - `LOG_FORMAT=json|pretty`      → stdout formatter, default `pretty` for
///                                    dev, `json` when this binary runs in
///                                    a container is recommended.
/// - `OTEL_EXPORTER_OTLP_ENDPOINT` → enables OTLP gRPC export when set.
///                                    Typical value: `http://otel-collector:4317`.
///
/// Resource attributes attached to every span:
///   service.name = api
///   service.version = CARGO_PKG_VERSION
///   deployment.environment = ENVIRONMENT (defaults to "dev")
pub fn init(default_filter: &str) -> anyhow::Result<TelemetryGuard> {
    // W3C TraceContext on all outgoing requests, so downstream services see
    // the same trace_id. Set this even when the OTLP exporter is off — local
    // logs still include trace IDs if a parent span exists in headers.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    // ─── Stdout layer (pretty for dev, JSON for prod/observability) ─────
    let log_format = std::env::var("LOG_FORMAT")
        .unwrap_or_else(|_| "pretty".into())
        .to_ascii_lowercase();

    let stdout_layer: Box<dyn Layer<_> + Send + Sync + 'static> = if log_format == "json" {
        Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_target(true),
        )
    } else {
        Box::new(tracing_subscriber::fmt::layer().with_target(false))
    };

    // ─── Optional OTLP layer ────────────────────────────────────────────
    let (otel_layer, provider) = match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) if !endpoint.is_empty() => {
            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&endpoint);

            let provider = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(exporter)
                .with_trace_config(
                    trace::Config::default()
                        // ParentBased + AlwaysOn: honour the upstream
                        // sampling decision when present, otherwise sample
                        // everything (fine at low volume; switch to
                        // TraceIdRatioBased(0.1) at >100 RPS).
                        .with_sampler(Sampler::ParentBased(Box::new(Sampler::AlwaysOn)))
                        .with_id_generator(RandomIdGenerator::default())
                        .with_resource(resource()),
                )
                .install_batch(runtime::Tokio)?;

            let tracer = provider.tracer("api");
            let layer = tracing_opentelemetry::layer().with_tracer(tracer);
            (Some(layer), Some(provider))
        }
        _ => (None, None),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(otel_layer)
        .init();

    Ok(TelemetryGuard { provider })
}

fn resource() -> Resource {
    Resource::new(vec![
        KeyValue::new(semconv::SERVICE_NAME, "api"),
        KeyValue::new(semconv::SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        KeyValue::new(
            semconv::DEPLOYMENT_ENVIRONMENT_NAME,
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".into()),
        ),
    ])
}
