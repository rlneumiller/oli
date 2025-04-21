// Tests moved directly into anthropic.rs module
#[cfg(test)]
#[path = "test_anthropic.rs"]
mod test_anthropic {
    // Empty module - tests have been moved to the actual module implementation
}
mod test_api_client;
mod test_api_client_enum;
mod test_gemini;
mod test_ollama;
mod test_openai;
