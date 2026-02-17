#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    Rust,
    TypeScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    // Python
    FastAPI,
    Django,
    // Rust
    Axum,
    Actix,
    RustBuiltIn,
    // TypeScript
    Express,
    Fastify,
    Hono,
    NodeBuiltIn,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateType {
    Simple,
    Http,
    Hybrid,
    MultiLlm,
}

impl Language {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python" => Some(Self::Python),
            "rust" => Some(Self::Rust),
            "typescript" | "ts" => Some(Self::TypeScript),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
        }
    }

    pub fn all() -> &'static [Language] {
        &[Self::Python, Self::Rust, Self::TypeScript]
    }
}

impl Framework {
    pub fn for_language(lang: Language) -> &'static [Framework] {
        match lang {
            Language::Python => &[Self::FastAPI, Self::Django],
            Language::Rust => &[Self::Axum, Self::Actix, Self::RustBuiltIn],
            Language::TypeScript => &[Self::Express, Self::Fastify, Self::Hono, Self::NodeBuiltIn],
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FastAPI => "FastAPI",
            Self::Django => "Django",
            Self::Axum => "Axum",
            Self::Actix => "Actix",
            Self::RustBuiltIn => "Built-in",
            Self::Express => "Express",
            Self::Fastify => "Fastify",
            Self::Hono => "Hono",
            Self::NodeBuiltIn => "Built-in Node.js",
        }
    }

    pub fn cli_name(&self) -> &'static str {
        match self {
            Self::FastAPI => "fastapi",
            Self::Django => "django",
            Self::Axum => "axum",
            Self::Actix => "actix",
            Self::RustBuiltIn => "built-in",
            Self::Express => "express",
            Self::Fastify => "fastify",
            Self::Hono => "hono",
            Self::NodeBuiltIn => "built-in node.js",
        }
    }

    #[allow(dead_code)]
    pub fn default_for(lang: Language) -> Self {
        match lang {
            Language::Python => Self::FastAPI,
            Language::Rust => Self::Axum,
            Language::TypeScript => Self::Express,
        }
    }

    pub fn from_str(s: &str, lang: Language) -> Option<Self> {
        let s = s.to_lowercase();
        let frameworks = Self::for_language(lang);
        frameworks.iter().find(|f| f.cli_name() == s).copied()
    }

    #[allow(dead_code)]
    pub fn detect_from_deps(deps: &[String]) -> Option<Self> {
        for dep in deps {
            let d = dep.to_lowercase();
            match d.as_str() {
                "fastapi" => return Some(Self::FastAPI),
                "django" => return Some(Self::Django),
                "express" => return Some(Self::Express),
                "fastify" => return Some(Self::Fastify),
                "hono" => return Some(Self::Hono),
                _ => {
                    if d.contains("axum") {
                        return Some(Self::Axum);
                    }
                    if d.contains("actix") {
                        return Some(Self::Actix);
                    }
                }
            }
        }
        None
    }
}

#[allow(dead_code)]
impl TemplateType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "simple" => Some(Self::Simple),
            "http" => Some(Self::Http),
            "hybrid" => Some(Self::Hybrid),
            "multi-llm" => Some(Self::MultiLlm),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_str() {
        assert_eq!(Language::from_str("python"), Some(Language::Python));
        assert_eq!(Language::from_str("RUST"), Some(Language::Rust));
        assert_eq!(Language::from_str("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_str("typescript"), Some(Language::TypeScript));
        assert_eq!(Language::from_str("go"), None);
    }

    #[test]
    fn language_all_contains_all_variants() {
        let all = Language::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&Language::Python));
        assert!(all.contains(&Language::Rust));
        assert!(all.contains(&Language::TypeScript));
    }

    #[test]
    fn framework_for_language() {
        let py = Framework::for_language(Language::Python);
        assert_eq!(py.len(), 2);
        assert!(py.contains(&Framework::FastAPI));
        assert!(py.contains(&Framework::Django));

        let rs = Framework::for_language(Language::Rust);
        assert_eq!(rs.len(), 3);
        assert!(rs.contains(&Framework::Axum));

        let ts = Framework::for_language(Language::TypeScript);
        assert_eq!(ts.len(), 4);
        assert!(ts.contains(&Framework::Express));
    }

    #[test]
    fn framework_from_str() {
        assert_eq!(
            Framework::from_str("fastapi", Language::Python),
            Some(Framework::FastAPI)
        );
        assert_eq!(
            Framework::from_str("axum", Language::Rust),
            Some(Framework::Axum)
        );
        assert_eq!(
            Framework::from_str("express", Language::TypeScript),
            Some(Framework::Express)
        );
        assert_eq!(Framework::from_str("express", Language::Python), None);
    }

    #[test]
    fn framework_default_for() {
        assert_eq!(Framework::default_for(Language::Python), Framework::FastAPI);
        assert_eq!(Framework::default_for(Language::Rust), Framework::Axum);
        assert_eq!(
            Framework::default_for(Language::TypeScript),
            Framework::Express
        );
    }

    #[test]
    fn framework_detect_from_deps() {
        let deps = vec!["fastapi".to_string(), "uvicorn".to_string()];
        assert_eq!(Framework::detect_from_deps(&deps), Some(Framework::FastAPI));

        let deps = vec!["tokio".to_string(), "axum".to_string()];
        assert_eq!(Framework::detect_from_deps(&deps), Some(Framework::Axum));

        let deps = vec!["tokio".to_string()];
        assert_eq!(Framework::detect_from_deps(&deps), None);
    }

    #[test]
    fn template_type_from_str() {
        assert_eq!(TemplateType::from_str("simple"), Some(TemplateType::Simple));
        assert_eq!(TemplateType::from_str("HTTP"), Some(TemplateType::Http));
        assert_eq!(
            TemplateType::from_str("multi-llm"),
            Some(TemplateType::MultiLlm)
        );
        assert_eq!(TemplateType::from_str("unknown"), None);
    }

    #[test]
    fn framework_cli_name_roundtrip() {
        for lang in Language::all() {
            for fw in Framework::for_language(*lang) {
                let name = fw.cli_name();
                assert_eq!(Framework::from_str(name, *lang), Some(*fw));
            }
        }
    }
}
