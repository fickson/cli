use crate::config;
use crate::error::CliError;
use crate::services::auth::AuthConfig;
use crate::services::device::{get_device_fingerprint, get_device_name, get_device_os};
use crate::ui;
use dialoguer::{theme::ColorfulTheme, Input};

pub async fn execute(web_url: &str, backend: &str) -> Result<(), CliError> {
    println!("Sekuire Login");
    println!("\nTo authenticate:");
    println!("   1. Go to: {}/dashboard/settings?tab=api-keys", web_url);
    println!("   2. Create a new API Key for CLI access");
    println!("   3. Copy the key (you'll only see it once!)");
    println!("   4. Paste it below\n");

    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your API Key")
        .interact_text()
        .map_err(|e| CliError::Other(e.into()))?;

    let device_fingerprint = get_device_fingerprint();
    let device_name = get_device_name();
    let device_os = get_device_os();

    let client = reqwest::Client::new();
    let verify_url = format!("{}/api/v1/api-keys/verify", backend);

    let verify_payload = serde_json::json!({
        "api_key": api_key,
        "device_fingerprint": device_fingerprint,
        "device_name": device_name,
        "device_os": device_os,
    });

    let response = ui::with_spinner_if(ui::spinner_enabled(None), "Verifying API key...", async {
        client.post(&verify_url).json(&verify_payload).send().await
    })
    .await?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(CliError::RegistryError {
            status: 0,
            body: error_text,
        });
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| CliError::Other(e.into()))?;

    if !result["valid"].as_bool().unwrap_or(false) {
        let error = result["error"]
            .as_str()
            .unwrap_or("Invalid API Key")
            .to_string();
        return Err(CliError::InvalidInput(error));
    }

    let org_id = result
        .get("org_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let home = dirs::home_dir()
        .ok_or_else(|| CliError::Other(anyhow::anyhow!("Could not find home directory")))?;
    let config_dir = home.join(config::SEKUIRE_DIR);
    if !config_dir.exists() {
        tokio::fs::create_dir_all(&config_dir).await?;
    }

    let existing_workspace_id = if config_dir.join(config::AUTH_CONFIG_FILE).exists() {
        let content = tokio::fs::read_to_string(config_dir.join(config::AUTH_CONFIG_FILE)).await?;
        serde_json::from_str::<AuthConfig>(&content)
            .ok()
            .and_then(|cfg| cfg.workspace_id)
    } else {
        None
    };

    let auth_config = AuthConfig {
        api_key: Some(api_key),
        token: None,
        device_fingerprint: Some(device_fingerprint.clone()),
        org_id,
        workspace_id: existing_workspace_id,
    };

    tokio::fs::write(
        config_dir.join(config::AUTH_CONFIG_FILE),
        serde_json::to_string_pretty(&auth_config).map_err(|e| CliError::Other(e.into()))?,
    )
    .await?;

    println!("Login successful! API Key verified and bound to this device.");
    println!("   Device: {}", device_name);
    println!("   OS: {}", device_os);
    println!(
        "   Fingerprint: {}...",
        crate::error::truncate_hash(&device_fingerprint, 16)
    );
    Ok(())
}
