//! `/docs` opens How-to Guides (in-TUI) or the online Build docs.
//!
//! Bare `/docs` opens the same DocPicker as command-palette "How-to Guides".
//! `/docs web` opens https://docs.x.ai/build/overview in the browser.
//! `/docs <title>` opens a single guide by title (case-insensitive).

use crate::app::actions::Action;
use crate::docs::{all_titles, find_doc};
use crate::slash::command::{
    AppCtx, ArgItem, CommandExecCtx, CommandResult, SlashCommand, slash_meta,
};

/// The online Build docs landing page, hardcoded like other TUI deep-links; docs.x.ai can redirect if the path moves.
pub const BUILD_DOCS_URL: &str = "https://docs.x.ai/build/overview";

pub struct DocsCommand;

impl SlashCommand for DocsCommand {
    slash_meta! {
        name: "docs",
        aliases: ["howto", "guides"],
        description: "Open How-to Guides or online Build docs",
        usage: "/docs [web|title]",
        takes_args: true,
        args_required: false,
        arg_placeholder: "[web|title]",
    }

    fn suggest_args(&self, _ctx: &AppCtx, args_query: &str) -> Option<Vec<ArgItem>> {
        // An exact guide title or picker name is the only row, so Enter keeps
        // what was typed instead of a nearby fuzzy highlight.
        if let Some(exact) = exact_docs_arg(args_query) {
            return Some(vec![exact]);
        }
        let mut items = vec![howto_arg(), web_arg()];
        items.extend(all_titles().map(title_arg));
        Some(items)
    }

    fn preselected_arg(&self, _ctx: &AppCtx, args_query: &str) -> Option<String> {
        exact_docs_arg(args_query).map(|item| item.insert_text)
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() || is_howto_list_arg(trimmed) {
            return CommandResult::Action(Action::OpenHowtoGuides);
        }
        if is_web_arg(trimmed) {
            return CommandResult::Action(Action::OpenUrl(BUILD_DOCS_URL.into()));
        }
        match find_doc(trimmed) {
            Some(doc) => CommandResult::Action(Action::ShowReleaseNotes {
                title: doc.title.into(),
                content: doc.content.into(),
            }),
            None => CommandResult::Error(format!(
                "Unknown docs target {trimmed:?}. Try /docs, /docs web, or a guide title (e.g. /docs Getting Started)."
            )),
        }
    }
}

fn is_howto_list_arg(arg: &str) -> bool {
    matches!(
        arg.to_ascii_lowercase().as_str(),
        "how-to" | "howto" | "guides" | "guide" | "list" | "tui"
    )
}

fn is_web_arg(arg: &str) -> bool {
    matches!(
        arg.to_ascii_lowercase().as_str(),
        "web" | "online" | "browser" | "site" | "www"
    )
}

fn howto_arg() -> ArgItem {
    ArgItem {
        display: "how-to".into(),
        match_text: "how-to".into(),
        insert_text: "how-to".into(),
        description: "Browse in-TUI How-to Guides".into(),
    }
}

fn web_arg() -> ArgItem {
    ArgItem {
        display: "web".into(),
        match_text: "web".into(),
        insert_text: "web".into(),
        description: "Open docs.x.ai/build in the browser".into(),
    }
}

fn title_arg(title: &str) -> ArgItem {
    ArgItem {
        display: title.into(),
        match_text: title.into(),
        insert_text: title.into(),
        description: format!("Open \"{title}\""),
    }
}

