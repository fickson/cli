use crate::error::CliError;
use crate::services::policy::fetch_active_policy;
use crate::templates::engine::{
    DockerContext, InitContext, LlmVariantContext, PackageContext, TemplateEngine,
};
use crate::templates::registry::{Framework, Language};
use crate::templates::statics::{
    get_calculator_tool_ts, get_claude_prompt, get_gemini_prompt, get_http_server_template,
    get_ollama_prompt, get_openai_prompt, get_python_http_server_template,
    get_rust_http_server_template, get_search_tool_ts, get_system_prompt, get_tools_json,
    PYTHON_AGENT_SHARED, PYTHON_CLI_INTERFACE, PYTHON_HYBRID_MAIN, PYTHON_MAIN,
    PYTHON_MAIN_MULTI_LLM, PYTHON_REQUIREMENTS, PYTHON_REQUIREMENTS_MULTI_LLM, RUST_AGENT_SHARED,
    RUST_CLI_INTERFACE, RUST_HYBRID_MAIN, RUST_MAIN, RUST_MAIN_MULTI_LLM, TS_AGENT_SHARED,
    TS_CLI_INTERFACE, TS_CONFIG, TS_HYBRID_INDEX, TS_INDEX, TS_INDEX_MULTI_LLM,
};
use crate::ui;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Deserialize;
use std::{collections::HashSet, path::Path, time::Duration};
use tokio::fs;

#[derive(Debug, Clone, Copy)]
struct LlmProviderOption {
    key: &'static str,
    provider: &'static str,
    display_name: &'static str,
    agent_suffix: &'static str,
    api_key_env: &'static str,
    api_key_example: &'static str,
    requires_api_key: bool,
    base_url: Option<&'static str>,
    temperature: f32,
    max_tokens: u32,
    default_models: &'static [&'static str],
}

#[derive(Debug, Clone)]
struct LlmSelection {
    option: LlmProviderOption,
    model: String,
}

const OPENAI_MODELS: &[&str] = &["gpt-5.2", "gpt-5-mini", "gpt-5-nano"];

const CLAUDE_MODELS: &[&str] = &["claude-sonnet-4-6", "claude-opus-4-6"];

const GEMINI_MODELS: &[&str] = &[
    "gemini-3.1-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-pro",
];

const OLLAMA_MODELS: &[&str] = &[
    "qwen3",
    "deepseek-r1",
    "llama3.1",
    "gemma3",
    "mistral-small3.1",
];

const LLM_PROVIDERS: [LlmProviderOption; 4] = [
    LlmProviderOption {
        key: "openai",
        provider: "openai",
        display_name: "OpenAI",
        agent_suffix: "openai",
        api_key_env: "OPENAI_API_KEY",
        api_key_example: "sk-...",
        requires_api_key: true,
        base_url: None,
        temperature: 0.7,
        max_tokens: 2000,
        default_models: OPENAI_MODELS,
    },
    LlmProviderOption {
        key: "claude",
        provider: "anthropic",
        display_name: "Claude",
        agent_suffix: "claude",
        api_key_env: "ANTHROPIC_API_KEY",
        api_key_example: "sk-ant-...",
        requires_api_key: true,
        base_url: None,
        temperature: 0.7,
        max_tokens: 2000,
        default_models: CLAUDE_MODELS,
    },
    LlmProviderOption {
        key: "gemini",
        provider: "google",
        display_name: "Google Gemini",
        agent_suffix: "gemini",
        api_key_env: "GOOGLE_API_KEY",
        api_key_example: "...",
        requires_api_key: true,
        base_url: None,
        temperature: 0.5,
        max_tokens: 2000,
        default_models: GEMINI_MODELS,
    },
    LlmProviderOption {
        key: "ollama",
        provider: "ollama",
        display_name: "Ollama (Local)",
        agent_suffix: "local",
        api_key_env: "OLLAMA_API_KEY",
        api_key_example: "",
        requires_api_key: false,
        base_url: Some("http://localhost:11434"),
        temperature: 0.6,
        max_tokens: 2000,
        default_models: OLLAMA_MODELS,
    },
];

const DEFAULT_PROVIDER_KEY: &str = "gemini";

fn provider_index(key: &str) -> usize {
    LLM_PROVIDERS.iter().position(|p| p.key == key).unwrap_or(0)
}

