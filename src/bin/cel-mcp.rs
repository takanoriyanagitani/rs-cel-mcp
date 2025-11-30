use clap::Parser;
use rmcp::{
    ServiceExt,
    transport::{
        stdio,
        streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
    },
};
use rs_cel_mcp::cel_tool::{CelTool, evaluator_service};
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Use stdio for transport. This is the default if --http is not specified.
    #[arg(long)]
    stdio: bool,

    /// Use HTTP for transport, specifying the listen address (e.g., "127.0.0.1:8080").
    #[arg(long)]
    http: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(evaluator_service(rx));

    if let Some(addr_str) = args.http {
        let addr: SocketAddr = addr_str.parse()?;
        tracing::info!("Starting HTTP server on http://{}", addr);

        let service = StreamableHttpService::new(
            move || Ok(CelTool::new(tx.clone())),
            LocalSessionManager::default().into(),
            rmcp::transport::streamable_http_server::StreamableHttpServerConfig {
                stateful_mode: false,
                ..Default::default()
            },
        );

        let app = axum::Router::new().nest_service("/mcp", service);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("Listening on {}", listener.local_addr()?);
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                if let Err(e) = tokio::signal::ctrl_c().await {
                    tracing::error!("Failed to listen for ctrl-c signal: {}", e);
                }
                tracing::info!("Ctrl-C received, shutting down.");
            })
            .await?;
    } else {
        println!("Starting CEL MCP server on stdio...");
        let service = CelTool::new(tx).serve(stdio()).await?;
        eprintln!("Server ready.");
        service.waiting().await?;
    }

    Ok(())
}
