use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Not authenticated. Run 'sekuire login' first.")]
    NotAuthenticated,

    #[error("No sekuire.yml or sekuire.json found in current directory.")]
    ManifestNotFound,

    #[error("No private key found. Run 'sekuire keygen' first.")]
    KeyNotFound,

    #[error("Registry error ({status}): {body}")]
    RegistryError { status: u16, body: String },

    #[error("{message}")]
    PlanLimitExceeded {
        message: String,
        required_plan: String,
        upgrade_url: String,
        code: String,
    },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Manifest parse error: {0}")]
    ManifestParse(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub fn parse_plan_limit_error(status: u16, body: &str) -> Option<CliError> {
    if status != 402 {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let error_type = parsed.get("error")?.as_str()?;
    if error_type != "plan_limit_exceeded" {
        return None;
    }
    Some(CliError::PlanLimitExceeded {
        message: parsed
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Plan limit exceeded")
            .to_string(),
        required_plan: parsed
            .get("required_plan")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        upgrade_url: parsed
            .get("upgrade_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://sekuire.ai/billing")
            .to_string(),
        code: parsed
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("PLAN_LIMIT_EXCEEDED")
            .to_string(),
    })
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        Self::ManifestParse(e.to_string())
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::ManifestParse(e.to_string())
    }
}

impl From<hex::FromHexError> for CliError {
    fn from(e: hex::FromHexError) -> Self {
        Self::InvalidInput(format!("Invalid hex: {}", e))
    }
}

impl From<tera::Error> for CliError {
    fn from(e: tera::Error) -> Self {
        Self::Template(e.to_string())
    }
}

pub fn truncate_hash(hash: &str, len: usize) -> &str {
    if hash.len() >= len {
        &hash[..len]
    } else {
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_hash_normal() {
        assert_eq!(truncate_hash("abcdef1234567890", 8), "abcdef12");
    }

    #[test]
    fn truncate_hash_exact_length() {
        assert_eq!(truncate_hash("abcdef", 6), "abcdef");
    }

    #[test]
    fn truncate_hash_shorter_than_len() {
        assert_eq!(truncate_hash("abc", 8), "abc");
    }

    #[test]
    fn truncate_hash_empty() {
        assert_eq!(truncate_hash("", 8), "");
    }

    #[test]
    fn truncate_hash_zero_len() {
        assert_eq!(truncate_hash("abcdef", 0), "");
    }

    #[test]
    fn error_display() {
        let err = CliError::NotAuthenticated;
        assert_eq!(
            err.to_string(),
            "Not authenticated. Run 'sekuire login' first."
        );

        let err = CliError::RegistryError {
            status: 404,
            body: "not found".into(),
        };
        assert_eq!(err.to_string(), "Registry error (404): not found");
    }

    #[test]
    fn parse_plan_limit_error_valid_402() {
        let body = r#"{
            "error": "plan_limit_exceeded",
            "message": "Your current plan (free) does not include fleet_ops.",
            "code": "FLEET_OPS_REQUIRES_UPGRADE",
            "required_plan": "starter",
            "upgrade_url": "https://sekuire.ai/billing"
        }"#;
        let result = parse_plan_limit_error(402, body);
        assert!(result.is_some());
        match result.unwrap() {
            CliError::PlanLimitExceeded {
                message,
                required_plan,
                upgrade_url,
                code,
            } => {
                assert!(message.contains("fleet_ops"));
                assert_eq!(required_plan, "starter");
                assert_eq!(upgrade_url, "https://sekuire.ai/billing");
                assert_eq!(code, "FLEET_OPS_REQUIRES_UPGRADE");
            }
            _ => panic!("Expected PlanLimitExceeded variant"),
        }
    }

    #[test]
    fn parse_plan_limit_error_non_402_returns_none() {
        let body = r#"{"error": "plan_limit_exceeded", "message": "test"}"#;
        assert!(parse_plan_limit_error(400, body).is_none());
        assert!(parse_plan_limit_error(403, body).is_none());
        assert!(parse_plan_limit_error(500, body).is_none());
        assert!(parse_plan_limit_error(200, body).is_none());
    }

    #[test]
    fn parse_plan_limit_error_wrong_error_type() {
        let body = r#"{"error": "not_found", "message": "Resource not found"}"#;
        assert!(parse_plan_limit_error(402, body).is_none());
    }

    #[test]
    fn parse_plan_limit_error_invalid_json() {
        assert!(parse_plan_limit_error(402, "not json").is_none());
        assert!(parse_plan_limit_error(402, "").is_none());
    }

    #[test]
    fn parse_plan_limit_error_missing_fields_uses_defaults() {
        let body = r#"{"error": "plan_limit_exceeded"}"#;
        let result = parse_plan_limit_error(402, body);
        assert!(result.is_some());
        match result.unwrap() {
            CliError::PlanLimitExceeded {
                message,
                required_plan,
                upgrade_url,
                code,
            } => {
                assert_eq!(message, "Plan limit exceeded");
                assert_eq!(required_plan, "unknown");
                assert_eq!(upgrade_url, "https://sekuire.ai/billing");
                assert_eq!(code, "PLAN_LIMIT_EXCEEDED");
            }
            _ => panic!("Expected PlanLimitExceeded variant"),
        }
    }

    #[test]
    fn parse_plan_limit_error_with_usage() {
        let body = r#"{
            "error": "plan_limit_exceeded",
            "message": "You have reached the workspaces limit (3/3).",
            "code": "WORKSPACES_REQUIRES_UPGRADE",
            "required_plan": "professional",
            "upgrade_url": "https://sekuire.ai/billing",
            "current_usage": {"current": 3, "max": 3}
        }"#;
        let result = parse_plan_limit_error(402, body);
        assert!(result.is_some());
        match result.unwrap() {
            CliError::PlanLimitExceeded {
                message,
                required_plan,
                ..
            } => {
                assert!(message.contains("3/3"));
                assert_eq!(required_plan, "professional");
            }
            _ => panic!("Expected PlanLimitExceeded variant"),
        }
    }

    #[test]
    fn plan_limit_exceeded_display() {
        let err = CliError::PlanLimitExceeded {
            message: "Your plan does not include fleet_ops.".into(),
            required_plan: "starter".into(),
            upgrade_url: "https://sekuire.ai/billing".into(),
            code: "FLEET_OPS_REQUIRES_UPGRADE".into(),
        };
        assert_eq!(err.to_string(), "Your plan does not include fleet_ops.");
    }
}
