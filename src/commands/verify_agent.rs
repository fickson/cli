use crate::error::CliError;
use crate::services::auth::calculate_identity_hash;
use crate::services::http_client::SekuireClient;

pub async fn execute(
    registry: String,
    repo_url: Option<String>,
    compliance: Option<String>,
) -> Result<(), CliError> {
    println!("Comprehensive Agent Verification (\"The Bar Exam\")");

    let (sekuire_id, manifest) = calculate_identity_hash().await.map_err(CliError::Other)?;
    println!("Agent ID: {}", sekuire_id);

    let repository_url = if let Some(url) = repo_url {
        url
    } else {
        match std::process::Command::new("git")
            .args(["config", "--get", "remote.origin.url"])
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => {
                return Err(CliError::InvalidInput(
                    "No repository URL provided and git remote not found. Use --repo-url.".into(),
                ));
            }
        }
    };

    println!("Repository: {}", repository_url);

    let client = SekuireClient::from_registry(&registry).await?;

    let verification_request = serde_json::json!({
        "sekuire_id": sekuire_id,
        "repository_url": repository_url,
        "compliance_framework": compliance,
        "manifest": manifest
    });

    println!("\nStarting comprehensive verification...");
    println!("   - Repository structure check");
    println!("   - Static code analysis");
    println!("   - CVE scanning");
    if let Some(ref framework) = compliance {
        println!("   - {} compliance validation", framework.to_uppercase());
    }
    println!();

    let response = client
        .post("/api/v1/agents/verify/comprehensive", &verification_request)
        .await
        .map_err(|e| {
            CliError::VerificationFailed(format!("Failed to connect to registry: {}", e))
        })?;

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(CliError::VerificationFailed(error_body));
    }

    let report: serde_json::Value = response
        .json()
        .await
        .map_err(|e| CliError::Other(e.into()))?;

    let passed = report["passed"].as_bool().unwrap_or(false);
    let security_score = report["security_score"].as_i64().unwrap_or(0);
    let compliance_score = report["compliance_score"].as_i64().unwrap_or(0);

    println!("VERIFICATION RESULTS");
    println!("   Security Score:    {}/100", security_score);
    println!("   Compliance Score:  {}/100", compliance_score);
    println!();

    if let Some(summary) = report["issues_summary"].as_object() {
        let critical = summary["critical"].as_i64().unwrap_or(0);
        let high = summary["high"].as_i64().unwrap_or(0);
        let medium = summary["medium"].as_i64().unwrap_or(0);
        let low = summary["low"].as_i64().unwrap_or(0);

        if critical + high + medium + low > 0 {
            println!("Issues Found:");
            if critical > 0 {
                println!("   Critical: {}", critical);
            }
            if high > 0 {
                println!("   High:     {}", high);
            }
            if medium > 0 {
                println!("   Medium:   {}", medium);
            }
            if low > 0 {
                println!("   Low:      {}", low);
            }
            println!();
        }
    }

    if let Some(recommendations) = report["recommendations"].as_array() {
        if !recommendations.is_empty() {
            println!("Recommendations:");
            for rec in recommendations {
                if let Some(text) = rec.as_str() {
                    println!("   - {}", text);
                }
            }
            println!();
        }
    }

    if passed {
        println!("VERIFICATION PASSED");
        println!("\n   Your agent meets the security and compliance requirements.");
    } else {
        return Err(CliError::VerificationFailed(format!(
            "Security: {}/100 (min 70), Compliance: {}/100 (min 60), no critical issues allowed.",
            security_score, compliance_score
        )));
    }
    Ok(())
}
