use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Stylize;
use ratatui::text::Span;
use std::io;

use crate::file::sql::SqlResult;
use crate::{app::AppState, tabs::Tab};

pub struct SqlTab;

impl SqlTab {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqlTab {
    fn default() -> Self {
        Self::new()
    }
}

impl Tab for SqlTab {
    fn on_event(&self, key_event: KeyEvent, state: &mut AppState) -> Result<(), io::Error> {
        match key_event.code {
            KeyCode::Backspace => {
                state.sql_query.pop();
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                if state.sql_result.as_ref().is_some_and(|r| matches!(r, SqlResult::Ok(_))) {
                    state.row_detail_row = Some(state.vertical_offset());
                    state.detail_scroll_offset = 0;
                    state.detail_scroll_horizontal = 0;
                }
            }
            KeyCode::Char(c) => {
                state.sql_query.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn instructions(&self) -> Vec<Span<'static>> {
        vec![
            "Enter".green(),
            " : Run query".into(),
            " | ".white(),
            "[Esc]".into(),
            " : Clear".into(),
            " | ".white(),
            "v".green(),
            " : Row detail".into(),
        ]
    }

    fn to_string(&self) -> String {
        "SQL".to_string()
    }
}
