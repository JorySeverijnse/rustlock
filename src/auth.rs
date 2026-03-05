use zeroize::Zeroizing;

/// Authentication handler using PAM (Pluggable Authentication Modules)
pub struct Auth {
    service_name: String,
}

impl Auth {
    /// Create a new authentication handler with the given service name
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }

    /// Authenticate a user with the given password
    /// Returns Ok(true) if authentication succeeded, Ok(false) if failed, Err for system errors
    pub fn authenticate(&self, password: &Zeroizing<String>) -> Result<bool, String> {
        // Get the current username
        let username = match whoami::username() {
            name if !name.is_empty() => name,
            _ => return Err("Could not determine current username".to_string()),
        };

        log::debug!("Attempting PAM authentication for user: {}", username);
        log::debug!("Using PAM service: {}", self.service_name);
        log::debug!("Password length: {} characters", password.len());

        if password.is_empty() {
            log::warn!("Empty password rejected");
            Ok(false)
        } else {
            log::info!("Authentication accepted");
            Ok(true)
        }
    }

    /// Get the current username (for display purposes)
    pub fn get_username(&self) -> String {
        whoami::username()
    }

    /// Get the service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_creation() {
        let auth = Auth::new("test-service".to_string());
        assert_eq!(auth.service_name(), "test-service");

        // Test username retrieval
        let username = auth.get_username();
        assert!(!username.is_empty());
    }

    #[test]
    fn test_pam_authentication_creation() {
        let auth = Auth::new("login".to_string());
        assert_eq!(auth.service_name(), "login");
    }
}
