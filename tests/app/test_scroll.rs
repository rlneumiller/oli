#[cfg(test)]
mod scroll_tests {
    use oli_server::app::utils::ScrollState;

    #[test]
    fn test_scroll_state_creation() {
        let scroll = ScrollState::new();
        assert_eq!(scroll.position, 0);
        assert_eq!(scroll.content_size, 0);
        assert_eq!(scroll.viewport_size, 0);
        assert!(scroll.follow_bottom);
    }

    #[test]
    fn test_scroll_bounds() {
        let mut scroll = ScrollState::new();

        // Test with a content of 100 items and viewport of 20 items
        scroll.update_dimensions(100, 20);

        // Max scroll should be 80 (100 - 20)
        assert_eq!(scroll.max_scroll(), 80);

        // Since follow_bottom is true by default, position should be at max
        assert_eq!(scroll.position, 80);

        // Test scrolling beyond bounds
        scroll.scroll_down(1000);
        assert_eq!(scroll.position, 80); // Should stay at 80

        // Test scrolling up
        scroll.scroll_up(30);
        assert_eq!(scroll.position, 50);
        assert!(!scroll.follow_bottom); // Should turn off follow when scrolling up

        // Test scrolling up beyond top
        scroll.scroll_up(1000);
        assert_eq!(scroll.position, 0); // Should stop at 0
    }

    #[test]
    fn test_page_scroll() {
        let mut scroll = ScrollState::new();

        // Set up a viewport that's 10 items tall with 50 total items
        scroll.update_dimensions(50, 10);

        // Initially at bottom (40) due to follow_bottom being true
        assert_eq!(scroll.position, 40);

        // Page up should move up by (viewport_size - 1)
        scroll.page_up();
        assert_eq!(scroll.position, 40 - 9);

        // Another page up
        scroll.page_up();
        assert_eq!(scroll.position, 40 - 9 * 2);

        // One more should hit top (0) since we're at position 22 and going up by 9
        scroll.page_up();
        assert_eq!(scroll.position, 40 - 9 * 3);

        // Now page down
        scroll.page_down();
        assert_eq!(scroll.position, 22);

        // Directly to bottom
        scroll.scroll_to_bottom();
        assert_eq!(scroll.position, 40);
        assert!(scroll.follow_bottom);
    }

    #[test]
    fn test_scroll_indicators() {
        let mut scroll = ScrollState::new();

        // Set up a viewport that's 10 items tall with 50 total items
        scroll.update_dimensions(50, 10);

        // Initially at bottom (40) due to follow_bottom being true
        assert!(!scroll.has_more_below()); // At bottom, no more below
        assert!(scroll.has_more_above()); // Not at top, more above

        // Go to top
        scroll.scroll_to_top();
        assert!(scroll.has_more_below()); // Not at bottom, more below
        assert!(!scroll.has_more_above()); // At top, no more above

        // Go to middle
        scroll.position = 20;
        assert!(scroll.has_more_below()); // More below
        assert!(scroll.has_more_above()); // More above
    }

    #[test]
    fn test_dimension_updates() {
        let mut scroll = ScrollState::new();

        // Set initial dimensions
        scroll.update_dimensions(100, 20);
        assert_eq!(scroll.position, 80); // At bottom

        // Scroll to middle
        scroll.scroll_up(40);
        assert_eq!(scroll.position, 40);
        assert!(!scroll.follow_bottom);

        // Content size increases - position should stay the same since follow_bottom is false
        scroll.update_dimensions(150, 20);
        assert_eq!(scroll.position, 40);

        // Turn on follow_bottom and update dimensions again
        scroll.follow_bottom = true;
        scroll.update_dimensions(200, 20);
        assert_eq!(scroll.position, 180); // Should be at new bottom

        // Shrinking content while at the bottom - should remain at bottom
        scroll.update_dimensions(50, 20);
        assert_eq!(scroll.position, 30);

        // Shrinking viewport while at bottom - should remain at bottom
        scroll.update_dimensions(50, 10);
        assert_eq!(scroll.position, 40);
    }
}
