use crate::error::CliError;
use crate::ui;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use sekuire_protocol::{AgentResponse, HandshakeHello, HandshakeWelcome};

pub async fn execute(url: String, id: Option<String>, registry: String) -> Result<(), CliError> {
    println!("Verifying agent at {}...", url);

    let mut csprng = OsRng;
    let mut nonce_bytes = [0u8; 32];
    csprng.fill_bytes(&mut nonce_bytes);
    let client_nonce = hex::encode(nonce_bytes);

    let hello = HandshakeHello {
        client_nonce: client_nonce.clone(),
    };

    let client = reqwest::Client::new();
    let handshake_url = format!("{}/sekuire/handshake", url.trim_end_matches('/'));

    let resp = ui::with_spinner_if(ui::spinner_enabled(None), "Contacting agent...", async {
        client.post(&handshake_url).json(&hello).send().await
    })
    .await
    .map_err(|e| CliError::VerificationFailed(format!("Failed to connect to agent: {}", e)))?;

    if !resp.status().is_success() {
        return Err(CliError::VerificationFailed(format!(
            "Handshake failed: Status {}",
            resp.status()
        )));
    }

    let welcome: HandshakeWelcome = resp
        .json()
        .await
        .map_err(|e| CliError::VerificationFailed(format!("Invalid handshake response: {}", e)))?;

    let registry_url = format!(
        "{}/api/v1/agents/{}",
        registry.trim_end_matches('/'),
        welcome.agent_id
    );
    let registry_resp = ui::with_spinner_if(
        ui::spinner_enabled(None),
        "Verifying identity in registry...",
        async { client.get(&registry_url).send().await },
    )
    .await
    .map_err(|e| CliError::VerificationFailed(format!("Failed to contact registry: {}", e)))?;

    if !registry_resp.status().is_success() {
        return Err(CliError::VerificationFailed(format!(
            "Agent not found in registry ({}): {}",
            welcome.agent_id,
            registry_resp.status()
        )));
    }

    let agent_info: AgentResponse = registry_resp
        .json()
        .await
        .map_err(|e| CliError::VerificationFailed(format!("Invalid registry response: {}", e)))?;

    println!(
        "   Found Agent: {} (v{})",
        agent_info.name, agent_info.version
    );

    let pub_key_bytes = hex::decode(&agent_info.public_key).map_err(|_| {
        CliError::VerificationFailed("Invalid public key format in registry".into())
    })?;

    let signature_bytes = hex::decode(&welcome.signature_c).map_err(|_| {
        CliError::VerificationFailed("Invalid signature format in handshake".into())
    })?;

    let pub_key_array: &[u8; 32] = pub_key_bytes.as_slice().try_into().map_err(|_| {
        CliError::VerificationFailed("Invalid public key length (expected 32 bytes)".into())
    })?;

    let verifying_key = VerifyingKey::from_bytes(pub_key_array)
        .map_err(|e| CliError::VerificationFailed(format!("Invalid public key: {}", e)))?;

    let signature_array: &[u8; 64] = signature_bytes.as_slice().try_into().map_err(|_| {
        CliError::VerificationFailed("Invalid signature length (expected 64 bytes)".into())
    })?;

    let signature = Signature::from_bytes(signature_array);

    verifying_key
        .verify(client_nonce.as_bytes(), &signature)
        .map_err(|e| {
            CliError::VerificationFailed(format!(
                "CRYPTOGRAPHIC VERIFICATION FAILED! The agent failed to sign the challenge: {}",
                e
            ))
        })?;

    println!("Cryptographic Proof Valid (Identity Verified)");

    if let Some(expected_id) = id {
        if expected_id != welcome.agent_id {
            return Err(CliError::VerificationFailed(format!(
                "ID Mismatch! Expected: {}, Actual: {}",
                expected_id, welcome.agent_id
            )));
        }
    }

    println!("Agent Verified Successfully!");
    println!("   Name:       {}", agent_info.name);
    println!("   Reputation: {}/100", agent_info.reputation_score);
    println!("   Status:     {:?}", agent_info.verification_status);
    println!("   ID:         {}", agent_info.sekuire_id);

    if agent_info.reputation_score < 50 {
        println!("WARNING: Low Reputation Score");
    }
    Ok(())
}
