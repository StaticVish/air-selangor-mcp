# Air Selangor MCP Server

[![TypeScript](https://img.shields.io/badge/TypeScript-5.7-blue.svg)](https://www.typescriptlang.org/)
[![Model Context Protocol](https://img.shields.io/badge/MCP-1.6.1-green.svg)](https://modelcontextprotocol.io/)
[![Steel.dev](https://img.shields.io/badge/Browser_Engine-Steel.dev-orange.svg)](https://steel.dev/)
[![Tailscale](https://img.shields.io/badge/Network-Tailscale-indigo.svg)](https://tailscale.com/)

A Model Context Protocol (MCP) server for monitoring, querying, and managing [Air Selangor](https://www.airselangor.com) water utility accounts.

Instead of running heavy Chromium binaries locally, this server delegates all browser automation to a self-hosted **Steel (`steel.dev`)** sandbox running securely over your private **Tailscale** mesh (`steel.kingfisher-noodlefish.ts.net`).

---

## 📑 Table of Contents
- [Architecture & Phased Approach](#-architecture--phased-approach)
- [MCP Tools Reference](#-mcp-tools-reference)
- [Prerequisites](#-prerequisites)
- [Quick Start Installation](#-quick-start-installation)
- [Configuration (.env)](#-configuration-env)
- [Client Integration (Antigravity & Claude Desktop)](#-client-integration)
- [Security & Human-in-the-Loop Safeguards](#-security--human-in-the-loop-safeguards)
- [Development & Troubleshooting](#-development--troubleshooting)

---

## 🏗️ Architecture & Phased Approach

The project is structured into two deliberate phases to protect financial security:

```
+--------------------------------------------------------------+
|                     AI Assistant Client                      |
|           (Antigravity CLI / IDE / Claude Desktop)           |
+------------------------------+-------------------------------+
                               | MCP Protocol (stdio / JSON-RPC)
                               v
+--------------------------------------------------------------+
|                   Air Selangor MCP Server                    |
|                      (Local Node.js)                         |
|                                                              |
|   - Tools Engine (login, list_accounts, check_due_bills...)  |
|   - Playwright CDP Adapter with Auto-DNS/IP Resolver         |
|   - Session Manager & Live Viewer Dispatcher                 |
+------------------------------+-------------------------------+
                               | Tailscale Encrypted Mesh
                               v
+--------------------------------------------------------------+
|                Self-Hosted Steel.dev Sandbox                 |
|             (steel.kingfisher-noodlefish.ts.net)             |
|                                                              |
|   - Anti-Bot & Fingerprint Stealth Engine                    |
|   - Isolated Chromium Browser Instances                      |
|   - Interactive Live Session Viewer UI                       |
+------------------------------+-------------------------------+
                               | HTTPS / DOM
                               v
+--------------------------------------------------------------+
|                  Air Selangor Web Portal                     |
|                https://www.airselangor.com                   |
+--------------------------------------------------------------+
```

### Phase 1: Information Gathering & Monitoring (Current)
- Automated login using your NRIC, Passport, or SSM credentials.
- Multi-account discovery and outstanding balance tracking.
- Itemized billing statement analysis (bill date, due date, water consumption in $m^3$).
- Proactive due-date alerts for overdue or approaching bills.
- Automated monthly e-Bill PDF statement downloads.
- Real-time **Tailscale Live Session Viewer** link generation for CAPTCHA/debugging.

### Phase 2: Assisted Bill Payment (Planned)
- Safe, staged checkout orchestration across supported channels (FPX Online Banking, DuitNow QR, Credit/Debit Card, e-Wallets).
- **Human-In-The-Loop (HITL)**: Hands over session control via the Steel Live Viewer so you enter banking passwords and approve TAC/OTP codes yourself. The AI never touches financial credentials.
- Post-payment balance verification and official receipt recording.

---

## 🛠️ MCP Tools Reference

### 1. `check_session`
Verifies whether the active Steel browser session is currently authenticated on the portal.
- **Inputs**: None
- **Sample Output**:
  ```json
  {
    "isAuthenticated": true,
    "userIdentifier": "900101-10-XXXX",
    "steelSessionId": "e7edcf63-cfcc-4664-8e59-2a633664896a",
    "sessionViewerUrl": "https://steel.kingfisher-noodlefish.ts.net/v1/sessions/e7edcf.../viewer"
  }
  ```

### 2. `login`
Authenticates against the Air Selangor portal. If a CAPTCHA or 2FA challenge occurs, returns the Steel Live Viewer URL.
- **Inputs**:
  - `id` *(string, optional)*: Override NRIC / Passport / SSM login ID.
  - `password` *(string, optional)*: Override login password.
- **Sample Output**:
  ```json
  {
    "isAuthenticated": true,
    "message": "Login successful.",
    "steelSessionId": "00336dab-def7-42ba-8dd2-c5ed97af173f"
  }
  ```

### 3. `list_accounts`
Discovers all registered water accounts linked to your profile, their premise addresses, and balances.
- **Inputs**: None
- **Sample Output**:
  ```json
  {
    "count": 2,
    "accounts": [
      {
        "accountNumber": "1002345678",
        "premiseAddress": "NO 12, JALAN SS 2/1, 47300 PETALING JAYA",
        "status": "Active",
        "currentBalance": 34.50,
        "isOverdue": false
      }
    ],
    "sessionViewerUrl": "https://steel.kingfisher-noodlefish.ts.net/..."
  }
  ```

### 4. `get_bill_details`
Extracts itemized billing statement metrics and water consumption volume.
- **Inputs**:
  - `accountNumber` *(string, required)*: The 10-digit water account number.
- **Sample Output**:
  ```json
  {
    "accountNumber": "1002345678",
    "billDate": "2026-10-15",
    "dueDate": "2026-11-14",
    "totalOutstanding": 34.50,
    "minimumPayable": 6.00,
    "status": "Due Soon",
    "daysRemaining": 11,
    "consumptionM3": 21.4
  }
  ```

### 5. `check_due_bills`
Scans all accounts and filters for bills needing attention.
- **Inputs**:
  - `dueWithinDays` *(number, optional, default: 7)*: Days threshold.
- **Sample Output**:
  ```json
  {
    "summary": "Found 1 account(s) with bills due within 7 days. Total amount due: RM 34.50.",
    "dueBills": [
      {
        "accountNumber": "1002345678",
        "amount": 34.50,
        "dueDate": "2026-11-14",
        "urgency": "Due Soon",
        "daysRemaining": 4
      }
    ]
  }
  ```

### 6. `download_bill_pdf`
Downloads the official monthly e-Bill PDF statement and saves it to the local filesystem.
- **Inputs**:
  - `accountNumber` *(string, required)*: 10-digit account number.
  - `outputDirectory` *(string, optional)*: Destination directory (defaults to `./downloads`).
- **Sample Output**:
  ```json
  {
    "success": true,
    "accountNumber": "1002345678",
    "filePath": "/Works/BASE/STATICVISH/MCP-Servers/AirSelangor/downloads/AirSelangor_Bill_1002345678.pdf",
    "fileSizeBytes": 142050
  }
  ```

### 7. `get_session_viewer`
Returns the interactive Steel Live Viewer URL so you can inspect or interact with the remote browser in real-time on any device across your Tailscale network.
- **Inputs**: None

---

## 💻 Prerequisites

1. **Node.js**: v20+ or v24+ installed on your host.
2. **Tailscale**: Active connection to your Tailnet so `steel.kingfisher-noodlefish.ts.net` is reachable.
3. **Air Selangor Account**: An active account registered on [airselangor.com](https://www.airselangor.com).

---

## ⚡ Quick Start Installation

```bash
# 1. Navigate to the server directory
cd /Works/BASE/STATICVISH/MCP-Servers/AirSelangor

# 2. Install dependencies
npm install

# 3. Create your local .env configuration
cp .env.example .env

# 4. Build TypeScript into dist/
npm run build
```

---

## ⚙️ Configuration (.env)

Edit the `.env` file in the project root:

```env
# Tailscale Steel Endpoint
STEEL_BASE_URL=https://steel.kingfisher-noodlefish.ts.net
STEEL_API_KEY=
STEEL_SESSION_TIMEOUT_MS=900000

# Air Selangor Portal Credentials
AIR_SELANGOR_ID=YOUR_NRIC_OR_SSM_OR_PASSPORT
AIR_SELANGOR_PASSWORD=YOUR_SECRET_PASSWORD
AIR_SELANGOR_ID_TYPE=nric

# Operational Settings
DOWNLOADS_DIR=./downloads
LOG_LEVEL=info
```

---

## 🔌 Client Integration

### Antigravity CLI / Antigravity IDE
Add the server configuration to your Antigravity settings or MCP configuration:

```json
{
  "mcpServers": {
    "airselangor": {
      "command": "node",
      "args": ["/Works/BASE/STATICVISH/MCP-Servers/AirSelangor/dist/index.js"],
      "env": {
        "STEEL_BASE_URL": "https://steel.kingfisher-noodlefish.ts.net",
        "AIR_SELANGOR_ID": "YOUR_NRIC_OR_SSM",
        "AIR_SELANGOR_PASSWORD": "YOUR_PASSWORD"
      }
    }
  }
}
```

### Claude Desktop
Add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "airselangor": {
      "command": "node",
      "args": ["/Works/BASE/STATICVISH/MCP-Servers/AirSelangor/dist/index.js"]
    }
  }
}
```

---

## 🔒 Security & Human-in-the-Loop Safeguards

- **No Local Browser Footprint**: No local Chromium binaries or temporary browser profiles stored unencrypted on the host machine.
- **Tailscale Overlay Encryption**: All communication between the MCP server and Steel takes place inside your private WireGuard-encrypted Tailscale network.
- **No Financial Secrets Stored**: Banking credentials, debit/credit cards, and TAC/OTP codes are **never** handled or saved by the server.
- **Live Viewer Handover**: Whenever visual verification or banking authentication is required, you are given a direct link (`sessionViewerUrl`) to complete it yourself securely in any browser on your Tailscale network.

---

## 🔧 Development & Troubleshooting

### Test Server Locally
You can test the MCP server over standard I/O by sending a JSON-RPC request:

```bash
node dist/index.js << 'EOF'
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
EOF
```

### Chromium DevTools Host-Header Resolution
Chromium's remote debugging security blocks non-IP and non-localhost `Host` headers. 
Our custom adapter in [`src/browser/steel.ts`](file:///Works/BASE/STATICVISH/MCP-Servers/AirSelangor/src/browser/steel.ts) automatically resolves `steel.kingfisher-noodlefish.ts.net` to its Tailscale IP (`100.77.67.114`) on the fly, ensuring CDP connects reliably every time.

### Available Scripts
- `npm run build`: Compiles TypeScript to `dist/`.
- `npm start`: Runs the built server via `node dist/index.js`.
- `npm run dev`: Runs with hot-reloading using `tsx`.
