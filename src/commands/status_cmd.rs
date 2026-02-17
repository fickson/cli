use crate::error::CliError;
use crate::services::auth::load_auth_session;
use crate::ui;

pub async fn execute(agent_id: String, registry: String) -> Result<(), CliError> {
    let auth = load_auth_session()
        .await
        .map_err(|_| CliError::NotAuthenticated)?;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/agents/{}",
        registry.trim_end_matches('/'),
        agent_id
    );

    let mut req = client.get(&url);
    if let Some(key) = auth.api_key.as_ref() {
        req = req.header("X-API-Key", key);
    } else if let Some(token) = auth.token.as_ref() {
        req = req.bearer_auth(token);
    }

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching agent status...",
        async { req.send().await },
    )
    .await
    .map_err(|e| CliError::Other(e.into()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::Other(anyhow::anyhow!(
            "Failed to fetch agent status: {} {}",
            status,
            body
        )));
    }

    let agent: serde_json::Value = resp.json().await.map_err(|e| CliError::Other(e.into()))?;

    let name = agent["name"].as_str().unwrap_or("unknown");
    let version = agent["version"].as_str().unwrap_or("0.0.0");
    let runtime_status = agent["runtime_status"].as_str().unwrap_or("unknown");
    let verification = agent["verification_status"]
        .as_str()
        .unwrap_or("unverified");
    let reputation = agent["reputation_score"].as_f64().unwrap_or(0.0);
    let last_heartbeat = agent["last_heartbeat_at"].as_str().unwrap_or("never");

    println!("Agent: {} v{}", name, version);
    println!("  Sekuire ID: {}", agent_id);
    println!("  Runtime:    {}", runtime_status);
    println!("  Verified:   {}", verification);
    println!("  Reputation: {:.0}", reputation);
    println!("  Heartbeat:  {}", last_heartbeat);

    if let Some(pending) = agent["pending_command"].as_str() {
        if !pending.is_empty() {
            println!("  Pending:    {}", pending);
        }
    }

    Ok(())
}
