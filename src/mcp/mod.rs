use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::error;

use crate::config::AppConfig;
use crate::proto::PROTO_SCHEMA;
use crate::services::account::AccountService;
use crate::services::auth::AuthService;
use crate::services::bill::BillService;
use crate::steel::SteelManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub struct McpServer {
    config: AppConfig,
    steel: Arc<SteelManager>,
    auth: Arc<AuthService>,
    account: Arc<AccountService>,
    bill: Arc<BillService>,
}

impl McpServer {
    pub fn new(config: AppConfig) -> Self {
        let steel = Arc::new(SteelManager::new(config.clone()));
        let auth = Arc::new(AuthService::new(config.clone(), steel.clone()));
        let account = Arc::new(AccountService::new(config.clone(), steel.clone(), auth.clone()));
        let bill = Arc::new(BillService::new(config.clone(), steel.clone(), account.clone(), auth.clone()));

        Self {
            config,
            steel,
            auth,
            account,
            bill,
        }
    }

    pub fn list_tools(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "check_session",
                    "description": "Checks whether the current Steel browser session is authenticated on Air Selangor.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "login",
                    "description": "Logs into the Air Selangor portal. If visual verification (CAPTCHA) or 2FA is required, returns the Steel Live Viewer URL.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Optional Air Selangor login ID (NRIC, Passport, or SSM). Defaults to AIR_SELANGOR_ID from environment."
                            },
                            "password": {
                                "type": "string",
                                "description": "Optional Air Selangor login password. Defaults to AIR_SELANGOR_PASSWORD from environment."
                            }
                        }
                    }
                },
                {
                    "name": "logout",
                    "description": "Logs out from the Air Selangor portal and completely clears all stored credentials, cookies, session files, and active Steel browser instances.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "list_accounts",
                    "description": "Retrieves all water accounts registered to the profile along with their current outstanding balances.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "autoLogout": {
                                "type": "boolean",
                                "description": "Optional: If true, logs out and clears all session state and browser instances immediately after fetching data."
                            }
                        }
                    }
                },
                {
                    "name": "get_bill_details",
                    "description": "Fetches detailed billing information for a specific Air Selangor water account.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "accountNumber": {
                                "type": "string",
                                "description": "The 10-digit Air Selangor water account number (e.g. 7400235711)."
                            },
                            "autoLogout": {
                                "type": "boolean",
                                "description": "Optional: If true, logs out and clears all session state and browser instances immediately after fetching data."
                            }
                        },
                        "required": ["accountNumber"]
                    }
                },
                {
                    "name": "check_due_bills",
                    "description": "Scans all water accounts and highlights bills that are overdue or due within a specified number of days.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "dueWithinDays": {
                                "type": "number",
                                "description": "Filter threshold in days (default: 7)."
                            },
                            "autoLogout": {
                                "type": "boolean",
                                "description": "Optional: If true, logs out and clears all session state and browser instances immediately after fetching data."
                            }
                        }
                    }
                },
                {
                    "name": "download_bill_pdf",
                    "description": "Downloads the official e-Bill PDF statement for a water account and saves it to the local filesystem.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "accountNumber": {
                                "type": "string",
                                "description": "The 10-digit Air Selangor water account number."
                            },
                            "outputDirectory": {
                                "type": "string",
                                "description": "Optional directory path where the PDF will be saved (defaults to ./downloads)."
                            },
                            "autoLogout": {
                                "type": "boolean",
                                "description": "Optional: If true, logs out and clears all session state and browser instances immediately after fetching data."
                            }
                        },
                        "required": ["accountNumber"]
                    }
                },
                {
                    "name": "get_session_viewer",
                    "description": "Returns the Steel Live Interactive Session Viewer URL so you can inspect or interact with the remote browser directly on your Tailscale network.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "get_proto_schema",
                    "description": "Returns the Protobuf 3 contract definitions describing all requests, responses, and data models for this MCP server.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        })
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        // Guard: Prevent triggering browser actions if credentials are not configured
        match name {
            "list_accounts" | "get_bill_details" | "check_due_bills" | "download_bill_pdf" => {
                if !self.config.is_ready_for_auth() {
                    anyhow::bail!(
                        "Cannot execute {}: Air Selangor credentials are not configured. Please set AIR_SELANGOR_ID and AIR_SELANGOR_PASSWORD in .env",
                        name
                    );
                }
            }
            "login" => {
                let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let pass = args.get("password").and_then(|v| v.as_str()).map(|s| s.to_string());
                let has_id = id.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                let has_pass = pass.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                if (!has_id || !has_pass) && !self.config.is_ready_for_auth() {
                    anyhow::bail!(
                        "Cannot execute login: Air Selangor credentials are not configured. Please provide id and password arguments or set AIR_SELANGOR_ID and AIR_SELANGOR_PASSWORD in .env"
                    );
                }
            }
            _ => {}
        }

        match name {
            "check_session" => {
                let status = self.auth.check_auth().await?;
                Ok(serde_json::to_value(&status)?)
            }
            "login" => {
                let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let password = args.get("password").and_then(|v| v.as_str()).map(|s| s.to_string());
                let status = self.auth.login(id, password).await?;
                Ok(serde_json::to_value(&status)?)
            }
            "logout" => {
                let resp = self.auth.logout().await?;
                Ok(serde_json::to_value(&resp)?)
            }
            "list_accounts" => {
                let auto_logout = args.get("autoLogout").and_then(|v| v.as_bool()).unwrap_or(false);
                let accounts = self.account.list_accounts(auto_logout).await?;
                let viewer_url = self.steel.get_viewer_url().await.unwrap_or_default();
                Ok(json!({
                    "count": accounts.len(),
                    "accounts": accounts,
                    "session_viewer_url": viewer_url
                }))
            }
            "get_bill_details" => {
                let acc = args.get("accountNumber").and_then(|v| v.as_str()).unwrap_or_default();
                let auto_logout = args.get("autoLogout").and_then(|v| v.as_bool()).unwrap_or(false);
                let details = self.bill.get_bill_details(acc, auto_logout).await?;
                Ok(serde_json::to_value(&details)?)
            }
            "check_due_bills" => {
                let days = args.get("dueWithinDays").and_then(|v| v.as_i64()).unwrap_or(7) as i32;
                let auto_logout = args.get("autoLogout").and_then(|v| v.as_bool()).unwrap_or(false);
                let summary = self.account.check_due_bills(days, auto_logout).await?;
                Ok(serde_json::to_value(&summary)?)
            }
            "download_bill_pdf" => {
                let acc = args.get("accountNumber").and_then(|v| v.as_str()).unwrap_or_default();
                let out_dir = args.get("outputDirectory").and_then(|v| v.as_str());
                let auto_logout = args.get("autoLogout").and_then(|v| v.as_bool()).unwrap_or(false);
                let result = self.bill.download_bill_pdf(acc, out_dir, auto_logout).await?;
                Ok(serde_json::to_value(&result)?)
            }
            "get_session_viewer" => {
                let session = self.steel.get_session(false).await?;
                Ok(json!({
                    "steel_session_id": session.session_id,
                    "session_viewer_url": session.session_viewer_url,
                    "message": "Open this URL in any browser on your Tailscale network to view or interact with the browser session."
                }))
            }
            "get_proto_schema" => {
                Ok(json!({
                    "proto_syntax": "proto3",
                    "package": "airselangor.v1",
                    "schema": PROTO_SCHEMA
                }))
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = match req.id {
            Some(id) => id,
            None => return None, // Notification, no response
        };

        match req.method.as_str() {
            "initialize" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "air-selangor-mcp",
                        "version": "1.0.0"
                    }
                })),
                error: None,
            }),
            "notifications/initialized" => None,
            "ping" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({})),
                error: None,
            }),
            "tools/list" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(self.list_tools()),
                error: None,
            }),
            "tools/call" => {
                let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));

                match self.call_tool(tool_name, args).await {
                    Ok(val) => {
                        let text = serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string());
                        Some(JsonRpcResponse {
                            jsonrpc: "2.0",
                            id,
                            result: Some(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text
                                    }
                                ]
                            })),
                            error: None,
                        })
                    }
                    Err(e) => Some(JsonRpcResponse {
                        jsonrpc: "2.0",
                        id,
                        result: Some(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Error executing {}: {}", tool_name, e)
                                }
                            ],
                            "isError": true
                        })),
                        error: None,
                    }),
                }
            }
            _ => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                }),
            }),
        }
    }

    pub async fn run_stdio_loop(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        let mut stdout = tokio::io::stdout();

        eprintln!("[AirSelangor MCP - Rust] Server running on stdio transport.");

        while let Some(line) = reader.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(req) => {
                    if let Some(resp) = self.handle_request(req).await {
                        let resp_json = serde_json::to_string(&resp)?;
                        stdout.write_all(resp_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    error!("Invalid JSON-RPC request received: {}", e);
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e)
                        }
                    });
                    stdout.write_all(err_resp.to_string().as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
        }

        self.steel.close_session().await;
        Ok(())
    }
}
