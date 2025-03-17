// Utility functions for the App

pub trait Scrollable {
    fn scroll_up(&mut self, amount: usize);
    fn scroll_down(&mut self, amount: usize);
    fn auto_scroll_to_bottom(&mut self);
}

// Error handling utilities
pub trait ErrorHandler {
    fn handle_error(&mut self, message: String);
}
