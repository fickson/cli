use crate::error::CliError;
use crate::ui;

pub async fn execute(
    query: String,
    tags: Option<String>,
    verified: Option<bool>,
    category: Option<String>,
    limit: usize,
    format: String,
    registry: String,
) -> Result<(), CliError> {
    println!("Searching registry for: \"{}\"", query);

    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/api/v1/agents?q={}",
        registry.trim_end_matches('/'),
        urlencoding::encode(&query)
    );

    if let Some(t) = tags {
        url.push_str(&format!("&tags={}", urlencoding::encode(&t)));
    }
    if let Some(v) = verified {
        url.push_str(&format!(
            "&verification_status={}",
            if v { "verified" } else { "unverified" }
        ));
    }
    if let Some(c) = category {
        url.push_str(&format!("&category={}", urlencoding::encode(&c)));
    }
    url.push_str(&format!("&limit={}", limit));

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(Some(&format)),
        "Searching registry...",
        async { client.get(&url).send().await },
    )
    .await
    .map_err(|e| CliError::VerificationFailed(format!("Could not reach registry: {}", e)))?;

    if !resp.status().is_success() {
        return Err(CliError::RegistryError {
            status: resp.status().as_u16(),
            body: "Search failed".into(),
        });
    }

    let result: serde_json::Value = resp.json().await.map_err(|e| CliError::Other(e.into()))?;
    let agents = result["agents"].as_array();

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| CliError::Other(e.into()))?
        );
    } else if let Some(agents) = agents {
        if agents.is_empty() {
            println!("   No agents found matching \"{}\"", query);
        } else {
            println!();
            println!(
                "   {:30} {:8} {:10} {:>8}",
                "NAME", "VERSION", "STATUS", "SCORE"
            );
            println!("   {:-<60}", "");
            for agent in agents.iter().take(limit) {
                let name = agent["name"].as_str().unwrap_or("unknown");
                let version = agent["version"].as_str().unwrap_or("-");
                let status = agent["verification_status"].as_str().unwrap_or("-");
                let score = agent["reputation_score"].as_i64().unwrap_or(0);
                let display_name = if name.len() > 28 { &name[..28] } else { name };
                println!(
                    "   {:30} {:8} {:10} {:>8}",
                    display_name, version, status, score
                );
            }
            println!("   {:-<60}", "");
            println!("\n   Found {} agent(s)", agents.len());
        }
    }
    Ok(())
}
