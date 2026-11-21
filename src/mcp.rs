//! MCP over stdio for restaurant sources.

use std::io::{BufRead, Write};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use crate::api::{self, Client};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: [&str; 2] = ["2025-06-18", "2024-11-05"];

pub async fn serve(client: Client) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    eprintln!("hkrestaurant MCP server listening on stdio");
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(value) => dispatch(&client, &value).await,
            Err(error) => Some(error_response(
                None,
                -32700,
                &format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

async fn dispatch(client: &Client, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => {
            return Some(error_response(
                id,
                -32600,
                "invalid request: missing method",
            ));
        }
    };
    if method.starts_with("notifications/") {
        return None;
    }
    match method {
        "initialize" => Some(success(id, initialize(request.get("params")))),
        "ping" => Some(success(id, json!({}))),
        "tools/list" => Some(success(id, tools())),
        "tools/call" => Some(call(client, id, request.get("params")).await),
        other => Some(error_response(
            id,
            -32601,
            &format!("method not found: {other}"),
        )),
    }
}

fn initialize(params: Option<&Value>) -> Value {
    let requested = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str);
    let version = requested
        .filter(|value| SUPPORTED_VERSIONS.contains(value))
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "hongkong-restaurant-skill",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools() -> Value {
    json!({ "tools": [
        {
            "name": "search_restaurant_licences",
            "description": "Search the official FEHD list of licensed restaurants by name, address, district or licence type.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Restaurant name or address substring."},
                    "district": {"type": "string", "description": "District substring, e.g. Central, Yau Tsim, Islands."},
                    "type": {"type": "string", "description": "Licence type substring or code: General, Light Refreshment, Marine, RL, RR, MR."},
                    "limit": {"type": "integer", "minimum": 0, "description": "Maximum rows; 0 means all. Default 50."}
                }
            }
        },
        {
            "name": "search_openrice",
            "description": "Search OpenRice Hong Kong by cuisine, dish or restaurant name. Uses the public website JSON endpoint.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Cuisine, dish or restaurant name."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 150, "description": "Maximum rows. Default 15."}
                },
                "required": ["query"]
            }
        },
        {
            "name": "list_foodpanda_vendors",
            "description": "List Foodpanda vendors delivering near a Hong Kong coordinate, optionally filtered client-side by name or cuisine.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "lat": {"type": "number", "description": "Latitude, e.g. 22.2793."},
                    "lng": {"type": "number", "description": "Longitude, e.g. 114.1628."},
                    "query": {"type": "string", "description": "Restaurant name or cuisine filter."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 48, "description": "Maximum rows. Default 24."}
                },
                "required": ["lat", "lng"]
            }
        },
        {
            "name": "list_restaurant_sources",
            "description": "Describe each restaurant source, its coverage, and known limitations (including Keeta not being supported).",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ]})
}

async fn call(client: &Client, id: Option<Value>, params: Option<&Value>) -> Value {
    let Some(params) = params else {
        return error_response(id, -32602, "missing params");
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let outcome: Result<Value> = async {
        match name {
            "search_restaurant_licences" => {
                let mut report = client.licences().await?;
                api::filter_licences(
                    &mut report,
                    args.get("query").and_then(Value::as_str),
                    args.get("district").and_then(Value::as_str),
                    args.get("type").and_then(Value::as_str),
                );
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(50);
                if limit > 0 {
                    report.restaurants.truncate(limit);
                    report.count = report.restaurants.len();
                }
                Ok(serde_json::to_value(report)?)
            }
            "search_openrice" => {
                let query = args
                    .get("query")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("query is required"))?;
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(15);
                Ok(serde_json::to_value(client.openrice(query, limit).await?)?)
            }
            "list_foodpanda_vendors" => {
                let lat = args
                    .get("lat")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| anyhow::anyhow!("lat is required"))?;
                let lng = args
                    .get("lng")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| anyhow::anyhow!("lng is required"))?;
                if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
                    bail!("lat must be -90..90 and lng must be -180..180");
                }
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(24);
                let vendors = client
                    .foodpanda(
                        lat,
                        lng,
                        args.get("query").and_then(Value::as_str),
                        limit,
                    )
                    .await?;
                Ok(serde_json::to_value(vendors)?)
            }
            "list_restaurant_sources" => {
                // Keep this in sync with the CLI's `sources` command.
                Ok(json!({
                    "sources": [
                        {"id": "fehd", "name": "FEHD restaurant licences", "kind": "official open data", "auth": "none"},
                        {"id": "openrice", "name": "OpenRice search", "kind": "undocumented public endpoint", "auth": "none"},
                        {"id": "foodpanda", "name": "Foodpanda nearby vendors", "kind": "undocumented public endpoint", "auth": "none"},
                        {"id": "keeta", "name": "Keeta", "kind": "not supported", "auth": "merchant credentials required"}
                    ]
                }))
            }
            other => bail!("unknown tool: {other}"),
        }
    }
    .await;
    match outcome {
        Ok(value) => success(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default()
                }],
                "structuredContent": value,
                "isError": false
            }),
        ),
        Err(error) => error_response(id, -32603, &format!("{error:#}")),
    }
}

fn success(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
