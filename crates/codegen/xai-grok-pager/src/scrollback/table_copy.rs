//! Copy buttons under a rendered markdown table: TSV, CSV, and Markdown.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::scrollback::types::{BlockLine, BlockOutput, prewrap_index_per_row};
use crate::theme::Theme;
use xai_grok_markdown::TableCopyMeta;

/// Spreadsheet export a table's copy button writes to the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCopyFormat {
    Tsv,
    Csv,
    Markdown,
}

/// One button on the row under a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableCopyButton {
    pub label: &'static str,
    pub format: TableCopyFormat,
    /// Column where the label starts, in the row's content.
    pub col: u16,
}

const BUTTON_GAP: u16 = 2;

/// `[TSV]  [CSV]  [Markdown]`, left to right.
pub fn table_copy_buttons() -> [TableCopyButton; 3] {
    let specs = [
        ("[TSV]", TableCopyFormat::Tsv),
        ("[CSV]", TableCopyFormat::Csv),
        ("[Markdown]", TableCopyFormat::Markdown),
    ];
    let mut col = 0u16;
    specs.map(|(label, format)| {
        let button = TableCopyButton { label, format, col };
        col += UnicodeWidthStr::width(label) as u16 + BUTTON_GAP;
        button
    })
}

/// Which button covers `col`, when the row is `width` columns wide.
pub fn table_copy_format_at(col: u16, width: u16) -> Option<TableCopyFormat> {
    table_copy_buttons().into_iter().find_map(|button| {
        let label_width = UnicodeWidthStr::width(button.label) as u16;
        let end = button.col.saturating_add(label_width);
        (button.col < width && end <= width && col >= button.col && col < end)
            .then_some(button.format)
    })
}

/// Whole-table export. `cells` is row-major. A short last row is kept.
pub fn export_table<T: AsRef<str>>(cells: &[T], n_cols: usize, format: TableCopyFormat) -> String {
    if n_cols == 0 || cells.is_empty() {
        return String::new();
    }
    let rows: Vec<&[T]> = cells.chunks(n_cols).collect();
    match format {
        TableCopyFormat::Tsv => rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| tsv_field(cell.as_ref()))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        TableCopyFormat::Csv => rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| csv_field(cell.as_ref()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        TableCopyFormat::Markdown => markdown_table(&rows),
    }
}

