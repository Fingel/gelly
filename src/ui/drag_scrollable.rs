use gtk::prelude::*;

/// Trait for widgets that support drag scrolling in their lists
pub trait DragScrollable {
    fn get_last_drag_focused(&self) -> Option<i32>;
    fn set_last_drag_focused(&self, index: Option<i32>);
    fn clear_drag_state(&self) {
        self.set_last_drag_focused(None);
    }
}

/// Handle drag scroll logic for a song widget
pub fn handle_drag_scroll(song: &crate::ui::song::Song) {
    let Some(listbox) = song
        .parent()
        .and_then(|p| p.downcast::<gtk::ListBox>().ok())
    else {
        return;
    };

    let current_index = song.index();

    // Pre-calculate the potential focus targets
    let next_row = listbox.row_at_index(current_index + 1);
    let prev_row = listbox.row_at_index(current_index.saturating_sub(1));

    if let Some(scrollable) = find_drag_scrollable_ancestor(song) {
        match scrollable.get_last_drag_focused() {
            Some(last_index) if current_index > last_index => {
                // Scroll down
                if let Some(row) = next_row {
                    row.grab_focus();
                }
            }
            Some(last_index) if current_index < last_index => {
                // Scroll up
                if let Some(row) = prev_row {
                    row.grab_focus();
                }
            }
            _ => {
                // First drag enter or same position - scroll down
                if let Some(row) = next_row {
                    row.grab_focus();
                }
            }
        }
        scrollable.set_last_drag_focused(Some(current_index));
    } else {
        // Fallback for non-drag-scrollable - scroll down
        if let Some(row) = next_row {
            row.grab_focus();
        }
    }
}

/// Clear drag state for any drag-scrollable ancestor
pub fn clear_drag_state(song: &crate::ui::song::Song) {
    if let Some(scrollable) = find_drag_scrollable_ancestor(song) {
        scrollable.clear_drag_state();
    }
}

/// Helper function to find any ancestor that implements DragScrollable
pub fn find_drag_scrollable_ancestor(
    widget: &impl gtk::prelude::WidgetExt,
) -> Option<Box<dyn DragScrollable>> {
    if let Some(playlist_detail) = widget
        .ancestor(crate::ui::playlist_detail::PlaylistDetail::static_type())
        .and_then(|w| {
            w.downcast::<crate::ui::playlist_detail::PlaylistDetail>()
                .ok()
        })
    {
        return Some(Box::new(playlist_detail));
    }

    // Add Queue when ready
    // if let Some(queue) = widget
    //     .ancestor(crate::ui::queue::Queue::static_type())
    //     .and_then(|w| w.downcast::<crate::ui::queue::Queue>().ok())
    // {
    //     return Some(Box::new(queue));
    // }

    None
}
