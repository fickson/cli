use crate::error::CliError;
use crate::services::auth::calculate_identity_hash;

pub async fn execute() -> Result<(), CliError> {
    let (hash, _) = calculate_identity_hash().await.map_err(CliError::Other)?;
    println!("Sekuire ID (Fingerprint): {}", hash);
    Ok(())
}
