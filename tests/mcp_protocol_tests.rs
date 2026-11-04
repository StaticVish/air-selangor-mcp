use air_selangor_mcp::config::AppConfig;
use air_selangor_mcp::mcp::{JsonRpcRequest, McpServer};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test]
async fn test_list_tools() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let tools_val = server.list_tools();
    let tools = tools_val["tools"].as_array().expect("tools must be an array");

    assert_eq!(tools.len(), 9);

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("name must be string"))
        .collect();

    assert!(tool_names.contains(&"check_session"));
    assert!(tool_names.contains(&"login"));
    assert!(tool_names.contains(&"logout"));
    assert!(tool_names.contains(&"list_accounts"));
    assert!(tool_names.contains(&"get_bill_details"));
    assert!(tool_names.contains(&"check_due_bills"));
    assert!(tool_names.contains(&"download_bill_pdf"));
    assert!(tool_names.contains(&"get_session_viewer"));
    assert!(tool_names.contains(&"get_proto_schema"));

    for tool in tools {
        assert!(tool.get("description").is_some());
        let schema = tool.get("inputSchema").expect("inputSchema required");
        assert_eq!(schema["type"], "object");
    }

    // Verify required params for get_bill_details
    let bill_tool = tools.iter().find(|t| t["name"] == "get_bill_details").unwrap();
    let req = bill_tool["inputSchema"]["required"].as_array().unwrap();
    assert!(req.iter().any(|r| r == "accountNumber"));

    // Verify required params for download_bill_pdf
    let dl_tool = tools.iter().find(|t| t["name"] == "download_bill_pdf").unwrap();
    let req_dl = dl_tool["inputSchema"]["required"].as_array().unwrap();
    assert!(req_dl.iter().any(|r| r == "accountNumber"));

    // Verify autoLogout option exists on data tools
    let acc_tool = tools.iter().find(|t| t["name"] == "list_accounts").unwrap();
    assert!(acc_tool["inputSchema"]["properties"].get("autoLogout").is_some());
}

#[tokio::test]
async fn test_handle_initialize() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: json!({}),
    };

    let resp = server.handle_request(req).await.expect("must return response");
    assert_eq!(resp.jsonrpc, "2.0");
    assert_eq!(resp.id, json!(1));
    assert!(resp.error.is_none());

    let result = resp.result.expect("result should exist");
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "air-selangor-mcp");
    assert_eq!(result["serverInfo"]["version"], "1.0.0");
}

#[tokio::test]
async fn test_handle_notification() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    // Notification without id
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "notifications/initialized".to_string(),
        params: json!({}),
    };

    let resp = server.handle_request(req).await;
    assert!(resp.is_none());
}

#[tokio::test]
async fn test_handle_ping() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!("ping-1")),
        method: "ping".to_string(),
        params: json!({}),
    };

    let resp = server.handle_request(req).await.expect("ping must reply");
    assert_eq!(resp.id, json!("ping-1"));
    assert_eq!(resp.result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_handle_tools_list() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(42)),
        method: "tools/list".to_string(),
        params: json!({}),
    };

    let resp = server.handle_request(req).await.expect("tools/list must reply");
    assert_eq!(resp.id, json!(42));
    let result = resp.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 9);
}

#[tokio::test]
async fn test_handle_get_proto_schema_call() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(100)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "get_proto_schema",
            "arguments": {}
        }),
    };

    let resp = server.handle_request(req).await.expect("tool call must reply");
    assert_eq!(resp.id, json!(100));
    assert!(resp.error.is_none());

    let result = resp.result.unwrap();
    let content = result["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");

    let text_val = content[0]["text"].as_str().unwrap();
    assert!(text_val.contains("proto3"));
    assert!(text_val.contains("airselangor.v1"));
    assert!(text_val.contains("service AirSelangorService"));
    assert!(text_val.contains("Logout"));
}

#[tokio::test]
async fn test_handle_logout_call() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(105)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "logout",
            "arguments": {}
        }),
    };

    let resp = server.handle_request(req).await.expect("logout must reply");
    assert_eq!(resp.id, json!(105));
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result["content"].as_array().unwrap();
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("Logged out successfully"));
}

#[tokio::test]
async fn test_credential_guard_blocks_unconfigured_calls() {
    let unconfigured = AppConfig {
        steel_base_url: "http://localhost:3000".to_string(),
        steel_api_key: None,
        steel_session_timeout_ms: 900_000,
        air_selangor_id: "".to_string(),
        air_selangor_password: "".to_string(),
        air_selangor_id_type: "nric".to_string(),
        downloads_dir: PathBuf::from("./downloads"),
        portal_url: "https://www.airselangor.com".to_string(),
        session_dir: PathBuf::from("/tmp/nonexistent/session"),
        session_state_file: PathBuf::from("/tmp/nonexistent/session/storageState.json"),
        auto_logout: false,
    };
    let server = McpServer::new(unconfigured);

    // Calling list_accounts with no credentials configured must fail immediately
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(200)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "list_accounts",
            "arguments": {}
        }),
    };

    let resp = server.handle_request(req).await.expect("must return error response");
    let result = resp.result.unwrap();
    assert_eq!(result["isError"], true);
    let content = result["content"].as_array().unwrap();
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("credentials are not configured"));

    // Check login without arguments on unconfigured server also fails immediately
    let login_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(201)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "login",
            "arguments": {}
        }),
    };
    let login_resp = server.handle_request(login_req).await.expect("must return error response");
    let login_result = login_resp.result.unwrap();
    assert_eq!(login_result["isError"], true);
    let login_text = login_result["content"][0]["text"].as_str().unwrap();
    assert!(login_text.contains("credentials are not configured"));
}

#[tokio::test]
async fn test_handle_unknown_tool() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(101)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "non_existent_tool",
            "arguments": {}
        }),
    };

    let resp = server.handle_request(req).await.expect("reply even on tool error");
    let result = resp.result.unwrap();
    assert_eq!(result["isError"], true);
    let content = result["content"].as_array().unwrap();
    assert!(content[0]["text"].as_str().unwrap().contains("Unknown tool"));
}

#[tokio::test]
async fn test_handle_unknown_method() {
    let config = AppConfig::load();
    let server = McpServer::new(config);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(999)),
        method: "unknown/method".to_string(),
        params: json!({}),
    };

    let resp = server.handle_request(req).await.expect("must return json-rpc error");
    assert_eq!(resp.id, json!(999));
    let err = resp.error.expect("error must be populated");
    assert_eq!(err.code, -32601);
    assert!(err.message.contains("Method not found"));
}
