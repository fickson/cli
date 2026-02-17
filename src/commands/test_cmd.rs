use crate::error::CliError;
use crate::services::auth::calculate_identity_hash;
use crate::services::policy::{load_policy_file, verify_active_policy};
use rand::rngs::OsRng;
use rand::RngCore;
use std::path::Path;

pub async fn execute(
    suite: String,
    _registry: String,
    fail_fast: bool,
    agent_url: String,
) -> Result<(), CliError> {
    println!("Running Sekuire Agent Tests");
    println!("   Suite: {}", suite);
    println!("   Agent URL: {}", agent_url);
    println!();

    let mut passed = 0u32;
    let mut failed = 0u32;
    let suites: Vec<&str> = if suite == "all" {
        vec!["manifest", "policy", "endpoints", "handshake"]
    } else {
        vec![suite.as_str()]
    };

    for test_suite in suites {
        match test_suite {
            "manifest" => {
                print!("   [manifest] Validating sekuire.yml... ");
                match calculate_identity_hash().await {
                    Ok((sekuire_id, manifest)) => {
                        println!("PASS");
                        println!("      Sekuire ID: {}", sekuire_id);
                        println!(
                            "      Agent: {} v{}",
                            manifest.project.name, manifest.project.version
                        );
                        passed += 1;
                    }
                    Err(e) => {
                        println!("FAIL");
                        println!("      Error: {}", e);
                        failed += 1;
                        if fail_fast {
                            return Err(CliError::VerificationFailed(
                                "Manifest validation failed".into(),
                            ));
                        }
                    }
                }
            }
            "policy" => {
                print!("   [policy] Checking policy.json... ");
                let policy_path = Path::new("policy.json");
                if policy_path.exists() {
                    match load_policy_file("policy.json").await {
                        Ok(policy) => match verify_active_policy(&policy) {
                            Ok(_) => {
                                println!("PASS");
                                println!("      Policy: {} v{}", policy.policy_id, policy.version);
                                passed += 1;
                            }
                            Err(e) => {
                                println!("FAIL");
                                println!("      Verification error: {}", e);
                                failed += 1;
                                if fail_fast {
                                    return Err(CliError::VerificationFailed(
                                        "Policy verification failed".into(),
                                    ));
                                }
                            }
                        },
                        Err(e) => {
                            println!("FAIL");
                            println!("      Parse error: {}", e);
                            failed += 1;
                            if fail_fast {
                                return Err(CliError::VerificationFailed(
                                    "Policy parse failed".into(),
                                ));
                            }
                        }
                    }
                } else {
                    println!("SKIP (no policy.json)");
                }
            }
            "endpoints" => {
                print!("   [endpoints] Checking agent endpoints... ");
                let client = reqwest::Client::new();
                let health_url = format!("{}/health", agent_url.trim_end_matches('/'));
                match client
                    .get(&health_url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        println!("PASS");
                        println!("      /health: OK");
                        passed += 1;

                        let metrics_url = format!("{}/metrics", agent_url.trim_end_matches('/'));
                        let info_url = format!("{}/agent/info", agent_url.trim_end_matches('/'));
                        let a2a_url = format!("{}/a2a/tasks", agent_url.trim_end_matches('/'));

                        if let Ok(r) = client
                            .get(&metrics_url)
                            .timeout(std::time::Duration::from_secs(2))
                            .send()
                            .await
                        {
                            println!(
                                "      /metrics: {}",
                                if r.status().is_success() {
                                    "OK"
                                } else {
                                    "WARN"
                                }
                            );
                        }
                        if let Ok(r) = client
                            .get(&info_url)
                            .timeout(std::time::Duration::from_secs(2))
                            .send()
                            .await
                        {
                            println!(
                                "      /agent/info: {}",
                                if r.status().is_success() {
                                    "OK"
                                } else {
                                    "WARN"
                                }
                            );
                        }
                        if let Ok(r) = client
                            .post(&a2a_url)
                            .json(&serde_json::json!({"task_id": "test", "type": "ping"}))
                            .timeout(std::time::Duration::from_secs(2))
                            .send()
                            .await
                        {
                            let ok = r.status().is_success() || r.status().as_u16() == 400;
                            println!("      /a2a/tasks: {}", if ok { "OK" } else { "WARN" });
                        }
                    }
                    Ok(resp) => {
                        println!("FAIL");
                        println!("      /health returned: {}", resp.status());
                        failed += 1;
                        if fail_fast {
                            return Err(CliError::VerificationFailed(
                                "Endpoint check failed".into(),
                            ));
                        }
                    }
                    Err(e) => {
                        println!("FAIL");
                        println!("      Could not reach agent: {}", e);
                        failed += 1;
                        if fail_fast {
                            return Err(CliError::VerificationFailed(
                                "Could not reach agent".into(),
                            ));
                        }
                    }
                }
            }
            "handshake" => {
                print!("   [handshake] Testing handshake protocol... ");
                let client = reqwest::Client::new();
                let handshake_url =
                    format!("{}/sekuire/handshake", agent_url.trim_end_matches('/'));
                let mut nonce_bytes = [0u8; 32];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = hex::encode(nonce_bytes);

                let hello = serde_json::json!({ "client_nonce": nonce });
                match client
                    .post(&handshake_url)
                    .json(&hello)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        println!("PASS");
                        println!("      Handshake endpoint responding");
                        passed += 1;
                    }
                    Ok(resp) => {
                        println!("WARN");
                        println!("      Handshake returned: {}", resp.status());
                    }
                    Err(_) => {
                        println!("SKIP (no /sekuire/handshake endpoint)");
                    }
                }
            }
            _ => {
                println!("   [{}] Unknown test suite", test_suite);
            }
        }
    }

    println!();
    println!("Test Results: {} passed, {} failed", passed, failed);
    if failed > 0 {
        return Err(CliError::VerificationFailed(format!(
            "{} test(s) failed",
            failed
        )));
    }
    println!("All tests passed!");
    Ok(())
}
