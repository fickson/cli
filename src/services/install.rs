use super::auth::load_auth_session;
use crate::templates::statics as templates;
use crate::ui;
use anyhow::{bail, Result};
use clap::Subcommand;
use sekuire_protocol::Manifest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use tokio::fs;

#[derive(Subcommand)]
pub enum InstallCommands {
    /// Create a workspace-scoped install token for an agent
    Token {
        /// Workspace ID the agent will register to
        #[arg(long)]
        workspace: Option<String>,
        /// Sekuire agent ID (defaults to local manifest hash)
        #[arg(long)]
        agent: Option<String>,
        /// Backend base URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
        /// Optional token TTL in minutes (if supported by backend)
        #[arg(long)]
        ttl_minutes: Option<u32>,
    },
    /// Generate a BYOC Helm install bundle for an agent
    Bundle {
        /// Workspace ID for the installation
        #[arg(long)]
        workspace: String,
        /// Sekuire agent ID (defaults to local manifest hash)
        #[arg(long)]
        agent: Option<String>,
        /// Backend base URL (used to mint install token unless --install-token provided)
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
        /// Output directory for the Helm chart
        #[arg(long, default_value = "deploy/helm")]
        output: String,
        /// Override container image (defaults to deployment.docker_image)
        #[arg(long)]
        image: Option<String>,
        /// Provide an install token manually (skip backend call)
        #[arg(long)]
        install_token: Option<String>,
        /// Kubernetes namespace to target
        #[arg(long, default_value = "default")]
        namespace: String,
        /// Service port exposed by Service
        #[arg(long, default_value_t = 80)]
        service_port: u16,
        /// Container port the agent listens on
        #[arg(long, default_value_t = 8000)]
        container_port: u16,
    },
    /// Install an agent into a workspace (Remote)
    Agent {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
        /// Agent ID (Sekuire ID)
        #[arg(long)]
        agent: String,
        /// Deployment type (managed, byoc)
        #[arg(long, default_value = "managed")]
        deployment_type: String,
        /// The Registry URL
        #[arg(short, long, default_value = "https://api.sekuire.ai")]
        registry: String,
    },
}

#[derive(Serialize, Default)]
pub struct HelmValues {
    #[serde(rename = "workspaceId")]
    workspace_id: String,
    #[serde(rename = "agentId")]
    agent_id: String,
    #[serde(rename = "installToken")]
    install_token: String,
    namespace: String,
    #[serde(rename = "containerPort")]
    container_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "upstreamUrl")]
    upstream_url: Option<String>,
    #[serde(rename = "replicaCount")]
    replica_count: u32,
    image: HelmImageValues,
    service: HelmServiceValues,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    env: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    secrets: BTreeMap<String, String>,
    #[serde(rename = "podAnnotations")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pod_annotations: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct InstallTokenResponse {
    pub token: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_helm_bundle(
    output: &str,
    manifest: &Manifest,
    workspace_id: &str,
    agent_id: &str,
    namespace: &str,
    image_override: Option<&str>,
    install_token: &str,
    service_port: u16,
    container_port: u16,
) -> Result<std::path::PathBuf> {
    let chart_name = sanitize_chart_name(&manifest.project.name);
    let chart_dir = Path::new(output).join(format!("{}-helm", chart_name));
    let templates_dir = chart_dir.join("templates");

    fs::create_dir_all(&templates_dir).await?;

    let image_ref = image_override
        .map(|s| s.to_string())
        .or_else(|| {
            manifest
                .deployment
                .as_ref()
                .and_then(|d| d.docker_image.clone())
        })
        .unwrap_or_else(|| "sekuire/agent:latest".to_string());
    let image_values = parse_image_reference(&image_ref);

    let mut env = BTreeMap::new();
    let mut secrets = BTreeMap::new();
    if let Some(deploy) = manifest.deployment.as_ref() {
        for var in &deploy.env {
            if var.secret {
                secrets.insert(
                    var.name.clone(),
                    var.default_value
                        .clone()
                        .or_else(|| var.example.clone())
                        .unwrap_or_else(|| "CHANGE_ME".to_string()),
                );
            } else {
                env.insert(
                    var.name.clone(),
                    var.default_value
                        .clone()
                        .or_else(|| var.example.clone())
                        .unwrap_or_default(),
                );
            }
        }
    }

    let values = HelmValues {
        workspace_id: workspace_id.to_string(),
        agent_id: agent_id.to_string(),
        install_token: install_token.to_string(),
        namespace: namespace.to_string(),
        container_port,
        upstream_url: manifest.a2a.as_ref().map(|a| a.upstream_url.clone()),
        replica_count: 1,
        image: image_values,
        service: HelmServiceValues {
            enabled: true,
            service_type: "ClusterIP".to_string(),
            port: service_port,
            target_port: container_port,
        },
        env,
        secrets,
        pod_annotations: BTreeMap::new(),
        resources: Some(serde_json::json!({})),
    };

    let mut values_yaml = serde_yaml::to_string(&values)?;
    values_yaml = format!(
        "# Generated by sekuire install bundle\n# Edit secrets before deploying\n{}",
        values_yaml
    );

    let chart_yaml = templates::helm_chart_yaml(&chart_name, &manifest.project.version);
    let helpers_tpl = templates::helm_helpers_tpl(&chart_name);
    let deployment_yaml = templates::helm_deployment_yaml(&chart_name);
    let service_yaml = templates::helm_service_yaml(&chart_name);
    let configmap_yaml = templates::helm_configmap_yaml(&chart_name);
    let secret_yaml = templates::helm_secret_yaml(&chart_name);
    let readme = templates::helm_readme(&chart_name, workspace_id, agent_id);

    fs::write(chart_dir.join("Chart.yaml"), chart_yaml).await?;
    fs::write(chart_dir.join("values.yaml"), values_yaml).await?;
    fs::write(templates_dir.join("_helpers.tpl"), helpers_tpl).await?;
    fs::write(templates_dir.join("deployment.yaml"), deployment_yaml).await?;
    fs::write(templates_dir.join("service.yaml"), service_yaml).await?;
    fs::write(templates_dir.join("configmap.yaml"), configmap_yaml).await?;
    fs::write(templates_dir.join("secret.yaml"), secret_yaml).await?;
    fs::write(chart_dir.join("README.md"), readme).await?;

    Ok(chart_dir)
}

