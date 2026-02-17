use crate::config;
use crate::error::{truncate_hash, CliError};
use crate::services::http_client::SekuireClient;
use crate::ui;
use std::path::Path;
use tokio::fs;

pub async fn execute(registry: String, sekuire_id: Option<String>) -> Result<(), CliError> {
    let agent_id = match sekuire_id {
        Some(id) => id,
        None => get_sekuire_id_from_manifest().await?,
    };

    println!("\nSekuire Source Analysis");
    println!("-----------------------");
    println!("Agent: {}\n", truncate_hash(&agent_id, 8));

    let client = SekuireClient::from_registry(&registry).await?;

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching source analysis...",
        async {
            client
                .get(&format!("/api/v1/agents/{}/analysis", agent_id))
                .await
        },
    )
    .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let analysis: serde_json::Value = resp.json().await.unwrap_or_default();
            print_analysis(&analysis);
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            println!("No analysis found for this agent.");
            println!("\nTo enable source analysis:");
            println!("  1. Connect your repository: sekuire publish --repo <url>");
            println!("  2. Install the Sekuire GitHub App");
            println!("  3. Trigger analysis: sekuire analyze");
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

fn print_analysis(analysis: &serde_json::Value) {
    let status = analysis["status"].as_str().unwrap_or("unknown");
    let analyzed_at = analysis["analyzed_at"].as_str().unwrap_or("");
    let commit = analysis["commit_sha"].as_str().unwrap_or("");

    println!("Status: {}", format_status(status));
    if !analyzed_at.is_empty() {
        println!("Analyzed: {}", analyzed_at);
    }
    if !commit.is_empty() {
        println!("Commit: {}", truncate_hash(commit, 8));
    }

    if status == "pending" || status == "running" {
        println!("\nAnalysis in progress. Check back soon.");
        return;
    }

    if status == "failed" {
        if let Some(error) = analysis["error_message"].as_str() {
            println!("\nError: {}", error);
        }
        return;
    }

    println!("\n--- SDK Compliance ---");
    let sdk = &analysis["sdk_compliance"];
    let sdk_detected = sdk["sdk_detected"].as_bool().unwrap_or(false);
    println!("SDK Detected: {}", if sdk_detected { "Yes" } else { "No" });
    if sdk_detected {
        if let Some(version) = sdk["sdk_version"].as_str() {
            println!("SDK Version: {}", version);
        }
        if let Some(lang) = sdk["language"].as_str() {
            println!("Language: {}", lang);
        }
        let lifecycle = sdk["lifecycle_complete"].as_bool().unwrap_or(false);
        println!(
            "Lifecycle Complete: {}",
            if lifecycle { "Yes" } else { "No" }
        );
    }

    println!("\n--- Capabilities ---");
    let caps = &analysis["capabilities"];

    if let Some(declared) = caps["declared"].as_object() {
        println!("Declared:");
        for (key, value) in declared {
            if let Some(arr) = value.as_array() {
                if !arr.is_empty() {
                    let items: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                    println!("  {}: {}", key, items.join(", "));
                }
            }
        }
    }

    if let Some(detected) = caps["detected"].as_object() {
        println!("Detected:");
        for (key, value) in detected {
            if let Some(arr) = value.as_array() {
                if !arr.is_empty() {
                    let items: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                    println!("  {}: {}", key, items.join(", "));
                }
            }
        }
    }

    if let Some(gaps) = caps["gaps"].as_array() {
        if !gaps.is_empty() {
            println!("Gaps (undeclared capabilities):");
            for gap in gaps {
                let gap_type = gap["type"].as_str().unwrap_or("unknown");
                let undeclared = gap["undeclared"].as_str().unwrap_or("");
                println!("  - [{}] {}", gap_type, undeclared);
            }
        }
    }

    println!("\n--- Security ---");
    let security = &analysis["security"];
    let sast_count = security["sast_issues"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let sca_count = security["sca_vulnerabilities"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let secrets = security["secrets_found"].as_i64().unwrap_or(0);

    println!("SAST Issues: {}", sast_count);
    println!("SCA Vulnerabilities: {}", sca_count);
    println!("Secrets Found: {}", secrets);

    if let Some(score) = analysis["transparency_score"].as_i64() {
        println!("\n--- Transparency Score ---");
        println!("Score: {}/100", score);
        println!("(Higher = more honest capability declarations)");
    }
}

fn format_status(status: &str) -> String {
    match status {
        "pending" => "Pending".to_string(),
        "running" => "Running...".to_string(),
        "completed" => "Completed".to_string(),
        "failed" => "Failed".to_string(),
        _ => status.to_string(),
    }
}

async fn get_sekuire_id_from_manifest() -> Result<String, CliError> {
    let manifest_path = Path::new(config::MANIFEST_YAML);
    if !manifest_path.exists() {
        return Err(CliError::InvalidInput(
            "No sekuire_id provided and no manifest found. Run from agent directory or provide --id.".into(),
        ));
    }

    let content = fs::read_to_string(manifest_path).await?;
    let manifest: serde_json::Value =
        serde_yaml::from_str(&content).map_err(|e| CliError::ManifestParse(e.to_string()))?;

    let model_name = manifest["model"].as_str().unwrap_or("");
    let system_prompt = manifest["system_prompt"].as_str().unwrap_or("");
    let tools_json = serde_json::to_string(&manifest["tools"]).unwrap_or_default();

    let mut hasher = blake3::Hasher::new();
    hasher.update(model_name.to_lowercase().trim().as_bytes());
    hasher.update(blake3::hash(system_prompt.as_bytes()).as_bytes());
    hasher.update(blake3::hash(tools_json.as_bytes()).as_bytes());

    Ok(truncate_hash(&hasher.finalize().to_hex(), 16).to_string())
}
