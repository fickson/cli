use crate::error::CliError;
use crate::services::http_client::SekuireClient;
use serde::Deserialize;

#[derive(Deserialize)]
struct AccountLimitsResponse {
    plan: String,
    limits: AccountLimits,
    features: AccountFeatures,
    upgrade_url: String,
}

#[derive(Deserialize)]
struct AccountLimits {
    workspaces: ResourceUsage,
    members: ResourceUsage,
    agents: ResourceUsage,
    audit_retention_days: Option<i32>,
}

#[derive(Deserialize)]
struct ResourceUsage {
    current: i64,
    max: Option<i32>,
}

#[derive(Deserialize)]
struct AccountFeatures {
    fleet_ops: bool,
    policies: bool,
    ai_policy_gen: bool,
    a2a_routing: bool,
    sso: bool,
    compliance: bool,
}

pub async fn execute_limits(registry: String) -> Result<(), CliError> {
    let client = SekuireClient::from_registry(&registry).await?;
    let resp = client.get_checked("/api/v1/account/limits").await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::RegistryError {
            status: status.as_u16(),
            body,
        });
    }

    let data: AccountLimitsResponse = resp
        .json()
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to parse account limits: {}", e)))?;

    println!("Plan: {}", data.plan.to_uppercase());
    println!();

    println!("Resource Limits:");
    print_usage("  Workspaces", &data.limits.workspaces);
    print_usage("  Members", &data.limits.members);
    print_usage("  Agents", &data.limits.agents);
    if let Some(days) = data.limits.audit_retention_days {
        println!("  Audit Retention: {} days", days);
    }

    println!();
    println!("Features:");
    print_feature(
        "  Fleet Ops (pause/resume/terminate)",
        data.features.fleet_ops,
    );
    print_feature("  Policies", data.features.policies);
    print_feature("  AI Policy Generation", data.features.ai_policy_gen);
    print_feature("  A2A Routing", data.features.a2a_routing);
    print_feature("  SSO", data.features.sso);
    print_feature("  Compliance", data.features.compliance);

    println!();
    println!("Upgrade: {}", data.upgrade_url);

    Ok(())
}

fn print_usage(label: &str, usage: &ResourceUsage) {
    match usage.max {
        Some(max) => println!("{}: {}/{}", label, usage.current, max),
        None => println!("{}: {} (unlimited)", label, usage.current),
    }
}

fn print_feature(label: &str, enabled: bool) {
    if enabled {
        println!("{}: enabled", label);
    } else {
        println!("{}: -", label);
    }
}
