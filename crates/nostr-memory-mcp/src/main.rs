use mcp_server::{ByteTransport, Server};
use mcp_server::router::RouterService;
use nostr_memory_mcp::NostrMcpRouter;
use std::env;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Nostr Memory MCP server");

    let nsec = env::var("NOSTR_NSEC").ok();
    let router = RouterService(NostrMcpRouter::new(nsec));
    
    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    server.run(transport).await?;
    
    Ok(())
}
