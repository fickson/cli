use sha2::{Digest, Sha256};
use sysinfo::System;

/// Get a stable device fingerprint based on hostname, OS, and architecture
pub fn get_device_fingerprint() -> String {
    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Create fingerprint from stable hardware/OS info
    let fingerprint_data = format!("{}|{}|{}", hostname, os, arch);
    let mut hasher = Sha256::new();
    hasher.update(fingerprint_data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Get human-readable device name
pub fn get_device_name() -> String {
    System::host_name().unwrap_or_else(|| "Unknown Device".to_string())
}

/// Get device OS and architecture
pub fn get_device_os() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_fingerprint_is_stable() {
        let fp1 = get_device_fingerprint();
        let fp2 = get_device_fingerprint();
        assert_eq!(fp1, fp2, "Device fingerprint should be stable");
        assert!(!fp1.is_empty(), "Device fingerprint should not be empty");
    }

    #[test]
    fn test_device_name() {
        let name = get_device_name();
        assert!(!name.is_empty(), "Device name should not be empty");
    }

    #[test]
    fn test_device_os() {
        let os = get_device_os();
        assert!(!os.is_empty(), "Device OS should not be empty");
        assert!(os.contains("macos") || os.contains("linux") || os.contains("windows"));
    }
}
