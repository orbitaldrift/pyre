use futures::FutureExt;
use opentelemetry_sdk::{
    error::OTelSdkResult,
    metrics::{exporter::PushMetricExporter, Temporality},
    Resource,
};

/// A compound telemetry exporter that can export metrics and traces to multiple endpoints.
#[derive(Debug)]
pub struct MetricsExporter {
    pub exporters: Vec<opentelemetry_otlp::MetricExporter>,
    pub temporality: Temporality,
}

impl PushMetricExporter for MetricsExporter {
    async fn export(
        &self,
        metrics: &mut opentelemetry_sdk::metrics::data::ResourceMetrics,
    ) -> opentelemetry_sdk::error::OTelSdkResult {
        for exporter in &self.exporters {
            exporter.export(metrics).await?;
        }
        Ok(())
    }

    fn force_flush(&self) -> OTelSdkResult {
        // Stateless
        Ok(())
    }

    fn shutdown(&self) -> OTelSdkResult {
        for exporter in &self.exporters {
            exporter.shutdown()?;
        }
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        self.temporality
    }
}

/// A compound telemetry exporter that can export metrics and traces to multiple endpoints.
#[derive(Debug)]
pub struct TracesExporter {
    pub exporters: Vec<opentelemetry_otlp::SpanExporter>,
}

impl opentelemetry_sdk::trace::SpanExporter for TracesExporter {
    async fn export(&self, spans: Vec<opentelemetry_sdk::trace::SpanData>) -> OTelSdkResult {
        let futs = self.exporters.iter().map(|exporter| {
            let spans = spans.clone();
            async move { exporter.export(spans).await }
        });
        futures::future::join_all(futs)
            .then(|_| async { Ok(()) })
            .await
    }

    fn set_resource(&mut self, resource: &Resource) {
        for exporter in &mut self.exporters {
            exporter.set_resource(resource);
        }
    }
}
