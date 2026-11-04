use anyhow::{Context, Result};
use chromiumoxide::browser::Browser;
use chromiumoxide::Page;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio::sync::Mutex;
use tracing::info;

use crate::config::AppConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct CookieItem {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub expires: f64,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub same_site: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalStorageItem {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginStorage {
    pub origin: String,
    #[serde(rename = "localStorage")]
    pub local_storage: Vec<LocalStorageItem>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StorageState {
    pub cookies: Vec<serde_json::Value>,
    #[serde(default)]
    pub origins: Vec<OriginStorage>,
}

#[derive(Clone)]
pub struct SteelSession {
    pub session_id: String,
    pub websocket_url: String,
    pub session_viewer_url: String,
    pub browser: Arc<Mutex<Browser>>,
    pub page: Arc<Page>,
}

pub struct SteelManager {
    config: AppConfig,
    http_client: Client,
    active_session: Arc<Mutex<Option<SteelSession>>>,
}

#[derive(Serialize)]
struct CreateSessionRequest {
    timeout: u64,
    dimensions: Dimensions,
}

#[derive(Serialize)]
struct Dimensions {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct CreateSessionResponse {
    id: String,
    #[serde(rename = "websocketUrl")]
    websocket_url: Option<String>,
    #[serde(rename = "sessionViewerUrl")]
    session_viewer_url: Option<String>,
}

impl SteelManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            active_session: Arc::new(Mutex::new(None)),
        }
    }

    /// Resolves hostname to IPv4 to circumvent Chromium DevTools Host-header check.
    async fn resolve_cdp_url(&self, raw_url: &str, session_id: &str) -> Result<String> {
        let trimmed = raw_url.trim_start_matches("ws://").trim_start_matches("http://").trim_start_matches("https://");
        let (host_port, _) = trimmed.split_once('/').unwrap_or((trimmed, ""));
        let (host, port) = if let Some((h, p)) = host_port.split_once(':') {
            (h, p)
        } else {
            (host_port, "3000")
        };

        // If host is already an IP or localhost, format directly
        if host.parse::<std::net::Ipv4Addr>().is_ok() || host == "localhost" {
            return Ok(format!("ws://{}:{}/?sessionId={}", host, port, session_id));
        }

        // Resolve DNS to IPv4
        let host_port_str = format!("{}:{}", host, port);
        if let Ok(mut addrs) = lookup_host(&host_port_str).await {
            if let Some(SocketAddr::V4(ipv4)) = addrs.next() {
                return Ok(format!("ws://{}:{}/?sessionId={}", ipv4.ip(), port, session_id));
            }
        }

        // Fallback to original
        Ok(format!("ws://{}:{}/?sessionId={}", host, port, session_id))
    }

    /// Gets or creates a connected Steel browser session.
    pub async fn get_session(&self, force_new: bool) -> Result<SteelSession> {
        let mut guard = self.active_session.lock().await;

        if !force_new {
            if let Some(ref session) = *guard {
                return Ok(session.clone());
            }
        }

        info!("Creating new Steel browser session...");
        let create_url = format!("{}/v1/sessions", self.config.steel_base_url.trim_end_matches('/'));
        let mut req = self.http_client.post(&create_url).json(&CreateSessionRequest {
            timeout: self.config.steel_session_timeout_ms,
            dimensions: Dimensions { width: 1920, height: 1080 },
        });

        if let Some(ref key) = self.config.steel_api_key {
            req = req.header("steel-api-key", key);
        }

        let resp = req.send().await.context("Failed to call Steel create session API")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Steel session creation returned {}: {}", status, body);
        }

        let session_data: CreateSessionResponse = resp.json().await.context("Failed to parse Steel session response")?;
        let raw_ws = session_data.websocket_url.unwrap_or_else(|| self.config.steel_base_url.clone());
        let cdp_url = self.resolve_cdp_url(&raw_ws, &session_data.id).await?;

        info!("Connecting to CDP at {}", cdp_url);
        let (browser, mut handler) = Browser::connect(&cdp_url)
            .await
            .context("Failed to connect chromiumoxide over CDP")?;

        // Drive the CDP message loop in the background
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if event.is_err() {
                    break;
                }
            }
        });

        let page = browser
            .new_page("about:blank")
            .await
            .context("Failed to open initial browser page")?;
        let browser_arc = Arc::new(Mutex::new(browser));
        let page_arc = Arc::new(page);

        // Fix session viewer URL if missing
        let session_viewer_url = session_data.session_viewer_url.unwrap_or_else(|| {
            format!("{}/v1/sessions/{}/viewer", self.config.steel_base_url.trim_end_matches('/'), session_data.id)
        });

        let steel_session = SteelSession {
            session_id: session_data.id,
            websocket_url: cdp_url,
            session_viewer_url,
            browser: browser_arc,
            page: page_arc,
        };

        // Rehydrate session storage if state file exists
        if self.config.session_state_file.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&self.config.session_state_file).await {
                if let Ok(state) = serde_json::from_str::<StorageState>(&content) {
                    self.rehydrate_session(&steel_session.page, &state).await;
                }
            }
        }

        *guard = Some(steel_session.clone());
        Ok(steel_session)
    }

    /// Injects saved cookies and localStorage tokens into the browser context.
    pub async fn rehydrate_session(&self, page: &Page, state: &StorageState) {
        for origin in &state.origins {
            if origin.local_storage.is_empty() {
                continue;
            }
            let js_items = serde_json::to_string(&origin.local_storage).unwrap_or_else(|_| "[]".to_string());
            let script = format!(
                r#"
                try {{
                    const items = {};
                    for (const item of items) {{
                        window.localStorage.setItem(item.name, item.value);
                    }}
                }} catch (e) {{}}
                "#,
                js_items
            );
            // Execute on page
            let _ = page.evaluate(script).await;
        }
    }

    pub async fn get_viewer_url(&self) -> Option<String> {
        let guard = self.active_session.lock().await;
        guard.as_ref().map(|s| s.session_viewer_url.clone())
    }

    pub async fn has_active_session(&self) -> bool {
        let guard = self.active_session.lock().await;
        guard.is_some()
    }

    /// Clears localStorage, sessionStorage, and document cookies in the active browser context.
    pub async fn clear_browser_storage(&self) {
        let guard = self.active_session.lock().await;
        if let Some(ref session) = *guard {
            let script = r#"
                (() => {
                    try {
                        window.localStorage.clear();
                        window.sessionStorage.clear();
                        const cookies = document.cookie.split(";");
                        for (let i = 0; i < cookies.length; i++) {
                            const cookie = cookies[i];
                            const eqPos = cookie.indexOf("=");
                            const name = eqPos > -1 ? cookie.substr(0, eqPos) : cookie;
                            document.cookie = name.trim() + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=" + window.location.hostname;
                            document.cookie = name.trim() + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/";
                        }
                    } catch (e) {}
                })()
            "#;
            let _ = session.page.evaluate(script).await;
        }
    }

    pub async fn close_session(&self) {
        let mut guard = self.active_session.lock().await;
        if let Some(session) = guard.take() {
            info!("Releasing Steel session {}", session.session_id);
            let release_url = format!(
                "{}/v1/sessions/{}/release",
                self.config.steel_base_url.trim_end_matches('/'),
                session.session_id
            );
            let mut req = self.http_client.post(&release_url);
            if let Some(ref key) = self.config.steel_api_key {
                req = req.header("steel-api-key", key);
            }
            let _ = req.send().await;
            let mut b = session.browser.lock().await;
            let _ = b.close().await;
        }
    }
}
