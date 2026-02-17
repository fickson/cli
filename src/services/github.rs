use crate::services::auth::AuthConfig;
use crate::ui;
use anyhow::{Context, Result};
use reqwest::Client;

pub async fn handle_github_install(backend: &str) -> Result<()> {
    println!("\nSekuire GitHub App Installation");
    println!("--------------------------------\n");

    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    let client = Client::new();

    // Send auth headers if logged in so backend can include org_id in state
    let mut req = client.get(format!("{}/api/v1/github/app-url", backend));

    if auth_path.exists() {
        let content = tokio::fs::read_to_string(&auth_path).await?;
        let config: AuthConfig = serde_json::from_str(&content)?;

        if let Some(ref api_key) = config.api_key {
            req = req.header("X-API-Key", api_key);
            if let Some(ref fp) = config.device_fingerprint {
                req = req.header("X-Device-Fingerprint", fp);
            }
        }
    }

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching GitHub App URL...",
        async { req.send().await },
    )
    .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to get GitHub App URL from server");
    }

    let data: serde_json::Value = resp.json().await?;
    let install_url = data["install_url"]
        .as_str()
        .context("No install_url in response")?;
    let app_slug = data["app_slug"].as_str().unwrap_or("sekuire");

    println!("The Sekuire GitHub App enables:");
    println!("  - Source code analysis for security scanning");
    println!("  - Automatic capability detection");
    println!("  - Push-triggered analysis updates");
    println!();
    println!("Opening browser to install '{}'...", app_slug);

    if webbrowser::open(install_url).is_err() {
        println!();
        println!("Could not open browser. Visit this URL manually:");
        println!("  {}", install_url);
    }

    println!();
    println!("After installation, the app will have access to selected repositories.");
    println!("You can then connect repos to agents with:");
    println!("  sekuire github connect <repo-url>");

    Ok(())
}

pub async fn handle_github_installations(backend: &str) -> Result<()> {
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
    let device_fingerprint = config
        .device_fingerprint
        .context("No device fingerprint. Run 'sekuire login' again.")?;

    println!("\nGitHub App Installations");
    println!("------------------------\n");

    let client = Client::new();
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching GitHub installations...",
        async {
            client
                .get(format!("{}/api/v1/github/installations", backend))
                .header("X-API-Key", &api_key)
                .header("X-Device-Fingerprint", &device_fingerprint)
                .send()
                .await
        },
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        if status.as_u16() == 401 {
            anyhow::bail!("Not authenticated. Run 'sekuire login' first.");
        }
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to list installations: {} {}", status, body);
    }

    let data: serde_json::Value = resp.json().await?;
    let installations = data["installations"].as_array();

    match installations {
        Some(list) if list.is_empty() => {
            println!("No GitHub App installations found.");
            println!();
            println!("Install the Sekuire GitHub App with:");
            println!("  sekuire github install");
        }
        Some(list) => {
            println!("{:<30} {:<20} {:<15}", "ACCOUNT", "TYPE", "INSTALLED");
            println!("{}", "-".repeat(65));

            for inst in list {
                let account = inst["account_login"].as_str().unwrap_or("-");
                let account_type = inst["account_type"].as_str().unwrap_or("-");
                let installed = inst["installed_at"].as_str().unwrap_or("-");
                let installed_short = if installed.len() > 10 {
                    &installed[..10]
                } else {
                    installed
                };

                println!(
                    "{:<30} {:<20} {:<15}",
                    account, account_type, installed_short
                );
            }

            println!();
            println!("{} installation(s) found", list.len());
        }
        None => {
            println!("No installations data returned.");
        }
    }

    Ok(())
}

pub async fn handle_github_connect(repo: &str, agent: Option<&str>, backend: &str) -> Result<()> {
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
    let device_fingerprint = config
        .device_fingerprint
        .context("No device fingerprint. Run 'sekuire login' again.")?;

    let sekuire_id = match agent {
        Some(id) => id.to_string(),
        None => get_local_agent_id().await?,
    };

    println!("\nConnecting Repository to Agent");
    println!("-------------------------------\n");
    println!("  Agent:      {}", sekuire_id);
    println!("  Repository: {}", repo);
    println!();

    let client = Client::new();
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Connecting repository...",
        async {
            client
                .post(format!("{}/api/v1/github/connect-repo", backend))
                .header("X-API-Key", &api_key)
                .header("X-Device-Fingerprint", &device_fingerprint)
                .json(&serde_json::json!({
                    "sekuire_id": sekuire_id,
                    "repo_url": repo,
                }))
                .send()
                .await
        },
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if body.contains("installation") {
            println!("GitHub App not installed for this repository.");
            println!();
            println!("Install the Sekuire GitHub App first:");
            println!("  sekuire github install");
            println!();
            println!("Make sure to grant access to: {}", repo);
            anyhow::bail!("GitHub App not installed");
        }

        anyhow::bail!("Failed to connect repo: {} {}", status, body);
    }

    let data: serde_json::Value = resp.json().await?;
    let analysis_triggered = data["analysis_triggered"].as_bool().unwrap_or(false);

    println!("Repository connected successfully!");
    println!();

    if analysis_triggered {
        println!("Source analysis has been triggered.");
        println!("Check status with: sekuire analysis status");
    } else {
        println!("Repository connected but analysis was not triggered.");
        println!("You may need to install the GitHub App for full source analysis.");
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