/// The guide or picker named by `query`, when the name is exact.
fn exact_docs_arg(query: &str) -> Option<ArgItem> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(doc) = find_doc(trimmed) {
        return Some(title_arg(doc.title));
    }
    if is_howto_list_arg(trimmed) {
        return Some(howto_arg());
    }
    if is_web_arg(trimmed) {
        return Some(web_arg());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;

    static DEFAULT_BUNDLE_STATE: crate::app::bundle::BundleState =
        crate::app::bundle::BundleState {
            has_cache: false,
            version: String::new(),
            personas: Vec::new(),
            roles: Vec::new(),
            agents: Vec::new(),
            skills: Vec::new(),
            persona_details: Vec::new(),
            role_details: Vec::new(),
        };

    fn make_ctx<'a>(models: &'a ModelState) -> CommandExecCtx<'a> {
        CommandExecCtx {
            models,
            session_id: None,
            bundle_state: &DEFAULT_BUNDLE_STATE,
            screen_mode: crate::app::ScreenMode::Inline,
            billing_surface_visible: true,
            usage_command_visible: true,
            pager_state: crate::settings::PagerLocalSnapshot {
                multiline_mode: false,
                yolo_mode: false,
                ..crate::settings::PagerLocalSnapshot::default()
            },
        }
    }

    #[test]
    fn bare_docs_opens_howto_guides() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        assert!(matches!(
            DocsCommand.run(&mut ctx, ""),
            CommandResult::Action(Action::OpenHowtoGuides)
        ));
    }

    #[test]
    fn howto_aliases_open_list() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        for args in ["how-to", "howto", "guides", "list", "tui"] {
            assert!(
                matches!(
                    DocsCommand.run(&mut ctx, args),
                    CommandResult::Action(Action::OpenHowtoGuides)
                ),
                "args={args:?}"
            );
        }
    }

    #[test]
    fn web_opens_build_docs_url() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        for args in ["web", "online", "browser"] {
            match DocsCommand.run(&mut ctx, args) {
                CommandResult::Action(Action::OpenUrl(url)) => {
                    assert_eq!(url, BUILD_DOCS_URL, "args={args:?}");
                }
                other => panic!("expected OpenUrl for args={args:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn title_opens_guide() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        match DocsCommand.run(&mut ctx, "Getting Started") {
            CommandResult::Action(Action::ShowReleaseNotes { title, content }) => {
                assert_eq!(title, "Getting Started");
                assert!(!content.is_empty());
            }
            other => panic!("expected ShowReleaseNotes, got {other:?}"),
        }
    }

    #[test]
    fn unknown_target_errors() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        assert!(matches!(
            DocsCommand.run(&mut ctx, "not-a-real-guide"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn aliases_and_metadata() {
        let cmd = DocsCommand;
        assert_eq!(cmd.name(), "docs");
        assert_eq!(cmd.aliases(), &["howto", "guides"]);
        assert!(cmd.takes_args());
        assert!(!cmd.args_required());
    }

    fn app_ctx(models: &ModelState) -> AppCtx<'_> {
        AppCtx {
            models,
            cwd: std::path::Path::new("."),
            has_session_announcements: false,
            billing_surface_visible: true,
            usage_command_visible: true,
            workflows_available: true,
            saved_workflows: &[],
            workflow_runs: &[],
            screen_mode: crate::app::ScreenMode::Fullscreen,
            current_title: None,
        }
    }

    #[test]
    fn typed_docs_target_is_the_exact_guide_or_picker() {
        let models = ModelState::default();
        let ctx = app_ctx(&models);
        let mut exec = make_ctx(&models);

        let guide = DocsCommand
            .suggest_args(&ctx, "getting started")
            .expect("guide");
        assert_eq!(guide.len(), 1);
        assert_eq!(guide.first().map(|item| item.insert_text.as_str()), Some("Getting Started"));
        assert_eq!(
            DocsCommand.preselected_arg(&ctx, "getting started").as_deref(),
            Some("Getting Started")
        );
        match DocsCommand.run(&mut exec, "Getting Started") {
            CommandResult::Action(Action::ShowReleaseNotes { title, content }) => {
                assert_eq!(title, "Getting Started");
                assert!(!content.is_empty());
            }
            other => panic!("expected the Getting Started guide, got {other:?}"),
        }

        let picker = DocsCommand.suggest_args(&ctx, "Guides").expect("picker");
        assert_eq!(picker.len(), 1);
        assert_eq!(picker.first().map(|item| item.insert_text.as_str()), Some("how-to"));
        assert!(matches!(
            DocsCommand.run(&mut exec, "how-to"),
            CommandResult::Action(Action::OpenHowtoGuides)
        ));

        let web = DocsCommand.suggest_args(&ctx, "online").expect("web");
        assert_eq!(web.len(), 1);
        assert_eq!(web.first().map(|item| item.insert_text.as_str()), Some("web"));
        match DocsCommand.run(&mut exec, "web") {
            CommandResult::Action(Action::OpenUrl(url)) => assert_eq!(url, BUILD_DOCS_URL),
            other => panic!("expected the docs site, got {other:?}"),
        }

        let partial = DocsCommand.suggest_args(&ctx, "get").expect("partial");
        assert!(
            partial.len() > 1,
            "a prefix still offers the rest of the list"
        );
    }

    #[test]
    fn suggest_args_includes_web_and_titles() {
        let models = ModelState::default();
        let cwd = std::path::Path::new(".");
        let ctx = AppCtx {
            models: &models,
            cwd,
            has_session_announcements: false,
            billing_surface_visible: true,
            usage_command_visible: true,
            workflows_available: true,
            saved_workflows: &[],
            workflow_runs: &[],
            screen_mode: crate::app::ScreenMode::Fullscreen,
            current_title: None,
        };
        let items = DocsCommand.suggest_args(&ctx, "").expect("suggestions");
        assert!(items.iter().any(|i| i.insert_text == "web"));
        assert!(items.iter().any(|i| i.insert_text == "how-to"));
        assert!(items.iter().any(|i| i.insert_text == "Getting Started"));
    }
}
