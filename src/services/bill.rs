use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::config::AppConfig;
use crate::proto::v1::{BillDetails, DownloadResult};
use crate::services::account::AccountService;
use crate::services::auth::AuthService;
use crate::steel::SteelManager;

pub struct BillService {
    config: AppConfig,
    steel: Arc<SteelManager>,
    account_service: Arc<AccountService>,
    auth: Arc<AuthService>,
    http_client: Client,
}

impl BillService {
    pub fn new(
        config: AppConfig,
        steel: Arc<SteelManager>,
        account_service: Arc<AccountService>,
        auth: Arc<AuthService>,
    ) -> Self {
        Self {
            config,
            steel,
            account_service,
            auth,
            http_client: Client::new(),
        }
    }

    /// Fetches detailed billing statement information for an account.
    pub async fn get_bill_details(&self, account_number: &str, auto_logout: bool) -> Result<BillDetails> {
        let accounts = self.account_service.list_accounts(false).await?;
        let account = accounts
            .iter()
            .find(|a| a.account_number == account_number)
            .or_else(|| accounts.first())
            .cloned()
            .context(format!("Account {} not found", account_number))?;

        let session = self.steel.get_session(false).await?;
        let page = &session.page;

        let wa_id = account.water_account_id.clone();
        let script = format!(
            r#"
            (async () => {{
                try {{
                    const token = localStorage.getItem('accesstoken');
                    if (!token) return null;

                    const headers = {{
                        'language': 'en',
                        'transactionid': crypto.randomUUID(),
                        'authorization': 'Bearer ' + token,
                        'timestamp': new Date().toISOString(),
                        'os-type': 'web',
                        'device-name': 'browser',
                        'content-type': 'application/json'
                    }};

                    const resUsage = await fetch('https://api3.airselangor.com/lifestyle/v1/lcd/latestbillwaterusage', {{
                        method: 'POST',
                        headers,
                        body: JSON.stringify({{ wateraccountid: '{}', channel: 'web' }})
                    }});
                    const usageData = await resUsage.json();

                    const resHist = await fetch('https://api3.airselangor.com/billing/v1/billpayment/history', {{
                        method: 'POST',
                        headers,
                        body: JSON.stringify({{ category: 0, wateraccountguid: '{}', limit: 5 }})
                    }});
                    const histData = await resHist.json();

                    return {{
                        usage: usageData?.data || null,
                        history: histData?.data?.billpaymentlist || []
                    }};
                }} catch (e) {{
                    return null;
                }}
            }})()
            "#,
            wa_id, wa_id
        );

        let eval_res = page.evaluate(script).await?;
        let details_val: serde_json::Value = eval_res.into_value()?;
        let mut consumption_m3 = 0.0;
        let mut bill_date = String::new();
        let mut bill_number = String::new();
        let mut bill_id = account.bill_id.clone();

        if let Some(usage) = details_val.get("usage") {
            consumption_m3 = usage.get("waterusage").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if let Some(bd) = usage.get("billdate").and_then(|v| v.as_str()) {
                bill_date = bd.split('T').next().unwrap_or(bd).to_string();
            }
        }

        if let Some(history) = details_val.get("history").and_then(|v| v.as_array()) {
            if let Some(latest) = history.iter().find(|b| {
                b.get("category").and_then(|v| v.as_i64()) == Some(2)
                    || b.get("isbillready").and_then(|v| v.as_bool()) == Some(true)
            }) {
                if let Some(bn) = latest.get("billnumber").and_then(|v| v.as_str()) {
                    bill_number = bn.to_string();
                }
                if let Some(bid) = latest.get("billid").and_then(|v| v.as_str()) {
                    bill_id = bid.to_string();
                }
                if bill_date.is_empty() {
                    if let Some(dt) = latest.get("datetime").and_then(|v| v.as_str()) {
                        bill_date = dt.split('T').next().unwrap_or(dt).to_string();
                    }
                }
            }
        }

        let current_balance = account.current_balance;
        let due_date = if !account.due_date.is_empty() {
            account.due_date.clone()
        } else {
            "See statement".to_string()
        };

        let now = Utc::now().date_naive();
        let mut days_remaining = 0;
        let mut status = "Normal".to_string();

        if !account.due_date.is_empty() {
            if let Ok(due_dt) = chrono::NaiveDate::parse_from_str(&account.due_date, "%Y-%m-%d") {
                days_remaining = due_dt.signed_duration_since(now).num_days() as i32;
                if days_remaining < 0 || account.is_overdue {
                    status = "Overdue".to_string();
                } else if days_remaining <= 7 {
                    status = "Due Soon".to_string();
                } else {
                    status = "Normal".to_string();
                }
            }
        }

        let viewer_url = self.steel.get_viewer_url().await.unwrap_or_default();

        let details = BillDetails {
            account_number: account.account_number,
            registered_name: account.display_nickname,
            premise_address: account.premise_address,
            bill_date: if !bill_date.is_empty() { bill_date } else { Utc::now().format("%Y-%m-%d").to_string() },
            due_date,
            previous_balance: 0.0,
            current_charges: current_balance,
            total_outstanding: current_balance,
            minimum_payable: if current_balance < 6.0 { current_balance } else { 6.0 },
            status,
            days_remaining,
            consumption_m3,
            bill_number,
            bill_id,
            session_viewer_url: viewer_url,
        };

        if auto_logout || self.config.auto_logout {
            let _ = self.auth.logout().await;
        }

        Ok(details)
    }

