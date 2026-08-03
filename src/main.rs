// AnyWords - Web-based File Full-Text Search Engine
// Main entry point (standalone server)

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = anywords::run_server().await?;
    // Keep the main thread alive since run_server spawns the server in background
    tracing::info!("Server running on port {}", port);
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");
    Ok(())
}
