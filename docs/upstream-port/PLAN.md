# Upstream port plan: 1.0.46 through 1.0.78

Living tracker for bringing this fork from package version **1.0.45** up to the behavior described in the official release notes through alpha **1.0.78** (6 November 2026). Stable that day was 1.0.77. Public GitHub `xai-org/grok-build` stopped at the 29 September 2026 sync.

This approval **adds this document to the repo**. It does not start the port. A later session executes it when asked to run the upstream port plan.

Repo path after approval: `docs/upstream-port/PLAN.md`. This file is the only progress tracker and the only decision log. Do not open a second plan.

## How a later session runs this

1. Read this file from the top through the decision log. Trust decisions already recorded. Do not reopen them.
2. Work waves in this order: W1, W2, W3, W6, W4, W5, W7, W8, W9, W10, W11, W12, W13, W14, W15, then W16.
3. One wave per session. Inside W14 and W15, stop at the next version boundary after the version you started, so a session stays the size of one release's leftovers. Do not start the next wave in the same session.
4. Take the next `pending` item in that wave, in the catalog order below.
5. Search this tree before writing code. Set the item status using the policy in the next section. Update the status word in the catalog in the same change as the code.
6. Append a decision whenever the note does not map cleanly onto the code, whenever a fork behavior is touched, and whenever an item is `deferred` or `already`. Skip the log only for a straightforward implementation that matches the note.
7. Commit the code and the tracker together. Subject: `port: <id> <few words>`. One commit per item, or one commit for a tight cluster in the same function. Do not commit official binaries, `/tmp/grok-rea`, or a version bump.
8. Test the crate you changed: `cargo test -p <crate> -- <filter>`. Cargo takes one filter before `--`. Extra names go after `--`. Run from the crate that owns the test.
9. This is a terminal app. Do not use a browser to verify it. Use unit tests, and a pty scenario in `xai-grok-pager-pty-harness` only when the note is about keys, focus, or layout and there is no unit seam.
10. At the end of the session, refresh the summary counts, set **Next**, and write what the next session should pick up. Stop there.

Do not ask the user about a choice this policy already covers. Stop the session only when a change would remove a protected fork behavior or when continuing would invent a network protocol. Record `deferred` and move to the next item in those cases; do not block the wave.

If alpha has moved past 1.0.78, fetch `https://x.ai/cli/changelogs/<version>.external.md` for each new version and append new catalog rows. Do not rewrite rows that are already `done`, `already`, or `skipped`. A later note that reverses a `done` item becomes a new row, and the old row stays `done` with a decision pointing at the new row.

## Status words

| Status | Meaning |
|---|---|
| `pending` | Not started. |
| `in_progress` | This session is in the middle of it. Clear this before ending the session. |
| `done` | Implemented and tested in this tree. |
| `already` | The final behavior is already in this tree. Cite the file in the decision log. Do not rewrite it. |
| `skipped` | Do not implement. The reason is the flag and the decision log. |
| `deferred` | Real work, but this tree cannot do it faithfully. Say what is missing. |

## Decisions you take while executing

Search first. Then pick one outcome. Record it.

- **Already.** The final behavior from the latest note is present. Mark `already`. Do not rebuild an earlier version of it.
- **Partial.** An earlier form is present. Change it to the final note. The catalog row is that final behavior, not the intermediate one.
- **Absent.** Implement it in the module that already owns the nearest behavior. Match the surrounding style. No new crate, no new framework.
- **Cannot find it.** If a bug note names a symptom and the code path does not exist, mark `deferred` with what you searched. Do not invent a bug to fix. Feature notes still get implemented when the subsystem exists.
- **Unspecified.** `Minor bug fixes` and any note that does not describe an observable behavior stay `skipped`.
- **Superseded.** A later note replaced it. The skip list below is closed. If you discover another reversal, mark the earlier row `skipped`, point `covered` at the later id, and record a decision. Implement the later row only.
- **External.** Plan-usage APIs, Cursor cloud pool wire protocol, and any other service this tree does not already speak are `deferred`. Do not invent the protocol. If the client code for that call already exists, wire the note to it.
- **Fork conflict.** The protected list below wins over a note that would delete it. Adapt the upstream behavior around it and record the choice. The known compatible case is folder trust: trusted plugin hooks still register at session start (this fork), and an untrusted folder still runs only managed hooks (1.0.64).
- **Windows or macOS only.** Implement shared code. Run the Linux tests. Record that the platform itself was not verified.
- **Two notes disagree.** The higher version wins. Both ids go in the decision.
- **Wrong wave.** Fix the wave field and record it. Do not force the change into the wrong subsystem.

Never decompile the official `grok` binary and never paste recovered pseudocode into this repo. The notes and this source are the inputs.

## Decisions already made

Append new entries as `D-019`, `D-020`, and so on. Do not renumber.

### D-126 — A hidden dock gives the queue back to the queue pane

Ctrl+G hides the dock but leaves it enabled. The queued prompts then show in the queue pane, and Ctrl+; toggles that pane. An enabled dock that is not hidden still owns the queue: Ctrl+; expands its Queued section, and an empty dock does not open the pane (`queue_pane_works_when_ctrl_g_hides_the_dock`, `toggle_queue_does_not_open_hidden_pane_when_dock_on_but_empty`) (`v50-x06`).

### D-125 — Command palette and model picker use the settings modal width

Ctrl+P and the model picker (`/model` and `/m`) open at the settings modal's width, 70% of the terminal up to 110 columns. Other arg pickers, including the theme picker, stay on the narrower picker width (`command_palette_and_model_picker_match_settings_width`) (`v50-f09`).

### D-124 — Compact mode hides the dock the same way Ctrl+G does

While compact mode is on, a painted dock uses the same sticky `dock_hidden` flag Ctrl+G sets, and an open tasks pane is closed. Ctrl+G shows that surface again, and later frames leave it shown. Turning compact off keeps whatever Ctrl+G last did. Turning it back on hides again. Minimal mode does not hide the dock, because Ctrl+G there opens the external editor (`compact_mode_hides_the_dock_like_ctrl_g`, `compact_mode_hides_the_tasks_pane_like_ctrl_g`, `compact_mode_hides_nothing_in_minimal_when_ctrl_g_is_the_editor`) (`v50-f05`).

### D-123 — `/resume` rows fold like rows inside a session

`/resume` uses the session transcript fold keys. `h` and Left collapse the selected row, `l` and Right expand it, `e` toggles that row, and `E` expands every row when any is collapsed and collapses every row when all are open. `h` and `l` do not start a search (`resume_rows_fold_with_the_session_keys`, `resume_list_folds_like_a_session_row`) (`v77-x05`).

### D-122 — `grok --resume <id>` opens the recorded folder

`materialize_startup_for_cwd` resolves an explicit resume id that is not under the launch directory through `resolve_local_session_any_cwd` and returns that recorded folder as `original_cwd`. Interactive startup loads the session there, and headless open uses the same folder. The launch directory does not refuse the resume (`resume_id_from_another_directory_opens_the_recorded_folder`) (`v75-x04`).

### D-121 — An unrecognized effort or unreadable history still resumes

A stored `reasoning_effort` that is not a known level does not fail `summary.json`. The session loads with that effort unset, so restore keeps the model's default, and the client gets one message: unrecognized effort, using the model's default. A later unknown effort in the same history does not add another sentence. Lines that are not valid UTF-8 are left out of the chat transcript and of `updates.jsonl`. A readable history line that names an unknown effort stays, with that field cleared (`unrecognized_effort_or_unreadable_history_still_loads`, `unrecognized_effort_notice_is_one_session_message`) (`v74-x23`).

### D-120 — A failed resume names the load error, and JSON prints it once

`grok -p --resume` (and continue or fork) reports the session/load error, and uses "Session does not exist" only when that error is empty. With `--output-format json`, a failure before the session opens writes one `{"type":"error","message":...}` object and the process does not print that failure again (`resume_load_failure_reports_the_reason_as_one_json_error`) (`v74-x22`).

### D-119 — A resumed background command finishes in the turn that ran it

Replayed `task_completed` uses the snapshot's start and end, and the completion row is inserted at the end of the turn that contains the started row. A live completion still appends where it arrives (`replayed_completion_stays_in_the_turn_that_ran_it`) (`v74-x04`).

### D-118 — Start in multiline is a saved default for new composers

`[ui] multiline_default` is the `/settings` row Start in multiline. It is off unless set. A new session and each opening of the dashboard start in that mode, so Enter inserts a newline there. The live Multiline toggle still changes only the composer that is open (`multiline_default_starts_session_and_dashboard_where_enter_inserts_newline`, `set_multiline_default_persists_and_rolls_back`) (`v73-f06`).

### D-117 — A folder pick updates the dispatch border before the process directory moves

`Ctrl+L` (Enter or a candidate click) and `/cd <path>` both dispatch `DashboardChangeLocation`. `dispatch_dashboard_change_location` assigns `dashboard.cwd` in the same step as `app.cwd`, then returns `Effect::SetWorkingDir`. The dispatch top border paints `location_parts(&state.cwd)` on the next frame, so the new folder is visible even when `set_current_dir` has not run (`dashboard_change_location_valid_updates_cwd_and_closes_modal`, `dispatch_top_border_shows_branch_and_folder`) (`v72-x04`).

### D-116 — The dashboard location sits on the dispatch box top border

The branch and folder are painted on the dispatch box top border, and that label is the location click target. The header row keeps the state chips and no longer shows the folder (`dispatch_top_border_shows_branch_and_folder`, `render_header_does_not_paint_the_location`) (`v69-f02`).

### D-115 — Directory grouping shows the folder path and agent count

With directory grouping, each folder's agents sit under a header of that folder's path and how many agents are in it. A pinned agent stays in the pinned block and is not counted again in the folder (`directory_grouping_header_shows_folder_path_and_agent_count`) (`v69-f01`).

### D-114 — Update cleanup keeps a binary a live session is executing

Download cleanup removes old versioned binaries but skips one a running process is still executing, including a hard link to that inode. A running session keeps its binary when a newer install is cleaned up (`cleanup_keeps_an_older_binary_marked_in_use`, `executable_is_in_use_detects_this_test_process`, `executable_is_in_use_matches_hardlink`) (`v68-x02`).

### D-113 — `/theme` restores the previous theme unless Enter confirms

Highlighting a theme previews it. Esc, Ctrl+U, Tab then Esc, opening Settings, and opening the dashboard put the previous theme back. Enter keeps the preview (`theme_picker_esc_restores_the_previous_theme`, `theme_picker_ctrl_u_restores_the_previous_theme`, `theme_picker_tab_then_esc_restores_the_previous_theme`, `theme_picker_enter_keeps_the_preview`, `theme_picker_settings_restores_the_previous_theme`, `theme_picker_dashboard_restores_the_previous_theme`) (`v66-x01`).

### D-112 — `/resume` search includes other folders, one-edit titles, and session text

A `/resume` query keeps a matching session from another folder and sorts the current folder first. A word of at least four characters may miss a title word by one edit. Shorter words stay prefixes, so `log` does not match `Blog`. The search index includes shell commands, assistant replies, and tool file paths (`resume_search_tolerates_one_edit`, `resume_search_keeps_a_typo_match_from_another_folder`, `resume_search_indexes_commands_paths_and_replies`) (`v65-f01`).

### D-111 — `/resume` loads the next page at the last row

Reaching the last loaded `/resume` row requests the next `x.ai/session/list` page with the stored `nextCursor`. That page is appended, and the picker shows how many sessions are loaded. An empty continuation ends the list (`resume_older_sessions_load_at_the_last_row`, `resume_older_sessions_fetch_keeps_next_cursor`, `resume_older_sessions_label_is_painted`) (`v64-f03`).

### D-110 — A cold `/resume` keeps the dashboard overlay

A `/resume` that loads a session while a dashboard overlay is open moves that overlay onto the loaded session. Left and Ctrl+\ still back out. A load that did not start on the overlay does not attach (`cold_resume_from_the_dashboard_overlay_stays_attached`) (`v62-x02`).

### D-109 — Dashboard peek transcript drag copies on release

Dragging across the dashboard peek transcript selects that span and copies it when the button is released. A click without a drag does not copy. A drag that starts on the reply row stays with the reply box (`peek_text_select_drag_copies_on_release`, `peek_text_select_click_does_not_copy`) (`v61-x01`).

### D-108 — A unique `/resume` match opens that session

`/resume <id or title>` loads the session when exactly one id or separator-insensitive title matches. Two exact titles, a word prefix, and a bare `/resume` still open the picker (`resume_unique_title_opens_the_session`, `resume_unique_id_opens_the_session`, `resume_ambiguous_or_empty_query_opens_the_picker`) (`v61-f02`).

### D-107 — Esc on the first dashboard question parks on the scrollback

Esc on the first question of a session opened from the dashboard moves focus to the scrollback and leaves the question pending, the same as a standalone session. The next Esc from that scrollback still leaves the overlay (`esc_on_the_first_dashboard_question_focuses_the_scrollback`, `overlay_esc_exits_when_question_nav_unselected`) (`v60-x02`).

### D-106 — Bare dashboard `/loop` shows usage

A `/loop` with no prompt on the dashboard shows the usage text and does not start a session. A `/loop` that includes a prompt still starts one (`dashboard_loop_without_a_prompt_shows_usage`) (`v59-f01`).

### D-105 — Closing a dashboard session removes it from Inactive

Closing a loaded session drops it from the on-disk and leader rosters. A later refresh does not put that session back under Inactive, and other dormant sessions stay listed (`closing_a_dashboard_session_drops_it_from_inactive`) (`v51-x108`).

### D-104 — The dashboard slash menu includes home-session skills

Opening the dashboard from Welcome lists skills the hidden home session already advertised, and a dashboard that is already open picks them up on the next tick. Session-scoped commands stay hidden (`dashboard_slash_menu_shows_home_session_skills_when_opened_early`, `open_dashboard_slash_menu_gains_home_session_skills_on_tick`) (`v51-x97`).

### D-103 — A peek number pick stays on the row it was made on

A 1–9 pick is cleared when the peeked row changes, including ask questions that have no request id to compare. A refresh of the same row keeps the pick (`apply_fields_drops_a_number_pick_when_the_row_changes`, `peek_number_pick_stays_on_the_row_it_was_made_on`) (`v51-x54`).

### D-102 — Saved pins survive a layout write and an identity change

A grouping change keeps pins for sessions that are not open. A workspace row that becomes a live agent, or the reverse, keeps its pin and that pin still saves under the session id (`v1_grouping_toggle_keeps_pins_for_sessions_that_are_not_open`, `workspace_identity_rebind_keeps_a_pin_in_the_saved_list`, `saved_pins_for_unloaded_sessions_survive_a_layout_write`) (`v51-x49`).

### D-101 — An aborted subagent task is cancelled

A blocking subagent that the user aborts returns a cancelled tool error, not a failed call. The tool row that resume replays says cancelled, and a real crash still says failed (`cancelled_subagent_is_a_cancelled_tool_error`, `cancelled_subagent_task_says_cancelled`, `failed_subagent_returns_error`) (`v51-x36`).

### D-100 — The session picker footer lists each shortcut once

The welcome resume footer takes `d:delete` from the picker action key, so that hint is not also written into the base list. A delete confirmation lists `y` only as confirm, and does not add `y:copy` beside it (`session_picker_footer_lists_each_shortcut_once`) (`v51-x24`).

### D-099 — The dashboard cursor and overlay counter use the grouping on screen

A workspace directory view keeps a selected row even when the saved classic grouping would hide that row behind a collapsed state header (`reanchor_keeps_a_row_cursor_under_the_workspace_directory_view`). The `‹ i/n ›` counter and the prev/next cycle both use that same visible order (`overlay_counter_and_cycle_follow_directory_grouping`, `workspace_overlay_counter_follows_the_workspace_directory_view`) (`v51-x22`).

### D-098 — A resumed foreground subagent shows its activity

When a session load finishes, each still-running foreground subagent reads its on-disk transcript and the parent row shows that activity, such as `Running: Read foo`. A background subagent stays unread until it is opened (`v51-x10`, `resumed_foreground_subagent_shows_its_activity`).

### D-097 — `/exit` in dashboard details closes that session

`/exit`, `/quit`, and the bare exit aliases typed in a dashboard details view close that session and return to the dashboard. The same text on the dashboard list, or in a normal agent, still quits Grok (`v51-f17`, `dashboard_details_exit_closes_only_that_session`).

### D-096 — Pinned sessions stay in pin order and reorder order

Pinning appends the row to the dashboard reorder list. Unpinning removes that slot. The pinned block sorts by that list, so activity, state, and session id do not reshuffle it (`v51-f11`, `pinned_rows_keep_the_order_they_were_pinned_in`). On the workspace dashboard a new pin is stored after the current highest `pin_rank`, an existing pin keeps its rank, and pins without a manual order sort by that rank. A manual reorder still wins (`later_pin_keeps_the_earlier_pin_ahead`, `workspace_pin_rank_order_survives_activity_changes`).

### D-095 — Notes directly above a queued prompt stay on screen

Page-flip still selects the prompt. The viewport top is the first system note in the run immediately above it, so a model-switch result (`Switched to …`) and a `Worktree ready` line both stay on screen. A system note with another entry between it and the prompt still scrolls away (`v75-x01`, widens `v73-x08` / D-093, `queued_prompt_page_flip_keeps_the_model_switch_note_on_screen`).

### D-094 — Restore-code records the stash it just created

`--restore-code` puts a unique token in the stash message and reads that commit back from `git stash list`, so a newer stash from another worktree is not reported as this restore's. When `git checkout` exits non-zero but HEAD is already the target, a post-checkout hook failed after the switch: the checkout still counts and the stash stays (`v73-x13`, `stash_sha_from_list_finds_its_own_commit_when_a_later_stash_is_tip`, `checkout_session_commit_keeps_the_stash_when_the_post_checkout_hook_fails`).

### D-093 — A worktree-ready line stays above a queued prompt

Page-flip still selects the prompt. When the system lines directly above it include `Worktree ready:`, the viewport top is that line, so the note and the strategy line under it stay on screen. A system line that is not in that run still scrolls away (`v73-x08`, `queued_prompt_page_flip_keeps_the_worktree_creation_note_on_screen`).

### D-092 — Resume sits above New worktree on the Welcome menu

The signed-in Welcome rows are Import (when present), Resume session (`ctrl+r`), New worktree (`ctrl+w`, omitted when the sandbox cannot create one), Changelog, Quit. Clicks use that same row list (`v69-f04`, `welcome_menu_lists_resume_above_new_worktree`).

### D-091 — A failed tracked copy stops creation

Linked and standalone creates check the copy issues before `git reset`. A `could not copy` line for a tracked file, or a disk-full line on any copy, returns `worktree copy failed` and the partial directory is removed. A lost mode stays a warning. An ignored file that fails for another reason stays a warning (`v68-x01`, `tracked_copy_failure_stops_creation_and_a_lost_mode_does_not`, `disk_full_stops_creation_even_for_an_ignored_file`).

### D-090 — Create and fork list files the copy could not finish

A file that cannot be written is named with `could not copy <path>`. A file whose bytes landed but whose mode could not be kept is named with `mode change lost for <path>`, and the file stays. Create shows that list from `copiedChanges.warnings`. A fork shows the same list from `copyWarnings` on the resume response. Both are appended to the worktree note (`v68-f01`, `copy_parallel_lists_a_file_it_could_not_copy`, `mode_change_lost_quotes_the_chmod_error`, `resume_response_lists_copy_warnings_for_a_fork`).

### D-089 — Worktrees read the main checkout's saved answers

A worktree keeps its own permission file. On load, the main checkout's always-allow grants are added in memory unless `[features] worktree_inherits_main_approvals` is false. A value that is not a boolean fails closed, so those allows are not inherited. Never-allow grants from the main checkout are added either way. A save writes only the worktree's own grants. Another worktree's grants are not copied. The key is a normal feature: default on, `GROK_WORKTREE_INHERITS_MAIN_APPROVALS` outranks the config file, and a requirements pin outranks both (`v67-f01`, `worktree_inherits_main_checkout_always_allow`).

### D-088 — A typed worktree name is not given a numbered suffix