fn pick_prompt(key: &str) -> &'static str {
    match key {
        "openai" => get_openai_prompt(),
        "claude" => get_claude_prompt(),
        "gemini" => get_gemini_prompt(),
        "ollama" => get_ollama_prompt(),
        _ => get_system_prompt(),
    }
}

fn add_models(models: &mut Vec<String>, additions: Vec<String>) {
    for model in additions {
        if !models.iter().any(|m| m.eq_ignore_ascii_case(&model)) {
            models.push(model);
        }
    }
}

fn dedupe_preserve_order(models: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for model in models {
        let normalized = model.to_lowercase();
        if seen.insert(normalized) {
            deduped.push(model);
        }
    }
    deduped
}

fn filter_openai_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.starts_with("gpt-5")
        && !lower.contains("audio")
        && !lower.contains("image")
        && !lower.contains("embedding")
        && !lower.contains("moderation")
        && !lower.contains("whisper")
        && !lower.contains("tts")
}

fn filter_anthropic_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.starts_with("claude-sonnet-4-6") || lower.starts_with("claude-opus-4-6")
}

fn filter_google_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.starts_with("gemini-3")
        && !lower.contains("embedding")
        && !lower.contains("aqa")
        && !lower.contains("imagen")
        && !lower.contains("veo")
        && !lower.contains("customtools")
        && !lower.contains("image")
        && !lower.contains("robotics")
        && !lower.contains("deep-research")
        && !lower.contains("computer-use")
}

#[derive(Deserialize)]
struct GenericModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[derive(Deserialize)]
struct GoogleModelsResponse {
    models: Vec<GoogleModelEntry>,
}

#[derive(Deserialize)]
struct GoogleModelEntry {
    name: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

async fn fetch_openai_models(client: &reqwest::Client) -> Result<Vec<String>, CliError> {
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => return Ok(Vec::new()),
    };

    let response: GenericModelsResponse = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let models = response
        .data
        .into_iter()
        .map(|entry| entry.id)
        .filter(|id| filter_openai_model(id))
        .collect();
    Ok(dedupe_preserve_order(models))
}

async fn fetch_anthropic_models(client: &reqwest::Client) -> Result<Vec<String>, CliError> {
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => return Ok(Vec::new()),
    };

    let response: GenericModelsResponse = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let models = response
        .data
        .into_iter()
        .map(|entry| entry.id)
        .filter(|id| filter_anthropic_model(id))
        .collect();
    Ok(dedupe_preserve_order(models))
}

async fn fetch_google_models(client: &reqwest::Client) -> Result<Vec<String>, CliError> {
    let api_key = match std::env::var("GOOGLE_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => return Ok(Vec::new()),
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    let response: GoogleModelsResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let models = response
        .models
        .into_iter()
        .map(|entry| entry.name.trim_start_matches("models/").to_string())
        .filter(|id| filter_google_model(id))
        .collect();
    Ok(dedupe_preserve_order(models))
}

async fn fetch_ollama_models(client: &reqwest::Client) -> Result<Vec<String>, CliError> {
    let base_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));

    let response: OllamaTagsResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let models = response
        .models
        .into_iter()
        .map(|entry| {
            entry
                .name
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|id| !id.is_empty())
        .collect();
    Ok(dedupe_preserve_order(models))
}

