use air_selangor_mcp::proto::{v1::*, PROTO_SCHEMA};
use prost::Message;

#[test]
fn test_proto_schema_content() {
    assert!(PROTO_SCHEMA.contains("syntax = \"proto3\";"));
    assert!(PROTO_SCHEMA.contains("package airselangor.v1;"));
    assert!(PROTO_SCHEMA.contains("service AirSelangorService"));
    assert!(PROTO_SCHEMA.contains("rpc Logout (LogoutRequest) returns (LogoutResponse);"));
    assert!(PROTO_SCHEMA.contains("message LogoutRequest"));
    assert!(PROTO_SCHEMA.contains("message LogoutResponse"));
    assert!(PROTO_SCHEMA.contains("message WaterAccount"));
    assert!(PROTO_SCHEMA.contains("message BillDetails"));
    assert!(PROTO_SCHEMA.contains("message DueBillSummary"));
    assert!(PROTO_SCHEMA.contains("message CheckDueBillsResponse"));
    assert!(PROTO_SCHEMA.contains("message DownloadResult"));
    assert!(PROTO_SCHEMA.contains("message SessionViewerResponse"));
}

#[test]
fn test_logout_protobuf_and_json() {
    let req = LogoutRequest {};
    let mut req_buf = Vec::new();
    req.encode(&mut req_buf).expect("encode failed");
    let _ = LogoutRequest::decode(req_buf.as_slice()).expect("decode failed");

    let resp = LogoutResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    };
    let mut resp_buf = Vec::new();
    resp.encode(&mut resp_buf).expect("encode failed");
    let decoded_resp = LogoutResponse::decode(resp_buf.as_slice()).expect("decode failed");
    assert!(decoded_resp.success);
    assert_eq!(decoded_resp.message, "Logged out successfully");

    let json_str = serde_json::to_string(&resp).expect("serialize json");
    let json_val: serde_json::Value = serde_json::from_str(&json_str).expect("deserialize json");
    assert_eq!(json_val["success"], true);
}

#[test]
fn test_water_account_protobuf_and_json() {
    let account = WaterAccount {
        account_number: "7400235711".to_string(),
        premise_address: "UNIT A-12-3, RESIDENSI SELANGOR".to_string(),
        display_nickname: "Home Water".to_string(),
        status: "ACTIVE".to_string(),
        current_balance: 35.50,
        is_overdue: false,
        due_date: "2026-11-09".to_string(),
        water_account_id: "WACC-9988".to_string(),
        bill_id: "BILL-1122".to_string(),
    };

    // Test Protobuf wire encoding and decoding
    let mut buf = Vec::new();
    account.encode(&mut buf).expect("encode failed");
    assert!(!buf.is_empty());

    let decoded = WaterAccount::decode(buf.as_slice()).expect("decode failed");
    assert_eq!(decoded.account_number, "7400235711");
    assert_eq!(decoded.premise_address, "UNIT A-12-3, RESIDENSI SELANGOR");
    assert_eq!(decoded.current_balance, 35.50);
    assert_eq!(decoded.due_date, "2026-11-09");
    assert_eq!(decoded.status, "ACTIVE");

    // Test JSON serde (used by MCP JSON-RPC)
    let json_str = serde_json::to_string(&account).expect("json serialize failed");
    let json_val: serde_json::Value = serde_json::from_str(&json_str).expect("json parse failed");
    assert_eq!(json_val["account_number"], "7400235711");
    assert_eq!(json_val["current_balance"], 35.5);

    let roundtrip_acc: WaterAccount = serde_json::from_str(&json_str).expect("json deserialize failed");
    assert_eq!(roundtrip_acc.account_number, account.account_number);
}

#[test]
fn test_bill_details_protobuf_and_json() {
    let details = BillDetails {
        account_number: "7400235711".to_string(),
        registered_name: "VISVESWARAN VAIDYANATHAN".to_string(),
        premise_address: "UNIT A-12-3, RESIDENSI SELANGOR".to_string(),
        bill_date: "2026-10-15".to_string(),
        due_date: "2026-11-09".to_string(),
        previous_balance: 0.0,
        current_charges: 35.50,
        total_outstanding: 35.50,
        minimum_payable: 35.50,
        status: "UNPAID".to_string(),
        days_remaining: 5,
        consumption_m3: 18.2,
        bill_number: "INV-2026-10-9988".to_string(),
        bill_id: "BILL-1122".to_string(),
        session_viewer_url: "https://steel.example.com/viewer".to_string(),
    };

    let mut buf = Vec::new();
    details.encode(&mut buf).expect("encode failed");
    let decoded = BillDetails::decode(buf.as_slice()).expect("decode failed");
    assert_eq!(decoded.account_number, details.account_number);
    assert_eq!(decoded.consumption_m3, 18.2);
    assert_eq!(decoded.total_outstanding, 35.50);

    let json_str = serde_json::to_string(&details).expect("json serialize failed");
    assert!(json_str.contains("INV-2026-10-9988"));
    let from_json: BillDetails = serde_json::from_str(&json_str).expect("json deserialize failed");
    assert_eq!(from_json.bill_number, "INV-2026-10-9988");
}

#[test]
fn test_due_bill_summary_and_response() {
    let due_bill = DueBillSummary {
        account_number: "7400235711".to_string(),
        premise_address: "UNIT A-12-3, RESIDENSI SELANGOR".to_string(),
        amount: 35.50,
        due_date: "2026-11-09".to_string(),
        urgency: "DUE_SOON".to_string(),
        days_remaining: 5,
    };

    let resp = CheckDueBillsResponse {
        summary: "1 bill due within 7 days (Total: RM 35.50)".to_string(),
        due_bills: vec![due_bill.clone()],
        session_viewer_url: "https://steel.example.com/viewer".to_string(),
    };

    let mut buf = Vec::new();
    resp.encode(&mut buf).expect("encode failed");
    let decoded = CheckDueBillsResponse::decode(buf.as_slice()).expect("decode failed");
    assert_eq!(decoded.summary, "1 bill due within 7 days (Total: RM 35.50)");
    assert_eq!(decoded.due_bills.len(), 1);
    assert_eq!(decoded.due_bills[0].days_remaining, 5);
}

#[test]
fn test_auth_status_and_download_result() {
    let auth = AuthStatus {
        is_authenticated: true,
        user_identifier: "VISVESWARAN VAIDYANATHAN".to_string(),
        steel_session_id: "steel-sess-12345".to_string(),
        session_viewer_url: "https://steel.example.com/session/123".to_string(),
        message: "Authenticated via Air Selangor Web Portal".to_string(),
    };

    let json_val = serde_json::to_value(&auth).unwrap();
    assert_eq!(json_val["is_authenticated"], true);
    assert_eq!(json_val["user_identifier"], "VISVESWARAN VAIDYANATHAN");

    let dl = DownloadResult {
        success: true,
        account_number: "7400235711".to_string(),
        file_path: "/tmp/downloads/bill.pdf".to_string(),
        file_size_bytes: 102400,
        error: String::new(),
    };

    let json_dl = serde_json::to_value(&dl).unwrap();
    assert_eq!(json_dl["file_size_bytes"], 102400);
    assert_eq!(json_dl["success"], true);

    let session_view = SessionViewerResponse {
        session_viewer_url: "https://steel.example.com/viewer".to_string(),
        steel_session_id: "steel-sess-12345".to_string(),
        message: "Live viewer active".to_string(),
    };
    let json_sv = serde_json::to_value(&session_view).unwrap();
    assert_eq!(json_sv["steel_session_id"], "steel-sess-12345");
}
