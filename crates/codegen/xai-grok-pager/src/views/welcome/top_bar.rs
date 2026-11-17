//! Top bar component: renders cwd and git info.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use std::path::{Path, PathBuf};

use unicode_width::UnicodeWidthStr;

use crate::git_info;
use crate::render::line_utils::shorten_path;
use crate::theme::Theme;
use crate::views::location::{location_parts, worktree_badge, LocationParts};

pub fn render_top_bar(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    announcement: Option<&xai_grok_announcements::RemoteAnnouncement>,
) {
    let line = welcome_location_line(&process_cwd(), area.width as usize, theme);
    let line_width = line.width() as u16;
    buf.set_line(area.x, area.y, &line, line_width.min(area.width));

    if let Some(a) = announcement
        && let Some(text) = a.message.as_deref()
        && area.height > 1
    {
        let text_style = Style::default().fg(theme.text_primary);
        let line = Line::from(Span::styled(text, text_style));
        Paragraph::new(line).render(
            Rect {
                y: area.y + 1,
                height: area.height.saturating_sub(1),
                ..area
            },
            buf,
        );
    }
}

/// `{git branch} {worktree} {cwd}` for the welcome top bar.
/// A narrow row shortens the branch before it touches the folder name.
pub(crate) fn welcome_location_line(cwd: &Path, max_width: usize, theme: &Theme) -> Line<'static> {
    fit_welcome_location(&location_parts(cwd), max_width, theme)
}

fn fit_welcome_location(parts: &LocationParts, max_width: usize, theme: &Theme) -> Line<'static> {
    if max_width == 0 {
        return Line::from(Vec::new());
    }
    let info_style = Style::default().fg(theme.gray);
    let cwd_style = Style::default().fg(theme.gray_dim);
    let badge_w = if parts.is_worktree {
        "worktree ".width()
    } else {
        0
    };
    let cwd_w = parts.cwd_display.width();
    // The folder stays whole. The worktree badge stays only when that still fits.
    let keep_badge = parts.is_worktree && badge_w + cwd_w <= max_width;
    let reserved = if keep_badge { badge_w } else { 0 };
    let cwd_budget = max_width.saturating_sub(reserved);
    let cwd_text = if cwd_w <= cwd_budget {
        parts.cwd_display.clone()
    } else {
        shorten_path(&parts.cwd_display, cwd_budget)
    };
    let tail_w = cwd_text.width() + reserved;
    let room = max_width.saturating_sub(tail_w);
    let branch = parts
        .branch
        .as_deref()
        .filter(|branch| !branch.is_empty())
        .and_then(|branch| fitted_branch_span(branch, room));

    let mut spans: Vec<Span> = Vec::new();
    if let Some(text) = branch {
        let git_style = Style::default()
            .fg(theme.text_primary)
            .add_modifier(Modifier::DIM);
        spans.push(Span::styled(text, git_style));
        spans.push(Span::styled(" ", info_style));
    }
    if keep_badge {
        spans.push(worktree_badge(theme));
    }
    spans.push(Span::styled(cwd_text, cwd_style));
    Line::from(spans)
}

/// `{icon} {branch}` plus the separating space must fit in `room`.
/// The branch is the piece that loses columns.
fn fitted_branch_span(branch: &str, room: usize) -> Option<String> {
    let gap = 1usize;
    let span_budget = room.saturating_sub(gap);
    if span_budget == 0 {
        return None;
    }
    let icon = git_info::branch_icon();
    let prefix = format!("{icon} ");
    let prefix_w = prefix.width();
    if span_budget <= prefix_w {
        return None;
    }
    let full = format!("{prefix}{branch}");
    let text = if full.width() <= span_budget {
        full
    } else {
        let shortened = crate::util::truncate_to_width(branch, span_budget - prefix_w);
        if shortened.is_empty() {
            return None;
        }
        format!("{prefix}{shortened}")
    };
    (text.width() + gap <= room).then_some(text)
}

fn process_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|span| span.content.as_ref()).collect()
    }

    fn row_text(buf: &Buffer, width: u16) -> String {
        (0..width)
            .filter_map(|x| buf.cell((x, 0)).map(|cell| cell.symbol().to_string()))
            .collect()
    }

    #[test]
    #[serial_test::serial]
    fn narrow_welcome_bar_shortens_the_branch_before_the_folder() {
        let _guard = crate::theme::cache::pin_theme();
        crate::theme::cache::set_terminal_native_lock(false);
        let theme = Theme::current();
        let folder = "quarterly-reports";
        let cwd_display = format!("/clients/acme/{folder}");
        let branch = "feature/quarterly-planning-review";
        let parts = LocationParts {
            branch: Some(branch.to_owned()),
            is_worktree: false,
            cwd_display: cwd_display.clone(),
        };

        let fitted = fit_welcome_location(&parts, cwd_display.width() + 4, &theme);
        let text = line_text(&fitted);
        assert!(text.contains(folder), "{text}");
        assert!(text.contains('\u{2026}'), "the branch shortens, got {text}");
        assert!(!text.contains(branch), "the full branch stays off the row, got {text}");
        assert!(fitted.width() <= cwd_display.width() + 4, "{text}");

        let wide = fit_welcome_location(&parts, 200, &theme);
        let wide_text = line_text(&wide);
        assert!(wide_text.contains(branch), "{wide_text}");
        assert!(wide_text.contains(folder), "{wide_text}");

        let folder_only = fit_welcome_location(&parts, cwd_display.width(), &theme);
        let folder_text = line_text(&folder_only);
        assert_eq!(folder_text, cwd_display);
        assert!(!folder_text.contains("feature"), "{folder_text}");

        let cwd = process_cwd();
        let full_branch = "feature/welcome-bar-branch-that-must-shrink";
        struct RestoreCwdGit(PathBuf);
        impl Drop for RestoreCwdGit {
            fn drop(&mut self) {
                let restored = git_info::compute_cwd_git_info(&self.0);
                git_info::set_cwd_git_info_for_test(&self.0, restored);
            }
        }
        let _restore = RestoreCwdGit(cwd.clone());
        git_info::set_cwd_git_info_for_test(
            &cwd,
            Some(git_info::CwdGitInfo {
                branch: Some(full_branch.to_owned()),
                is_worktree: false,
                main_repo: None,
                worktree_label: None,
            }),
        );
        let shown = crate::util::display_location_path(&cwd);
        let leaf = shown.rsplit(['/', '\\']).next().unwrap_or(&shown);
        let width = (shown.width() + 4) as u16;
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(area, &mut buf, &theme, None);
        let painted = row_text(&buf, width);
        assert!(
            painted.contains(leaf),
            "the folder stays on the welcome bar, got {painted:?}"
        );
        assert!(
            painted.contains('\u{2026}'),
            "the welcome bar shortens the branch, got {painted:?}"
        );
        assert!(
            !painted.contains(full_branch),
            "the full branch is not painted, got {painted:?}"
        );
    }
}
