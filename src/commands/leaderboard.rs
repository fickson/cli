use crate::error::CliError;
use crate::ui;

pub async fn execute(
    limit: usize,
    category: Option<String>,
    verified_only: bool,
    format: String,
    registry: String,
) -> Result<(), CliError> {
    println!("Agent Leaderboard");

    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/api/v1/leaderboard?limit={}",
        registry.trim_end_matches('/'),
        limit
    );

    if let Some(c) = category {
        url.push_str(&format!("&category={}", urlencoding::encode(&c)));
    }
    if verified_only {
        url.push_str("&verified_only=true");
    }

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(Some(&format)),
        "Fetching leaderboard...",
        async { client.get(&url).send().await },
    )
    .await
    .map_err(|e| CliError::VerificationFailed(format!("Could not reach registry: {}", e)))?;

    if !resp.status().is_success() {
        return Err(CliError::RegistryError {
            status: resp.status().as_u16(),
            body: "Could not fetch leaderboard".into(),
        });
    }

    let result: serde_json::Value = resp.json().await.map_err(|e| CliError::Other(e.into()))?;

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| CliError::Other(e.into()))?
        );
    } else {
        let entries = result["entries"]
            .as_array()
            .or(result["leaderboard"].as_array());

        if let Some(entries) = entries {
            if entries.is_empty() {
                println!("   No agents found on leaderboard");
            } else {
                println!();
                println!(
                    "   {:4} {:30} {:>8} {:>10} {:10}",
                    "RANK", "NAME", "SCORE", "TASKS", "STATUS"
                );
                println!("   {:-<70}", "");
                for (i, entry) in entries.iter().enumerate().take(limit) {
                    let rank = entry["rank"].as_i64().unwrap_or((i + 1) as i64);
                    let name = entry["name"].as_str().unwrap_or("unknown");
                    let score = entry["reputationScore"]
                        .as_i64()
                        .or(entry["reputation_score"].as_i64())
                        .unwrap_or(0);
                    let tasks = entry["taskCount"]
                        .as_i64()
                        .or(entry["task_count"].as_i64())
                        .unwrap_or(0);
                    let status = entry["verificationStatus"]
                        .as_str()
                        .or(entry["verification_status"].as_str())
                        .unwrap_or("-");

                    let display_name = if name.len() > 28 { &name[..28] } else { name };
                    println!(
                        "   {:4} {:30} {:>8} {:>10} {:10}",
                        rank, display_name, score, tasks, status
                    );
                }
                println!("   {:-<70}", "");
            }
        } else {
            println!("   No leaderboard data available");
        }
    }
    Ok(())
}
