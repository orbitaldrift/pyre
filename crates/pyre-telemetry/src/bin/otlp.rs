use std::{self, net::SocketAddr, process::exit, sync::Arc};

use clap::{command, Parser, Subcommand};
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
    metrics::v1::Metric,
    trace::v1::Span,
};
use pyre_telemetry::Telemetry;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info};

#[derive(Debug, Default)]
struct TelemetryStore {
    _metrics: Vec<Metric>,
    _spans: Vec<Span>,
}

#[derive(Debug, Default)]
struct MetricsServiceImpl {
    _store: Arc<Mutex<TelemetryStore>>,
}

#[derive(Debug, Default)]
struct TraceServiceImpl {
    _store: Arc<Mutex<TelemetryStore>>,
}

#[tonic::async_trait]
impl MetricsService for MetricsServiceImpl {
    async fn export(
        &self,
        _request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        info!("Received metrics export request");

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
        _request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        info!("Received traces export request");

        // Return success response
        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Start otlp
    Start {
        /// Address to bind the server to
        #[clap(short = 'a', long, default_value = "127.0.0.1:4318", value_parser = clap::value_parser!(SocketAddr))]
        addr: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        let _g = Telemetry::stdout();
        Telemetry::default()
            .init()
            .inspect_err(|e| {
                error!("Failed to initialize telemetry: {}", e);
                exit(1);
            })
            .expect("Failed to initialize telemetry");
    }

    let args = Args::parse();

    match args.cmd {
        Commands::Start { addr } => {
            let telemetry_store = Arc::new(Mutex::new(TelemetryStore::default()));

            let metrics_service = MetricsServiceImpl {
                _store: telemetry_store.clone(),
            };

            let trace_service = TraceServiceImpl {
                _store: telemetry_store.clone(),
            };

            info!("starting OTLP server on {}", addr);

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
        }
    }

    Ok(())
}
