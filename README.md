# Air Selangor MCP Server

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Model Context Protocol](https://img.shields.io/badge/MCP-1.6.1-green.svg)](https://modelcontextprotocol.io/)
[![Container: GHCR](https://img.shields.io/badge/Container-GHCR-blue?logo=docker)](https://github.com/StaticVish/air-selangor-mcp/pkgs/container/air-selangor-mcp)
[![Contracts: Protobuf 3](https://img.shields.io/badge/Contracts-Protobuf_3-blue?logo=google)](proto/air_selangor.proto)
[![Steel.dev](https://img.shields.io/badge/Browser_Engine-Steel.dev-orange.svg)](https://steel.dev/)
[![Tailscale](https://img.shields.io/badge/Network-Tailscale-indigo.svg)](https://tailscale.com/)
[![License: MIT-NC](https://img.shields.io/badge/License-MIT--NC-yellow.svg)](LICENSE)

A high-performance, statically compiled Model Context Protocol (MCP) server written in **Rust** for querying, monitoring, and managing [Air Selangor](https://www.airselangor.com) water utility accounts.

Instead of running heavy Chromium binaries locally, this server connects remotely to a self-hosted **Steel (`steel.dev`)** sandbox running securely over your private **Tailscale** mesh (`steel.your-tailnet.ts.net`), driven by Rust's native `chromiumoxide` CDP engine and typed with **Protobuf 3** schemas.

---

## 📑 Table of Contents
- [🐳 Zero-Install Quick Start (Docker / GHCR)](#-zero-install-quick-start-docker--ghcr)
- [Architecture & Design](#-architecture--design)
- [Protobuf 3 Request & Response Contracts](#-protobuf-3-request--response-contracts)
- [MCP Tools Reference](#-mcp-tools-reference)
- [Prerequisites](#-prerequisites)
- [Building from Source (Rust)](#-building-from-source-rust)
- [Automated Testing](#-automated-testing)
- [Configuration (.env)](#-configuration-env)
- [Client Integration (Antigravity, Claude Desktop, Cursor)](#-client-integration)
- [Security & Human-in-the-Loop Safeguards](#-security--human-in-the-loop-safeguards)
- [Automated Builds & Container Lifecycle](#-automated-builds--container-lifecycle)
- [License & Commercial Restrictions](#-license--commercial-restrictions)

---

## 🐳 Zero-Install Quick Start (Docker / GHCR)

**You do NOT need to clone this repository, install Rust, or install Node.js.** 

A pre-built, ultra-minimal, statically compiled multi-architecture container (`linux/amd64` and `linux/arm64` / `aarch64`) is published to the **GitHub Container Registry (GHCR)**.

### 1. Pull the Container Image
```bash
docker pull ghcr.io/staticvish/air-selangor-mcp:latest
```

### 2. Run Directly with Docker
```bash
docker run -i --rm \
  -e STEEL_BASE_URL="https://steel.your-tailnet.ts.net" \
  -e AIR_SELANGOR_ID="YOUR_NRIC_OR_PASSPORT" \
  -e AIR_SELANGOR_PASSWORD="YOUR_PASSWORD" \
  -v airselangor-session:/app/.session \
  -v $(pwd)/downloads:/app/downloads \
  ghcr.io/staticvish/air-selangor-mcp:latest
```

> [!TIP]
> The container is ultra-minimal (~21 MB total) with zero browser binaries or heavy runtime dependencies on your host because browser automation is executed remotely in your Steel sandbox.

---

## 🏗️ Architecture & Design

```
+--------------------------------------------------------------+
|                     AI Assistant Client                      |
|           (Antigravity CLI / IDE / Claude Desktop)           |
+------------------------------+-------------------------------+
                               | MCP Protocol (stdio / JSON-RPC 2.0)
                               v
+--------------------------------------------------------------+
|                   Air Selangor MCP Server                    |
|             (Statically Compiled Rust Binary)                |
|                                                              |
|   - Native Rust MCP Stdio Transport                          |
|   - Prost Protobuf 3 Codec & Schema Introspection            |
|   - chromiumoxide Remote CDP Client (WebSocket)              |
|   - Tailscale IPv4 DNS Resolver (Host Header Bypass)         |
|   - Encrypted Session State Rehydrator                       |
+------------------------------+-------------------------------+
                               | Tailscale Encrypted Mesh
                               v
+--------------------------------------------------------------+
|                Self-Hosted Steel.dev Sandbox                 |
|                 (steel.your-tailnet.ts.net)                  |
|                                                              |
|   - Anti-Bot Stealth Engine                                  |
|   - Isolated Remote Chromium Browser Instances               |
|   - Interactive Live Session Viewer UI                       |
+------------------------------+-------------------------------+
                               | HTTPS / DOM
                               v
+--------------------------------------------------------------+
|                  Air Selangor Web Portal                     |
|                https://www.airselangor.com                   |
+--------------------------------------------------------------+
```

### Core Features
- **Statically Linked Rust**: Instant startup, minimal memory footprint (~15 MB RAM), no runtime dependency on Node.js or Python.
- **Protobuf 3 Contract**: Fully documented requests, responses, and data models defined in `proto/air_selangor.proto` and inspectable via the `get_proto_schema` tool.
- **Human-Mimicking Keystroke Auth**: Handles Air Selangor modal popups (`#modal-root`) and types credentials with humanized random pauses and keyboard focus triggers (`Tab`).
- **Headless Document Extraction**: Direct binary streaming of official monthly e-Bill PDFs from Azure Blob Storage SAS URLs to disk.
- **Tailscale Host-Header Resolution**: Automatic background resolution of Tailscale MagicDNS domains to raw IPs to satisfy Chromium's remote debugging security constraints.

---

## 📜 Protobuf 3 Request & Response Contracts

All models are codified in [`proto/air_selangor.proto`](file:///Works/BASE/STATICVISH/MCP-Servers/AirSelangor/proto/air_selangor.proto). AI clients can inspect the live contract anytime by calling the `get_proto_schema` tool:

```protobuf
syntax = "proto3";

package airselangor.v1;

service AirSelangorService {
  rpc CheckSession (CheckSessionRequest) returns (AuthStatus);
  rpc Login (LoginRequest) returns (AuthStatus);
  rpc ListAccounts (ListAccountsRequest) returns (ListAccountsResponse);
  rpc GetBillDetails (GetBillDetailsRequest) returns (BillDetails);
  rpc CheckDueBills (CheckDueBillsRequest) returns (CheckDueBillsResponse);
  rpc DownloadBillPdf (DownloadBillPdfRequest) returns (DownloadResult);
  rpc GetSessionViewer (GetSessionViewerRequest) returns (SessionViewerResponse);
}

message WaterAccount {
  string account_number = 1;
  string premise_address = 2;
  string display_nickname = 3;
  string status = 4;
  double current_balance = 5;
  bool is_overdue = 6;
  string due_date = 7;
  string water_account_id = 8;
  string bill_id = 9;
}

message BillDetails {
  string account_number = 1;
  string registered_name = 2;
  string premise_address = 3;
  string bill_date = 4;
  string due_date = 5;
  double previous_balance = 6;
  double current_charges = 7;
  double total_outstanding = 8;
  double minimum_payable = 9;
  string status = 10;
  int32 days_remaining = 11;
  double consumption_m3 = 12;
  string bill_number = 13;
  string bill_id = 14;
  string session_viewer_url = 15;
}
```

---

## 🛠️ MCP Tools Reference

| Tool | Description | Inputs |
|---|---|---|
| `check_session` | Checks whether the current Steel browser session is authenticated on Air Selangor. | None |
| `login` | Logs into the portal. If visual verification (CAPTCHA) or 2FA is needed, provides the Steel Live Viewer URL. | `id` *(string)*, `password` *(string)* |
| `logout` | Logs out from the portal, clears all cookies, localStorage, session files, and terminates the remote Steel browser instance. | None |
| `list_accounts` | Lists all registered water accounts with their balance, address, and status. | `autoLogout` *(bool, optional)* |
| `get_bill_details` | Retrieves itemized billing statement metrics and water consumption ($m^3$). | `accountNumber` *(string, required)*, `autoLogout` *(bool, optional)* |
| `check_due_bills` | Scans all accounts and filters bills due within $N$ days (or overdue). | `dueWithinDays` *(number, default: 7)*, `autoLogout` *(bool, optional)* |
| `download_bill_pdf` | Downloads the official e-Bill PDF statement directly to the local filesystem. | `accountNumber` *(string, req)*, `outputDirectory` *(string)*, `autoLogout` *(bool, optional)* |
| `get_session_viewer`| Returns the interactive Steel Live Interactive Session Viewer URL. | None |
| `get_proto_schema` | Returns the raw Protobuf 3 contract definitions for AI context introspection. | None |

---

## 💻 Prerequisites

- **Option A (Containerized - Recommended)**: Docker installed on your host. No build tools required.
- **Option B (Source Build)**: 
  - Rust 1.85+ (`cargo`, `rustc`)
  - Protocol Buffers compiler (`protoc`)
- **Tailscale Network**: Connection to your Tailnet so your Steel node (`steel.your-tailnet.ts.net`) is reachable.
- **Air Selangor Account**: An active account registered on [airselangor.com](https://www.airselangor.com).

---

## ⚡ Building from Source (Rust)

```bash
# 1. Clone repository
git clone https://github.com/StaticVish/air-selangor-mcp.git
cd air-selangor-mcp

# 2. Configure environment
cp .env.example .env
# Edit .env with your credentials and Steel URL

# 3. Build optimized release binary
cargo build --release

# The compiled binary is located at target/release/air-selangor-mcp (~13MB stripped)
```

---

## 🧪 Automated Testing

Comprehensive test suites verify Protobuf serialization, JSON-RPC protocol compliance, and configuration parsing:

```bash
# Run all unit and integration tests
cargo test

# Run specific test suites
cargo test --test proto_tests
cargo test --test config_tests
cargo test --test mcp_protocol_tests
```

---

## ⚙️ Configuration (.env)

```env
# Tailscale Steel Endpoint
STEEL_BASE_URL=https://steel.your-tailnet.ts.net
STEEL_API_KEY=
STEEL_SESSION_TIMEOUT_MS=900000

# Air Selangor Portal Credentials
AIR_SELANGOR_ID=YOUR_NRIC_OR_SSM_OR_PASSPORT
AIR_SELANGOR_PASSWORD=YOUR_SECRET_PASSWORD
AIR_SELANGOR_ID_TYPE=nric

# Session & Security Settings
AUTO_LOGOUT=false # Set to true to automatically logout and wipe cookies after every query

# Operational Settings
DOWNLOADS_DIR=./downloads
```

> [!IMPORTANT]
> **Strict Credential Guard**: The server will **refuse to trigger remote browser sessions** if `AIR_SELANGOR_ID` and `AIR_SELANGOR_PASSWORD` are not configured (and no active session file exists). This prevents wasting compute or creating orphan Steel sessions when credentials are missing.

---

## 🔌 Client Integration

### Method 1: Using Docker (GHCR - Recommended)

#### Antigravity CLI
```bash
agy mcp add airselangor docker run -i --rm \
  -e STEEL_BASE_URL="https://steel.your-tailnet.ts.net" \
  -e AIR_SELANGOR_ID="YOUR_NRIC_OR_PASSPORT" \
  -e AIR_SELANGOR_PASSWORD="YOUR_PASSWORD" \
  -v airselangor-session:/app/.session \
  -v ~/Downloads:/app/downloads \
  ghcr.io/staticvish/air-selangor-mcp:latest
```

#### Antigravity IDE / Global MCP Config (`mcp_config.json`)
```json
{
  "mcpServers": {
    "airselangor": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e", "STEEL_BASE_URL=https://steel.your-tailnet.ts.net",
        "-e", "AIR_SELANGOR_ID=YOUR_NRIC_OR_PASSPORT",
        "-e", "AIR_SELANGOR_PASSWORD=YOUR_PASSWORD",
        "-v", "airselangor-session:/app/.session",
        "-v", "airselangor-downloads:/app/downloads",
        "ghcr.io/staticvish/air-selangor-mcp:latest"
      ]
    }
  }
}
```

#### Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "airselangor": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e", "STEEL_BASE_URL=https://steel.your-tailnet.ts.net",
        "-e", "AIR_SELANGOR_ID=YOUR_NRIC_OR_PASSPORT",
        "-e", "AIR_SELANGOR_PASSWORD=YOUR_PASSWORD",
        "-v", "airselangor-session:/app/.session",
        "-v", "airselangor-downloads:/app/downloads",
        "ghcr.io/staticvish/air-selangor-mcp:latest"
      ]
    }
  }
}
```

---

### Method 2: Using Local Rust Binary (Source Build)

#### Antigravity / Claude Desktop
```json
{
  "mcpServers": {
    "airselangor": {
      "command": "/path/to/air-selangor-mcp/target/release/air-selangor-mcp",
      "args": [],
      "env": {
        "STEEL_BASE_URL": "https://steel.your-tailnet.ts.net",
        "AIR_SELANGOR_ID": "YOUR_NRIC_OR_PASSPORT",
        "AIR_SELANGOR_PASSWORD": "YOUR_PASSWORD"
      }
    }
  }
}
```

---

## 🔒 Security & Human-in-the-Loop Safeguards

- **Zero Local Browser Footprint**: No local Chromium binaries or temporary browser profiles stored unencrypted on the host machine.
- **Tailscale Overlay Encryption**: All communication between the MCP server and Steel takes place inside your private WireGuard-encrypted Tailscale network.
- **No Financial Secrets Stored**: Banking credentials, debit/credit cards, and TAC/OTP codes are **never** handled or saved by the server.
- **Live Viewer Handover**: Whenever visual verification or banking authentication is required, you are given a direct link (`session_viewer_url`) to complete it yourself securely in any browser on your Tailscale network.
- **Pristine Stdio Protocol Stream**: All Rust logs are routed strictly to `stderr`, guaranteeing that the stdio JSON-RPC transport stream remains clean and uncorrupted.

---

## 🚀 Automated Builds & Container Lifecycle

The container image is built and maintained automatically via **GitHub Actions**:

- **Automated CI**: Runs `cargo test` across all unit, protocol, and integration test suites on every push.
- **Multi-Architecture**: Built natively with Docker Buildx & QEMU for both Intel/AMD64 (`linux/amd64`) and ARM64 / AArch64 (`linux/arm64`) runtimes (supporting Apple Silicon, AWS Graviton, and Raspberry Pi).
- **Free GitHub Container Registry (GHCR)**: Available publicly at `ghcr.io/staticvish/air-selangor-mcp:latest`.
- **Automatic Pruning**: Automatically retains only the **latest single container release** while preserving multi-arch child manifests, keeping registry storage clean, lightweight, and within GitHub's free tier.

---

## 📄 License & Commercial Restrictions

This project is licensed under the **MIT Non-Commercial License (MIT-NC)** - see the [LICENSE](LICENSE) file for full details.

- ✅ **Free for Personal Use**: You are free to use, fork, study, modify, and run this project for personal, educational, research, and non-commercial purposes.
- 🚫 **Commercial Use Restricted**: Commercial use, monetizing, selling, offering as a paid service, or embedding in proprietary platforms is strictly prohibited without prior written permission.
- 💬 **Commercial Licensing**: For commercial licensing inquiries or enterprise use, please reach out via [GitHub](https://github.com/StaticVish).
