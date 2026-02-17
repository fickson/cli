use crate::config;
use crate::error::CliError;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::path::Path;
use tokio::fs;

pub async fn execute(force: bool) -> Result<(), CliError> {
    let sekuiredir = Path::new(config::SEKUIRE_DIR);
    if !sekuiredir.exists() {
        fs::create_dir(sekuiredir).await?;
    }
    let key_path = sekuiredir.join(config::SECRET_KEY_FILE);
    let pub_path = sekuiredir.join(config::PUBLIC_KEY_FILE);

    if key_path.exists() && !force {
        return Err(CliError::InvalidInput(
            "Keys already exist in .sekuire/. Use --force to overwrite.".into(),
        ));
    }

    println!("Generating Ed25519 Keypair...");
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let priv_hex = hex::encode(signing_key.to_bytes());
    let pub_hex = hex::encode(verifying_key.as_bytes());

    fs::write(key_path, &priv_hex).await?;
    fs::write(pub_path, &pub_hex).await?;

    println!("Keys generated!");
    println!("   Private: .sekuire/secret.key (KEEP SAFE)");
    println!("   Public:  .sekuire/public.key");
    println!("   Public Key Hash: {}", pub_hex);
    Ok(())
}
