use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::io;

use crate::file::parquet_ctx::ParquetCtx;
use crate::file::sample_data::ParquetSampleData;
use crate::file::sql::{SqlResult, run_sql};
use crate::tabs::TabManager;

pub struct AppRenderView<'a> {
    pub title: &'a str,
    pub parquet_ctx: &'a ParquetCtx,
    file_name: &'a str,
    tabs: &'a TabManager,
    pub state: &'a AppState,
}

impl<'a> AppRenderView<'a> {
    fn from_app(app: &'a App) -> Self {
        Self {
            title: "parqeye",
            parquet_ctx: app.parquet_ctx,
            file_name: &app.file_name,
            tabs: &app.tabs,
            state: &app.state,
        }
    }

    pub fn tabs(&self) -> &TabManager {
        self.tabs
    }

    pub fn file_name(&self) -> &str {
        self.file_name
    }

    pub fn state(&self) -> &AppState {
        self.state
    }
}

pub struct App<'a> {
    pub parquet_ctx: &'a ParquetCtx,
    pub file_name: String,
    pub exit: bool,
    pub tabs: TabManager,
    pub state: AppState,
}

pub struct AppState {
    horizontal_offset: usize,
    vertical_offset: usize,
    tree_scroll_offset: usize,
    data_vertical_scroll: usize,
    visible_data_rows: usize,
    // Search: "/" to enter search mode, Enter to filter, Esc to cancel or clear filter
    pub search_mode: bool,
    pub search_query: String,
    pub search_filter: Option<String>,
    pub filtered_sample_data: Option<ParquetSampleData>,
    // SQL tab
    pub sql_query: String,
    pub sql_result: Option<SqlResult>,
    // Row detail overlay: when Some(row_idx), show full row data for that row
    pub row_detail_row: Option<usize>,
    pub detail_scroll_offset: usize,     // vertical (lines)
    pub detail_scroll_horizontal: usize, // horizontal (columns)
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            horizontal_offset: 0,
            vertical_offset: 0,
            tree_scroll_offset: 0,
            data_vertical_scroll: 0,
            visible_data_rows: 20, // Default fallback
            search_mode: false,
            search_query: String::new(),
            search_filter: None,
            filtered_sample_data: None,
            sql_query: String::new(),
            sql_result: None,
            row_detail_row: None,
            detail_scroll_offset: 0,
            detail_scroll_horizontal: 0,
        }
    }

    pub fn reset(&mut self) {
        self.horizontal_offset = 0;
        self.vertical_offset = 0;
        self.tree_scroll_offset = 0;
        self.data_vertical_scroll = 0;
    }

    pub fn clear_search_filter(&mut self) {
        self.search_filter = None;
        self.filtered_sample_data = None;
    }

    pub fn horizontal_offset(&self) -> usize {
        self.horizontal_offset
    }

    pub fn vertical_offset(&self) -> usize {
        self.vertical_offset
    }

    pub fn down(&mut self) {
        self.vertical_offset += 1;
    }

    pub fn up(&mut self) {
        self.vertical_offset = self.vertical_offset.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.horizontal_offset += 1;
    }

    pub fn left(&mut self) {
        self.horizontal_offset = self.horizontal_offset.saturating_sub(1);
    }

    pub fn tree_scroll_offset(&self) -> usize {
        self.tree_scroll_offset
    }

    pub fn tree_scroll_up(&mut self) {
        self.tree_scroll_offset = self.tree_scroll_offset.saturating_sub(1);
    }

    pub fn tree_scroll_down(&mut self) {
        self.tree_scroll_offset += 1;
    }

    pub fn data_vertical_scroll(&self) -> usize {
        self.data_vertical_scroll
    }

    pub fn set_data_vertical_scroll(&mut self, scroll: usize) {
        self.data_vertical_scroll = scroll;
    }

    pub fn visible_data_rows(&self) -> usize {
        self.visible_data_rows
    }

    pub fn set_visible_data_rows(&mut self, rows: usize) {
        self.visible_data_rows = rows;
    }

    pub fn page_up(&mut self, visible_rows: usize, max_rows: usize) {
        // Move selection up by visible_rows
        self.vertical_offset = self.vertical_offset.saturating_sub(visible_rows);
        // Adjust scroll to keep selection visible
        self.adjust_scroll_to_selection(visible_rows, max_rows);
    }

    pub fn page_down(&mut self, visible_rows: usize, max_rows: usize) {
        // Move selection down by visible_rows, clamped to max_rows - 1
        self.vertical_offset =
            (self.vertical_offset + visible_rows).min(max_rows.saturating_sub(1));
        // Adjust scroll to keep selection visible
        self.adjust_scroll_to_selection(visible_rows, max_rows);
    }

    pub fn adjust_scroll_to_selection(&mut self, visible_rows: usize, max_rows: usize) {
        // Ensure selected row is visible in viewport
        if self.vertical_offset < self.data_vertical_scroll {
            // Selection is above viewport, scroll up
            self.data_vertical_scroll = self.vertical_offset;
        } else if self.vertical_offset >= self.data_vertical_scroll + visible_rows {
            // Selection is below viewport, scroll down
            self.data_vertical_scroll = self.vertical_offset.saturating_sub(visible_rows - 1);
        }

        // Clamp scroll to valid range
        let max_scroll = max_rows.saturating_sub(visible_rows);
        self.data_vertical_scroll = self.data_vertical_scroll.min(max_scroll);
    }
}

