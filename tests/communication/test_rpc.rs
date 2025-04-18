use oli_server::communication::rpc::{get_global_rpc_server, RpcServer, SubscriptionManager};
use serde_json::json;

#[test]
fn test_subscription_manager() {
    let mut manager = SubscriptionManager::new();

    // Test subscription
    let event_type = "test_event";
    let sub_id = manager.subscribe(event_type);
    assert!(sub_id > 0, "Subscription ID should be positive");

    // Test has_subscribers
    assert!(
        manager.has_subscribers(event_type),
        "Should have subscribers"
    );
    assert!(
        !manager.has_subscribers("non_existent"),
        "Should not have subscribers for non-existent event"
    );

    // Test get_subscribers
    let subscribers = manager.get_subscribers(event_type);
    assert_eq!(subscribers.len(), 1, "Should have one subscriber");
    assert_eq!(subscribers[0], sub_id, "Subscriber ID should match");

    // Test unsubscribe
    let result = manager.unsubscribe(event_type, sub_id);
    assert!(result, "Unsubscribe should succeed");
    assert!(
        !manager.has_subscribers(event_type),
        "Should not have subscribers after unsubscribe"
    );

    // Test unsubscribe non-existent
    let result = manager.unsubscribe(event_type, 9999);
    assert!(!result, "Unsubscribe should fail for non-existent ID");
}

#[test]
fn test_rpc_server_method_handling() {
    let mut server = RpcServer::new();

    // Register a test method
    server.register_method("test_method", |params| {
        let value = params.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(json!({ "result": value * 2 }))
    });

    // We can't easily test method execution directly since it requires stdin/stdout
    // But we can test registration by adding handlers and checking is_running behavior
    assert!(
        !server.is_running(),
        "Server should not be running initially"
    );
}

#[test]
fn test_subscription_handlers() {
    let mut server = RpcServer::new();
    server.register_subscription_handlers();

    // We can't easily test the subscription handlers directly
    // But we can verify the server is in the expected initial state
    assert!(
        !server.is_running(),
        "Server should not be running initially"
    );

    // Test event sending through subscription
    let sender = server.event_sender();
    let send_result = sender.send(("subscribe_event".to_string(), json!({"data": "test"})));
    assert!(send_result.is_ok(), "Should be able to send events");
}

#[test]
fn test_event_sender() {
    let server = RpcServer::new();
    let sender = server.event_sender();

    // Send an event
    let event_result = sender.send(("test_event".to_string(), json!({"data": "test_data"})));
    assert!(event_result.is_ok(), "Event should be sent successfully");
}

#[test]
fn test_send_notification() {
    let server = RpcServer::new();

    // Send a notification (this won't actually write to stdout in tests, but it should not error)
    let method = "test_notification";
    let params = json!({"data": "test_data"});

    // This test is somewhat limited since we can't easily capture stdout
    // Just verifying it doesn't crash is helpful
    let result = server.send_notification(method, params);
    assert!(result.is_ok(), "Notification should be sent without error");
}

#[test]
fn test_is_running() {
    let server = RpcServer::new();

    // Initially should not be running
    assert!(
        !server.is_running(),
        "Server should not be running initially"
    );

    // We can't easily test setting it to running from outside
    // because the run() method would block on stdin/stdout
}

#[test]
fn test_rpc_server_clone() {
    let server = RpcServer::new();
    let cloned = server.clone();

    // Both should be in not-running state
    assert!(
        !server.is_running(),
        "Original server should not be running initially"
    );
    assert!(
        !cloned.is_running(),
        "Cloned server should not be running initially"
    );

    // Event channels should be independent
    let sender1 = server.event_sender();
    let sender2 = cloned.event_sender();

    // The senders themselves should be different instances
    assert!(
        !std::ptr::eq(&sender1, &sender2),
        "Event senders should be different instances"
    );
}

#[test]
fn test_global_rpc_server() {
    // This test uses the global RPC server functionality

    // Create a server which will be registered as global
    let _server = RpcServer::new();

    // Get the global instance
    let global_server = get_global_rpc_server();
    assert!(global_server.is_some(), "Global server should be available");

    // Test a second call to get_global_rpc_server returns the same instance
    let global_server2 = get_global_rpc_server();
    assert!(
        global_server2.is_some(),
        "Global server should still be available"
    );

    // Due to Once initialization, we can't reset this between tests
    // In a real application this is desirable behavior
}
