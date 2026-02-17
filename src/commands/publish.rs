use crate::config;
use crate::error::{truncate_hash, CliError};
use crate::services::http_client::SekuireClient;
use crate::ui;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::path::Path;
use tokio::fs;

pub async fn execute(
    registry: String,
    repo: Option<String>,
    public: bool,
    no_workflow: bool,
    yes: bool,
) -> Result<(), CliError> {
    println!("\nSekuire Agent Publisher");
    println!("------------------------\n");

    let manifest_path = Path::new(config::MANIFEST_YAML);
    if !manifest_path.exists() {
        return Err(CliError::ManifestNotFound);
    }

    let content = fs::read_to_string(manifest_path).await?;
    let manifest: serde_json::Value =
        serde_yaml::from_str(&content).map_err(|e| CliError::ManifestParse(e.to_string()))?;
    let agent_name = manifest["name"].as_str().unwrap_or("Unknown Agent");

    let model_name = manifest["model"].as_str().unwrap_or("");
    let system_prompt = manifest["system_prompt"].as_str().unwrap_or("");
    let tools_json = serde_json::to_string(&manifest["tools"]).unwrap_or_default();
    let mut hasher = blake3::Hasher::new();
    hasher.update(model_name.to_lowercase().trim().as_bytes());
    hasher.update(blake3::hash(system_prompt.as_bytes()).as_bytes());
    hasher.update(blake3::hash(tools_json.as_bytes()).as_bytes());
    let sekuire_id = truncate_hash(&hasher.finalize().to_hex(), 16).to_string();

    println!("Agent: {} ({})", agent_name, truncate_hash(&sekuire_id, 8));

    let client = SekuireClient::from_registry(&registry).await?;

    if public {
        publish_public(&client, &sekuire_id, repo, yes).await
    } else {
        publish_private(&client, &sekuire_id, agent_name, repo, no_workflow, yes).await
    }
}

