use crate::config;
use crate::error::CliError;
use crate::services::auth::{calculate_identity_hash, load_auth_session, redact_token};
use crate::services::install::{bootstrap_installation, create_install_token, send_heartbeat};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    cmd: Option<String>,
    workspace: Option<String>,
    backend: String,
    agent: Option<String>,
    install_token: Option<String>,
    upstream_url: Option<String>,
    capability: Vec<String>,
    heartbeat_interval: i64,
    no_heartbeat: bool,
) -> Result<(), CliError> {
    let (sekuire_id, manifest) = calculate_identity_hash().await.map_err(CliError::Other)?;
    let agent_id = agent.unwrap_or(sekuire_id);

    let auth = load_auth_session()
        .await
        .map_err(|_| CliError::NotAuthenticated)?;
    let workspace_id = if let Some(ws) = workspace {
        ws
    } else {
        auth.workspace_id.ok_or_else(|| {
            CliError::InvalidInput(
                "Workspace ID required. Run `sekuire config set workspace.id <id>` or pass --workspace.".into(),
            )
        })?
    };

    let upstream = upstream_url
        .or_else(|| manifest.a2a.as_ref().map(|a| a.upstream_url.clone()))
        .unwrap_or_else(|| config::DEFAULT_UPSTREAM_URL.to_string());

    let capabilities = if !capability.is_empty() {
        capability
    } else {
        manifest
            .capabilities
            .as_ref()
            .map(|c| c.provides.clone())
            .unwrap_or_default()
    };

    let install_token_value = if let Some(token) = install_token {
        token
    } else {
        let token = create_install_token(&backend, &workspace_id, &agent_id, None)
            .await
            .map_err(CliError::Other)?;
        token.token
    };

    let bootstrap = bootstrap_installation(
        &backend,
        &install_token_value,
        &upstream,
        &capabilities,
        heartbeat_interval,
    )
    .await
    .map_err(CliError::Other)?;

    println!("Installation bootstrapped");
    println!("   Workspace: {}", workspace_id);
    println!("   Agent ID:  {}", agent_id);
    println!("   Install:   {}", bootstrap.installation_id);
    println!("   Runtime:   {}", redact_token(&bootstrap.runtime_token));

    if !no_heartbeat {
        let _ = send_heartbeat(
            &backend,
            &bootstrap.installation_id,
            &bootstrap.runtime_token,
            heartbeat_interval,
        )
        .await;
    }

    let run_cmd = if let Some(c) = cmd {
        c
    } else if Path::new("package.json").exists() {
        "npm start".to_string()
    } else if Path::new("Cargo.toml").exists() {
        "cargo run".to_string()
    } else if Path::new("main.py").exists() {
        "python main.py".to_string()
    } else {
        return Err(CliError::InvalidInput(
            "Could not determine run command. Use --cmd".into(),
        ));
    };

    println!("   Agent Command: {}", run_cmd);

    let parts: Vec<&str> = run_cmd.split_whitespace().collect();
    let mut child = tokio::process::Command::new(parts[0])
        .args(&parts[1..])
        .env("SEKUIRE_WORKSPACE_ID", &workspace_id)
        .env("SEKUIRE_AUTH_TOKEN", &bootstrap.runtime_token)
        .env("SEKUIRE_RUNTIME_TOKEN", &bootstrap.runtime_token)
        .env("SEKUIRE_API_URL", &backend)
        .env("CORE_API_URL", &backend)
        .env("ENABLE_RUNTIME_REGISTRATION", "false")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to spawn agent process: {}", e)))?;

    let child_pid = Arc::new(AtomicU32::new(child.id().unwrap_or(0)));

    let heartbeat_task = if !no_heartbeat {
        let backend = backend.clone();
        let installation_id = bootstrap.installation_id.clone();
        let runtime_token = bootstrap.runtime_token.clone();
        let pid = Arc::clone(&child_pid);
        Some(tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(heartbeat_interval as u64));
            loop {
                interval.tick().await;
                match send_heartbeat(
                    &backend,
                    &installation_id,
                    &runtime_token,
                    heartbeat_interval,
                )
                .await
                {
                    Ok(Some(cmd)) => {
                        handle_pending_command(&cmd, pid.load(Ordering::Relaxed));
                    }
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("Heartbeat failed: {}", err);
                    }
                }
            }
        }))
    } else {
        None
    };

    let status = child.wait().await.map_err(|e| CliError::Other(e.into()))?;
    if let Some(task) = heartbeat_task {
        task.abort();
    }
    println!("Agent exited with status: {}", status);
    Ok(())
}

#[cfg(unix)]
fn handle_pending_command(cmd: &str, pid: u32) {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    if pid == 0 {
        return;
    }
    let nix_pid = Pid::from_raw(pid as i32);
    match cmd {
        "pause" => {
            eprintln!("[sekuire] Received pause command, sending SIGSTOP to agent");
            let _ = signal::kill(nix_pid, Signal::SIGSTOP);
        }
        "resume" => {
            eprintln!("[sekuire] Received resume command, sending SIGCONT to agent");
            let _ = signal::kill(nix_pid, Signal::SIGCONT);
        }
        "terminate" => {
            eprintln!("[sekuire] Received terminate command, sending SIGTERM to agent");
            let _ = signal::kill(nix_pid, Signal::SIGTERM);
            std::process::exit(0);
        }
        _ => {
            eprintln!("[sekuire] Unknown pending command: {}", cmd);
        }
    }
}

#[cfg(not(unix))]
fn handle_pending_command(cmd: &str, _pid: u32) {
    match cmd {
        "terminate" => {
            eprintln!("[sekuire] Received terminate command");
            std::process::exit(0);
        }
        other => {
            eprintln!(
                "[sekuire] Pending command '{}' not supported on this platform",
                other
            );
        }
    }
}
