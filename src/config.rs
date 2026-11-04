use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub steel_base_url: String,
    pub steel_api_key: Option<String>,
    pub steel_session_timeout_ms: u64,
    pub air_selangor_id: String,
    pub air_selangor_password: String,
    pub air_selangor_id_type: String,
    pub downloads_dir: PathBuf,
    pub portal_url: String,
    pub session_dir: PathBuf,
    pub session_state_file: PathBuf,
    pub auto_logout: bool,
}

impl AppConfig {
    pub fn load() -> Self {
        // Attempt loading .env
        let _ = dotenvy::dotenv();

        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let session_dir = base_dir.join(".session");
        let session_state_file = session_dir.join("storageState.json");
        let downloads_dir = std::env::var("DOWNLOADS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base_dir.join("downloads"));

        let auto_logout = std::env::var("AUTO_LOGOUT")
            .map(|v| {
                let v = v.trim().to_lowercase();
                v == "true" || v == "1" || v == "yes"
            })
            .unwrap_or(false);

        Self {
            steel_base_url: std::env::var("STEEL_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            steel_api_key: std::env::var("STEEL_API_KEY").ok().filter(|s| !s.is_empty()),
            steel_session_timeout_ms: std::env::var("STEEL_SESSION_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(900_000), // 15 mins default
            air_selangor_id: std::env::var("AIR_SELANGOR_ID").unwrap_or_default(),
            air_selangor_password: std::env::var("AIR_SELANGOR_PASSWORD").unwrap_or_default(),
            air_selangor_id_type: std::env::var("AIR_SELANGOR_ID_TYPE")
                .unwrap_or_else(|_| "nric".to_string())
                .to_lowercase(),
            downloads_dir,
            portal_url: "https://www.airselangor.com".to_string(),
            session_dir,
            session_state_file,
            auto_logout,
        }
    }

    /// Checks if Air Selangor login credentials (ID and password) are configured.
    pub fn has_credentials(&self) -> bool {
        !self.air_selangor_id.trim().is_empty() && !self.air_selangor_password.trim().is_empty()
    }

    /// Checks if a saved authenticated session state file exists.
    pub fn has_session_state(&self) -> bool {
        self.session_state_file.exists()
    }

    /// Checks if the server can authenticate (either via credentials or saved session).
    pub fn is_ready_for_auth(&self) -> bool {
        self.has_credentials() || self.has_session_state()
    }
}