fn tsv_field(cell: &str) -> String {
    cell.chars()
        .map(|ch| match ch {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

fn csv_field(cell: &str) -> String {
    let quote = cell.contains([',', '"', '\n', '\r']);
    if !quote {
        return cell.to_string();
    }
    let mut out = String::from("\"");
    for ch in cell.chars() {
        if ch == '"' {
            out.push_str("\"\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

fn markdown_field(cell: &str) -> String {
    let mut out = String::new();
    for ch in cell.chars() {
        match ch {
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

fn markdown_table<T: AsRef<str>>(rows: &[&[T]]) -> String {
    let Some(header) = rows.first() else {
        return String::new();
    };
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(markdown_row(header));
    let sep = header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ");
    lines.push(format!("| {sep} |"));
    for row in rows.iter().skip(1) {
        lines.push(markdown_row(row));
    }
    lines.join("\n")
}

fn markdown_row<T: AsRef<str>>(row: &[T]) -> String {
    let body = row
        .iter()
        .map(|cell| markdown_field(cell.as_ref()))
        .collect::<Vec<_>>()
        .join(" | ");
    format!("| {body} |")
}

/// The button row under one table, with the cells the buttons export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCopyAffordance {
    /// Index of the button row in the block's `output()` lines.
    pub row_offset: u16,
    pub cells: Vec<String>,
    pub n_cols: usize,
}

impl TableCopyAffordance {
    pub fn clipboard_text(&self, format: TableCopyFormat) -> String {
        export_table(&self.cells, self.n_cols, format)
    }
}

fn button_line() -> BlockLine {
    let theme = Theme::current();
    let style = Style::default().fg(theme.gray);
    let mut spans = Vec::new();
    for (index, button) in table_copy_buttons().into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ".to_string(), style));
        }
        spans.push(Span::styled(button.label.to_string(), style));
    }
    BlockLine::separator(Line::from(spans)).with_joiner(Some(String::new()))
}

/// Insert a non-selectable `[TSV] [CSV] [Markdown]` row after each table.
/// The row is not part of the message copy. Returns one affordance per inserted row, in document order.
pub fn apply_table_copy_rows(
    output: &mut BlockOutput,
    tables: &[TableCopyMeta],
) -> Vec<TableCopyAffordance> {
    if tables.is_empty() || output.lines.is_empty() {
        return Vec::new();
    }
    let prewrap = prewrap_index_per_row(&output.lines);
    let mut inserts: Vec<(usize, Vec<String>, usize)> = Vec::new();
    for table in tables {
        if table.n_cols == 0 || table.cells.is_empty() {
            continue;
        }
        let end = table
            .line_index
            .saturating_add(table.line_count.saturating_sub(1));
        let Some(last_row) = prewrap.iter().rposition(|&index| index == end) else {
            continue;
        };
        let cells = table
            .cells
            .iter()
            .map(|cell| cell.text.clone())
            .collect::<Vec<_>>();
        inserts.push((last_row + 1, cells, table.n_cols));
    }
    let affordances = inserts
        .iter()
        .enumerate()
        .map(|(index, (at, cells, n_cols))| TableCopyAffordance {
            row_offset: at.saturating_add(index) as u16,
            cells: cells.clone(),
            n_cols: *n_cols,
        })
        .collect();
    for (at, _, _) in inserts.iter().rev() {
        output.lines.insert(*at, button_line());
    }
    affordances
}

/// A visible table-copy row and the cells its buttons export.
#[derive(Debug, Clone)]
pub struct TableCopyPlacement {
    pub screen_rect: ratatui::layout::Rect,
    pub cells: Vec<String>,
    pub n_cols: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_copy_buttons_export_tsv_csv_and_markdown() {
        let cells = [
            "Name",
            "Note",
            "Ada",
            "a, b",
            "Lin",
            "say \"hi\"",
            "tab\there",
            "line\nbreak",
            "a|b",
            "plain",
        ];
        let tsv = export_table(&cells, 2, TableCopyFormat::Tsv);
        assert_eq!(
            tsv,
            "Name\tNote\nAda\ta, b\nLin\tsay \"hi\"\ntab here\tline break\na|b\tplain"
        );
        let csv = export_table(&cells, 2, TableCopyFormat::Csv);
        assert_eq!(
            csv,
            "Name,Note\nAda,\"a, b\"\nLin,\"say \"\"hi\"\"\"\ntab\there,\"line\nbreak\"\na|b,plain"
        );
        let markdown = export_table(&cells, 2, TableCopyFormat::Markdown);
        assert_eq!(
            markdown,
            "\
| Name | Note |
| --- | --- |
| Ada | a, b |
| Lin | say \"hi\" |
| tab\there | line break |
| a\\|b | plain |"
        );
        assert!(export_table(&[] as &[&str], 2, TableCopyFormat::Tsv).is_empty());
        assert!(export_table(&["x"], 0, TableCopyFormat::Csv).is_empty());

        assert_eq!(table_copy_format_at(0, 40), Some(TableCopyFormat::Tsv));
        assert_eq!(table_copy_format_at(4, 40), Some(TableCopyFormat::Tsv));
        assert_eq!(table_copy_format_at(5, 40), None, "the gap is not a button");
        assert_eq!(table_copy_format_at(7, 40), Some(TableCopyFormat::Csv));
        assert_eq!(
            table_copy_format_at(14, 40),
            Some(TableCopyFormat::Markdown)
        );
        assert_eq!(
            table_copy_format_at(14, 10),
            None,
            "a button that does not fit is not clickable"
        );
    }
}
