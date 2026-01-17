#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Local,
    Production,
}

impl AppMode {
    pub fn from_env() -> Self {
        match std::env::var("APP_MODE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "local" => AppMode::Local,
            _ => AppMode::Production, // Default to production for safety
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_mode_defaults_to_production() {
        std::env::remove_var("APP_MODE");
        assert_eq!(AppMode::from_env(), AppMode::Production);
    }

    #[test]
    fn test_app_mode_local() {
        std::env::set_var("APP_MODE", "local");
        assert_eq!(AppMode::from_env(), AppMode::Local);
        std::env::remove_var("APP_MODE");
    }

    #[test]
    fn test_app_mode_case_insensitive() {
        std::env::set_var("APP_MODE", "LOCAL");
        assert_eq!(AppMode::from_env(), AppMode::Local);
        std::env::remove_var("APP_MODE");
    }

    #[test]
    fn test_app_mode_invalid_defaults_to_production() {
        std::env::set_var("APP_MODE", "invalid");
        assert_eq!(AppMode::from_env(), AppMode::Production);
        std::env::remove_var("APP_MODE");
    }
}
