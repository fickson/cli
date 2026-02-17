use crate::error::CliError;
use crate::services::auth::load_auth_session;
use crate::ui;

pub async fn execute(agent_id: String, limit: usize, registry: String) -> Result<(), CliError> {
    let auth = load_auth_session()
        .await
        .map_err(|_| CliError::NotAuthenticated)?;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/agents/{}/logs?limit={}",
        registry.trim_end_matches('/'),
        agent_id,
        limit
    );

    let mut req = client.get(&url);
    if let Some(key) = auth.api_key.as_ref() {
        req = req.header("X-API-Key", key);
    } else if let Some(token) = auth.token.as_ref() {
        req = req.bearer_auth(token);
    }

    let resp = ui::with_spinner_if(ui::spinner_enabled(None), "Fetching logs...", async {
        req.send().await
    })
    .await
    .map_err(|e| CliError::Other(e.into()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::Other(anyhow::anyhow!(
            "Failed to fetch logs: {} {}",
            status,
            body
        )));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| CliError::Other(e.into()))?;

    let logs = body["logs"].as_array().or_else(|| body.as_array());

    if let Some(entries) = logs {
        if entries.is_empty() {
            println!("No logs found for agent {}", agent_id);
            return Ok(());
        }
        for entry in entries {
            let ts = entry["event_timestamp"]
                .as_str()
                .or_else(|| entry["timestamp"].as_str())
                .unwrap_or("-");
            let severity = entry["severity"].as_str().unwrap_or("info");
            let event_type = entry["event_type"].as_str().unwrap_or("unknown");
            let data = &entry["event_data"];

            let severity_label = match severity {
                "error" => "ERR",
                "warning" | "warn" => "WRN",
                "debug" => "DBG",
                _ => "INF",
            };

            let detail = if let Some(tool) = data["tool"].as_str() {
                format!("tool={}", tool)
            } else if let Some(msg) = data["message"].as_str() {
                msg.to_string()
            } else {
                serde_json::to_string(data).unwrap_or_default()
            };

            println!("[{}] {} {} {}", ts, severity_label, event_type, detail);
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }

    Ok(())
}
