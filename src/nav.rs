//! Pure grid-navigation state machine. `handle_key` mutates [`NavState`] and
//! returns an [`NavAction`] describing what happened — it never calls back
//! into application code and takes no closures, so it stays trivially unit
//! testable and reusable across any app that hands it a grid of N items.

use ratatui::crossterm::event::{KeyCode, KeyEvent};

/// Grid-navigation state: which item is selected, how many columns the grid
/// has, and how many items exist in total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavState {
    /// Index of the currently focused item.
    pub selected: usize,
    /// Number of columns in the grid (used to compute up/down movement).
    pub columns: usize,
    /// Total number of items in the grid.
    pub len: usize,
}

impl NavState {
    /// Construct a state with `len` items laid out in `columns` columns,
    /// focus starting at index 0. `columns` is clamped to at least 1 and
    /// `selected` is clamped into `0..len` (or 0 when `len == 0`).
    pub fn new(len: usize, columns: usize) -> Self {
        Self {
            selected: 0,
            columns: columns.max(1),
            len,
        }
    }
}

/// The result of a navigation key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    /// Focus moved to the given index (already applied to `NavState::selected`).
    Focus(usize),
    /// The focused item at the given index was activated (Enter).
    Activate(usize),
    /// The user asked to leave the grid (Esc).
    Back,
}

