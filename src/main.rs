mod commands;
#[allow(dead_code)]
mod config;
mod error;
mod services;
mod templates;
mod ui;

use clap::Parser;
use services::auth::{handle_login, handle_logout, handle_status, handle_whoami, AuthCommands};
use services::core::{
    handle_config_get, handle_config_set, handle_config_show, AccountCommands, AddCommands,
    BuildsCommands, Cli, ConfigCommands, GitHubCommands, ImageCommands, RegistryCommands,
    SekuireCommands,
};
use services::install::InstallCommands;
use services::policy::PolicyCommands;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = run(cli).await;

    if let Err(e) = result {
        match &e {
            error::CliError::PlanLimitExceeded {
                message,
                required_plan,
                upgrade_url,
                ..
            } => {
                eprintln!("Plan limit: {}", message);
                eprintln!("Required plan: {}", required_plan);
                eprintln!("Upgrade at: {}", upgrade_url);
            }
            _ => eprintln!("Error: {}", e),
        }
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), error::CliError> {
    match cli.command {
        SekuireCommands::Init {
            name,
            language,
            template,
            server,
            framework,
            hybrid,
            workspace,
            backend,
        } => {
            commands::init::execute(
                name, language, template, server, framework, hybrid, workspace, backend,
            )
            .await
        }

        SekuireCommands::Keygen { force } => commands::keygen::execute(force).await,

        SekuireCommands::Hash => commands::hash::execute().await,

        SekuireCommands::Status => handle_status().await.map_err(error::CliError::Other),

        SekuireCommands::Login { web_url, backend } => {
            commands::login::execute(&web_url, &backend).await
        }

        SekuireCommands::Logout => handle_logout().await.map_err(error::CliError::Other),

        SekuireCommands::Push {
            registry,
            workspace,
            policy,
            force,
        } => commands::push::execute(registry, workspace, policy, force).await,

        SekuireCommands::Publish {
            registry,
            repo,
            public,
            no_workflow,
            yes,
        } => commands::publish::execute(registry, repo, public, no_workflow, yes).await,

        SekuireCommands::Verify { url, id, registry } => {
            commands::verify::execute(url, id, registry).await
        }

        SekuireCommands::VerifyAgent {
            registry,
            repo_url,
            compliance,
        } => commands::verify_agent::execute(registry, repo_url, compliance).await,

        SekuireCommands::Run {
            cmd,
            workspace,
            backend,
            agent,
            install_token,
            upstream_url,
            capability,
            heartbeat_interval,
            no_heartbeat,
        } => {
            commands::run::execute(
                cmd,
                workspace,
                backend,
                agent,
                install_token,
                upstream_url,
                capability,
                heartbeat_interval,
                no_heartbeat,
            )
            .await
        }

        SekuireCommands::Auth { command } => match command {
            AuthCommands::Login { web_url, backend } => handle_login(&web_url, &backend)
                .await
                .map_err(error::CliError::Other),
            AuthCommands::Logout => handle_logout().await.map_err(error::CliError::Other),
            AuthCommands::Status => handle_status().await.map_err(error::CliError::Other),
            AuthCommands::Whoami { backend_url } => handle_whoami(&backend_url)
                .await
                .map_err(error::CliError::Other),
        },

        SekuireCommands::Config { command } => match command {
            ConfigCommands::Show => handle_config_show().await.map_err(error::CliError::Other),
            ConfigCommands::Get { key } => handle_config_get(&key)
                .await
                .map_err(error::CliError::Other),
            ConfigCommands::Set { key, value } => handle_config_set(&key, &value)
                .await
                .map_err(error::CliError::Other),
        },

        SekuireCommands::Policy { command } => match command {
            PolicyCommands::Pull {
                workspace,
                backend,
                out,
            } => {
                let policy = services::policy::fetch_active_policy(&backend, &workspace)
                    .await
                    .map_err(error::CliError::Other)?;
                let json = serde_json::to_string_pretty(&policy)?;
                tokio::fs::write(&out, json).await?;
                println!(
                    "Saved active policy {} (v{}) for workspace {} to {}",
                    policy.policy_id, policy.version, workspace, out
                );
                if policy.signature.is_some() {
                    println!("   Signature: verified");
                } else {
                    println!("   Warning: no signature provided by backend");
                }
                Ok(())
            }
            PolicyCommands::Validate { file } => {
                let policy = services::policy::load_policy_file(&file)
                    .await
                    .map_err(error::CliError::Other)?;
                services::policy::verify_active_policy(&policy).map_err(error::CliError::Other)?;
                println!(
                    "Policy {} (v{}) integrity verified (hash/signature)",
                    policy.policy_id, policy.version
                );
                Ok(())
            }
            PolicyCommands::Show { file } => {
                let policy = services::policy::load_policy_file(&file)
                    .await
                    .map_err(error::CliError::Other)?;
                println!("Policy ID:     {}", policy.policy_id);
                println!("Workspace ID:  {}", policy.workspace_id);
                println!("Version:       {}", policy.version);
                println!("Status:        {}", policy.status);
                println!("Hash:          {}", policy.hash);
                println!(
                    "Signature:     {}",
                    policy.signature.as_deref().unwrap_or("<not signed>")
                );
                println!(
                    "Signing Key:   {}",
                    policy.signing_key_id.as_deref().unwrap_or("<unknown>")
                );
                Ok(())
            }
        },

        SekuireCommands::Install { command } => match command {
            InstallCommands::Token {
                workspace,
                agent,
                backend,
                ttl_minutes,
            } => {
                let agent_id = if let Some(id) = agent {
                    id
                } else {
                    let (sekuire_id, _) = services::auth::calculate_identity_hash()
                        .await
                        .map_err(error::CliError::Other)?;
                    sekuire_id
                };
                let workspace_id = if let Some(ws) = workspace {
                    ws
                } else {
                    let auth = services::auth::load_auth_session()
                        .await
                        .map_err(|_| error::CliError::NotAuthenticated)?;
                    auth.workspace_id.ok_or_else(|| {
                        error::CliError::InvalidInput(
                            "Workspace ID required. Use --workspace or `sekuire config set workspace.id`".into(),
                        )
                    })?
                };
                let token = services::install::create_install_token(
                    &backend,
                    &workspace_id,
                    &agent_id,
                    ttl_minutes,
                )
                .await
                .map_err(error::CliError::Other)?;
                println!("Install token created");
                println!("   Workspace: {}", workspace_id);
                println!("   Agent ID:  {}", agent_id);
                println!("   Token:     {}", token.token);
                if let Some(exp) = token.expires_at {
                    println!("   Expires:   {}", exp);
                }
                Ok(())
            }
            InstallCommands::Bundle {
                workspace,
                agent,
                backend,
                output,
                image,
                install_token,
                namespace,
                service_port,
                container_port,
            } => {
                let (sekuire_id, manifest) = services::auth::calculate_identity_hash()
                    .await
                    .map_err(error::CliError::Other)?;
                let agent_id = agent.unwrap_or(sekuire_id);
                let token_value = if let Some(token) = install_token {
                    token
                } else {
                    let token = services::install::create_install_token(
                        &backend, &workspace, &agent_id, None,
                    )
                    .await
                    .map_err(error::CliError::Other)?;
                    token.token
                };
                let chart_dir = services::install::prepare_helm_bundle(
                    &output,
                    &manifest,
                    &workspace,
                    &agent_id,
                    &namespace,
                    image.as_deref(),
                    &token_value,
                    service_port,
                    container_port,
                )
                .await
                .map_err(error::CliError::Other)?;
                println!("Helm install bundle generated");
                println!("   Chart directory: {}", chart_dir.display());
                println!("   Agent ID:        {}", agent_id);
                println!("   Workspace ID:    {}", workspace);
                Ok(())
            }
            InstallCommands::Agent {
                workspace,
                agent,
                deployment_type,
                registry,
            } => {
                let auth = services::auth::load_auth_session()
                    .await
                    .map_err(|_| error::CliError::NotAuthenticated)?;
                services::install::install_agent_remote(
                    &registry,
                    &workspace,
                    &agent,
                    &deployment_type,
                    auth.api_key,
                )
                .await
                .map_err(error::CliError::Other)
            }
        },

        SekuireCommands::Test {
            suite,
            registry,
            fail_fast,
            agent_url,
        } => commands::test_cmd::execute(suite, registry, fail_fast, agent_url).await,

        SekuireCommands::Search {
            query,
            tags,
            verified,
            category,
            limit,
            format,
            registry,
        } => {
            commands::search::execute(query, tags, verified, category, limit, format, registry)
                .await
        }

        SekuireCommands::Metrics {
            agent,
            range,
            format,
            registry,
        } => commands::metrics::execute(agent, range, format, registry).await,

        SekuireCommands::Leaderboard {
            limit,
            category,
            verified_only,
            format,
            registry,
        } => commands::leaderboard::execute(limit, category, verified_only, format, registry).await,

        SekuireCommands::DockerInit {
            output,
            cloudflare,
            port,
            force,
            ci,
        } => commands::docker_init::execute(output, cloudflare, port, force, ci).await,

        SekuireCommands::Adopt { name, model } => commands::adopt::execute(name, model).await,

        SekuireCommands::Analysis { id, registry } => {
            commands::analysis::execute(registry, id).await
        }

        SekuireCommands::Builds { command } => match command {
            BuildsCommands::Create {
                agent,
                tag,
                registry,
            } => commands::builds::execute_create(registry, agent, tag).await,
            BuildsCommands::Status {
                build_id,
                agent,
                registry,
            } => commands::builds::execute_status(registry, build_id, agent).await,
            BuildsCommands::List {
                agent,
                status,
                format,
                registry,
            } => commands::builds::execute_list(registry, agent, status, format).await,
        },

        SekuireCommands::Logs {
            agent,
            limit,
            registry,
        } => commands::logs::execute(agent, limit, registry).await,

        SekuireCommands::AgentStatus { agent, registry } => {
            commands::status_cmd::execute(agent, registry).await
        }

        SekuireCommands::Stop {
            agent,
            reason,
            registry,
        } => {
            commands::lifecycle::execute(
                agent,
                commands::lifecycle::LifecycleAction::Pause,
                reason,
                registry,
            )
            .await
        }

        SekuireCommands::Resume { agent, registry } => {
            commands::lifecycle::execute(
                agent,
                commands::lifecycle::LifecycleAction::Resume,
                None,
                registry,
            )
            .await
        }

        SekuireCommands::Kill {
            agent,
            reason,
            registry,
        } => {
            commands::lifecycle::execute(
                agent,
                commands::lifecycle::LifecycleAction::Terminate,
                reason,
                registry,
            )
            .await
        }

        SekuireCommands::Registry { command } => match command {
            RegistryCommands::Login { registry, backend } => {
                services::registry::handle_registry_login(&registry, &backend)
                    .await
                    .map_err(error::CliError::Other)
            }
            RegistryCommands::Push {
                image,
                registry,
                backend,
            } => services::registry::handle_registry_push(&image, &registry, &backend)
                .await
                .map_err(error::CliError::Other),
            RegistryCommands::Pull {
                image,
                registry,
                backend,
            } => services::registry::handle_registry_pull(&image, &registry, &backend)
                .await
                .map_err(error::CliError::Other),
        },

        SekuireCommands::Images { command } => match command {
            ImageCommands::List {
                agent,
                backend,
                format,
            } => services::registry::handle_images_list(agent.as_deref(), &backend, &format)
                .await
                .map_err(error::CliError::Other),
            ImageCommands::Scan { image_id, backend } => {
                services::registry::handle_images_scan(&image_id, &backend)
                    .await
                    .map_err(error::CliError::Other)
            }
            ImageCommands::ScanReport {
                image_id,
                backend,
                format,
            } => services::registry::handle_images_scan_report(&image_id, &backend, &format)
                .await
                .map_err(error::CliError::Other),
        },

        SekuireCommands::Github { command } => match command {
            GitHubCommands::Install { backend } => {
                services::github::handle_github_install(&backend)
                    .await
                    .map_err(error::CliError::Other)
            }
            GitHubCommands::Installations { backend } => {
                services::github::handle_github_installations(&backend)
                    .await
                    .map_err(error::CliError::Other)
            }
            GitHubCommands::Connect {
                repo,
                agent,
                backend,
            } => services::github::handle_github_connect(&repo, agent.as_deref(), &backend)
                .await
                .map_err(error::CliError::Other),
        },

        SekuireCommands::Add { command } => match command {
            AddCommands::Memory => commands::add::execute_memory().await,
        },

        SekuireCommands::Account { command } => match command {
            AccountCommands::Limits { registry } => {
                commands::account::execute_limits(registry).await
            }
        },
    }
}
