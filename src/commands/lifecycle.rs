use crate::error::CliError;
use crate::services::auth::load_auth_session;
use crate::ui;

pub enum LifecycleAction {
    Pause,
    Resume,
    Terminate,
}

impl LifecycleAction {
    fn endpoint(&self) -> &'static str {
        match self {
            LifecycleAction::Pause => "pause",
            LifecycleAction::Resume => "resume",
            LifecycleAction::Terminate => "terminate",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            LifecycleAction::Pause => "paused",
            LifecycleAction::Resume => "resumed",
            LifecycleAction::Terminate => "terminated",
        }
    }
}

pub async fn execute(
    agent_id: String,
    action: LifecycleAction,
    reason: Option<String>,
    registry: String,
) -> Result<(), CliError> {
    let auth = load_auth_session()
        .await
        .map_err(|_| CliError::NotAuthenticated)?;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/agents/{}/{}",
        registry.trim_end_matches('/'),
        agent_id,
        action.endpoint()
    );

    let body = if let Some(r) = &reason {
        serde_json::json!({ "reason": r })
    } else {
        serde_json::json!({})
    };

    let mut req = client.post(&url).json(&body);
    if let Some(key) = auth.api_key.as_ref() {
        req = req.header("X-API-Key", key);
    } else if let Some(token) = auth.token.as_ref() {
        req = req.bearer_auth(token);
    }

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Sending lifecycle command...",
        async { req.send().await },
    )
    .await
    .map_err(|e| CliError::Other(e.into()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(CliError::Other(anyhow::anyhow!(
            "Failed to {} agent: {} {}",
            action.endpoint(),
            status,
            text
        )));
    }

    println!("Agent {} {}", agent_id, action.label());
    if let Some(r) = reason {
        println!("  Reason: {}", r);
    }

    Ok(())
}
