use anyhow::{Context, Result};
use chromiumoxide::Page;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use crate::config::AppConfig;
use crate::proto::v1::{AuthStatus, LogoutResponse};
use crate::steel::{LocalStorageItem, OriginStorage, SteelManager, StorageState};

pub struct AuthService {
    config: AppConfig,
    steel: Arc<SteelManager>,
}

impl AuthService {
    pub fn new(config: AppConfig, steel: Arc<SteelManager>) -> Self {
        Self { config, steel }
    }

    /// Dismisses any popup promotional modal or overlay banner.
    pub async fn dismiss_modals(&self, page: &Page) {
        let script = r#"
            (() => {
                const closeBtn = document.querySelector('#modal-root button');
                if (closeBtn) closeBtn.click();
                const root = document.getElementById('modal-root');
                if (root) root.innerHTML = '';
            })()
        "#;
        let _ = page.evaluate(script).await;
        sleep(Duration::from_millis(500)).await;
    }

    /// Checks if current session is authenticated.
    pub async fn check_auth(&self) -> Result<AuthStatus> {
        // Guard: Do not trigger Steel if credentials are not configured and no session file exists
        if !self.config.is_ready_for_auth() {
            return Ok(AuthStatus {
                is_authenticated: false,
                user_identifier: String::new(),
                steel_session_id: String::new(),
                session_viewer_url: String::new(),
                message: "Air Selangor credentials not configured. Please set AIR_SELANGOR_ID and AIR_SELANGOR_PASSWORD in .env".to_string(),
            });
        }

        let session = self.steel.get_session(false).await?;
        let page = &session.page;

        let current_url = page.url().await?.unwrap_or_default();
        if !current_url.contains("airselangor.com") {
            let target = format!("{}/my-water-account?lang=en", self.config.portal_url);
            let _ = page.goto(&target).await;
            self.dismiss_modals(page).await;
            sleep(Duration::from_millis(1500)).await;
        }

        let verify_script = r#"
            (async () => {
                try {
                    const token = localStorage.getItem('accesstoken');
                    if (!token) return { valid: false };

                    const res = await fetch('https://api3.airselangor.com/userprofile/v2/view/basic', {
                        method: 'POST',
                        headers: {
                            'language': 'en',
                            'transactionid': crypto.randomUUID(),
                            'authorization': 'Bearer ' + token,
                            'timestamp': new Date().toISOString(),
                            'os-type': 'web',
                            'device-name': 'browser',
                            'content-type': 'application/json'
                        },
                        body: JSON.stringify({})
                    });
                    const data = await res.json();
                    if (data && data.status && data.data) {
                        return {
                            valid: true,
                            name: data.data.name || data.data.idno,
                            idno: data.data.idno
                        };
                    }
                } catch (e) {}
                return { valid: false };
            })()
        "#;

        let eval_res = page.evaluate(verify_script).await?;
        let auth_val: serde_json::Value = eval_res.into_value()?;
        let is_valid = auth_val.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);

        if is_valid {
            let name = auth_val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&self.config.air_selangor_id)
                .to_string();

