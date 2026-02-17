use crate::config;
use crate::error::{truncate_hash, CliError};
use crate::services::http_client::SekuireClient;
use crate::ui;
use std::path::Path;
use tokio::fs;

pub async fn execute_create(
    registry: String,
    sekuire_id: Option<String>,
    tag: String,
) -> Result<(), CliError> {
    let agent_id = match sekuire_id {
        Some(id) => id,
        None => get_sekuire_id_from_manifest().await?,
    };

    println!("\nTriggering Build");
    println!("----------------");
    println!("Agent: {}", truncate_hash(&agent_id, 8));
    println!("Tag: {}\n", tag);

    let client = SekuireClient::from_registry(&registry).await?;

    let resp = ui::with_spinner_if(ui::spinner_enabled(None), "Triggering build...", async {
        client
            .post(
                &format!("/api/v1/agents/{}/builds", agent_id),
                &serde_json::json!({ "tag": tag }),
            )
            .await
    })
    .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().await.unwrap_or_default();
            let build_id = data["id"].as_str().unwrap_or("unknown");

            println!("Build triggered successfully!");
            println!("Build ID: {}", build_id);
            println!("\nTrack progress:");
            println!("  sekuire builds status {}", build_id);
        }
        Ok(resp) if resp.status().as_u16() == 400 => {
            let error: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = error["error"].as_str().unwrap_or("Bad request");
            println!("Failed to trigger build: {}", msg);
            if msg.contains("repo") {
                println!("\nConnect a repository first:");
                println!("  sekuire github connect <repo-url>");
            }
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

    Ok(())
}

pub async fn execute_status(
    registry: String,
    build_id: String,
    sekuire_id: Option<String>,
) -> Result<(), CliError> {
    let agent_id = match sekuire_id {
        Some(id) => id,
        None => get_sekuire_id_from_manifest().await?,
    };

    println!("\nBuild Status");
    println!("------------");
    println!("Agent: {}", truncate_hash(&agent_id, 8));
    println!("Build: {}\n", truncate_hash(&build_id, 8));

    let client = SekuireClient::from_registry(&registry).await?;

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching build status...",
        async {
            client
                .get(&format!("/api/v1/agents/{}/builds/{}", agent_id, build_id))
                .await
        },
    )
    .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let build: serde_json::Value = resp.json().await.unwrap_or_default();
            print_build_detail(&build);
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            println!("Build not found.");
            println!("\nTo list builds: sekuire builds list");
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

    Ok(())
}

pub async fn execute_list(
    registry: String,
    sekuire_id: Option<String>,
    status_filter: Option<String>,
    format: String,
) -> Result<(), CliError> {
    let agent_id = match sekuire_id {
        Some(id) => id,
        None => get_sekuire_id_from_manifest().await?,
    };

    let client = SekuireClient::from_registry(&registry).await?;

    let url = match &status_filter {
        Some(status) => format!("/api/v1/agents/{}/builds?status={}", agent_id, status),
        None => format!("/api/v1/agents/{}/builds", agent_id),
    };

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(Some(&format)),
        "Fetching build list...",
        async { client.get(&url).await },
    )
    .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().await.unwrap_or_default();
            let builds = data["builds"].as_array();

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
                return Ok(());
            }

            println!("\nBuilds for {}", truncate_hash(&agent_id, 8));
            println!("{}", "-".repeat(60));

            match builds {
                Some(arr) if !arr.is_empty() => {
                    println!(
                        "{:<12} {:<12} {:<10} {:<20}",
                        "BUILD ID", "STATUS", "TAG", "CREATED"
                    );
                    println!("{}", "-".repeat(60));

                    for build in arr {
                        let id = build["id"].as_str().unwrap_or("");
                        let status = build["status"].as_str().unwrap_or("unknown");
                        let tag = build["tag"].as_str().unwrap_or("-");
                        let created = build["created_at"].as_str().unwrap_or("");

                        println!(
                            "{:<12} {:<12} {:<10} {:<20}",
                            truncate_hash(id, 8),
                            format_status(status),
                            truncate_string(tag, 10),
                            format_timestamp(created),
                        );
                    }
                }
                _ => {
                    println!("No builds found.");
                    println!("\nTo trigger a build:");
                    println!("  sekuire publish --public --repo <github-url>");
                }
            }
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

    Ok(())
}

