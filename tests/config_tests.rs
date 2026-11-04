use air_selangor_mcp::config::AppConfig;
use std::path::PathBuf;

#[test]
fn test_app_config_load() {
    let config = AppConfig::load();
    assert!(!config.portal_url.is_empty());
    assert_eq!(config.portal_url, "https://www.airselangor.com");
    assert!(config.steel_session_timeout_ms > 0);
    assert!(!config.steel_base_url.is_empty());
    assert!(config.session_dir.ends_with(".session"));
    assert!(config.session_state_file.ends_with("storageState.json"));
}

#[test]
fn test_config_clone() {
    let config = AppConfig::load();
    let cloned = config.clone();
    assert_eq!(cloned.portal_url, config.portal_url);
    assert_eq!(cloned.steel_base_url, config.steel_base_url);
    assert_eq!(cloned.steel_session_timeout_ms, config.steel_session_timeout_ms);
}

#[test]
fn test_credential_guards() {
    let mut config = AppConfig {
        steel_base_url: "http://localhost:3000".to_string(),
        steel_api_key: None,
        steel_session_timeout_ms: 900_000,
        air_selangor_id: "".to_string(),
        air_selangor_password: "".to_string(),
        air_selangor_id_type: "nric".to_string(),
        downloads_dir: PathBuf::from("./downloads"),
        portal_url: "https://www.airselangor.com".to_string(),
        session_dir: PathBuf::from("/nonexistent/session"),
        session_state_file: PathBuf::from("/nonexistent/session/storageState.json"),
        auto_logout: false,
    };

    assert!(!config.has_credentials());
    assert!(!config.has_session_state());
    assert!(!config.is_ready_for_auth());

    config.air_selangor_id = "USER123".to_string();
    assert!(!config.has_credentials(), "Password is still missing");
    assert!(!config.is_ready_for_auth());

    config.air_selangor_password = "PASS".to_string();
    assert!(config.has_credentials());
    assert!(config.is_ready_for_auth());
}
