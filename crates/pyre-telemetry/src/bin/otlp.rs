use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::{
    collector::{
        metrics::v1::{
            metrics_service_server::{MetricsService, MetricsServiceServer},
            ExportMetricsServiceRequest,
            ExportMetricsServiceResponse,
        },
        trace::v1::{
            trace_service_server::{TraceService, TraceServiceServer},
            ExportTraceServiceRequest,
            ExportTraceServiceResponse,
        },
    },
    metrics::v1::{
        metric::Data,
        number_data_point::Value,
        HistogramDataPoint,
        Metric,
        NumberDataPoint,
    },
    trace::v1::Span,
};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

// Storage for last received metrics and traces
#[derive(Debug, Default)]
struct TelemetryStore {
    metrics: Vec<Metric>,
    spans: Vec<Span>,
}

#[derive(Debug, Default)]
struct MetricsServiceImpl {
    store: Arc<Mutex<TelemetryStore>>,
}

#[derive(Debug, Default)]
struct TraceServiceImpl {
    store: Arc<Mutex<TelemetryStore>>,
}

#[tonic::async_trait]
impl MetricsService for MetricsServiceImpl {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        println!("\n==== RECEIVED METRICS ====");
        let datetime: DateTime<Utc> = SystemTime::now().into();
        println!("Time: {}", datetime.format("%Y-%m-%d %H:%M:%S"));

        let req = request.into_inner();
        let mut metrics_count = 0;

        for resource_metrics in &req.resource_metrics {
            // Print resource information
            if let Some(resource) = &resource_metrics.resource {
                println!("Resource:");
                for attr in &resource.attributes {
                    println!(
                        "  {}: {:?}",
                        attr.key,
                        attr.value.as_ref().map(|v| format!("{v:?}"))
                    );
                }
            }

            // Process all metrics
            for scope_metrics in &resource_metrics.scope_metrics {
                if let Some(scope) = &scope_metrics.scope {
                    println!("Scope: {}", scope.name);
                    if !scope.version.is_empty() {
                        println!("  Version: {}", scope.version);
                    }
                }

                for metric in &scope_metrics.metrics {
                    metrics_count += 1;
                    println!("\nMetric: {}", metric.name);
                    println!("  Description: {}", metric.description);

                    match &metric.data {
                        Some(data) => {
                            match data {
                                Data::Gauge(gauge) => {
                                    println!("  Type: Gauge");
                                    for dp in &gauge.data_points {
                                        print_data_point(dp);
                                    }
                                }
                                Data::Sum(sum) => {
                                    println!("  Type: Sum (Monotonic: {})", sum.is_monotonic);
                                    for dp in &sum.data_points {
                                        print_data_point(dp);
                                    }
                                }
                                Data::Histogram(histogram) => {
                                    println!("  Type: Histogram");
                                    for dp in &histogram.data_points {
                                        print_histogram_data_point(dp);
                                    }
                                }
                                Data::ExponentialHistogram(exp_hist) => {
                                    println!("  Type: Exponential Histogram");
                                    for dp in &exp_hist.data_points {
                                        println!(
                                            "    Data point with {} attributes",
                                            dp.attributes.len()
                                        );
                                    }
                                }
                                Data::Summary(summary) => {
                                    println!("  Type: Summary");
                                    for dp in &summary.data_points {
                                        println!(
                                            "    Data point with {} attributes",
                                            dp.attributes.len()
                                        );
                                    }
                                }
                            }
                        }
                        None => println!("  No data"),
                    }
                }

                // Store metrics for potential TUI usage
                let mut store = self.store.lock().await;
                for metric in &scope_metrics.metrics {
                    store.metrics.push(metric.clone());
                    // Keep only the last 100 metrics to avoid memory issues
                    if store.metrics.len() > 100 {
                        store.metrics.remove(0);
                    }
                }
            }
        }

        println!("Total metrics received: {}", metrics_count);
        println!("==== END METRICS ====\n");

