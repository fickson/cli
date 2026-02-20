use crate::config;
use crate::error::CliError;
use crate::templates::engine::TemplateEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::path::Path;
use tokio::fs;

#[derive(Debug)]
#[allow(dead_code)]
struct ProjectDetection {
    language: DetectedLanguage,
    web_framework: Option<String>,
    ai_framework: Option<AiFramework>,
    has_manifest: bool,
    has_dockerfile: bool,
}

#[derive(Debug)]
enum DetectedLanguage {
    Python,
    TypeScript,
    Rust,
    Unknown,
}

#[derive(Debug)]
enum AiFramework {
    LangChain,
    CrewAI,
    AutoGen,
    DirectApi(String),
}

impl std::fmt::Display for DetectedLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Python => write!(f, "Python"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Rust => write!(f, "Rust"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::fmt::Display for AiFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LangChain => write!(f, "LangChain"),
            Self::CrewAI => write!(f, "CrewAI"),
            Self::AutoGen => write!(f, "AutoGen"),
            Self::DirectApi(provider) => write!(f, "Direct API ({})", provider),
        }
    }
}

pub async fn execute(name: Option<String>, model: Option<String>) -> Result<(), CliError> {
    let dir = std::env::current_dir().map_err(CliError::Io)?;
    println!("Scanning project at {}...", dir.display());

    let detection = detect_project(&dir).await?;

    println!("   Language:      {}", detection.language);
    if let Some(ref fw) = detection.web_framework {
        println!("   Web Framework: {}", fw);
    }
    if let Some(ref ai) = detection.ai_framework {
        println!("   AI Framework:  {}", ai);
    }
    if detection.has_manifest {
        println!("   sekuire.yml already exists - updating.");
    }

    let agent_name = name.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-agent")
            .to_string()
    });

    let model_name = model.unwrap_or_else(|| "gemini-3.1-pro-preview".to_string());

    // Generate sekuire.yml
    let manifest_content = generate_manifest(&agent_name, &model_name, &detection);
    fs::write(dir.join(config::MANIFEST_YAML), &manifest_content).await?;
    println!("\n   Created sekuire.yml");

    // Generate keypair if not exists
    let sekuire_dir = dir.join(config::SEKUIRE_DIR);
    if !sekuire_dir.exists() {
        fs::create_dir_all(&sekuire_dir).await?;
    }

    let key_path = sekuire_dir.join(config::SECRET_KEY_FILE);
    let pub_path = sekuire_dir.join(config::PUBLIC_KEY_FILE);

    if !key_path.exists() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        fs::write(&key_path, hex::encode(signing_key.to_bytes())).await?;
        fs::write(&pub_path, hex::encode(verifying_key.as_bytes())).await?;
        println!("   Created .sekuire/secret.key and .sekuire/public.key");
    } else {
        println!("   Keypair already exists (.sekuire/)");
    }

    // Generate .env.example
    let env_path = dir.join(".env.example");
    if !env_path.exists() {
        let env_content = generate_env_example(&detection);
        fs::write(&env_path, env_content).await?;
        println!("   Created .env.example");
    }

    // Compute identity hash
    println!();
    let hash = compute_sekuire_id(&model_name);
    println!("   Sekuire ID: {}", hash);

    // Print integration snippet
    println!();
    print_integration_snippet(&detection)?;

    println!("\nAdoption complete. Your agent is ready for registration.");
    println!("   Next: sekuire push");
    Ok(())
}