A name the user typed is claimed exactly. If that directory exists, or another live create already claimed it, create stops with `worktree name '<name>' is already taken`. Names grok chose still use `label-2` when they collide (D-063). Deleting the folder by hand drops the stale `.git/worktrees` registration on the next add, so that name can be used again (`v64-x06`, `typed_name_is_refused_while_taken_and_reusable_after_the_folder_is_gone`, `two_prepares_with_the_same_label_choose_different_directories`, `worktree_add_reuses_a_name_after_the_folder_was_deleted`).

### D-087 — Worktree choices stay hidden when the profile cannot create one

The same strict-profile refusal hides New worktree on the Welcome menu, the session-picker worktree hint, and the `/new` and `/fork` questions. Those commands stay in the current directory. An explicit worktree flag still reaches create, which reports the profile (`v64-x05`, `menu_action_hides_new_worktree_when_the_sandbox_refuses`).

### D-086 — A strict sandbox profile refuses worktree creation up front

`strict` can write `sessions/` and not `<grok home>/worktrees`. Create, fork, jj, and a subagent worktree now stop before making a directory and say `worktree creation is unavailable under sandbox profile 'strict'`. An unknown profile returns the sandbox's own not-found error. `workspace` and `read-only` can write the grok home, so they still create (`v64-x04`, `strict_profile_refuses_worktree_creation`).

### D-085 — Tracked ignored files stay, exclude-only files stay out

The worktree copy treated every `.gitignore` match as skippable, so a tracked file such as `secret.env` never arrived. It also copied untracked files whose only ignore rule was `.git/info/exclude` or `core.excludesFile`. The copy now applies those two sources, and a tracked path the walker skipped is copied afterwards. An untracked file that matches only one of those rules is left out (`v64-x01`, `create_keeps_tracked_gitignored_files_and_omits_exclude_only_files`).

### D-084 — Unicode worktree names stay, and a leading dash is refused

A name typed with Ctrl+W such as `café` was stored as `caf` because the directory name kept only ASCII letters. Unicode letters now stay. A name or ref that starts with `-` is refused before a directory is claimed, and `git checkout` / `git worktree add` refuse that ref before git runs. A hyphen later in the name is still allowed (`v64-f01`, `unicode_worktree_name_is_kept_and_a_leading_dash_is_refused`, `worktree_dialog_keeps_unicode_letters`, `worktree_dialog_refuses_a_name_that_starts_with_a_dash`, `checkout_ref_refuses_a_leading_dash`, `worktree_add_refuses_a_leading_dash`).

### D-083 — `-w` and Ctrl+W say that no worktree was created

Outside a git repository, `grok -w` and Ctrl+W already created nothing. The messages did not say so, and Welcome Ctrl+W was silent. They now say `No worktree was created: this directory is not a git repository`. The `-w` banner still adds the git-init guidance. Dashboard confirm uses the same sentence. `/fork` is unchanged (`v63-x03`, `new_worktree_session_rejects_non_git_cwd`, `dashboard_toggle_worktree_outside_git_repo_is_noop_with_toast`, `dashboard_confirm_worktree_without_git_repo_creates_nothing`, `welcome_ctrl_w_outside_git_repo_explains_nothing_was_created`).

### D-082 — List and show name a missing folder or a moved source

`grok worktree list` and `show` now say when the worktree directory is gone, or when the recorded source is no longer a directory. The repair line is `grok worktree rm <id>` for a missing folder, and the same command plus creating the worktree again from the new location when the source moved. A relative source such as `unknown` is not treated as moved (`v63-f02`, `list_and_show_mark_a_missing_folder_and_a_moved_source`).

### D-081 — `rm` suggests `-f` only when `-f` can remove

A git repository, a `.git` directory, and any other folder grok did not create are refused before the unsaved-work check, and those messages do not say `pass -f`. `-f` refuses them too. A dirty directory under `<grok home>/worktrees` still says `pass -f`, and `-f` removes it (`v59-x02`, `rm_suggests_force_only_when_force_can_remove`). Already present after `v52-x11`.

### D-080 — A worktree fork shows and runs in the worktree

The fork copied the source checkout into the prompt and left that path in the history, so a shell command edited the original checkout. A worktree fork now keeps the worktree as the session directory and rewrites copied paths onto it. The reminder names the source checkout and does not split the shown path from the shell path (`v54-x06`, `worktree_fork_runs_in_and_shows_the_worktree_path`).

### D-079 — A subfolder launch records the checkout root

Creating from `repo/crates/pager` stored that subfolder as `source_repo`. `list` and `show` print that field. Registration now stores the checkout root `find_worktree_root` already uses for the copy (`v53-x01`, `create_from_a_subfolder_records_the_repository_root`).

### D-078 — `-f` does not delete a folder grok did not create

`v50-x58` let `-f` remove an unmanaged path. `rm` now refuses that path even with `-f`: a git repository, a `.git` directory (including one under the managed root), and any other directory that is not registered and does not live under `<grok home>/worktrees`. A directory grok created can still be removed (`v52-x11`, `forced_rm_refuses_a_repo_a_git_dir_and_an_unmanaged_folder`).

### D-077 — An open session keeps its worktree

`rm` and `gc` already skipped a live creator and a process whose cwd was inside the worktree. A session can use the worktree while that process cwd is elsewhere. A live entry in `active_sessions.json` whose cwd is inside the worktree now keeps it. A dead pid does not. An unreadable registry refuses `rm` and skips age expiry (`v52-x06`, `open_session_keeps_a_worktree_rm_and_gc_would_delete`).

### D-076 — Empty untracked folders are not unsaved work

A folder that contains only other folders collapses into one untracked directory, so `rm` and `gc` kept the worktree. That directory holds nothing git can store. A file anywhere under it still keeps the worktree, and a read error still refuses the removal (`v51-x98`, `empty_untracked_folders_do_not_keep_the_worktree`).

### D-075 — Rebuild restores the worktree's directory name

A rebuilt registry row had no `metadata.label`, and its id is a path hash, so `show` and `rm` by the directory name missed it. The rebuild now stores that directory name as the label (`v51-x79`, `rebuild_restores_the_directory_name_for_lookup`).

### D-074 — `worktree list` includes an unfinished copy

`list_worktrees` only read the registry. An interrupted create leaves a directory under `<grok home>/worktrees/<repo>/<name>` that was never registered, sometimes before `.git` exists. The list now adds those directories from the on-disk scan and does not repeat a path the registry already has (`v51-x68`, `list_worktrees_shows_an_unfinished_copy`).

### D-073 — Session lists stay inside one git checkout

`/resume` and `grok sessions list` load worktree sessions through `candidate_worktree_cwds_for_same_repo`, which keyed every repo by `commondir().parent()`. Sibling submodules then shared `.git/modules`, and separate-git-dir checkouts shared the folder that holds their git dirs. The main root is now the checkout whose `.git` is that git dir, so each list only includes that checkout's worktrees. A linked worktree of a normal repo still resolves to the main checkout (`v51-x59`, `session_candidates_skip_sibling_submodule_and_separate_gitdir`).

### D-072 — `rm` and `gc` free a name whose folder was deleted by hand

Deleting the directory leaves `.git/worktrees/<name>`. The next `-w NAME` then fails. When the folder is already gone, `remove_worktree` and worktree gc drop that registration before forgetting the registry row (`v51-x56`, `rm_frees_a_name_after_the_folder_was_deleted`, `gc_frees_a_name_after_the_folder_was_deleted`). A directory that is still there keeps its registration.

### D-071 — A symlinked `GROK_HOME` still tracks and resumes the worktree

`$GROK_HOME` stays the env spelling. `current_dir()` is the physical path, so a raw prefix check missed the worktree and the session was opened on the link path. Lookups now compare the canonical directory, and `-w` opens the physical worktree path, which is the path resume reads back (`v51-x55`, `symlinked_grok_home_still_tracks_the_worktree`, `create_response_uses_the_physical_path_when_grok_home_is_a_symlink`).

### D-070 — `-w` opens the worktree root when the launch folder is absent

`parse_create_response` still prefers `<worktree>/<launch subfolder>` when that directory exists. When it does not, the session cwd is the worktree root and the strategy note says the folder is not in the new worktree. That note is the system line `-w` already shows for strategy summaries (`v51-x42`, `create_response_opens_the_root_when_the_launch_folder_is_missing`).

### D-069 — Worktree copy skips named pipes and sockets

The parallel copy used for worktree creation treated every non-directory as a file. Opening a FIFO waits for a writer, so a repo that contains one never finished. Those entries, and Unix sockets, are now skipped (`v51-x34`, `copy_parallel_skips_a_fifo_and_a_socket`). Regular files in the same tree are still copied.

### D-068 — `grok worktree` treats a `.jj` checkout as Jujutsu

Colocated repos were already sent to `jj workspace add` and `jj workspace forget`. A checkout libgit2 cannot open still has a `.jj` directory, and `checkout_is_jj` now accepts that too, which is what `grok worktree create` and resume consult (`v51-f26`, `checkout_is_jj_accepts_a_repo_without_git`). The created workspace is written into the worktree registry so `show` can find it by label (`jj_workspace_is_visible_to_worktree_show`).

### D-067 — Session lists include the same subfolder inside each worktree

`grok sessions list` and `/resume` collect sessions through `build_candidate_list`. A launch from `repo/crates/pager` stores the session under `<worktree>/crates/pager`, and the list previously looked only at the worktree root. Each worktree candidate now also includes that relative folder (`v51-f16`, `build_candidate_list_includes_the_same_subfolder_of_each_worktree`). A directory that is not inside the main repo does not gain a suffix.

### D-066 — Headless JSON names the worktree folder after the session starts

`--output-format json` adds `worktreePath` to the result object when `-w` opened a session. That path is the created worktree root, so a label collision still reports `…/label-2`. The same field is added to a JSON error only after that session exists. Errors before the session starts omit it (`v51-f13`, `json_result_reports_the_opened_worktree_folder`; `v51-f15`, `json_error_includes_the_worktree_folder_after_the_session_starts`).

### D-065 — `rm` by label refuses when live worktrees in different repos share it

`get_by_label` still returns the newest record. `resolve_worktree_by_id_or_path` now refuses a bare label when more than one alive record with that label has a different `source_repo`, including with `-f`, and tells the caller to pass the path (`v50-x62`, `rm_by_label_refuses_when_the_label_is_in_multiple_repos`). The same label inside one repo still removes the newest. A dead record does not count. An exact id still wins over a label of the same text.

### D-064 — `rm` without `-f` leaves a folder grok did not create

`resolve_worktree_by_id_or_path` no longer treats any existing directory as a worktree. Without `-f`, `remove_worktree` deletes a path only when the registry already has it or the directory is under `<grok home>/worktrees`. Anything else returns `refusing to remove …: it is not a grok worktree` and the folder stays (`v50-x58`, `unforced_rm_keeps_an_unrelated_folder_that_shares_the_name`). `-f` still removes an unmanaged path. `v52-x11` may later refuse git repos and `.git` folders as well; that row stays pending.

### D-001 — version identity
The package version stays `1.0.45`. Do not stamp this fork as 1.0.78. `crates/codegen/xai-grok-shell/CHANGELOG.md` stays the 1.0.45 history. `/release-notes` already prepends newer CDN notes when online. Adding 1.0.46–1.0.78 headings locally would claim this binary shipped them.

### D-002 — later note wins
When two notes describe the same control, implement the highest version only.

### D-003 — no binary reconstruction
Official 1.0.45 and 1.0.78 Linux binaries were inspected earlier. They are stripped, they have no cargo-auditable dependency list, and their `.gnu_debuglink` name `xai-grok-pager.debug` is not on the public CLI bucket. They are not an input to implementation.

### D-004 — unspecified releases
`v55-x01`, `v56-x01`, `v57-x01`, and `v58-x01` say only "Minor bug fixes." Status `skipped`. There is nothing to build.

### D-005 — do not invent services
If the note needs a server behavior this tree does not already call, mark `deferred`.

### D-006 — protected fork behavior
These stay, from the README section "What this fork adds":

- Copyable tool errors, including the expanded Edit reason and the searched text.
- Session turn index on plain user prompts (`Turn N`) and the composer prefix. Bash, cron, and interjection prompts stay unlabeled.
- Sticky Edit path while an expanded diff is taller than the window. Config remains `[scrollback.blocks.edit] sticky_header`.
- Subagent session ids in the child window and on the parent row, including `Ran N subagents` groups, with click or `c` to copy.
- Trusted plugin hooks register at session start, not only after a reload.
- `/release-notes` lists every shipped version, and CDN notes newer than the embedded file are prepended.
- `Ctrl+Shift+Y` / `F6` copies assistant markdown source. `[ui].copy_markdown_shortcut` still works.
- `Shift+Down` / `Shift+Up` step through tool runs as the README describes. Plain arrows, Left/Right, and Shift+Left/Right keep their current jobs.
- Native Windows proto codegen and the 8 MiB main-thread stack.

W16 re-checks this list after the last wave. A wave that touches scrollback, hooks, or subagent rows re-checks the overlapping items before the session ends.

### D-007 — subagent message layout
Skip the centered-row designs: `v71-f08`, `v71-f09`, `v73-f02`. Implement `v74-f01`: left-aligned rows under the bullet, naming sender and recipient. Keep the 1.0.71 behavior that consecutive messages fold into one row, unless that fold cannot be drawn on the left-aligned row. Record the result on `v74-f01`.

### D-008 — Rex status
Skip `v52-f01` (one OSC 7501 status for the whole process). Implement `v63-f01`: one record per open session and per running subagent, with `[ui.notifications.program_status] children = false` reporting only the main status.

### D-009 — vim settings
Final config is `[keys] vim` with `off`, `nav`, and `all` (`v68` and `v71`). `[ui] vim_mode = true` still means `nav`. Do not add a second independent boolean. Suggestion keys from 1.0.67 follow this setting.

### D-010 — memory config
User-facing memory is on or off (`v51` breaking). `rollout` and `file_writes_enabled` are removed as controls. `compact_index_enabled` is accepted and ignored (`v63` breaking). Old config files must still parse. The index is the concise `MEMORY.md` of titles and paths, and `/memory` sorts by how often a topic is read.

### D-011 — Ctrl+L
`v75` binds Ctrl+L in the scrollback to quoting the selection, and moves Extensions to `/plugins`. `v76` redraws after a terminal clear without flashing. `v77` makes that redraw work on more screens. Implement both when the context differs: a scrollback selection quotes; a clear or a screen with no selection redraws. If one handler cannot do both, redraw when nothing is selected and quote when something is. Record which binding won. Do not bring back Ctrl+L as "open extensions."

### D-012 — remove /share
`v51` breaking removes `/share` and the session-sharing command path. This tree still lists `share` and has `share_session` in `remote/client.rs`. Remove the command and the client call that users and the model can reach. Leave proxy types in place if deleting them breaks the crate, and record that.

### D-013 — timestamps
Today `show_timestamps` is a bool. The 1.0.50 setting is `off`, `minimal`, or `on`. Map old `false` to `off` and old `true` to `on`.

### D-014 — tests and commits
A behavior change needs a test in the crate that owns it, unless the existing tests already cover the final behavior. Then say so in the decision. Follow the cargo test filter rules in the user rules. Commit the tracker with the code.

### D-015 — Ctrl+L is not a fork shortcut
The fork's copy-markdown chord stays `Ctrl+Shift+Y` / `F6`. Do not move it onto Ctrl+L.

### D-016 — resume matching is one feature
1.0.54, 1.0.59, 1.0.60, 1.0.61, 1.0.65, and 1.0.69 all change `/resume`. Implement the final combination, and mark the earlier rows `done` in the same commit once that combination holds, each with a decision pointing at the row that landed the final behavior. Do not ship the intermediate matchers.

### D-017 — session size
One wave per session. W14 and W15 stop at a version boundary. Ambiguities covered here are not questions for the user.

### D-018 — new upstream notes
Versions published after 1.0.78 are appended. Completed rows are not rewritten.

### D-019 — run until completion
The user asked to run this plan until every catalog row is terminal. That suspends D-017's one-wave stop for this execution. Isolated worktrees may implement later waves in parallel. This file stays the only tracker. A row is `done` only after its change is in this checkout and its test passed. Checked 6 November 2026 and again on 7 November 2026: alpha is still 1.0.78, and `1.0.79.external.md` is 404, so no rows were appended.

### D-020 — v51-b01 deferred
Searched `rewind_files` (only `checkpoint.rs` `rewind_to`), `finish_rewound_cancel` (truncates the chat snapshot and file-state metadata, and does not write file bytes), pager `pending_cancel_resend` (re-sends `session/cancel` with `rewind_prompt_id: None`), and `handle_rewind` (file bytes change only for an explicit rewind in `RewindMode::All` or `FilesOnly`). Cancel and edit-resend do not restore workspace files in this tree. There is no failing path to fix. Status `deferred`.

### D-021 — v51-b03 share removal
Removed the pager `/share` slash command, the command-palette Share row, and the hidden `grok share` subcommand. `x.ai/share_session` stays registered and returns "Session sharing has been removed." without uploading. `BackendClient::share_session`, `ShareSessionRequest`, and `ShareSessionResponse` stay: nothing calls the client method, and the pager effect match still names the request types. `set_share_visible` stays so registry tests can insert their own command. The model reserved-name list no longer contains `share`.

### D-022 — v64-b01 untrusted hooks
An untrusted folder keeps hooks whose provenance is `SystemManaged`, `Requirements`, `SignedRequirements`, or `Managed`. `UserRequirements`, user config, and user hook files run only when `user_hooks_trusted` (headless via `startup_hints.non_interactive`, reload `attach_non_interactive`, or session meta `sessionKind=headless`). Project, plugin, and agent hooks stay off until the folder is trusted. Plugin hooks are not merged on spawn or on mid-session reload when the folder is untrusted. Trusted plugin hooks still register at session start (D-006).

### D-023 — v50-b01 and v51-x96 worktree rm
`refuse_unforced_removal` runs before dry-run. In use means a live `creator_pid` on the worktree record, or a live process whose cwd is inside the worktree. Unsaved means `KeepReason::Dirty` or `HiddenFromStatus`. `CheckFailed` and `GateTimedOut` refuse, which is v51-x96. `Unpushed` does not refuse. A missing path is not refused, so a stale registry row can still be removed. The CLI bails on the first removal error so the process exits non-zero.

### D-024 — W6 memory follow-through
`web_fetch` rejects a body from `Content-Length` before reading it, and stops the byte stream at the cap (`v50-x22`). `memory_get` uses the topic read limit and does not return a larger file (`v50-x23`). CJK queries that FTS5 unicode61 cannot match inside a longer token are found by a substring scan of chunk text (`v50-x25`). The main and subagent prompts include `<memory-status>` when memory is off, and the existing `<memory>` block when it is on, so a resumed head still reconciles on the on-block only (`v51-x100`). Search snippet lines keep the query match instead of a prefix cut (`v54-x08`). A symlinked note is skipped in `/memory` listing; a symlinked topics or inbox directory still fails the listing (`v54-x09`). `v73-x12` is Kitty image memory, not this cluster.

### D-026 — W3 permissions, sandbox, and config
The eleven `port:` commits from the W3 worktree are on `rebuild`. Already: symlink canonicalization (`v46-x04`), custom-model notices (`v48-x02`), skipping one bad managed or compact rule (`v50-x56`, `v50-x60`), unknown tables (`v51-f24`), always-approve before the web prompt (`v51-x04`), unmatched typing does not answer (`v51-x67`), prompts name the session (`v52-x10`), a non-boolean known feature fails closed (`v53-x22`), and make/cmake/just are routine prefixes (`v64-f04`). Deferred: no separate shell allow/ask/deny mismatch (`v50-x01`), no in-flight accidental always-approve (`v51-x31`), no separate delete tool (`v51-x69`, `v51-x88`), no allow path for a denied wildcard redirect (`v52-x30`), and no macOS window-server sandbox path (`v74-x21`).

### D-063 — Simultaneous same-label creates both get a directory