async fn build_model_catalog() -> Vec<(LlmProviderOption, Vec<String>)> {
    let mut catalog: Vec<(LlmProviderOption, Vec<String>)> = LLM_PROVIDERS
        .iter()
        .map(|provider| {
            (
                *provider,
                provider
                    .default_models
                    .iter()
                    .map(|model| (*model).to_string())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return catalog,
    };

    let (openai, anthropic, google, ollama) = tokio::join!(
        fetch_openai_models(&client),
        fetch_anthropic_models(&client),
        fetch_google_models(&client),
        fetch_ollama_models(&client),
    );

    if let Ok(models) = openai {
        if let Some((_, list)) = catalog
            .iter_mut()
            .find(|(provider, _)| provider.key == "openai")
        {
            let mut merged = list.clone();
            add_models(&mut merged, models);
            *list = dedupe_preserve_order(merged);
        }
    }
    if let Ok(models) = anthropic {
        if let Some((_, list)) = catalog
            .iter_mut()
            .find(|(provider, _)| provider.key == "claude")
        {
            let mut merged = list.clone();
            add_models(&mut merged, models);
            *list = dedupe_preserve_order(merged);
        }
    }
    if let Ok(models) = google {
        if let Some((_, list)) = catalog
            .iter_mut()
            .find(|(provider, _)| provider.key == "gemini")
        {
            let mut merged = list.clone();
            add_models(&mut merged, models);
            *list = dedupe_preserve_order(merged);
        }
    }
    if let Ok(models) = ollama {
        if let Some((_, list)) = catalog
            .iter_mut()
            .find(|(provider, _)| provider.key == "ollama")
        {
            let mut merged = list.clone();
            add_models(&mut merged, models);
            *list = dedupe_preserve_order(merged);
        }
    }

    catalog
}

fn select_llms(
    template_type: &str,
    catalog: &[(LlmProviderOption, Vec<String>)],
) -> Result<Vec<LlmSelection>, CliError> {
    let labels: Vec<&str> = catalog
        .iter()
        .map(|(provider, _)| provider.display_name)
        .collect();
    let default_idx = provider_index(DEFAULT_PROVIDER_KEY);
    let mut selections = Vec::new();

    if template_type == "multi-llm" {
        let mut defaults = vec![false; labels.len()];
        defaults[default_idx] = true;
        let selected_indices = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose LLM providers (Space to select, Enter to confirm)")
            .items(&labels)
            .defaults(&defaults)
            .interact()
            .map_err(|e| CliError::Other(e.into()))?;

        let selected_indices = if selected_indices.is_empty() {
            vec![default_idx]
        } else {
            selected_indices
        };

        for idx in selected_indices {
            let (option, models) = &catalog[idx];
            let model_names: Vec<&str> = models.iter().map(String::as_str).collect();
            let model_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Choose model for {}", option.display_name))
                .default(0)
                .items(&model_names)
                .interact()
                .map_err(|e| CliError::Other(e.into()))?;
            selections.push(LlmSelection {
                option: *option,
                model: models[model_idx].clone(),
            });
        }
    } else {
        let idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose LLM provider")
            .default(default_idx)
            .items(&labels)
            .interact()
            .map_err(|e| CliError::Other(e.into()))?;
        let (option, models) = &catalog[idx];
        let model_names: Vec<&str> = models.iter().map(String::as_str).collect();
        let model_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Choose model for {}", option.display_name))
            .default(0)
            .items(&model_names)
            .interact()
            .map_err(|e| CliError::Other(e.into()))?;
        selections.push(LlmSelection {
            option: *option,
            model: models[model_idx].clone(),
        });
    }

    Ok(selections)
}

fn apply_llm_context(ctx: &mut InitContext, llms: &[LlmSelection], multi_llm: bool) {
    let primary = &llms[0];
    ctx.llm_provider = primary.option.provider.to_string();
    ctx.llm_provider_display = primary.option.display_name.to_string();
    ctx.llm_model = primary.model.clone();
    ctx.llm_api_key_env = primary.option.api_key_env.to_string();
    ctx.llm_api_key_example = primary.option.api_key_example.to_string();
    ctx.llm_requires_api_key = primary.option.requires_api_key;
    ctx.llm_base_url = primary.option.base_url.unwrap_or_default().to_string();
    ctx.llm_primary_prompt_path = if multi_llm {
        format!("./prompts/{}.md", primary.option.key)
    } else {
        "./system_prompt.md".to_string()
    };
    ctx.llm_primary_tools_path = if multi_llm {
        format!("./tools/{}.json", primary.option.key)
    } else {
        "./tools.json".to_string()
    };
    ctx.llm_variants = llms
        .iter()
        .map(|llm| LlmVariantContext {
            key: llm.option.key.to_string(),
            agent_suffix: llm.option.agent_suffix.to_string(),
            display_name: llm.option.display_name.to_string(),
            provider: llm.option.provider.to_string(),
            model: llm.model.clone(),
            api_key_env: llm.option.api_key_env.to_string(),
            api_key_example: llm.option.api_key_example.to_string(),
            requires_api_key: llm.option.requires_api_key,
            base_url: llm.option.base_url.unwrap_or_default().to_string(),
            temperature: llm.option.temperature,
            max_tokens: llm.option.max_tokens,
        })
        .collect();
}

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    name: Option<String>,
    language: Option<String>,
    template: Option<String>,
    server: bool,
    framework: Option<String>,
    hybrid: bool,
    workspace: Option<String>,
    backend: String,
) -> Result<(), CliError> {
    let project_name = match name {
        Some(n) => n,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Project Name")
            .interact_text()
            .map_err(|e| CliError::Other(e.into()))?,
    };

    let selected_lang = match language {
        Some(l) => {
            let parsed = Language::from_str(&l).ok_or_else(|| {
                CliError::InvalidInput(
                    "Invalid language. TypeScript is currently available. Rust and Python are coming soon."
                        .into(),
                )
            })?;
            if parsed != Language::TypeScript {
                return Err(CliError::InvalidInput(
                    "Rust and Python templates are coming soon. Please use TypeScript for now."
                        .into(),
                ));
            }
            parsed
        }
        None => {
            let language_options = [
                "TypeScript (coming soon - default for now)",
                "Rust (coming soon)",
                "Python (coming soon)",
            ];
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Choose your language")
                .default(0)
                .items(&language_options)
                .interact()
                .map_err(|e| CliError::Other(e.into()))?;
            if idx == 0 {
                eprintln!("   TypeScript templates are in preview. Continuing with TypeScript.");
            } else if idx == 1 {
                eprintln!("   Rust templates are coming soon. Continuing with TypeScript.");
            } else {
                eprintln!("   Python templates are coming soon. Continuing with TypeScript.");
            }
            Language::TypeScript
        }
    };
    let lang = selected_lang.display_name();

    let tool_categories = &[
        "Files (read, write, delete, copy, move)",
        "Directories (mkdir, rmdir, list)",
        "Network (HTTP requests, web search)",
        "Data Formats (JSON, CSV, Base64)",
        "System (env vars, datetime, random)",
    ];

    println!("\nSelect built-in tools for your agent:");
    let selected_tools = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose tools (Space to select, Enter to confirm)")
        .items(&tool_categories[..])
        .interact()
        .map_err(|e| CliError::Other(e.into()))?;

    let mut allowed_tool_patterns = Vec::new();
    for idx in selected_tools {
        match idx {
            0 => allowed_tool_patterns.push("files:*"),
            1 => allowed_tool_patterns.push("directories:*"),
            2 => allowed_tool_patterns.push("network:*"),
            3 => allowed_tool_patterns.push("data:*"),
            4 => allowed_tool_patterns.push("system:*"),
            _ => {}
        }
    }

    let memory_options = &[
        "In-Memory (default)",
        "SQLite (local file)",
        "Redis (self-hosted)",
        "PostgreSQL",
        "Upstash Redis (serverless)",
        "Cloudflare KV",
        "Cloudflare D1",
        "AWS DynamoDB",
        "Turso (edge SQLite)",
        "Convex",
        "File-based (JSON)",
    ];
    println!("\nConfigure agent memory:");
    let memory_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose memory backend")
        .default(0)
        .items(&memory_options[..])
        .interact()
        .map_err(|e| CliError::Other(e.into()))?;

    let memory_config = match memory_idx {
        1 => {
            let filename: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("SQLite database file")
                .default("./agent_memory.db".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some(("sqlite", Box::leak(filename.into_boxed_str()) as &str))
        }
        2 => Some(("redis", "redis://localhost:6379")),
        3 => Some(("postgres", "postgresql://localhost:5432/sekuire")),
        4 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Upstash Redis URL")
                .default("https://xxx.upstash.io".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some(("upstash", Box::leak(url.into_boxed_str()) as &str))
        }
        5 => {
            let namespace_id: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Cloudflare KV Namespace ID")
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some((
                "cloudflare-kv",
                Box::leak(namespace_id.into_boxed_str()) as &str,
            ))
        }
        6 => {
            let database_id: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Cloudflare D1 Database ID")
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some((
                "cloudflare-d1",
                Box::leak(database_id.into_boxed_str()) as &str,
            ))
        }
        7 => {
            let table_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("DynamoDB Table Name")
                .default("sekuire-memory".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some(("dynamodb", Box::leak(table_name.into_boxed_str()) as &str))
        }
        8 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Turso Database URL")
                .default("libsql://xxx.turso.io".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some(("turso", Box::leak(url.into_boxed_str()) as &str))
        }
        9 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Convex Deployment URL")
                .default("https://xxx.convex.cloud".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            Some(("convex", Box::leak(url.into_boxed_str()) as &str))
        }
        10 => Some(("file", "./agent_memory.json")),
        _ => None,
    };

    let server_mode = if hybrid || server {
        true
    } else if let Some(ref t) = template {
        let t_lower = t.to_lowercase();
        t_lower == "http" || t_lower == "hybrid"
    } else {
        dialoguer::Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Enable HTTP server mode?")
            .default(false)
            .interact()
            .map_err(|e| CliError::Other(e.into()))?
    };

    let selected_framework = if server_mode {
        match framework {
            Some(f) => {
                let fw = Framework::from_str(&f, selected_lang).ok_or_else(|| {
                    CliError::InvalidInput(format!("Invalid framework '{}' for {}.", f, lang))
                })?;
                Some(fw.cli_name().to_string())
            }
            None => {
                println!("\nChoose HTTP framework:");
                let frameworks = Framework::for_language(selected_lang);
                let framework_names: Vec<&str> =
                    frameworks.iter().map(|f| f.display_name()).collect();

                let framework_idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Choose HTTP framework")
                    .default(0)
                    .items(&framework_names)
                    .interact()
                    .map_err(|e| CliError::Other(e.into()))?;
                Some(frameworks[framework_idx].cli_name().to_string())
            }
        }
    } else {
        None
    };

    let template_type =
        if hybrid {
            "hybrid".to_string()
        } else if server_mode {
            "http".to_string()
        } else {
            match template {
                Some(t) => {
                    let t = t.to_lowercase();
                    match t.as_str() {
                        "simple" | "multi-llm" | "react" | "rag" | "http" | "hybrid" => t,
                        _ => return Err(CliError::InvalidInput(
                            "Invalid template. Choose: simple, multi-llm, http, hybrid, react, rag"
                                .into(),
                        )),
                    }
                }
                None => "simple".to_string(),
            }
        };

    let model_catalog = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Checking latest LLM model catalogs...",
        async { build_model_catalog().await },
    )
    .await;
    let selected_llms = select_llms(&template_type, &model_catalog)?;
    let primary_llm = &selected_llms[0];

    println!(
        "\nInitializing {} project: {} (template: {})",
        lang, project_name, template_type
    );
    println!(
        "   LLM: {} ({})",
        primary_llm.model, primary_llm.option.display_name
    );
    println!("   Tools: {}", allowed_tool_patterns.join(", "));
    if let Some((backend_name, _)) = memory_config {
        println!("   Memory: {}", backend_name);
    }
    if template_type == "multi-llm" && selected_llms.len() > 1 {
        println!("   Multi-LLM providers: {}", selected_llms.len());
    }
    if let Some(ref fw) = selected_framework {
        println!("   Framework: {}", fw);
    }

    let root = Path::new(&project_name);
    if root.exists() {
        return Err(CliError::InvalidInput(format!(
            "Directory '{}' already exists.",
            project_name
        )));
    }

    fs::create_dir_all(&root).await?;
    println!("   Created directory: {}", project_name);

    let engine = TemplateEngine::new()?;

    let default_framework = "express".to_string();
    let mut ctx = match template_type.as_str() {
        "http" | "hybrid" => {
            let fw = selected_framework.as_ref().unwrap_or(&default_framework);
            InitContext::with_server(&project_name, &allowed_tool_patterns, memory_config, fw)
        }
        _ => InitContext::simple(&project_name, &allowed_tool_patterns, memory_config),
    };
    apply_llm_context(&mut ctx, &selected_llms, template_type == "multi-llm");

    let yaml_template = if template_type == "multi-llm" {
        "shared/sekuire_multi_llm.yml.tera"
    } else {
        "shared/sekuire.yml.tera"
    };
    let yaml_content = engine.render(yaml_template, &ctx)?;
    fs::write(root.join("sekuire.yml"), yaml_content).await?;
    println!("   Created sekuire.yml");

    if template_type == "multi-llm" {
        let prompts_dir = root.join("prompts");
        if !prompts_dir.exists() {
            fs::create_dir(&prompts_dir).await?;
        }
        for llm in &selected_llms {
            let prompt_name = format!("{}.md", llm.option.key);
            fs::write(prompts_dir.join(prompt_name), pick_prompt(llm.option.key)).await?;
        }
        println!("   Created LLM-specific prompts");

        let tools_dir = root.join("tools");
        if !tools_dir.exists() {
            fs::create_dir(&tools_dir).await?;
        }
        for llm in &selected_llms {
            let tools_name = format!("{}.json", llm.option.key);
            fs::write(tools_dir.join(tools_name), get_tools_json()).await?;
        }
        println!("   Created LLM-specific tool configs");
    } else {
        fs::write(root.join("system_prompt.md"), get_system_prompt()).await?;
        println!("   Created system_prompt.md");

        fs::write(root.join("tools.json"), get_tools_json()).await?;
        println!("   Created tools.json");

        let tools_dir = root.join("tools");
        if !tools_dir.exists() {
            fs::create_dir(&tools_dir).await?;
        }
    }

    let env_template = if server_mode {
        "shared/env_example_server.tera"
    } else {
        "shared/env_example.tera"
    };
    let env_content = engine.render_static(env_template)?;
    fs::write(root.join(".env.example"), env_content).await?;
    println!("   Created .env.example");

    #[derive(serde::Serialize)]
    struct NameCtx {
        name: String,
        llm_provider_display: String,
        llm_model: String,
    }
    let readme_content = engine.render(
        "shared/readme.md.tera",
        &NameCtx {
            name: project_name.clone(),
            llm_provider_display: primary_llm.option.display_name.to_string(),
            llm_model: primary_llm.model.clone(),
        },
    )?;
    fs::write(root.join("README.md"), readme_content).await?;
    println!("   Created README.md");

    create_sqlite_file_if_needed(root, memory_config).await?;

    if let Some(ws) = workspace {
        let policy_result = ui::with_spinner_if(
            ui::spinner_enabled(None),
            "Fetching active workspace policy...",
            async { fetch_active_policy(&backend, &ws).await },
        )
        .await;
        match policy_result {
            Ok(policy) => {
                let policy_path = root.join("policy.json");
                let json =
                    serde_json::to_string_pretty(&policy).map_err(|e| CliError::Other(e.into()))?;
                fs::write(&policy_path, json).await?;
                println!(
                    "   Saved active policy {} (v{}) to {}",
                    policy.policy_id,
                    policy.version,
                    policy_path.display()
                );
            }
            Err(e) => {
                eprintln!(
                    "   Could not fetch active policy for workspace {}: {}",
                    ws, e
                );
            }
        }
    }

    write_language_files(
        lang,
        &template_type,
        &selected_framework,
        memory_config.map(|(memory_type, _)| memory_type) == Some("sqlite"),
        &project_name,
        root,
        &engine,
    )
    .await?;

    let sekuiredir = root.join(".sekuire");
    if !sekuiredir.exists() {
        fs::create_dir(&sekuiredir).await?;
    }
    let key_path = sekuiredir.join("secret.key");
    let pub_path = sekuiredir.join("public.key");

    if !key_path.exists() {
        println!("   Generating Ed25519 Keypair...");
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let priv_hex = hex::encode(signing_key.to_bytes());
        let pub_hex = hex::encode(verifying_key.as_bytes());

        fs::write(&key_path, priv_hex).await?;
        fs::write(&pub_path, pub_hex).await?;
        println!("   Generated keypair (.sekuire/secret.key, .sekuire/public.key)");
    }

    println!("\nCreating local development environment...");
    let docker_ctx = DockerContext::new(&project_name, 8000, lang);
    let docker_compose = engine.render("shared/docker-compose.yml.tera", &docker_ctx)?;
    fs::write(root.join("docker-compose.yml"), docker_compose).await?;
    println!("   Created docker-compose.yml");

    let dockerfile_dev = engine.render("shared/dockerfile_dev.tera", &docker_ctx)?;
    fs::write(root.join("Dockerfile.dev"), dockerfile_dev).await?;
    println!("   Created Dockerfile.dev");

    let dockerfile_prod = engine.render("shared/dockerfile_prod.tera", &docker_ctx)?;
    fs::write(root.join("Dockerfile"), dockerfile_prod).await?;
    println!("   Created Dockerfile");

    let dockerignore = engine.render_static("shared/dockerignore.tera")?;
    fs::write(root.join(".dockerignore"), dockerignore).await?;
    println!("   Created .dockerignore");

    println!("\nProject initialized successfully!");
    println!("\nNext steps:");
    println!("   1. Copy .env.example to .env and add your API keys");
    match lang {
        "Python" => println!("   2. pip install -r requirements.txt"),
        "Rust" => println!("   2. cargo build"),
        "TypeScript" => println!("   2. npm install"),
        _ => {}
    }
    println!("   3. Review sekuire.yml to configure your agent");
    match lang {
        "Python" => println!("   4. python main.py"),
        "Rust" => println!("   4. cargo run"),
        "TypeScript" => println!("   4. npm start"),
        _ => {}
    }
    println!("\nOr use Docker:");
    println!("   docker compose up");
    Ok(())
}

async fn create_sqlite_file_if_needed(
    root: &Path,
    memory_config: Option<(&str, &str)>,
) -> Result<(), CliError> {
    if let Some(("sqlite", filename)) = memory_config {
        let configured_path = Path::new(filename);
        let sqlite_path = if configured_path.is_absolute() {
            configured_path.to_path_buf()
        } else {
            root.join(configured_path)
        };

        if let Some(parent) = sqlite_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).await?;
            }
        }

        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&sqlite_path)
            .await?;

        println!("   Created SQLite database file: {}", sqlite_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::create_sqlite_file_if_needed;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::fs;

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("{}_{}", prefix, nanos))
    }

    #[tokio::test]
    async fn sqlite_memory_creates_database_file() {
        let root = unique_temp_dir("sekuire-init-sqlite");
        fs::create_dir_all(&root)
            .await
            .expect("failed to create temp project dir");

        create_sqlite_file_if_needed(&root, Some(("sqlite", "./data/agent_memory.db")))
            .await
            .expect("sqlite file creation should succeed");

        assert!(root.join("data/agent_memory.db").exists());

        fs::remove_dir_all(&root)
            .await
            .expect("failed to remove temp project dir");
    }
}

