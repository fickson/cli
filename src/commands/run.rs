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

    let (program, args, env_overrides) = parse_run_command(&run_cmd)?;
    let mut command = tokio::process::Command::new(program);
    command.args(args);

    for (key, value) in env_overrides {
        command.env(key, value);
    }

    let mut child = command
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

fn parse_run_command(
    run_cmd: &str,
) -> Result<(String, Vec<String>, Vec<(String, String)>), CliError> {
    let parts = shell_words::split(run_cmd).map_err(|err| {
        CliError::InvalidInput(format!(
            "Could not parse --cmd. Check quoting and escaping: {}",
            err
        ))
    })?;

    if parts.is_empty() {
        return Err(CliError::InvalidInput(
            "Run command cannot be empty. Use --cmd '<command>'".into(),
        ));
    }

    let mut env_overrides = Vec::new();
    let mut command_index = 0usize;

    while command_index < parts.len() {
        if let Some((key, value)) = parse_env_assignment(&parts[command_index]) {
            env_overrides.push((key.to_string(), value.to_string()));
            command_index += 1;
        } else {
            break;
        }
    }

    if command_index >= parts.len() {
        return Err(CliError::InvalidInput(
            "Run command must include an executable after env assignments".into(),
        ));
    }

    let program = parts[command_index].clone();
    let args = parts[command_index + 1..].to_vec();

    Ok((program, args, env_overrides))
}

fn parse_env_assignment(token: &str) -> Option<(&str, &str)> {
    let (key, value) = token.split_once('=')?;
    if !is_valid_env_key(key) {
        return None;
    }
    Some((key, value))
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if first != '_' && !first.is_ascii_alphabetic() {
        return false;
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::parse_run_command;

    #[test]
    fn parses_simple_command() {
        let (program, args, envs) = parse_run_command("npm start").expect("command should parse");
        assert_eq!(program, "npm");
        assert_eq!(args, vec!["start".to_string()]);
        assert!(envs.is_empty());
    }

    #[test]
    fn parses_command_with_env_prefix() {
        let (program, args, envs) =
            parse_run_command("PORT=8011 NODE_ENV=production node dist/index.js")
                .expect("command should parse");
        assert_eq!(program, "node");
        assert_eq!(args, vec!["dist/index.js".to_string()]);
        assert_eq!(
            envs,
            vec![
                ("PORT".to_string(), "8011".to_string()),
                ("NODE_ENV".to_string(), "production".to_string())
            ]
        );
    }

    #[test]
    fn parses_quoted_arguments() {
        let (program, args, envs) =
            parse_run_command("node --title \"my agent\" app.js").expect("command should parse");
        assert_eq!(program, "node");
        assert_eq!(
            args,
            vec![
                "--title".to_string(),
                "my agent".to_string(),
                "app.js".to_string()
            ]
        );
        assert!(envs.is_empty());
    }

    #[test]
    fn rejects_env_only_command() {
        let err = parse_run_command("PORT=8011").expect_err("must fail without executable");
        assert!(err
            .to_string()
            .contains("must include an executable after env assignments"));
    }
}
