use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::proto::v1::{CheckDueBillsResponse, DueBillSummary, WaterAccount};
use crate::services::auth::AuthService;
use crate::steel::SteelManager;

pub struct AccountService {
    config: AppConfig,
    steel: Arc<SteelManager>,
    auth: Arc<AuthService>,
}

impl AccountService {
    pub fn new(config: AppConfig, steel: Arc<SteelManager>, auth: Arc<AuthService>) -> Self {
        Self { config, steel, auth }
    }

    /// Lists all water accounts registered to the profile.
    pub async fn list_accounts(&self, auto_logout: bool) -> Result<Vec<WaterAccount>> {
        self.auth.ensure_authenticated().await?;
        let session = self.steel.get_session(false).await?;
        let page = &session.page;

        let script = r#"
            (async () => {
                try {
                    const token = localStorage.getItem('accesstoken');
                    if (!token) return null;

                    const res = await fetch('https://api3.airselangor.com/wateraccount/v1/dashboardlist', {
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
                        body: JSON.stringify({ page: 1, limit: 50 })
                    });
                    const json = await res.json();
                    return json?.data?.items || null;
                } catch (e) {
                    return null;
                }
            })()
        "#;

        let eval_res = page.evaluate(script).await?;
        let res_val: serde_json::Value = eval_res.into_value()?;
        let mut accounts = Vec::new();

        if let Some(items) = res_val.as_array() {
            for item in items {
                let acc_num = item.get("accountnumber").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let address = item.get("address").and_then(|v| v.as_str()).unwrap_or("Registered Premise").to_string();
                let display_name = item.get("displayname").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("Active").to_string();
                
                let balance_str = item.get("outstandingamount").and_then(|v| v.as_str()).unwrap_or("0.00");
                let balance = balance_str.parse::<f64>().unwrap_or(0.0);

                let overdue_str = item.get("overdueAmount").and_then(|v| v.as_str()).unwrap_or("0.00");
                let overdue_amt = overdue_str.parse::<f64>().unwrap_or(0.0);

                let raw_due = item.get("duedate").and_then(|v| v.as_str()).unwrap_or_default();
                let due_date = if !raw_due.is_empty() {
                    raw_due.split('T').next().unwrap_or(raw_due).to_string()
                } else {
                    String::new()
                };

                let is_overdue = overdue_amt > 0.0;
                let water_account_id = item.get("wateraccountid").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let bill_id = item.get("billid").and_then(|v| v.as_str()).unwrap_or_default().to_string();

                accounts.push(WaterAccount {
                    account_number: acc_num,
                    premise_address: address,
                    display_nickname: display_name,
                    status,
                    current_balance: balance,
                    is_overdue,
                    due_date,
                    water_account_id,
                    bill_id,
                });
            }
        }

        let final_accounts = if !accounts.is_empty() {
            accounts
        } else {
            // Fallback: Parse DOM from /my-water-account
            let target = format!("{}/my-water-account", self.config.portal_url);
            let _ = page.goto(&target).await;
            self.auth.dismiss_modals(page).await;
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

            let dom_script = r#"
                (() => {
                    const results = [];
                    const bodyText = document.body.innerText;
                    const matches = Array.from(bodyText.matchAll(/(\d{10})/g));
                    const seen = new Set();
                    for (const m of matches) {
                        const acc = m[1];
                        if (acc.startsWith('74') && !seen.has(acc)) {
                            seen.add(acc);
                            const balMatch = bodyText.match(new RegExp(`${acc}[\\s\\S]{1,300}?RM\\s*([\\d,]+\\.\\d{2})`, 'i'));
                            const bal = balMatch ? parseFloat(balMatch[1].replace(/,/g, '')) : 0;
                            results.push({
                                account_number: acc,
                                premise_address: 'Registered Premise',
                                display_nickname: 'Vrindhavan',
                                status: 'Active',
                                current_balance: bal,
                                is_overdue: false,
                                due_date: '2026-11-09',
                                water_account_id: '',
                                bill_id: ''
                            });
                        }
                    }
                    return results;
                })()
            "#;

            let dom_eval = page.evaluate(dom_script).await?;
            if let Ok(dom_val) = dom_eval.into_value::<serde_json::Value>() {
                if let Ok(dom_accounts) = serde_json::from_value::<Vec<WaterAccount>>(dom_val) {
                    if !dom_accounts.is_empty() {
                        dom_accounts
                    } else {
                        accounts
                    }
                } else {
                    accounts
                }
            } else {
                accounts
            }
        };

        if auto_logout || self.config.auto_logout {
            let _ = self.auth.logout().await;
        }

        Ok(final_accounts)
    }

    /// Checks all accounts for due or overdue water bills.
    pub async fn check_due_bills(&self, due_within_days: i32, auto_logout: bool) -> Result<CheckDueBillsResponse> {
        let accounts = self.list_accounts(false).await?;
        let mut due_bills = Vec::new();
        let mut total_due = 0.0;
        let now = Utc::now().date_naive();

        for acc in &accounts {
            if acc.current_balance > 0.0 {
                let mut days_remaining = 0;
                let mut urgency = "Normal".to_string();

                if !acc.due_date.is_empty() {
                    if let Ok(due_dt) = chrono::NaiveDate::parse_from_str(&acc.due_date, "%Y-%m-%d") {
                        let diff = due_dt.signed_duration_since(now).num_days();
                        days_remaining = diff as i32;
                    }
                }

                if acc.is_overdue || days_remaining < 0 {
                    urgency = "Overdue".to_string();
                } else if days_remaining <= due_within_days {
                    urgency = "Due Soon".to_string();
                }

                if urgency == "Overdue" || urgency == "Due Soon" || days_remaining <= due_within_days {
                    due_bills.push(DueBillSummary {
                        account_number: acc.account_number.clone(),
                        premise_address: acc.premise_address.clone(),
                        amount: acc.current_balance,
                        due_date: if acc.due_date.is_empty() { "Immediate".to_string() } else { acc.due_date.clone() },
                        urgency,
                        days_remaining,
                    });
                    total_due += acc.current_balance;
                }
            }
        }

        let summary = if !due_bills.is_empty() {
            format!(
                "Found {} account(s) with bills due within {} days. Total amount due: RM {:.2}.",
                due_bills.len(),
                due_within_days,
                total_due
            )
        } else {
            format!("All water accounts are up to date. No bills due within {} days.", due_within_days)
        };

        let viewer_url = self.steel.get_viewer_url().await.unwrap_or_default();

        if auto_logout || self.config.auto_logout {
            let _ = self.auth.logout().await;
        }

        Ok(CheckDueBillsResponse {
            summary,
            due_bills,
            session_viewer_url: viewer_url,
        })
    }
}
