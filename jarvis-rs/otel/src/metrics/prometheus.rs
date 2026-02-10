// Temporariamente comentado - código experimental que requer dependência prometheus
// TODO: Adicionar dependência prometheus ao Cargo.toml ou implementar de forma diferente

use crate::metrics::error::MetricsError;
use crate::metrics::error::Result;

// Temporariamente comentado - prometheus não está nas dependências
// use prometheus::Encoder;
// use prometheus::Registry;
// use prometheus::TextEncoder;
// use opentelemetry::metrics::Meter;
// use opentelemetry::metrics::MeterProvider as _;
// use opentelemetry_sdk::Resource;
// use opentelemetry_sdk::metrics::ManualReader;
// use opentelemetry_sdk::metrics::SdkMeterProvider;
// use opentelemetry_sdk::metrics::Temporality;
// use opentelemetry_sdk::metrics::data::ResourceMetrics;
// use opentelemetry_semantic_conventions as semconv;
// use std::sync::Arc;
// use std::sync::Mutex;

// const ENV_ATTRIBUTE: &str = "env";

// Temporariamente comentado - prometheus não está nas dependências
// /// Prometheus exporter for OpenTelemetry metrics.
// ///
// /// This exporter collects metrics from OpenTelemetry and exposes them
// /// via a Prometheus-compatible HTTP endpoint.
// pub struct PrometheusExporter {
//     registry: Arc<Registry>,
//     provider: Arc<SdkMeterProvider>,
//     meter: Meter,
// }
//
// impl PrometheusExporter {
//     /// Create a new Prometheus exporter.
//     pub fn new(
//         environment: String,
//         service_name: String,
//         service_version: String,
//     ) -> Result<Self> {
//         let registry = Arc::new(Registry::new());
//         let resource = Resource::builder()
//             .with_service_name(service_name.clone())
//             .with_attributes(vec![
//                 opentelemetry::KeyValue::new(semconv::attribute::SERVICE_VERSION, service_version),
//                 opentelemetry::KeyValue::new(ENV_ATTRIBUTE, environment),
//             ])
//             .build();
//
//         // Create a manual reader that collects metrics on demand
//         let reader = Arc::new(
//             ManualReader::builder()
//                 .with_temporality(Temporality::Cumulative)
//                 .build(),
//         );
//
//         let provider = Arc::new(
//             SdkMeterProvider::builder()
//                 .with_resource(resource)
//                 .with_reader(reader.clone())
//                 .build(),
//         );
//
//         let meter = provider.meter("Jarvis");
//
//         Ok(Self {
//             registry,
//             provider,
//             meter,
//         })
//     }
//
//     /// Get the Prometheus registry for direct metric registration.
//     pub fn registry(&self) -> &Registry {
//         &self.registry
//     }
//
//     /// Get the OpenTelemetry meter for instrumenting code.
//     pub fn meter(&self) -> &Meter {
//         &self.meter
//     }
//
//     /// Export metrics in Prometheus text format.
//     pub fn export(&self) -> Result<String> {
//         let mut buffer = Vec::new();
//         let encoder = TextEncoder::new();
//         let metric_families = self.registry.gather();
//         encoder.encode(&metric_families, &mut buffer)?;
//         Ok(String::from_utf8(buffer)?)
//     }
//
//     /// Collect metrics from the OpenTelemetry provider and convert to Prometheus format.
//     pub fn collect(&self) -> Result<String> {
//         let mut snapshot = ResourceMetrics::default();
//         // Note: Manual collection would require access to the reader
//         // For now, we rely on the registry-based approach
//         self.export()
//     }
//
//     /// Shutdown the exporter.
//     pub fn shutdown(&self) -> Result<()> {
//         self.provider
//             .shutdown()
//             .map_err(|source| MetricsError::ProviderShutdown { source })?;
//         Ok(())
//     }
// }
