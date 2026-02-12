use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::{Color, Position},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

use crate::app::AppRenderView;
use crate::components::{
    DataTable, FileSchemaTable, RowGroupColumnMetadataComponent, RowGroupMetadata,
    RowGroupProgressBar, SchemaTreeComponent, ScrollbarComponent,
};
use crate::file::sql::SqlResult;
use crate::file::Renderable;

pub fn render_app<'a, 'b>(app: &'b AppRenderView<'a>, frame: &mut Frame)
where
    'b: 'a,
{
    frame.render_widget(AppWidget(app), frame.area());
}

struct AppWidget<'a>(&'a AppRenderView<'a>);

impl<'a> AppWidget<'a> {
    // Helper function to calculate the tree index of the selected primitive column
    fn calculate_selected_tree_index(&self, vertical_offset: usize) -> Option<usize> {
        if vertical_offset == 0 {
            return None;
        }

        let primitive_to_schema_map: Vec<usize> = self
            .0
            .parquet_ctx
            .schema
            .columns
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                matches!(line, crate::file::schema::SchemaInfo::Primitive { .. }).then_some(idx)
            })
            .collect();

        primitive_to_schema_map.get(vertical_offset - 1).copied()
    }

    // Helper function to calculate adjusted scroll offset to keep selected item visible
    fn calculate_scroll_to_show_item(
        &self,
        selected_tree_idx: Option<usize>,
        current_scroll: usize,
        visible_items: usize,
    ) -> usize {
        match selected_tree_idx {
            Some(idx) => {
                // Ensure selected item is visible
                if idx < current_scroll {
                    idx
                } else if idx >= current_scroll + visible_items {
                    idx.saturating_sub(visible_items - 1)
                } else {
                    current_scroll
                }
            }
            None => current_scroll,
        }
    }

    // Calculate the adjusted scroll offset for the schema tree
    fn calculate_adjusted_scroll_offset(&self, visible_tree_items: usize) -> usize {
        let selected_tree_idx =
            self.calculate_selected_tree_index(self.0.state().vertical_offset());
        self.calculate_scroll_to_show_item(
            selected_tree_idx,
            self.0.state().tree_scroll_offset(),
            visible_tree_items,
        )
    }

    // Calculate the total width needed for the tree section (including scrollbar if needed)
    fn calculate_tree_width(&self, tree_width: u16, needs_scrollbar: bool) -> u16 {
        if needs_scrollbar {
            tree_width + 2 // +1 for scrollbar, +1 for spacing
        } else {
            tree_width + 1
        }
    }

    // Calculate tree width for row groups view (slightly different spacing)
    fn calculate_tree_width_for_row_groups(&self, tree_width: u16, needs_scrollbar: bool) -> u16 {
        if needs_scrollbar {
            tree_width + 2 // +1 for scrollbar, +1 for spacing
        } else {
            tree_width
        }
    }

    // Render the schema tree section (tree + optional scrollbar
    #[allow(clippy::too_many_arguments)]
    fn render_schema_tree_section(
        &self,
        area: Rect,
        tree_width: u16,
        needs_scrollbar: bool,
        total_tree_items: usize,
        visible_tree_items: usize,
        adjusted_scroll: usize,
        buf: &mut Buffer,
    ) {
        if needs_scrollbar {
            let [tree_area, scrollbar_area] =
                Layout::horizontal([Constraint::Length(tree_width + 1), Constraint::Length(1)])
                    .areas(area);

            self.render_schema_tree_with_scroll(tree_area, adjusted_scroll, buf);

            ScrollbarComponent::vertical(total_tree_items, visible_tree_items, adjusted_scroll)
                .render(scrollbar_area, buf);
        } else {
            self.render_schema_tree_with_scroll(area, adjusted_scroll, buf);
        }
    }

    // Render the schema table
    fn render_schema_table(&self, area: Rect, adjusted_scroll: usize, buf: &mut Buffer) {
        FileSchemaTable::new(&self.0.parquet_ctx.schema)
            .with_selected_index(self.0.state().vertical_offset())
            .with_horizontal_scroll(self.0.state().horizontal_offset())
            .with_vertical_scroll(adjusted_scroll)
            .render(area, buf);
    }

    fn render_tabs_view(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::LightYellow));
        let inner_area = block.inner(area);
        block.render(area, buf);

        let file_name_length = self.0.file_name().len() as u16;

        let [tabs_area, file_name_area] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(file_name_length)])
                .areas(inner_area);
        self.0.tabs().render_content(tabs_area, buf);
        self.0.file_name().green().render(file_name_area, buf);
    }

    fn render_footer_view(&self, area: Rect, buf: &mut Buffer) {
        let title_width = self.0.title.len() as u16;
        let [title_area, footer_area] =
            Layout::horizontal([Constraint::Length(title_width), Constraint::Fill(1)]).areas(area);
        self.0.title.bold().fg(Color::Green).render(title_area, buf);

        if self.0.state().search_mode {
            let prompt = format!("Search: {}|", self.0.state().search_query);
            let line = Line::from(vec![
                prompt.green(),
                "  Enter=filter, Esc=cancel".into(),
            ]);
            line.render(footer_area, buf);
        } else if self.0.state().search_filter.is_some() {
            let n = self.0
                .state()
                .filtered_sample_data
                .as_ref()
                .map(|d| d.total_rows)
                .unwrap_or(0);
            use ratatui::text::Line;
            let mut span = self.0.tabs().active_tab().instructions();
            if !span.is_empty() {
                span.push(" - ".into());
            }
            span.extend(vec![
                format!("{} rows filtered", n).green(),
                " (Esc to show all)".into(),
            ]);
            Line::from(span).render(footer_area, buf);
        } else {
            self.0.tabs().render_instructions(footer_area, buf);
        }
    }

    fn render_metadata_view(&self, area: Rect, buf: &mut Buffer) {
        // render the metadata
        self.0.parquet_ctx.metadata.render_content(area, buf);
    }

    fn render_schema_view(&self, area: Rect, buf: &mut Buffer) {
        let tree_width = self.0.parquet_ctx.schema.tree_width() as u16;
        let total_tree_items = self.0.parquet_ctx.schema.columns.len();
        let visible_tree_items = area.height.saturating_sub(2) as usize;

        let needs_scrollbar = total_tree_items > visible_tree_items;
        let adjusted_scroll = self.calculate_adjusted_scroll_offset(visible_tree_items);
        let tree_total_width = self.calculate_tree_width(tree_width, needs_scrollbar);

        let [tree_container_area, central_area] =
            Layout::horizontal([Constraint::Length(tree_total_width), Constraint::Fill(1)])
                .areas(area);

        self.render_schema_tree_section(
            tree_container_area,
            tree_width,
            needs_scrollbar,
            total_tree_items,
            visible_tree_items,
            adjusted_scroll,
            buf,
        );
        self.render_schema_table(central_area, adjusted_scroll, buf);
    }

    fn render_schema_tree_with_scroll(&self, area: Rect, scroll_offset: usize, buf: &mut Buffer) {
        SchemaTreeComponent::new(&self.0.parquet_ctx.schema.columns)
            .with_title("Schema Tree".to_string())
            .with_selected_index(self.0.state().vertical_offset())
            .with_scroll_offset(scroll_offset)
            .render(area, buf);
    }

    fn render_row_groups_view(&self, area: Rect, buf: &mut Buffer) {
        let tree_width = self.0.parquet_ctx.schema.tree_width() as u16;
        let total_tree_items = self.0.parquet_ctx.schema.columns.len();
        let visible_tree_items = area.height.saturating_sub(2) as usize;

        let needs_scrollbar = total_tree_items > visible_tree_items;
        let adjusted_scroll = self.calculate_adjusted_scroll_offset(visible_tree_items);
        let tree_total_width =
            self.calculate_tree_width_for_row_groups(tree_width, needs_scrollbar);

        let [tree_container_area, main_area] =
            Layout::horizontal([Constraint::Length(tree_total_width), Constraint::Fill(1)])
                .areas(area);

        self.render_schema_tree_section(
            tree_container_area,
            tree_width,
            needs_scrollbar,
            total_tree_items,
            visible_tree_items,
            adjusted_scroll,
            buf,
        );

        let [rg_progress, central_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(main_area);

        RowGroupProgressBar::new(
            &self.0.parquet_ctx.row_groups.row_groups,
            self.0.state().horizontal_offset(),
        )
        .render(rg_progress, buf);

        if self.0.state().vertical_offset() > 0 {
            RowGroupColumnMetadataComponent::new(
                &self.0.parquet_ctx.row_groups.row_groups[self.0.state().horizontal_offset()]
                    .column_metadata[self.0.state().vertical_offset() - 1],
            )
            .render(central_area, buf);
        } else {
            // Display row group level statistics and charts when no column is selected
            RowGroupMetadata::new(
                &self.0.parquet_ctx.row_groups.row_groups,
                &self.0.parquet_ctx.row_groups.avg_median_stats,
                self.0.state().horizontal_offset(),
            )
            .render(central_area, buf);
        }
    }

    fn render_visualize_view(&self, area: Rect, buf: &mut Buffer) {
        let data = self
            .0
            .state()
            .filtered_sample_data
            .as_ref()
            .unwrap_or(&self.0.parquet_ctx.sample_data);
        let mut table = DataTable::new(data)
            .with_horizontal_scroll(self.0.state().horizontal_offset())
            .with_vertical_scroll(self.0.state().data_vertical_scroll())
            .with_selected_row(Some(self.0.state().vertical_offset()));
        if self.0.state().search_filter.is_some() {
            table = table.with_title(format!(
                "Data (filtered: {} rows)",
                data.total_rows
            ));
        }
        table.render(area, buf)
    }

    fn render_sql_view(&self, area: Rect, buf: &mut Buffer) {
        let [input_area, results_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" SQL ");
        let inner_input = block.inner(input_area);
        block.render(input_area, buf);

        let query_line = format!("SQL> {}", self.0.state().sql_query);
        Paragraph::new(Line::from(Span::raw(query_line)))
            .render(inner_input, buf);

        // Cursor at end of SQL input
        let cursor_x = inner_input.x + 5 + (self.0.state().sql_query.chars().count() as u16);
        let cursor_y = inner_input.y;
        if cursor_x < inner_input.x + inner_input.width {
            if let Some(cell) = buf.cell_mut(Position::new(cursor_x, cursor_y)) {
                cell.set_symbol(" ");
                cell.set_style(Style::default().bg(Color::Cyan).fg(Color::Black));
            }
        }

        match &self.0.state().sql_result {
            Some(SqlResult::Ok(data)) => {
                DataTable::new(data)
                    .with_title("Query result".to_string())
                    .with_horizontal_scroll(self.0.state().horizontal_offset())
                    .with_vertical_scroll(self.0.state().data_vertical_scroll())
                    .with_selected_row(Some(self.0.state().vertical_offset()))
                    .render(results_area, buf);
            }
            Some(SqlResult::Err(msg)) => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Red))
                    .title(" Error ");
                let inner = block.inner(results_area);
                block.render(results_area, buf);
                Paragraph::new(Line::from(Span::styled(
                    msg.as_str(),
                    Style::default().fg(Color::Red),
                )))
                .render(inner, buf);
            }
            None => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray));
                let inner = block.inner(results_area);
                block.render(results_area, buf);
                Paragraph::new(Line::from(Span::raw(
                    "Enter SQL and press Enter to run. Table name: parquet",
                )))
                .render(inner, buf);
            }
        }
    }

    fn render_row_detail_view(&self, area: Rect, buf: &mut Buffer) {
        let state = self.0.state();
        let row_idx = match state.row_detail_row {
            Some(i) => i,
            None => return,
        };
        let (lines, title) = match self.0.tabs().active_tab().to_string().as_str() {
            "Visualize" => {
                let data = state
                    .filtered_sample_data
                    .as_ref()
                    .unwrap_or(&self.0.parquet_ctx.sample_data);
                match data.rows.get(row_idx) {
                    Some(row) => (
                        Self::row_detail_lines(&data.flattened_columns, row),
                        format!("Row {} (Visualize)", row_idx + 1),
                    ),
                    None => (
                        vec![Line::from(Span::raw("Row out of range"))],
                        "Row detail".to_string(),
                    ),
                }
            }
            "SQL" => {
                if let Some(SqlResult::Ok(data)) = &state.sql_result {
                    match data.rows.get(row_idx) {
                        Some(row) => (
                            Self::row_detail_lines(&data.flattened_columns, row),
                            format!("Row {} (SQL result)", row_idx + 1),
                        ),
                        None => (
                            vec![Line::from(Span::raw("Row out of range"))],
                            "Row detail".to_string(),
                        ),
                    }
                } else {
                    (
                        vec![Line::from(Span::raw("No result data"))],
                        "Row detail".to_string(),
                    )
                }
            }
            _ => (
                vec![Line::from(Span::raw("No data"))],
                "Row detail".to_string(),
            ),
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .title(format!(
                " {} (Esc close, ↑↓ PgUp PgDn scroll, ←→ horizontal) ",
                title
            ));
        let inner = block.inner(area);
        block.render(area, buf);
        // Paragraph scroll is (vertical, horizontal)
        let max_vertical = lines
            .len()
            .saturating_sub(inner.height as usize)
            .max(0);
        let vertical_scroll = state.detail_scroll_offset.min(max_vertical) as u16;
        let max_line_width = lines.iter().map(|l| l.width()).max().unwrap_or(0) as usize;
        let max_horizontal = max_line_width.saturating_sub(inner.width as usize).max(0);
        let horizontal_scroll = state.detail_scroll_horizontal.min(max_horizontal) as u16;
        Paragraph::new(Text::from(lines))
            .scroll((vertical_scroll, horizontal_scroll))
            .render(inner, buf);
    }

    fn row_detail_lines(columns: &[String], row: &[String]) -> Vec<Line<'static>> {
        columns
            .iter()
            .zip(row.iter())
            .map(|(c, v)| Line::from(Span::raw(format!("{}: {}", c, v))))
            .collect()
    }
}

impl<'a> Widget for AppWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let app = self.0;

        let vertical = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        self.render_tabs_view(header_area, buf);
        self.render_footer_view(footer_area, buf);

        if app.state().row_detail_row.is_some() {
            self.render_row_detail_view(inner_area, buf);
        } else {
            match app.tabs().active_tab().to_string().as_str() {
                "Metadata" => self.render_metadata_view(inner_area, buf),
                "Schema" => self.render_schema_view(inner_area, buf),
                "Row Groups" => self.render_row_groups_view(inner_area, buf),
                "Visualize" => self.render_visualize_view(inner_area, buf),
                "SQL" => self.render_sql_view(inner_area, buf),
                _ => {}
            }
        }
    }
}