/// Handle one key event against `state`, mutating `state.selected` in place
/// for movement keys and returning the resulting action. Returns `None` for
/// keys this navigator doesn't handle.
///
/// - Left/h and Right/l move focus by one column, clamped at row edges (no
///   wraparound to the previous/next row).
/// - Up/k and Down/j move focus by one row (`columns` items), clamped at the
///   grid edges.
/// - Enter activates the focused item.
/// - Esc requests leaving the grid.
pub fn handle_key(state: &mut NavState, key: KeyEvent) -> Option<NavAction> {
    if state.len == 0 {
        return match key.code {
            KeyCode::Esc => Some(NavAction::Back),
            _ => None,
        };
    }

    let columns = state.columns.max(1);
    let last = state.len - 1;
    let row = state.selected / columns;
    let col = state.selected % columns;
    let row_start = row * columns;
    let row_end = (row_start + columns - 1).min(last);

    match key.code {
        KeyCode::Right | KeyCode::Char('l') => {
            if state.selected < row_end {
                state.selected += 1;
            }
            Some(NavAction::Focus(state.selected))
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if col > 0 {
                state.selected -= 1;
            }
            Some(NavAction::Focus(state.selected))
        }
        KeyCode::Down | KeyCode::Char('j') => {
            // Only move if a next row exists at all; when it does, land on
            // the same column or the last item in that row if it's shorter
            // (a partial row can only be the final one).
            let next_row_start = row_start + columns;
            if next_row_start <= last {
                state.selected = (next_row_start + col).min(last);
            }
            Some(NavAction::Focus(state.selected))
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected >= columns {
                state.selected -= columns;
            }
            Some(NavAction::Focus(state.selected))
        }
        KeyCode::Enter => Some(NavAction::Activate(state.selected)),
        KeyCode::Esc => Some(NavAction::Back),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // 6 items, 3 columns:
    // [0 1 2]
    // [3 4 5]

    #[test]
    fn right_moves_focus_within_row() {
        let mut s = NavState::new(6, 3);
        s.selected = 0;
        let action = handle_key(&mut s, key(KeyCode::Right));
        assert_eq!(action, Some(NavAction::Focus(1)));
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn right_clamps_at_row_end() {
        let mut s = NavState::new(6, 3);
        s.selected = 2;
        let action = handle_key(&mut s, key(KeyCode::Right));
        assert_eq!(action, Some(NavAction::Focus(2)));
        assert_eq!(s.selected, 2, "must not wrap into the next row");
    }

    #[test]
    fn l_key_behaves_like_right() {
        let mut s = NavState::new(6, 3);
        s.selected = 0;
        let action = handle_key(&mut s, key(KeyCode::Char('l')));
        assert_eq!(action, Some(NavAction::Focus(1)));
    }

    #[test]
    fn left_moves_focus_within_row() {
        let mut s = NavState::new(6, 3);
        s.selected = 2;
        let action = handle_key(&mut s, key(KeyCode::Left));
        assert_eq!(action, Some(NavAction::Focus(1)));
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn left_clamps_at_row_start() {
        let mut s = NavState::new(6, 3);
        s.selected = 3;
        let action = handle_key(&mut s, key(KeyCode::Left));
        assert_eq!(action, Some(NavAction::Focus(3)));
        assert_eq!(s.selected, 3, "must not wrap into the previous row");
    }

    #[test]
    fn h_key_behaves_like_left() {
        let mut s = NavState::new(6, 3);
        s.selected = 1;
        let action = handle_key(&mut s, key(KeyCode::Char('h')));
        assert_eq!(action, Some(NavAction::Focus(0)));
    }

    #[test]
    fn down_moves_focus_one_row() {
        let mut s = NavState::new(6, 3);
        s.selected = 0;
        let action = handle_key(&mut s, key(KeyCode::Down));
        assert_eq!(action, Some(NavAction::Focus(3)));
        assert_eq!(s.selected, 3);
    }

    #[test]
    fn down_clamps_into_shorter_next_row() {
        // 5 items, 3 columns: [0 1 2] [3 4]. From index 2 (row0 col2), the
        // next row exists but has no col2 -> land on its last item (4).
        let mut s = NavState::new(5, 3);
        s.selected = 2;
        let action = handle_key(&mut s, key(KeyCode::Down));
        assert_eq!(action, Some(NavAction::Focus(4)));
        assert_eq!(s.selected, 4);
    }

    #[test]
    fn down_clamps_at_grid_bottom() {
        let mut s = NavState::new(6, 3);
        s.selected = 3;
        let action = handle_key(&mut s, key(KeyCode::Down));
        assert_eq!(action, Some(NavAction::Focus(3)));
        assert_eq!(s.selected, 3);
    }

    #[test]
    fn j_key_behaves_like_down() {
        let mut s = NavState::new(6, 3);
        s.selected = 0;
        let action = handle_key(&mut s, key(KeyCode::Char('j')));
        assert_eq!(action, Some(NavAction::Focus(3)));
    }

    #[test]
    fn up_moves_focus_one_row() {
        let mut s = NavState::new(6, 3);
        s.selected = 4;
        let action = handle_key(&mut s, key(KeyCode::Up));
        assert_eq!(action, Some(NavAction::Focus(1)));
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn up_clamps_at_grid_top() {
        let mut s = NavState::new(6, 3);
        s.selected = 1;
        let action = handle_key(&mut s, key(KeyCode::Up));
        assert_eq!(action, Some(NavAction::Focus(1)));
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn k_key_behaves_like_up() {
        let mut s = NavState::new(6, 3);
        s.selected = 4;
        let action = handle_key(&mut s, key(KeyCode::Char('k')));
        assert_eq!(action, Some(NavAction::Focus(1)));
    }

    #[test]
    fn enter_activates_selected() {
        let mut s = NavState::new(6, 3);
        s.selected = 4;
        let action = handle_key(&mut s, key(KeyCode::Enter));
        assert_eq!(action, Some(NavAction::Activate(4)));
        assert_eq!(s.selected, 4, "activate must not move focus");
    }

    #[test]
    fn esc_requests_back() {
        let mut s = NavState::new(6, 3);
        s.selected = 4;
        let action = handle_key(&mut s, key(KeyCode::Esc));
        assert_eq!(action, Some(NavAction::Back));
        assert_eq!(s.selected, 4, "back must not move focus");
    }

    #[test]
    fn unhandled_key_returns_none() {
        let mut s = NavState::new(6, 3);
        let action = handle_key(&mut s, key(KeyCode::Char('z')));
        assert_eq!(action, None);
    }

    #[test]
    fn empty_grid_only_handles_esc() {
        let mut s = NavState::new(0, 3);
        assert_eq!(handle_key(&mut s, key(KeyCode::Right)), None);
        assert_eq!(handle_key(&mut s, key(KeyCode::Enter)), None);
        assert_eq!(handle_key(&mut s, key(KeyCode::Esc)), Some(NavAction::Back));
    }
}
