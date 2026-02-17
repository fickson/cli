use crate::error::CliError;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::path::Path;
use tokio::fs;

pub async fn execute_memory() -> Result<(), CliError> {
    let config_path = Path::new("sekuire.yml");

    if !config_path.exists() {
        return Err(CliError::InvalidInput(
            "No sekuire.yml found. Run `sekuire init` first or navigate to a Sekuire project."
                .into(),
        ));
    }

    let content = fs::read_to_string(config_path).await?;

    let memory_options = &[
        "In-Memory (ephemeral)",
        "SQLite (local file)",
        "Redis (self-hosted)",
        "PostgreSQL",
        "Upstash Redis (serverless)",
        "Cloudflare KV",
        "Cloudflare D1",
        "AWS DynamoDB",
        "Turso (edge SQLite)",
        "Convex",
    ];

    println!("\nSelect memory backend for your agent:");
    let memory_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose memory backend")
        .default(0)
        .items(&memory_options[..])
        .interact()
        .map_err(|e| CliError::Other(e.into()))?;

    let (memory_type, memory_config) = match memory_idx {
        0 => ("buffer".to_string(), "    max_messages: 10".to_string()),
        1 => {
            let filename: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("SQLite database file")
                .default("./agent_memory.db".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            (
                "sqlite".to_string(),
                format!("    config:\n      filename: \"{}\"", filename),
            )
        }
        2 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Redis URL")
                .default("redis://localhost:6379".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            (
                "redis".to_string(),
                format!("    config:\n      url: \"{}\"", url),
            )
        }
        3 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("PostgreSQL connection string")
                .default("postgresql://localhost:5432/sekuire".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            (
                "postgres".to_string(),
                format!("    config:\n      url: \"{}\"", url),
            )
        }
        4 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Upstash Redis REST URL")
                .default("https://xxx.upstash.io".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            ("upstash".to_string(), format!("    config:\n      url: \"{}\"\n      token: \"${{UPSTASH_REDIS_REST_TOKEN}}\"", url))
        }
        5 => {
            let namespace_id: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Cloudflare KV Namespace ID")
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            ("cloudflare-kv".to_string(), format!(
                "    config:\n      namespace_id: \"{}\"\n      account_id: \"${{CF_ACCOUNT_ID}}\"\n      api_token: \"${{CF_API_TOKEN}}\"",
                namespace_id
            ))
        }
        6 => {
            let database_id: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Cloudflare D1 Database ID")
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            ("cloudflare-d1".to_string(), format!(
                "    config:\n      database_id: \"{}\"\n      account_id: \"${{CF_ACCOUNT_ID}}\"\n      api_token: \"${{CF_API_TOKEN}}\"",
                database_id
            ))
        }
        7 => {
            let table_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("DynamoDB Table Name")
                .default("sekuire-memory".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            let region: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("AWS Region")
                .default("us-east-1".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            (
                "dynamodb".to_string(),
                format!(
                    "    config:\n      table_name: \"{}\"\n      region: \"{}\"",
                    table_name, region
                ),
            )
        }
        8 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Turso Database URL")
                .default("libsql://xxx.turso.io".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            (
                "turso".to_string(),
                format!(
                    "    config:\n      url: \"{}\"\n      auth_token: \"${{TURSO_AUTH_TOKEN}}\"",
                    url
                ),
            )
        }
        9 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Convex Deployment URL")
                .default("https://xxx.convex.cloud".into())
                .interact_text()
                .map_err(|e| CliError::Other(e.into()))?;
            (
                "convex".to_string(),
                format!("    config:\n      url: \"{}\"", url),
            )
        }
        _ => ("buffer".to_string(), "    max_messages: 10".to_string()),
    };

    let memory_block = format!(
        "  memory:\n    type: \"{}\"\n{}",
        memory_type, memory_config
    );

    let updated_content = if content.contains("  memory:") {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut skip_memory_block = false;
        let mut indent_level = 0;

        for line in lines {
            if line.trim_start().starts_with("memory:")
                && line.starts_with("  ")
                && !line.starts_with("    ")
            {
                skip_memory_block = true;
                indent_level = line.len() - line.trim_start().len();
                result.push(&memory_block[..]);
                continue;
            }

            if skip_memory_block {
                let current_indent = line.len() - line.trim_start().len();
                if !line.trim().is_empty()
                    && current_indent <= indent_level
                    && !line.trim_start().starts_with('#')
                {
                    skip_memory_block = false;
                    result.push(line);
                }
            } else {
                result.push(line);
            }
        }

        result.join("\n")
    } else if content.contains("agent:") {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut inserted = false;

        for (i, line) in lines.iter().enumerate() {
            result.push(*line);

            if !inserted && line.trim_start().starts_with("agent:") {
                let next_lines_start = i + 1;
                let mut insert_after = i;

                for (j, next_line) in lines.iter().enumerate().skip(next_lines_start) {
                    if next_line.trim().is_empty() || next_line.starts_with("  ") {
                        insert_after = j;
                    } else {
                        break;
                    }
                }

                if insert_after == i {
                    result.push("");
                    result.push(&memory_block);
                    inserted = true;
                }
            }
        }

        if !inserted {
            result.push("");
            result.push(&memory_block);
        }

        result.join("\n")
    } else {
        format!("{}\n\nagent:\n{}", content.trim_end(), memory_block)
    };

    fs::write(config_path, updated_content).await?;

    println!("\nMemory backend configured successfully!");
    println!("   Type: {}", memory_type);
    println!("   Config updated in sekuire.yml");

    print_install_instructions(&memory_type);

    Ok(())
}

fn print_install_instructions(memory_type: &str) {
    println!("\nInstall the required dependency:");

    match memory_type {
        "sqlite" => {
            println!("   npm install better-sqlite3");
            println!("   # or");
            println!("   pip install sqlite3  # (included in Python stdlib)");
        }
        "redis" => {
            println!("   npm install redis");
            println!("   # or");
            println!("   pip install redis");
        }
        "postgres" => {
            println!("   npm install pg");
            println!("   # or");
            println!("   pip install psycopg2-binary");
        }
        "upstash" => {
            println!("   npm install @upstash/redis");
            println!("   # or");
            println!("   pip install upstash-redis");
            println!("\nSet environment variables:");
            println!("   UPSTASH_REDIS_REST_TOKEN=your_token");
        }
        "cloudflare-kv" | "cloudflare-d1" => {
            println!("   # No additional dependencies needed for Cloudflare Workers");
            println!("   # For Node.js REST API access, use fetch (built-in)");
            println!("\nSet environment variables:");
            println!("   CF_ACCOUNT_ID=your_account_id");
            println!("   CF_API_TOKEN=your_api_token");
        }
        "dynamodb" => {
            println!("   npm install @aws-sdk/client-dynamodb @aws-sdk/lib-dynamodb");
            println!("   # or");
            println!("   pip install boto3");
            println!("\nSet environment variables:");
            println!("   AWS_ACCESS_KEY_ID=your_key");
            println!("   AWS_SECRET_ACCESS_KEY=your_secret");
            println!("   AWS_REGION=us-east-1");
        }
        "turso" => {
            println!("   npm install @libsql/client");
            println!("   # or");
            println!("   pip install libsql-experimental");
            println!("\nSet environment variables:");
            println!("   TURSO_AUTH_TOKEN=your_token");
        }
        "convex" => {
            println!("   npm install convex");
            println!("\nDeploy the Convex schema:");
            println!("   npx convex dev");
            println!("\nNote: You need to create the sekuireMemory table in your Convex schema.");
            println!("See: https://docs.sekuire.ai/sdk/memory/convex");
        }
        _ => {}
    }
}
