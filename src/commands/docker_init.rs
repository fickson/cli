use crate::error::CliError;
use crate::templates::engine::{DockerContext, TemplateEngine};
use std::path::PathBuf;

pub async fn execute(
    output: Option<PathBuf>,
    cloudflare: bool,
    port: u16,
    force: bool,
    ci: bool,
) -> Result<(), CliError> {
    let output_dir =
        output.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    println!("Generating Docker deployment files...");
    println!("   Output directory: {}", output_dir.display());

    let language = if output_dir.join("package.json").exists() {
        "typescript"
    } else if output_dir.join("Cargo.toml").exists() {
        "rust"
    } else if output_dir.join("requirements.txt").exists()
        || output_dir.join("pyproject.toml").exists()
    {
        "python"
    } else {
        println!("   Could not detect project type. Defaulting to TypeScript.");
        "typescript"
    };

    println!("   Detected language: {}", language);
    println!("   Port: {}", port);

    let agent_name = if output_dir.join("sekuire.yml").exists() {
        let content = tokio::fs::read_to_string(output_dir.join("sekuire.yml")).await?;
        let manifest: sekuire_protocol::Manifest =
            serde_yaml::from_str(&content).map_err(|e| CliError::ManifestParse(e.to_string()))?;
        manifest.project.name.clone()
    } else {
        output_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("agent")
            .to_string()
    };

    println!("   Agent name: {}", agent_name);

    let engine = TemplateEngine::new()?;
    let ctx = DockerContext::new(&agent_name, port, language);

    let docker_compose = engine.render("shared/docker-compose.yml.tera", &ctx)?;
    let dockerfile_dev = engine.render("shared/dockerfile_dev.tera", &ctx)?;
    let dockerfile_prod = engine.render("shared/dockerfile_prod.tera", &ctx)?;
    let dockerignore = engine.render_static("shared/dockerignore.tera")?;

    let files: &[(&str, &str)] = &[
        ("docker-compose.yml", &docker_compose),
        ("Dockerfile.dev", &dockerfile_dev),
        ("Dockerfile", &dockerfile_prod),
        (".dockerignore", &dockerignore),
    ];

    for (filename, content) in files {
        let path = output_dir.join(filename);
        if path.exists() && !force {
            println!(
                "   {} exists, skipping (use --force to overwrite)",
                filename
            );
        } else {
            tokio::fs::write(&path, content).await?;
            println!("   Created {}", filename);
        }
    }

    if cloudflare {
        println!("\n   Generating Cloudflare Container configuration...");
        let cf_dir = output_dir.join("cloudflare");
        tokio::fs::create_dir_all(&cf_dir).await?;

        let wrangler_toml = format!(
            r#"# Cloudflare Container Configuration
name = "{agent_name}"
main = "src/worker.ts"
compatibility_date = "2024-01-01"
compatibility_flags = ["nodejs_compat"]

[containers]
  [containers.agent]
    class_name = "Agent"
    image = "your-registry/{agent_name}:latest"
    max_instances = 3

[[durable_objects.bindings]]
  name = "AGENT"
  class_name = "Agent"

[vars]
  ENVIRONMENT = "production"
  PORT = "{port}"
"#,
            agent_name = agent_name,
            port = port
        );

        let worker_ts = r#"import { Container } from "cloudflare:container";

export class Agent extends Container {
  defaultPort = 8000;
  sleepAfter = "5m";
}

interface Env {
  AGENT: DurableObjectNamespace<Agent>;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const stub = env.AGENT.get(env.AGENT.idFromName("default"));
    const container = await stub.getContainer();
    return container.fetch(request);
  }
};
"#;

        let wrangler_path = cf_dir.join("wrangler.toml");
        if wrangler_path.exists() && !force {
            println!("   cloudflare/wrangler.toml exists, skipping");
        } else {
            tokio::fs::write(&wrangler_path, wrangler_toml).await?;
            println!("   Created cloudflare/wrangler.toml");
        }

        let src_dir = cf_dir.join("src");
        tokio::fs::create_dir_all(&src_dir).await?;
        let worker_path = src_dir.join("worker.ts");
        if worker_path.exists() && !force {
            println!("   cloudflare/src/worker.ts exists, skipping");
        } else {
            tokio::fs::write(&worker_path, worker_ts).await?;
            println!("   Created cloudflare/src/worker.ts");
        }
    }

    if ci {
        println!("\n   Generating GitHub Actions CI/CD workflow...");
        let workflow_dir = output_dir.join(".github").join("workflows");
        tokio::fs::create_dir_all(&workflow_dir).await?;

        let workflow_content = engine.render("shared/github-workflow.yml.tera", &ctx)?;
        let workflow_path = workflow_dir.join("sekuire-build.yml");
        if workflow_path.exists() && !force {
            println!("   .github/workflows/sekuire-build.yml exists, skipping");
        } else {
            tokio::fs::write(&workflow_path, workflow_content).await?;
            println!("   Created .github/workflows/sekuire-build.yml");
        }
        println!("\n   Required GitHub Secrets:");
        println!("   - SEKUIRE_PUSH_TOKEN  (from 'sekuire registry login')");
        println!("   - SEKUIRE_API_KEY     (from dashboard settings)");
        println!("   - SEKUIRE_BACKEND_URL (e.g. https://api.sekuire.ai)");
    }

    println!("\nDocker files generated successfully!");
    println!("\n   Quick start:");
    println!("   1. docker compose up         # Start development environment");
    println!(
        "   2. docker build -t {} .  # Build production image",
        agent_name
    );
    if cloudflare {
        println!("   3. cd cloudflare && wrangler deploy  # Deploy to Cloudflare");
    }
    Ok(())
}