pub async fn create_install_token(
    backend: &str,
    workspace_id: &str,
    agent_id: &str,
    ttl_minutes: Option<u32>,
) -> Result<InstallTokenResponse> {
    let auth = load_auth_session().await?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/workspaces/{}/install-tokens",
        backend.trim_end_matches('/'),
        workspace_id
    );

    let mut body = serde_json::json!({
        "agent_id": agent_id,
        "sekuire_id": agent_id,
    });
    if let Some(ttl) = ttl_minutes {
        body["expiry_minutes"] = serde_json::json!(ttl);
    }

    let mut req = client.post(&url).json(&body);
    if let Some(key) = auth.api_key.as_ref() {
        req = req.header("X-API-Key", key);
        if let Some(fp) = auth.device_fingerprint.as_ref() {
            req = req.header("X-Device-Fingerprint", fp);
        }
    } else if let Some(token) = auth.token.as_ref() {
        req = req.bearer_auth(token);
    } else {
        bail!("Not logged in. Run 'sekuire login' to create install tokens.");
    }

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Creating install token...",
        async { req.send().await },
    )
    .await?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        bail!("Authentication failed. Run 'sekuire login' and try again.");
    }

    let token: InstallTokenResponse = resp.error_for_status()?.json().await?;
    Ok(token)
}

pub async fn bootstrap_installation(
    backend: &str,
    install_token: &str,
    upstream_url: &str,
    capabilities: &[String],
    heartbeat_interval: i64,
) -> Result<BootstrapResponse> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/installations/bootstrap",
        backend.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "install_token": install_token,
        "upstream_url": upstream_url,
        "capabilities": capabilities,
        "heartbeat_interval_seconds": heartbeat_interval,
    });

    let resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Bootstrapping installation...",
        async { client.post(&url).json(&body).send().await },
    )
    .await?;
    if resp.status().is_success() {
        let payload: BootstrapResponse = resp.json().await?;
        return Ok(payload);
    }

    let status = resp.status();
    let body = resp.text().await.unwrap_or_else(|_| "".to_string());
    bail!("Bootstrap failed: {} {}", status, body);
}

pub async fn send_heartbeat(
    backend: &str,
    installation_id: &str,
    runtime_token: &str,
    heartbeat_interval: i64,
) -> Result<Option<String>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/installations/{}/lease",
        backend.trim_end_matches('/'),
        installation_id
    );
    let body = serde_json::json!({
        "status": "running",
        "heartbeat_interval_seconds": heartbeat_interval,
    });
    let resp = client
        .post(&url)
        .bearer_auth(runtime_token)
        .json(&body)
        .send()
        .await?;

    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        let pending = json["pending_command"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        return Ok(pending);
    }

    let status = resp.status();
    let body = resp.text().await.unwrap_or_else(|_| "".to_string());
    bail!("Heartbeat failed: {} {}", status, body);
}

