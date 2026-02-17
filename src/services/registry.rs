use crate::services::auth::AuthConfig;
use crate::ui;
use anyhow::{Context, Result};
use reqwest::Client;
use std::process::Command;

pub async fn handle_registry_login(registry: &str, backend: &str) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    if !auth_path.exists() {
        anyhow::bail!("Not logged in. Run 'sekuire login' first.");
    }

    let content = tokio::fs::read_to_string(&auth_path).await?;
    let config: AuthConfig = serde_json::from_str(&content)?;
    let api_key = config
        .api_key
        .context("No API key configured. Run 'sekuire login' first.")?;

    println!("  Fetching registry credentials...");

    let client = Client::new();

    let me_resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching registry credentials...",
        async {
            client
                .get(format!("{}/api/v1/me", backend))
                .header("X-API-Key", &api_key)
                .send()
                .await
        },
    )
    .await?;

    let org_id = if me_resp.status().is_success() {
        let me: serde_json::Value = me_resp.json().await?;
        me["organization"]["id"].as_str().map(|s| s.to_string())
    } else {
        None
    };

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Requesting registry token...",
        async {
            client
                .post(format!("{}/api/v1/registry/push-token", backend))
                .header("X-API-Key", &api_key)
                .json(&serde_json::json!({
                    "org_id": org_id,
                    "namespace_type": "private"
                }))
                .send()
                .await
        },
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get registry token: {} {}", status, body);
    }

    let data: serde_json::Value = resp.json().await?;
    let username = data["username"].as_str().unwrap_or("token");
    let password = data["token"].as_str().context("No token in response")?;

    println!("  Logging in to {}...", registry);

    let output = Command::new("docker")
        .args(["login", registry, "-u", username, "--password-stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to run docker. Is Docker installed?")?
        .wait_with_output()?;

    if !output.status.success() {
        // Try writing password via pipe
        let mut child = Command::new("docker")
            .args(["login", registry, "-u", username, "-p", password])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to run docker login")?;
        let result = child.wait()?;
        if !result.success() {
            anyhow::bail!("docker login failed");
        }
    }

    println!("  Logged in to {}", registry);
    Ok(())
}

pub async fn handle_registry_pull(image: &str, registry: &str, backend: &str) -> Result<()> {
    let full_image = if image.contains('/') && image.starts_with(registry) {
        image.to_string()
    } else if image.starts_with("org/") || image.starts_with("registry.") {
        format!(
            "{}/{}",
            registry,
            image.trim_start_matches("registry.sekuire.ai/")
        )
    } else {
        format!("{}/{}", registry, image)
    };

    let is_private = full_image.contains("/org/");

    if is_private {
        println!("  Private image detected, authenticating...");
        handle_registry_login(registry, backend).await?;
    }

    println!("  Pulling {}...", full_image);

    let pull_status = Command::new("docker")
        .args(["pull", &full_image])
        .status()
        .context("Failed to run docker pull. Is Docker installed?")?;

    if !pull_status.success() {
        if is_private {
            anyhow::bail!(
                "docker pull failed. Ensure you're a member of the organization that owns this image."
            );
        } else {
            anyhow::bail!("docker pull failed");
        }
    }

    println!("  Pulled {}", full_image);
    Ok(())
}

pub async fn handle_registry_push(image: &str, registry: &str, backend: &str) -> Result<()> {
    // Ensure logged in
    handle_registry_login(registry, backend).await.ok();

    let registry_image = if image.contains('/') && image.starts_with(registry) {
        image.to_string()
    } else {
        format!("{}/{}", registry, image)
    };

    println!("  Tagging {} -> {}", image, registry_image);

    let tag_status = Command::new("docker")
        .args(["tag", image, &registry_image])
        .status()
        .context("Failed to run docker tag")?;

    if !tag_status.success() {
        anyhow::bail!("docker tag failed");
    }

    println!("  Pushing {}...", registry_image);

    let push_status = Command::new("docker")
        .args(["push", &registry_image])
        .status()
        .context("Failed to run docker push")?;

    if !push_status.success() {
        anyhow::bail!("docker push failed");
    }

    println!("  Pushed {}", registry_image);
    Ok(())
}

