use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, Once};

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

/// Subscription manager for event-based communication
pub struct SubscriptionManager {
    subscribers: HashMap<String, Vec<u64>>, // event_type -> list of subscription IDs
    subscription_counter: AtomicU64,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self {
            subscribers: HashMap::new(),
            subscription_counter: AtomicU64::new(1),
        }
    }
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, event_type: &str) -> u64 {
        let sub_id = self.subscription_counter.fetch_add(1, Ordering::SeqCst);
        self.subscribers
            .entry(event_type.to_string())
            .or_default()
            .push(sub_id);
        sub_id
    }

    pub fn unsubscribe(&mut self, event_type: &str, sub_id: u64) -> bool {
        if let Some(subs) = self.subscribers.get_mut(event_type) {
            let pos = subs.iter().position(|&id| id == sub_id);
            if let Some(idx) = pos {
                subs.remove(idx);
                return true;
            }
        }
        false
    }

    pub fn has_subscribers(&self, event_type: &str) -> bool {
        self.subscribers
            .get(event_type)
            .is_some_and(|subs| !subs.is_empty())
    }

    pub fn get_subscribers(&self, event_type: &str) -> Vec<u64> {
        self.subscribers
            .get(event_type)
            .cloned()
            .unwrap_or_default()
    }
}

/// JSON-RPC server over stdio
pub struct RpcServer {
    methods: Arc<Mutex<HashMap<String, MethodHandler>>>,
    event_sender: Sender<(String, serde_json::Value)>,
    // Replace the standard mpsc::Receiver with an Arc<Mutex<>> wrapper to make it thread-safe
    event_receiver: Arc<Mutex<Receiver<(String, serde_json::Value)>>>,
    is_running: Arc<AtomicBool>,
    // Add subscription manager for real-time event streaming
    subscription_manager: Arc<Mutex<SubscriptionManager>>,
}

// Global RPC server instance
static mut GLOBAL_RPC_SERVER: Option<Arc<RpcServer>> = None;
static INIT: Once = Once::new();

// Clone implementation for RpcServer
impl Clone for RpcServer {
    fn clone(&self) -> Self {
        // Create a new channel for the cloned instance
        let (event_sender, event_receiver) = channel();

        Self {
            methods: self.methods.clone(),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            is_running: self.is_running.clone(),
            subscription_manager: self.subscription_manager.clone(),
        }
    }
}

/// Get global RPC server instance
#[allow(static_mut_refs)]
pub fn get_global_rpc_server() -> Option<Arc<RpcServer>> {
    unsafe { GLOBAL_RPC_SERVER.clone() }
}

/// Set global RPC server instance
fn set_global_rpc_server(server: Arc<RpcServer>) {
    INIT.call_once(|| unsafe {
        GLOBAL_RPC_SERVER = Some(server);
    });
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new() -> Self {
        let (event_sender, event_receiver) = channel();
        let server = Self {
            methods: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            is_running: Arc::new(AtomicBool::new(false)),
            subscription_manager: Arc::new(Mutex::new(SubscriptionManager::new())),
        };

        // Create a clone for global registration
        let server_clone = server.clone();

        // Register as global RPC server
        #[allow(clippy::arc_with_non_send_sync)]
        let server_arc = Arc::new(server_clone);
        set_global_rpc_server(server_arc);

        // Return the original server
        server
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

    /// Send a notification event - will send to all subscribers of this event type
    pub fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        // First, check if anyone is subscribed to this event
        let has_subscribers = {
            let manager = self.subscription_manager.lock().unwrap();
            manager.has_subscribers(method)
        };

        // Always send through the event channel for internal event processing
        self.event_sender
            .send((method.to_string(), params.clone()))?;

        // If this is not a subscribed event or there are no subscribers, we're done
        if !has_subscribers {
            return Ok(());
        }

        // For events with subscribers, we'll immediately send a notification through stdout
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        // Send directly to stdout to ensure immediate delivery
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        serde_json::to_writer(&mut stdout, &notification)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;

        Ok(())
    }

    /// Register subscription method handlers
    pub fn register_subscription_handlers(&mut self) {
        // Handle subscribe requests
        let sub_manager = self.subscription_manager.clone();
        self.register_method("subscribe", move |params| {
            let event_type = params
                .get("event_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing event_type parameter"))?;

            let mut manager = sub_manager.lock().unwrap();
            let sub_id = manager.subscribe(event_type);

            Ok(serde_json::json!({ "subscription_id": sub_id }))
        });

        // Handle unsubscribe requests
        let sub_manager = self.subscription_manager.clone();
        self.register_method("unsubscribe", move |params| {
            let event_type = params
                .get("event_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing event_type parameter"))?;

            let sub_id = params
                .get("subscription_id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing subscription_id parameter"))?;

            let mut manager = sub_manager.lock().unwrap();
            let success = manager.unsubscribe(event_type, sub_id);

            Ok(serde_json::json!({ "success": success }))
        });
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Run the RPC server, processing stdin and writing to stdout
    pub fn run(&self) -> Result<()> {
        // Set running state
        self.is_running.store(true, Ordering::SeqCst);

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
            if let Ok(receiver) = self.event_receiver.try_lock() {
                while let Ok((method, params)) = receiver.try_recv() {
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
        }

        // Set running state to false
        self.is_running.store(false, Ordering::SeqCst);

        Ok(())
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}