`resolve_label_collision` claims the name before the directory exists. A second create of the same label, in this process or another live one, gets `label-2` instead of both choosing `label` and one failing with "already exists". A claim whose process is gone does not keep the name, and the claim is released when create finishes so a deleted folder can be used again (`v50-x57`, `simultaneous_same_label_claims_distinct_names`, `two_prepares_with_the_same_label_choose_different_directories`).

### D-062 — A failed worktree create is the home-screen banner

The welcome screen shows one warning, the first `Warning` in `startup_warnings`. `handle_worktree_session_failed` now inserts `Cannot create worktree: …` at the front, the same slot a failed session create uses, so an older probe warning does not cover it. The queued prompt goes back to the welcome composer (`v50-x53`, `worktree_create_failure_restores_queued_prompt_to_welcome`).

### D-061 — Worktree create errors already share one disk-full message

`WorktreeBuilder::create` runs every strategy through `execute_plan` and `annotate_disk_full`. A full disk, whether it is `ErrorKind::StorageFull` or git's `No space left on device` text, displays as `not enough free disk space` (`annotate_disk_full_promotes_reason_to_top_context`, `is_out_of_disk_detects_message_text`). Grove `StorageFull` uses that same string. The pager then shows `No space left on device` (`sanitize_user_error_collapses_disk_full`). Other failures keep their own text. `v50-x52` is `already`.

### D-060 — `--worktree-ref` checks out the recorded submodule and LFS bytes

A non-HEAD ref checkout runs with `GIT_LFS_SKIP_SMUDGE` and does not move submodule directories, so the new worktree kept the source submodule commit and the LFS pointer text. `materialize_ref_checkout` checks each gitlink out of the superproject's module store and replaces a pointer whose object is already local (`v50-x51`, `test_worktree_ref_checks_out_recorded_submodule_and_lfs_bytes`). A ref of `HEAD` is unchanged.

### D-059 — Git-mode `-w` keeps uncommitted changes

`CreationMode::GitCheckout` runs `git worktree add`, which checks out the commit and drops the source working tree. When the mode is `PreserveWorkingTree` and the ref is `HEAD`, the create path then copies that dirty state with `WorktreeSync::sync_dirty_state` (`v50-x50`, `test_git_checkout_preserves_uncommitted_changes`). A named `--ref` stays a clean checkout of that ref. `CleanAll` still drops the changes.

### D-058 — Fork worktree image and log paths stay real

`PathRewriter::rewrite` still maps a project file from the worktree onto the display path. It no longer rewrites the URL-encoded worktree cwd. Session images and terminal logs live under `sessions/<encoded real cwd>/`, and replacing that component reported a directory that was never written (`v50-x30`, `test_rewrite_handles_url_encoded_paths`, `test_rewrite_json_handles_url_encoded_paths`).

### D-057 — Dashboard worktree column, model badge, and folder-picker toggle

A session row's subtitle names the worktree label, the branch, and a `worktree` marker (`subtitle_worktree_shows_label_branch_and_marker`). The new-agent box paints `pending_model`, or the current model name when nothing is staged (`paint_dispatch_config_badge`). `-m` is `default_model_override`, which becomes that current model, and opening the dashboard copies `app.models` onto the box. The folder picker paints `[worktree:on]` / `[worktree:off]` on a git target and a click flips `worktree_mode` (`render_location_picker_shows_worktree_toggle_in_repo`). `v50-f11` is `already`.

### D-056 — Hidden Kitty images are already deleted

`clear_kitty_image` sends `a=d,d=i`, which deletes the image data in the terminal. `AgentView::draw` emits that delete for an inline id placed last frame and not placed this frame, then drops the id (`render.rs`). A modal or viewer covering the transcript deletes every known inline id the same way. A missing prompt preview calls `overlay::clear()`, which deletes placement id 1. Fullscreen takeover and a closed child view drain through `take_inline_media_clear_escapes` (`take_inline_media_clear_escapes_drains_placements`, `subagent_fullscreen_draw_clears_parent_inline_media`, `draw_after_subagent_close_clears_child_inline_media`, `active_modal_returns_clear_without_committing_discarded_state`, `low_level_overlay_deletes_then_transmits`). The 64 MiB CPU cache eviction keeps a still-placed id, because that image is on screen. `v73-x12` is `already`.

### D-055 — Dashboard rows stay Working for background work
A running background subagent, or an active workflow run, keeps the dashboard row in Working after the parent turn is idle. A foreground subagent does not. Stopping that row cancels the background subagent and sends `/workflow stop` for each active run (`v75-x16`, `background_subagent_or_active_workflow_keeps_the_row_working`, `dashboard_stop_ends_background_subagents_and_workflows`).

### D-054 — Hover on question submit, popup close, and workflow rows
The question card's Enter button uses `enter_button_styles`: under the pointer the label is bold `text_primary` on `bg_hover` (`enter_submit_button_highlights_under_the_pointer`). The dashboard session popup's `[✗]` uses `popup_close_button_style`, and `sync_popup_close_hover` tracks the pointer (`popup_close_highlights_under_the_pointer`). A collapsed workflow run row passes that hover into `render_picker_row` (`collapsed_workflow_row_highlights_under_the_pointer`, `mouse_over_a_collapsed_workflow_row_sets_hover`). `v75-x11`.

### D-053 — Running color on workflow marks and starting MCP servers
An active workflow run, a running agent, and a phase that still has running agents use `theme.running`. An MCP server that is still initializing already uses that same color (`v74-x31`, `running_markers_use_the_theme_running_color`).

### D-052 — Workflow result, `/tasks` time, unknown task row
A finished workflow transcript line ends with `result_summary` (`deep-research done in 1m30s: shipped the port`). An empty objective and an empty result drop the colon (`v74-x19`, `done_line_ends_with_the_result_instead_of_a_bare_colon`). `/tasks` adds the scheduled row's next-fire time, the same suffix as the tasks pane (`subagent_shutdown_drops_its_loop_from_the_rendered_task_list` expects ` (due now)`). Polling unknown task ids keeps one result row per id, and that row's output is `Task {id} not found.` (`multi_task_ids_poll_returns_multi_result_mode_poll`).

### D-051 — Inline tags and live workflow stop
An inline tag such as `<decl>` stays on the same wrapped line as the surrounding sentence. The soft break after the tag becomes a space (`v52-x23`, `test_soft_break_around_inline_html_decl_tag`, `test_soft_break_original_bug_repro`). `/workflow stop` already prefers the live numbered sibling over an ended exact name (`stop_prefers_live_sibling_over_ended_exact_name`, D-044).

### D-050 — No pause line after a headless goal completes
`record_headless_goal_update` prints a success status (`complete`, `completed`, `done`) and then ignores later updates for that goal id. A following `paused` status would otherwise be the plain line `Goal paused.` A stale `active` does not go back into the exit set, so exit does not send `/goal pause` (`headless_exit_pauses_goal`). A goal that never completed still prints a pause. The memory flush keeps the same finished-goal set (`v51-x92`, `headless_goal_completion_does_not_print_a_later_pause`).

### D-049 — Quit confirms when a workflow is running
The first quit while any session has an active workflow arms "stop the running workflow and quit". The second press of the same key quits. A finished workflow keeps the plain "quit" label (`v53-f05`, `quit_with_a_running_workflow_warns_and_confirms`).

### D-048 — Reopening keeps an active goal running
`GoalTracker::from_snapshot` leaves an `Active` goal active and starts its elapsed clock again. In-flight Planning or Executing still returns to Idle, because those subagents are gone. An explicit pause stays paused (`v51-f30`, `from_snapshot_keeps_an_active_goal_running`). The resume reminder says to continue an active goal. A paused goal still says it can be resumed (`active_goal_on_resume_says_to_continue`).

### D-047 — Images on a skill command
A pasted image on a skill command such as `/pr-workflow` stays on the queued skill row and is sent as an image block with the slash line (`v51-f27`, `skill_prompt_with_images_sends_them`). Expanded wire payloads (`/imagine`, `/loop`) and command rows (`/compact`) still drop images and say so.

### D-046 — Goal cancel, disabled `/goal`, and the bundle cache
Ctrl+C cancels the turn through `SessionCommand::Cancel`, and that handler pauses an active goal with `GoalPauseReason::User` (`v51-f28`, `run_loop.rs`, `auto_pause_goal_if_active`). With the goal gate off, `resolve_human_intent` returns the `/goal` line as a normal prompt. With the gate on, the same line is `GoalSet` (`v51-x110`, `disabled_goal_line_passes_through_as_a_prompt`). A missing or unparseable bundled-skills manifest is not fresh, so the next sync refreshes skills, agents, and workflows (`v74-x01`, `bundle_cache_is_fresh`, `bundle_cache_is_fresh_false_for_corrupted_manifest`, `maybe_sync_runs_when_no_manifest_exists`).

### D-045 — Workflow run roster
Status lines already say how many agents are running. `activity_label` counts roster rows in `running`, and ingest ignores a stale `active_agents` value (`v75-f08`, `workflow_suffix_counts_only_running_roster_rows`, `workflow_snapshot_derives_active_count_from_roster`). A running workflow in the dock already spins (`v51-x02`, `dock_workflow_rows` sets `spinning` when the run is active, `running_workflow_is_a_dock_row_above_watchers`).

Agents on a phase that was not declared are added to the rail, and agents with no phase are listed under Other. The done numerator counts only `state == "done"` (`v53-x10`). Wheeling down the roster drops the previous top-agent anchor, so the window moves toward newer agents instead of snapping back (`v61-x02`). A roster narrower than 48 columns hides live status text and gives that width to the agent name (`v63-x06`). Right, or a click on a phase or an agent, focuses that phase's agent list. Up and Down then pick an agent, Enter opens it, and Left returns to the phases (`v72-f05`). Workflow row marks use `pause_mark` and `idle_mark`. Todo rows already use `play_mark`, and list rows already use `play_mark`. On a legacy console those helpers draw `>`, `=`, and `o` (`v75-x05`, `legacy_row_marks_are_one_column_and_drawable`).

### D-044 — Workflow list, stop, and the tasks pane
`/workflow`, `/workflow runs`, `/workflow stop`, and `/workflow resume` are local builtins. They list or control runs and do not send the line to the model (`v51-f21`, `manage_workflow_run`, `workflow_manage_parses_both_orders_and_optional_id`). Background runs are rows in the tasks pane. A run that `can_stop` shows the stop button, including a paused run (`v51-f22`). Stop and pause require an exact name or id. An ended exact name still reaches its live numbered sibling. A loose prefix does not pick a run. Headless prints each refused approval request before it cancels it (`v53-x09`).

### D-043 — Headless waits for a background goal
`grok -p` keeps a goal in the background set while its status is `active`, and prints the outcome when that status changes (`v51-f02`). `--background-wait-timeout` then pauses the goal with `/goal pause` and exits with an error (`v51-f19`). The later follow-up-turn wait in `v62-x01` is still pending.

### D-042 — Goal chip with workflow runs
Clicking the goal status chip opens the goal panel when a goal is active, including while workflow runs exist. It no longer toggles the workflow overlay instead (`v47-x02`).

### D-041 — Remaining W4 hook notes
`[[ui.notifications.hooks]]` commands run from `NotificationService::notify` when the hook's event list matches. An empty list matches every kind. `only_unfocused` waits until the terminal loses focus (`v50-x18`). `/hooks` lists `load_errors` and the expanded row keeps the full message (`v51-f25`). Plugin inline hooks and the main agent's inline hooks are included in that list (`v53-f02`). Tool hooks for one session take a turnstile in `run_hook`, so parallel tool calls do not overlap. Cancel and exit still kill the hook process group (`v51-x63`, `command_hook_session_close_reaps_whole_group`). `TaskCompleted` fires a `task_complete` Notification hook, and `send_xai_notification` awaits that dispatch before it returns (`v51-x91`). A PowerShell command that reads `$input` runs, with a warning, instead of being refused as a missing environment variable (`v59-x03`). A spawn failure names the program, including `powershell.exe` or Git Bash (`v61-x03`).

Already: a hook ask is the first line of every permission card, including web search and a shell delete (`v51-f23`, `permission_description_lines`). MCP calls use that same request, and a hook ask forces the prompt (`v51-x66`). Hook notes and tool outcomes are different session events (`v51-x101`, `child_hook_note_shows_in_child_view`).

Deferred: no code path emits "hook state lost". Tool preparation is already one call at a time (`v51-x37`). Compaction in `xai-chat-state` and `xai-grok-compaction` does not treat PostToolUse output differently from other conversation text (`v52-x17`).

### D-040 — Hook prompts and detached children
A hook `ask` forces a permission prompt even in always-approve mode (`v49-x01`, `pre_tool_use_ask_hook_forces_prompt_under_yolo_and_runs_on_approve`). Hook children cannot open `/dev/tty` after `setsid` (`v51-x105`, `hook_child_cannot_open_dev_tty`). Tool commands started from the TUI use the same session detach in `streaming_local_terminal`.

### D-039 — Quiet hook events
`SubagentStart` and `Notification` hooks still run. Their start and completion are not sent to the UI (`v51-x115`).

### D-038 — Managed hook policy on the next session
Each session reads managed settings from disk via `load_managed_settings`, so `allow_managed_hooks_only` applies to the next session without restarting the process (`v51-x58`). A blocked prompt does not send the content chunk that titles the session (`v51-x80`, `turn.rs`). A shell PreToolUse hook receives the model's raw arguments, including `block_until_ms` (`v51-x65`).

### D-037 — Permission block keeps PreToolUse context
When a permission rule blocks a tool, the context returned by its PreToolUse hooks is still queued for the model (`v52-x19`). The deny path uses `pre_tool_context_for_model`, and that helper is unit tested.

### D-036 — Subfolder project hooks
Launching inside a repository subdirectory also loads that directory's `.grok/hooks`, in addition to the git root (`v52-f03`).

### D-035 — Quit runs Stop and SessionEnd together
On exit, `SessionEnd` hooks and the session-end `Stop` hook run at the same time. `Stop` is capped at 1.5 seconds (`v67-x03`).

### D-034 — Project hook disable is per directory
`/hooks` writes `source_dir|name` into the disabled-hooks file. A hook with the same name loaded from another directory stays enabled. A legacy line that is only the name still disables every copy (`v75-x18`).

### D-033 — Localhost HTTP hooks and web-search rewrites
`http://127.0.0.1` and `http://localhost` are allowed. Any other host still requires `https`. A URL that does not parse, or uses the wrong scheme, is a load error (`v52-f04`). A web-search rewrite drops `allowed_domains` and `excluded_domains` before the schema check (`v51-x89`). `WebSearch` matches `web_search` and `Glob` matches `list_dir`; the hook payload is the tool's raw input, which carries `query` or the directory (`v51-x44`, `v51-x46`). Invalid rewrites were already denied by `validate_hook_rewrite` (`v51-x40`).

### D-032 — One hook run, and continue false stops the turn
A hook with the same command under two matchers runs once for a tool call that matches both (`v51-x60`). `continue: false` on PreToolUse denies the tool and ends the turn. On UserPromptSubmit it blocks the prompt. On PostToolUse it becomes a block with the same prefix, and that also ends the turn. A plain deny still lets the turn continue (`v59-x04`).

### D-031 — First hooks slice
Tested in `xai-grok-hooks`: a bare command missing beside the hook file is launched from `PATH` (`v51-f14`); stdin ends with a newline (`v51-x57`); an unreadable disabled-hooks file keeps non-managed hooks off and still runs system policy hooks (`v53-x15`); `args` is stored and `failClosed` parses (`v63-f03`, `v71-f04`); a missing `failClosed` PreToolUse command is denied. A matcher with a comma or a space was already a regex, not a name list (`v52-x33`). PostToolUse already sends `duration_ms` from `Duration::as_millis` (`v51-x64`, `tool_calls.rs`).

### D-030 — W2 is complete, plus tested CLI rows
Pager tests passed for rewind leaving the caret at the end of the restored prompt (`v50-x09`), ghost suggestions that do not cover text after the cursor (`v51-x01`), editable `@` file chips (`v74-f10`), and interactive `--tools` / `--disallowed-tools` (`v53-f03`). Also tested: narrow home and login wrapping (`v47-x01`), plugin suggestions hidden when plugins are disabled (`v64-f06`), a case-insensitive single title match (`v59-f02`), a local title miss that does not start a remote restore (`v60-x01`), and `--allowed-tools` as an alias (`v51-f34`). `--add-dir` refuses a missing or untrusted directory; that behavior is tested and has no separate catalog row.

### D-029 — Tested dashboard slice
On `rebuild`, with pager tests: closed stdout is a clean stop (`v62-x04`); Left on an empty focused prompt opens the dashboard (`v75-f06`); the folder picker keeps 50 recent folders (`v65-f03`); closing the peek cancels a waiting reply (`v59-x01`); the path field starts unfocused (`v68-f05`); the peek says Responding and the footer says new agent (`v74-x29`); `/resume` filters by id or title (`v54-f01`); `grok --resume <title>` orders matches and names similar titles (`v60-f01`); words match in any order at word starts (`v61-f01`, `v65-x01`); the in-session picker follows the grouped setting (`v52-x31`); a model pick keeps its context window (`v68-x03`). Not in this slice: direct open on one `/resume` match (`v61-f02`), and typo or transcript search (`v65-f01`).

### D-028 — W8 shell resume slice
`v69-f03` matches an exact id, then a separator-insensitive title, then fuzzy word prefixes, includes other folders, and sorts the current folder and its checkouts first. `v53-x13` skips an unreadable session directory. `v75-x15` retries a delete that hits a non-empty directory. The shell tests for those three passed before the commits were cherry-picked (140 passed, including the new resume, relocation, and jsonl checks). Dashboard rows are not in this slice.

### D-027 — Cherry-picked workflow, image, MCP, and CLI commits
Unit tests in this checkout passed for `v50-x12`, `v50-x27`, `v50-x28`, `v50-x29`, `v50-x32`, `v50-x33`, `v51-f01`, `v51-x86`, `v51-x117`, `v52-x08`, `v53-f04`, `v53-x07`, `v53-x08`, `v53-x19`, `v54-x01`, `v64-f05`, `v66-x02`, `v72-x09`, and `v74-x24`. `v54-x01` covers the stray markdown `!`, currency amounts, and Windows pastes: `\r\n` and bare `\r` become newlines in the composer.

### D-025 — W15 classifications
`v54-p01` is already the slow welcome tick (`view_tick_demand` returns `TickDemand::Slow`, 83 ms). `v54-p04` is already `evict_offscreen_render_caches`, which drops off-screen render caches and keeps heights. `v69-p01` is already `run_and_shutdown`: `shutdown_timeout` is a ceiling, and its test checks that a clean exit returns before the grace. `v73-p01` is deferred: auth startup, `read_auth_json`, managed policy, and the config watcher have no ~100 ms wait on a missing sign-in file. The other 20 performance rows still need code. `v74-p01` is the later markdown-layout release and stays pending even though `v54-p04` already drops render caches.

## Skip list

These rows are already `skipped` in the catalog. Do not implement them.

| Id | Why | Implement instead |
|---|---|---|
| `v52-f01` | 1.0.52 Rex reports a single process status. 1.0.63 replaces that shape. | `v63-f01` |
| `v55-x01`, `v56-x01`, `v57-x01`, `v58-x01` | The note is "Minor bug fixes." with no behavior. | nothing |
| `v71-f08`, `v71-f09`, `v73-f02` | Centered inter-agent rows. 1.0.74 replaces that layout. Folding is kept inside `v74-f01` per D-007. | `v74-f01` |

## Protected fork check (W16)

Status: `pending`. Run after W15, and sooner when a wave touches that behavior.

- [ ] `w16-copyable-errors` Copyable collapsed and expanded tool errors.
- [ ] `w16-turn-index` Turn index on plain user prompts only.
- [ ] `w16-sticky-edit` Sticky Edit path and its config flag.
- [ ] `w16-subagent-id` Subagent session id display and copy.
- [ ] `w16-hooks-at-start` Trusted plugin hooks register at session start.
- [ ] `w16-release-notes` Full `/release-notes` plus CDN prepend. Changelog file not rewritten as 1.0.78.
- [ ] `w16-copy-markdown` Ctrl+Shift+Y / F6 markdown copy.
- [ ] `w16-tool-step` Shift+Up / Shift+Down tool stepping, other chords unchanged.
- [ ] `w16-windows-link` Windows proto and stack changes still present in the build files.