async fn write_language_files(
    lang: &str,
    template_type: &str,
    selected_framework: &Option<String>,
    include_sqlite: bool,
    project_name: &str,
    root: &Path,
    engine: &TemplateEngine,
) -> Result<(), CliError> {
    match lang {
        "Python" => {
            let default_py_framework = "fastapi".to_string();
            if template_type == "multi-llm" {
                fs::write(root.join("requirements.txt"), PYTHON_REQUIREMENTS_MULTI_LLM).await?;
                fs::write(root.join("main.py"), PYTHON_MAIN_MULTI_LLM).await?;
            } else if template_type == "hybrid" {
                let fw = selected_framework.as_ref().unwrap_or(&default_py_framework);
                let reqs = engine.render(
                    "python/requirements_http.txt.tera",
                    &PackageContext::new(project_name, fw),
                )?;
                fs::write(root.join("requirements.txt"), reqs).await?;
                fs::write(root.join("main.py"), PYTHON_HYBRID_MAIN).await?;
                fs::write(root.join("agent.py"), PYTHON_AGENT_SHARED).await?;
                fs::write(root.join("cli.py"), PYTHON_CLI_INTERFACE).await?;
                fs::write(root.join("server.py"), get_python_http_server_template(fw)).await?;
                println!("   Created hybrid Python files (CLI + HTTP with {})", fw);
            } else if template_type == "http" {
                let fw = selected_framework.as_ref().unwrap_or(&default_py_framework);
                let reqs = engine.render(
                    "python/requirements_http.txt.tera",
                    &PackageContext::new(project_name, fw),
                )?;
                fs::write(root.join("requirements.txt"), reqs).await?;
                fs::write(root.join("main.py"), get_python_http_server_template(fw)).await?;
                fs::write(root.join("agent.py"), PYTHON_AGENT_SHARED).await?;
                println!("   Created HTTP Python files with {}", fw);
            } else {
                fs::write(root.join("requirements.txt"), PYTHON_REQUIREMENTS).await?;
                fs::write(root.join("main.py"), PYTHON_MAIN).await?;
            }
            fs::write(
                root.join(".gitignore"),
                "__pycache__\n*.pyc\n.env\n.sekuire/secret.key\n",
            )
            .await?;
            println!("   Created Python files");
        }
        "Rust" => {
            let src_dir = root.join("src");
            let default_rust_framework = "axum".to_string();

            if template_type == "multi-llm" {
                let cargo = engine.render(
                    "rust/cargo_multi_llm.toml.tera",
                    &PackageContext::new(project_name, ""),
                )?;
                fs::write(root.join("Cargo.toml"), cargo).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("main.rs"), RUST_MAIN_MULTI_LLM).await?;
            } else if template_type == "hybrid" {
                let fw = selected_framework
                    .as_ref()
                    .unwrap_or(&default_rust_framework);
                let cargo = engine.render(
                    "rust/cargo_http.toml.tera",
                    &PackageContext::new(project_name, fw),
                )?;
                fs::write(root.join("Cargo.toml"), cargo).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("main.rs"), RUST_HYBRID_MAIN).await?;
                fs::write(src_dir.join("agent.rs"), RUST_AGENT_SHARED).await?;
                fs::write(src_dir.join("cli.rs"), RUST_CLI_INTERFACE).await?;
                fs::write(src_dir.join("server.rs"), get_rust_http_server_template(fw)).await?;
                println!("   Created hybrid Rust files (CLI + HTTP with {})", fw);
            } else if template_type == "http" {
                let fw = selected_framework
                    .as_ref()
                    .unwrap_or(&default_rust_framework);
                let cargo = engine.render(
                    "rust/cargo_http.toml.tera",
                    &PackageContext::new(project_name, fw),
                )?;
                fs::write(root.join("Cargo.toml"), cargo).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("main.rs"), get_rust_http_server_template(fw)).await?;
                fs::write(src_dir.join("agent.rs"), RUST_AGENT_SHARED).await?;
                println!("   Created HTTP Rust files with {}", fw);
            } else {
                let cargo = engine.render(
                    "rust/cargo.toml.tera",
                    &PackageContext::new(project_name, ""),
                )?;
                fs::write(root.join("Cargo.toml"), cargo).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("main.rs"), RUST_MAIN).await?;
            }
            fs::write(
                root.join(".gitignore"),
                "target/\n.env\n.sekuire/secret.key\nCargo.lock\n",
            )
            .await?;
            println!("   Created Rust files");
        }
        "TypeScript" => {
            let src_dir = root.join("src");
            let tools_dir = root.join("tools");
            let default_ts_framework = "express".to_string();

            if template_type == "multi-llm" {
                let pkg = engine.render(
                    "typescript/package_multi_llm.json.tera",
                    &PackageContext::with_sqlite(project_name, "", include_sqlite),
                )?;
                fs::write(root.join("package.json"), pkg).await?;
                fs::write(root.join("tsconfig.json"), TS_CONFIG).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("index.ts"), TS_INDEX_MULTI_LLM).await?;
            } else if template_type == "hybrid" {
                let fw = selected_framework.as_ref().unwrap_or(&default_ts_framework);
                let pkg = engine.render(
                    "typescript/package_http.json.tera",
                    &PackageContext::with_sqlite(project_name, fw, include_sqlite),
                )?;
                fs::write(root.join("package.json"), pkg).await?;
                fs::write(root.join("tsconfig.json"), TS_CONFIG).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("index.ts"), TS_HYBRID_INDEX).await?;
                fs::write(src_dir.join("agent.ts"), TS_AGENT_SHARED).await?;
                fs::write(src_dir.join("cli.ts"), TS_CLI_INTERFACE).await?;
                fs::write(src_dir.join("server.ts"), get_http_server_template(fw)).await?;
                fs::create_dir_all(&tools_dir).await?;
                fs::write(tools_dir.join("calculator.ts"), get_calculator_tool_ts()).await?;
                fs::write(tools_dir.join("search.ts"), get_search_tool_ts()).await?;
                println!(
                    "   Created hybrid TypeScript files (CLI + HTTP with {})",
                    fw
                );
            } else if template_type == "http" {
                let fw = selected_framework.as_ref().unwrap_or(&default_ts_framework);
                let pkg = engine.render(
                    "typescript/package_http.json.tera",
                    &PackageContext::with_sqlite(project_name, fw, include_sqlite),
                )?;
                fs::write(root.join("package.json"), pkg).await?;
                fs::write(root.join("tsconfig.json"), TS_CONFIG).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("index.ts"), get_http_server_template(fw)).await?;
                fs::write(src_dir.join("agent.ts"), TS_AGENT_SHARED).await?;
                fs::create_dir_all(&tools_dir).await?;
                fs::write(tools_dir.join("calculator.ts"), get_calculator_tool_ts()).await?;
                fs::write(tools_dir.join("search.ts"), get_search_tool_ts()).await?;
                println!("   Created HTTP TypeScript files with {}", fw);
            } else {
                let pkg = engine.render(
                    "typescript/package.json.tera",
                    &PackageContext::with_sqlite(project_name, "", include_sqlite),
                )?;
                fs::write(root.join("package.json"), pkg).await?;
                fs::write(root.join("tsconfig.json"), TS_CONFIG).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("index.ts"), TS_INDEX).await?;
                fs::create_dir_all(&tools_dir).await?;
                fs::write(tools_dir.join("calculator.ts"), get_calculator_tool_ts()).await?;
                fs::write(tools_dir.join("search.ts"), get_search_tool_ts()).await?;
            }
            fs::write(
                root.join(".gitignore"),
                "node_modules\ndist\n.env\n.sekuire/secret.key\n",
            )
            .await?;
            println!("   Created TypeScript files");
        }
        _ => {}
    }
    Ok(())
}
