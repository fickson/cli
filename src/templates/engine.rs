use rust_embed::Embed;
use serde::Serialize;
use tera::{Context, Tera};

use crate::error::CliError;

#[derive(Embed)]
#[folder = "src/templates/bundled/"]
struct BundledTemplates;

pub struct TemplateEngine {
    tera: Tera,
}

#[derive(Serialize)]
pub struct InitContext {
    pub name: String,
    pub package_name: String,
    pub tools: Vec<String>,
    pub memory_type: String,
    pub memory_url: String,
    pub server_mode: bool,
    pub framework: String,
}

#[derive(Serialize)]
pub struct DockerContext {
    pub agent_name: String,
    pub port: u16,
    pub language: String,
    pub dev_command: String,
}

#[derive(Serialize)]
pub struct PackageContext {
    pub name: String,
    pub package_name: String,
    pub framework: String,
}

impl TemplateEngine {
    pub fn new() -> Result<Self, CliError> {
        let mut tera = Tera::default();

        for path in BundledTemplates::iter() {
            let path_str = path.as_ref();
            if let Some(file) = BundledTemplates::get(path_str) {
                let content = std::str::from_utf8(file.data.as_ref()).map_err(|e| {
                    CliError::Template(format!("Invalid UTF-8 in template {}: {}", path_str, e))
                })?;
                tera.add_raw_template(path_str, content).map_err(|e| {
                    CliError::Template(format!("Failed to parse template {}: {}", path_str, e))
                })?;
            }
        }

        Ok(Self { tera })
    }

    pub fn render(
        &self,
        template_name: &str,
        context: &impl Serialize,
    ) -> Result<String, CliError> {
        let ctx = Context::from_serialize(context)
            .map_err(|e| CliError::Template(format!("Context serialization error: {}", e)))?;
        self.tera
            .render(template_name, &ctx)
            .map_err(|e| CliError::Template(format!("Render error for '{}': {}", template_name, e)))
    }

    pub fn render_static(&self, template_name: &str) -> Result<String, CliError> {
        let ctx = Context::new();
        self.tera
            .render(template_name, &ctx)
            .map_err(|e| CliError::Template(format!("Render error for '{}': {}", template_name, e)))
    }

    #[allow(dead_code)]
    pub fn has_template(&self, name: &str) -> bool {
        self.tera.get_template(name).is_ok()
    }
}

impl InitContext {
    pub fn simple(name: &str, tools: &[&str], memory: Option<(&str, &str)>) -> Self {
        let package_name = name.to_lowercase().replace(' ', "-");
        let (memory_type, memory_url) = match memory {
            Some((t, u)) => (t.to_string(), u.to_string()),
            None => ("buffer".to_string(), String::new()),
        };
        Self {
            name: name.to_string(),
            package_name,
            tools: tools.iter().map(|s| s.to_string()).collect(),
            memory_type,
            memory_url,
            server_mode: false,
            framework: String::new(),
        }
    }

    pub fn with_server(
        name: &str,
        tools: &[&str],
        memory: Option<(&str, &str)>,
        framework: &str,
    ) -> Self {
        let mut ctx = Self::simple(name, tools, memory);
        ctx.server_mode = true;
        ctx.framework = framework.to_string();
        ctx
    }
}

impl DockerContext {
    pub fn new(agent_name: &str, port: u16, language: &str) -> Self {
        let dev_command = match language {
            "Python" | "python" => "python main.py",
            "Rust" | "rust" => "cargo watch -x run",
            _ => "npm run dev",
        };
        Self {
            agent_name: agent_name.to_lowercase().replace(' ', "-"),
            port,
            language: language.to_lowercase(),
            dev_command: dev_command.to_string(),
        }
    }
}

impl PackageContext {
    pub fn new(name: &str, framework: &str) -> Self {
        Self {
            name: name.to_string(),
            package_name: name.to_lowercase().replace(' ', "-"),
            framework: framework.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_loads_bundled_templates() {
        let engine = TemplateEngine::new().unwrap();
        assert!(engine.has_template("shared/sekuire.yml.tera"));
        assert!(engine.has_template("shared/docker-compose.yml.tera"));
        assert!(engine.has_template("shared/dockerignore.tera"));
        assert!(engine.has_template("adopt/integration.tera"));
    }

    #[test]
    fn engine_renders_static_template() {
        let engine = TemplateEngine::new().unwrap();
        let result = engine.render_static("shared/dockerignore.tera").unwrap();
        assert!(result.contains("node_modules"));
        assert!(result.contains("target"));
    }

    #[test]
    fn engine_renders_with_context() {
        let engine = TemplateEngine::new().unwrap();
        let ctx = InitContext::simple("test-agent", &["files:*"], None);
        let result = engine.render("shared/sekuire.yml.tera", &ctx).unwrap();
        assert!(result.contains("test-agent"));
        assert!(result.contains("files:*"));
    }

    #[test]
    fn engine_renders_server_mode() {
        let engine = TemplateEngine::new().unwrap();
        let ctx = InitContext::with_server("my-api", &[], None, "express");
        let result = engine.render("shared/sekuire.yml.tera", &ctx).unwrap();
        assert!(result.contains("my-api"));
    }

    #[test]
    fn engine_renders_docker_compose() {
        let engine = TemplateEngine::new().unwrap();
        let ctx = DockerContext::new("my-agent", 9000, "python");
        let result = engine
            .render("shared/docker-compose.yml.tera", &ctx)
            .unwrap();
        assert!(result.contains("my-agent"));
        assert!(result.contains("9000"));
        assert!(result.contains("python main.py"));
    }

    #[test]
    fn engine_renders_github_workflow() {
        let engine = TemplateEngine::new().unwrap();
        let ctx = DockerContext::new("my-agent", 8000, "rust");
        let result = engine
            .render("shared/github-workflow.yml.tera", &ctx)
            .unwrap();
        assert!(result.contains("my-agent"));
        assert!(result.contains("rust-toolchain"));
    }

    #[test]
    fn engine_error_on_missing_template() {
        let engine = TemplateEngine::new().unwrap();
        let result = engine.render_static("nonexistent.tera");
        assert!(result.is_err());
    }

    #[test]
    fn init_context_simple_defaults() {
        let ctx = InitContext::simple("My Agent", &["files:*", "network:*"], None);
        assert_eq!(ctx.name, "My Agent");
        assert_eq!(ctx.package_name, "my-agent");
        assert_eq!(ctx.memory_type, "buffer");
        assert!(!ctx.server_mode);
    }

    #[test]
    fn init_context_with_memory() {
        let ctx = InitContext::simple("test", &[], Some(("redis", "redis://localhost:6379")));
        assert_eq!(ctx.memory_type, "redis");
        assert_eq!(ctx.memory_url, "redis://localhost:6379");
    }

    #[test]
    fn docker_context_dev_commands() {
        let py = DockerContext::new("x", 8000, "python");
        assert_eq!(py.dev_command, "python main.py");

        let rs = DockerContext::new("x", 8000, "rust");
        assert_eq!(rs.dev_command, "cargo watch -x run");

        let ts = DockerContext::new("x", 8000, "typescript");
        assert_eq!(ts.dev_command, "npm run dev");
    }

    #[test]
    fn package_context_name_normalization() {
        let ctx = PackageContext::new("My Cool Agent", "express");
        assert_eq!(ctx.package_name, "my-cool-agent");
        assert_eq!(ctx.framework, "express");
    }
}
