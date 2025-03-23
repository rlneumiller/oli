// Utility functions for the App

/// A scrollable state for managing UI scrolling and positioning
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    /// Current scroll position (0 = top of content)
    pub position: usize,
    /// Flag indicating if content should auto-scroll to bottom on new content
    pub follow_bottom: bool,
    /// Total content size (in lines) - updated by each render cycle
    pub content_size: usize,
    /// Visible area size (in lines) - updated by each render cycle
    pub viewport_size: usize,
}

impl ScrollState {
    /// Create a new scroll state
    pub fn new() -> Self {
        Self {
            position: 0,
            follow_bottom: true,
            content_size: 0,
            viewport_size: 0,
        }
    }

    /// Update the content and viewport sizes
    pub fn update_dimensions(&mut self, content_size: usize, viewport_size: usize) {
        self.content_size = content_size;
        self.viewport_size = viewport_size;

        // If we're following the bottom, update position
        if self.follow_bottom {
            self.scroll_to_bottom();
        } else {
            // Ensure position is still valid after update
            self.clamp_position();
        }
    }

    /// Get the maximum valid scroll position
    pub fn max_scroll(&self) -> usize {
        self.content_size.saturating_sub(self.viewport_size)
    }

    /// Ensure scroll position is within valid bounds
    pub fn clamp_position(&mut self) {
        let max = self.max_scroll();
        if self.position > max {
            self.position = max;
        }
    }

    /// Scroll down by the specified amount
    pub fn scroll_down(&mut self, amount: usize) {
        let max = self.max_scroll();

        // If we're already at max scroll, just turn on follow
        if self.position >= max {
            self.follow_bottom = true;
            return;
        }

        // Calculate new position without going beyond max
        self.position = (self.position + amount).min(max);

        // If we've scrolled to the bottom, enable follow
        self.follow_bottom = self.position >= max;
    }

    /// Scroll up by the specified amount
    pub fn scroll_up(&mut self, amount: usize) {
        // Whenever we scroll up, we disable following
        self.follow_bottom = false;

        // Don't underflow below 0
        self.position = self.position.saturating_sub(amount);
    }

    /// Scroll to the top of the content
    pub fn scroll_to_top(&mut self) {
        self.follow_bottom = false;
        self.position = 0;
    }

    /// Scroll to the bottom of the content
    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom = true;
        self.position = self.max_scroll();
    }

    /// Page up (scroll up by viewport height)
    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_size.saturating_sub(1).max(1));
    }

    /// Page down (scroll down by viewport height)
    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_size.saturating_sub(1).max(1));
    }

    /// Determine if we need to show the "more above" indicator
    pub fn has_more_above(&self) -> bool {
        self.position > 0
    }

    /// Determine if we need to show the "more below" indicator
    pub fn has_more_below(&self) -> bool {
        self.position < self.max_scroll()
    }
}

/// Interface for scrollable components
pub trait Scrollable {
    /// Get a mutable reference to the message scroll state
    fn message_scroll_state(&mut self) -> &mut ScrollState;

    /// Get a mutable reference to the task scroll state
    fn task_scroll_state(&mut self) -> &mut ScrollState;

    /// Scroll message view up by amount
    fn scroll_up(&mut self, amount: usize) {
        self.message_scroll_state().scroll_up(amount);
    }

    /// Scroll message view down by amount
    fn scroll_down(&mut self, amount: usize) {
        self.message_scroll_state().scroll_down(amount);
    }

    /// Auto scroll messages to bottom
    fn auto_scroll_to_bottom(&mut self) {
        self.message_scroll_state().scroll_to_bottom();
    }

    /// Scroll task list up by amount
    fn scroll_tasks_up(&mut self, amount: usize) {
        self.task_scroll_state().scroll_up(amount);
    }

    /// Scroll task list down by amount
    fn scroll_tasks_down(&mut self, amount: usize) {
        self.task_scroll_state().scroll_down(amount);
    }
}

// Error handling utilities
pub trait ErrorHandler {
    fn handle_error(&mut self, message: String);
}
