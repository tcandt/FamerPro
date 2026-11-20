use anyhow::{bail, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct SecretRedactor {
    pattern: Regex,
}

impl Default for SecretRedactor {
    fn default() -> Self {
        // Redacts patterns matching tokens, bearer keys, passwords
        let pattern = Regex::new(r#"(?i)(password|token|secret|key|authorization|bearer)\s*[:=]\s*["']?([A-Za-z0-9_\-\.]{6,})["']?"#)
            .unwrap();
        Self { pattern }
    }
}

impl SecretRedactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn redact(&self, input: &str) -> String {
        self.pattern
            .replace_all(input, "$1=***REDACTED***")
            .to_string()
    }
}

#[derive(Clone, Default)]
pub struct CredentialVault {
    // In-memory mock or wrapper for Windows Credential Manager
    store: Arc<Mutex<HashMap<String, String>>>,
}

impl CredentialVault {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_secret(&self, key: &str, secret: &str) {
        let mut map = self.store.lock().unwrap();
        map.insert(key.to_string(), secret.to_string());
    }

    pub fn get_secret(&self, key: &str) -> Option<String> {
        let map = self.store.lock().unwrap();
        map.get(key).cloned()
    }
}

pub struct DestructiveActionGuard;

impl DestructiveActionGuard {
    pub fn verify_destructive_permission(
        action_name: &str,
        confirmation_token: Option<&str>,
    ) -> Result<()> {
        match action_name {
            "clear_app_data" | "reboot_device" | "wipe_cache" => {
                if let Some(token) = confirmation_token {
                    if token == "CONFIRM_DESTRUCTIVE_ACTION" {
                        return Ok(());
                    }
                }
                bail!(
                    "Destructive operation '{}' blocked: requires explicit confirmation token",
                    action_name
                );
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let redactor = SecretRedactor::new();
        let log = "Connecting with token: my_secret_token_123456 and password=super_secret_pass";
        let redacted = redactor.redact(log);
        assert!(!redacted.contains("my_secret_token_123456"));
        assert!(!redacted.contains("super_secret_pass"));
        assert!(redacted.contains("***REDACTED***"));
    }

    #[test]
    fn test_destructive_action_guard() {
        let res = DestructiveActionGuard::verify_destructive_permission("clear_app_data", None);
        assert!(res.is_err());

        let res_ok = DestructiveActionGuard::verify_destructive_permission(
            "clear_app_data",
            Some("CONFIRM_DESTRUCTIVE_ACTION"),
        );
        assert!(res_ok.is_ok());
    }
}