pub async fn handle_images_list(agent: Option<&str>, backend: &str, format: &str) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    let content = tokio::fs::read_to_string(&auth_path).await?;
    let config: AuthConfig = serde_json::from_str(&content)?;
    let api_key = config.api_key.context("No API key configured")?;

    let agent_id = match agent {
        Some(id) => id.to_string(),
        None => get_local_agent_id().await?,
    };

    let client = Client::new();
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(Some(format)),
        "Fetching images...",
        async {
            client
                .get(format!("{}/api/v1/agents/{}/images", backend, agent_id))
                .header("X-API-Key", &api_key)
                .send()
                .await
        },
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to list images: {} {}", status, body);
    }

    let images: Vec<serde_json::Value> = resp.json().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&images)?);
        return Ok(());
    }

    if images.is_empty() {
        println!("  No images found for agent {}", agent_id);
        println!("  Push one with: sekuire registry push <image>");
        return Ok(());
    }

    println!(
        "{:<20} {:<16} {:<10} {:<20} {:<12}",
        "TAG", "DIGEST", "SIZE", "PUSHED", "SCAN"
    );
    println!("{}", "-".repeat(80));

    for img in &images {
        let tag = img["tag"].as_str().unwrap_or("-");
        let digest = img["digest"].as_str().unwrap_or("-");
        let digest_short = if digest.len() > 14 {
            &digest[7..21]
        } else {
            digest
        };
        let size = img["size_bytes"].as_i64().unwrap_or(0);
        let size_str = format_bytes(size);
        let pushed = img["pushed_at"].as_str().unwrap_or("-");
        let pushed_short = if pushed.len() > 19 {
            &pushed[..19]
        } else {
            pushed
        };
        let scan = img["scan_status"].as_str().unwrap_or("none");

        println!(
            "{:<20} {:<16} {:<10} {:<20} {:<12}",
            tag, digest_short, size_str, pushed_short, scan
        );
    }

    println!("\n  {} image(s) total", images.len());
    Ok(())
}

pub async fn handle_images_scan(image_id: &str, backend: &str) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    let content = tokio::fs::read_to_string(&auth_path).await?;
    let config: AuthConfig = serde_json::from_str(&content)?;
    let api_key = config.api_key.context("No API key configured")?;

    println!("  Triggering scan for image {}...", image_id);

    let client = Client::new();
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Triggering image scan...",
        async {
            client
                .post(format!("{}/api/v1/images/{}/scan", backend, image_id))
                .header("X-API-Key", &api_key)
                .send()
                .await
        },
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to trigger scan: {} {}", status, body);
    }

    println!(
        "  Scan triggered. Run 'sekuire images scan-report {}' to view results.",
        image_id
    );
    Ok(())
}

pub async fn handle_images_scan_report(image_id: &str, backend: &str, format: &str) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    let content = tokio::fs::read_to_string(&auth_path).await?;
    let config: AuthConfig = serde_json::from_str(&content)?;
    let api_key = config.api_key.context("No API key configured")?;

    let client = Client::new();
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(Some(format)),
        "Fetching scan report...",
        async {
            client
                .get(format!("{}/api/v1/images/{}/scan", backend, image_id))
                .header("X-API-Key", &api_key)
                .send()
                .await
        },
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get scan report: {} {}", status, body);
    }

    let report: serde_json::Value = resp.json().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let scanned_at = report["scanned_at"].as_str().unwrap_or("unknown");
    println!("  Scan Report for image {}", image_id);
    println!("  Scanned: {}", scanned_at);
    println!();

    let vulns = report["vulnerabilities"].as_array();
    match vulns {
        Some(v) if v.is_empty() => {
            println!("  No vulnerabilities found.");
        }
        Some(v) => {
            let critical = v
                .iter()
                .filter(|x| x["severity"].as_str() == Some("CRITICAL"))
                .count();
            let high = v
                .iter()
                .filter(|x| x["severity"].as_str() == Some("HIGH"))
                .count();
            let medium = v
                .iter()
                .filter(|x| x["severity"].as_str() == Some("MEDIUM"))
                .count();
            let low = v
                .iter()
                .filter(|x| x["severity"].as_str() == Some("LOW"))
                .count();

            println!(
                "  Summary: {} critical, {} high, {} medium, {} low",
                critical, high, medium, low
            );
            println!();

            println!(
                "{:<20} {:<16} {:<12} {:<10}",
                "PACKAGE", "VERSION", "FIXED IN", "SEVERITY"
            );
            println!("{}", "-".repeat(60));

            for vuln in v {
                let pkg = vuln["package_name"].as_str().unwrap_or("-");
                let ver = vuln["installed_version"].as_str().unwrap_or("-");
                let fixed = vuln["fixed_version"].as_str().unwrap_or("-");
                let sev = vuln["severity"].as_str().unwrap_or("-");

                println!("{:<20} {:<16} {:<12} {:<10}", pkg, ver, fixed, sev);
            }
        }
        None => {
            println!(
                "  No scan results available. Run 'sekuire images scan {}' first.",
                image_id
            );
        }
    }

    Ok(())
}

async fn get_local_agent_id() -> Result<String> {
    if std::path::Path::new("sekuire.yml").exists() {
        let content = tokio::fs::read_to_string("sekuire.yml").await?;
        let manifest: serde_json::Value = serde_yaml::from_str(&content)?;
        if let Some(id) = manifest["project"]["sekuire_id"].as_str() {
            return Ok(id.to_string());
        }
    }
    if std::path::Path::new("sekuire.json").exists() {
        let content = tokio::fs::read_to_string("sekuire.json").await?;
        let manifest: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(id) = manifest["project"]["sekuire_id"].as_str() {
            return Ok(id.to_string());
        }
    }
    anyhow::bail!(
        "No agent ID found. Use --agent flag or run from a project directory with sekuire.yml"
    )
}

fn format_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