## Summary

Refresh these numbers at the end of a session.

| Wave | Name | Pending | Skipped | Done | Already | Deferred |
|---|---|---:|---:|---:|---:|---:|
| W1 | Breaking changes | 0 | 0 | 5 | 0 | 1 |
| W2 | CLI surface | 0 | 0 | 7 | 0 | 0 |
| W3 | Permissions, sandbox, and config | 0 | 0 | 11 | 10 | 6 |
| W6 | Memory | 0 | 0 | 6 | 1 | 0 |
| W4 | Hooks | 0 | 0 | 24 | 10 | 2 |
| W5 | Workflows and goals | 0 | 0 | 23 | 6 | 0 |
| W7 | Worktrees | 0 | 0 | 38 | 3 | 0 |
| W8 | Sessions, resume, and dashboard | 0 | 0 | 41 | 2 | 0 |
| W9 | Prompt, mentions, and keyboard | 33 | 0 | 3 | 0 | 0 |
| W10 | Transcript, density, and copy | 45 | 0 | 0 | 0 | 0 |
| W11 | Headless mode and tools | 38 | 0 | 3 | 0 | 0 |
| W12 | MCP, web, images, and browser | 50 | 0 | 9 | 0 | 0 |
| W13 | Update, install, and sign-in | 22 | 0 | 5 | 0 | 0 |
| W14 | Pager and remaining bug fixes | 147 | 0 | 0 | 0 | 0 |
| W15 | Performance | 20 | 0 | 0 | 3 | 1 |
| SKIP | Not implemented | 0 | 8 | 0 | 0 | 0 |
| W16 | Fork regression | 9 | 0 | 0 | 0 | 0 |

Catalog rows: 583. Pending: 355. Skipped: 8. Done: 175. Already: 35. Deferred: 10.

**Next:** W9 prompt, mentions, and keyboard. First open row: Ctrl+\ and modified digits no longer select permission or cancel options (`v50-x36`).

## Waves

### W1 — Breaking changes

Pending: 0. Done: 5. Deferred: 1 (`v51-b01`, D-020). Look first: Share: `crates/codegen/xai-grok-shell/src/session/slash_commands.rs` (`"share"`) and `src/remote/client.rs` (`share_session`). Memory knobs: `crates/codegen/xai-grok-config-types/src/memory.rs` (`rollout`, `file_writes_enabled`, `compact_index_enabled`). Worktree remove: `xai-fast-worktree`. Folder trust: `xai-grok-shell/src/agent/folder_trust.rs` and `xai-grok-workspace/src/folder_trust.rs`. The fork already registers trusted plugin hooks at session start; keep that, and add the untrusted-folder wait on top.

### W2 — CLI surface

Pending: 0. Done: 7. Look first: Clap structs in `crates/codegen/xai-grok-pager-bin/src/main.rs`. Cursor worker already exists: `xai-grok-shell/src/agent/cursor_worker_config.rs` and the leader's `CursorWorkerStartArgs`. `grok usage` today prints session tokens. A plan-limit query that needs a background service not present in this tree is deferred, not invented.

### W3 — Permissions, sandbox, and config

Pending: 0. Done: 11. Already: 10. Deferred: 6 (D-026). Look first: `xai-grok-permission-rules`, `xai-grok-workspace/src/permission`, and config types under `xai-grok-config` / `xai-grok-config-types`. Auto-review is the name this tree already uses for auto mode.

### W6 — Memory

Pending: 0. Done: 6. Already: 1 (`v73-x12`, D-056). Look first: Do this after W1. Implementation lives in `xai-grok-memory` and the `/memory` pager view. Final behavior is an on/off switch plus the concise `MEMORY.md` index sorted by read count. A hidden Kitty image is already deleted with `a=d,d=i`.

### W4 — Hooks

Pending: 0. Done: 24. Already: 10. Deferred: 2 (`v51-x37`, `v52-x17`, D-041). Look first: Search `xai-grok-hooks` and the shell hook loader. `fail_closed` already exists for managed requirements; hook JSON `failClosed` is a different flag until you find it in the hook schema.

### W5 — Workflows and goals

Pending: 0. Done: 23. Already: 6. Look first: `xai-workflow` and the `/workflow` slash command. `/goal` already exists; the three-continuation pause is the new behavior. The goal chip opens the goal panel while workflow runs exist (`v47-x02`, D-042). The run roster lists agents outside a declared phase, counts only finished agents as done, and focuses an agent with Right or a click (`v53-x10`, `v72-f05`, D-045). Ctrl+C pauses an active goal, a disabled `/goal` line is a normal prompt, and a corrupt bundle manifest is refreshed (`v51-f28`, `v51-x110`, `v74-x01`, D-046). A pasted image on `/pr-workflow` is sent with the skill line (`v51-f27`, D-047). Reopening a chat keeps an active goal running (`v51-f30`, D-048). Quit names a running workflow and confirms with a second press (`v53-f05`, D-049). A headless goal that already completed does not print a later `Goal paused.` line (`v51-x92`, D-050). Inline tags stay on the sentence line, and `/workflow stop` prefers the live run (`v52-x23`, D-051). A finished workflow line ends with the run's result, `/tasks` shows the scheduled time, and an unknown task poll keeps an error row (`v74-x19`, D-052). Workflow running marks and a starting MCP server use `theme.running` (`v74-x31`, D-053). The question Enter button, the session popup `[✗]`, and a collapsed workflow row highlight under the pointer (`v75-x11`, D-054). A background subagent or an active workflow keeps the dashboard row Working, and stopping the row ends that work (`v75-x16`, D-055).

### W7 — Worktrees

Pending: 0. Done: 38 (`v51-x96`, with `v50-b01`, D-023; `v50-x30`, D-058; `v50-x50`, D-059; `v50-x51`, D-060; `v50-x53`, D-062; `v50-x57`, D-063; `v50-x58`, D-064; `v50-x62`, D-065; `v51-f13` and `v51-f15`, D-066; `v51-f16`, D-067; `v51-f26`, D-068; `v51-x34`, D-069; `v51-x42`, D-070; `v51-x55`, D-071; `v51-x56`, D-072; `v51-x59`, D-073; `v51-x68`, D-074; `v51-x79`, D-075; `v51-x98`, D-076; `v52-x06`, D-077; `v52-x11`, D-078; `v53-x01`, D-079; `v54-x06`, D-080; `v63-f02`, D-082; `v63-x03`, D-083; `v64-f01`, D-084; `v64-x01`, D-085; `v64-x04`, D-086; `v64-x05`, D-087; `v64-x06`, D-088; `v67-f01`, D-089; `v68-f01`, D-090; `v68-x01`, D-091; `v69-f04`, D-092; `v73-x08`, D-093; `v73-x13`, D-094; `v75-x01`, D-095). Already: 3 (`v50-f11`, D-057; `v50-x52`, D-061; `v59-x02`, D-081). Look first: Subcommands such as `detach` and `salvage` already exist. This wave is safety, Jujutsu, missing-folder reporting, and copy behavior, not a new command tree.

### W8 — Sessions, resume, and dashboard

Pending: 0. Done: 41 (`v51-f11`, D-096; `v51-f17`, D-097; `v51-x10`, D-098; `v51-x22`, D-099; `v51-x24`, D-100; `v51-x36`, D-101; `v51-x49`, D-102; `v51-x54`, D-103; `v51-x97`, D-104; `v51-x108`, D-105; `v59-f01`, D-106; `v60-x02`, D-107; `v61-f02`, D-108; `v61-x01`, D-109; `v62-x02`, D-110; `v64-f03`, D-111; `v65-f01`, D-112; `v66-x01`, D-113; `v69-f01`, D-115; `v69-f02`, D-116; `v73-f06`, D-118; `v74-x04`, D-119; `v74-x22`, D-120; `v74-x23`, D-121; `v75-x04`, D-122; `v77-x05`, D-123). Already: 2 (`v68-x02`, D-114; `v72-x04`, D-117). Look first: `/resume`, dashboard, and session list in `xai-grok-pager` and `xai-grok-shell` session storage. Title matching grows across 1.0.59, 1.0.60, 1.0.61, 1.0.65, and 1.0.69. Implement the latest rule: exact id, then separator-insensitive title, then fuzzy words, other folders included, current folder first.

### W9 — Prompt, mentions, and keyboard

Pending: 33. Done: 3 (`v50-f05`, D-124; `v50-f09`, D-125; `v50-x06`, D-126). Look first: `[ui].vim_mode` and `/vim-mode` already exist (`xai-grok-shared/src/ui_config.rs`, `xai-grok-pager-render/src/appearance/cache.rs`). Kitty protocol code is `xai-grok-pager-render/src/terminal/kitty_keyboard.rs`. Final vim setting is `[keys] vim = "off" | "nav" | "all"`, and the old boolean still loads. `@` picking lives in the pager prompt. `v63-f01` is the Rex OSC 7501 status (D-008); do not also build the skipped single-status `v52-f01`.

### W10 — Transcript, density, and copy

Pending: 45. Look first: `show_timestamps` is a bool today. Extend it to off / minimal / on; map the old `false` to off and `true` to on. `/density` is not in the slash list. Subagent rows: implement `v74-f01` (left-aligned). Do not build the centered rows skipped under D-007. Folding of consecutive inter-agent messages from the skipped 1.0.71 notes stays, unless it cannot fit the left-aligned row; record that choice.

### W11 — Headless mode and tools

Pending: 38. Done: 3. Look first: Headless entry points in `xai-grok-shell` and `grok agent stdio`. Tool implementations in `xai-grok-tools`. `--allowed-tools` is an alias to add next to the existing `--allowedTools` compat.

### W12 — MCP, web, images, and browser

Pending: 50. Done: 9. Look first: MCP crate `xai-grok-mcp`. Web and image tools in `xai-grok-tools`. Browser/Chrome state in the workspace server. `/imagine` is already a reserved slash name; the new behavior is the bundled-skill fallback when the tool is missing.

### W13 — Update, install, and sign-in

Pending: 22. Done: 5. Look first: `xai-grok-update` and login in `xai-grok-auth`. Installer scripts under `crates/codegen/xai-grok-pager/scripts/`. Do not replace a good local binary with an empty or failed download.

### W14 — Pager and remaining bug fixes

Pending: 147. Look first: Everything that did not match a feature wave. Work in version order. Many items are one behavior in `xai-grok-pager`. If an item clearly belongs in an earlier wave, move its wave field, record a decision, and leave it pending for that wave if the wave is already finished; otherwise do it now.

### W15 — Performance

Pending: 20. Already: 3 (`v54-p01`, `v54-p04`, `v69-p01`). Deferred: 1 (`v73-p01`, D-025). Look first: Performance notes. Confirm the cost with the existing code before adding caches. Skip a note whose hot path cannot be found and record that.

## Catalog

One row per release-note bullet. Edit the status word in place. `flag` is empty unless the row is a skip. `covered` points at the row to implement instead.

```
id | status | wave | kind | flag | covered | note
```