async fn publish_public(
    client: &SekuireClient,
    sekuire_id: &str,
    repo: Option<String>,
    yes: bool,
) -> Result<(), CliError> {
    println!("\nPublishing to PUBLIC registry (Sekuire-verified)");
    println!("  - Sekuire will build from source");
    println!("  - SDK compliance required");
    println!("  - Security scans required");
    println!("  - Anyone can pull");

    let repo_url = match repo {
        Some(r) => r,
        None => {
            return Err(CliError::InvalidInput(
                "--repo is required for public registry. Sekuire needs source access to build."
                    .into(),
            ));
        }
    };

    println!("\nStep 1: GitHub App");
    let app_url_resp =
        ui::with_spinner_if(ui::spinner_enabled(None), "Checking GitHub App...", async {
            client.get("/api/v1/github/app-url").await
        })
        .await;
    let app_url = match app_url_resp {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().await.unwrap_or_default();
            data["install_url"].as_str().unwrap_or("").to_string()
        }
        _ => String::new(),
    };

    let installations_resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching GitHub installations...",
        async { client.get("/api/v1/github/installations").await },
    )
    .await;
    let has_installation = match installations_resp {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().await.unwrap_or_default();
            data["installations"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
        }
        _ => false,
    };

    if !has_installation {
        println!("   GitHub App not installed.");
        if !yes && !app_url.is_empty() {
            let install = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Install Sekuire GitHub App?")
                .items(&["Yes - Open browser", "No - Cancel"])
                .default(0)
                .interact()
                .map_err(|e| CliError::Other(e.into()))?;

            if install == 0 {
                println!("   Opening browser...");
                let _ = webbrowser::open(&app_url);
                println!("   After installing, press Enter to continue.");
                let _: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Press Enter when ready")
                    .allow_empty(true)
                    .interact_text()
                    .map_err(|e| CliError::Other(e.into()))?;
            } else {
                return Err(CliError::InvalidInput(
                    "GitHub App required for public registry".into(),
                ));
            }
        } else {
            return Err(CliError::InvalidInput(format!(
                "GitHub App required. Install at: {}",
                app_url
            )));
        }
    } else {
        println!("   GitHub App installed");
    }

    println!("\nStep 2: Connect Repository");
    let connect_resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Connecting repository...",
        async {
            client
                .post_checked(
                    "/api/v1/github/connect-repo",
                    &serde_json::json!({
                        "sekuire_id": sekuire_id,
                        "repo_url": repo_url,
                    }),
                )
                .await
        },
    )
    .await;

    match connect_resp {
        Ok(resp) if resp.status().is_success() => {
            println!("   Repository connected: {}", repo_url);
        }
        Ok(resp) => {
            let error = resp.text().await.unwrap_or_default();
            return Err(CliError::RegistryError {
                status: 400,
                body: format!("Failed to connect repo: {}", error),
            });
        }
        Err(e) => return Err(e),
    }

    println!("\nStep 3: Trigger Build");
    let build_resp = ui::with_spinner_if(ui::spinner_enabled(None), "Triggering build...", async {
        client
            .post_checked(
                &format!("/api/v1/agents/{}/builds", sekuire_id),
                &serde_json::json!({"tag": "latest", "public": true}),
            )
            .await
    })
    .await;

    match build_resp {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().await.unwrap_or_default();
            let build_id = data["id"].as_str().unwrap_or("unknown");
            println!("   Build queued: {}", build_id);
            println!("\nBuild in progress. Sekuire will:");
            println!("   1. Clone repository");
            println!("   2. Analyze source (SDK compliance, security)");
            println!("   3. Build container image");
            println!("   4. Push to public registry");
            println!("\nTrack status:");
            println!("   sekuire builds status {}", build_id);
            println!("\nOnce complete, anyone can pull:");
            println!("   docker pull registry.sekuire.ai/{}:latest", sekuire_id);
        }
        Ok(resp) => {
            let error = resp.text().await.unwrap_or_default();
            println!("   Build endpoint not available: {}", error);
            println!("\n   Public builds are coming soon.");
            println!("   For now, use --private to publish to your organization.");
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

async fn publish_private(
    client: &SekuireClient,
    sekuire_id: &str,
    agent_name: &str,
    repo: Option<String>,
    no_workflow: bool,
    yes: bool,
) -> Result<(), CliError> {
    println!("\nPublishing to PRIVATE registry (org-managed)");
    println!("  - You build and push images");
    println!("  - Optional source analysis for visibility");
    println!("  - Org members only can pull");

    let repo_url = if let Some(r) = repo.clone() {
        r
    } else if yes {
        detect_git_remote()?
    } else {
        prompt_for_repo()?
    };
    println!("\nRepository: {}", repo_url);

    println!("\nStep 1: Register Agent");
    let register_resp =
        ui::with_spinner_if(ui::spinner_enabled(None), "Registering agent...", async {
            client
                .post_checked(
                    "/api/v1/publish",
                    &serde_json::json!({
                        "sekuire_id": sekuire_id,
                        "name": agent_name,
                        "repository_url": repo_url,
                        "visibility": "private",
                    }),
                )
                .await
        })
        .await;

    match register_resp {
        Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 409 => {
            println!("   Agent registered");
        }
        Ok(resp) => {
            let status = resp.status().as_u16();
            let error = resp.text().await.unwrap_or_default();
            return Err(CliError::RegistryError {
                status,
                body: error,
            });
        }
        Err(e) => return Err(e),
    }

    if repo.is_some() {
        println!("\nStep 2: Source Analysis (opt-in)");
        let connect_resp = ui::with_spinner_if(
            ui::spinner_enabled(None),
            "Connecting repository...",
            async {
                client
                    .post(
                        "/api/v1/github/connect-repo",
                        &serde_json::json!({
                            "sekuire_id": sekuire_id,
                            "repo_url": repo_url,
                        }),
                    )
                    .await
            },
        )
        .await;

        match connect_resp {
            Ok(resp) if resp.status().is_success() => {
                let data: serde_json::Value = resp.json().await.unwrap_or_default();
                if data["analysis_triggered"].as_bool().unwrap_or(false) {
                    println!("   Repository connected");
                    println!("   Analysis triggered (runs in background)");
                    println!("   View results: sekuire analysis status");
                } else {
                    println!("   Repository connected (no GitHub App)");
                    println!("   Install GitHub App for source analysis:");
                    let app_url_resp = ui::with_spinner_if(
                        ui::spinner_enabled(None),
                        "Checking GitHub App...",
                        async { client.get("/api/v1/github/app-url").await },
                    )
                    .await;
                    if let Ok(resp) = app_url_resp {
                        if resp.status().is_success() {
                            let data: serde_json::Value = resp.json().await.unwrap_or_default();
                            if let Some(url) = data["install_url"].as_str() {
                                println!("   {}", url);
                            }
                        }
                    }
                }
            }
            Ok(_) | Err(_) => {
                println!("   Skipping source analysis (optional)");
            }
        }
    } else {
        println!("\nStep 2: Source Analysis");
        println!("   Skipped (no --repo provided)");
        println!("   Add --repo to enable visibility into agent capabilities");
    }

    if !no_workflow {
        println!("\nStep 3: Build Workflow");
        if yes {
            println!("   Use 'sekuire workflow' to create GitHub Actions workflow");
        } else {
            let create = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Create GitHub Actions workflow?")
                .items(&["No - I'll build manually", "Yes - Create workflow"])
                .default(0)
                .interact()
                .map_err(|e| CliError::Other(e.into()))?;

            if create == 1 {
                println!("   Run: sekuire workflow");
            }
        }
    }

    let org_slug = get_org_slug(client)
        .await
        .unwrap_or_else(|| "your-org".to_string());

    println!("\n-----------------------------");
    println!("Agent Published Successfully!");
    println!("-----------------------------");
    println!("\nAgent Details:");
    println!("   Name:       {}", agent_name);
    println!("   ID:         {}", sekuire_id);
    println!("   Registry:   Private (org-managed)");
    println!("   Repository: {}", repo_url);

    println!("\nNext Steps - Build and Push:");
    println!(
        "   docker build -t registry.sekuire.ai/org/{}/{} .",
        org_slug,
        agent_name.to_lowercase().replace(' ', "-")
    );
    println!("   sekuire registry login");
    println!(
        "   docker push registry.sekuire.ai/org/{}/{}",
        org_slug,
        agent_name.to_lowercase().replace(' ', "-")
    );

    println!("\nEnvironment Variables for Deployment:");
    println!("   SEKUIRE_AGENT_ID={}", sekuire_id);
    println!("   SEKUIRE_PRIVATE_KEY=<from .sekuire/secret.key>");

    Ok(())
}

fn detect_git_remote() -> Result<String, CliError> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output();

    match output {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).trim().to_string()),
        _ => Err(CliError::InvalidInput(
            "Could not detect git remote. Please provide --repo.".into(),
        )),
    }
}

fn prompt_for_repo() -> Result<String, CliError> {
    let detected = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let default = detected.unwrap_or_default();
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt("GitHub repo URL")
        .default(default)
        .interact_text()
        .map_err(|e| CliError::Other(e.into()))
}

async fn get_org_slug(client: &SekuireClient) -> Option<String> {
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching org details...",
        async { client.get("/api/v1/me").await },
    )
    .await
    .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: serde_json::Value = resp.json().await.ok()?;
    data["organization"]["slug"].as_str().map(|s| s.to_string())
}
