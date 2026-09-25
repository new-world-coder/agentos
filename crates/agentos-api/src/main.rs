use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "agentos-api", about = "AgentOS HTTP API server")]
struct Args {
    /// Bind address
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// SQLite database path (default: in-memory)
    #[arg(long)]
    db: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    agentos_api::serve(args.bind, args.db).await
}
