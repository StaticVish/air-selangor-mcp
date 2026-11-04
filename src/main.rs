use air_selangor_mcp::{AppConfig, McpServer};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Direct all tracing logs to stderr so stdout remains exclusively for MCP JSON-RPC messages
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("air_selangor_mcp=info".parse()?))
        .init();

    let config = AppConfig::load();
    let server = McpServer::new(config);

    server.run_stdio_loop().await?;
    Ok(())
}