        // Return success response
        Ok(Response::new(ExportMetricsServiceResponse {
            partial_success: None,
        }))
    }
}

#[tonic::async_trait]
impl TraceService for TraceServiceImpl {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        println!("\n==== RECEIVED TRACES ====");
        let datetime: DateTime<Utc> = SystemTime::now().into();
        println!("Time: {}", datetime.format("%Y-%m-%d %H:%M:%S"));

        let req = request.into_inner();
        let mut span_count = 0;

        for resource_span in &req.resource_spans {
            // Print resource information
            if let Some(resource) = &resource_span.resource {
                println!("Resource:");
                for attr in &resource.attributes {
                    println!(
                        "  {}: {:?}",
                        attr.key,
                        attr.value.as_ref().map(|v| format!("{v:?}"))
                    );
                }
            }

            // Process all spans
            for scope_span in &resource_span.scope_spans {
                if let Some(scope) = &scope_span.scope {
                    println!("Scope: {}", scope.name);
                }

                for span in &scope_span.spans {
                    span_count += 1;
                    println!("\nSpan: {} (ID: {})", span.name, hex::encode(&span.span_id));
                    if !span.trace_id.is_empty() {
                        println!("  Trace ID: {}", hex::encode(&span.trace_id));
                    }
                    if !span.parent_span_id.is_empty() {
                        println!("  Parent Span ID: {}", hex::encode(&span.parent_span_id));
                    }

                    println!("  Kind: {:?}", span.kind);

                    // Convert timestamps to human-readable format
                    if span.start_time_unix_nano > 0 {
                        let secs = (span.start_time_unix_nano / 1_000_000_000) as i64;
                        let nsecs = (span.start_time_unix_nano % 1_000_000_000) as u32;
                        let datetime = chrono::DateTime::from_timestamp(secs, nsecs)
                            .unwrap_or_default()
                            .to_utc();
                        println!("  Start Time: {}", datetime.format("%Y-%m-%d %H:%M:%S%.3f"));
                    }

                    if span.end_time_unix_nano > 0 {
                        let duration_ms = (span.end_time_unix_nano - span.start_time_unix_nano)
                            as f64
                            / 1_000_000.0;
                        println!("  Duration: {duration_ms:.3} ms");
                    }

                    // Print attributes
                    if !span.attributes.is_empty() {
                        println!("  Attributes:");
                        for attr in &span.attributes {
                            println!(
                                "    {}: {:?}",
                                attr.key,
                                attr.value.as_ref().map(|v| format!("{v:?}"))
                            );
                        }
                    }

                    // Print events
                    if !span.events.is_empty() {
                        println!("  Events:");
                        for event in &span.events {
                            let event_time = (event.time_unix_nano / 1_000_000_000) as i64;
                            let event_nsecs = (event.time_unix_nano % 1_000_000_000) as u32;
                            let event_datetime =
                                chrono::DateTime::from_timestamp(event_time, event_nsecs)
                                    .unwrap_or_default()
                                    .to_utc();
                            println!("    [{}] {}", event_datetime.format("%H:%M:%S"), event.name);

                            if !event.attributes.is_empty() {
                                for attr in &event.attributes {
                                    println!(
                                        "      {}: {:?}",
                                        attr.key,
                                        attr.value.as_ref().map(|v| format!("{v:?}"))
                                    );
                                }
                            }
                        }
                    }

                    // Print status
                    if let Some(status) = &span.status {
                        println!("  Status: {:?} - {}", status.code, status.message);
                    }
                }

                // Store spans for potential TUI usage
                let mut store = self.store.lock().await;
                for span in &scope_span.spans {
                    store.spans.push(span.clone());
                    // Keep only the last 100 spans to avoid memory issues
                    if store.spans.len() > 100 {
                        store.spans.remove(0);
                    }
                }
            }
        }

        println!("Total spans received: {}", span_count);
        println!("==== END TRACES ====\n");

