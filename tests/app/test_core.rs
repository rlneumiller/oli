use anyhow::Result;
use oli_server::apis::api_client::SessionManager;
use oli_server::app::core::App;
use oli_server::models::ModelConfig;
use std::env;

#[test]
fn test_local_model_no_api_key_required() -> Result<()> {
    // Create a new App instance
    let mut app = App::new();

    // Add a mock local model to the available models
    app.available_models = vec![ModelConfig {
        name: "Test Local Model (local)".into(),
        file_name: "test-model".into(),
        description: "Test local model via Ollama".into(),
        recommended_for: "Testing".into(),
        supports_agent: true,
    }];

    // Ensure no API keys are set in the environment
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("OPENAI_API_KEY");
    env::remove_var("GEMINI_API_KEY");

    // Create a simple mock tokio runtime for the app
    app.tokio_runtime = Some(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?,
    );

    // Ensure we have a session manager
    if app.session_manager.is_none() {
        app.session_manager = Some(SessionManager::new(100));
    }

    // Mock the query_model function call
    // This would normally try to connect to Ollama, but we'll intercept before that happens
    // We're only testing that the API key check doesn't fail for local models

    // The test passes if this doesn't panic with an API key error
    let result = app.query_model("test prompt", Some(0));

    // The test should fail for other reasons (like Ollama not running)
    // but not because of missing API keys
    assert!(
        result.is_err(),
        "Expected query to fail for other reasons, but not due to API key issues"
    );

    // Verify the error is not about missing API keys
    let err_msg = result.unwrap_err().to_string();
    assert!(
        !err_msg.contains("No API key available"),
        "Test failed with API key error: {}",
        err_msg
    );

    Ok(())
}

#[test]
fn test_cloud_model_requires_api_key() -> Result<()> {
    // Create a new App instance
    let mut app = App::new();

    // Add a mock Claude model to the available models
    app.available_models = vec![ModelConfig {
        name: "Test Claude Model".into(),
        file_name: "claude-test".into(),
        description: "Test Claude model".into(),
        recommended_for: "Testing".into(),
        supports_agent: true,
    }];

    // Ensure no API keys are set in the environment
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("OPENAI_API_KEY");
    env::remove_var("GEMINI_API_KEY");

    // Create a simple mock tokio runtime for the app
    app.tokio_runtime = Some(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?,
    );

    // Ensure we have a session manager
    if app.session_manager.is_none() {
        app.session_manager = Some(SessionManager::new(100));
    }

    // Try to query the model, which should fail due to missing API key
    let result = app.query_model("test prompt", Some(0));

    // Verify the error is about missing API keys
    assert!(
        result.is_err(),
        "Expected query to fail due to missing API key"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No API key available"),
        "Error message should mention missing API key: {}",
        err_msg
    );

    Ok(())
}
