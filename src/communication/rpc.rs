use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// JSON-RPC 2.0 request structure
#[derive(Debug, Deserialize)]
struct Request {
    // jsonrpc field is required by the JSON-RPC 2.0 spec
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<u64>,
    method: String,
    params: serde_json::Value,
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: String,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification structure
#[derive(Debug, Serialize)]
struct Notification {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

/// Method handler type
type MethodHandler =
    Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, anyhow::Error> + Send + Sync>;

/// JSON-RPC server over stdio
pub struct RpcServer {
    methods: Arc<Mutex<HashMap<String, MethodHandler>>>,
    event_sender: Sender<(String, serde_json::Value)>,
    event_receiver: Receiver<(String, serde_json::Value)>,
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new() -> Self {
        let (event_sender, event_receiver) = channel();
        Self {
            methods: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            event_receiver,
        }
    }

    /// Register a method handler
    pub fn register_method<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, anyhow::Error>
            + Send
            + Sync
            + 'static,
    {
        self.methods
            .lock()
            .unwrap()
            .insert(name.to_string(), Box::new(handler));
    }

    /// Get event sender for emitting events
    pub fn event_sender(&self) -> Sender<(String, serde_json::Value)> {
        self.event_sender.clone()
    }

    /// Run the RPC server, processing stdin and writing to stdout
    pub fn run(&self) -> Result<()> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        let reader = BufReader::new(stdin.lock());
        let methods = self.methods.clone();

        // Process each line of input as a JSON-RPC request
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            // Parse the request
            let request: Request = match serde_json::from_str(&line) {
                Ok(request) => request,
                Err(e) => {
                    // Send parse error
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(RpcError {
                            code: -32700,
                            message: "Parse error".to_string(),
                            data: Some(serde_json::Value::String(e.to_string())),
                        }),
                    };
                    serde_json::to_writer(&mut stdout, &response)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                    continue;
                }
            };

            // Check for method
            let methods = methods.lock().unwrap();
            let handler = match methods.get(&request.method) {
                Some(handler) => handler,
                None => {
                    // Send method not found error
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(RpcError {
                            code: -32601,
                            message: "Method not found".to_string(),
                            data: None,
                        }),
                    };
                    serde_json::to_writer(&mut stdout, &response)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                    continue;
                }
            };

            // Execute the method
            match handler(request.params.clone()) {
                Ok(result) => {
                    // Send success response
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(result),
                        error: None,
                    };
                    serde_json::to_writer(&mut stdout, &response)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                }
                Err(e) => {
                    // Send error response
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(RpcError {
                            code: -32603,
                            message: "Internal error".to_string(),
                            data: Some(serde_json::Value::String(e.to_string())),
                        }),
                    };
                    serde_json::to_writer(&mut stdout, &response)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                }
            };

            // Check for any events to send
            while let Ok((method, params)) = self.event_receiver.try_recv() {
                let notification = Notification {
                    jsonrpc: "2.0".to_string(),
                    method,
                    params,
                };
                serde_json::to_writer(&mut stdout, &notification)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
        }

        Ok(())
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}