    /// Downloads the latest official e-Bill PDF statement.
    pub async fn download_bill_pdf(
        &self,
        account_number: &str,
        output_directory: Option<&str>,
        auto_logout: bool,
    ) -> Result<DownloadResult> {
        let accounts = self.account_service.list_accounts(false).await?;
        let account = accounts
            .iter()
            .find(|a| a.account_number == account_number)
            .or_else(|| accounts.first())
            .cloned()
            .context(format!("Account {} not found", account_number))?;

        let dest_dir = output_directory
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.downloads_dir.clone());

        tokio::fs::create_dir_all(&dest_dir).await
            .context("Failed to create output directory")?;

        let session = self.steel.get_session(false).await?;
        let page = &session.page;

        let wa_id = account.water_account_id.clone();
        let script = format!(
            r#"
            (async () => {{
                try {{
                    const token = localStorage.getItem('accesstoken');
                    if (!token) return null;

                    const headers = {{
                        'language': 'en',
                        'transactionid': crypto.randomUUID(),
                        'authorization': 'Bearer ' + token,
                        'timestamp': new Date().toISOString(),
                        'os-type': 'web',
                        'device-name': 'browser',
                        'content-type': 'application/json'
                    }};

                    const resHist = await fetch('https://api3.airselangor.com/billing/v1/billpayment/history', {{
                        method: 'POST',
                        headers,
                        body: JSON.stringify({{ category: 0, wateraccountguid: '{}', limit: 10 }})
                    }});
                    const histData = await resHist.json();
                    const bills = histData?.data?.billpaymentlist || [];
                    const latest = bills.find(b => b.category === 2 || b.isbillready) || bills[0];
                    if (!latest) return null;

                    const dlRes = await fetch('https://api3.airselangor.com/billing/v1/download', {{
                        method: 'POST',
                        headers,
                        body: JSON.stringify({{
                            billid: latest.billid,
                            billnumber: latest.billnumber,
                            amount: latest.amount,
                            isbillready: true,
                            status: latest.status || 'Bill issued',
                            datetime: latest.datetime,
                            category: latest.category || 2
                        }})
                    }});
                    const dlData = await dlRes.json();
                    return {{
                        pdf_url: dlData?.data?.documenturlpath || null,
                        bill_number: latest.billnumber || null,
                        bill_date: latest.datetime ? latest.datetime.split('T')[0] : null
                    }};
                }} catch (e) {{
                    return null;
                }}
            }})()
            "#,
            wa_id
        );

        let dl_eval = page.evaluate(script).await?;
        let dl_val: serde_json::Value = dl_eval.into_value()?;
        let pdf_url = dl_val.get("pdf_url").and_then(|v| v.as_str());

        let result = if let Some(url) = pdf_url {
            info!("Downloading PDF from Azure Blob Storage SAS URL...");
            let resp = self.http_client.get(url).send().await
                .context("Failed to download PDF from storage")?;

            if resp.status().is_success() {
                let bytes = resp.bytes().await.context("Failed to read PDF response bytes")?;
                let bill_no = dl_val.get("bill_number").and_then(|v| v.as_str()).unwrap_or("statement");
                let file_name = format!("AirSelangor_Bill_{}_{}.pdf", account.account_number, bill_no);
                let file_path = dest_dir.join(&file_name);

                tokio::fs::write(&file_path, &bytes).await
                    .context("Failed to write PDF file to disk")?;

                let file_size_bytes = bytes.len() as i64;
                let path_str = file_path.to_string_lossy().to_string();

                DownloadResult {
                    success: true,
                    account_number: account.account_number,
                    file_path: path_str,
                    file_size_bytes,
                    error: String::new(),
                }
            } else {
                DownloadResult {
                    success: false,
                    account_number: account.account_number,
                    file_path: String::new(),
                    file_size_bytes: 0,
                    error: "Failed to download PDF stream from storage URL.".to_string(),
                }
            }
        } else {
            DownloadResult {
                success: false,
                account_number: account.account_number,
                file_path: String::new(),
                file_size_bytes: 0,
                error: "Unable to retrieve PDF statement download link.".to_string(),
            }
        };

        if auto_logout || self.config.auto_logout {
            let _ = self.auth.logout().await;
        }

        Ok(result)
    }
}
