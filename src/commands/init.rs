use crate::error::CliError;
use crate::services::policy::fetch_active_policy;
use crate::templates::engine::{DockerContext, InitContext, PackageContext, TemplateEngine};
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
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::path::Path;
use tokio::fs;

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

    let all_languages = Language::all();
    let language_names: Vec<&str> = all_languages.iter().map(|l| l.display_name()).collect();
    let selected_lang = match language {
        Some(l) => Language::from_str(&l).ok_or_else(|| {
            CliError::InvalidInput("Invalid language. Choose python, rust, or typescript.".into())
        })?,
        None => {
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Choose your language")
                .default(0)
                .items(&language_names)
                .interact()
                .map_err(|e| CliError::Other(e.into()))?;
            all_languages[idx]
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

    println!(
        "\nInitializing {} project: {} (template: {})",
        lang, project_name, template_type
    );
    println!("   Tools: {}", allowed_tool_patterns.join(", "));
    if let Some((backend_name, _)) = memory_config {
        println!("   Memory: {}", backend_name);
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
    let yaml_content = match template_type.as_str() {
        "multi-llm" => {
            let ctx = InitContext::simple(&project_name, &allowed_tool_patterns, memory_config);
            engine.render("shared/sekuire_multi_llm.yml.tera", &ctx)?
        }
        "http" | "hybrid" => {
            let fw = selected_framework.as_ref().unwrap_or(&default_framework);
            let ctx =
                InitContext::with_server(&project_name, &allowed_tool_patterns, memory_config, fw);
            engine.render("shared/sekuire.yml.tera", &ctx)?
        }
        _ => {
            let ctx = InitContext::simple(&project_name, &allowed_tool_patterns, memory_config);
            engine.render("shared/sekuire.yml.tera", &ctx)?
        }
    };
    fs::write(root.join("sekuire.yml"), yaml_content).await?;
    println!("   Created sekuire.yml");

    if template_type == "multi-llm" {
        let prompts_dir = root.join("prompts");
        if !prompts_dir.exists() {
            fs::create_dir(&prompts_dir).await?;
        }
        fs::write(prompts_dir.join("openai.md"), get_openai_prompt()).await?;
        fs::write(prompts_dir.join("claude.md"), get_claude_prompt()).await?;
        fs::write(prompts_dir.join("gemini.md"), get_gemini_prompt()).await?;
        fs::write(prompts_dir.join("ollama.md"), get_ollama_prompt()).await?;
        println!("   Created LLM-specific prompts");

        let tools_dir = root.join("tools");
        if !tools_dir.exists() {
            fs::create_dir(&tools_dir).await?;
        }
        fs::write(tools_dir.join("openai.json"), get_tools_json()).await?;
        fs::write(tools_dir.join("claude.json"), get_tools_json()).await?;
        fs::write(tools_dir.join("gemini.json"), get_tools_json()).await?;
        fs::write(tools_dir.join("ollama.json"), get_tools_json()).await?;
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
    }
    let readme_content = engine.render(
        "shared/readme.md.tera",
        &NameCtx {
            name: project_name.clone(),
        },
    )?;
    fs::write(root.join("README.md"), readme_content).await?;
    println!("   Created README.md");

    if let Some(ws) = workspace {
        println!("   Fetching active workspace policy...");
        match fetch_active_policy(&backend, &ws).await {
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

async fn write_language_files(
    lang: &str,
    template_type: &str,
    selected_framework: &Option<String>,
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
                    &PackageContext::new(project_name, ""),
                )?;
                fs::write(root.join("package.json"), pkg).await?;
                fs::write(root.join("tsconfig.json"), TS_CONFIG).await?;
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("index.ts"), TS_INDEX_MULTI_LLM).await?;
            } else if template_type == "hybrid" {
                let fw = selected_framework.as_ref().unwrap_or(&default_ts_framework);
                let pkg = engine.render(
                    "typescript/package_http.json.tera",
                    &PackageContext::new(project_name, fw),
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
                    &PackageContext::new(project_name, fw),
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
                    &PackageContext::new(project_name, ""),
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