async fn detect_project(dir: &Path) -> Result<ProjectDetection, CliError> {
    let has_manifest =
        dir.join(config::MANIFEST_YAML).exists() || dir.join(config::MANIFEST_JSON).exists();
    let has_dockerfile = dir.join("Dockerfile").exists();

    if dir.join("package.json").exists() {
        let content = fs::read_to_string(dir.join("package.json")).await?;
        let pkg: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::Value::Null);
        let deps = collect_js_deps(&pkg);

        let web_framework = detect_js_web_framework(&deps);
        let ai_framework = detect_js_ai_framework(&deps);

        return Ok(ProjectDetection {
            language: DetectedLanguage::TypeScript,
            web_framework,
            ai_framework,
            has_manifest,
            has_dockerfile,
        });
    }

    if dir.join("requirements.txt").exists() || dir.join("pyproject.toml").exists() {
        let deps = collect_python_deps(dir).await;
        let web_framework = detect_python_web_framework(&deps);
        let ai_framework = detect_python_ai_framework(&deps);

        return Ok(ProjectDetection {
            language: DetectedLanguage::Python,
            web_framework,
            ai_framework,
            has_manifest,
            has_dockerfile,
        });
    }

    if dir.join("Cargo.toml").exists() {
        let content = fs::read_to_string(dir.join("Cargo.toml")).await?;
        let web_framework = if content.contains("axum") {
            Some("Axum".to_string())
        } else if content.contains("actix") {
            Some("Actix".to_string())
        } else {
            None
        };

        return Ok(ProjectDetection {
            language: DetectedLanguage::Rust,
            web_framework,
            ai_framework: None,
            has_manifest,
            has_dockerfile,
        });
    }

    Ok(ProjectDetection {
        language: DetectedLanguage::Unknown,
        web_framework: None,
        ai_framework: None,
        has_manifest,
        has_dockerfile,
    })
}

fn collect_js_deps(pkg: &serde_json::Value) -> Vec<String> {
    let mut deps = Vec::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = pkg[key].as_object() {
            for k in obj.keys() {
                deps.push(k.clone());
            }
        }
    }
    deps
}

fn detect_js_web_framework(deps: &[String]) -> Option<String> {
    if deps.iter().any(|d| d == "express") {
        Some("Express".to_string())
    } else if deps.iter().any(|d| d == "fastify") {
        Some("Fastify".to_string())
    } else if deps.iter().any(|d| d == "hono") {
        Some("Hono".to_string())
    } else {
        None
    }
}

fn detect_js_ai_framework(deps: &[String]) -> Option<AiFramework> {
    if deps.iter().any(|d| d.contains("langchain")) {
        Some(AiFramework::LangChain)
    } else if deps.iter().any(|d| d == "openai") {
        Some(AiFramework::DirectApi("OpenAI".to_string()))
    } else if deps.iter().any(|d| d == "@anthropic-ai/sdk") {
        Some(AiFramework::DirectApi("Anthropic".to_string()))
    } else if deps
        .iter()
        .any(|d| d.contains("google-generativeai") || d.contains("@google/generative-ai"))
    {
        Some(AiFramework::DirectApi("Google".to_string()))
    } else {
        None
    }
}

async fn collect_python_deps(dir: &Path) -> Vec<String> {
    let mut deps = Vec::new();
    if let Ok(content) = fs::read_to_string(dir.join("requirements.txt")).await {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let pkg = line
                    .split(&['=', '>', '<', '!', '[', ';'][..])
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();
                if !pkg.is_empty() {
                    deps.push(pkg);
                }
            }
        }
    }
    if let Ok(content) = fs::read_to_string(dir.join("pyproject.toml")).await {
        for line in content.lines() {
            let trimmed = line.trim().trim_matches('"').trim_matches('\'');
            let pkg = trimmed
                .split(&['=', '>', '<', '!', '[', ';'][..])
                .next()
                .unwrap_or("")
                .trim()
                .to_lowercase();
            if !pkg.is_empty()
                && pkg
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                deps.push(pkg);
            }
        }
    }
    deps
}

fn detect_python_web_framework(deps: &[String]) -> Option<String> {
    if deps.iter().any(|d| d == "fastapi") {
        Some("FastAPI".to_string())
    } else if deps.iter().any(|d| d == "django") {
        Some("Django".to_string())
    } else if deps.iter().any(|d| d == "flask") {
        Some("Flask".to_string())
    } else {
        None
    }
}

