use anyhow::Result;
use oli_server::app::logger::{format_log_with_color, LogLevel};
use oli_server::communication::rpc::RpcServer;
use oli_server::App;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Package version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main function to initialize and run the oli server
fn main() -> Result<()> {
    // Initialize app state
    let app = Arc::new(Mutex::new(App::new()));

    // Set up RPC server
    let mut rpc_server = RpcServer::new();

    // Get a clone of the event sender for use in closures
    let global_event_sender = rpc_server.event_sender();

    // Register all API methods
    register_model_interaction_apis(&mut rpc_server, &app, &global_event_sender);
    register_agent_control_apis(&mut rpc_server, &app);
    register_model_discovery_apis(&mut rpc_server, &app);
    register_task_management_apis(&mut rpc_server, &app);
    register_conversation_apis(&mut rpc_server, &app);
    register_system_apis(&mut rpc_server);

    // Register subscription handlers for real-time event streaming
    rpc_server.register_subscription_handlers();

    // Log that we've registered subscription handlers
    eprintln!(
        "{}",
        format_log_with_color(
            LogLevel::Info,
            "Registered subscription handlers for real-time event streaming"
        )
    );

    // Run the RPC server
    {
        // Log with INFO log level for visibility
        let starting_msg = format_log_with_color(LogLevel::Info, "Starting oli server");
        eprintln!("{}", starting_msg);

        // Log server started message before starting
        let success_msg = format_log_with_color(LogLevel::Info, "oli server started successfully");
        eprintln!("{}", success_msg);
    }
    rpc_server.run()?;

    Ok(())
}

/// Register APIs for model interaction
fn register_model_interaction_apis(
    rpc_server: &mut RpcServer,
    app: &Arc<Mutex<App>>,
    event_sender: &std::sync::mpsc::Sender<(String, serde_json::Value)>,
) {
    // Clone app state and event sender for query_model handler
    let app_clone = app.clone();
    let event_sender = event_sender.clone();

    // Register query_model method
    rpc_server.register_method("query_model", move |params| {
        let mut app = app_clone.lock().unwrap();

        // Extract query from params
        let prompt = params["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing prompt parameter"))?;

        // Get model index if provided
        let model_index = params["model_index"].as_u64().unwrap_or(0) as usize;

        // Check if agent mode is explicitly specified
        let use_agent = params["use_agent"].as_bool().unwrap_or(app.use_agent);

        // Update agent usage flag
        app.use_agent = use_agent;

        // Log model selection
        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Info,
                &format!(
                    "Using model at index: {} with agent mode: {}",
                    model_index, use_agent
                )
            )
        );

        // Send processing started event
        let _ = event_sender.send((
            "processing_started".to_string(),
            json!({
                "model_index": model_index,
                "use_agent": use_agent
            }),
        ));

        // Query the model
        match app.query_model(prompt) {
            Ok(response) => {
                // Send processing complete event
                let _ = event_sender.send(("processing_complete".to_string(), json!({})));

                Ok(json!({ "response": response }))
            }
            Err(err) => {
                // Send processing error event
                let _ = event_sender.send((
                    "processing_error".to_string(),
                    json!({ "error": err.to_string() }),
                ));

                Err(anyhow::anyhow!("Error querying model: {}", err))
            }
        }
    });
}

/// Register APIs for agent control
fn register_agent_control_apis(rpc_server: &mut RpcServer, app: &Arc<Mutex<App>>) {
    // Clone app state for set_agent_mode handler
    let app_clone = app.clone();

    // Register set_agent_mode method
    rpc_server.register_method("set_agent_mode", move |params| {
        let mut app = app_clone.lock().unwrap();

        // Get the agent mode parameter
        let use_agent = params["use_agent"].as_bool().unwrap_or(false);

        // Update the app state
        app.use_agent = use_agent;

        // Return success response
        Ok(json!({
            "success": true,
            "agent_mode": use_agent
        }))
    });
}

/// Register APIs for model discovery
fn register_model_discovery_apis(rpc_server: &mut RpcServer, app: &Arc<Mutex<App>>) {
    // Clone app state for get_available_models handler
    let app_clone = app.clone();

    // Register get_available_models method
    rpc_server.register_method("get_available_models", move |_| {
        let app = app_clone.lock().unwrap();

        // Get available models
        let models = app
            .available_models
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "id": m.file_name,
                    "description": m.description,
                    "supports_agent": m.has_agent_support()
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({ "models": models }))
    });
}

/// Register APIs for task management
fn register_task_management_apis(rpc_server: &mut RpcServer, app: &Arc<Mutex<App>>) {
    // Clone app state for get_tasks handler
    let app_clone = app.clone();

    // Register get_tasks method
    rpc_server.register_method("get_tasks", move |_| {
        let app = app_clone.lock().unwrap();
        Ok(json!({ "tasks": app.get_task_statuses() }))
    });

    // Clone app state for cancel_task handler
    let app_clone = app.clone();

    // Register cancel_task method
    rpc_server.register_method("cancel_task", move |params| {
        let mut app = app_clone.lock().unwrap();

        // Extract task ID from params if provided
        let task_id = params["task_id"].as_str();

        if let Some(task_id) = task_id {
            // Cancel specific task
            app.fail_current_task(&format!("Task canceled by user: {}", task_id));
            Ok(json!({ "success": true, "message": "Task canceled" }))
        } else {
            // Cancel current task if any
            if app.current_task_id.is_some() {
                app.fail_current_task("Task canceled by user");
                Ok(json!({ "success": true, "message": "Current task canceled" }))
            } else {
                Ok(json!({ "success": false, "message": "No active task to cancel" }))
            }
        }
    });
}

/// Register APIs for conversation management
fn register_conversation_apis(rpc_server: &mut RpcServer, app: &Arc<Mutex<App>>) {
    // Clone app state for clear_conversation handler
    let app_clone = app.clone();

    // Register clear_conversation method
    rpc_server.register_method("clear_conversation", move |_| {
        let mut app = app_clone.lock().unwrap();

        // Use the history.rs implementation to clear everything
        // This clears messages, summaries, session manager, and agent history
        use oli_server::app::history::ContextCompressor;
        app.clear_history();

        // Log the action
        eprintln!(
            "{}",
            format_log_with_color(LogLevel::Info, "Conversation history cleared")
        );

        // Return success
        Ok(json!({
            "success": true,
            "message": "Conversation history cleared"
        }))
    });
}

/// Register system APIs
fn register_system_apis(rpc_server: &mut RpcServer) {
    // Register get_version method to expose the Rust backend version
    rpc_server.register_method("get_version", move |_| Ok(json!({ "version": VERSION })));
}