impl<'a> App<'a> {
    pub fn new(file_info: &'a ParquetCtx) -> Self {
        let sample_data_rows = file_info.sample_data.total_rows;

        let tab_manager = TabManager::new(
            file_info.schema.column_size(),
            file_info.row_groups.num_row_groups(),
            sample_data_rows,
        );

        Self {
            parquet_ctx: file_info,
            file_name: file_info.file_path.clone(),
            exit: false,
            tabs: tab_manager,
            state: AppState::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            // Calculate visible data rows based on terminal size
            let terminal_size = terminal.size()?;
            // Account for: header (3 lines), footer (1 line), table header (3 lines) = 7 lines total
            let visible_data_rows = (terminal_size.height.saturating_sub(7) as usize).max(1);
            self.state.set_visible_data_rows(visible_data_rows);

            let render_view = AppRenderView::from_app(self);
            terminal.draw(|frame| crate::ui::render_app(&render_view, frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        // Row detail overlay: Esc (close), ↑↓ PgUp PgDn (vertical), ←→ (horizontal), Ctrl+X (quit)
        if self.state.row_detail_row.is_some() {
            const DETAIL_PAGE_SIZE: usize = 10;
            match key_event.code {
                KeyCode::Esc => {
                    self.state.row_detail_row = None;
                }
                KeyCode::Up => {
                    self.state.detail_scroll_offset =
                        self.state.detail_scroll_offset.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.state.detail_scroll_offset += 1;
                }
                KeyCode::PageUp => {
                    self.state.detail_scroll_offset = self
                        .state
                        .detail_scroll_offset
                        .saturating_sub(DETAIL_PAGE_SIZE);
                }
                KeyCode::PageDown => {
                    self.state.detail_scroll_offset += DETAIL_PAGE_SIZE;
                }
                KeyCode::Left => {
                    self.state.detail_scroll_horizontal =
                        self.state.detail_scroll_horizontal.saturating_sub(1);
                }
                KeyCode::Right => {
                    self.state.detail_scroll_horizontal += 1;
                }
                KeyCode::Char('x') | KeyCode::Char('X')
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.exit();
                }
                _ => {}
            }
            return;
        }

        // Search mode: consume input until Enter or Esc
        if self.state.search_mode {
            match key_event.code {
                KeyCode::Esc => {
                    self.state.search_mode = false;
                    self.state.search_query.clear();
                }
                KeyCode::Enter => {
                    let query = self.state.search_query.clone();
                    let filtered = self.parquet_ctx.sample_data.filter_rows(&query);
                    self.state.search_filter = Some(query);
                    self.state.filtered_sample_data = Some(filtered);
                    self.state.reset();
                    self.state.search_mode = false;
                }
                KeyCode::Backspace => {
                    self.state.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.state.search_query.push(c);
                }
                _ => {}
            }
            return;
        }

        match key_event.code {
            KeyCode::Char('x') | KeyCode::Char('X')
                if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.exit()
            }
            KeyCode::Esc => {
                if self.state.search_filter.is_some() {
                    self.state.clear_search_filter();
                    self.state.reset();
                } else if self.tabs.active_tab().to_string() == "SQL" {
                    self.state.sql_query.clear();
                    self.state.sql_result = None;
                } else {
                    self.state.reset();
                }
            }
            KeyCode::Char('/') => {
                self.state.search_mode = true;
                self.state.search_query.clear();
            }
            KeyCode::Enter if self.tabs.active_tab().to_string() == "SQL" => {
                self.state.sql_result =
                    Some(run_sql(&self.parquet_ctx.file_path, &self.state.sql_query));
            }
            KeyCode::Tab => {
                self.tabs.next();
                self.state.reset();
            }
            KeyCode::BackTab => {
                self.tabs.prev();
                self.state.reset();
            }
            _ => {
                self.tabs
                    .active_tab()
                    .on_event(key_event, &mut self.state)
                    .unwrap();
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