            return Ok(AuthStatus {
                is_authenticated: true,
                user_identifier: name,
                steel_session_id: session.session_id,
                session_viewer_url: session.session_viewer_url,
                message: "Session is authenticated.".to_string(),
            });
        }

        Ok(AuthStatus {
            is_authenticated: false,
            user_identifier: String::new(),
            steel_session_id: session.session_id,
            session_viewer_url: session.session_viewer_url,
            message: "Session token missing or expired.".to_string(),
        })
    }

    /// Logs into the portal using realistic keystrokes and blur triggers.
    pub async fn login(&self, custom_id: Option<String>, custom_password: Option<String>) -> Result<AuthStatus> {
        let login_id = custom_id.unwrap_or_else(|| self.config.air_selangor_id.clone());
        let password = custom_password.unwrap_or_else(|| self.config.air_selangor_password.clone());

        // Guard: Do not trigger Steel if credentials are not provided
        if login_id.trim().is_empty() || password.trim().is_empty() {
            return Ok(AuthStatus {
                is_authenticated: false,
                user_identifier: String::new(),
                steel_session_id: String::new(),
                session_viewer_url: String::new(),
                message: "Missing credentials. Please set AIR_SELANGOR_ID and AIR_SELANGOR_PASSWORD in .env or provide them as arguments.".to_string(),
            });
        }

        // Check if already authenticated
        if let Ok(status) = self.check_auth().await {
            if status.is_authenticated {
                return Ok(status);
            }
        }

        let session = self.steel.get_session(false).await?;
        let page = &session.page;

        info!("Navigating to Air Selangor login page...");
        let login_url = format!("{}/login?lang=en", self.config.portal_url);
        page.goto(&login_url).await?;
        sleep(Duration::from_millis(2500)).await;
        self.dismiss_modals(page).await;

        // Type Login ID
        let id_elem = page.find_element("input#loginid, input[name='loginid']").await
            .context("Could not find login ID input field")?;
        id_elem.click().await?;
        sleep(Duration::from_millis(200)).await;
        id_elem.type_str(&login_id).await?;
        sleep(Duration::from_millis(100)).await;
        id_elem.press_key("Tab").await?;
        sleep(Duration::from_millis(300)).await;

        // Type Password
        let pass_elem = page.find_element("input#password, input[name='password']").await
            .context("Could not find password input field")?;
        pass_elem.click().await?;
        sleep(Duration::from_millis(200)).await;
        pass_elem.type_str(&password).await?;
        sleep(Duration::from_millis(100)).await;
        pass_elem.press_key("Tab").await?;
        sleep(Duration::from_millis(400)).await;

        // Click Submit
        let submit_elem = page.find_element("button[class*='login_buttonWrapper'], form button[type='submit']").await
            .context("Could not find login submit button")?;
        submit_elem.click().await?;

        // Wait for login processing
        sleep(Duration::from_millis(5000)).await;

        // Check for error messages
        let error_check = r#"
            (() => {
                const el = Array.from(document.querySelectorAll('[class*="error" i], [class*="alert" i], [style*="color: red" i], p, span'))
                    .find(e => /invalid|incorrect|attempt|failed|salah/i.test(e.innerText || ''));
                return el ? el.innerText.trim() : null;
            })()
        "#;
        let err_eval = page.evaluate(error_check).await?;
        let err_val: serde_json::Value = err_eval.into_value()?;
        if let Some(err_msg) = err_val.as_str() {
            return Ok(AuthStatus {
                is_authenticated: false,
                user_identifier: String::new(),
                steel_session_id: session.session_id,
                session_viewer_url: session.session_viewer_url,
                message: format!("Portal error: {}", err_msg),
            });
        }

        // Verify successful login
        let status = self.check_auth().await?;
        if status.is_authenticated {
            // Extract tokens and save to storageState.json
            let extract_script = r#"
                (() => {
                    const items = [];
                    for (let i = 0; i < localStorage.length; i++) {
                        const key = localStorage.key(i);
                        items.push({ name: key, value: localStorage.getItem(key) });
                    }
                    return items;
                })()
            "#;
            if let Ok(eval_res) = page.evaluate(extract_script).await {
                if let Ok(items_val) = eval_res.into_value::<serde_json::Value>() {
                    if let Ok(ls_items) = serde_json::from_value::<Vec<LocalStorageItem>>(items_val) {
                        let state = StorageState {
                            cookies: Vec::new(),
                            origins: vec![OriginStorage {
                                origin: "https://www.airselangor.com".to_string(),
                                local_storage: ls_items,
                            }],
                        };
                        if let Ok(serialized) = serde_json::to_string_pretty(&state) {
                            let _ = tokio::fs::create_dir_all(&self.config.session_dir).await;
                            let _ = tokio::fs::write(&self.config.session_state_file, serialized).await;
                        }
                    }
                }
            }

            return Ok(AuthStatus {
                message: "Login successful.".to_string(),
                ..status
            });
        }

        Ok(AuthStatus {
            is_authenticated: false,
            user_identifier: String::new(),
            steel_session_id: session.session_id,
            session_viewer_url: session.session_viewer_url,
            message: "Login pending or requires verification.".to_string(),
        })
    }

    /// Logs out: clears cookies, localStorage, session storage, deletes local state file, and releases Steel session.
    pub async fn logout(&self) -> Result<LogoutResponse> {
        info!("Logging out: clearing browser storage and session state...");

        // 1. Clear browser storage in active Steel session
        if self.steel.has_active_session().await {
            self.steel.clear_browser_storage().await;
            self.steel.close_session().await;
        }

        // 2. Remove persisted session state file from disk
        if self.config.session_state_file.exists() {
            let _ = tokio::fs::remove_file(&self.config.session_state_file).await;
        }

        Ok(LogoutResponse {
            success: true,
            message: "Logged out successfully. All credentials, cookies, and Steel browser sessions have been cleared.".to_string(),
        })
    }

    /// Ensures the session is authenticated; logs in automatically if not.
    pub async fn ensure_authenticated(&self) -> Result<AuthStatus> {
        // Guard: Do not trigger Steel if credentials are not configured and no session file exists
        if !self.config.is_ready_for_auth() {
            anyhow::bail!("Air Selangor credentials not configured. Please set AIR_SELANGOR_ID and AIR_SELANGOR_PASSWORD in .env");
        }

        let status = self.check_auth().await?;
        if status.is_authenticated {
            return Ok(status);
        }
        let login_res = self.login(None, None).await?;
        if !login_res.is_authenticated {
            anyhow::bail!("{}. Inspect live session: {}", login_res.message, login_res.session_viewer_url);
        }
        Ok(login_res)
    }
}
