use crate::error::CliError;
use crate::services::auth::calculate_identity_hash;
use crate::ui;

pub async fn execute(
    agent: Option<String>,
    _range: String,
    format: String,
    registry: String,
) -> Result<(), CliError> {
    let sekuire_id = if let Some(id) = agent {
        id
    } else {
        let (id, _) = calculate_identity_hash().await.map_err(CliError::Other)?;
        id
    };

    println!("Fetching metrics for agent: {}", sekuire_id);

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/agents/{}/reputation",
        registry.trim_end_matches('/'),
        sekuire_id
    );

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(Some(&format)),
        "Fetching metrics...",
        async { client.get(&url).send().await },
    )
    .await
    .map_err(|e| CliError::VerificationFailed(format!("Could not reach registry: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(CliError::RegistryError { status, body: text });
    }

    let result: serde_json::Value = resp.json().await.map_err(|e| CliError::Other(e.into()))?;

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| CliError::Other(e.into()))?
        );
    } else {
        println!();
        println!("   Agent Reputation & Metrics");
        println!("   {:-<50}", "");
        println!(
            "   Reputation Score: {}/100",
            result["score"].as_i64().unwrap_or(0)
        );
        println!(
            "   Tasks Completed:  {}",
            result["taskCount"]
                .as_i64()
                .or(result["task_count"].as_i64())
                .unwrap_or(0)
        );
        if let Some(badge) = result["verificationBadge"]
            .as_str()
            .or(result["verification_badge"].as_str())
        {
            if !badge.is_empty() {
                println!("   Verification:     {}", badge);
            }
        }
        println!("   {:-<50}", "");

        if let Some(logs) = result["recentLogs"]
            .as_array()
            .or(result["recent_logs"].as_array())
        {
            if !logs.is_empty() {
                println!("\n   Recent Activity:");
                for log in logs.iter().take(5) {
                    let rating = log["rating"].as_i64().unwrap_or(0);
                    let stars = "*".repeat(rating as usize);
                    let comment = log["comment"].as_str().unwrap_or("");
                    let display_comment = if comment.len() > 40 {
                        &comment[..40]
                    } else {
                        comment
                    };
                    println!("      {} {}", stars, display_comment);
                }
            }
        }
    }
    Ok(())
}
