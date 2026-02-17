use crate::services::{
    auth::{AuthCommands, AuthConfig},
    device,
    install::InstallCommands,
    policy::PolicyCommands,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sekuire_protocol::Manifest;
use tokio::fs;

#[derive(Parser)]
#[command(name = "sekuire")]
#[command(about = "The Trust Protocol for AI Agents", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: SekuireCommands,
}

#[derive(Subcommand)]
pub enum SekuireCommands {
    /// Initialize a new Sekuire Identity in the current directory [Local]
    Init {
        /// The name of the project
        #[arg(short, long)]
        name: Option<String>,

        /// The language template to use (python, rust, typescript)
        #[arg(short, long)]
        language: Option<String>,

        /// The project template (simple, multi-llm, react, rag, http, hybrid)
        #[arg(short, long)]
        template: Option<String>,

        /// Enable HTTP server mode (shorthand for --template http)
        #[arg(long)]
        server: bool,

        /// HTTP framework to use (express, fastify, hono, fastapi, django, axum, actix)
        #[arg(long)]
        framework: Option<String>,

        /// Create hybrid agent with both CLI and HTTP modes
        #[arg(long)]
        hybrid: bool,

        /// Workspace ID to pull active policy for
        #[arg(long)]
        workspace: Option<String>,

        /// Backend URL for policy/template retrieval
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// Generate a new Keypair for signing agents [Local]
    Keygen {
        /// Force overwrite if keys exist
        #[arg(long)]
        force: bool,
    },
    /// Calculate the deterministic Sekuire ID (Fingerprint) [Local]
    Hash,
    /// Check current authentication status
    Status,
    /// Login to the Sekuire Registry with API Key
    Login {
        /// The web app URL
        #[arg(long, default_value = "https://sekuire.ai")]
        web_url: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// Logout (remove credentials)
    Logout,
    /// Push the Agent to the Registry (Draft) [Starter]
    Push {
        /// The registry URL
        #[arg(short, long, default_value = "https://api.sekuire.ai")]
        registry: String,

        /// Workspace ID to validate against active policy
        #[arg(long)]
        workspace: Option<String>,

        /// Path to local policy file
        #[arg(long, default_value = "policy.json")]
        policy: String,

        /// Force push even if local policy validation fails
        #[arg(long)]
        force: bool,
    },
    /// Publish a draft agent with interactive setup (repo, visibility, workflow) [Free Cloud]
    Publish {
        /// The registry URL
        #[arg(short, long, default_value = "https://api.sekuire.ai")]
        registry: String,

        /// GitHub repository URL (owner/repo or full URL)
        #[arg(long)]
        repo: Option<String>,

        /// Make agent public in the registry
        #[arg(long)]
        public: bool,

        /// Skip GitHub Actions workflow creation
        #[arg(long)]
        no_workflow: bool,

        /// Skip interactive prompts (use defaults)
        #[arg(long)]
        yes: bool,
    },
    /// Verify a remote agent's identity via Handshake
    Verify {
        /// The URL of the agent (e.g., http://localhost:8000)
        #[arg(short, long)]
        url: String,
        /// The expected Agent ID (optional)
        #[arg(short, long)]
        id: Option<String>,
        /// The Registry URL
        #[arg(short, long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Perform comprehensive security verification ("The Bar Exam")
    VerifyAgent {
        /// The registry URL
        #[arg(short, long, default_value = "https://api.sekuire.ai")]
        registry: String,
        /// The repository URL (defaults to current git remote)
        #[arg(long)]
        repo_url: Option<String>,
        /// Compliance framework to validate (hipaa, soc2, gdpr, pci-dss)
        #[arg(long)]
        compliance: Option<String>,
    },
    /// Run an agent locally with automatic install + bootstrap [Starter]
    Run {
        /// Command to run the agent (e.g., "npm start", "python main.py")
        #[arg(long, short)]
        cmd: Option<String>,
        /// Workspace ID (defaults to config workspace.id)
        #[arg(long)]
        workspace: Option<String>,
        /// Backend base URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
        /// Sekuire agent ID (defaults to local manifest hash)
        #[arg(long)]
        agent: Option<String>,
        /// Optional install token (skip token minting)
        #[arg(long)]
        install_token: Option<String>,
        /// Override upstream URL (defaults to manifest a2a.upstream_url)
        #[arg(long)]
        upstream_url: Option<String>,
        /// Capabilities to register (defaults to manifest capabilities.provides)
        #[arg(long)]
        capability: Vec<String>,
        /// Heartbeat interval seconds (default 60)
        #[arg(long, default_value_t = 60)]
        heartbeat_interval: i64,
        /// Disable automatic heartbeats
        #[arg(long)]
        no_heartbeat: bool,
    },
    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Configuration commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Workspace policy operations [Starter]
    Policy {
        #[command(subcommand)]
        command: PolicyCommands,
    },
    /// Installation helpers (tokens, bundles)
    Install {
        #[command(subcommand)]
        command: InstallCommands,
    },
    /// Test agent locally before publishing
    Test {
        /// Test suite to run (all, handshake, policy, endpoints, performance)
        #[arg(long, default_value = "all")]
        suite: String,
        /// Registry URL for verification checks
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
        /// Stop on first failure
        #[arg(long)]
        fail_fast: bool,
        /// Agent URL (for endpoint tests)
        #[arg(long, default_value = "http://localhost:8000")]
        agent_url: String,
    },
    /// Search for agents in the registry
    Search {
        /// Search query
        query: String,
        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
        /// Filter by verification status
        #[arg(long)]
        verified: Option<bool>,
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Number of results to show
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// View agent metrics and reputation
    Metrics {
        /// Agent ID (defaults to current project's sekuire ID)
        #[arg(long)]
        agent: Option<String>,
        /// Time range for metrics (1h, 24h, 7d, 30d)
        #[arg(long, default_value = "24h")]
        range: String,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// View agent leaderboard
    Leaderboard {
        /// Number of agents to show
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Only show verified agents
        #[arg(long)]
        verified_only: bool,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Container registry operations
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },
    /// Container image management
    Images {
        #[command(subcommand)]
        command: ImageCommands,
    },
    /// GitHub App management for source analysis
    Github {
        #[command(subcommand)]
        command: GitHubCommands,
    },
    /// Generate Docker deployment files for an existing agent
    DockerInit {
        /// Output directory (default: current directory)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Include Cloudflare Container configuration
        #[arg(long)]
        cloudflare: bool,
        /// Port to expose (default: 8000)
        #[arg(short, long, default_value = "8000")]
        port: u16,
        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
        /// Generate GitHub Actions CI/CD workflow for registry push
        #[arg(long)]
        ci: bool,
    },
    /// Adopt an existing agent project (non-destructive onboarding)
    Adopt {
        /// Agent name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
        /// Default model to configure
        #[arg(short, long)]
        model: Option<String>,
    },
    /// View source analysis results for an agent
    Analysis {
        /// Agent ID (defaults to current project's sekuire ID)
        #[arg(long)]
        id: Option<String>,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Manage container image builds (public registry) [Enterprise]
    Builds {
        #[command(subcommand)]
        command: BuildsCommands,
    },
    /// View recent logs for an agent [Starter]
    Logs {
        /// Agent ID (Sekuire ID)
        agent: String,
        /// Number of log entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Show runtime status for an agent
    AgentStatus {
        /// Agent ID (Sekuire ID)
        agent: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Pause an agent (stop accepting new tasks) [Starter]
    Stop {
        /// Agent ID (Sekuire ID)
        agent: String,
        /// Reason for pausing
        #[arg(long)]
        reason: Option<String>,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Resume a paused agent [Starter]
    Resume {
        /// Agent ID (Sekuire ID)
        agent: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Terminate an agent (irreversible) [Starter]
    Kill {
        /// Agent ID (Sekuire ID)
        agent: String,
        /// Reason for termination
        #[arg(long)]
        reason: Option<String>,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Add components to an existing project
    Add {
        #[command(subcommand)]
        command: AddCommands,
    },
    /// Account management
    Account {
        #[command(subcommand)]
        command: AccountCommands,
    },
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Show current plan, usage limits, and feature flags
    Limits {
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
}

#[derive(Subcommand)]
pub enum AddCommands {
    /// Add memory storage backend to your agent
    Memory,
}

#[derive(Subcommand)]
pub enum GitHubCommands {
    /// Install the Sekuire GitHub App (opens browser)
    Install {
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// List GitHub App installations for your organization
    Installations {
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// Connect a repository to an agent for source analysis
    Connect {
        /// Repository URL (e.g., https://github.com/org/repo)
        repo: String,
        /// Agent sekuire_id (defaults to current project)
        #[arg(long)]
        agent: Option<String>,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
}

#[derive(Subcommand)]
pub enum RegistryCommands {
    /// Login to the container registry
    Login {
        /// Registry URL
        #[arg(long, default_value = "https://registry.sekuire.ai")]
        registry: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// Push an image to the registry (wraps docker push with auth)
    Push {
        /// Image to push (e.g., my-agent:latest)
        image: String,
        /// Registry URL
        #[arg(long, default_value = "https://registry.sekuire.ai")]
        registry: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// Pull an image from the registry (handles auth automatically)
    Pull {
        /// Image to pull (e.g., org/myorg/agent:v1 or sekuire_id:tag)
        image: String,
        /// Registry URL
        #[arg(long, default_value = "registry.sekuire.ai")]
        registry: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
}

#[derive(Subcommand)]
pub enum ImageCommands {
    /// List images for an agent
    List {
        /// Agent sekuire_id (defaults to current project)
        #[arg(long)]
        agent: Option<String>,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Trigger a vulnerability scan for an image
    Scan {
        /// Image ID to scan
        image_id: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// View scan results for an image
    ScanReport {
        /// Image ID to view results for
        image_id: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum BuildsCommands {
    /// Trigger a new build for an agent
    Create {
        /// Agent ID (defaults to current project's sekuire ID)
        #[arg(long)]
        agent: Option<String>,
        /// Tag for the build (e.g., v1.0.0, latest)
        #[arg(long, default_value = "latest")]
        tag: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// Check the status of a specific build
    Status {
        /// Build ID
        build_id: String,
        /// Agent ID (defaults to current project's sekuire ID)
        #[arg(long)]
        agent: Option<String>,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
    /// List builds for an agent
    List {
        /// Agent ID (defaults to current project's sekuire ID)
        #[arg(long)]
        agent: Option<String>,
        /// Filter by status (pending, building, completed, failed, blocked)
        #[arg(long)]
        status: Option<String>,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
        /// Registry URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
}

// Configuration handler functions
pub async fn handle_config_show() -> Result<()> {
    println!("⚙️  Sekuire Configuration:");

    let home = dirs::home_dir().context("Could not find home directory")?;
    let config_dir = home.join(".sekuire");

    // Show auth config
    let auth_path = config_dir.join("auth.json");
    if auth_path.exists() {
        println!("\n   📁 Authentication Config:");
        println!("   Location: {}", auth_path.display());

        let content = fs::read_to_string(&auth_path).await?;
        let config: AuthConfig = serde_json::from_str(&content)?;

        if config.api_key.is_some() {
            println!("   Auth Method: API Key");
        } else if config.token.is_some() {
            println!("   Auth Method: Token (legacy)");
        }
        if let Some(workspace_id) = config.workspace_id {
            println!("   Default Workspace: {}", workspace_id);
        }
    } else {
        println!("\n   ❌ No authentication configured");
    }

    // Show project config if in a project directory
    if std::path::Path::new("sekuire.yml").exists() {
        println!("\n   📁 Project Config:");
        println!("   Location: ./sekuire.yml");

        let content = fs::read_to_string("sekuire.yml").await?;
        let manifest: Manifest = serde_yaml::from_str(&content)?;

        println!("   Name: {}", manifest.project.name);
        println!("   Version: {}", manifest.project.version);
        println!("   Model: {}", manifest.identity.model);
    } else if std::path::Path::new("sekuire.json").exists() {
        println!("\n   📁 Project Config:");
        println!("   Location: ./sekuire.json");

        let content = fs::read_to_string("sekuire.json").await?;
        let manifest: Manifest = serde_json::from_str(&content)?;

        println!("   Name: {}", manifest.project.name);
        println!("   Version: {}", manifest.project.version);
        println!("   Model: {}", manifest.identity.model);
    }

    println!();
    Ok(())
}

pub async fn handle_config_get(key: &str) -> Result<()> {
    match key {
        "auth.method" => {
            let home = dirs::home_dir().context("Could not find home directory")?;
            let auth_path = home.join(".sekuire/auth.json");

            if auth_path.exists() {
                let content = fs::read_to_string(auth_path).await?;
                let config: AuthConfig = serde_json::from_str(&content)?;

                if config.api_key.is_some() {
                    println!("api-key");
                } else if config.token.is_some() {
                    println!("token");
                } else {
                    println!("none");
                }
            } else {
                println!("none");
            }
        }
        "device.fingerprint" => {
            println!("{}", device::get_device_fingerprint());
        }
        "device.name" => {
            println!("{}", device::get_device_name());
        }
        "device.os" => {
            println!("{}", device::get_device_os());
        }
        "workspace.id" | "workspace_id" => {
            let home = dirs::home_dir().context("Could not find home directory")?;
            let auth_path = home.join(".sekuire/auth.json");
            if auth_path.exists() {
                let content = fs::read_to_string(auth_path).await?;
                let config: AuthConfig = serde_json::from_str(&content)?;
                if let Some(id) = config.workspace_id {
                    println!("{}", id);
                } else {
                    println!();
                }
            } else {
                println!();
            }
        }
        _ => {
            eprintln!("❌ Unknown config key: {}", key);
            eprintln!("   Available keys:");
            eprintln!("   - auth.method");
            eprintln!("   - device.fingerprint");
            eprintln!("   - device.name");
            eprintln!("   - device.os");
            eprintln!("   - workspace.id");
        }
    }

    Ok(())
}

pub async fn handle_config_set(key: &str, value: &str) -> Result<()> {
    match key {
        "workspace.id" | "workspace_id" => {
            let home = dirs::home_dir().context("Could not find home directory")?;
            let config_dir = home.join(".sekuire");
            let auth_path = config_dir.join("auth.json");

            if !config_dir.exists() {
                fs::create_dir_all(&config_dir).await?;
            }

            let mut config = if auth_path.exists() {
                let content = fs::read_to_string(&auth_path).await?;
                serde_json::from_str::<AuthConfig>(&content).unwrap_or(AuthConfig {
                    api_key: None,
                    token: None,
                    device_fingerprint: None,
                    org_id: None,
                    workspace_id: None,
                })
            } else {
                AuthConfig {
                    api_key: None,
                    token: None,
                    device_fingerprint: None,
                    org_id: None,
                    workspace_id: None,
                }
            };

            let trimmed = value.trim();
            config.workspace_id = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };

            fs::write(&auth_path, serde_json::to_string_pretty(&config)?).await?;
            println!(
                "✅ Default workspace set to {}",
                config.workspace_id.unwrap_or_else(|| "<none>".into())
            );
            return Ok(());
        }
        _ => {}
    }

    eprintln!("❌ Config key '{}' is read-only or not supported", key);
    eprintln!("   Most configuration is managed automatically.");
    eprintln!("   Use 'sekuire login' to update authentication.");

    Ok(())
}