fn detect_python_ai_framework(deps: &[String]) -> Option<AiFramework> {
    if deps
        .iter()
        .any(|d| d == "langchain" || d == "langchain-core")
    {
        Some(AiFramework::LangChain)
    } else if deps.iter().any(|d| d == "crewai") {
        Some(AiFramework::CrewAI)
    } else if deps.iter().any(|d| d == "autogen" || d == "pyautogen") {
        Some(AiFramework::AutoGen)
    } else if deps.iter().any(|d| d == "openai") {
        Some(AiFramework::DirectApi("OpenAI".to_string()))
    } else if deps.iter().any(|d| d == "anthropic") {
        Some(AiFramework::DirectApi("Anthropic".to_string()))
    } else if deps.iter().any(|d| d == "google-generativeai") {
        Some(AiFramework::DirectApi("Google".to_string()))
    } else {
        None
    }
}

fn generate_manifest(name: &str, model: &str, detection: &ProjectDetection) -> String {
    let language = match detection.language {
        DetectedLanguage::Python => "python",
        DetectedLanguage::TypeScript => "typescript",
        DetectedLanguage::Rust => "rust",
        DetectedLanguage::Unknown => "python",
    };

    format!(
        r#"# Sekuire Agent Manifest (adopted from existing project)
name: "{name}"
version: "0.1.0"

project:
  name: "{name}"
  version: "0.1.0"
  language: {language}

identity:
  model: "{model}"
  system_prompt: "system_prompt.md"
  tools: "tools.json"

capabilities:
  provides:
    - "chat"
"#,
        name = name,
        language = language,
        model = model,
    )
}

fn generate_env_example(detection: &ProjectDetection) -> String {
    let mut lines = vec![
        "# Sekuire Configuration".to_string(),
        "SEKUIRE_API_URL=https://api.sekuire.ai".to_string(),
        "SEKUIRE_AGENT_ID=<from sekuire hash>".to_string(),
        "SEKUIRE_PRIVATE_KEY=<from .sekuire/secret.key>".to_string(),
        "".to_string(),
        "# LLM Provider".to_string(),
    ];

    match &detection.ai_framework {
        Some(AiFramework::DirectApi(provider)) => match provider.as_str() {
            "OpenAI" => lines.push("OPENAI_API_KEY=".to_string()),
            "Anthropic" => lines.push("ANTHROPIC_API_KEY=".to_string()),
            "Google" => lines.push("GOOGLE_API_KEY=".to_string()),
            _ => lines.push("LLM_API_KEY=".to_string()),
        },
        Some(AiFramework::LangChain) => {
            lines.push("OPENAI_API_KEY=".to_string());
            lines.push("# ANTHROPIC_API_KEY=".to_string());
        }
        _ => {
            lines.push("OPENAI_API_KEY=".to_string());
        }
    }

    lines.join("\n") + "\n"
}

fn compute_sekuire_id(model: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(model.to_lowercase().trim().as_bytes());
    hasher.update(blake3::hash(b"").as_bytes());
    hasher.update(blake3::hash(b"[]").as_bytes());
    let hash = hasher.finalize().to_hex();
    if hash.len() >= 16 {
        hash[..16].to_string()
    } else {
        hash.to_string()
    }
}

fn print_integration_snippet(detection: &ProjectDetection) -> Result<(), CliError> {
    let engine = TemplateEngine::new()?;

    let language = match detection.language {
        DetectedLanguage::Python => "python",
        DetectedLanguage::TypeScript => "typescript",
        DetectedLanguage::Rust => "rust",
        DetectedLanguage::Unknown => "unknown",
    };

    let ai_framework = match &detection.ai_framework {
        Some(AiFramework::LangChain) => "langchain",
        Some(AiFramework::CrewAI) => "crewai",
        Some(AiFramework::AutoGen) => "autogen",
        Some(AiFramework::DirectApi(_)) => "direct",
        None => "none",
    };

    #[derive(serde::Serialize)]
    struct AdoptContext {
        language: String,
        ai_framework: String,
    }

    let ctx = AdoptContext {
        language: language.to_string(),
        ai_framework: ai_framework.to_string(),
    };

    let output = engine.render("adopt/integration.tera", &ctx)?;
    print!("{}", output);
    Ok(())
}
