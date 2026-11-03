# Product Requirements Document (PRD): Air Selangor MCP Server

## 1. Executive Summary
The **Air Selangor MCP Server** is a specialized Model Context Protocol (MCP) integration that connects Large Language Model (LLM) assistants to the official [Air Selangor Portal](https://www.airselangor.com). It enables automated tracking, inspection, statement retrieval, and assisted payment of water utility accounts.

By leveraging a self-hosted **Steel (`steel.dev`)** browser automation sandbox over a private **Tailscale** mesh network (`steel.kingfisher-noodlefish.ts.net`), the system offloads browser execution from the host, mitigates anti-bot/Cloudflare challenges, and provides an interactive live session viewer for secure user-facing authorizations (CAPTCHA, 2FA, and bank payments).

---

## 2. Problem Statement & User Value

### 2.1 The Problem
- **No Public API**: Air Selangor does not offer a public REST/GraphQL API for retail consumers to check balances or retrieve bills programmatically.
- **Dynamic Frontend**: The Air Selangor portal is a client-rendered Single-Page Application (Next.js) requiring session handling, authentication (NRIC/Passport/SSM + Password), and dynamic DOM interactions.
- **Manual Overhead**: Consumers frequently forget payment deadlines or experience friction logging into web portals just to check statement amounts and due dates.
- **Security in Financial Automation**: Delegating payments entirely to an autonomous agent without a human-in-the-loop creates severe financial and security risks.

### 2.2 The Solution
An AI-accessible MCP server that operates in two distinct phases:
1. **Phase 1 (Monitoring & Bill Inquiries)**: Purely autonomous and read-only. Gathers account balances, inspects due dates, triggers alerts, and downloads official PDF statements.
2. **Phase 2 (Assisted Payments)**: Staged by the agent and authorized by the human user via a private Tailscale interactive browser session, ensuring zero exposure of bank credentials or 2FA/TAC codes to the AI.

---

## 3. Product Phasing & Scope

```
+-------------------------------------------------------------+
| Phase 1: Information Gathering & Monitoring (CURRENT)       |
|                                                             |
|  - Authenticate to Air Selangor via Steel.dev sandbox       |
|  - Check and maintain session validity                      |
|  - Enumerate registered water accounts and addresses        |
|  - Fetch itemized bill details, due dates, and consumption  |
|  - Scan for overdue or approaching bills (threshold alerts) |
|  - Download official monthly e-Bill PDF statements          |
|  - Provide Tailscale Live Session Viewer for CAPTCHA/debug  |
+-------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------+
| Phase 2: Assisted Bill Payment (PLANNED)                    |
|                                                             |
|  - Prepare & stage payment checkout for water accounts      |
|  - Validate minimum payable amounts (>= RM 6.00)            |
|  - Select payment channels (FPX, DuitNow QR, Card, eWallet) |
|  - Hand over session via Steel Live Viewer for Bank 2FA/TAC |
|  - Verify updated balance and capture official receipt ID   |
+-------------------------------------------------------------+
```

---

## 4. Key Personas & Use Cases

### 4.1 Target Persona
- **Homeowners / Tenants / Property Managers** residing in Selangor, Kuala Lumpur, or Putrajaya who manage one or multiple water accounts and want autonomous monitoring without missing payment deadlines.

### 4.2 Primary Use Cases
1. **Proactive Due-Date Checking**: *"Antigravity, do I have any Air Selangor water bills due this week?"* -> The agent runs `check_due_bills`, reports outstanding balances and due dates.
2. **Detailed Consumption Inquiry**: *"How much water did I consume last month on account 1234567890?"* -> The agent runs `get_bill_details`, extracting charges and consumption volume in $m^3$.
3. **e-Bill Archive**: *"Download my latest water bill PDF to my documents folder."* -> The agent executes `download_bill_pdf` and saves the file locally.
4. **Assisted Settlement (Phase 2)**: *"Pay my overdue water bill via FPX."* -> The agent stages checkout and provides the secure Tailscale session URL for bank authentication.

---

## 5. Functional Requirements

### 5.1 Authentication & Session Management
- **FR-01 (Session Checking)**: Server shall verify whether an active browser session on Steel is authenticated.
- **FR-02 (Portal Login)**: Server shall authenticate to Air Selangor using NRIC / Passport / SSM and password stored in local environment variables.
- **FR-03 (Interactive Challenge Handover)**: If Cloudflare Turnstile, reCAPTCHA, or SMS OTP is encountered during login, the server shall generate a Steel Live Viewer URL (`sessionViewerUrl`) allowing the user to complete verification in their browser.

### 5.2 Account & Bill Information
- **FR-04 (Account Listing)**: Server shall extract all registered water accounts, premise addresses, and current outstanding balances.
- **FR-05 (Bill Details Extraction)**: Server shall parse bill date, payment due date, previous balance, current charges, minimum payable amount, and water consumption in $m^3$.
- **FR-06 (Due-Date Alerting)**: Server shall categorize accounts into `Overdue`, `Due Soon`, or `Normal` based on a configurable threshold (default: 7 days).
- **FR-07 (Statement Download)**: Server shall download official e-Bill PDF statements and save them to a specified directory (default: `./downloads`).

### 5.3 Assisted Payments (Phase 2)
- **FR-08 (Payment Staging)**: Server shall navigate to `/my-water-account/payment`, select the target account, validate minimum payment rules, and select the designated payment gateway.
- **FR-09 (Human-in-the-Loop Gateway Handover)**: Server shall pause automated actions upon reaching the payment gateway and hand over control to the user via the Tailscale Live Viewer.
- **FR-10 (Receipt Verification)**: Server shall verify updated account balances post-payment and record transaction receipt numbers.

---

## 6. Non-Functional Requirements & Security

| Category | Requirement |
| :--- | :--- |
| **Network Security** | Browser sandbox runs on the user's private, encrypted **Tailscale** overlay network (`steel.kingfisher-noodlefish.ts.net`). |
| **Credential Safety** | Login credentials reside in local `.env`; never exposed in logs or MCP tool responses. |
| **Financial Isolation** | The AI assistant has **zero access** to bank account passwords, card CVV, or TAC/OTP codes. Authorization occurs solely by the human user via the Steel viewer. |
| **Resilience & Stealth** | Steel's built-in stealth profile and fingerprinting protect against bot false-positives and cloud security blocks. |
| **Protocol Conformance** | Complies with the Model Context Protocol (JSON-RPC 2.0 over standard I/O). |

---

## 7. Success Metrics
- **Zero Missed Deadlines**: AI proactively detects overdue or upcoming bills within configured thresholds.
- **Zero Credential Leaks**: Absolute isolation of banking and utility credentials from model prompt context.
- **Frictionless Session Access**: Steel Live Viewer provides single-click access for human intervention on any device in the Tailscale mesh.
