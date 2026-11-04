# Air Selangor MCP Server Specification

## 1. Overview & Objectives

### 1.1 Purpose
The **Air Selangor MCP Server** is a Model Context Protocol (MCP) server that empowers AI assistants to monitor, manage, and facilitate water bill payments on the official [Air Selangor Portal](https://www.airselangor.com).

Because Air Selangor does not provide a public developer API and operates on a dynamic, client-side rendered Single-Page Application (Next.js), this MCP server utilizes **Steel (`steel.dev`)**, a dedicated AI-native browser automation sandbox self-hosted on the user's **Tailscale network**.

By connecting to Steel via its REST API and Chrome DevTools Protocol (CDP) WebSocket, the MCP server gains:
- **Offloaded Browser Infrastructure**: No local Chromium binaries or heavy X11/Wayland dependencies on the local host.
- **Tailscale-Accessible Live Session Viewer**: For interactive operations (CAPTCHA resolution, FPX online banking login, and TAC/OTP verification), the server provides a live interactive web stream URL (`session_viewer_url`) that the user can open in any browser across their Tailscale network.
- **Built-in Stealth & Fingerprint Management**: Mitigates automated bot detection and Cloudflare challenges out of the box.
- **Session Persistence**: Maintains browser contexts, cookies, and local storage state across server invocations.

### 1.2 Core Goals
1. **Automated Bill Inquiries**: Query account statuses, outstanding balances, latest charges, and due dates across all registered water accounts.
2. **Bill Notification & Due Tracking**: Identify bills due soon or overdue to prevent late penalties or service interruptions.
3. **e-Bill Statement Retrieval**: Download official PDF statements for record keeping and audit trails.
4. **Assisted Bill Payment**: Guide and stage payments via FPX, DuitNow, cards, or e-wallets with robust **Human-In-The-Loop (HITL)** safeguards via the Steel Live Viewer.
5. **Seamless Tailscale Connectivity**: Connect securely to the self-hosted Steel server running on the user's private Tailscale node.

---

## 2. Architecture & Technology Stack

```
+--------------------------------------------------------------+
|                    AI Assistant Client                       |
|           (Antigravity CLI / IDE / Claude Desktop)            |
+------------------------------+-------------------------------+
                               | MCP Protocol (JSON-RPC / stdio)
                               v
+--------------------------------------------------------------+
|                 Air Selangor MCP Server                      |
|                                                              |
|   +------------------------------------------------------+   |
|   | Tools Layer                                          |   |
|   |  - check_session       - get_bill_details            |   |
|   |  - login               - download_bill_pdf           |   |
|   |  - list_accounts       - initiate_bill_payment       |   |
|   |  - check_due_bills     - verify_payment_status       |   |
|   +--------------------------+---------------------------+   |
|                              |                               |
|   +--------------------------v---------------------------+   |
|   | Steel Browser Adapter                                |   |
|   |  - steel-sdk (Session creation, release, status)     |   |
|   |  - Playwright: chromium.connectOverCDP(websocketUrl) |   |
|   |  - Session Viewer URL generator                      |   |
|   +--------------------------+---------------------------+   |
+------------------------------|-------------------------------+
                               | Tailscale Network (HTTP / WSS)
                               v
+--------------------------------------------------------------+
|              Self-Hosted Steel.dev Instance                  |
|               (Tailscale Node :3000 / :9223)                 |
|                                                              |
|   - Sandboxed Chromium Engine                                |
|   - Anti-bot / Stealth & Proxy Layer                         |
|   - Built-in Live Interactive Session Viewer (:3000/ui)      |
|   - Persistent Browser Profiles & Storage                    |
+------------------------------+-------------------------------+
                               | HTTPS / DOM Interaction
                               v
+--------------------------------------------------------------+
|             Air Selangor Web Portal (Next.js)                |
|             https://www.airselangor.com                      |
+--------------------------------------------------------------+
```

### 2.1 Technology Stack
- **Language**: Rust (2021 edition, 1.85+)
- **Async Runtime**: `tokio` (multi-threaded)
- **MCP Stdio Transport**: JSON-RPC 2.0 stdio engine with stderr logging isolation
- **Protobuf 3 Codec**: `prost` & `prost-build` with Serde JSON derives
- **Automation Driver**: `chromiumoxide` (connecting remotely over CDP WebSocket to Steel)
- **HTTP Client**: `reqwest` (rustls-tls static linking)
- **Networking**: Tailscale private mesh connectivity to the self-hosted Steel node

---

## 3. Browser Automation via Self-Hosted Steel

### 3.1 Connection Workflow
1. The MCP server initializes a `SteelManager` targeting `STEEL_BASE_URL` (e.g., `https://steel.your-tailnet.ts.net`).
2. To execute an operation, the server requests an active session or creates a new session via Steel's REST API (`POST /v1/sessions`):
   ```rust
   let session = steel.create_session().await?;
   ```
3. `chromiumoxide` connects directly to the session's WebSocket CDP endpoint:
   ```rust
   let (mut browser, mut handler) = Browser::connect(&cdp_url).await?;
   let page = browser.new_page("about:blank").await?;
   ```

### 3.2 Live Session Viewer for Human-in-the-Loop (HITL)
One of the most powerful features of self-hosted Steel on Tailscale is the **Live Session Viewer (`session.sessionViewerUrl`)**:
- When user interaction is required—such as solving a CAPTCHA, logging into online banking (FPX), entering a TAC/OTP, or scanning a DuitNow QR code—the server returns the `sessionViewerUrl`.
- The user can click or open this URL on **any device on their Tailscale network** (laptop, phone, tablet) and interact with the remote browser in real-time.
- Once the user completes the action, the MCP server detects the URL/DOM state transition and resumes automated processing.

### 3.3 Session Caching & Cookie Persistence
- Steel provides persistent browser sessions and cookie management.
- The MCP server can attach an existing session ID (`STEEL_SESSION_ID`) or persist cookies so that repeated queries do not re-trigger login prompts.

---

## 4. MCP Tools Specification

### 4.1 `check_session`
*Checks whether the current cached browser session is authenticated on Air Selangor.*

- **Parameters**: None
- **Returns**:
  ```json
  {
    "isAuthenticated": true,
    "lastChecked": "2026-11-03T17:45:00Z",
    "userIdentifier": "900101-10-XXXX",
    "steelSessionId": "sess_01j7..."
  }
  ```

---

### 4.2 `login`
*Logs into the Air Selangor portal. If visual verification (CAPTCHA) or 2FA is encountered, provides the Steel Live Viewer URL.*

- **Parameters**:
  - `idType` *(string, optional, enum: `["nric", "passport", "police_id", "military_id", "ssm"]`)*: Identification type (defaults to environment setting).
  - `interactive` *(boolean, optional, default: false)*: Immediately opens session in interactive mode with live viewer URL.
- **Execution Flow**:
  1. Requests a browser session from the Steel instance.
  2. Navigates to `https://www.airselangor.com`.
  3. Clicks **Login / Register** -> **Log in**.
  4. Fills in the Login ID and Password from `.env`.
  5. If CAPTCHA or 2FA is triggered:
     - Returns `status: "AwaitingUserAction"` along with `sessionViewerUrl`.
     - Waits for successful redirect to `/my-water-account` or dashboard.
  6. On successful login, caches cookies and returns profile overview.
- **Returns**:
  ```json
  {
    "status": "Success",
    "message": "Successfully authenticated with Air Selangor.",
    "sessionViewerUrl": "http://100.x.y.z:3000/ui/sessions/sess_01j7..."
  }
  ```

---

### 4.3 `list_accounts`
*Retrieves all water accounts linked to the user's profile.*

- **Parameters**: None
- **Returns**:
  ```json
  {
    "accounts": [
      {
        "accountNumber": "1234567890",
        "premiseAddress": "NO 12, JALAN SS 2/1, 47300 PETALING JAYA, SELANGOR",
        "status": "Active",
        "currentBalance": 45.20,
        "isOverdue": false
      }
    ]
  }
  ```

---

### 4.4 `get_bill_details`
*Fetches comprehensive bill details for a specified water account.*

- **Parameters**:
  - `accountNumber` *(string, required)*: The 10-digit Air Selangor water account number.
- **Returns**:
  ```json
  {
    "accountNumber": "1234567890",
    "registeredName": "JOHN DOE",
    "billDate": "2026-10-15",
    "dueDate": "2026-11-14",
    "previousBalance": 0.00,
    "currentCharges": 45.20,
    "totalOutstanding": 45.20,
    "minimumPayable": 6.00,
    "status": "Due Soon",
    "daysRemaining": 11,
    "consumptionM3": 24.5
  }
  ```

---

### 4.5 `check_due_bills`
*Scans all registered accounts, calculates days until due date, and returns accounts requiring payment.*

- **Parameters**:
  - `dueWithinDays` *(number, optional, default: 7)*: Filter threshold for bills due within N days.
- **Returns**:
  ```json
  {
    "summary": "1 bill due within 7 days. Total amount due: RM 45.20",
    "dueBills": [
      {
        "accountNumber": "1234567890",
        "amount": 45.20,
        "dueDate": "2026-11-14",
        "urgency": "Urgent",
        "daysRemaining": 3
      }
    ]
  }
  ```

---

### 4.6 `download_bill_pdf`
*Fetches and downloads the official monthly e-Bill statement PDF.*

- **Parameters**:
  - `accountNumber` *(string, required)*: Target water account number.
  - `outputDirectory` *(string, optional)*: Destination path (defaults to `./downloads`).
- **Returns**:
  ```json
  {
    "success": true,
    "filePath": "/Works/BASE/STATICVISH/MCP-Servers/AirSelangor/downloads/eBill_1234567890_202610.pdf",
    "fileSizeBytes": 142050
  }
  ```

---

### 4.7 `initiate_bill_payment`
*Prepares and stages a bill payment session for a water account, then hands over the secure session to the user via the Steel Live Viewer.*

- **Parameters**:
  - `accountNumber` *(string, required)*: Target water account number.
  - `amount` *(number, optional)*: Amount to pay (defaults to total outstanding balance; minimum RM 6.00).
  - `paymentMethod` *(string, required, enum: `["fpx", "duitnow_qr", "credit_debit_card", "ewallet"]`)*: Selected payment channel.
- **Execution Flow**:
  1. Validates `accountNumber` and `amount >= 6.00`.
  2. Navigates on Steel to `https://www.airselangor.com/my-water-account/payment`.
  3. Selects the target account and inputs the payment amount.
  4. Selects the designated payment gateway option (FPX / DuitNow / Card / eWallet).
  5. Clicks **Continue** / **Pay Now** to advance to the payment gateway page.
  6. Pauses automated actions and returns the **Steel Live Viewer URL** so the user can enter their banking credentials, scan the DuitNow QR, or approve the TAC/Secure2u.
- **Returns**:
  ```json
  {
    "status": "AwaitingUserAuthorization",
    "accountNumber": "1234567890",
    "amount": 45.20,
    "paymentMethod": "fpx",
    "sessionViewerUrl": "http://100.x.y.z:3000/ui/sessions/sess_01j7...",
    "instruction": "Payment has been staged. Open the sessionViewerUrl on any device on your Tailscale network to authorize payment via your bank."
  }
  ```

---

### 4.8 `verify_payment_status`
*Verifies whether an initiated payment has cleared and updates balance records.*

- **Parameters**:
  - `accountNumber` *(string, required)*: The 10-digit water account number.
- **Execution**:
  - Navigates to payment history / account summary.
  - Extracts the latest transaction status and updated balance.
- **Returns**:
  ```json
  {
    "accountNumber": "1234567890",
    "previousBalance": 45.20,
    "updatedBalance": 0.00,
    "paymentStatus": "Paid",
    "receiptNumber": "AS-20261103-998822"
  }
  ```

---

## 5. Security & Human-In-The-Loop (HITL) Guardrails

| Guardrail | Implementation |
| :--- | :--- |
| **Tailscale Network Isolation** | Steel instance communicates solely over the private, encrypted Tailscale overlay network (e.g. `100.x.y.z`). |
| **Credential Safety** | Login ID and Password are stored exclusively in `.env` on the MCP host; never printed in logs or MCP tool responses. |
| **Zero Bank Data Exposure** | Banking credentials, debit/credit card CVV, and TAC/OTP codes are **never** handled or read by the AI agent. They are entered solely by the user via Steel's Live Viewer UI. |
| **Two-Phase Payment Gate** | The AI assistant only *stages* the checkout flow; the monetary authorization is executed directly by the human in the loop. |
| **Amount Validation** | Minimum payment limit is enforced (RM 6.00 per Air Selangor rules). Maximum allowable threshold (`PAYMENT_MAX_ALLOWABLE_RM`) prevents erroneous transactions. |

---

## 6. Error Handling & Edge Cases

1. **Tailscale / Steel Server Unreachable**:
   - The MCP server performs a health check ping to `STEEL_BASE_URL` on startup and returns clear troubleshooting guidance if the Tailscale node or port 3000 is down.
2. **CAPTCHA / Cloudflare Challenges**:
   - Captured by Steel. If unsolvable automatically, the server supplies the `sessionViewerUrl` so the user can solve it with one click.
3. **Session Expiry**:
   - Re-authenticates automatically using stored credentials without interrupting the user.
4. **Scheduled Maintenance**:
   - Air Selangor periodically schedules counter maintenance. The server detects maintenance alerts and outputs a structured notice with estimated resumption times.

---

## 7. Project File Structure

```
AirSelangor/
├── Cargo.toml                   # Rust package manifest & dependencies
├── Cargo.lock                   # Deterministic dependency tree
├── build.rs                     # Prost Protobuf 3 build script
├── Dockerfile                   # Multi-stage minimal static Alpine container
├── proto/
│   └── air_selangor.proto       # Protobuf 3 contracts
├── src/
│   ├── main.rs                  # CLI entrypoint (stdio JSON-RPC & stderr logging)
│   ├── lib.rs                   # Library root exports
│   ├── config.rs                # Environment configuration loader
│   ├── proto.rs                 # Generated Protobuf v1 modules & schema export
│   ├── steel.rs                 # Steel session lifecycle & remote CDP manager
│   ├── mcp/
│   │   └── mod.rs               # JSON-RPC 2.0 protocol handler & 8 tools
│   └── services/
│       ├── mod.rs               # Service module definitions
│       ├── auth.rs              # Human-mimic login & session validation
│       ├── account.rs           # Account listing & due bill scanning
│       └── bill.rs              # Billing metrics & Azure PDF statement downloader
├── tests/
│   ├── config_tests.rs          # Environment & fallback tests
│   ├── proto_tests.rs           # Protobuf 3 serialization & wire tests
│   └── mcp_protocol_tests.rs    # JSON-RPC tool schemas & execution tests
├── .env.example                 # Environment variables template
├── .gitignore
├── README.md                    # Setup and usage documentation
├── PRD.md                       # Product Requirements Document
└── SPEC.md                      # Technical Specification (This document)
```

---

## 8. Configuration & Environment Variables

Create a `.env` file in the project root:

```env
# Tailscale / Self-Hosted Steel Configuration
STEEL_BASE_URL=https://steel.your-tailnet.ts.net # Tailscale Steel endpoint
STEEL_API_KEY=                                          # Optional if configured on your self-hosted Steel
STEEL_SESSION_TIMEOUT_MS=900000                         # 15 minutes session timeout

# Air Selangor Portal Credentials
AIR_SELANGOR_ID=90010110XXXX
AIR_SELANGOR_PASSWORD=YourSecurePasswordHere
AIR_SELANGOR_ID_TYPE=nric            # nric | passport | ssm | police_id | military_id

# Operational Settings
DOWNLOADS_DIR=./downloads
PAYMENT_MAX_ALLOWABLE_RM=500.00
LOG_LEVEL=info
```

---

## 9. Next Steps / Implementation Roadmap

1. **Phase 1: Project Initialization**:
   - Initialize project with `@modelcontextprotocol/sdk`, `steel-sdk`, `playwright`, and `zod`.
   - Setup configuration with Tailscale `STEEL_BASE_URL`.
2. **Phase 2: Steel Integration & Connection Service**:
   - Implement `steel.ts` to manage sessions and generate `sessionViewerUrl`.
   - Connect Playwright over CDP to Steel's WebSocket.
3. **Phase 3: Authentication & Account Service**:
   - Implement login flow, session persistence, and CAPTCHA fallback via Live Viewer.
   - Implement `list_accounts`, `get_bill_details`, and `check_due_bills`.
4. **Phase 4: Bill Download & Payment Staging**:
   - Implement `download_bill_pdf`.
   - Implement `initiate_bill_payment` with Live Viewer handover for banking authorization, followed by `verify_payment_status`.
5. **Phase 5: Verification & Testing**:
   - Run end-to-end dry runs verifying session management and tool invocations via Antigravity CLI.
