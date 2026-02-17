use crate::config;
use crate::error::{truncate_hash, CliError};
use crate::services::auth::{calculate_identity_hash, AuthConfig};
use crate::services::device::{get_device_fingerprint, get_device_name, get_device_os};
use crate::services::policy::{
    fetch_active_policy, load_policy_file, validate_manifest_against_policy, verify_active_policy,
};
use crate::ui;
use ed25519_dalek::{Signer, SigningKey};
use std::path::Path;
use tokio::fs;

pub async fn execute(
    registry: String,
    workspace: Option<String>,
    policy: String,
    force: bool,
) -> Result<(), CliError> {
    let (sekuire_id, manifest) = calculate_identity_hash().await.map_err(CliError::Other)?;
    println!("Identity: {}", sekuire_id);

    let key_path = Path::new(config::SEKUIRE_DIR).join(config::SECRET_KEY_FILE);
    if !key_path.exists() {
        return Err(CliError::KeyNotFound);
    }
    let priv_hex = fs::read_to_string(&key_path).await?;
    let priv_bytes = hex::decode(priv_hex.trim())?;
    let priv_array: [u8; 32] = priv_bytes
        .as_slice()
        .try_into()
        .map_err(|_| CliError::InvalidInput("Invalid private key length".into()))?;
    let signing_key = SigningKey::from_bytes(&priv_array);
    let pub_key = signing_key.verifying_key();
    let pub_hex = hex::encode(pub_key.as_bytes());

    let signature = signing_key.sign(sekuire_id.as_bytes());
    let signature_hex = hex::encode(signature.to_bytes());
    println!("Signature: {}...", truncate_hash(&signature_hex, 12));

    let home = dirs::home_dir()
        .ok_or_else(|| CliError::Other(anyhow::anyhow!("Could not find home directory")))?;
    let auth_path = home
        .join(config::SEKUIRE_DIR)
        .join(config::AUTH_CONFIG_FILE);

    if !auth_path.exists() {
        return Err(CliError::NotAuthenticated);
    }

    let content = fs::read_to_string(&auth_path).await?;
    let mut config_data: AuthConfig = serde_json::from_str(&content)?;

    let api_key = config_data.api_key.clone();
    let token = config_data.token.clone();
    let device_fingerprint = config_data
        .device_fingerprint
        .clone()
        .unwrap_or_else(get_device_fingerprint);

    if config_data.device_fingerprint.is_none() {
        config_data.device_fingerprint = Some(device_fingerprint.clone());
    }

    let api_key_for_push = if api_key.is_some() {
        api_key.clone()
    } else if token.is_some() {
        eprintln!("Warning: Using legacy token authentication.");
        eprintln!("   Run 'sekuire login' to upgrade to API keys.");
        None
    } else {
        return Err(CliError::NotAuthenticated);
    };

    println!("Pushing agent as DRAFT...");

    if let Some(ws) = workspace.as_ref() {
        println!("Validating active policy for workspace {}...", ws);
        let local_policy = if Path::new(&policy).exists() {
            load_policy_file(&policy).await.map_err(CliError::Other)?
        } else {
            println!(
                "   Local policy file {} not found. Fetching from backend...",
                policy
            );
            fetch_active_policy(&registry, ws)
                .await
                .map_err(CliError::Other)?
        };

        verify_active_policy(&local_policy).map_err(CliError::Other)?;
        let violations =
            validate_manifest_against_policy(&manifest, &local_policy).map_err(CliError::Other)?;
        if !violations.is_empty() {
            println!("   Policy violations detected:");
            for v in &violations {
                println!("      - {}", v);
            }
            if !force {
                return Err(CliError::VerificationFailed(
                    "Policy violations. Re-run with --force to override.".into(),
                ));
            }
            println!("   --force supplied; proceeding despite violations.");
        } else {
            println!(
                "   Policy {} (v{}) verified (hash/signature)",
                local_policy.policy_id, local_policy.version
            );
        }
    }

    let mut publisher_org_id = config_data.org_id.clone();
    if publisher_org_id.is_none() {
        if let Some(key) = api_key.as_ref() {
            let device_name = get_device_name();
            let device_os = get_device_os();
            let verify_url = format!("{}/api/v1/api-keys/verify", registry.trim_end_matches('/'));

            let verify_payload = serde_json::json!({
                "api_key": key,
                "device_fingerprint": device_fingerprint.clone(),
                "device_name": device_name,
                "device_os": device_os,
            });

            let client = reqwest::Client::new();
            let resp =
                ui::with_spinner_if(ui::spinner_enabled(None), "Verifying API key...", async {
                    client.post(&verify_url).json(&verify_payload).send().await
                })
                .await?;
            if resp.status().is_success() {
                let result: serde_json::Value =
                    resp.json().await.map_err(|e| CliError::Other(e.into()))?;
                if result["valid"].as_bool().unwrap_or(false) {
                    publisher_org_id = result
                        .get("org_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if publisher_org_id.is_some() {
                        config_data.org_id = publisher_org_id.clone();
                        fs::write(
                            &auth_path,
                            serde_json::to_string_pretty(&config_data)
                                .map_err(|e| CliError::Other(e.into()))?,
                        )
                        .await?;
                    }
                }
            }
        }
    }

    let payload = serde_json::json!({
        "manifest": manifest,
        "sekuire_id": sekuire_id.clone(),
        "signature": signature_hex,
        "public_key": pub_hex,
        "publisher_org_id": publisher_org_id,
        "visibility": null,
        "status": "draft"
    });

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/publish", registry);

    let mut req = client.post(&url).json(&payload);
    if let Some(key) = api_key_for_push {
        req = req.header("X-API-Key", key);
        req = req.header("X-Device-Fingerprint", device_fingerprint);
    } else if let Some(t) = token {
        req = req.bearer_auth(t);
    }

    let res = ui::with_spinner_if(ui::spinner_enabled(None), "Pushing agent draft...", async {
        req.send().await
    })
    .await?;

    if res.status().is_success() {
        // Save sekuire_id to manifest for future commands
        let manifest_path = Path::new(config::MANIFEST_YAML);
        if manifest_path.exists() {
            let content = fs::read_to_string(manifest_path).await?;
            let mut manifest_value: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| CliError::ManifestParse(e.to_string()))?;
            if let Some(project) = manifest_value.get_mut("project") {
                if let Some(project_map) = project.as_mapping_mut() {
                    project_map.insert(
                        serde_yaml::Value::String("sekuire_id".to_string()),
                        serde_yaml::Value::String(sekuire_id.clone()),
                    );
                    let updated = serde_yaml::to_string(&manifest_value)
                        .map_err(|e| CliError::ManifestParse(e.to_string()))?;
                    fs::write(manifest_path, updated).await?;
                }
            }
        }

        println!("Agent pushed as DRAFT!");
        println!("   This agent is in draft mode and not publicly listed.");
        println!(
            "   Go to the dashboard to verify and publish: {}/dashboard",
            registry.replace("9300", "9301")
        );
        println!();
        println!("Deployment Environment Variables:");
        println!("   SEKUIRE_AGENT_ID={}", sekuire_id);
        println!("   SEKUIRE_PRIVATE_KEY=<copy from .sekuire/secret.key>");
        println!();
        println!("   Optional:");
        println!(
            "   SEKUIRE_API_URL={}  (default)",
            config::DEFAULT_REGISTRY_URL
        );
        println!("   SEKUIRE_AGENT_NAME={}", manifest.project.name);
    } else {
        let status = res.status().as_u16();
        let text = res
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(CliError::RegistryError { status, body: text });
    }
    Ok(())
}
