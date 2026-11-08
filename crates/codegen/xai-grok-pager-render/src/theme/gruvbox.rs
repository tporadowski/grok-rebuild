//! Gruvbox dark, hard contrast.
//!
//! Hex values are the published Gruvbox dark palette (morhetz/gruvbox).
//! Hard contrast uses `dark0_hard` (`#1d2021`) as the canvas. Bright accents
//! are the dark-mode foregrounds; faded red and green are the diff bands.

use ratatui::style::{Color, Modifier};

use super::tokyonight::Theme;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

mod palette {
    use super::*;

    pub const BG0_H: Color = rgb(29, 32, 33); // #1d2021 dark0_hard
    pub const BG0: Color = rgb(40, 40, 40); // #282828 dark0
    pub const BG1: Color = rgb(60, 56, 54); // #3c3836 dark1
    pub const BG2: Color = rgb(80, 73, 69); // #504945 dark2
    pub const BG3: Color = rgb(102, 92, 84); // #665c54 dark3
    pub const BG4: Color = rgb(124, 111, 100); // #7c6f64 dark4
    pub const GRAY: Color = rgb(146, 131, 116); // #928374

    pub const FG0: Color = rgb(251, 241, 199); // #fbf1c7 light0
    pub const FG1: Color = rgb(235, 219, 178); // #ebdbb2 light1
    pub const FG2: Color = rgb(213, 196, 161); // #d5c4a1 light2
    pub const FG3: Color = rgb(189, 174, 147); // #bdae93 light3
    pub const FG4: Color = rgb(168, 153, 132); // #a89984 light4

    pub const RED: Color = rgb(251, 73, 52); // #fb4934
    pub const GREEN: Color = rgb(184, 187, 38); // #b8bb26
    pub const YELLOW: Color = rgb(250, 189, 47); // #fabd2f
    pub const BLUE: Color = rgb(131, 165, 152); // #83a598
    pub const PURPLE: Color = rgb(211, 134, 155); // #d3869b
    pub const AQUA: Color = rgb(142, 192, 124); // #8ec07c
    pub const ORANGE: Color = rgb(254, 128, 25); // #fe8019

    pub const FADED_RED: Color = rgb(157, 0, 6); // #9d0006
    pub const FADED_GREEN: Color = rgb(121, 116, 14); // #79740e
}
use palette::*;

impl Theme {
    pub const fn gruvbox_dark_hard() -> Self {
        Self {
            bg_base: BG0_H,
            bg_light: BG1,
            bg_dark: BG0,
            bg_highlight: BG1,
            bg_hover: BG2,
            bg_terminal: BG0_H,

            accent_user: ORANGE,
            accent_assistant: YELLOW,
            accent_thinking: BG4,
            accent_tool: GRAY,
            accent_system: AQUA,
            accent_error: RED,
            accent_success: GREEN,
            accent_running: AQUA,
            accent_skill: PURPLE,

            text_primary: FG1,
            text_secondary: FG2,

            gray_dim: BG4,
            gray: GRAY,
            gray_bright: FG3,

            command: YELLOW,
            path: ORANGE,
            running: AQUA,
            warning: YELLOW,

            fuzzy_accent: ORANGE,

            accent_plan: YELLOW,

            accent_verify: PURPLE,

            accent_remember: GREEN,

            selection_border: BG3,
            hover_border: BG2,
            prompt_border: BG3,
            prompt_border_active: FG4,

            accent_model: BLUE,

            // Thumb (gray, Σ393) stays ≥30 lighter than the bg1 track (Σ170) after follow-mode blends it.
            scrollbar_bg: BG1,
            scrollbar_fg: GRAY,

            diff_delete_bg: FADED_RED,
            diff_delete_fg: RED,
            diff_insert_bg: FADED_GREEN,
            diff_insert_fg: GREEN,
            diff_equal_fg: GRAY,
            diff_gutter_fg: BG4,

            bg_visual: BG2,

            paste_bg: BG0,
            paste_fg: FG2,
            paste_dim: GRAY,

            md_heading_h1: FG0,
            md_heading_h1_mod: Modifier::BOLD,
            md_heading_h2: ORANGE,
            md_heading_h2_mod: Modifier::BOLD,
            md_heading_h3: YELLOW,
            md_heading_h3_mod: Modifier::BOLD,
            md_heading_h4: GREEN,
            md_heading_h4_mod: Modifier::BOLD.union(Modifier::ITALIC),
            md_heading_h5: AQUA,
            md_heading_h5_mod: Modifier::BOLD,
            md_heading_h6: BLUE,
            md_heading_h6_mod: Modifier::BOLD,
            md_code: AQUA,
            md_task_checked: GREEN,
            md_task_unchecked: FG4,
            md_muted: GRAY,
            md_code_bg: BG0,
            md_text: FG1,
            link_fg: BLUE,
        }
    }
}
