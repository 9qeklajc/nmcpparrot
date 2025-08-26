//! CLI utility tool for one-on-one private messaging on Nostr for CLI and agent use
//!
//! It uses the `nostr_sdk` crate to interact with the Nostr network. It sends and receives direct messages that are encrypted with NIP-17 by default.
mod combined_mcp;
mod goose_mcp;
mod mcp;
mod multi_agent;
mod nostr_mcp;
mod process_management;
mod profile;
mod response_tracker;
mod searxng_mcp;
mod utils;

use clap::{Parser, Subcommand};
use combined_mcp::CombinedServer;
use dotenv::dotenv;
use goose_mcp::GooseServer;
use mcp::{chat::Chat, EnhancedMcpServer};
use multi_agent::MultiAgentMcp;
use nostr_mcp::NostrMemoryServer;
use nostr_sdk::prelude::*;
use rmcp::{transport::stdio, ServiceExt};
use std::sync::Arc;
use std::{
    io::{self, Read},
    process::exit,
};
use tokio::sync::Mutex;
use utils::listen_for_messages;
use utils::run_command_on_message;
use utils::wait_for_message;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Pubkey of the target user to talk to via DMs (in bech32 format)
    #[arg(long, env = "TARGET_PUBKEY")]
    target_pubkey: String,

    /// The private key (nsec) identity to use on the DMs
    #[arg(long, env = "NSEC")]
    nsec: String,

    /// Optional private key (nsec) identity to use for progress/debug DMs
    #[arg(long, env = "PROGRESS_NSEC")]
    progress_nsec: Option<String>,

    /// Relay URL to use for sending/receiving messages
    #[arg(long, env = "RELAY_URL", default_value = "wss://relay.damus.io")]
    relay: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Sends a private message via NIP-17. If the message is omitted, reads it from stdin.
    Send {
        /// The message to send
        message: Option<String>,
    },
    /// Sends a private message via NIP-17 using the progress identity. If the message is omitted, reads it from stdin.
    SendProgress {
        /// The message to send
        message: Option<String>,
    },
    /// Waits for a private NIP-17 message to be received and prints the decrypted contents to stdout once received.
    Wait,
    /// Listens for private NIP-17 messages to be received and prints the decrypted contents to stdout after each one is received.
    Listen,
    /// Starts an MCP server to allow an AI agent to manage the conversation
    Mcp,
    /// Starts an MCP server to provide Goose AI agent command execution capabilities
    GooseMcp,
    /// Starts a combined MCP server with both chat and Goose command capabilities
    CombinedMcp,
    /// Starts an enhanced MCP server with chat, notes, and events management
    EnhancedMcp,
    /// Starts a multi-agent MCP server that can run multiple agents in parallel
    MultiAgentMcp,
    /// Starts a Nostr Memory MCP server for agent memory storage using encrypted DMs
    NostrMemoryMcp,
    /// Runs a specified shell command each time it receives a NIP-17 direct message, passing the decrypted message contents to it via stdin.
    Onmessage {
        #[clap(required = true)]
        shell_command: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args = Cli::parse();

    // Initialize logging based on the command
    match &args.command {
        Commands::CombinedMcp
        | Commands::GooseMcp
        | Commands::EnhancedMcp
        | Commands::MultiAgentMcp
        | Commands::NostrMemoryMcp
        | Commands::Onmessage { .. } => {
            // For MCP servers and onmessage, use file-based logging to avoid interfering with stdio
            use std::fs::OpenOptions;

            if let Ok(_log_level) = std::env::var("RUST_LOG") {
                let log_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("nparrot.log");

                if log_file.is_ok() {
                    env_logger::Builder::from_env("RUST_LOG")
                        .target(env_logger::Target::Pipe(Box::new(log_file.unwrap())))
                        .init();
                }
            }
        }
        _ => {
            // For non-MCP commands, use normal stdout logging
            env_logger::init();
        }
    }

    // Parse our keys from the provided identity (nsec)
    let keys = Keys::parse(&args.nsec)?;
    let our_pubkey = keys.public_key();

    // Parse the target public key
    let target_pk: PublicKey = args.target_pubkey.parse()?;

    // Create a client with our keys
    let client = Client::builder().signer(keys.clone()).build();

    // Optional progress client
    let progress_client = if let Some(progress_nsec) = &args.progress_nsec {
        let progress_keys = Keys::parse(progress_nsec)?;
        let c = Client::builder().signer(progress_keys).build();
        Some(c)
    } else {
        None
    };

    let relay_urls: Vec<&str> = args
        .relay
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for url in &relay_urls {
        client.add_relay(*url).await?;
    }
    client.connect().await;

    if let Some(ref c) = progress_client {
        for url in &relay_urls {
            c.add_relay(*url).await?;
        }
        c.connect().await;
    }

    // Setup profiles for The Fux Family agents
    log::info!("🔥 Setting up The Fux Family profiles...");
    if let Err(e) = profile::setup_main_client_profile(&client).await {
        log::warn!("Could not setup main profile: {}", e);
    }

    if let Some(ref progress_client) = progress_client {
        if let Err(e) = profile::setup_progress_client_profile(progress_client).await {
            log::warn!("Could not setup progress profile: {}", e);
        }
    }

    // The Fux Family is ready for action
    log::info!("💎 The Fux Family ready for action!");

    match args.command {
        Commands::Send { message } => {
            // Obtain the message from argument or via stdin
            let content = match message {
                Some(msg) => msg,
                None => {
                    let mut buffer = String::new();
                    io::stdin().read_to_string(&mut buffer)?;
                    buffer
                }
            };

            eprintln!("Sending direct message to {}...", args.target_pubkey);
            client.send_private_msg(target_pk, content, []).await?;
            eprintln!("Message sent!");
            exit(0);
        }
        Commands::SendProgress { message } => {
            let progress_client = progress_client.ok_or_else(|| {
                io::Error::other("progress identity not configured (set --progress-nsec)")
            })?;
            let content = match message {
                Some(msg) => msg,
                None => {
                    let mut buffer = String::new();
                    io::stdin().read_to_string(&mut buffer)?;
                    buffer
                }
            };

            eprintln!(
                "Sending PROGRESS direct message to {}...",
                args.target_pubkey
            );
            progress_client
                .send_private_msg(target_pk, content, [])
                .await?;
            eprintln!("Progress message sent!");
            exit(0);
        }
        Commands::Wait => {
            let message = wait_for_message(&client, &our_pubkey, &target_pk).await?;
            println!("{}", message);
        }
        Commands::Listen => {
            let message_callback = {
                async move |message: String| {
                    println!("{}", message);
                    false // Never returns
                }
            };

            listen_for_messages(
                &client,
                &our_pubkey,
                &target_pk,
                Arc::new(Mutex::new(message_callback)),
            )
            .await?;
        }
        Commands::Mcp => {
            // Create and serve our chat service
            let service = Chat::new(
                client.clone(),
                progress_client.clone(),
                our_pubkey,
                target_pk,
            )
            .serve(stdio())
            .await
            .inspect_err(|e| {
                log::error!("{e}");
            })?;
            service.waiting().await?;
            progress_client.unwrap()
                .send_private_msg(target_pk, "Task completed", [])
                .await?;
        }
        Commands::GooseMcp => {
            // Create and serve the Goose MCP server
            let service = GooseServer::new().serve(stdio()).await.inspect_err(|e| {
                log::error!("{e}");
            })?;
            service.waiting().await?;
        }
        Commands::CombinedMcp => {
            // Create and serve the combined MCP server with both chat, Goose, and SearXNG capabilities
            let searxng_url =
                std::env::var("SEARXNG_URL").unwrap_or_else(|_| "https://searx.stream".to_string());

            let server = CombinedServer::new(
                client.clone(),
                progress_client.clone(),
                our_pubkey,
                target_pk,
                searxng_url,
            );

            let service = server.serve(stdio()).await.inspect_err(|e| {
                log::error!("Failed to start MCP server: {}", e);
            })?;

            service.waiting().await?;
        }
        Commands::EnhancedMcp => {
            // Create and serve the enhanced MCP server with chat, notes, and events capabilities
            let service = EnhancedMcpServer::new(
                client.clone(),
                progress_client.clone(),
                our_pubkey,
                target_pk,
                None,
            )
            .serve(stdio())
            .await
            .inspect_err(|e| {
                log::error!("{e}");
            })?;
            service.waiting().await?;
        }
        Commands::MultiAgentMcp => {
            // Create and serve the multi-agent MCP server
            let service = MultiAgentMcp::new(
                client.clone(),
                progress_client.clone(),
                keys.clone(),
                our_pubkey,
                target_pk,
            )
            .serve(stdio())
            .await
            .inspect_err(|e| {
                log::error!("{e}");
            })?;
            service.waiting().await?;
        }
        Commands::NostrMemoryMcp => {
            // Create and serve the Nostr Memory MCP server
            let service = NostrMemoryServer::new(
                client.clone(),
                progress_client.clone(),
                keys.clone(),
                our_pubkey,
                target_pk,
            )
            .serve(stdio())
            .await
            .inspect_err(|e| {
                log::error!("{e}");
            })?;
            service.waiting().await?;
        }
        Commands::Onmessage { shell_command } => {
            log::info!("Listening for messages");
            run_command_on_message(&client, &our_pubkey, &target_pk, &shell_command).await?;
        }
    }

    Ok(())
}
