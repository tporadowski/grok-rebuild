//! File-search and prompt-history panels on a terminal too short for their full height.
use super::{AgentView, AppRenderParams, BannerSlotParams, test_fixtures};
use crate::actions::ActionRegistry;
use crate::scrollback::render::ScratchBuffer;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn draw(agent: &mut AgentView, height: u16) {
    let area = Rect::new(0, 0, 80, height);
    agent.last_terminal_size = (80, height);
    let mut buf = Buffer::empty(area);
    let mut scratch = ScratchBuffer::new();
    let registry = ActionRegistry::defaults();
    agent.draw(
        area,
        &mut buf,
        &registry,
        &mut scratch,
        None,
        false,
        BannerSlotParams::none(),
        false,
        &mut Vec::new(),
        AppRenderParams::default(),
    );
}

#[test]
fn file_search_panel_survives_a_very_short_terminal() {
    let mut agent = test_fixtures::make_agent();
    agent.prompt.set_text("@src");
    let ctx = crate::views::file_search::context::detect("@src", 4).expect("context");
    let hits = (0..8)
        .map(|i| xai_grok_workspace::file_system::FuzzyMatchResult {
            path: nucleo::Utf32String::from(format!("src/file{i}.rs")),
            score: 1,
            indices: Vec::new(),
            is_dir: false,
        })
        .collect();
    agent.prompt.file_search.set_test_state(ctx, hits, 0);
    assert!(agent.prompt.file_search_visible());
    draw(&mut agent, 6);
}

#[test]
fn history_panel_survives_a_very_short_terminal() {
    let mut agent = test_fixtures::make_agent();
    let prompts = [
        "alpha one",
        "beta two",
        "gamma three",
        "delta four",
        "epsilon five",
        "zeta six",
        "eta seven",
        "theta eight",
    ];
    agent.prompt.history_search.set_matches_for_test(&prompts);
    assert!(agent.prompt.history_search.is_active());
    assert_eq!(agent.prompt.history_search.result_count(), 8);
    draw(&mut agent, 6);
}
