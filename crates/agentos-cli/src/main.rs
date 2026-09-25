use std::path::PathBuf;
use std::sync::Arc;

use agentos_core::RunId;
use agentos_policy::PolicyEngine;
use agentos_provider::MockProvider;
use agentos_runtime::{ApprovalDecision, Runtime, RuntimeConfig, StepOutcome};
use agentos_store::{MemoryStore, SqliteStore, Store};
use agentos_tools::ToolRegistry;
use anyhow::Context;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "agentos", about = "AgentOS durable agent runtime CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// SQLite database path (default: in-memory for ephemeral runs)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check toolchain, crates, and store health
    Doctor,
    /// Create a run with the given goal
    Run {
        /// Goal / prompt for the agent
        goal: String,
        /// Drive the run immediately
        #[arg(long, default_value_t = true)]
        drive: bool,
    },
    /// Drive an existing run
    Drive { run_id: String },
    /// Resume a run after crash / restart
    Resume { run_id: String },
    /// Approve or reject a pending effect
    Approve {
        run_id: String,
        effect_id: String,
        #[arg(long)]
        reject: bool,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Show run status
    Status { run_id: String },
    /// Print journal for a run
    Journal { run_id: String },
}

fn build_runtime(db: Option<PathBuf>) -> anyhow::Result<Runtime> {
    let store: Arc<dyn Store> = match db {
        Some(path) => Arc::new(SqliteStore::open(path).context("open sqlite")?),
        None => Arc::new(MemoryStore::new()),
    };
    let tools = ToolRegistry::with_builtins();
    let policy = PolicyEngine::with_known_tools(tools.names());
    Ok(Runtime::new(
        store,
        Arc::new(MockProvider::echo()),
        tools,
        policy,
        RuntimeConfig::default(),
    ))
}

fn print_outcome(outcome: &StepOutcome) {
    match outcome {
        StepOutcome::Completed => println!("outcome: completed"),
        StepOutcome::AwaitingApproval { effect_id } => {
            println!("outcome: awaiting_approval");
            println!("effect_id: {effect_id}");
        }
        StepOutcome::Failed { error } => {
            println!("outcome: failed");
            println!("error: {error}");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("agentos=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor => doctor(cli.db).await?,
        Commands::Run { goal, drive } => {
            let rt = build_runtime(cli.db)?;
            let run = rt.create_run(goal).await?;
            println!("run_id: {}", run.id);
            if drive {
                let outcome = rt.drive(&run.id).await?;
                print_outcome(&outcome);
                let run = rt.store().get_run(&run.id).await?.unwrap();
                println!("status: {}", run.status);
            }
        }
        Commands::Drive { run_id } => {
            let rt = build_runtime(cli.db)?;
            let outcome = rt.drive(&RunId(run_id.clone())).await?;
            print_outcome(&outcome);
        }
        Commands::Resume { run_id } => {
            let rt = build_runtime(cli.db)?;
            let outcome = rt.resume(&RunId(run_id)).await?;
            print_outcome(&outcome);
        }
        Commands::Approve {
            run_id,
            effect_id,
            reject,
            reason,
        } => {
            let rt = build_runtime(cli.db)?;
            let decision = if reject {
                ApprovalDecision::Reject {
                    reason: reason.unwrap_or_else(|| "rejected via cli".into()),
                }
            } else {
                ApprovalDecision::Approve
            };
            let outcome = rt
                .resolve_approval(&RunId(run_id), &effect_id, decision)
                .await?;
            print_outcome(&outcome);
        }
        Commands::Status { run_id } => {
            let rt = build_runtime(cli.db)?;
            let run = rt
                .store()
                .get_run(&RunId(run_id.clone()))
                .await?
                .with_context(|| format!("run {run_id} not found"))?;
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
        Commands::Journal { run_id } => {
            let rt = build_runtime(cli.db)?;
            let entries = rt.store().list_journal(&RunId(run_id)).await?;
            agentos_core::verify_chain(&entries)?;
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
    }
    Ok(())
}

async fn doctor(db: Option<PathBuf>) -> anyhow::Result<()> {
    println!("AgentOS doctor");
    println!("==============");
    println!("version:      {}", env!("CARGO_PKG_VERSION"));
    println!("rustc:        {}", rustc_version());
    println!("crates:       core, store, provider, tools, policy, runtime, api, cli");

    // Memory store
    {
        let store = MemoryStore::new();
        let run = agentos_core::Run::new("doctor-memory");
        store.create_run(&run).await?;
        let got = store.get_run(&run.id).await?;
        assert!(got.is_some());
        println!("memory store: ok");
    }

    // SQLite store
    let sqlite_path = db.unwrap_or_else(|| std::env::temp_dir().join("agentos-doctor.db"));
    {
        let store = SqliteStore::open(&sqlite_path)?;
        let run = agentos_core::Run::new("doctor-sqlite");
        store.create_run(&run).await?;
        let tip = store.tip_hash(&run.id).await?;
        assert_eq!(tip, agentos_core::Hash::genesis());
        println!("sqlite store: ok ({})", sqlite_path.display());
    }

    // Runtime smoke
    {
        let rt = build_runtime(Some(sqlite_path))?;
        let run = rt.create_run("doctor smoke").await?;
        let outcome = rt.drive(&run.id).await?;
        match outcome {
            StepOutcome::Completed => println!("runtime:      ok (mock provider completed)"),
            other => anyhow::bail!("unexpected outcome: {other:?}"),
        }
        let journal = rt.store().list_journal(&run.id).await?;
        agentos_core::verify_chain(&journal)?;
        println!(
            "journal:      ok ({} entries, hash chain valid)",
            journal.len()
        );
    }

    println!("policy:       policy-before-effect enabled");
    println!("hitl:         approval gates available");
    println!("status:       healthy");
    Ok(())
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}