- v50-b01 | done | W1 | Breaking Changes |  |  | **grok worktree rm** without -f now refuses to delete worktrees that are in use or contain unsaved changes and exits with an error.
- v51-b01 | deferred | W1 | Breaking Changes |  |  | Cancel and edit-resend on long sessions no longer unexpectedly revert workspace files.
- v51-b02 | done | W1 | Breaking Changes |  |  | **Memory settings** simplified: rollout and file_writes_enabled options removed; memory is on or off.
- v51-b03 | done | W1 | Breaking Changes |  |  | **Removed the `/share` command** and related session sharing features from the CLI and SDK.
- v63-b01 | done | W1 | Breaking Changes |  |  | **Memory** always uses the concise `MEMORY.md` index of topic titles and paths, and `/memory` lists topics by how often Grok reads them. The `[memory_v2] compact_index_enabled` setting no longer has an effect.
- v64-b01 | done | W1 | Breaking Changes |  |  | **Untrusted folders** run only your organization's managed hooks. User, project, plugin, and agent hooks wait until you trust the folder. Headless sessions treat user hooks as trusted.
- v50-x09 | done | W2 | Bug Fixes |  |  | **/rewind** now places the cursor at the end of the restored prompt.
- v50-x12 | done | W2 | Bug Fixes |  |  | **Fixed --disallowed-tools** when only one scheduler tool is named; the other two no longer cause startup failure.
- v51-f01 | done | W2 | Features |  |  | **Inline viewports** can now be restored at a previously saved screen area without extra cursor queries.
- v51-x01 | done | W2 | Bug Fixes |  |  | Slash command suggestions no longer make the text after the cursor disappear while typing.
- v51-x117 | done | W2 | Bug Fixes |  |  | **disable_zdr_incompatible_tools** changes now apply to new turns without restarting the worker.
- v53-f03 | done | W2 | Features |  |  | **`--tools` and `--disallowed-tools`** also apply in the interactive TUI.
- v74-f10 | done | W2 | Features |  |  | **Picked `@` files in the prompt** can be edited in place: `Backspace` at the end of one reopens the picker on its path, and `Left` or `Right` puts the cursor inside it to change the path.
- v46-x04 | already | W3 | Bug Fixes |  |  | **Permission rules** with relative paths now correctly apply even when the cwd contains symlinks.
- v48-x02 | already | W3 | Bug Fixes |  |  | Custom model tables in config.toml now preserve the original model's notice banner.
- v50-x01 | deferred | W3 | Bug Fixes |  |  | **Shell permission decisions** now correctly prompt or approve commands matching the permission manager.
- v50-x08 | done | W3 | Bug Fixes |  |  | **Permission rules** now accept native tool names such as `run_terminal_command` and `Shell`.
- v50-x56 | already | W3 | Bug Fixes |  |  | Managed permission files with one bad key no longer lose all their valid rules.
- v50-x60 | already | W3 | Bug Fixes |  |  | **Permission rules** with one bad entry no longer lose the other rules in the same section.
- v51-f24 | already | W3 | Features |  |  | config.toml now accepts an [update] section without breaking other commands.
- v51-x04 | already | W3 | Bug Fixes |  |  | Always-approve mode now automatically approves web requests without extra prompts.
- v51-x27 | done | W3 | Bug Fixes |  |  | **Permission prompts** that fail now show a clearer message without blaming the interface.
- v51-x31 | deferred | W3 | Bug Fixes |  |  | Typing, arrows or Enter while an approval card appears no longer accidentally approve the action or enable always-approve mode.
- v51-x67 | already | W3 | Bug Fixes |  |  | **Typing while a permission or ask card is visible** no longer selects or answers the card.
- v51-x69 | deferred | W3 | Bug Fixes |  |  | **Auto mode** now asks before deleting files that the classifier has not already allowed.
- v51-x72 | done | W3 | Bug Fixes |  |  | **Settings saves** no longer re-insert the built-in default fork model into config.toml.
- v51-x75 | done | W3 | Bug Fixes |  |  | **/new** now inherits always-approve mode from the tab it was run from instead of another tab.
- v51-x88 | deferred | W3 | Bug Fixes |  |  | Delete operations now respect the same permission rules as shell rm commands.
- v52-x10 | already | W3 | Bug Fixes |  |  | Permission prompts now show which session needs approval.
- v52-x20 | done | W3 | Bug Fixes |  |  | An invalid shell_environment_policy table in config.toml now causes grok to exit with a clear error instead of silently using the full environment.
- v52-x25 | done | W3 | Bug Fixes |  |  | **Minimal mode** now shows the permission prompt instead of a hidden plan or cancel panel when both are present.
- v52-x28 | done | W3 | Bug Fixes |  |  | **Unrecognized permission keys** or [permissions] sections now appear as skipped entries in grok inspect.
- v52-x30 | deferred | W3 | Bug Fixes |  |  | Shell redirects to wildcard or variable paths now correctly respect deny rules in Auto mode.
- v53-x22 | already | W3 | Bug Fixes |  |  | **`grok inspect`** warns when a known `[features]` key in config.toml has a non-boolean value.
- v64-f04 | already | W3 | Features |  |  | **Auto mode** runs `make --version`, `cmake --version`, and `just --version` without asking.
- v65-f02 | done | W3 | Features |  |  | **`--permission-mode plan`** starts the session in plan mode, as documented, instead of the default mode.
- v74-x21 | deferred | W3 | Bug Fixes |  |  | **macOS**: the TUI no longer freezes when it runs inside a sandbox that blocks the window server.
- v75-f01 | done | W3 | Features |  |  | **Permission rules that don't load**, such as a rule that doesn't parse or a `.claude/settings.json` that isn't valid JSON, show a warning when a session opens instead of being skipped silently.
- v75-x19 | done | W3 | Bug Fixes |  |  | **Turning a skill on or off** in the Skills tab saves only `[skills].disabled` to `config.toml`, so an organization's managed skill folders and ignore list keep applying.
- v78-x01 | done | W3 | Bug Fixes |  |  | **Always-approve help text** now correctly says the setting is saved to config.
- v50-x22 | done | W6 | Bug Fixes |  |  | **web_fetch** no longer downloads huge files into memory before rejecting them.
- v50-x23 | done | W6 | Bug Fixes |  |  | **memory_get** no longer returns huge files that exceed the read limit.
- v50-x25 | done | W6 | Bug Fixes |  |  | **memory_search** now finds Japanese, Chinese, and Korean words inside longer notes.
- v51-x100 | done | W6 | Bug Fixes |  |  | The model is now told when memory is turned off or on for the session.
- v54-x08 | done | W6 | Bug Fixes |  |  | **Memory search** shows the matching part of a long line instead of cutting it off before the match.
- v54-x09 | done | W6 | Bug Fixes |  |  | **`/memory`** skips a symlinked note instead of failing the whole listing.
- v73-x12 | already | W6 | Bug Fixes |  |  | **Kitty graphics protocol**: images Grok hides are freed in the terminal, so they no longer use its memory.
- v49-x01 | already | W4 | Bug Fixes |  |  | Hook permission prompts now appear regardless of approval mode.
- v50-x18 | done | W4 | Bug Fixes |  |  | Custom notification commands defined under [[ui.notifications.hooks]] now execute on matching events.
- v51-f14 | done | W4 | Features |  |  | Bare hook commands such as `my-guard` now run from PATH when not found next to the hook file.
- v51-f23 | already | W4 | Features |  |  | Hooks that ask about web searches or file deletes now show the hook name on the approval card.
- v51-f25 | done | W4 | Features |  |  | Hook load errors listed in /hooks can now be expanded to see the full message.
- v51-x37 | deferred | W4 | Bug Fixes |  |  | **Large parallel shell batches** no longer drop calls with "hook state lost" errors.
- v51-x40 | already | W4 | Bug Fixes |  |  | **PreToolUse hooks** now validate rewrites against the tool schema and deny invalid ones before running the tool.
- v51-x44 | done | W4 | Bug Fixes |  |  | Web search hooks now fire under the correct `web_search` name and carry `query`.
- v51-x46 | done | W4 | Bug Fixes |  |  | Glob tool hooks now match under `list_dir` and carry the correct directory.
- v51-x57 | done | W4 | Bug Fixes |  |  | Hook scripts that read stdin with `read` now receive a complete line.
- v51-x58 | done | W4 | Bug Fixes |  |  | A policy that disables non-managed hooks now applies to the next session instead of only after restart.
- v51-x60 | done | W4 | Bug Fixes |  |  | A hook listed under two different matchers now runs only once per tool call.
- v51-x63 | done | W4 | Bug Fixes |  |  | **Tool hooks** now run one at a time per session and stop promptly on cancel or exit.
- v51-x64 | already | W4 | Bug Fixes |  |  | **PostToolUse hooks** now receive a correct whole-millisecond duration.
- v51-x65 | already | W4 | Bug Fixes |  |  | **PreToolUse hooks** for shell commands now see the model's block_until_ms value.
- v51-x66 | already | W4 | Bug Fixes |  |  | **MCP tool calls** now show hook permission prompts and cards name the hook that asked.
- v51-x80 | already | W4 | Bug Fixes |  |  | A prompt blocked by a hook no longer titles the session.
- v51-x89 | done | W4 | Bug Fixes |  |  | Hooks can no longer rewrite a web-search call to restrict domains.
- v51-x91 | done | W4 | Bug Fixes |  |  | Notification hooks for permission prompts and task completion now run reliably before a headless session exits.
- v51-x101 | already | W4 | Bug Fixes |  |  | Hook results in the UI now correctly distinguish between notes and tool outcomes.
- v51-x105 | already | W4 | Bug Fixes |  |  | Hooks and tool commands started from the TUI can no longer read or write the interactive terminal.
- v51-x115 | done | W4 | Bug Fixes |  |  | SubagentStart and Notification hook activity is no longer reported to the UI.
- v52-f03 | done | W4 | Features |  |  | Project hooks placed in a repository subfolder are now loaded when you launch inside that folder.
- v52-f04 | done | W4 | Features |  |  | HTTP hooks to localhost now work, and bad hook URLs are reported at load time.
- v52-x17 | deferred | W4 | Bug Fixes |  |  | Long tool turns that include PostToolUse hook output now compact correctly instead of hitting the prompt length limit.
- v52-x19 | done | W4 | Bug Fixes |  |  | When a permission rule blocks a tool call, any context returned by a PreToolUse hook still reaches the model.
- v52-x33 | already | W4 | Bug Fixes |  |  | StopFailure hook matchers containing commas or spaces are now treated as regex patterns.
- v53-f02 | done | W4 | Features |  |  | **`/hooks`** lists load errors from the main agent's inline hooks and from plugins.
- v53-x15 | done | W4 | Bug Fixes |  |  | **Hooks you disabled** stay disabled when the disabled-hooks file cannot be read.
- v59-x03 | done | W4 | Bug Fixes |  |  | **Advanced PowerShell inline hooks** that read `$input` run with a warning instead of failing.
- v59-x04 | done | W4 | Bug Fixes |  |  | **Hooks** that return `continue: false` from PreToolUse, PostToolUse, or UserPromptSubmit stop the turn instead of being ignored.
- v61-x03 | done | W4 | Bug Fixes |  |  | **Hook spawn failures** on Windows name the program that failed to start, such as PowerShell or Git Bash, instead of blaming the hook's script.
- v63-f03 | done | W4 | Features |  |  | **Command hooks** accept an `args` list: Grok runs `command` as one program with those arguments and no shell, and fills hook variables such as `$GROK_SESSION_ID` in each argument.
- v67-x03 | done | W4 | Bug Fixes |  |  | **Quitting a session with hooks configured** is faster: the session-end `Stop` hook runs at the same time as the `SessionEnd` hooks and gets at most 1.5 seconds.
- v71-f04 | done | W4 | Features |  |  | **Hooks** accept `"failClosed": true`: when such a `PreToolUse` or `UserPromptSubmit` hook times out or crashes, the tool call is denied or the prompt is blocked instead of going ahead.
- v75-x18 | done | W4 | Bug Fixes |  |  | **Disabling a project's hook** in `/hooks` applies only to that repository, instead of also skipping hooks with the same name in other repositories.
- v47-x02 | done | W5 | Bug Fixes |  |  | **Clicking the Goal chip** now opens the goal panel even when workflow runs are present.
- v50-x28 | done | W5 | Bug Fixes |  |  | A workflow file that fails to parse now reports the exact file and parse error instead of 'unknown workflow'.
- v51-f02 | done | W5 | Features |  |  | Headless `grok -p` now waits for background goals to finish and reports their outcome.
- v51-f19 | done | W5 | Features |  |  | **--background-wait-timeout** now also stops an active headless goal after the timeout and exits with an error.
- v51-f21 | already | W5 | Features |  |  | **New `/workflow` commands** let you list, stop, and resume background workflow runs without sending them to the model.
- v51-f22 | done | W5 | Features |  |  | **Background workflow runs** now appear in the tasks pane with a stop button.
- v51-f27 | done | W5 | Features |  |  | Images pasted with skill commands like /pr-workflow are now sent to the model.
- v51-f28 | already | W5 | Features |  |  | Pressing Ctrl+C on a goal now pauses the goal instead of letting it continue.
- v51-f30 | done | W5 | Features |  |  | Reopening a chat with an active goal now continues the goal automatically.
- v51-x02 | already | W5 | Bug Fixes |  |  | Running workflows in the dock now show the same pulsing indicator as in the main scrollback.
- v51-x86 | done | W5 | Bug Fixes |  |  | **/workflow stop|resume <name>** now correctly targets the live run instead of an ended first run.
- v51-x92 | done | W5 | Bug Fixes |  |  | Headless goal runs no longer print a spurious "Goal paused" message after completion.
- v51-x110 | done | W5 | Bug Fixes |  |  | With goals disabled, a headless `/goal` line is now treated as a normal prompt instead of failing.
- v52-x23 | already | W5 | Bug Fixes |  |  | Inline tags in agent replies no longer break line layout, and /workflow stop now picks the correct live run.
- v53-f04 | done | W5 | Features |  |  | **`/goal`** pauses after three continuations in a row with no tool call; run `/goal resume` to continue.
- v53-f05 | done | W5 | Features |  |  | **Quitting** warns when it would stop a running workflow and asks you to confirm with a second quit.
- v53-x09 | done | W5 | Bug Fixes |  |  | **`/workflow stop`** and **`/workflow pause`** no longer pick a run from a loose fuzzy match, and headless `-p` runs print each refused approval request to stderr.
- v53-x10 | done | W5 | Bug Fixes |  |  | **Workflow run views** list agents outside a declared phase and count only finished agents as done.
- v61-x02 | done | W5 | Bug Fixes |  |  | **Workflow run views** scroll toward newer agents when you wheel down the agent roster, instead of snapping back.
- v63-x06 | done | W5 | Bug Fixes |  |  | **Workflow run views** hide each agent's live status text on narrower terminals, so agent names are no longer cut off.
- v72-f05 | done | W5 | Features |  |  | **`/workflow runs`** detail view: `→` or a click focuses a phase's agent list, `↑`/`↓` pick an agent, `Enter` opens it, and `←` returns to the phases.
- v74-x01 | already | W5 | Bug Fixes |  |  | **Built-in skills, agents and workflows** are refreshed when their local cache's manifest is missing or corrupt, instead of staying outdated or failing every sync.
- v74-x19 | done | W5 | Bug Fixes |  |  | **Workflow run lines** in the transcript end with what the run returned instead of a bare colon, `/tasks` shows the time it was taken, and polling an unknown task id leaves a row with its error.
- v74-x24 | done | W5 | Bug Fixes |  |  | **Windows**: on older consoles such as plain `cmd.exe` and Windows PowerShell windows, status marks and selection arrows in workflow views, the dashboard, and the feedback and extensions dialogs show as `>`, `•`, `√` or `x` instead of empty boxes.
- v74-x31 | done | W5 | Bug Fixes |  |  | **Running markers** in workflow views and MCP servers that are still starting use the theme's running color, like other progress indicators.
- v75-f08 | already | W5 | Features |  |  | **Workflow status lines** say how many agents are still running instead of how many exist.
- v75-x05 | done | W5 | Bug Fixes |  |  | **Windows**: on older consoles such as plain `cmd.exe` and Windows PowerShell windows, the `▶`, `⏸` and `◌` marks in todo, workflow and list rows show as characters the console can draw instead of empty boxes.
- v75-x11 | done | W5 | Bug Fixes |  |  | **A question card's `Enter:submit` button**, the dashboard session popup's `[✗]` and collapsed workflow rows highlight under the mouse pointer like other clickable controls.
- v75-x16 | done | W5 | Bug Fixes |  |  | **Dashboard rows** stay in Working while a session's background subagents or workflow runs are going, and stopping the row ends them.
- v50-f11 | already | W7 | Features |  |  | Dashboard now shows which worktree each session belongs to, displays the `-m` model in the new-agent box, and lets you toggle worktree mode in the folder picker.
- v50-x30 | done | W7 | Bug Fixes |  |  | **Image and log paths** reported after `/fork --worktree` now point to real files instead of missing locations.
- v50-x50 | done | W7 | Bug Fixes |  |  | **Git-mode worktrees created with -w** now preserve uncommitted changes instead of dropping them.
- v50-x51 | done | W7 | Bug Fixes |  |  | **Worktrees created with --worktree-ref** no longer contain wrong submodule contents or LFS pointer text.
- v50-x52 | already | W7 | Bug Fixes |  |  | Worktree creation now shows the same clear error messages on every code path.
- v50-x53 | done | W7 | Bug Fixes |  |  | Failed worktree creates now show a clear error on the home screen instead of being hidden.
- v50-x57 | done | W7 | Bug Fixes |  |  | Simultaneous worktree creates with the same label now succeed instead of one failing with "already exists".
- v50-x58 | done | W7 | Bug Fixes |  |  | `grok worktree rm` without `-f` no longer deletes arbitrary folders that happen to share a name with a removed worktree.
- v50-x62 | done | W7 | Bug Fixes |  |  | **grok worktree rm** by label now refuses when the label exists in multiple repos.
- v51-f13 | done | W7 | Features |  |  | **grok -p --output-format json** now reports the exact worktree folder used (useful after -w name collisions).
- v51-f15 | done | W7 | Features |  |  | Headless JSON error output now includes the worktree folder when a session was started.
- v51-f16 | done | W7 | Features |  |  | **grok sessions list** and /resume now show worktree sessions started from the same subfolder.
- v51-f26 | done | W7 | Features |  |  | `grok worktree` commands now work correctly inside Jujutsu repositories.
- v51-x34 | done | W7 | Bug Fixes |  |  | **Worktree creation** no longer hangs when the repo contains named pipes or sockets.
- v51-x42 | done | W7 | Bug Fixes |  |  | **grok -w** now opens the session at the worktree root (with a note) when the original launch folder does not exist in the new worktree.
- v51-x55 | done | W7 | Bug Fixes |  |  | **Worktrees created with -w** are now correctly tracked and resumable even when GROK_HOME is reached through a symlink.
- v51-x56 | done | W7 | Bug Fixes |  |  | `grok worktree rm` and `gc` now clean up names after manually deleting a worktree folder so `-w NAME` works again.
- v51-x59 | done | W7 | Bug Fixes |  |  | `/resume` and `grok sessions list` no longer show worktree sessions from a sibling submodule or separate-git-dir repo.
- v51-x68 | done | W7 | Bug Fixes |  |  | **grok worktree list** now shows unfinished copies left by an interrupted create.
- v51-x79 | done | W7 | Bug Fixes |  |  | **Worktree db rebuild** now restores names so `rm` and `show` by name work again.
- v51-x96 | done | W7 | Bug Fixes |  | v50-b01 | `grok worktree rm` without `-f` now safely keeps worktrees when the unsaved-work check fails or times out.
- v51-x98 | done | W7 | Bug Fixes |  |  | `grok worktree rm` and `gc` now correctly remove worktrees whose only untracked content is empty folders.
- v52-x06 | done | W7 | Bug Fixes |  |  | `grok worktree rm` and `gc` no longer delete a worktree that an open session is using.
- v52-x11 | done | W7 | Bug Fixes |  |  | **grok worktree rm** now refuses to delete git repositories, .git folders, or any directory grok did not create.
- v53-x01 | done | W7 | Bug Fixes |  |  | **Worktrees created from a repository subfolder** report the repository root as their source in `grok worktree list` and `grok worktree show`.
- v54-x06 | done | W7 | Bug Fixes |  |  | **Worktree forks** (`grok -w -r` or `/fork` into a worktree) run in and show the worktree's path, so a shell command no longer edits the original checkout.
- v59-x02 | already | W7 | Bug Fixes |  |  | **`grok worktree rm`** no longer suggests `-f` for a worktree that `-f` would also refuse to remove.
- v63-f02 | done | W7 | Features |  |  | **`grok worktree list` and `grok worktree show`** mark a worktree whose folder is missing or whose source repository moved, with a hint on how to repair it.
- v63-x03 | done | W7 | Bug Fixes |  |  | **`grok -w` and `Ctrl+W`** explain that no worktree was made when you run them outside a git repository.
- v64-f01 | done | W7 | Features |  |  | **Worktree names** typed with `Ctrl+W` keep Unicode letters such as `café`, and a name or ref that starts with `-` is refused.
- v64-x01 | done | W7 | Bug Fixes |  |  | **New worktrees** keep tracked files that match `.gitignore`, and leave out files excluded only by `.git/info/exclude` or `core.excludesFile`.
- v64-x04 | done | W7 | Bug Fixes |  |  | **`grok -w`, `grok worktree create`, worktree forks, and subagent worktrees** refuse up front with the sandbox's message when the sandbox profile does not allow creating worktrees.
- v64-x05 | done | W7 | Bug Fixes |  |  | **Worktree options** are hidden in `/fork`, `/new`, and the Welcome screen when the sandbox profile does not allow creating worktrees.
- v64-x06 | done | W7 | Bug Fixes |  |  | **A worktree name you type** that is already taken is refused instead of getting a numbered suffix, and a name whose folder you deleted by hand can be used again.
- v67-f01 | done | W7 | Features |  |  | **Worktrees** also use their main checkout's saved "always allow" answers, so you are not asked again for a tool you already allowed there. Set `[features] worktree_inherits_main_approvals = false` to turn this off; the main checkout's "never allow" answers apply either way.
- v68-f01 | done | W7 | Features |  |  | **Worktree create and fork** now list files that could not be copied and any mode changes that were lost.
- v68-x01 | done | W7 | Bug Fixes |  |  | **Worktree create** now fails cleanly instead of falling back when it cannot copy tracked files or runs out of disk space.
- v69-f04 | done | W7 | Features |  |  | **Welcome menu** now lists Resume session (`Ctrl+R`) above New worktree (`Ctrl+W`).
- v73-x08 | done | W7 | Bug Fixes |  |  | **Worktree creation notes** stay on screen when a queued prompt starts.
- v73-x13 | done | W7 | Bug Fixes |  |  | **`--restore-code`** restores its own stash even when another worktree changes the stash list meanwhile, and keeps your changes stashed when a post-checkout hook fails after the checkout.
- v75-x01 | done | W7 | Bug Fixes |  |  | **Notes above a queued prompt**, such as a model switch result or a ready worktree, stay on screen when the prompt starts instead of scrolling out of view.
- v51-f11 | done | W8 | Features |  |  | **Pinned sessions** on the dashboard now stay in the order you pinned or reordered them.
- v51-f17 | done | W8 | Features |  |  | **/exit** (and aliases) in a dashboard details view now closes only that session instead of quitting Grok.
- v51-x10 | done | W8 | Bug Fixes |  |  | Foreground subagent activity now appears correctly after resume.
- v51-x22 | done | W8 | Bug Fixes |  |  | **Dashboard cursor and overlay counter** now correctly track the active grouping view.
- v51-x24 | done | W8 | Bug Fixes |  |  | **Session picker footer** no longer lists the same shortcut twice.
- v51-x36 | done | W8 | Bug Fixes |  |  | **Resumed sessions** now correctly show aborted subagent tasks as cancelled instead of failed.
- v51-x49 | done | W8 | Bug Fixes |  |  | Pinned dashboard sessions now survive restarts and layout changes instead of disappearing from the saved list.
- v51-x54 | done | W8 | Bug Fixes |  |  | **Number-key picks in the dashboard peek** no longer leak to the wrong agent after Up/Down navigation.
- v51-x97 | done | W8 | Bug Fixes |  |  | The dashboard slash menu now shows skills from the home session even when opened early.
- v51-x108 | done | W8 | Bug Fixes |  |  | Closing a session in the dashboard no longer leaves it listed under Inactive.
- v52-x31 | done | W8 | Bug Fixes |  |  | The in-session resume picker now matches the ungrouped layout shown on the welcome screen when configured.
- v53-x13 | done | W8 | Bug Fixes |  |  | **`grok sessions list`** and `/resume` keep working when one session directory is unreadable.
- v54-f01 | done | W8 | Features |  |  | **`/resume`** takes an optional session id or title and opens the picker filtered to matching sessions.
- v59-f01 | done | W8 | Features |  |  | **Dashboard `/loop`** without a prompt shows its usage instead of starting a new session.
- v59-x01 | done | W8 | Bug Fixes |  |  | **Dashboard peek replies** waiting on a clipboard paste are canceled with a toast when you move the peek to another row or press Esc, instead of being sent later.
- v60-f01 | done | W8 | Features |  |  | **`grok --resume <title>`** lists matching sessions newest first with when each was last updated, and suggests similar titles when nothing matches.
- v60-x02 | done | W8 | Bug Fixes |  |  | **Esc** on the first question in a session opened from the dashboard moves focus to the scrollback, as it does in a standalone session, instead of returning to the dashboard.
- v61-f01 | done | W8 | Features |  |  | **Resume picker** finds sessions whose titles contain every word you type, in any order.
- v61-f02 | done | W8 | Features |  |  | **`/resume <id or name>`** opens the session directly when the ID or title matches exactly one session, instead of always opening the picker.
- v61-x01 | done | W8 | Bug Fixes |  |  | **Dashboard peek** text can be selected by dragging over it with the mouse, and the selection is copied to the clipboard when you release.
- v62-x02 | done | W8 | Bug Fixes |  |  | **Cold `/resume` from the dashboard overlay** keeps the overlay attached, so you can still back out with Left or `Ctrl+\`.
- v62-x04 | done | W8 | Bug Fixes |  |  | **CLI commands** such as `grok models`, `grok sessions` and `grok export` no longer crash when their output pipe closes early, such as when piped to `head`, or when the output device is full.
- v64-f03 | done | W8 | Features |  |  | **`/resume`** loads older sessions when you reach the last row, and shows how many sessions are loaded.
- v65-f01 | done | W8 | Features |  |  | **`/resume` search** finds sessions from other folders, tolerates typos, and also matches the commands, file paths, and replies in a session.
- v65-f03 | done | W8 | Features |  |  | **The dashboard's `Ctrl+L` folder picker** lists up to 50 recent folders instead of 10.
- v65-x01 | done | W8 | Bug Fixes |  |  | **`/resume` search** keeps a single typed character literal and matches each typed word at the start of a word, so `log fix` no longer matches "Blog prefix cleanup".
- v66-x01 | done | W8 | Bug Fixes |  |  | **`/theme`** restores your previous theme when you leave the picker any way other than `Enter`, such as `Esc`, `Ctrl+U`, `Tab` then `Esc`, or opening Settings or the dashboard.
- v68-f05 | done | W8 | Features |  |  | **Change-directory picker** (`Ctrl+L` on the dashboard) opens with the path field unfocused, like the `/resume` search.
- v68-x02 | already | W8 | Bug Fixes |  |  | **Running Grok sessions** are no longer broken when an update deletes their binary.
- v68-x03 | done | W8 | Bug Fixes |  |  | **Dashboard model picks** now keep their selected context window when spawning the next session.
- v69-f01 | done | W8 | Features |  |  | **Dashboard directory grouping** now shows a header with the folder path and agent count above each folder's agents.
- v69-f02 | done | W8 | Features |  |  | **Dashboard location** (branch and folder) now sits on the dispatch box's top border instead of its own header row.
- v69-f03 | done | W8 | Features |  |  | **`/resume` on the dashboard** now lists sessions from every folder, with the current folder and its checkouts first.
- v72-x04 | already | W8 | Bug Fixes |  |  | **Dashboard** shows the new folder on the dispatch box's top border right after a `Ctrl+L` or `/cd` pick.
- v73-f06 | done | W8 | Features |  |  | **Start in multiline** in `/settings` (`[ui] multiline_default`) starts new sessions and the dashboard in multiline mode, where `Enter` inserts a newline.
- v74-x04 | done | W8 | Bug Fixes |  |  | **Resumed sessions** show a background command's completion in the turn that ran it, with its real run time instead of about 0.0s.
- v74-x22 | done | W8 | Bug Fixes |  |  | **`grok -p --resume`** shows why a session couldn't load instead of always "Session does not exist", and with `--output-format json` a resume, continue or fork that fails before the session opens prints one JSON error.
- v74-x23 | done | W8 | Bug Fixes |  |  | **Sessions with an unrecognized reasoning effort or unreadable bytes in their history** resume instead of failing: the session runs at the model's default effort and says so once, and the unreadable lines are left out of the transcript.
- v74-x29 | done | W8 | Bug Fixes |  |  | **Dashboard**: the peek header reads `Responding` while a reply streams, and the `Esc` footer hint reads `new agent` like the other lowercase hints.
- v75-f06 | done | W8 | Features |  |  | **`Left` on an empty prompt** opens the dashboard from any session or the welcome screen, not only from a session you opened from the dashboard.
- v75-x04 | done | W8 | Bug Fixes |  |  | **`grok --resume <id>`** opens the session in the folder it was recorded in, wherever you launch Grok from, instead of refusing.
- v75-x15 | done | W8 | Bug Fixes |  |  | **`grok sessions delete`** no longer fails with "Directory not empty" when another process is still writing into the session's folder.
- v77-x05 | done | W8 | Bug Fixes |  |  | Rows in the `/resume` list now expand and collapse the same way they do inside a session.
- v50-f05 | done | W9 | Features |  |  | **Compact mode** now hides the dock (or tasks pane) the same way Ctrl+G does.
- v50-f09 | done | W9 | Features |  |  | **Command palette (Ctrl+P)** and model picker now open at the same width as the settings modal.
- v50-x06 | done | W9 | Bug Fixes |  |  | **Queue pane** and Ctrl+; now appear and work when the dock is hidden with Ctrl+G.
- v50-x36 | pending | W9 | Bug Fixes |  |  | **Ctrl+\** and other modified digits no longer accidentally select options on permission or cancel cards.
- v51-f12 | pending | W9 | Features |  |  | **Shift+Enter, Ctrl+., Ctrl+/ and Ctrl+[ / ]** now work inside tmux when extended-keys are enabled.
- v51-x11 | pending | W9 | Bug Fixes |  |  | Very large pasted prompts are now offloaded to a file instead of being sent inline.
- v51-x14 | pending | W9 | Bug Fixes |  |  | The send-now shortcut no longer accidentally enables always-approve mode while a permission card is open.
- v51-x19 | pending | W9 | Bug Fixes |  |  | Send-now (Ctrl+Enter) no longer accidentally answers an open question card.
- v51-x29 | pending | W9 | Bug Fixes |  |  | Long pastes without bracketed-paste markers are now read completely instead of stalling after ~1 KB.
- v51-x62 | pending | W9 | Bug Fixes |  |  | **@-file completion** no longer accepts a match from an earlier query when you type quickly.
- v51-x78 | pending | W9 | Bug Fixes |  |  | **@ file search** no longer accepts keys or moves the highlight while results are still updating.
- v52-f02 | pending | W9 | Features |  |  | **Tool call density improvements**: expanded tool call groups now stay expanded as new calls join them, and Ctrl+E expands or collapses thinking faster in long transcripts.
- v54-x14 | pending | W9 | Bug Fixes |  |  | **Send now (Ctrl+Enter)** on a slash command runs or queues it as a command instead of sending it as a prompt.
- v63-x01 | pending | W9 | Bug Fixes |  |  | **Plan reviews** that open while you are typing keep your text in the review's notes, so a typed letter no longer triggers a plan shortcut such as approve.
- v63-f01 | pending | W9 | Features |  |  | **Rex terminal support** reports each open session and each running subagent as its own record, so Rex shows what each one is doing. Set `children = false` under `[ui.notifications.program_status]` to report only the main status.
- v67-f02 | pending | W9 | Features |  |  | **Prompt suggestions** follow vim mode: with `[ui].vim_mode` on, `Tab` moves focus to the scrollback instead of accepting the suggestion. `→` or `Ctrl+→` accepts a suggestion, and `Esc` dismisses it.
- v68-f02 | pending | W9 | Features |  |  | **Vim navigation** is now one setting, `[keys] vim` (`off` or `nav`), also set with `/vim-mode`; the older `[ui] vim_mode` still works.
- v71-f01 | pending | W9 | Features |  |  | **`ui.single_ctrl_c_quit = true`** lets a single `Ctrl+C` quit when no draft or unfinished work would be lost. It is also under Editor in `/settings`; the default still asks for a second press.
- v71-f05 | pending | W9 | Features |  |  | **Vim editing in the prompt** (experimental): set `[keys] vim = "all"` in `config.toml` for Insert and Normal modes with vim motions, operators, counts and undo. The prompt's info line shows `NORMAL` or `INSERT`.
- v71-f10 | pending | W9 | Features |  |  | **Prompt vim cursor** is a block in Normal mode and a bar in Insert mode; `ui.cursor_blink` sets whether they blink.
- v71-f11 | pending | W9 | Features |  |  | **Minimal mode** starts its info row with the prompt's vim mode (`NORMAL` or `INSERT`), and `NORMAL` is shown in bold.
- v73-x05 | pending | W9 | Bug Fixes |  |  | **Windows**: shortcuts typed with `Shift`-held symbols or AZERTY digit keys work.
- v74-f02 | pending | W9 | Features |  |  | **Shift+Enter** inserts a newline in VS Code and other xterm.js-based terminals that use the Kitty keyboard protocol.
- v74-f05 | pending | W9 | Features |  |  | **Shell approval cards** say when Auto-review blocked a command or couldn't review it, and `Ctrl+F` shows the reviewer's explanation.
- v74-x02 | pending | W9 | Bug Fixes |  |  | **Ctrl+Y** on a question card no longer closes it without an answer; while you type an answer, it pastes back text cut with `Ctrl+K`.
- v74-x27 | pending | W9 | Bug Fixes |  |  | **Dialog tab labels, shortcuts and settings rows** that don't fit end in `…` instead of being cut off.
- v75-f02 | pending | W9 | Features |  |  | **`Cmd+Up`/`Cmd+Down` and `Ctrl+Home`/`Ctrl+End`** move the cursor to the start or end of the whole prompt, and with `Shift` they select to there.
- v75-f03 | pending | W9 | Features |  |  | **Send now** shows as `Enter Enter` in the footer where `Ctrl+Enter` arrives as plain `Enter`, such as inside tmux with its default settings.
- v75-f09 | pending | W9 | Features |  |  | **`Ctrl+L`** in the scrollback adds the selected message or highlighted text to the prompt as a quote; open Extensions with `/plugins` instead.
- v75-x10 | pending | W9 | Bug Fixes |  |  | **16-color terminals** show the paste label and tool accents in the muted gray other secondary text uses, and the paste label's brackets no longer vanish on the light theme.
- v75-x14 | pending | W9 | Bug Fixes |  |  | **Typing a digit or symbol** while the scrollback has focus, with vim keys off, moves focus to the prompt and types it there, as letters already do; `?` still opens the command palette.
- v76-f01 | pending | W9 | Features |  |  | **Ctrl+L** redraws the screen after a terminal clear without flashing.
- v77-f01 | pending | W9 | Features |  |  | **New [ui] kitty_keyboard = "query"** setting lets Shift+Enter insert newlines in unidentified terminals.
- v77-x03 | pending | W9 | Bug Fixes |  |  | **Ctrl+L** now works on more screens and copy hints match what actually copies to the clipboard.
- v78-f01 | pending | W9 | Features |  |  | **@ file chips** can now be edited in place; typed @~/ paths resolve to the home folder.
- v78-f03 | pending | W9 | Features |  |  | **@ mentions** now support :N line suffixes and @!hidden files.
- v50-f01 | pending | W10 | Features |  |  | **Compact mode** now collapses recaps and keeps inner spacing on tall terminals.
- v50-f06 | pending | W10 | Features |  |  | New **timestamps** setting offers off, minimal, and on modes in Settings and `/timestamps`.
- v50-f10 | pending | W10 | Features |  |  | Tables rendered by grok now show copy buttons underneath that export clean TSV, CSV or Markdown suitable for spreadsheets.
- v50-x04 | pending | W10 | Bug Fixes |  |  | **Em-dashes** no longer appear in status messages and subagent rows.
- v50-x24 | pending | W10 | Bug Fixes |  |  | **Continue to run** no longer kills commands running inside background subagents.
- v51-f04 | pending | W10 | Features |  |  | **New `/density` command** and Tool call density setting let you choose how much detail tool calls show in the transcript.
- v51-f18 | pending | W10 | Features |  |  | **Editors offering file access** now receive a one-time transcript notice that the agent reads and writes disk files.
- v51-x03 | pending | W10 | Bug Fixes |  |  | Mode switch tool calls now appear in the transcript with the target mode name.
- v51-x28 | pending | W10 | Bug Fixes |  |  | **--disallowed-tools** now accepts the exact names shown to the model (run_terminal_command, spawn_subagent, etc.).
- v51-x35 | pending | W10 | Bug Fixes |  |  | **--no-subagents** flag now works correctly in headless / -p sessions.
- v51-x43 | pending | W10 | Bug Fixes |  |  | **`/cd`** and the location picker now accept quoted or escaped folder paths.
- v51-x81 | pending | W10 | Bug Fixes |  |  | **Background subagents** now respect saved approvals and --always-approve after the parent turn finishes.
- v51-x93 | pending | W10 | Bug Fixes |  |  | Legacy UI density settings now combine correctly across config tiers and produce expected folding.
- v51-x104 | pending | W10 | Bug Fixes |  |  | Scroll logging and acceleration calculations no longer produce backwards timestamps under load.
- v52-x05 | pending | W10 | Bug Fixes |  |  | Approval cards for background subagents no longer disappear when the parent turn ends.
- v52-x14 | pending | W10 | Bug Fixes |  |  | **Wrapped URLs** in prompts and quotes now link to the complete address instead of being cut at the wrap.
- v53-f01 | pending | W10 | Features |  |  | **Wrapped shell commands** in tool and background-task rows copy back as one line.
- v59-f03 | pending | W10 | Features |  |  | **`[Copy]` buttons** appear under code blocks in finished replies, labeled with the code's language or file name.
- v59-x06 | pending | W10 | Bug Fixes |  |  | **Copy rows** under code blocks, tables, and diagrams display correctly after switching between minimal and fullscreen modes.
- v60-f02 | pending | W10 | Features |  |  | **Shift+Y** in the fullscreen markdown viewer copies the code block under the cursor.
- v67-x01 | pending | W10 | Bug Fixes |  |  | **Headless `grok -p` runs** exit when their work is done instead of hanging when a background task or subagent finishes and its output is read in the same turn.
- v67-x02 | pending | W10 | Bug Fixes |  |  | **The theme picker in Settings** restores your previous theme when you leave it any way other than `Enter`, as `/theme` already does.
- v68-f03 | pending | W10 | Features |  |  | **Terminal theme** now draws a subtle band behind your prompts using the colors your terminal reports.
- v70-f01 | pending | W10 | Features |  |  | **Answered questions** now have a blank line between them, except in compact mode (`/compact-mode`).
- v72-f02 | pending | W10 | Features |  |  | **Subagent and background task ids** mentioned in the transcript open that subagent's view or the task's output on click.
- v72-f04 | pending | W10 | Features |  |  | **Subagents named in a reply** show as the subagent's label instead of its raw id once the reply finishes, as a link that opens its transcript.
- v73-f03 | pending | W10 | Features |  |  | **Drag-selecting text in a `/btw` answer** scrolls the answer while you hold the pointer on or past the panel's edge.
- v73-f05 | pending | W10 | Features |  |  | **Gruvbox dark theme** (`gruvbox-dark-hard`), selectable with `/theme`.
- v73-x02 | pending | W10 | Bug Fixes |  |  | **Mouse wheel** in a `/btw` answer reaches the last lines on short terminals.
- v73-x03 | pending | W10 | Bug Fixes |  |  | **Several Send nows in a row** no longer label the turns they stop "Turn cancelled by user" or jump the transcript view.
- v73-x07 | pending | W10 | Bug Fixes |  |  | **Typing while the transcript has focus** moves any character, including non-ASCII text, to the prompt, and leaves a pending card or plan approval as it is.
- v73-x10 | pending | W10 | Bug Fixes |  |  | **Lines with `|` that are not a Markdown table** wrap like other text instead of being cut off at the terminal edge.
- v73-x11 | pending | W10 | Bug Fixes |  |  | **Transcript selection** stays on the same row when a row above it is removed.
- v74-f01 | pending | W10 | Features |  |  | **Messages between subagents** appear as left-aligned rows under the bullet that name the sender and recipient, replacing the centered rows.
- v74-f04 | pending | W10 | Features |  |  | **`[Copy]` button** in a finished `/btw` panel and its folded transcript row copies the whole answer, even when only part of it is visible.
- v74-f07 | pending | W10 | Features |  |  | **Quotes in replies** get a `[Copy]` button that copies the whole quote as Markdown without its `>` markers.
- v74-f08 | pending | W10 | Features |  |  | **Named subagent links** open on a plain click, so they work over SSH from macOS where a modified click can't reach Grok.
- v74-x06 | pending | W10 | Bug Fixes |  |  | **`/btw` on a very short terminal** no longer swallows arrow keys, Page Up/Down or clicks while its panel is too small to draw.
- v74-x07 | pending | W10 | Bug Fixes |  |  | **A code block's `[Copy]`** in the transcript copies while a queued prompt is open for editing, instead of copying nothing or closing the edit.
- v75-f04 | pending | W10 | Features |  |  | **`Esc` on a `/btw` side question that hasn't been answered yet** cancels it, instead of letting it run on and reopening the panel when the answer arrives.
- v75-f07 | pending | W10 | Features |  |  | **The `/btw` question in the panel's title** can be selected with the mouse and is copied to the clipboard when you release, like the answer text.
- v75-x17 | pending | W10 | Bug Fixes |  |  | **Code blocks inside quotes** render every line under the quote's bars, quote copies turn reference links into inline links, and lines after multi-line math are numbered correctly.
- v75-x20 | pending | W10 | Bug Fixes |  |  | **`/btw`** starts a side question only when it begins the message; a `/btw` later in the text goes to the main conversation as typed.
- v77-x01 | pending | W10 | Bug Fixes |  |  | **Quote copy** now preserves & in links and double-clicks no longer fold thoughts when missing text.
- v78-f02 | pending | W10 | Features |  |  | **Dragging in the transcript** now selects and copies colored text across message blocks instead of whole blocks.
- v46-x05 | pending | W11 | Bug Fixes |  |  | **Context compacted** line now shows the correct token count immediately after compaction instead of at the end of a long turn.
- v48-x03 | pending | W11 | Bug Fixes |  |  | `read_file`, `search_replace`, and `grep` now handle large files, special files, and binary content more reliably and surface better errors.
- v48-x05 | pending | W11 | Bug Fixes |  |  | **Cancelling one tool call** no longer kills commands started by other parallel calls.
- v49-x03 | pending | W11 | Bug Fixes |  |  | **Allow once** in permission cards now applies only to the next shell command instead of the whole session.
- v49-x04 | pending | W11 | Bug Fixes |  |  | **pkill -f** and **pgrep -f** commands are now rejected earlier when the pattern would kill their own shell wrapper.
- v50-f07 | pending | W11 | Features |  |  | **Shell command output** now shows "Running …" while executing and "Ran …" when finished.
- v50-x02 | pending | W11 | Bug Fixes |  |  | **Cancelled turns** now correctly mark unanswered tool calls as cancelled instead of succeeded.
- v50-x16 | pending | W11 | Bug Fixes |  |  | `grok` now accepts a prompt via piped stdin in headless mode instead of trying to open the UI.
- v50-x41 | pending | W11 | Bug Fixes |  |  | **Slash commands** now run regardless of case (e.g. /COMPACT or /Compact).
- v50-x42 | pending | W11 | Bug Fixes |  |  | **/context** no longer shows an invented auto-compact threshold when the agent does not report one.
- v50-x43 | pending | W11 | Bug Fixes |  |  | **Agent text** no longer keeps the pager redrawing when a tool call interrupts the sentence.
- v50-x44 | pending | W11 | Bug Fixes |  |  | **CLI tool calls** no longer fail or overflow when the workspace directory is deleted or output is huge.
- v50-x48 | pending | W11 | Bug Fixes |  |  | **Shell tool descriptions** now correctly state the actual output truncation limit, and grep respects globs relative to the current session directory.
- v50-x49 | pending | W11 | Bug Fixes |  |  | **Compaction messages** now scroll into view instead of appearing below the bottom of the screen after a turn finishes.
- v50-x63 | pending | W11 | Bug Fixes |  |  | **Grep** now respects Read deny rules that use a leading `./`.
- v51-f32 | pending | W11 | Features |  |  | `grok agent stdio` now supports listing sessions via ACP and returns consistent reply envelopes.
- v51-f34 | done | W11 | Features |  |  | **--allowed-tools** is now accepted as an alias for --allow in headless mode.
- v51-x23 | pending | W11 | Bug Fixes |  |  | **Headless JSON output** now reports the correct stop reason when the turn limit is reached and removes duplicate error prefixes.
- v51-x71 | pending | W11 | Bug Fixes |  |  | **grok -p -s** with a reused session id now shows the plain refusal message instead of JSON-RPC noise.
- v51-x95 | pending | W11 | Bug Fixes |  |  | Background shell commands now fail immediately when the working directory does not exist.
- v51-x103 | pending | W11 | Bug Fixes |  |  | The agent stdio mode now returns clearer errors for malformed client requests and consistent recap replies.
- v51-x109 | pending | W11 | Bug Fixes |  |  | Running /compact on an empty session no longer resets the reported context window.
- v51-x111 | pending | W11 | Bug Fixes |  |  | **Background shell commands** no longer flip back to running when late output arrives.
- v52-x01 | pending | W11 | Bug Fixes |  |  | **Grep searches** that time out or fail now show an error instead of appearing as empty results.
- v52-x04 | pending | W11 | Bug Fixes |  |  | **Shell commands** no longer keep running after you quit grok or it receives a signal.
- v52-x09 | pending | W11 | Bug Fixes |  |  | `grok -p --effort` now prints a warning when the model ignores the flag and refuses invalid effort levels.
- v52-x12 | pending | W11 | Bug Fixes |  |  | **Grep** and **list_dir** now report a clear error when the search root or directory cannot be read instead of silently returning no results.
- v52-x18 | pending | W11 | Bug Fixes |  |  | Headless prompts that start with a byte-order mark no longer send the mark to the model or break JSON prompt files.
- v52-x22 | pending | W11 | Bug Fixes |  |  | **Context overflow errors** now trigger automatic compaction instead of failing the turn.
- v52-x24 | pending | W11 | Bug Fixes |  |  | **Auto-compaction** now stops after three context refills in a row instead of looping forever.
- v52-x27 | pending | W11 | Bug Fixes |  |  | **[permission] rules** now load alongside compact deny/allow/ask lists in config.toml.
- v53-x07 | done | W11 | Bug Fixes |  |  | **search_replace** refuses an `old_string` that matches again at an overlapping position, so it no longer edits the wrong match.
- v53-x08 | done | W11 | Bug Fixes |  |  | **read_file** reports the correct line count for files that end with a newline and no longer returns an empty line past the end.
- v53-x20 | pending | W11 | Bug Fixes |  |  | **Editing a queued command** such as `/compact` is refused with a message, so your new text is kept.
- v54-x05 | pending | W11 | Bug Fixes |  |  | **Wrapped URLs** in shell command descriptions link to the full address on every row.
- v62-x01 | pending | W11 | Bug Fixes |  |  | **Headless mode** (`grok -p`) waits for the follow-up turn that a finished background task starts before exiting, and a `--background-wait-timeout` that stops pending work exits with an error instead of success.
- v64-f02 | pending | W11 | Features |  |  | **Auto mode** runs a shell command without review only when every command in it matches an allowlist entry; anything else asks first. Permission rules read each word the way bash passes it to the program.
- v64-x11 | pending | W11 | Bug Fixes |  |  | **Shell commands** run correctly on zsh when your startup files alias `grep` or replace `run-help`.
- v74-x20 | pending | W11 | Bug Fixes |  |  | **Headless runs stopped by Ctrl-C or SIGTERM** end their `json`, `streaming-json` or `streaming-messages-json` output with a closing record that keeps the run's outcome, instead of cutting off the stream.
- v74-x33 | pending | W11 | Bug Fixes |  |  | **Context window**: a larger window picked with `/context-window` or set with `context_window` in a `[model."<id>"]` table stays in effect, so compaction no longer starts early after the session sits idle, after a failed request such as a rate limit, or until you restart.
- v76-x01 | pending | W11 | Bug Fixes |  |  | **Shell commands** correctly handle startup functions containing special characters.
- v46-x02 | pending | W12 | Bug Fixes |  |  | **Tool search** activity now correctly folds as "Ran N tool searches" instead of claiming MCP tool results.
- v46-x06 | pending | W12 | Bug Fixes |  |  | **Enter on video cards** no longer attempts playback in terminals that cannot show it or lack ffmpeg.
- v46-x07 | pending | W12 | Bug Fixes |  |  | **Video cards** now open the file in the default player on Enter in terminals without inline playback support, and no longer get stuck trying to extract posters without ffmpeg.
- v48-x04 | pending | W12 | Bug Fixes |  |  | **Ask rules for WebSearch and WebFetch** now correctly show a prompt instead of auto-running.
- v49-x02 | pending | W12 | Bug Fixes |  |  | **Web searches, fetches, and image generation** now correctly respect the prompt_policy setting and rule decisions before prompting.
- v50-x20 | pending | W12 | Bug Fixes |  |  | When multiple MCP servers expose a tool with the same name, `search_tool` now returns all of them.
- v50-x21 | pending | W12 | Bug Fixes |  |  | Concurrent Grok windows editing settings no longer lose or corrupt config.toml entries such as MCP servers or permission rules.
- v50-x27 | done | W12 | Bug Fixes |  |  | Imagine replies that are not valid images are now rejected instead of being saved as .jpg files.
- v50-x29 | done | W12 | Bug Fixes |  |  | **Re-encoded images** are now oriented correctly using their EXIF tags instead of appearing sideways.
- v50-x32 | done | W12 | Bug Fixes |  |  | **Transparent images** are now kept as PNG when re-encoded instead of turning black.
- v51-f03 | pending | W12 | Features |  |  | **`/imagine`** and similar commands now run the matching bundled skill when the required tools are unavailable.
- v51-f05 | pending | W12 | Features |  |  | **Web fetch approval cards** now include "always allow" and "never allow" domain options that persist for the folder.
- v51-f08 | pending | W12 | Features |  |  | **Chrome profiles** can now be persisted across restarts using durable state directories.
- v51-f09 | pending | W12 | Features |  |  | Workspace server sessions now share one Chrome instance for cookies and storage.
- v51-f31 | pending | W12 | Features |  |  | Browser tabs can now be restored on demand instead of all at once when a session resumes.
- v51-x16 | pending | W12 | Bug Fixes |  |  | Images from tools are now validated so bad files no longer break the current turn or future replays.
- v51-x17 | pending | W12 | Bug Fixes |  |  | Screenshots and other image artifacts saved by the browser tool are now stored as real image files.
- v51-x18 | pending | W12 | Bug Fixes |  |  | Rate-limit errors now show the expected message; very small images are rejected early.
- v51-x25 | pending | W12 | Bug Fixes |  |  | **Dragging images** from a file manager onto terminals like GNOME Terminal or xfce4-terminal now attaches them correctly.
- v51-x41 | pending | W12 | Bug Fixes |  |  | **Ctrl+C during a turn** now cancels the turn even when a modal such as /mcps or the block viewer is open.
- v51-x45 | pending | W12 | Bug Fixes |  |  | Large file writes or many screenshots no longer end the session.
- v51-x48 | pending | W12 | Bug Fixes |  |  | Downloads via FetchMcpResource are now refused in plan mode and resolve paths consistently.
- v51-x50 | pending | W12 | Bug Fixes |  |  | Slash commands like /imagine now work regardless of capitalization and correctly substitute skills when tools are missing.
- v51-x90 | pending | W12 | Bug Fixes |  |  | Workspace server now starts with per-session private Chrome by default.
- v51-x102 | pending | W12 | Bug Fixes |  |  | Permission cards for MCP tools can no longer be tricked into showing fake option rows.
- v51-x107 | pending | W12 | Bug Fixes |  |  | **Web search and image approval cards** now only offer always-approve mode to clients that support it.
- v51-x113 | pending | W12 | Bug Fixes |  |  | **Browser tabs** restore and close more reliably with fewer races.
- v51-x116 | pending | W12 | Bug Fixes |  |  | **Voice error messages** now display with correct punctuation.
- v52-x08 | done | W12 | Bug Fixes |  |  | **Pasted images** that exceed the 50 MB send limit are now rejected with a clear message instead of silently dropped.
- v52-x15 | pending | W12 | Bug Fixes |  |  | **@mentions and image placeholders** now respect Read deny rules and are omitted from the prompt.
- v52-x21 | pending | W12 | Bug Fixes |  |  | MCP tool replies containing invalid characters no longer cause the call to time out.
- v52-x32 | pending | W12 | Bug Fixes |  |  | HTTP MCP tools that return a JSON-RPC error are now executed only once.
- v53-x02 | pending | W12 | Bug Fixes |  |  | **`/feedback`** stays open while you type when a permission request, question, or MCP prompt arrives.
- v53-x11 | pending | W12 | Bug Fixes |  |  | **Parallel MCP tool calls** that ask for input each show their own prompt.
- v53-x19 | done | W12 | Bug Fixes |  |  | **Stdio MCP servers** expand a leading `~` in their command and arguments to your home directory.
- v53-x23 | pending | W12 | Bug Fixes |  |  | **Send now (Ctrl+Enter)** no longer answers the stop-subagents panel or an MCP prompt card.
- v54-x01 | done | W12 | Bug Fixes |  |  | **Pasted Windows line endings** reach the model as newlines, markdown images no longer show a stray "!", and dollar amounts that are not math stay plain text.
- v54-x11 | pending | W12 | Bug Fixes |  |  | **Failed MCP tool rows** wrap the error to the screen width and keep its line breaks.
- v63-x05 | pending | W12 | Bug Fixes |  |  | **Custom models on Chat Completions endpoints outside xAI** no longer fail turns with a tool schema error when MCP servers are connected.
- v64-f05 | done | W12 | Features |  |  | **HTTP connections**, including remote MCP servers, accept gzip, brotli, zstd, and deflate compressed responses.
- v64-x03 | pending | W12 | Bug Fixes |  |  | **Web search** works from a session's first turn, and in subagents, when the search model signs in through an external auth provider.
- v64-x10 | pending | W12 | Bug Fixes |  |  | **Custom models on the Responses API** keep the turn going when a web search call starts without an action.
- v64-x12 | pending | W12 | Bug Fixes |  |  | **Voice dictation preview** grows the prompt as you speak and keeps the newest words in view.
- v66-x02 | done | W12 | Bug Fixes |  |  | **Selected and highlighted rows** stay readable on every theme because reverse-video text is never dimmed. This fixes faint descriptions in selected menu rows on the `terminal` theme in terminals such as Kitty.
- v68-f06 | pending | W12 | Features |  |  | **Image placeholders** in the transcript now show whether the actual image was sent or only the text was typed.
- v68-x04 | pending | W12 | Bug Fixes |  |  | **Send Now prompts** and rewound turns now correctly restore and mark any images that were originally sent.
- v68-x05 | pending | W12 | Bug Fixes |  |  | **Resumed sessions** now show the correct sent or struck-through state for image placeholders.
- v71-f02 | pending | W12 | Features |  |  | **Image chips in the prompt** use the same green as sent images in the transcript.
- v71-f03 | pending | W12 | Features |  |  | **Hovering an image chip** in the transcript shows the image that was sent, or why it was not attached.
- v71-f06 | pending | W12 | Features |  |  | **Inline images** display in Noctty on Windows.
- v71-x01 | pending | W12 | Bug Fixes |  |  | **Image chips** in your messages stay whole on one row when the message wraps.
- v71-x02 | pending | W12 | Bug Fixes |  |  | **Images the server rejects** are removed from the conversation, so the notice that images were left out no longer repeats, and grows, on every later turn.
- v72-x07 | pending | W12 | Bug Fixes |  |  | **MCP tools built with FastMCP or the official MCP SDK** send their result to the model once instead of twice, so each call uses about half the context.
- v72-x09 | done | W12 | Bug Fixes |  |  | **MCP servers over stdio** no longer lose part of a long reply line, such as a large tool list, when a read is interrupted.
- v73-x06 | pending | W12 | Bug Fixes |  |  | **Queued images** show as green chips in the queue pane, as in the prompt.
- v75-x06 | pending | W12 | Bug Fixes |  |  | **Generated image and video paths** with wide characters such as Chinese, Japanese or emoji fit their row and keep the file name, and Windows paths keep it too.
- v75-x09 | pending | W12 | Bug Fixes |  |  | **Cut-off Fetch, Web Search and MCP tool output** ends with the same `… +N lines · <key> to view all` row as code cells, naming your viewer key, and one hidden line reads `+1 line`.
- v77-x02 | pending | W12 | Bug Fixes |  |  | MCP server connections no longer restart unnecessarily on session reloads or updates.
- v78-x02 | pending | W12 | Bug Fixes |  |  | **Dropped-image notice** no longer appears inside the user's own message in transcripts or history.
- v46-x01 | pending | W13 | Bug Fixes |  |  | **MCP server sources** are now correctly reported by `grok inspect` and the MCP doctor.
- v47-x01 | done | W13 | Bug Fixes |  |  | Error messages on the home and login screens now wrap properly in narrow terminals.
- v48-x06 | pending | W13 | Bug Fixes |  |  | **Authentication recovery** no longer automatically retries refused prompts after login; the sign-in block now requires manual resend.
- v50-x33 | done | W13 | Bug Fixes |  |  | **The bundled ripgrep** is now available to terminal commands even on images that do not install it.
- v50-x38 | pending | W13 | Bug Fixes |  |  | **grok doctor** now reports the exact config file or setting that prevents Grok from starting instead of saying zero issues.
- v50-x40 | pending | W13 | Bug Fixes |  |  | **grok doctor fix** now opens its confirmation card on Cancel to prevent accidental edits to shell or tmux config.
- v51-f29 | pending | W13 | Features |  |  | grok -p now runs auth_provider_command when no login is stored.
- v51-x12 | pending | W13 | Bug Fixes |  |  | Sign-in failures now show a clear plain-English message with the next step instead of raw errors.
- v53-x14 | pending | W13 | Bug Fixes |  |  | **`grok plugin install`** interrupted mid-clone on Linux no longer leaves a partial folder that blocks the retry.
- v59-f02 | done | W13 | Features |  |  | **`grok --resume <title>`** matches a title whatever separates its words, so `fix-login-bug` finds "Fix login bug".
- v60-x01 | done | W13 | Bug Fixes |  |  | **`grok --resume <title>`** with a title that matches no local session reports the miss right away instead of trying a remote restore and asking you to sign in.
- v62-x05 | pending | W13 | Bug Fixes |  |  | **`grok update`** and **`grok login`** no longer crash when stdout or stderr is closed or unwritable.
- v64-f06 | done | W13 | Features |  |  | **Plugin install suggestions** above the prompt are off when `disable_plugins = true` is set.
- v69-x02 | pending | W13 | Bug Fixes |  |  | **`grok doctor`** and `/doctor` now point out a leftover `[ui] vim_mode` key (now `[keys] vim`) or the retired `[ui] simple_mode` key in your config.
- v73-f04 | pending | W13 | Features |  |  | **Automatic updates** stop retrying a new version that fails to start on this machine until a newer version is out, and each launch says to run `grok update` to see why.
- v74-f03 | pending | W13 | Features |  |  | **`/mcps`**: when a browser can't open an MCP server's sign-in URL, the URL shows in a panel you can copy with `c` or the `[copy the url]` button.
- v74-f06 | pending | W13 | Features |  |  | **`!` mode Tab completion** offers commands installed as symlinks, such as Homebrew, npm, rustup and Debian alternatives binaries.
- v74-x05 | pending | W13 | Bug Fixes |  |  | **`grok update`** runs when `GROK_SANDBOX`, `--sandbox` or config sets a sandbox profile, unless Grok is already running confined.
- v74-x08 | pending | W13 | Bug Fixes |  |  | **`grok logout`** stays signed out when another running Grok refreshes its sign-in, instead of signing you back in on the next launch.
- v74-x12 | pending | W13 | Bug Fixes |  |  | **Grok running confined in a sandbox** never starts an update on its own and tells you to run `grok update` instead.
- v74-x13 | pending | W13 | Bug Fixes |  |  | **Quitting to install an update** with `Ctrl+U` on the welcome screen exits within the usual exit deadline instead of sometimes hanging after "Update installed".
- v74-x14 | pending | W13 | Bug Fixes |  |  | **`grok update --force-reinstall`**, or `grok update --version` with the installed version, keeps your working `grok` on Linux and macOS when the download fails its checks, instead of deleting it.
- v74-x15 | pending | W13 | Bug Fixes |  |  | **Windows installers** check that the downloaded build runs before replacing `grok.exe` and `agent.exe`, and keep the existing install when it doesn't, such as when a proxy serves a blocked-download page.
- v74-x17 | pending | W13 | Bug Fixes |  |  | **Installers** keep the installed `grok` when a proxy or cache returns an empty download, instead of replacing it with an empty file and reporting success.
- v74-x32 | pending | W13 | Bug Fixes |  |  | **Minimal mode**: "Sign-in failed" shows in the error color instead of the warning color.
- v78-x03 | pending | W13 | Bug Fixes |  |  | **grok update --channel** now gives a clear error and steps when bin/grok was installed by npm.
- v78-x04 | pending | W13 | Bug Fixes |  |  | `grok update` and channel switches now work correctly for Homebrew cask installs and show the right commands.
- v46-x03 | pending | W14 | Bug Fixes |  |  | **Client rules** supplied at session start are now preserved when the system prompt is rebuilt later.
- v47-f01 | pending | W14 | Features |  |  | **Model switch responses** from the agent now update the displayed model list and context window.
- v48-x01 | pending | W14 | Bug Fixes |  |  | **`/rewind`** now only lists prompts you typed yourself and skips automatic runtime turns.
- v50-f02 | pending | W14 | Features |  |  | **Clicking the git branch** in the header now copies the full branch name.
- v50-f03 | pending | W14 | Features |  |  | **Long branch names** now abbreviate on narrow terminals while remaining copyable.
- v50-f04 | pending | W14 | Features |  |  | **Targeted cancels** now stop only the specific response when requested by clients.
- v50-f08 | pending | W14 | Features |  |  | **Live preview pins** now include whether an app was observed behind the URL.
- v50-x03 | pending | W14 | Bug Fixes |  |  | **Long error messages** in the /usage and /session-info modals now wrap instead of being truncated.
- v50-x05 | pending | W14 | Bug Fixes |  |  | **Stalled model calls** during turns now retry before failing the turn.
- v50-x07 | pending | W14 | Bug Fixes |  |  | **Long sessions** no longer disconnect when the conversation exceeds 4 MiB.
- v50-x10 | pending | W14 | Bug Fixes |  |  | **Failed file writes** now leave the previous contents instead of an empty file.
- v50-x11 | pending | W14 | Bug Fixes |  |  | **Fixed /loop and scheduler_create** so non-ASCII intervals like 5分 no longer crash the session.
- v50-x13 | pending | W14 | Bug Fixes |  |  | **Fixed closing a busy tab** so queued prompts are no longer sent to the model.
- v50-x14 | pending | W14 | Bug Fixes |  |  | **Fixed relative --cwd** so `grok --cwd a` no longer ends up running inside `a/a`.
- v50-x15 | pending | W14 | Bug Fixes |  |  | The terminal title now shows "Action Required" for any background session waiting on approval.
- v50-x17 | pending | W14 | Bug Fixes |  |  | An admin-set `[features] title_refresh = false` in requirements.toml now correctly disables title refresh even when GROK_TITLE_REFRESH is set.
- v50-x19 | pending | W14 | Bug Fixes |  |  | An admin pin of `[cli] use_leader = false` now correctly prevents `--leader` from starting a leader process.
- v50-x26 | pending | W14 | Bug Fixes |  |  | Drafts saved by the model now appear under their actual titles in /feedback.
- v50-x31 | pending | W14 | Bug Fixes |  |  | **Tab characters** in terminal command output are now preserved when shown to the model.
- v50-x34 | pending | W14 | Bug Fixes |  |  | **Request bodies** now stay under a small configured `max_request_bytes` limit instead of growing past it.
- v50-x35 | pending | W14 | Bug Fixes |  |  | **Effort level** now stays the same after a bare `/model` switch instead of jumping to the new model's default.
- v50-x37 | pending | W14 | Bug Fixes |  |  | **Feedback submissions** now show accurate success or failure messages without premature thanks or duplicated prefixes.
- v50-x39 | pending | W14 | Bug Fixes |  |  | **Recap summaries** now appear on screen automatically instead of requiring manual scrolling.
- v50-x45 | pending | W14 | Bug Fixes |  |  | **Logs written to files or when NO_COLOR is set** are now plain text instead of containing color codes.
- v50-x46 | pending | W14 | Bug Fixes |  |  | **Prompts now fail immediately** with a clear message if the session's working directory no longer exists instead of failing later or recreating it.
- v50-x47 | pending | W14 | Bug Fixes |  |  | **grok -w** now works correctly with separate-git-dir repositories and shows clear errors when run outside a git repo or on an empty repo.
- v50-x54 | pending | W14 | Bug Fixes |  |  | Merge and rebase conflicts now appear in the changes list instead of being hidden.
- v50-x55 | pending | W14 | Bug Fixes |  |  | Fixed a crash when starting a new session with durable rewind checkpoints enabled.
- v50-x59 | pending | W14 | Bug Fixes |  |  | Discard now affects only the exact files you select instead of also deleting or reverting similarly-named files.
- v50-x61 | pending | W14 | Bug Fixes |  |  | **Chat Completions streams** that end without a finish reason are now retried instead of returning partial output.
- v51-f06 | pending | W14 | Features |  |  | **Slash command completion** now updates live when skills are added or changed during a session.
- v51-f07 | pending | W14 | Features |  |  | **Request form** now securely collects and fills sensitive details like passwords or card numbers without sending values to the model.
- v51-f10 | pending | W14 | Features |  |  | **`/feedback` Drafts tab** now supports `G` / `Home` and `End` to jump to the first or last draft.
- v51-f20 | pending | W14 | Features |  |  | **Interactive sessions** now include the Grok Build user guide location so the agent can answer usage questions.
- v51-f33 | pending | W14 | Features |  |  | Answering an approval with a message now tells the model it is a follow-up.
- v51-x05 | pending | W14 | Bug Fixes |  |  | **The follow indicator no longer appears** when only blank padding sits below the viewport.
- v51-x06 | pending | W14 | Bug Fixes |  |  | **Search failures now keep the original pattern** instead of showing an empty Search "".
- v51-x07 | pending | W14 | Bug Fixes |  |  | **Sessions that open without a model list now use the public Auto id** and succeed on every account.
- v51-x08 | pending | W14 | Bug Fixes |  |  | Background wake turns that request plan changes or mode switches are now handled correctly.
- v51-x09 | pending | W14 | Bug Fixes |  |  | Model-call retries now show progress status instead of a generic waiting message.
- v51-x13 | pending | W14 | Bug Fixes |  |  | Logging out now stops any running turns on local sessions instead of leaving them active.
- v51-x15 | pending | W14 | Bug Fixes |  |  | `/find` now keeps match counts and navigation accurate while replies or command output stream in.
- v51-x20 | pending | W14 | Bug Fixes |  |  | Opening the queue with Up now scrolls so the selected follow-up is visible.
- v51-x21 | pending | W14 | Bug Fixes |  |  | Command output that contains clear-screen codes is no longer discarded.
- v51-x26 | pending | W14 | Bug Fixes |  |  | **Plan comments** added while a revision runs are now preserved and sent when the plan is approved.
- v51-x30 | pending | W14 | Bug Fixes |  |  | A feedback send now shows "Sending feedback…" immediately; failed sends no longer repeat the lead phrase.
- v51-x32 | pending | W14 | Bug Fixes |  |  | **Error hints** for legacy session import now appear in full instead of being cut off by the pager.
- v51-x33 | pending | W14 | Bug Fixes |  |  | **Skills** in the folder where grok started now correctly show as (local).
- v51-x38 | pending | W14 | Bug Fixes |  |  | **Write approval cards** now show the existing file contents that will be replaced.
- v51-x39 | pending | W14 | Bug Fixes |  |  | **Slash command completion** now shows the correct name for files under commands/ instead of always showing /commands.
- v51-x47 | pending | W14 | Bug Fixes |  |  | **`/find`** now scrolls to the exact wrapped row containing the current match.
- v51-x51 | pending | W14 | Bug Fixes |  |  | Turning off bypass mode without naming a session now correctly restores prompts in all open sessions.
- v51-x52 | pending | W14 | Bug Fixes |  |  | /delete with extra text now shows an error instead of deleting the current session.
- v51-x53 | pending | W14 | Bug Fixes |  |  | **Failed /feedback** status messages now wrap inside the dialog instead of clipping at the border.
- v51-x61 | pending | W14 | Bug Fixes |  |  | Typing a hidden command such as `/cd` in a session and pressing Enter now runs it instead of accepting a near-match skill.
- v51-x70 | pending | W14 | Bug Fixes |  |  | **Fixed intermittent** List/Read/Open failures during concurrent v2 hardening migrations.
- v51-x73 | pending | W14 | Bug Fixes |  |  | **Fixed several** pager rows that showed wrong paths, truncated names, or hid failure/cancel states.
- v51-x74 | pending | W14 | Bug Fixes |  |  | **--system-prompt** now correctly appends rules from --append-system-prompt instead of dropping them.
- v51-x76 | pending | W14 | Bug Fixes |  |  | **Multi-select questions** now let you toggle several answers with number keys before submitting.
- v51-x77 | pending | W14 | Bug Fixes |  |  | **Slash-command notices** typed while a reply streams no longer hide the streaming reply.
- v51-x82 | pending | W14 | Bug Fixes |  |  | **Per-session system prompt overrides** now correctly use only the session's own rules.
- v51-x83 | pending | W14 | Bug Fixes |  |  | **Connection stalled** errors now show a clearer message with network advice.
- v51-x84 | pending | W14 | Bug Fixes |  |  | **Invalid model provider tables** now produce a clear startup error instead of silent fallback.
- v51-x85 | pending | W14 | Bug Fixes |  |  | **Narrow feedback dialog** now shows full draft titles and readable confirm prompts.
- v51-x87 | pending | W14 | Bug Fixes |  |  | Dialogs like /feedback no longer disappear on very short terminal windows.
- v51-x94 | pending | W14 | Bug Fixes |  |  | Approval answers sent after a turn ends no longer affect the turn.
- v51-x99 | pending | W14 | Bug Fixes |  |  | Editing a queued prompt now prevents accidental removal with a stale client view.
- v51-x106 | pending | W14 | Bug Fixes |  |  | Delete operations now correctly match `Bash(rm …)` ask/deny rules even when the path contains `..` or the cwd is a symlink.
- v51-x112 | pending | W14 | Bug Fixes |  |  | Only slash commands at the start of a message trigger skills.
- v51-x114 | pending | W14 | Bug Fixes |  |  | Fast model names in the footer no longer show double parentheses.
- v51-x118 | pending | W14 | Bug Fixes |  |  | **Project skills** are now labeled correctly for nested projects.
- v52-f05 | pending | W14 | Features |  |  | Command status-line scripts now see the current turn's prompt ID and start time while the turn is active.
- v52-x02 | pending | W14 | Bug Fixes |  |  | **`/model` switches** that fail no longer change the footer or saved default model.
- v52-x03 | pending | W14 | Bug Fixes |  |  | **Export and copy** commands no longer overwrite an existing file; they refuse with a clear message.
- v52-x07 | pending | W14 | Bug Fixes |  |  | **grok inspect** no longer warns about valid pins that only requirements.toml accepts.
- v52-x13 | pending | W14 | Bug Fixes |  |  | **Cancelled terminal commands** now show the actual command that was stopped.
- v52-x16 | pending | W14 | Bug Fixes |  |  | **Restarting** after a model switch no longer reverts to the previous model.
- v52-x26 | pending | W14 | Bug Fixes |  |  | **Project rules files** that resolve outside the git root or workspace folder are now skipped.
- v52-x29 | pending | W14 | Bug Fixes |  |  | **Approval cards** now close the block viewer so the card is visible.
- v53-f06 | pending | W14 | Features |  |  | **Multi-file edits** show how many other files the edit changed.
- v53-x03 | pending | W14 | Bug Fixes |  |  | **Persistent shell** runs your command even when restoring its saved state fails.
- v53-x04 | pending | W14 | Bug Fixes |  |  | **`/fork`** is refused while an approval request is open, so the approval keys keep answering that request.
- v53-x05 | pending | W14 | Bug Fixes |  |  | **`grok --help`** and **`grok --version`** no longer create a `.grok` folder.
- v53-x06 | pending | W14 | Bug Fixes |  |  | **Wrapped URLs** in tool headers link to the full address on every row.
- v53-x12 | pending | W14 | Bug Fixes |  |  | **`/feedback` Drafts** load the valid drafts when one saved draft is invalid.
- v53-x16 | pending | W14 | Bug Fixes |  |  | **Edit diffs** mark removed and added lines with `-` and `+` when color is off.
- v53-x17 | pending | W14 | Bug Fixes |  |  | **Session usage** keeps every turn when two windows write to the same session.
- v53-x18 | pending | W14 | Bug Fixes |  |  | **Failed directory listings** expand to show why they failed.
- v53-x21 | pending | W14 | Bug Fixes |  |  | **`/docs`** opens the exact guide or picker you type.
- v54-x02 | pending | W14 | Bug Fixes |  |  | **Write approval cards** show a binary or unreadable file as unreadable instead of garbled text.
- v54-x03 | pending | W14 | Bug Fixes |  |  | **Prompts you send while a session is resuming at startup** are queued and sent once it loads instead of staying in the composer.
- v54-x04 | pending | W14 | Bug Fixes |  |  | **A corrupt active-sessions registry** no longer stops new sessions from registering: grok keeps a backup, salvages the session rows it can read, and warns you.
- v54-x07 | pending | W14 | Bug Fixes |  |  | **Links** are clickable in herdr panes.
- v54-x10 | pending | W14 | Bug Fixes |  |  | **Startup** is faster when `TMUX` names a tmux server that has stopped.
- v54-x12 | pending | W14 | Bug Fixes |  |  | **`[skills].ignore`** matches a symlinked SKILL.md by the folder that holds it.
- v54-x13 | pending | W14 | Bug Fixes |  |  | **Failed file reads** expand to show why they failed.
- v59-x05 | pending | W14 | Bug Fixes |  |  | **Grok** no longer crashes at startup when the `SENTRY_DSN` environment variable holds an invalid value.
- v59-x07 | pending | W14 | Bug Fixes |  |  | **Copying** a code block or a whole reply keeps its original tabs instead of turning them into spaces.
- v62-x03 | pending | W14 | Bug Fixes |  |  | **Enter on a highlighted slash command** with optional arguments runs the command instead of leaving a trailing space in the composer.
- v63-x02 | pending | W14 | Bug Fixes |  |  | **Plan reviews** at the end of a turn wait until you close an open `/feedback` dialog, question, tool prompt, or queued prompt edit.
- v63-x04 | pending | W14 | Bug Fixes |  |  | **`/fork`** keeps the parent session's model and effort when Grok was started with `-m` or `--effort`.
- v64-x02 | pending | W14 | Bug Fixes |  |  | **A new session that fails to start** puts what you typed back in the prompt you return to, instead of losing it.
- v64-x07 | pending | W14 | Bug Fixes |  |  | **External OpenTelemetry export** keeps sending logs and metrics when `OTEL_EXPORTER_OTLP_COMPRESSION` is set to `gzip` or `zstd`.
- v64-x08 | pending | W14 | Bug Fixes |  |  | **`[model.*]` tables** without a context window take the window and API of another table that sends the same model, instead of the 200,000-token fallback.
- v64-x09 | pending | W14 | Bug Fixes |  |  | **Grok on macOS without a GUI session**, such as over SSH, no longer hangs on its first modifier-key check.
- v65-f04 | pending | W14 | Features |  |  | **`grok --help`** gives each command and option a one-line summary, and `grok help -h` and `grok help --help` print the help command's own help instead of an error.
- v65-x02 | pending | W14 | Bug Fixes |  |  | **`DISABLE_TELEMETRY` and `DISABLE_ERROR_REPORTING`** turn telemetry and crash reports off for any non-empty value, including `0` and `false`. A `GROK_TELEMETRY_ENABLED` or `GROK_ERROR_REPORTING` value Grok can't read also counts as off.
- v68-f04 | pending | W14 | Features |  |  | **Mode-switch toast** now shows the mode name (Agent, Plan, Ask) in the mode's own color.
- v69-x01 | pending | W14 | Bug Fixes |  |  | **`[ui] simple_mode`** is retired and no longer has any effect; sessions always open with the prompt focused.
- v70-f02 | pending | W14 | Features |  |  | **`GROK_DISABLE_EXPERIMENTS`** turns off every `[[experiments]]` entry in your configuration; `[experiments] enabled = false` in any configuration file does the same.
- v70-x01 | pending | W14 | Bug Fixes |  |  | **`/plugins`** now counts only the plugin skills that load, and says how many were skipped because they link outside the plugin folder.
- v71-f07 | pending | W14 | Features |  |  | **Answers to the agent's questions** show one row per question and answer, and end in `…` only when the text is cut.
- v71-x03 | pending | W14 | Bug Fixes |  |  | **Feedback draft rows** show the draft's state: drafted, declined, cancelled or failed.
- v71-x04 | pending | W14 | Bug Fixes |  |  | **A slash menu closed with `Esc`** no longer reopens by itself when the command list refreshes, such as right after startup.
- v71-x05 | pending | W14 | Bug Fixes |  |  | **Turn failed rows** no longer repeat a generic "Request failed:" before the error.
- v72-f01 | pending | W14 | Features |  |  | **Relative file paths** in the agent's output, such as `src/app/main.rs`, open on click when the file exists under the session's folder, as absolute paths already do.
- v72-f03 | pending | W14 | Features |  |  | **Pull-request numbers** such as `#123` in the agent's replies open that pull request on the session repository's forge.
- v72-f06 | pending | W14 | Features |  |  | **Feedback draft rows** show on their own instead of inside a group of tool rows, and opening one shows the `/feedback` Drafts tab with that draft selected.
- v72-x01 | pending | W14 | Bug Fixes |  |  | **Warp**: clicking a URL that wraps across rows opens the whole URL instead of only its first row.
- v72-x02 | pending | W14 | Bug Fixes |  |  | **`/delete`** confirmation starts on Cancel, so pressing `Enter` no longer deletes the session; press `1` to delete.
- v72-x03 | pending | W14 | Bug Fixes |  |  | **Max thoughts width** (`/settings`, `[ui].max_thoughts_width`) sets how wide thinking blocks wrap; it previously had no effect.
- v72-x05 | pending | W14 | Bug Fixes |  |  | **Custom models on the Responses API** with a non-xAI `base_url` no longer fail every turn with an HTTP 400 schema error for `use_tool`.
- v72-x06 | pending | W14 | Bug Fixes |  |  | **`/feedback` Drafts tab** keeps the selected draft when the list reloads, such as after leaving Drafts and coming back.
- v72-x08 | pending | W14 | Bug Fixes |  |  | **Plan review notes** accept every key, so a note that starts with `a` no longer approves the plan.
- v73-f01 | pending | W14 | Features |  |  | **`Page Up`/`Page Down`, `Home` and `End`** move the selection in `/skills` and other pickers.
- v73-x01 | pending | W14 | Bug Fixes |  |  | **Windows**: quitting Grok ends `grok.exe` once the terminal is restored, so the shell no longer sometimes waits with no prompt.
- v73-x04 | pending | W14 | Bug Fixes |  |  | **`--restore-code`**: when the code restore fails, your uncommitted and untracked files stay in the working tree instead of in a stash.
- v73-x09 | pending | W14 | Bug Fixes |  |  | **Session load errors** show the whole reason instead of cutting out the middle of a long message.
- v74-f09 | pending | W14 | Features |  |  | **`@` paths outside the workspace**: an `@` path that starts with `/`, `~/` or `../` lists files from that folder; set `[ui] at_picker_outside_workspace = false` to turn it off.
- v74-x03 | pending | W14 | Bug Fixes |  |  | **`/new` tabs** show the context window you just switched to before the new session replies.
- v74-x09 | pending | W14 | Bug Fixes |  |  | **Opening links and files** from the TUI no longer leaves a zombie process behind for each click.
- v74-x10 | pending | W14 | Bug Fixes |  |  | **A failed session delete** shows the "Couldn't delete session" prefix once, so the reason is no longer cut off.
- v74-x11 | pending | W14 | Bug Fixes |  |  | **Windows**: Grok exits fully after a startup error, a `/minimal` or `/fullscreen` switch, or a signal, instead of sometimes hanging.
- v74-x16 | pending | W14 | Bug Fixes |  |  | **Command-line arguments that aren't valid UTF-8** no longer crash `grok` at startup: paths such as `--cwd` accept them, and text such as `-p` gets a usage error.
- v74-x18 | pending | W14 | Bug Fixes |  |  | **Environment variables with non-UTF-8 bytes** no longer stop Grok from starting.
- v74-x25 | pending | W14 | Bug Fixes |  |  | **File paths with spaces** in inline code, such as `~/Library/Application Support/...`, link to the whole path when it exists instead of stopping at the first space.
- v74-x26 | pending | W14 | Bug Fixes |  |  | **Feedback that can't be sent** leads with the outcome, such as "Couldn't send feedback. Saved to Drafts.", then gives the reason and says to retry from `/feedback`.
- v74-x28 | pending | W14 | Bug Fixes |  |  | **Prompt history search** highlights matched letters in the same color as other fuzzy lists, and the counts on the `/` and `@` menu borders are dimmed like other border captions.
- v74-x30 | pending | W14 | Bug Fixes |  |  | **Custom agent files** saved with a UTF-8 byte order mark, as some Windows editors write them, load instead of being skipped.
- v75-f05 | pending | W14 | Features |  |  | **Clicking a link or file that can't open**, such as over SSH with no display, shows why, for example "Can't open links over SSH".
- v75-x02 | pending | W14 | Bug Fixes |  |  | **Stop buttons in the dock** stop a background run while its approval card is open, instead of doing nothing until you answer the card.
- v75-x03 | pending | W14 | Bug Fixes |  |  | **Rejecting a change** keeps edits you saved to the file yourself, even when Grok missed the save, instead of overwriting them.
- v75-x07 | pending | W14 | Bug Fixes |  |  | **The rewind confirmation** no longer crashes Grok on a short terminal or when you shrink the window while it's open.
- v75-x08 | pending | W14 | Bug Fixes |  |  | **Footer and status bars** in narrow terminals drop or shorten the items that don't fit, instead of painting them over each other or cutting buttons off.
- v75-x12 | pending | W14 | Bug Fixes |  |  | **Emoji and keycap symbols** such as `⚠️` or `#️⃣` are measured at the width the terminal draws them, so rows no longer run past their box or lose their `…`.
- v75-x13 | pending | W14 | Bug Fixes |  |  | **`/export` to a named pipe** that nothing is reading fails right away instead of freezing Grok until it is killed.
- v77-x04 | pending | W14 | Bug Fixes |  |  | Context usage shown in the footer no longer drops to zero or shows stale values after a rewind.
- v46-p01 | pending | W15 | Performance |  |  | list_dir on network or virtual filesystems now skips slow subtree counts and prints a notice instead.
- v46-p02 | pending | W15 | Performance |  |  | **Session startup** is faster when loading skills from large directories and shows what it is doing instead of a generic spinner.
- v47-p01 | pending | W15 | Performance |  |  | **Sessions** in repositories with an .envrc file now start faster.
- v50-p01 | pending | W15 | Performance |  |  | Command hooks no longer cause large memory spikes when they print a lot of output.
- v50-p02 | pending | W15 | Performance |  |  | **Retry waits** now vary between processes instead of being identical.
- v51-p01 | pending | W15 | Performance |  |  | Opening the resume picker is now much faster on machines with many worktrees.
- v51-p02 | pending | W15 | Performance |  |  | **grok worktree commands start faster** by skipping unnecessary model catalog work at boot.
- v52-p01 | pending | W15 | Performance |  |  | Multiple Notification hook chains now start in parallel instead of waiting for the previous chain to finish.
- v53-p01 | pending | W15 | Performance |  |  | **@-mentions** reuse the file list until the folder tree changes, so suggestions appear faster.
- v54-p01 | already | W15 | Performance |  |  | **The idle welcome screen** uses less CPU.
- v54-p02 | pending | W15 | Performance |  |  | **Prompt undo history** is capped by size, so large pastes you delete no longer stay in memory.
- v54-p03 | pending | W15 | Performance |  |  | **Typing in a prompt that holds large pastes** is faster.
- v54-p04 | already | W15 | Performance |  |  | **Long sessions** use less memory: long replies scrolled off screen release their render caches.
- v66-p01 | pending | W15 | Performance |  |  | **`@` file completion** shows results about 5 ms after you type instead of about 40 ms.
- v69-p01 | already | W15 | Performance |  |  | **Quitting Grok** returns you to your shell sooner because it no longer waits out fixed intervals.
- v73-p01 | deferred | W15 | Performance |  |  | **Startup** no longer waits about 100 ms for a sign-in file when you use an API key or have not signed in yet.
- v74-p01 | pending | W15 | Performance |  |  | **Long sessions** use less memory: finished Markdown blocks far off screen release their rendered layout, and search still finds their text.
- v74-p02 | pending | W15 | Performance |  |  | **Typing** no longer lags when ffmpeg isn't installed and `PATH` holds slow directories, such as Windows drives under WSL.
- v74-p03 | pending | W15 | Performance |  |  | **Long pastes that arrive as keystrokes**, such as pastes on native Windows or in terminals without bracketed paste, no longer freeze the prompt for seconds while the plan-mode tip is on.
- v74-p04 | pending | W15 | Performance |  |  | **Pasting long single-line text** such as logs, JSON or base64 into the prompt no longer freezes the TUI for seconds.
- v74-p05 | pending | W15 | Performance |  |  | **File watching on Linux** no longer keeps a CPU core busy for about a minute after startup in large repositories.
- v74-p06 | pending | W15 | Performance |  |  | **Lower CPU use while subagents run**: the TUI repaints only for animations on screen, about 7 times a second instead of 30 when no running row is visible.
- v74-p07 | pending | W15 | Performance |  |  | **`grok --resume <id>`** opens much faster on large session stores, reading only the resumed session instead of every session.
- v78-p01 | pending | W15 | Performance |  |  | **grok export** of sessions containing large shell output is now faster and uses less memory.
- v52-f01 | skipped | SKIP | Features | skip-superseded | v63-f01 | **Rex terminal support**: Grok now reports its status (working, waiting for you, done, or failed) to Rex through OSC 7501, so Rex tabs show what each session is doing.
- v55-x01 | skipped | SKIP | Bug Fixes | skip-unspecified |  | Minor bug fixes.
- v56-x01 | skipped | SKIP | Bug Fixes | skip-unspecified |  | Minor bug fixes.
- v57-x01 | skipped | SKIP | Bug Fixes | skip-unspecified |  | Minor bug fixes.
- v58-x01 | skipped | SKIP | Bug Fixes | skip-unspecified |  | Minor bug fixes.
- v71-f08 | skipped | SKIP | Features | skip-superseded | v74-f01 | **Messages from other agents** appear as compact centered rows, and consecutive messages fold into one row.
- v71-f09 | skipped | SKIP | Features | skip-superseded | v74-f01 | **Messages the agent sends to subagents** appear as the same centered rows (`you → <subagent>`) and fold together with the messages it receives.
- v73-f02 | skipped | SKIP | Features | skip-superseded | v74-f01 | **Messages between two subagents** appear in the main session's transcript as centered rows once delivered, such as `find callers → scan src/`.