pub async fn install_agent_remote(
    registry: &str,
    workspace_id: &str,
    agent_id: &str,
    deployment_type: &str,
    api_key: Option<String>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/agents/{}/install", registry, agent_id);

    let payload = serde_json::json!({
        "workspace_id": workspace_id,
        "deployment_type": deployment_type,
        "config": {}
    });

    let mut req = client.post(&url).json(&payload);

    if let Some(key) = api_key {
        req = req.header("X-API-Key", key);
    }

    let res = ui::with_spinner_if(ui::spinner_enabled(None), "Installing agent...", async {
        req.send().await
    })
    .await?;

    if res.status().is_success() {
        let json: serde_json::Value = res.json().await?;
        println!("✅ Agent installed successfully!");
        println!(
            "   Installation ID: {}",
            json["installation_id"].as_str().unwrap_or("unknown")
        );
        println!(
            "   Status: {}",
            json["runtime_status"].as_str().unwrap_or("unknown")
        );
        if let Some(url) = json["upstream_url"].as_str() {
            println!("   Upstream URL: {}", url);
        }
    } else {
        let status = res.status();
        let text = res.text().await?;
        anyhow::bail!("Failed to install agent: Status {} - {}", status, text);
    }

    Ok(())
}

pub fn sanitize_chart_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.chars() {
        let lc = c.to_ascii_lowercase();
        if lc.is_ascii_alphanumeric() {
            out.push(lc);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "sekuire-agent".to_string()
    } else {
        trimmed
    }
}

#[derive(Serialize, Default)]
struct HelmImageValues {
    repository: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
    #[serde(rename = "pullPolicy")]
    pull_policy: String,
}

fn parse_image_reference(image: &str) -> HelmImageValues {
    if let Some((repo, digest)) = image.split_once('@') {
        return HelmImageValues {
            repository: repo.to_string(),
            tag: None,
            digest: Some(digest.to_string()),
            pull_policy: "IfNotPresent".to_string(),
        };
    }

    if let Some(pos) = image.rfind(':') {
        if image[pos + 1..].contains('/') {
            return HelmImageValues {
                repository: image.to_string(),
                tag: None,
                digest: None,
                pull_policy: "IfNotPresent".to_string(),
            };
        }

        return HelmImageValues {
            repository: image[..pos].to_string(),
            tag: Some(image[pos + 1..].to_string()),
            digest: None,
            pull_policy: "IfNotPresent".to_string(),
        };
    }

    HelmImageValues {
        repository: image.to_string(),
        tag: None,
        digest: None,
        pull_policy: "IfNotPresent".to_string(),
    }
}

#[derive(Serialize, Default)]
struct HelmServiceValues {
    enabled: bool,
    #[serde(rename = "type")]
    service_type: String,
    port: u16,
    #[serde(rename = "targetPort")]
    target_port: u16,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BootstrapResponse {
    pub installation_id: String,
    pub runtime_token: String,
    refresh_token: String,
    expires_at: String,
    heartbeat_interval: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_chart_name_basic() {
        assert_eq!(sanitize_chart_name("my-agent"), "my-agent");
    }

    #[test]
    fn sanitize_chart_name_uppercase() {
        assert_eq!(sanitize_chart_name("MyAgent"), "myagent");
    }

    #[test]
    fn sanitize_chart_name_spaces_and_special() {
        assert_eq!(sanitize_chart_name("My Cool Agent!"), "my-cool-agent");
    }

    #[test]
    fn sanitize_chart_name_consecutive_special() {
        assert_eq!(sanitize_chart_name("foo---bar"), "foo-bar");
    }

    #[test]
    fn sanitize_chart_name_leading_trailing_dash() {
        assert_eq!(sanitize_chart_name("--agent--"), "agent");
    }

    #[test]
    fn sanitize_chart_name_empty() {
        assert_eq!(sanitize_chart_name(""), "sekuire-agent");
    }

    #[test]
    fn sanitize_chart_name_only_special() {
        assert_eq!(sanitize_chart_name("!!!"), "sekuire-agent");
    }

    #[test]
    fn parse_image_with_tag() {
        let img = parse_image_reference("nginx:1.25");
        assert_eq!(img.repository, "nginx");
        assert_eq!(img.tag.as_deref(), Some("1.25"));
        assert!(img.digest.is_none());
    }

    #[test]
    fn parse_image_with_digest() {
        let img = parse_image_reference("nginx@sha256:abc123");
        assert_eq!(img.repository, "nginx");
        assert!(img.tag.is_none());
        assert_eq!(img.digest.as_deref(), Some("sha256:abc123"));
    }

    #[test]
    fn parse_image_no_tag() {
        let img = parse_image_reference("my-registry/my-image");
        assert_eq!(img.repository, "my-registry/my-image");
        assert!(img.tag.is_none());
        assert!(img.digest.is_none());
    }

    #[test]
    fn parse_image_registry_with_port() {
        let img = parse_image_reference("registry.example.com:5000/myimage");
        assert_eq!(img.repository, "registry.example.com:5000/myimage");
        assert!(img.tag.is_none());
    }

    #[test]
    fn parse_image_registry_with_port_and_tag() {
        let img = parse_image_reference("registry.example.com:5000/myimage:v2");
        assert_eq!(img.repository, "registry.example.com:5000/myimage");
        assert_eq!(img.tag.as_deref(), Some("v2"));
    }
}