        // Return success response
        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

fn print_data_point(dp: &NumberDataPoint) {
    // Print attributes
    if !dp.attributes.is_empty() {
        println!("    Attributes:");
        for attr in &dp.attributes {
            println!(
                "      {}: {:?}",
                attr.key,
                attr.value.as_ref().map(|v| format!("{v:?}"))
            );
        }
    }

    // Convert timestamp to readable format
    if dp.time_unix_nano > 0 {
        let secs = (dp.time_unix_nano / 1_000_000_000) as i64;
        let nsecs = (dp.time_unix_nano % 1_000_000_000) as u32;
        let datetime = chrono::DateTime::from_timestamp(secs, nsecs)
            .unwrap_or_default()
            .to_utc();
        println!("    Time: {}", datetime.format("%Y-%m-%d %H:%M:%S"));
    }

    // Print value
    match &dp.value {
        Some(v) => {
            match v {
                Value::AsDouble(val) => {
                    println!("    Value: {val}");
                }
                Value::AsInt(val) => {
                    println!("    Value: {val}");
                }
            }
        }
        None => println!("    No value"),
    }
}

fn print_histogram_data_point(dp: &HistogramDataPoint) {
    // Print attributes
    if !dp.attributes.is_empty() {
        println!("    Attributes:");
        for attr in &dp.attributes {
            println!(
                "      {}: {:?}",
                attr.key,
                attr.value.as_ref().map(|v| format!("{v:?}"))
            );
        }
    }

    // Convert timestamp to readable format
    if dp.time_unix_nano > 0 {
        let secs = (dp.time_unix_nano / 1_000_000_000) as i64;
        let nsecs = (dp.time_unix_nano % 1_000_000_000) as u32;
        let datetime = chrono::DateTime::from_timestamp(secs, nsecs)
            .unwrap_or_default()
            .to_utc();
        println!("    Time: {}", datetime.format("%Y-%m-%d %H:%M:%S"));
    }

    println!("    Count: {}", dp.count);
    println!("    Sum: {:?}", dp.sum);

    // Print buckets
    if !dp.bucket_counts.is_empty() && dp.bucket_counts.len() == dp.explicit_bounds.len() + 1 {
        println!("    Buckets:");
        for i in 0..dp.bucket_counts.len() {
            if i == 0 {
                println!(
                    "      (-∞, {}]: {}",
                    dp.explicit_bounds[i], dp.bucket_counts[i]
                );
            } else if i == dp.bucket_counts.len() - 1 {
                println!(
                    "      ({}, +∞): {}",
                    dp.explicit_bounds[i - 1],
                    dp.bucket_counts[i]
                );
            } else {
                println!(
                    "      ({}, {}]: {}",
                    dp.explicit_bounds[i - 1],
                    dp.explicit_bounds[i],
                    dp.bucket_counts[i]
                );
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:4318".parse::<SocketAddr>()?;
    println!("OTLP Receiver starting on {addr}");

    // Create shared telemetry store
    let telemetry_store = Arc::new(Mutex::new(TelemetryStore::default()));

    // Create service implementations
    let metrics_service = MetricsServiceImpl {
        store: telemetry_store.clone(),
    };

    let trace_service = TraceServiceImpl {
        store: telemetry_store.clone(),
    };

    println!("Waiting for metrics and traces...");

    // Start the server
    Server::builder()
        .add_service(
            MetricsServiceServer::new(metrics_service)
                .accept_compressed(tonic::codec::CompressionEncoding::Zstd)
                .send_compressed(tonic::codec::CompressionEncoding::Zstd),
        )
        .add_service(
            TraceServiceServer::new(trace_service)
                .accept_compressed(tonic::codec::CompressionEncoding::Zstd)
                .send_compressed(tonic::codec::CompressionEncoding::Zstd),
        )
        .serve(addr)
        .await?;

    Ok(())
}