fn print_build_detail(build: &serde_json::Value) {
    let status = build["status"].as_str().unwrap_or("unknown");
    let tag = build["tag"].as_str().unwrap_or("");
    let commit = build["commit_sha"].as_str().unwrap_or("");
    let repo = build["repo_url"].as_str().unwrap_or("");
    let created = build["created_at"].as_str().unwrap_or("");
    let started = build["started_at"].as_str().unwrap_or("");
    let completed = build["completed_at"].as_str().unwrap_or("");

    println!("Status: {}", format_status_detailed(status));
    println!("Tag: {}", tag);

    if !repo.is_empty() {
        println!("Repository: {}", repo);
    }
    if !commit.is_empty() {
        println!("Commit: {}", truncate_hash(commit, 8));
    }

    println!("\n--- Timeline ---");
    println!("Created: {}", format_timestamp(created));
    if !started.is_empty() {
        println!("Started: {}", format_timestamp(started));
    }
    if !completed.is_empty() {
        println!("Completed: {}", format_timestamp(completed));
    }

    let sdk_passed = build["sdk_passed"].as_bool();
    let security_passed = build["security_passed"].as_bool();

    if sdk_passed.is_some() || security_passed.is_some() {
        println!("\n--- Checks ---");
        if let Some(passed) = sdk_passed {
            println!(
                "SDK Compliance: {}",
                if passed { "Passed" } else { "Failed" }
            );
        }
        if let Some(passed) = security_passed {
            println!(
                "Security Scan: {}",
                if passed { "Passed" } else { "Failed" }
            );
        }
    }

    if let Some(reason) = build["blocked_reason"].as_str() {
        if !reason.is_empty() {
            println!("\n--- Blocked ---");
            println!("Reason: {}", reason);
        }
    }

    if let Some(image) = build["image_digest"].as_str() {
        if !image.is_empty() {
            println!("\n--- Image ---");
            println!("Digest: {}", truncate_hash(image, 12));
        }
    }

    if let Some(size) = build["image_size"].as_i64() {
        if size > 0 {
            println!("Size: {}", format_bytes(size as u64));
        }
    }

    if status == "pending"
        || status == "cloning"
        || status == "analyzing"
        || status == "building"
        || status == "pushing"
    {
        println!("\nBuild in progress. Check back soon or run:");
        println!(
            "  sekuire builds status {}",
            build["id"].as_str().unwrap_or("<build-id>")
        );
    }
}

fn format_status(status: &str) -> String {
    match status {
        "pending" => "Pending".to_string(),
        "cloning" => "Cloning".to_string(),
        "analyzing" => "Analyzing".to_string(),
        "building" => "Building".to_string(),
        "pushing" => "Pushing".to_string(),
        "completed" => "Done".to_string(),
        "failed" => "Failed".to_string(),
        "blocked" => "Blocked".to_string(),
        _ => status.to_string(),
    }
}

fn format_status_detailed(status: &str) -> String {
    match status {
        "pending" => "Pending (queued)".to_string(),
        "cloning" => "Cloning repository...".to_string(),
        "analyzing" => "Running source analysis...".to_string(),
        "building" => "Building Docker image...".to_string(),
        "pushing" => "Pushing to registry...".to_string(),
        "completed" => "Completed successfully".to_string(),
        "failed" => "Failed".to_string(),
        "blocked" => "Blocked (compliance failure)".to_string(),
        _ => status.to_string(),
    }
}

fn format_timestamp(ts: &str) -> String {
    if ts.is_empty() {
        return "-".to_string();
    }
    if ts.len() > 16 {
        ts[..16].replace('T', " ")
    } else {
        ts.to_string()
    }
}

fn truncate_string(s: &str, len: usize) -> String {
    if s.len() <= len {
        s.to_string()
    } else {
        format!("{}...", &s[..len.saturating_sub(3)])
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

async fn get_sekuire_id_from_manifest() -> Result<String, CliError> {
    let manifest_path = Path::new(config::MANIFEST_YAML);
    if !manifest_path.exists() {
        return Err(CliError::InvalidInput(
            "No agent ID provided and no manifest found. Run from agent directory or provide --agent.".into(),
        ));
    }

    let content = fs::read_to_string(manifest_path).await?;
    let manifest: serde_json::Value =
        serde_yaml::from_str(&content).map_err(|e| CliError::ManifestParse(e.to_string()))?;

    // First check if sekuire_id is already saved in the manifest
    if let Some(id) = manifest["project"]["sekuire_id"].as_str() {
        return Ok(id.to_string());
    }

    // Fall back to calculating from manifest contents
    let model_name = manifest["identity"]["model"].as_str().unwrap_or("");
    let system_prompt = manifest["identity"]["system_prompt"].as_str().unwrap_or("");
    let tools_json = serde_json::to_string(&manifest["identity"]["tools"]).unwrap_or_default();

    let mut hasher = blake3::Hasher::new();
    hasher.update(model_name.to_lowercase().trim().as_bytes());
    hasher.update(blake3::hash(system_prompt.as_bytes()).as_bytes());
    hasher.update(blake3::hash(tools_json.as_bytes()).as_bytes());

    Ok(truncate_hash(&hasher.finalize().to_hex(), 16).to_string())
}
