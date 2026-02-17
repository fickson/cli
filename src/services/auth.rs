use crate::services::device;
use crate::ui;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use blake3::Hasher;
use chrono::Utc;
use clap::Subcommand;
use dialoguer::{theme::ColorfulTheme, Input};
use sekuire_protocol::Manifest;
use std::path::Path;
use tokio::fs;

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Login with API key
    Login {
        /// The web app URL
        #[arg(long, default_value = "https://sekuire.ai")]
        web_url: String,
        /// Backend API URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend: String,
    },
    /// Logout (remove credentials)
    Logout,
    /// Check authentication status
    Status,
    /// Show current user info
    Whoami {
        /// The backend URL
        #[arg(long, default_value = "https://api.sekuire.ai")]
        backend_url: String,
    },
}

pub struct AuthSession {
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub device_fingerprint: Option<String>,
    pub workspace_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuthConfig {
    pub api_key: Option<String>,            // NEW: Primary auth method
    pub token: Option<String>,              // LEGACY: For backward compatibility
    pub device_fingerprint: Option<String>, // NEW: For validation
    pub org_id: Option<String>,             // NEW: Default org for publishing
    pub workspace_id: Option<String>,       // NEW: Default workspace for install tokens
}

pub async fn handle_status() -> Result<()> {
    println!("🔍 Authentication Status:");

    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    if auth_path.exists() {
        let content = fs::read_to_string(auth_path).await?;
        let config: AuthConfig = serde_json::from_str(&content)?;

        if let Some(api_key) = config.api_key {
            println!("   ✅ Authenticated with API Key");
            let key_len = api_key.len();
            if key_len > 16 {
                println!(
                    "   🔑 Key: {}...{}",
                    &api_key[0..12],
                    &api_key[key_len - 4..]
                );
            } else {
                println!("   🔑 Key: {}...", &api_key[0..8]);
            }

            if let Some(fp) = config.device_fingerprint {
                println!("   💻 Device Fingerprint: {}...", &fp[0..16]);
            }

            println!("   📍 Device: {}", device::get_device_name());
            println!("   🖥️  OS: {}", device::get_device_os());
            if let Some(workspace_id) = config.workspace_id {
                println!("   🧭 Workspace: {}", workspace_id);
            }
        } else if let Some(token) = config.token {
            println!("   ⚠️  Using legacy token authentication");
            println!("   💡 Run 'sekuire login' to upgrade to API key authentication");

            // Try to decode JWT to show expiry
            if let Some(payload_part) = token.split('.').nth(1) {
                if let Ok(decoded) = STANDARD_NO_PAD.decode(payload_part) {
                    if let Ok(json_str) = String::from_utf8(decoded) {
                        if let Ok(claims) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
                                let now = Utc::now().timestamp();
                                if now > exp {
                                    println!("   ❌ Token expired - please run 'sekuire login'");
                                } else {
                                    let expires_in = exp - now;
                                    println!("   ⏰ Token expires in: {} minutes", expires_in / 60);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("   ❌ Not logged in");
            println!("   💡 Run 'sekuire login' to authenticate");
        }
    } else {
        println!("   ❌ Not logged in");
        println!("   💡 Run 'sekuire login' to authenticate");
    }

    Ok(())
}

pub async fn handle_whoami(backend_url: &str) -> Result<()> {
    println!("👤 Current User Information:");

    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    if !auth_path.exists() {
        println!("   ❌ Not logged in");
        println!("   💡 Run 'sekuire login' to authenticate");
        return Ok(());
    }

    let content = fs::read_to_string(auth_path).await?;
    let config: AuthConfig = serde_json::from_str(&content)?;

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/me", backend_url);

    let mut req = client.get(&url);

    // Use API key if available
    if let Some(ref key) = config.api_key {
        req = req.header("X-API-Key", key);
        if let Some(ref fp) = config.device_fingerprint {
            req = req.header("X-Device-Fingerprint", fp);
        }
    } else if let Some(ref token) = config.token {
        req = req.bearer_auth(token);
    } else {
        println!("   ❌ No valid authentication found");
        return Ok(());
    }

    let response = ui::with_spinner_if(ui::spinner_enabled(None), "Fetching user info...", async {
        req.send().await
    })
    .await?;

    if response.status().is_success() {
        let user: serde_json::Value = response.json().await?;

        println!("   ✅ Authenticated as:");
        if let Some(email) = user.get("email").and_then(|v| v.as_str()) {
            println!("   📧 Email: {}", email);
        }
        if let Some(id) = user.get("id").and_then(|v| v.as_str()) {
            println!("   🆔 User ID: {}", id);
        }
        if let Some(org) = user.get("organization").and_then(|v| v.as_str()) {
            println!("   🏢 Organization: {}", org);
        }

        println!("\n   💻 Device Info:");
        println!("   📍 Device: {}", device::get_device_name());
        println!("   🖥️  OS: {}", device::get_device_os());
        if let Some(fp) = config.device_fingerprint {
            println!("   🔐 Fingerprint: {}...", &fp[0..16]);
        }
    } else {
        eprintln!("   ❌ Failed to fetch user info: {}", response.status());
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("   {}", error_text);
    }

    Ok(())
}

pub async fn calculate_identity_hash() -> Result<(String, Manifest)> {
    // Try YAML first (preferred), then JSON (legacy)
    let (content, format) = if Path::new("sekuire.yml").exists() {
        (fs::read_to_string("sekuire.yml").await?, "yaml")
    } else if Path::new("sekuire.yaml").exists() {
        (fs::read_to_string("sekuire.yaml").await?, "yaml")
    } else if Path::new("sekuire.json").exists() {
        (fs::read_to_string("sekuire.json").await?, "json")
    } else {
        eprintln!("\n❌ Error: Configuration file not found\n");
        eprintln!("   Expected: sekuire.yml, sekuire.yaml, or sekuire.json");
        eprintln!(
            "   Current directory: {}\n",
            std::env::current_dir()?.display()
        );
        eprintln!("💡 Fix: Run 'sekuire init' to create a new agent project\n");
        return Err(anyhow::anyhow!("Configuration file not found"));
    };

    let mut manifest: Manifest = match format {
        "yaml" => serde_yaml::from_str(&content)
            .context("Failed to parse sekuire.yml. Check YAML syntax.")?,
        "json" => serde_json::from_str(&content)
            .context("Failed to parse sekuire.json. Check JSON syntax.")?,
        _ => unreachable!(),
    };

    // 1. Hash System Prompt
    let prompt_path = Path::new(&manifest.identity.system_prompt_path);
    if !prompt_path.exists() {
        return Err(anyhow::anyhow!(
            "System prompt file not found: {:?}",
            prompt_path
        ));
    }
    let prompt_content = fs::read_to_string(prompt_path).await?;
    let prompt_hash = blake3::hash(prompt_content.trim().as_bytes())
        .to_hex()
        .to_string();
    manifest.identity.system_prompt_hash = Some(prompt_hash.clone());

    // 2. Hash Tools
    let tools_path = Path::new(&manifest.identity.tools_path);
    if !tools_path.exists() {
        return Err(anyhow::anyhow!("Tools file not found: {:?}", tools_path));
    }
    let tools_content = fs::read_to_string(tools_path).await?;
    // Canonicalize JSON
    let tools_json: serde_json::Value = serde_json::from_str(&tools_content)?;
    let canonical_tools = serde_json::to_string(&tools_json)?;
    let tools_hash = blake3::hash(canonical_tools.as_bytes())
        .to_hex()
        .to_string();
    manifest.identity.tools_hash = Some(tools_hash.clone());

    // 3. Compute Final Sekuire ID
    let mut hasher = Hasher::new();
    let fingerprint_str = format!(
        "model:{}|prompt:{}|tools:{}",
        manifest.identity.model, prompt_hash, tools_hash
    );
    hasher.update(fingerprint_str.as_bytes());
    let sekuire_id = hasher.finalize().to_hex().to_string();

    Ok((sekuire_id, manifest))
}

pub async fn handle_logout() -> Result<()> {
    println!("🚪 Logging out...");

    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    if auth_path.exists() {
        fs::remove_file(&auth_path).await?;
        println!("✅ Logged out successfully!");
        println!("   Credentials removed from: {}", auth_path.display());
    } else {
        println!("   ℹ️  No active session found");
    }

    Ok(())
}

// Authentication handler functions
pub async fn handle_login(web_url: &str, backend: &str) -> Result<()> {
    println!("🔐 Sekuire Login");
    println!("\n📋 To authenticate:");
    println!("   1. Go to: {}/dashboard/settings?tab=api-keys", web_url);
    println!("   2. Create a new API Key for CLI access");
    println!("   3. Copy the key (you'll only see it once!)");
    println!("   4. Paste it below\n");

    // Prompt for API key
    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your API Key")
        .interact_text()?;

    // Get device info
    let device_fingerprint = device::get_device_fingerprint();
    let device_name = device::get_device_name();
    let device_os = device::get_device_os();

    // Verify key with backend
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

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;

        if result["valid"].as_bool().unwrap_or(false) {
            let org_id = result
                .get("org_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Save credentials
            let home = dirs::home_dir().context("Could not find home directory")?;
            let config_dir = home.join(".sekuire");
            if !config_dir.exists() {
                fs::create_dir_all(&config_dir).await?;
            }

            let existing_workspace_id = if config_dir.join("auth.json").exists() {
                let content = fs::read_to_string(config_dir.join("auth.json")).await?;
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

            fs::write(
                config_dir.join("auth.json"),
                serde_json::to_string_pretty(&auth_config)?,
            )
            .await?;

            println!("✅ Login successful! API Key verified and bound to this device.");
            println!("   Device: {}", device_name);
            println!("   OS: {}", device_os);
            println!("   Fingerprint: {}...", &device_fingerprint[0..16]);
        } else {
            let msg = if let Some(error) = result["error"].as_str() {
                format!("Invalid API Key or verification failed: {}", error)
            } else {
                "Invalid API Key or verification failed".to_string()
            };
            bail!(msg);
        }
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        bail!("Verification failed ({}): {}", status, error_text);
    }

    Ok(())
}

pub async fn load_auth_session() -> Result<AuthSession> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let auth_path = home.join(".sekuire/auth.json");

    if !auth_path.exists() {
        bail!("Not logged in. Run 'sekuire login' first.");
    }

    let content = fs::read_to_string(&auth_path).await?;
    let mut config: AuthConfig = serde_json::from_str(&content)?;

    if config.device_fingerprint.is_none() {
        config.device_fingerprint = Some(device::get_device_fingerprint());
        fs::write(
            &auth_path,
            serde_json::to_string_pretty(&config)
                .context("Failed to serialize auth config with device fingerprint")?,
        )
        .await
        .context("Failed to persist updated auth config")?;
    }

    Ok(AuthSession {
        api_key: config.api_key,
        token: config.token,
        device_fingerprint: config.device_fingerprint,
        workspace_id: config.workspace_id,
    })
}

pub fn redact_token(token: &str) -> String {
    if token.len() <= 8 {
        return "***".to_string();
    }
    format!("***{}", &token[token.len() - 4..])
}
