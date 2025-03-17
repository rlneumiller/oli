mod components;
mod draw;
mod events;
mod guards;
mod messages;
mod styles;
// Re-export the run_app function as the main entry point
pub use events::run_app;
