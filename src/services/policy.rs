use crate::ui;
use anyhow::{bail, Context, Result};
use clap::Subcommand;
use ed25519_dalek::VerifyingKey;
use sekuire_protocol::Manifest;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct ActivePolicy {
    pub policy_id: String,
    pub workspace_id: String,
    pub version: String,
    pub status: String,
    pub hash: String,
    pub content: serde_json::Value,
    pub activated_at: Option<String>,
    pub updated_at: Option<String>,
    pub signature: Option<String>,
    pub signing_key_id: Option<String>,
    pub signing_public_key: Option<String>,
}

#[derive(Subcommand)]
pub enum PolicyCommands {
    /// Fetch the active workspace policy and store it locally
    Pull {
        /// Workspace ID to fetch policy for
        #[arg(long)]
        workspace: String,
        /// Backend base URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
        /// Output path for the policy JSON
        #[arg(long, default_value = "policy.json")]
        out: String,
    },
    /// Validate a downloaded policy hash/signature
    Validate {
        /// Path to policy file (JSON)
        #[arg(long, default_value = "policy.json")]
        file: String,
    },
    /// Show a summary of the downloaded policy
    Show {
        /// Path to policy file (JSON)
        #[arg(long, default_value = "policy.json")]
        file: String,
    },
}

pub async fn fetch_active_policy(backend: &str, workspace_id: &str) -> Result<ActivePolicy> {
    let url = format!(
        "{}/api/v1/workspaces/{}/policy/active",
        backend.trim_end_matches('/'),
        workspace_id
    );
    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Fetching active policy...",
        async { reqwest::get(&url).await },
    )
    .await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("No active policy found for workspace {}", workspace_id);
    }
    let policy: ActivePolicy = resp.error_for_status()?.json().await?;
    verify_active_policy(&policy)?;
    Ok(policy)
}

pub async fn load_policy_file(path: &str) -> Result<ActivePolicy> {
    let data = fs::read_to_string(path).await?;
    let policy: ActivePolicy = serde_json::from_str(&data)?;
    Ok(policy)
}

pub fn verify_active_policy(policy: &ActivePolicy) -> Result<()> {
    let serialized = serde_json::to_vec(&policy.content)?;
    let recalculated = blake3::hash(&serialized).to_hex().to_string();
    if recalculated != policy.hash {
        bail!(
            "Policy hash mismatch. Expected {}, got {}",
            policy.hash,
            recalculated
        );
    }

    if let Some(signature_hex) = &policy.signature {
        let pub_key_hex = policy
            .signing_public_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Signature present but signing_public_key missing"))?;
        let pub_key_bytes = hex::decode(pub_key_hex)
            .context("Failed to decode signing_public_key hex for policy verification")?;
        let verifying_key = VerifyingKey::from_bytes(
            pub_key_bytes
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("signing_public_key must be 32 bytes"))?,
        )
        .context("Failed to parse verifying key")?;

        let message = format!("{}:{}:{}", policy.policy_id, policy.version, policy.hash);
        let sig_bytes = hex::decode(signature_hex)
            .context("Failed to decode policy signature hex for verification")?;
        let signature = ed25519_dalek::Signature::from_bytes(
            sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("policy signature must be 64 bytes"))?,
        );

        verifying_key
            .verify_strict(message.as_bytes(), &signature)
            .context("Policy signature verification failed")?;
    }

    Ok(())
}

pub fn validate_manifest_against_policy(
    manifest: &Manifest,
    policy: &ActivePolicy,
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    let content = &policy.content;

    // Allowed providers/models
    let agent = content.get("agent").and_then(|v| v.as_object());
    if let Some(models) = agent
        .and_then(|a| a.get("models"))
        .and_then(|m| m.as_object())
    {
        if let Some(allowed) = models.get("allowed_models").and_then(|v| v.as_array()) {
            let allowed_models: Vec<String> = allowed
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !allowed_models.is_empty()
                && !allowed_models.iter().any(|m| m == &manifest.identity.model)
            {
                violations.push(format!(
                    "Model {} not allowed by workspace policy",
                    manifest.identity.model
                ));
            }
        }

        if let Some(providers) = models.get("allowed_providers").and_then(|v| v.as_array()) {
            let providers: Vec<String> = providers
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !providers.is_empty()
                && !providers
                    .iter()
                    .any(|p| manifest.identity.model.starts_with(p))
            {
                violations.push(format!(
                    "Model provider for {} not in allowed providers {:?}",
                    manifest.identity.model, providers
                ));
            }
        }
    }

    Ok(violations)
}
