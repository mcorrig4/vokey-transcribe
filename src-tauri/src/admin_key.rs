//! Secure storage for OpenAI Admin API key using system keyring.
//!
//! The admin API key is stored in the OS's native secret storage:
//! - Linux: libsecret (GNOME Keyring/KDE Wallet)
//! - macOS: Keychain
//! - Windows: Credential Manager
//!
//! Security notes:
//! - Never log the key value
//! - Always use masked display in UI
//! - Key is encrypted at rest by OS

use keyring::Entry;

const SERVICE_NAME: &str = "vokey-transcribe";
const ADMIN_KEY_NAME: &str = "openai-admin-api-key";

/// Retrieve the stored admin API key, if any.
/// Returns None if not configured or on error (errors are logged).
pub fn get_admin_api_key() -> Option<String> {
    let entry = match Entry::new(SERVICE_NAME, ADMIN_KEY_NAME) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("AdminKey: failed to create keyring entry: {}", e);
            return None;
        }
    };

    match entry.get_password() {
        Ok(key) => {
            if key.is_empty() {
                None
            } else {
                Some(key)
            }
        }
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            log::warn!("AdminKey: failed to retrieve key: {}", e);
            None
        }
    }
}

/// Store the admin API key in the system keyring.
/// Pass None to delete the key.
pub fn set_admin_api_key(key: Option<&str>) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, ADMIN_KEY_NAME)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    match key {
        Some(k) if !k.is_empty() => {
            entry
                .set_password(k)
                .map_err(|e| format!("Failed to store admin key: {}", e))?;
            // Log action without the key value
            log::info!("AdminKey: stored new admin API key");
        }
        _ => {
            // Delete the key
            match entry.delete_credential() {
                Ok(()) => log::info!("AdminKey: deleted admin API key"),
                Err(keyring::Error::NoEntry) => {
                    // Already deleted, that's fine
                }
                Err(e) => return Err(format!("Failed to delete admin key: {}", e)),
            }
        }
    }

    Ok(())
}

/// Validate an admin API key by making a test request to OpenAI Usage API.
/// Returns Ok(true) if the key has usage read permissions.
/// Returns Ok(false) if the key is invalid or lacks permissions.
/// Returns Err on network/other errors.
pub async fn validate_admin_api_key(key: &str) -> Result<bool, String> {
    if key.is_empty() {
        return Ok(false);
    }

    // Make a minimal request to the usage API to check permissions
    // We use /v1/organization/costs with a 1-day window to minimize data
    let client = reqwest::Client::new();

    // Get time range for validation (last 24 hours as Unix timestamps)
    let now = chrono::Utc::now();
    let end_time = now.timestamp();
    let start_time = (now - chrono::Duration::days(1)).timestamp();

    let url = format!(
        "https://api.openai.com/v1/organization/costs?start_time={}&end_time={}",
        start_time, end_time
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    match response.status().as_u16() {
        200 => Ok(true),
        401 => {
            log::debug!("AdminKey validation: unauthorized (invalid key)");
            Ok(false)
        }
        403 => {
            log::debug!("AdminKey validation: forbidden (lacks usage read permission)");
            Ok(false)
        }
        status => {
            log::warn!("AdminKey validation: unexpected status {}", status);
            Err(format!("Unexpected API response: {}", status))
        }
    }
}

/// Returns whether an admin API key is currently configured.
pub fn is_admin_key_configured() -> bool {
    get_admin_api_key().is_some()
}

/// Returns a masked version of the key for display (e.g., "sk-...abc123")
pub fn get_masked_admin_key() -> Option<String> {
    get_admin_api_key().map(|key| {
        if key.len() <= 8 {
            "*".repeat(key.len())
        } else {
            format!("{}...{}", &key[..3], &key[key.len() - 6..])
        }
    })
}
