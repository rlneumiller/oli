mod agent;
pub mod apis;
pub mod app;
pub mod communication;
mod errors;
mod models;
mod prompts;
pub mod tools;

// Re-export key backend components
pub use agent::core::Agent;
pub use app::core::App;
pub use app::core::AppState;
pub use communication::rpc::RpcServer;
