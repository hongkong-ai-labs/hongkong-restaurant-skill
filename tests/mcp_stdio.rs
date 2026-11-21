//! Offline MCP stdio regression tests.
//!
//! Spawns the real binary, performs the handshake and exercises the error
//! paths that do not touch the network. The point is to catch servers that
//! panic, exit, or answer protocol notifications.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(20);

struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Server {
    fn start() -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hkrestaurant"));
        command.arg("mcp");

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn MCP server");

        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            child,
            stdin: Some(stdin),
            lines,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin still open");
        writeln!(stdin, "{message}").expect("write request");
        stdin.flush().expect("flush request");
    }

    fn receive(&mut self) -> Value {
        let line = self
            .lines
            .recv_timeout(WAIT)
            .unwrap_or_else(|_| panic!("server did not answer within {WAIT:?}"));
        serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("non-JSON line {line:?}: {error}"))
    }

    fn finish(mut self) {
        drop(self.stdin.take());
        let (sender, status) = mpsc::channel();
        let mut child = self.child;
        std::thread::spawn(move || {
            let _ = sender.send(child.wait());
        });
        let status = status
            .recv_timeout(WAIT)
            .expect("server exits after EOF")
            .expect("wait for server");
        assert!(status.success(), "server exit status: {status}");
    }
}

fn initialize(server: &mut Server) {
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "mcp-stdio-test", "version": "0" }
        }
    }));
    let initialize = server.receive();
    assert_eq!(initialize["id"], 1);
    assert!(
        initialize["result"]["protocolVersion"]
            .as_str()
            .is_some_and(|version| version.starts_with("20")),
        "unexpected initialize result: {initialize}"
    );
    assert!(
        initialize["result"]["serverInfo"]["name"]
            .as_str()
            .is_some_and(|name| !name.is_empty()),
        "serverInfo.name is missing: {initialize}"
    );
}

#[test]
fn answers_initialize_tools_list_and_unknown_tool() {
    let mut server = Server::start();
    initialize(&mut server);

    // A notification must not produce a reply, so the next line is tools/list.
    server.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    server.send(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let list = server.receive();
    assert_eq!(list["id"], 2, "notifications must not produce a response");
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list has no tools array: {list}"));
    assert!(!tools.is_empty(), "tools/list was empty");
    assert!(
        tools.iter().any(|tool| tool["name"] == "search_openrice"),
        "search_openrice is missing from tools/list"
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "not_a_tool", "arguments": {} }
    }));
    let unknown = server.receive();
    assert_eq!(unknown["id"], 3);
    assert!(
        unknown.get("error").is_some() || unknown["result"]["isError"] == true,
        "an unknown tool must fail without killing the server: {unknown}"
    );

    server.send(&json!({ "jsonrpc": "2.0", "id": 4, "method": "ping" }));
    let ping = server.receive();
    assert_eq!(ping["id"], 4);
    assert!(ping["result"].is_object());

    server.finish();
}

#[test]
fn reports_parse_errors_and_keeps_serving() {
    let mut server = Server::start();

    let stdin = server.stdin.as_mut().expect("stdin still open");
    writeln!(stdin, "this is not json").expect("write bad line");
    stdin.flush().expect("flush bad line");

    let parse_error = server.receive();
    assert_eq!(parse_error["error"]["code"], -32700);
    assert!(parse_error["id"].is_null());

    server.send(&json!({ "jsonrpc": "2.0", "id": "after", "method": "ping" }));
    let ping = server.receive();
    assert_eq!(ping["id"], "after");
    assert!(ping["result"].is_object());

    server.finish();
}
