# Changelog

# 1.0.78 — 2026-11-06

## Features

- **@ file chips** can now be edited in place; typed @~/ paths resolve to the home folder.
- **Dragging in the transcript** now selects and copies colored text across message blocks instead of whole blocks.
- **@ mentions** now support :N line suffixes and @!hidden files.

## Bug Fixes

- **Always-approve help text** now correctly says the setting is saved to config.
- **Dropped-image notice** no longer appears inside the user's own message in transcripts or history.
- **grok update --channel** now gives a clear error and steps when bin/grok was installed by npm.
- `grok update` and channel switches now work correctly for Homebrew cask installs and show the right commands.

## Performance

- **grok export** of sessions containing large shell output is now faster and uses less memory.


# 1.0.77 — 2026-11-05

## Features

- **New [ui] kitty_keyboard = "query"** setting lets Shift+Enter insert newlines in unidentified terminals.

## Bug Fixes

- **Quote copy** now preserves & in links and double-clicks no longer fold thoughts when missing text.
- MCP server connections no longer restart unnecessarily on session reloads or updates.
- **Ctrl+L** now works on more screens and copy hints match what actually copies to the clipboard.
- Context usage shown in the footer no longer drops to zero or shows stale values after a rewind.
- Rows in the `/resume` list now expand and collapse the same way they do inside a session.


# 1.0.76 — 2026-11-04

## Features

- **Ctrl+L** redraws the screen after a terminal clear without flashing.

## Bug Fixes

- **Shell commands** correctly handle startup functions containing special characters.


# 1.0.75 — 2026-11-04

## Features

- **Permission rules that don't load**, such as a rule that doesn't parse or a `.claude/settings.json` that isn't valid JSON, show a warning when a session opens instead of being skipped silently.
- **`Cmd+Up`/`Cmd+Down` and `Ctrl+Home`/`Ctrl+End`** move the cursor to the start or end of the whole prompt, and with `Shift` they select to there.
- **Send now** shows as `Enter Enter` in the footer where `Ctrl+Enter` arrives as plain `Enter`, such as inside tmux with its default settings.
- **`Esc` on a `/btw` side question that hasn't been answered yet** cancels it, instead of letting it run on and reopening the panel when the answer arrives.
- **Clicking a link or file that can't open**, such as over SSH with no display, shows why, for example "Can't open links over SSH".
- **`Left` on an empty prompt** opens the dashboard from any session or the welcome screen, not only from a session you opened from the dashboard.
- **The `/btw` question in the panel's title** can be selected with the mouse and is copied to the clipboard when you release, like the answer text.
- **Workflow status lines** say how many agents are still running instead of how many exist.
- **`Ctrl+L`** in the scrollback adds the selected message or highlighted text to the prompt as a quote; open Extensions with `/plugins` instead.

## Bug Fixes

- **Notes above a queued prompt**, such as a model switch result or a ready worktree, stay on screen when the prompt starts instead of scrolling out of view.
- **Stop buttons in the dock** stop a background run while its approval card is open, instead of doing nothing until you answer the card.
- **Rejecting a change** keeps edits you saved to the file yourself, even when Grok missed the save, instead of overwriting them.
- **`grok --resume <id>`** opens the session in the folder it was recorded in, wherever you launch Grok from, instead of refusing.
- **Windows**: on older consoles such as plain `cmd.exe` and Windows PowerShell windows, the `▶`, `⏸` and `◌` marks in todo, workflow and list rows show as characters the console can draw instead of empty boxes.
- **Generated image and video paths** with wide characters such as Chinese, Japanese or emoji fit their row and keep the file name, and Windows paths keep it too.
- **The rewind confirmation** no longer crashes Grok on a short terminal or when you shrink the window while it's open.
- **Footer and status bars** in narrow terminals drop or shorten the items that don't fit, instead of painting them over each other or cutting buttons off.
- **Cut-off Fetch, Web Search and MCP tool output** ends with the same `… +N lines · <key> to view all` row as code cells, naming your viewer key, and one hidden line reads `+1 line`.
- **16-color terminals** show the paste label and tool accents in the muted gray other secondary text uses, and the paste label's brackets no longer vanish on the light theme.
- **A question card's `Enter:submit` button**, the dashboard session popup's `[✗]` and collapsed workflow rows highlight under the mouse pointer like other clickable controls.
- **Emoji and keycap symbols** such as `⚠️` or `#️⃣` are measured at the width the terminal draws them, so rows no longer run past their box or lose their `…`.
- **`/export` to a named pipe** that nothing is reading fails right away instead of freezing Grok until it is killed.
- **Typing a digit or symbol** while the scrollback has focus, with vim keys off, moves focus to the prompt and types it there, as letters already do; `?` still opens the command palette.
- **`grok sessions delete`** no longer fails with "Directory not empty" when another process is still writing into the session's folder.
- **Dashboard rows** stay in Working while a session's background subagents or workflow runs are going, and stopping the row ends them.
- **Code blocks inside quotes** render every line under the quote's bars, quote copies turn reference links into inline links, and lines after multi-line math are numbered correctly.
- **Disabling a project's hook** in `/hooks` applies only to that repository, instead of also skipping hooks with the same name in other repositories.
- **Turning a skill on or off** in the Skills tab saves only `[skills].disabled` to `config.toml`, so an organization's managed skill folders and ignore list keep applying.
- **`/btw`** starts a side question only when it begins the message; a `/btw` later in the text goes to the main conversation as typed.


# 1.0.74 — 2026-11-02

## Features

- **Messages between subagents** appear as left-aligned rows under the bullet that name the sender and recipient, replacing the centered rows.
- **Shift+Enter** inserts a newline in VS Code and other xterm.js-based terminals that use the Kitty keyboard protocol.
- **`/mcps`**: when a browser can't open an MCP server's sign-in URL, the URL shows in a panel you can copy with `c` or the `[copy the url]` button.
- **`[Copy]` button** in a finished `/btw` panel and its folded transcript row copies the whole answer, even when only part of it is visible.
- **Shell approval cards** say when Auto-review blocked a command or couldn't review it, and `Ctrl+F` shows the reviewer's explanation.
- **`!` mode Tab completion** offers commands installed as symlinks, such as Homebrew, npm, rustup and Debian alternatives binaries.
- **Quotes in replies** get a `[Copy]` button that copies the whole quote as Markdown without its `>` markers.
- **Named subagent links** open on a plain click, so they work over SSH from macOS where a modified click can't reach Grok.
- **`@` paths outside the workspace**: an `@` path that starts with `/`, `~/` or `../` lists files from that folder; set `[ui] at_picker_outside_workspace = false` to turn it off.
- **Picked `@` files in the prompt** can be edited in place: `Backspace` at the end of one reopens the picker on its path, and `Left` or `Right` puts the cursor inside it to change the path.

## Bug Fixes

- **Built-in skills, agents and workflows** are refreshed when their local cache's manifest is missing or corrupt, instead of staying outdated or failing every sync.
- **Ctrl+Y** on a question card no longer closes it without an answer; while you type an answer, it pastes back text cut with `Ctrl+K`.
- **`/new` tabs** show the context window you just switched to before the new session replies.
- **Resumed sessions** show a background command's completion in the turn that ran it, with its real run time instead of about 0.0s.
- **`grok update`** runs when `GROK_SANDBOX`, `--sandbox` or config sets a sandbox profile, unless Grok is already running confined.
- **`/btw` on a very short terminal** no longer swallows arrow keys, Page Up/Down or clicks while its panel is too small to draw.
- **A code block's `[Copy]`** in the transcript copies while a queued prompt is open for editing, instead of copying nothing or closing the edit.
- **`grok logout`** stays signed out when another running Grok refreshes its sign-in, instead of signing you back in on the next launch.
- **Opening links and files** from the TUI no longer leaves a zombie process behind for each click.
- **A failed session delete** shows the "Couldn't delete session" prefix once, so the reason is no longer cut off.
- **Windows**: Grok exits fully after a startup error, a `/minimal` or `/fullscreen` switch, or a signal, instead of sometimes hanging.
- **Grok running confined in a sandbox** never starts an update on its own and tells you to run `grok update` instead.
- **Quitting to install an update** with `Ctrl+U` on the welcome screen exits within the usual exit deadline instead of sometimes hanging after "Update installed".
- **`grok update --force-reinstall`**, or `grok update --version` with the installed version, keeps your working `grok` on Linux and macOS when the download fails its checks, instead of deleting it.
- **Windows installers** check that the downloaded build runs before replacing `grok.exe` and `agent.exe`, and keep the existing install when it doesn't, such as when a proxy serves a blocked-download page.
- **Command-line arguments that aren't valid UTF-8** no longer crash `grok` at startup: paths such as `--cwd` accept them, and text such as `-p` gets a usage error.
- **Installers** keep the installed `grok` when a proxy or cache returns an empty download, instead of replacing it with an empty file and reporting success.
- **Environment variables with non-UTF-8 bytes** no longer stop Grok from starting.
- **Workflow run lines** in the transcript end with what the run returned instead of a bare colon, `/tasks` shows the time it was taken, and polling an unknown task id leaves a row with its error.
- **Headless runs stopped by Ctrl-C or SIGTERM** end their `json`, `streaming-json` or `streaming-messages-json` output with a closing record that keeps the run's outcome, instead of cutting off the stream.
- **macOS**: the TUI no longer freezes when it runs inside a sandbox that blocks the window server.
- **`grok -p --resume`** shows why a session couldn't load instead of always "Session does not exist", and with `--output-format json` a resume, continue or fork that fails before the session opens prints one JSON error.
- **Sessions with an unrecognized reasoning effort or unreadable bytes in their history** resume instead of failing: the session runs at the model's default effort and says so once, and the unreadable lines are left out of the transcript.
- **Windows**: on older consoles such as plain `cmd.exe` and Windows PowerShell windows, status marks and selection arrows in workflow views, the dashboard, and the feedback and extensions dialogs show as `>`, `•`, `√` or `x` instead of empty boxes.
- **File paths with spaces** in inline code, such as `~/Library/Application Support/...`, link to the whole path when it exists instead of stopping at the first space.
- **Feedback that can't be sent** leads with the outcome, such as "Couldn't send feedback. Saved to Drafts.", then gives the reason and says to retry from `/feedback`.
- **Dialog tab labels, shortcuts and settings rows** that don't fit end in `…` instead of being cut off.
- **Prompt history search** highlights matched letters in the same color as other fuzzy lists, and the counts on the `/` and `@` menu borders are dimmed like other border captions.
- **Dashboard**: the peek header reads `Responding` while a reply streams, and the `Esc` footer hint reads `new agent` like the other lowercase hints.
- **Custom agent files** saved with a UTF-8 byte order mark, as some Windows editors write them, load instead of being skipped.
- **Running markers** in workflow views and MCP servers that are still starting use the theme's running color, like other progress indicators.
- **Minimal mode**: "Sign-in failed" shows in the error color instead of the warning color.
- **Context window**: a larger window picked with `/context-window` or set with `context_window` in a `[model."<id>"]` table stays in effect, so compaction no longer starts early after the session sits idle, after a failed request such as a rate limit, or until you restart.

## Performance

- **Long sessions** use less memory: finished Markdown blocks far off screen release their rendered layout, and search still finds their text.
- **Typing** no longer lags when ffmpeg isn't installed and `PATH` holds slow directories, such as Windows drives under WSL.
- **Long pastes that arrive as keystrokes**, such as pastes on native Windows or in terminals without bracketed paste, no longer freeze the prompt for seconds while the plan-mode tip is on.
- **Pasting long single-line text** such as logs, JSON or base64 into the prompt no longer freezes the TUI for seconds.
- **File watching on Linux** no longer keeps a CPU core busy for about a minute after startup in large repositories.
- **Lower CPU use while subagents run**: the TUI repaints only for animations on screen, about 7 times a second instead of 30 when no running row is visible.
- **`grok --resume <id>`** opens much faster on large session stores, reading only the resumed session instead of every session.


# 1.0.73 — 2026-10-28

## Features

- **`Page Up`/`Page Down`, `Home` and `End`** move the selection in `/skills` and other pickers.
- **Messages between two subagents** appear in the main session's transcript as centered rows once delivered, such as `find callers → scan src/`.
- **Drag-selecting text in a `/btw` answer** scrolls the answer while you hold the pointer on or past the panel's edge.
- **Automatic updates** stop retrying a new version that fails to start on this machine until a newer version is out, and each launch says to run `grok update` to see why.
- **Gruvbox dark theme** (`gruvbox-dark-hard`), selectable with `/theme`.
- **Start in multiline** in `/settings` (`[ui] multiline_default`) starts new sessions and the dashboard in multiline mode, where `Enter` inserts a newline.

## Bug Fixes

- **Windows**: quitting Grok ends `grok.exe` once the terminal is restored, so the shell no longer sometimes waits with no prompt.
- **Mouse wheel** in a `/btw` answer reaches the last lines on short terminals.
- **Several Send nows in a row** no longer label the turns they stop "Turn cancelled by user" or jump the transcript view.
- **`--restore-code`**: when the code restore fails, your uncommitted and untracked files stay in the working tree instead of in a stash.
- **Windows**: shortcuts typed with `Shift`-held symbols or AZERTY digit keys work.
- **Queued images** show as green chips in the queue pane, as in the prompt.
- **Typing while the transcript has focus** moves any character, including non-ASCII text, to the prompt, and leaves a pending card or plan approval as it is.
- **Worktree creation notes** stay on screen when a queued prompt starts.
- **Session load errors** show the whole reason instead of cutting out the middle of a long message.
- **Lines with `|` that are not a Markdown table** wrap like other text instead of being cut off at the terminal edge.
- **Transcript selection** stays on the same row when a row above it is removed.
- **Kitty graphics protocol**: images Grok hides are freed in the terminal, so they no longer use its memory.
- **`--restore-code`** restores its own stash even when another worktree changes the stash list meanwhile, and keeps your changes stashed when a post-checkout hook fails after the checkout.

## Performance

- **Startup** no longer waits about 100 ms for a sign-in file when you use an API key or have not signed in yet.


# 1.0.72 — 2026-10-27

## Features

- **Relative file paths** in the agent's output, such as `src/app/main.rs`, open on click when the file exists under the session's folder, as absolute paths already do.
- **Subagent and background task ids** mentioned in the transcript open that subagent's view or the task's output on click.
- **Pull-request numbers** such as `#123` in the agent's replies open that pull request on the session repository's forge.
- **Subagents named in a reply** show as the subagent's label instead of its raw id once the reply finishes, as a link that opens its transcript.
- **`/workflow runs`** detail view: `→` or a click focuses a phase's agent list, `↑`/`↓` pick an agent, `Enter` opens it, and `←` returns to the phases.
- **Feedback draft rows** show on their own instead of inside a group of tool rows, and opening one shows the `/feedback` Drafts tab with that draft selected.

## Bug Fixes

- **Warp**: clicking a URL that wraps across rows opens the whole URL instead of only its first row.
- **`/delete`** confirmation starts on Cancel, so pressing `Enter` no longer deletes the session; press `1` to delete.
- **Max thoughts width** (`/settings`, `[ui].max_thoughts_width`) sets how wide thinking blocks wrap; it previously had no effect.
- **Dashboard** shows the new folder on the dispatch box's top border right after a `Ctrl+L` or `/cd` pick.
- **Custom models on the Responses API** with a non-xAI `base_url` no longer fail every turn with an HTTP 400 schema error for `use_tool`.
- **`/feedback` Drafts tab** keeps the selected draft when the list reloads, such as after leaving Drafts and coming back.
- **MCP tools built with FastMCP or the official MCP SDK** send their result to the model once instead of twice, so each call uses about half the context.
- **Plan review notes** accept every key, so a note that starts with `a` no longer approves the plan.
- **MCP servers over stdio** no longer lose part of a long reply line, such as a large tool list, when a read is interrupted.


# 1.0.71 — 2026-10-26

## Features

- **`ui.single_ctrl_c_quit = true`** lets a single `Ctrl+C` quit when no draft or unfinished work would be lost. It is also under Editor in `/settings`; the default still asks for a second press.
- **Image chips in the prompt** use the same green as sent images in the transcript.
- **Hovering an image chip** in the transcript shows the image that was sent, or why it was not attached.
- **Hooks** accept `"failClosed": true`: when such a `PreToolUse` or `UserPromptSubmit` hook times out or crashes, the tool call is denied or the prompt is blocked instead of going ahead.
- **Vim editing in the prompt** (experimental): set `[keys] vim = "all"` in `config.toml` for Insert and Normal modes with vim motions, operators, counts and undo. The prompt's info line shows `NORMAL` or `INSERT`.
- **Inline images** display in Noctty on Windows.
- **Answers to the agent's questions** show one row per question and answer, and end in `…` only when the text is cut.
- **Messages from other agents** appear as compact centered rows, and consecutive messages fold into one row.
- **Messages the agent sends to subagents** appear as the same centered rows (`you → <subagent>`) and fold together with the messages it receives.
- **Prompt vim cursor** is a block in Normal mode and a bar in Insert mode; `ui.cursor_blink` sets whether they blink.
- **Minimal mode** starts its info row with the prompt's vim mode (`NORMAL` or `INSERT`), and `NORMAL` is shown in bold.

## Bug Fixes

- **Image chips** in your messages stay whole on one row when the message wraps.
- **Images the server rejects** are removed from the conversation, so the notice that images were left out no longer repeats, and grows, on every later turn.
- **Feedback draft rows** show the draft's state: drafted, declined, cancelled or failed.
- **A slash menu closed with `Esc`** no longer reopens by itself when the command list refreshes, such as right after startup.
- **Turn failed rows** no longer repeat a generic "Request failed:" before the error.


# 1.0.70 — 2026-10-23

## Features

- **Answered questions** now have a blank line between them, except in compact mode (`/compact-mode`).
- **`GROK_DISABLE_EXPERIMENTS`** turns off every `[[experiments]]` entry in your configuration; `[experiments] enabled = false` in any configuration file does the same.

## Bug Fixes

- **`/plugins`** now counts only the plugin skills that load, and says how many were skipped because they link outside the plugin folder.


# 1.0.69 — 2026-10-23

## Features

- **Dashboard directory grouping** now shows a header with the folder path and agent count above each folder's agents.
- **Dashboard location** (branch and folder) now sits on the dispatch box's top border instead of its own header row.
- **`/resume` on the dashboard** now lists sessions from every folder, with the current folder and its checkouts first.
- **Welcome menu** now lists Resume session (`Ctrl+R`) above New worktree (`Ctrl+W`).

## Bug Fixes

- **`[ui] simple_mode`** is retired and no longer has any effect; sessions always open with the prompt focused.
- **`grok doctor`** and `/doctor` now point out a leftover `[ui] vim_mode` key (now `[keys] vim`) or the retired `[ui] simple_mode` key in your config.

## Performance

- **Quitting Grok** returns you to your shell sooner because it no longer waits out fixed intervals.


# 1.0.68 — 2026-10-22

## Features

- **Worktree create and fork** now list files that could not be copied and any mode changes that were lost.
- **Vim navigation** is now one setting, `[keys] vim` (`off` or `nav`), also set with `/vim-mode`; the older `[ui] vim_mode` still works.
- **Terminal theme** now draws a subtle band behind your prompts using the colors your terminal reports.
- **Mode-switch toast** now shows the mode name (Agent, Plan, Ask) in the mode's own color.
- **Change-directory picker** (`Ctrl+L` on the dashboard) opens with the path field unfocused, like the `/resume` search.
- **Image placeholders** in the transcript now show whether the actual image was sent or only the text was typed.

## Bug Fixes

- **Worktree create** now fails cleanly instead of falling back when it cannot copy tracked files or runs out of disk space.
- **Running Grok sessions** are no longer broken when an update deletes their binary.
- **Dashboard model picks** now keep their selected context window when spawning the next session.
- **Send Now prompts** and rewound turns now correctly restore and mark any images that were originally sent.
- **Resumed sessions** now show the correct sent or struck-through state for image placeholders.


# 1.0.67 — 2026-10-21

## Features

- **Worktrees** also use their main checkout's saved "always allow" answers, so you are not asked again for a tool you already allowed there. Set `[features] worktree_inherits_main_approvals = false` to turn this off; the main checkout's "never allow" answers apply either way.
- **Prompt suggestions** follow vim mode: with `[ui].vim_mode` on, `Tab` moves focus to the scrollback instead of accepting the suggestion. `→` or `Ctrl+→` accepts a suggestion, and `Esc` dismisses it.

## Bug Fixes

- **Headless `grok -p` runs** exit when their work is done instead of hanging when a background task or subagent finishes and its output is read in the same turn.
- **The theme picker in Settings** restores your previous theme when you leave it any way other than `Enter`, as `/theme` already does.
- **Quitting a session with hooks configured** is faster: the session-end `Stop` hook runs at the same time as the `SessionEnd` hooks and gets at most 1.5 seconds.


# 1.0.66 — 2026-10-20

## Bug Fixes

- **`/theme`** restores your previous theme when you leave the picker any way other than `Enter`, such as `Esc`, `Ctrl+U`, `Tab` then `Esc`, or opening Settings or the dashboard.
- **Selected and highlighted rows** stay readable on every theme because reverse-video text is never dimmed. This fixes faint descriptions in selected menu rows on the `terminal` theme in terminals such as Kitty.

## Performance

- **`@` file completion** shows results about 5 ms after you type instead of about 40 ms.


# 1.0.65 — 2026-10-20

## Features

- **`/resume` search** finds sessions from other folders, tolerates typos, and also matches the commands, file paths, and replies in a session.
- **`--permission-mode plan`** starts the session in plan mode, as documented, instead of the default mode.
- **The dashboard's `Ctrl+L` folder picker** lists up to 50 recent folders instead of 10.
- **`grok --help`** gives each command and option a one-line summary, and `grok help -h` and `grok help --help` print the help command's own help instead of an error.

## Bug Fixes

- **`/resume` search** keeps a single typed character literal and matches each typed word at the start of a word, so `log fix` no longer matches "Blog prefix cleanup".
- **`DISABLE_TELEMETRY` and `DISABLE_ERROR_REPORTING`** turn telemetry and crash reports off for any non-empty value, including `0` and `false`. A `GROK_TELEMETRY_ENABLED` or `GROK_ERROR_REPORTING` value Grok can't read also counts as off.


# 1.0.64 — 2026-10-19

## Breaking Changes

- **Untrusted folders** run only your organization's managed hooks. User, project, plugin, and agent hooks wait until you trust the folder. Headless sessions treat user hooks as trusted.

## Features

- **Worktree names** typed with `Ctrl+W` keep Unicode letters such as `café`, and a name or ref that starts with `-` is refused.
- **Auto mode** runs a shell command without review only when every command in it matches an allowlist entry; anything else asks first. Permission rules read each word the way bash passes it to the program.
- **`/resume`** loads older sessions when you reach the last row, and shows how many sessions are loaded.
- **Auto mode** runs `make --version`, `cmake --version`, and `just --version` without asking.
- **HTTP connections**, including remote MCP servers, accept gzip, brotli, zstd, and deflate compressed responses.
- **Plugin install suggestions** above the prompt are off when `disable_plugins = true` is set.

## Bug Fixes

- **New worktrees** keep tracked files that match `.gitignore`, and leave out files excluded only by `.git/info/exclude` or `core.excludesFile`.
- **A new session that fails to start** puts what you typed back in the prompt you return to, instead of losing it.
- **Web search** works from a session's first turn, and in subagents, when the search model signs in through an external auth provider.
- **`grok -w`, `grok worktree create`, worktree forks, and subagent worktrees** refuse up front with the sandbox's message when the sandbox profile does not allow creating worktrees.
- **Worktree options** are hidden in `/fork`, `/new`, and the Welcome screen when the sandbox profile does not allow creating worktrees.
- **A worktree name you type** that is already taken is refused instead of getting a numbered suffix, and a name whose folder you deleted by hand can be used again.
- **External OpenTelemetry export** keeps sending logs and metrics when `OTEL_EXPORTER_OTLP_COMPRESSION` is set to `gzip` or `zstd`.
- **`[model.*]` tables** without a context window take the window and API of another table that sends the same model, instead of the 200,000-token fallback.
- **Grok on macOS without a GUI session**, such as over SSH, no longer hangs on its first modifier-key check.
- **Custom models on the Responses API** keep the turn going when a web search call starts without an action.
- **Shell commands** run correctly on zsh when your startup files alias `grep` or replace `run-help`.
- **Voice dictation preview** grows the prompt as you speak and keeps the newest words in view.


# 1.0.63 — 2026-10-17

## Breaking Changes

- **Memory** always uses the concise `MEMORY.md` index of topic titles and paths, and `/memory` lists topics by how often Grok reads them. The `[memory_v2] compact_index_enabled` setting no longer has an effect.

## Features

- **Rex terminal support** reports each open session and each running subagent as its own record, so Rex shows what each one is doing. Set `children = false` under `[ui.notifications.program_status]` to report only the main status.
- **`grok worktree list` and `grok worktree show`** mark a worktree whose folder is missing or whose source repository moved, with a hint on how to repair it.
- **Command hooks** accept an `args` list: Grok runs `command` as one program with those arguments and no shell, and fills hook variables such as `$GROK_SESSION_ID` in each argument.

## Bug Fixes

- **Plan reviews** that open while you are typing keep your text in the review's notes, so a typed letter no longer triggers a plan shortcut such as approve.
- **Plan reviews** at the end of a turn wait until you close an open `/feedback` dialog, question, tool prompt, or queued prompt edit.
- **`grok -w` and `Ctrl+W`** explain that no worktree was made when you run them outside a git repository.
- **`/fork`** keeps the parent session's model and effort when Grok was started with `-m` or `--effort`.
- **Custom models on Chat Completions endpoints outside xAI** no longer fail turns with a tool schema error when MCP servers are connected.
- **Workflow run views** hide each agent's live status text on narrower terminals, so agent names are no longer cut off.


# 1.0.62 — 2026-10-16

## Bug Fixes

- **Headless mode** (`grok -p`) waits for the follow-up turn that a finished background task starts before exiting, and a `--background-wait-timeout` that stops pending work exits with an error instead of success.
- **Cold `/resume` from the dashboard overlay** keeps the overlay attached, so you can still back out with Left or `Ctrl+\`.
- **Enter on a highlighted slash command** with optional arguments runs the command instead of leaving a trailing space in the composer.
- **CLI commands** such as `grok models`, `grok sessions` and `grok export` no longer crash when their output pipe closes early, such as when piped to `head`, or when the output device is full.
- **`grok update`** and **`grok login`** no longer crash when stdout or stderr is closed or unwritable.


# 1.0.61 — 2026-10-16

## Features

- **Resume picker** finds sessions whose titles contain every word you type, in any order.
- **`/resume <id or name>`** opens the session directly when the ID or title matches exactly one session, instead of always opening the picker.

## Bug Fixes

- **Dashboard peek** text can be selected by dragging over it with the mouse, and the selection is copied to the clipboard when you release.
- **Workflow run views** scroll toward newer agents when you wheel down the agent roster, instead of snapping back.
- **Hook spawn failures** on Windows name the program that failed to start, such as PowerShell or Git Bash, instead of blaming the hook's script.


# 1.0.60 — 2026-10-15

## Features

- **`grok --resume <title>`** lists matching sessions newest first with when each was last updated, and suggests similar titles when nothing matches.
- **Shift+Y** in the fullscreen markdown viewer copies the code block under the cursor.

## Bug Fixes

- **`grok --resume <title>`** with a title that matches no local session reports the miss right away instead of trying a remote restore and asking you to sign in.
- **Esc** on the first question in a session opened from the dashboard moves focus to the scrollback, as it does in a standalone session, instead of returning to the dashboard.


# 1.0.59 — 2026-10-14

## Features

- **Dashboard `/loop`** without a prompt shows its usage instead of starting a new session.
- **`grok --resume <title>`** matches a title whatever separates its words, so `fix-login-bug` finds "Fix login bug".
- **`[Copy]` buttons** appear under code blocks in finished replies, labeled with the code's language or file name.

## Bug Fixes

- **Dashboard peek replies** waiting on a clipboard paste are canceled with a toast when you move the peek to another row or press Esc, instead of being sent later.
- **`grok worktree rm`** no longer suggests `-f` for a worktree that `-f` would also refuse to remove.
- **Advanced PowerShell inline hooks** that read `$input` run with a warning instead of failing.
- **Hooks** that return `continue: false` from PreToolUse, PostToolUse, or UserPromptSubmit stop the turn instead of being ignored.
- **Grok** no longer crashes at startup when the `SENTRY_DSN` environment variable holds an invalid value.
- **Copy rows** under code blocks, tables, and diagrams display correctly after switching between minimal and fullscreen modes.
- **Copying** a code block or a whole reply keeps its original tabs instead of turning them into spaces.


# 1.0.58 — 2026-10-14

## Bug Fixes

- Minor bug fixes.


# 1.0.57 — 2026-10-14

## Bug Fixes

- Minor bug fixes.


# 1.0.56 — 2026-10-13

## Bug Fixes

- Minor bug fixes.


# 1.0.55 — 2026-10-13

## Bug Fixes

- Minor bug fixes.


# 1.0.54 — 2026-10-12

## Features

- **`/resume`** takes an optional session id or title and opens the picker filtered to matching sessions.

## Bug Fixes

- **Pasted Windows line endings** reach the model as newlines, markdown images no longer show a stray "!", and dollar amounts that are not math stay plain text.
- **Write approval cards** show a binary or unreadable file as unreadable instead of garbled text.
- **Prompts you send while a session is resuming at startup** are queued and sent once it loads instead of staying in the composer.
- **A corrupt active-sessions registry** no longer stops new sessions from registering: grok keeps a backup, salvages the session rows it can read, and warns you.
- **Wrapped URLs** in shell command descriptions link to the full address on every row.
- **Worktree forks** (`grok -w -r` or `/fork` into a worktree) run in and show the worktree's path, so a shell command no longer edits the original checkout.
- **Links** are clickable in herdr panes.
- **Memory search** shows the matching part of a long line instead of cutting it off before the match.
- **`/memory`** skips a symlinked note instead of failing the whole listing.
- **Startup** is faster when `TMUX` names a tmux server that has stopped.
- **Failed MCP tool rows** wrap the error to the screen width and keep its line breaks.
- **`[skills].ignore`** matches a symlinked SKILL.md by the folder that holds it.
- **Failed file reads** expand to show why they failed.
- **Send now (Ctrl+Enter)** on a slash command runs or queues it as a command instead of sending it as a prompt.

## Performance

- **The idle welcome screen** uses less CPU.
- **Prompt undo history** is capped by size, so large pastes you delete no longer stay in memory.
- **Typing in a prompt that holds large pastes** is faster.
- **Long sessions** use less memory: long replies scrolled off screen release their render caches.


# 1.0.53 — 2026-10-12

## Features

- **Wrapped shell commands** in tool and background-task rows copy back as one line.
- **`/hooks`** lists load errors from the main agent's inline hooks and from plugins.
- **`--tools` and `--disallowed-tools`** also apply in the interactive TUI.
- **`/goal`** pauses after three continuations in a row with no tool call; run `/goal resume` to continue.
- **Quitting** warns when it would stop a running workflow and asks you to confirm with a second quit.
- **Multi-file edits** show how many other files the edit changed.

## Bug Fixes

- **Worktrees created from a repository subfolder** report the repository root as their source in `grok worktree list` and `grok worktree show`.
- **`/feedback`** stays open while you type when a permission request, question, or MCP prompt arrives.
- **Persistent shell** runs your command even when restoring its saved state fails.
- **`/fork`** is refused while an approval request is open, so the approval keys keep answering that request.
- **`grok --help`** and **`grok --version`** no longer create a `.grok` folder.
- **Wrapped URLs** in tool headers link to the full address on every row.
- **search_replace** refuses an `old_string` that matches again at an overlapping position, so it no longer edits the wrong match.
- **read_file** reports the correct line count for files that end with a newline and no longer returns an empty line past the end.
- **`/workflow stop`** and **`/workflow pause`** no longer pick a run from a loose fuzzy match, and headless `-p` runs print each refused approval request to stderr.
- **Workflow run views** list agents outside a declared phase and count only finished agents as done.
- **Parallel MCP tool calls** that ask for input each show their own prompt.
- **`/feedback` Drafts** load the valid drafts when one saved draft is invalid.
- **`grok sessions list`** and `/resume` keep working when one session directory is unreadable.
- **`grok plugin install`** interrupted mid-clone on Linux no longer leaves a partial folder that blocks the retry.
- **Hooks you disabled** stay disabled when the disabled-hooks file cannot be read.
- **Edit diffs** mark removed and added lines with `-` and `+` when color is off.
- **Session usage** keeps every turn when two windows write to the same session.
- **Failed directory listings** expand to show why they failed.
- **Stdio MCP servers** expand a leading `~` in their command and arguments to your home directory.
- **Editing a queued command** such as `/compact` is refused with a message, so your new text is kept.
- **`/docs`** opens the exact guide or picker you type.
- **`grok inspect`** warns when a known `[features]` key in config.toml has a non-boolean value.
- **Send now (Ctrl+Enter)** no longer answers the stop-subagents panel or an MCP prompt card.

## Performance

- **@-mentions** reuse the file list until the folder tree changes, so suggestions appear faster.


# 1.0.52 — 2026-10-11

## Features

- **Rex terminal support**: Grok now reports its status (working, waiting for you, done, or failed) to Rex through OSC 7501, so Rex tabs show what each session is doing.
- **Tool call density improvements**: expanded tool call groups now stay expanded as new calls join them, and Ctrl+E expands or collapses thinking faster in long transcripts.
- Project hooks placed in a repository subfolder are now loaded when you launch inside that folder.
- HTTP hooks to localhost now work, and bad hook URLs are reported at load time.
- Command status-line scripts now see the current turn's prompt ID and start time while the turn is active.

## Bug Fixes

- **Grep searches** that time out or fail now show an error instead of appearing as empty results.
- **`/model` switches** that fail no longer change the footer or saved default model.
- **Export and copy** commands no longer overwrite an existing file; they refuse with a clear message.
- **Shell commands** no longer keep running after you quit grok or it receives a signal.
- Approval cards for background subagents no longer disappear when the parent turn ends.
- `grok worktree rm` and `gc` no longer delete a worktree that an open session is using.
- **grok inspect** no longer warns about valid pins that only requirements.toml accepts.
- **Pasted images** that exceed the 50 MB send limit are now rejected with a clear message instead of silently dropped.
- `grok -p --effort` now prints a warning when the model ignores the flag and refuses invalid effort levels.
- Permission prompts now show which session needs approval.
- **grok worktree rm** now refuses to delete git repositories, .git folders, or any directory grok did not create.
- **Grep** and **list_dir** now report a clear error when the search root or directory cannot be read instead of silently returning no results.
- **Cancelled terminal commands** now show the actual command that was stopped.
- **Wrapped URLs** in prompts and quotes now link to the complete address instead of being cut at the wrap.
- **@mentions and image placeholders** now respect Read deny rules and are omitted from the prompt.
- **Restarting** after a model switch no longer reverts to the previous model.
- Long tool turns that include PostToolUse hook output now compact correctly instead of hitting the prompt length limit.
- Headless prompts that start with a byte-order mark no longer send the mark to the model or break JSON prompt files.
- When a permission rule blocks a tool call, any context returned by a PreToolUse hook still reaches the model.
- An invalid shell_environment_policy table in config.toml now causes grok to exit with a clear error instead of silently using the full environment.
- MCP tool replies containing invalid characters no longer cause the call to time out.
- **Context overflow errors** now trigger automatic compaction instead of failing the turn.
- Inline tags in agent replies no longer break line layout, and /workflow stop now picks the correct live run.
- **Auto-compaction** now stops after three context refills in a row instead of looping forever.
- **Minimal mode** now shows the permission prompt instead of a hidden plan or cancel panel when both are present.
- **Project rules files** that resolve outside the git root or workspace folder are now skipped.
- **[permission] rules** now load alongside compact deny/allow/ask lists in config.toml.
- **Unrecognized permission keys** or [permissions] sections now appear as skipped entries in grok inspect.
- **Approval cards** now close the block viewer so the card is visible.
- Shell redirects to wildcard or variable paths now correctly respect deny rules in Auto mode.
- The in-session resume picker now matches the ungrouped layout shown on the welcome screen when configured.
- HTTP MCP tools that return a JSON-RPC error are now executed only once.
- StopFailure hook matchers containing commas or spaces are now treated as regex patterns.

## Performance

- Multiple Notification hook chains now start in parallel instead of waiting for the previous chain to finish.


# 1.0.51 — 2026-10-09

## Breaking Changes

- Cancel and edit-resend on long sessions no longer unexpectedly revert workspace files.
- **Memory settings** simplified: rollout and file_writes_enabled options removed; memory is on or off.
- **Removed the `/share` command** and related session sharing features from the CLI and SDK.

## Features

- **Inline viewports** can now be restored at a previously saved screen area without extra cursor queries.
- Headless `grok -p` now waits for background goals to finish and reports their outcome.
- **`/imagine`** and similar commands now run the matching bundled skill when the required tools are unavailable.
- **New `/density` command** and Tool call density setting let you choose how much detail tool calls show in the transcript.
- **Web fetch approval cards** now include "always allow" and "never allow" domain options that persist for the folder.
- **Slash command completion** now updates live when skills are added or changed during a session.
- **Request form** now securely collects and fills sensitive details like passwords or card numbers without sending values to the model.
- **Chrome profiles** can now be persisted across restarts using durable state directories.
- Workspace server sessions now share one Chrome instance for cookies and storage.
- **`/feedback` Drafts tab** now supports `G` / `Home` and `End` to jump to the first or last draft.
- **Pinned sessions** on the dashboard now stay in the order you pinned or reordered them.
- **Shift+Enter, Ctrl+., Ctrl+/ and Ctrl+[ / ]** now work inside tmux when extended-keys are enabled.
- **grok -p --output-format json** now reports the exact worktree folder used (useful after -w name collisions).
- Bare hook commands such as `my-guard` now run from PATH when not found next to the hook file.
- Headless JSON error output now includes the worktree folder when a session was started.
- **grok sessions list** and /resume now show worktree sessions started from the same subfolder.
- **/exit** (and aliases) in a dashboard details view now closes only that session instead of quitting Grok.
- **Editors offering file access** now receive a one-time transcript notice that the agent reads and writes disk files.
- **--background-wait-timeout** now also stops an active headless goal after the timeout and exits with an error.
- **Interactive sessions** now include the Grok Build user guide location so the agent can answer usage questions.
- **New `/workflow` commands** let you list, stop, and resume background workflow runs without sending them to the model.
- **Background workflow runs** now appear in the tasks pane with a stop button.
- Hooks that ask about web searches or file deletes now show the hook name on the approval card.
- config.toml now accepts an [update] section without breaking other commands.
- Hook load errors listed in /hooks can now be expanded to see the full message.
- `grok worktree` commands now work correctly inside Jujutsu repositories.
- Images pasted with skill commands like /pr-workflow are now sent to the model.
- Pressing Ctrl+C on a goal now pauses the goal instead of letting it continue.
- grok -p now runs auth_provider_command when no login is stored.
- Reopening a chat with an active goal now continues the goal automatically.
- Browser tabs can now be restored on demand instead of all at once when a session resumes.
- `grok agent stdio` now supports listing sessions via ACP and returns consistent reply envelopes.
- Answering an approval with a message now tells the model it is a follow-up.
- **--allowed-tools** is now accepted as an alias for --allow in headless mode.

## Bug Fixes

- Slash command suggestions no longer make the text after the cursor disappear while typing.
- Running workflows in the dock now show the same pulsing indicator as in the main scrollback.
- Mode switch tool calls now appear in the transcript with the target mode name.
- Always-approve mode now automatically approves web requests without extra prompts.
- **The follow indicator no longer appears** when only blank padding sits below the viewport.
- **Search failures now keep the original pattern** instead of showing an empty Search "".
- **Sessions that open without a model list now use the public Auto id** and succeed on every account.
- Background wake turns that request plan changes or mode switches are now handled correctly.
- Model-call retries now show progress status instead of a generic waiting message.
- Foreground subagent activity now appears correctly after resume.
- Very large pasted prompts are now offloaded to a file instead of being sent inline.
- Sign-in failures now show a clear plain-English message with the next step instead of raw errors.
- Logging out now stops any running turns on local sessions instead of leaving them active.
- The send-now shortcut no longer accidentally enables always-approve mode while a permission card is open.
- `/find` now keeps match counts and navigation accurate while replies or command output stream in.
- Images from tools are now validated so bad files no longer break the current turn or future replays.
- Screenshots and other image artifacts saved by the browser tool are now stored as real image files.
- Rate-limit errors now show the expected message; very small images are rejected early.
- Send-now (Ctrl+Enter) no longer accidentally answers an open question card.
- Opening the queue with Up now scrolls so the selected follow-up is visible.
- Command output that contains clear-screen codes is no longer discarded.
- **Dashboard cursor and overlay counter** now correctly track the active grouping view.
- **Headless JSON output** now reports the correct stop reason when the turn limit is reached and removes duplicate error prefixes.
- **Session picker footer** no longer lists the same shortcut twice.
- **Dragging images** from a file manager onto terminals like GNOME Terminal or xfce4-terminal now attaches them correctly.
- **Plan comments** added while a revision runs are now preserved and sent when the plan is approved.
- **Permission prompts** that fail now show a clearer message without blaming the interface.
- **--disallowed-tools** now accepts the exact names shown to the model (run_terminal_command, spawn_subagent, etc.).
- Long pastes without bracketed-paste markers are now read completely instead of stalling after ~1 KB.
- A feedback send now shows "Sending feedback…" immediately; failed sends no longer repeat the lead phrase.
- Typing, arrows or Enter while an approval card appears no longer accidentally approve the action or enable always-approve mode.
- **Error hints** for legacy session import now appear in full instead of being cut off by the pager.
- **Skills** in the folder where grok started now correctly show as (local).
- **Worktree creation** no longer hangs when the repo contains named pipes or sockets.
- **--no-subagents** flag now works correctly in headless / -p sessions.
- **Resumed sessions** now correctly show aborted subagent tasks as cancelled instead of failed.
- **Large parallel shell batches** no longer drop calls with "hook state lost" errors.
- **Write approval cards** now show the existing file contents that will be replaced.
- **Slash command completion** now shows the correct name for files under commands/ instead of always showing /commands.
- **PreToolUse hooks** now validate rewrites against the tool schema and deny invalid ones before running the tool.
- **Ctrl+C during a turn** now cancels the turn even when a modal such as /mcps or the block viewer is open.
- **grok -w** now opens the session at the worktree root (with a note) when the original launch folder does not exist in the new worktree.
- **`/cd`** and the location picker now accept quoted or escaped folder paths.
- Web search hooks now fire under the correct `web_search` name and carry `query`.
- Large file writes or many screenshots no longer end the session.
- Glob tool hooks now match under `list_dir` and carry the correct directory.
- **`/find`** now scrolls to the exact wrapped row containing the current match.
- Downloads via FetchMcpResource are now refused in plan mode and resolve paths consistently.
- Pinned dashboard sessions now survive restarts and layout changes instead of disappearing from the saved list.
- Slash commands like /imagine now work regardless of capitalization and correctly substitute skills when tools are missing.
- Turning off bypass mode without naming a session now correctly restores prompts in all open sessions.
- /delete with extra text now shows an error instead of deleting the current session.
- **Failed /feedback** status messages now wrap inside the dialog instead of clipping at the border.
- **Number-key picks in the dashboard peek** no longer leak to the wrong agent after Up/Down navigation.
- **Worktrees created with -w** are now correctly tracked and resumable even when GROK_HOME is reached through a symlink.
- `grok worktree rm` and `gc` now clean up names after manually deleting a worktree folder so `-w NAME` works again.
- Hook scripts that read stdin with `read` now receive a complete line.
- A policy that disables non-managed hooks now applies to the next session instead of only after restart.
- `/resume` and `grok sessions list` no longer show worktree sessions from a sibling submodule or separate-git-dir repo.
- A hook listed under two different matchers now runs only once per tool call.
- Typing a hidden command such as `/cd` in a session and pressing Enter now runs it instead of accepting a near-match skill.
- **@-file completion** no longer accepts a match from an earlier query when you type quickly.
- **Tool hooks** now run one at a time per session and stop promptly on cancel or exit.
- **PostToolUse hooks** now receive a correct whole-millisecond duration.
- **PreToolUse hooks** for shell commands now see the model's block_until_ms value.
- **MCP tool calls** now show hook permission prompts and cards name the hook that asked.
- **Typing while a permission or ask card is visible** no longer selects or answers the card.
- **grok worktree list** now shows unfinished copies left by an interrupted create.
- **Auto mode** now asks before deleting files that the classifier has not already allowed.
- **Fixed intermittent** List/Read/Open failures during concurrent v2 hardening migrations.
- **grok -p -s** with a reused session id now shows the plain refusal message instead of JSON-RPC noise.
- **Settings saves** no longer re-insert the built-in default fork model into config.toml.
- **Fixed several** pager rows that showed wrong paths, truncated names, or hid failure/cancel states.
- **--system-prompt** now correctly appends rules from --append-system-prompt instead of dropping them.
- **/new** now inherits always-approve mode from the tab it was run from instead of another tab.
- **Multi-select questions** now let you toggle several answers with number keys before submitting.
- **Slash-command notices** typed while a reply streams no longer hide the streaming reply.
- **@ file search** no longer accepts keys or moves the highlight while results are still updating.
- **Worktree db rebuild** now restores names so `rm` and `show` by name work again.
- A prompt blocked by a hook no longer titles the session.
- **Background subagents** now respect saved approvals and --always-approve after the parent turn finishes.
- **Per-session system prompt overrides** now correctly use only the session's own rules.
- **Connection stalled** errors now show a clearer message with network advice.
- **Invalid model provider tables** now produce a clear startup error instead of silent fallback.
- **Narrow feedback dialog** now shows full draft titles and readable confirm prompts.
- **/workflow stop|resume <name>** now correctly targets the live run instead of an ended first run.
- Dialogs like /feedback no longer disappear on very short terminal windows.
- Delete operations now respect the same permission rules as shell rm commands.
- Hooks can no longer rewrite a web-search call to restrict domains.
- Workspace server now starts with per-session private Chrome by default.
- Notification hooks for permission prompts and task completion now run reliably before a headless session exits.
- Headless goal runs no longer print a spurious "Goal paused" message after completion.
- Legacy UI density settings now combine correctly across config tiers and produce expected folding.
- Approval answers sent after a turn ends no longer affect the turn.
- Background shell commands now fail immediately when the working directory does not exist.
- `grok worktree rm` without `-f` now safely keeps worktrees when the unsaved-work check fails or times out.
- The dashboard slash menu now shows skills from the home session even when opened early.
- `grok worktree rm` and `gc` now correctly remove worktrees whose only untracked content is empty folders.
- Editing a queued prompt now prevents accidental removal with a stale client view.
- The model is now told when memory is turned off or on for the session.
- Hook results in the UI now correctly distinguish between notes and tool outcomes.
- Permission cards for MCP tools can no longer be tricked into showing fake option rows.
- The agent stdio mode now returns clearer errors for malformed client requests and consistent recap replies.
- Scroll logging and acceleration calculations no longer produce backwards timestamps under load.
- Hooks and tool commands started from the TUI can no longer read or write the interactive terminal.
- Delete operations now correctly match `Bash(rm …)` ask/deny rules even when the path contains `..` or the cwd is a symlink.
- **Web search and image approval cards** now only offer always-approve mode to clients that support it.
- Closing a session in the dashboard no longer leaves it listed under Inactive.
- Running /compact on an empty session no longer resets the reported context window.
- With goals disabled, a headless `/goal` line is now treated as a normal prompt instead of failing.
- **Background shell commands** no longer flip back to running when late output arrives.
- Only slash commands at the start of a message trigger skills.
- **Browser tabs** restore and close more reliably with fewer races.
- Fast model names in the footer no longer show double parentheses.
- SubagentStart and Notification hook activity is no longer reported to the UI.
- **Voice error messages** now display with correct punctuation.
- **disable_zdr_incompatible_tools** changes now apply to new turns without restarting the worker.
- **Project skills** are now labeled correctly for nested projects.

## Performance

- Opening the resume picker is now much faster on machines with many worktrees.
- **grok worktree commands start faster** by skipping unnecessary model catalog work at boot.


# 1.0.50 — 2026-10-06

## Breaking Changes

- **grok worktree rm** without -f now refuses to delete worktrees that are in use or contain unsaved changes and exits with an error.

## Features

- **Compact mode** now collapses recaps and keeps inner spacing on tall terminals.
- **Clicking the git branch** in the header now copies the full branch name.
- **Long branch names** now abbreviate on narrow terminals while remaining copyable.
- **Targeted cancels** now stop only the specific response when requested by clients.
- **Compact mode** now hides the dock (or tasks pane) the same way Ctrl+G does.
- New **timestamps** setting offers off, minimal, and on modes in Settings and `/timestamps`.
- **Shell command output** now shows "Running …" while executing and "Ran …" when finished.
- **Live preview pins** now include whether an app was observed behind the URL.
- **Command palette (Ctrl+P)** and model picker now open at the same width as the settings modal.
- Tables rendered by grok now show copy buttons underneath that export clean TSV, CSV or Markdown suitable for spreadsheets.
- Dashboard now shows which worktree each session belongs to, displays the `-m` model in the new-agent box, and lets you toggle worktree mode in the folder picker.

## Bug Fixes

- **Shell permission decisions** now correctly prompt or approve commands matching the permission manager.
- **Cancelled turns** now correctly mark unanswered tool calls as cancelled instead of succeeded.
- **Long error messages** in the /usage and /session-info modals now wrap instead of being truncated.
- **Em-dashes** no longer appear in status messages and subagent rows.
- **Stalled model calls** during turns now retry before failing the turn.
- **Queue pane** and Ctrl+; now appear and work when the dock is hidden with Ctrl+G.
- **Long sessions** no longer disconnect when the conversation exceeds 4 MiB.
- **Permission rules** now accept native tool names such as `run_terminal_command` and `Shell`.
- **/rewind** now places the cursor at the end of the restored prompt.
- **Failed file writes** now leave the previous contents instead of an empty file.
- **Fixed /loop and scheduler_create** so non-ASCII intervals like 5分 no longer crash the session.
- **Fixed --disallowed-tools** when only one scheduler tool is named; the other two no longer cause startup failure.
- **Fixed closing a busy tab** so queued prompts are no longer sent to the model.
- **Fixed relative --cwd** so `grok --cwd a` no longer ends up running inside `a/a`.
- The terminal title now shows "Action Required" for any background session waiting on approval.
- `grok` now accepts a prompt via piped stdin in headless mode instead of trying to open the UI.
- An admin-set `[features] title_refresh = false` in requirements.toml now correctly disables title refresh even when GROK_TITLE_REFRESH is set.
- Custom notification commands defined under [[ui.notifications.hooks]] now execute on matching events.
- An admin pin of `[cli] use_leader = false` now correctly prevents `--leader` from starting a leader process.
- When multiple MCP servers expose a tool with the same name, `search_tool` now returns all of them.
- Concurrent Grok windows editing settings no longer lose or corrupt config.toml entries such as MCP servers or permission rules.
- **web_fetch** no longer downloads huge files into memory before rejecting them.
- **memory_get** no longer returns huge files that exceed the read limit.
- **Continue to run** no longer kills commands running inside background subagents.
- **memory_search** now finds Japanese, Chinese, and Korean words inside longer notes.
- Drafts saved by the model now appear under their actual titles in /feedback.
- Imagine replies that are not valid images are now rejected instead of being saved as .jpg files.
- A workflow file that fails to parse now reports the exact file and parse error instead of 'unknown workflow'.
- **Re-encoded images** are now oriented correctly using their EXIF tags instead of appearing sideways.
- **Image and log paths** reported after `/fork --worktree` now point to real files instead of missing locations.
- **Tab characters** in terminal command output are now preserved when shown to the model.
- **Transparent images** are now kept as PNG when re-encoded instead of turning black.
- **The bundled ripgrep** is now available to terminal commands even on images that do not install it.
- **Request bodies** now stay under a small configured `max_request_bytes` limit instead of growing past it.
- **Effort level** now stays the same after a bare `/model` switch instead of jumping to the new model's default.
- **Ctrl+\** and other modified digits no longer accidentally select options on permission or cancel cards.
- **Feedback submissions** now show accurate success or failure messages without premature thanks or duplicated prefixes.
- **grok doctor** now reports the exact config file or setting that prevents Grok from starting instead of saying zero issues.
- **Recap summaries** now appear on screen automatically instead of requiring manual scrolling.
- **grok doctor fix** now opens its confirmation card on Cancel to prevent accidental edits to shell or tmux config.
- **Slash commands** now run regardless of case (e.g. /COMPACT or /Compact).
- **/context** no longer shows an invented auto-compact threshold when the agent does not report one.
- **Agent text** no longer keeps the pager redrawing when a tool call interrupts the sentence.
- **CLI tool calls** no longer fail or overflow when the workspace directory is deleted or output is huge.
- **Logs written to files or when NO_COLOR is set** are now plain text instead of containing color codes.
- **Prompts now fail immediately** with a clear message if the session's working directory no longer exists instead of failing later or recreating it.
- **grok -w** now works correctly with separate-git-dir repositories and shows clear errors when run outside a git repo or on an empty repo.
- **Shell tool descriptions** now correctly state the actual output truncation limit, and grep respects globs relative to the current session directory.
- **Compaction messages** now scroll into view instead of appearing below the bottom of the screen after a turn finishes.
- **Git-mode worktrees created with -w** now preserve uncommitted changes instead of dropping them.
- **Worktrees created with --worktree-ref** no longer contain wrong submodule contents or LFS pointer text.
- Worktree creation now shows the same clear error messages on every code path.
- Failed worktree creates now show a clear error on the home screen instead of being hidden.
- Merge and rebase conflicts now appear in the changes list instead of being hidden.
- Fixed a crash when starting a new session with durable rewind checkpoints enabled.
- Managed permission files with one bad key no longer lose all their valid rules.
- Simultaneous worktree creates with the same label now succeed instead of one failing with "already exists".
- `grok worktree rm` without `-f` no longer deletes arbitrary folders that happen to share a name with a removed worktree.
- Discard now affects only the exact files you select instead of also deleting or reverting similarly-named files.
- **Permission rules** with one bad entry no longer lose the other rules in the same section.
- **Chat Completions streams** that end without a finish reason are now retried instead of returning partial output.
- **grok worktree rm** by label now refuses when the label exists in multiple repos.
- **Grep** now respects Read deny rules that use a leading `./`.

## Performance

- Command hooks no longer cause large memory spikes when they print a lot of output.
- **Retry waits** now vary between processes instead of being identical.


# 1.0.49 — 2026-10-02

## Bug Fixes

- Hook permission prompts now appear regardless of approval mode.
- **Web searches, fetches, and image generation** now correctly respect the prompt_policy setting and rule decisions before prompting.
- **Allow once** in permission cards now applies only to the next shell command instead of the whole session.
- **pkill -f** and **pgrep -f** commands are now rejected earlier when the pattern would kill their own shell wrapper.


# 1.0.48 — 2026-10-02

## Bug Fixes

- **`/rewind`** now only lists prompts you typed yourself and skips automatic runtime turns.
- Custom model tables in config.toml now preserve the original model's notice banner.
- `read_file`, `search_replace`, and `grep` now handle large files, special files, and binary content more reliably and surface better errors.
- **Ask rules for WebSearch and WebFetch** now correctly show a prompt instead of auto-running.
- **Cancelling one tool call** no longer kills commands started by other parallel calls.
- **Authentication recovery** no longer automatically retries refused prompts after login; the sign-in block now requires manual resend.


# 1.0.47 — 2026-10-01

## Features

- **Model switch responses** from the agent now update the displayed model list and context window.

## Bug Fixes

- Error messages on the home and login screens now wrap properly in narrow terminals.
- **Clicking the Goal chip** now opens the goal panel even when workflow runs are present.

## Performance

- **Sessions** in repositories with an .envrc file now start faster.


# 1.0.46 — 2026-09-30

## Bug Fixes

- **MCP server sources** are now correctly reported by `grok inspect` and the MCP doctor.
- **Tool search** activity now correctly folds as "Ran N tool searches" instead of claiming MCP tool results.
- **Client rules** supplied at session start are now preserved when the system prompt is rebuilt later.
- **Permission rules** with relative paths now correctly apply even when the cwd contains symlinks.
- **Context compacted** line now shows the correct token count immediately after compaction instead of at the end of a long turn.
- **Enter on video cards** no longer attempts playback in terminals that cannot show it or lack ffmpeg.
- **Video cards** now open the file in the default player on Enter in terminals without inline playback support, and no longer get stuck trying to extract posters without ffmpeg.

## Performance

- list_dir on network or virtual filesystems now skips slow subtree counts and prints a notice instead.
- **Session startup** is faster when loading skills from large directories and shows what it is doing instead of a generic spinner.


# 1.0.45 — 2026-09-29

## Features

- **Custom agents** from plugins or your config can now be chosen directly with spawn_subagent.
- **MCP servers** can now use a token file that is re-read on every request so rotating credentials stay fresh.
- **Models can now show** a colored notice banner above the prompt while selected.

## Bug Fixes

- **Subagent wait status** now correctly shows "Waiting for N subagents…" when the parent turn is blocked.


# 1.0.44 — 2026-09-28

## Features

- **Sandbox enforcement** on macOS now uses the system Seatbelt (`sandbox-exec`) backend.
- **Models can now advertise multiple context window sizes**; the first listed value is the default.
- **New `/context-window` slash command** lets you pick a context window size for the current session when the model offers choices.
- **`/model` selection now offers context window choices** (when the model supports them) before the effort level.

## Bug Fixes

- **Plan comments** can now be deleted by clicking the [✗] button that appears on hover.
- **Shortcuts panel search** now finds entries when you type any of the words in the label, keys, description or help text.
- **Fixed crashes** when pasting on Windows caused by overlapping clipboard operations.

## Performance

- **Parallel tool calls** that edit the same file now run safely in sequence; unrelated tools stay concurrent.


# 1.0.43 — 2026-09-27

## Bug Fixes

- **MCP tool prompts** now appear correctly in minimal mode and users are notified when a prompt goes unanswered.


# 1.0.42 — 2026-09-26

## Features

- Footer and `/usage` now display the model actually served on smart-auto turns.
- **New `grok worktree create` command** creates a managed worktree without launching an interactive session.
- **Auto permission mode** now appears as Auto-review everywhere in the UI.

## Bug Fixes

- **Bracketed pastes** that did not come from the clipboard no longer attach leftover images (e.g. from IME commits).
- `grok -p` now exits cleanly with the documented code when interrupted and performs session cleanup instead of being killed.
- Pressing Ctrl+C twice while writing a plan comment now cancels the comment instead of getting stuck.
- **`grok --minimal`** now reprints history correctly after terminal resize and keeps the prompt/status rows anchored.
- **Restoring a stashed draft** now places the cursor at the end so continued typing appends correctly.
- **`grok update`** on WinGet installs now prints the `winget upgrade` command and exits without writing files.
- Inline images are re-uploaded after a full repaint so they no longer appear blank in terminals that drop image data on ESC[2J.
- Plugin hooks now run on a fresh session without requiring an explicit reload.
- **Fixed plugin auto-update detection** when marketplace URLs differ only by .git suffix or host case.


# 1.0.41 — 2026-09-22

## Features

- **Subagent model inheritance** setting added to /settings; persists in config.toml and respects managed/overlay layers.
- **Per-model request size limits** can now be configured to match provider HTTP body caps and control inline image eviction.

## Bug Fixes

- **Fixed** the subagent fullscreen view so it no longer shows a stray [Dashboard] button in the header.
- **Fixed** agent frontmatter `mcpServers` so that headers and URLs from the active agent's agent.md now correctly override config.toml and survive config reloads or agent switches.
- **Ctrl+P** now opens the command palette immediately from the welcome screen.
- **Dashboard** now focuses the "+ New Agent" row instead of leaving the previous session selected.
- Sessions that were interrupted by a crash now show a clear marker instead of silently dropping the turn.
- **Ctrl+Z** right after stashing a prompt now restores it.
- **Collapsed edit blocks** setting now correctly collapses edits even if `expanded_by_default` was pinned true.
- Interjections now receive a visible reply before the agent resumes prior tasks.
- Canceling a turn that blocked on spawn_subagent now tells the model the child moved to the background instead of claiming it was never executed.
- Saving a queued-prompt edit now returns focus to the composer so the next keys type the next message.
- Subagent activation is now consistent between the TUI and `grok agent stdio`; tables that only set limits or models no longer disable subagents.
- **Effort level selection** now accepts menu labels in addition to IDs.


# 1.0.40 — 2026-09-20

## Bug Fixes

- **Miscellaneous bug fixes and updates**.


# 1.0.39 — 2026-09-20

## Features

- **Subagents** can always use the parent model, with that choice locked in when the session starts.
- Earlier image attachments now survive multiple compactions via a persisted path list.
- **Subagent spawning** no longer requires choosing a type; omitted calls default to general-purpose.
- The agent now keeps helper scripts, logs, and PR drafts in the system temp directory instead of the repository.
- read_file descriptions now tell the model when offset/limit are ignored for SKILL.md and instruction files.
- Effort levels for models now come from the API instead of hard-coded lists, and config aliases inherit the menu.
- MCP tools that receive the wrong JSON shape are now automatically fixed when the mismatch is unambiguous.

## Bug Fixes

- Local worktree capture no longer refuses repositories that contain uninitialized submodules.
- **Memory captures** no longer include opaque model reasoning blobs.
- Idle timeouts now show a clear message instead of raw internal text.
- Images attached to the last user prompt now survive compaction.
- **Custom agent profiles** now persist correctly across session resume and reload.
- **Subagent labels** now appear as "Subagent" instead of "General" in transcripts and the tasks pane.


# 1.0.38 — 2026-09-19

## Features

- **Long agent replies** in prompt-suggestion transcripts are no longer cut off mid-sentence.
- **read_file** on skill and instruction files can now be configured per deployment.
- **Pasted images** survive yank/undo/history recall and the user is notified when any cannot be sent.

## Bug Fixes

- **Subagent overlays** no longer stay stuck showing "Cancelling" after the child turn ends.
- **Long quoted arguments** in permission prompts now wrap instead of clipping.
- **Fixed tool-calling errors** by sending inline-only use_tool schema.


# 1.0.37 — 2026-09-18

## Bug Fixes

- **Mermaid flowcharts** with complex labels or large sizes now render and open correctly.
- **`/context`** legend now shows accurate percentages that always add up to 100%.
- **Pressing `y`** on a queue row now copies the prompt text to the clipboard.
- **`/model`** effort picker now defaults to the model's recommended effort level.
- **Bash commands** using quoted filename variables now respect configured permission rules instead of always prompting.


# 1.0.36 — 2026-09-17

## Features

- **New Dashboard preview setting** lets you hide the selected-session preview panel on the dashboard.
- **New policy setting** allows organizations to disable hooks that are not from managed policy.
- **Background shell commands** now appear as live task rows with streaming output.

## Bug Fixes

- **`grok --sandbox`** no longer exits when accepting folder trust.
- **Pinned agents** in the dashboard no longer jump when their activity changes.
- **Swift code** with triple-quoted strings now highlights correctly in the pager.
- **Mixed-case reasoning_effort** values (e.g. "xHigh") are now accepted in config.toml.
- **Fixed accidental empty agent sessions** when pressing Enter right after sending from the dashboard.
- **Fixed default TUI sessions** so an [agent] set in config.toml is respected instead of always using a builtin plan.
- **MCP plugin authentication** now succeeds for servers that declare a client ID in their manifest.
- **Shell command approvals** from background subagents are now shown instead of being silently rejected.


# 1.0.35 — 2026-09-16

## Bug Fixes

- **Headless MCP status reporting** and connecting reminders now match actual server handshake state.
- **MCP tool searches and calls** now render with query, results, arguments and output on the daemon path.
- **Swift string interpolation** with nested parentheses now highlights correctly in the TUI.
- **Copy-paste in --minimal mode** no longer inserts extra blank lines or breaks long paths at wrap points.
- **`/memory` modal** now shows copy confirmations, searches note contents, and works on narrow terminals.
- **Memory delete** is now much faster and toggling memory on/off from a config-disabled session now correctly wires the agent.

## Performance

- **Syntax highlighting** now uses far less memory and runs faster on large TypeScript and other files.

# 1.0.34 — 2026-09-16

## Bug Fixes

- **Markdown headings** now receive theme colors correctly.


# 1.0.33 — 2026-09-15

## Features

- **MCP tool results** now include structured JSON data when the server provides it.
- **Composer footer** now shows the Alt+Enter chord for inserting a newline when Cmd+Enter is unavailable over SSH.

## Bug Fixes

- **Fixed a bug** where pasting an image on macOS could attach the wrong image.
- **MCP tool calls** listed directly now show their name, arguments, and any error message.
- **Background subagent tasks** now correctly show cancelled status when the parent session closes.
- The Subagents dock no longer shows an unclickable "Done N completed" row after all subagents finish.
- Cancelled or timed-out MCP tool calls now notify the server so it can stop work instead of continuing in the background.
- **/rewind** no longer fails when older compaction checkpoints are missing after a session sweep; only the base checkpoint for the target matters.
- Large skill files are now capped at the same 25k-token limit as other reads; oversized skill bodies show a truncation note instead of flooding context.
- Long-running sessions no longer lose old compaction checkpoints or prompt offloads during the 30-day cleanup sweep.
- Auto "where was I" recaps no longer appear while a scheduled task, monitor, or workflow is still running.
- In minimal mode, confirming /delete or /exit now immediately opens a new empty session instead of appearing stuck.
- In minimal mode, resuming a session after /new now correctly reloads its history instead of showing the wrong session.
- MCP tools that take no arguments no longer show a stray "{}" line in the permission sheet.
- **`grok clone` on Windows** no longer fails when the repository's git config contains backslashes or quotes.


# 1.0.32 — 2026-09-14

## Bug Fixes

- **Plugin and skill listings** from x.ai/plugins/list and x.ai/skills/* now reflect the latest config.toml even before the first session.
- **Fixed crashes** on the first TLS handshake for Windows on ARM64 builds.
- **Slash commands** like `/feedback` typed during plan approval now execute correctly.


# 1.0.31 — 2026-09-13

## Bug Fixes

- **Folded subagent groups** in scrollback now correctly label how many are still running versus completed.
- **Dashboard search** and the resume picker now clearly indicate the active text field and restore list selection after search.
- **Dock sections** no longer keep a selection highlight after you collapse them with a click.
- **Worktree paths** no longer show an extra suffix in the header or dashboard.
- **MCP tools** with long server prefixes are now kept instead of being silently dropped.
- **Turn cancelled** banners now correctly name the cause instead of always saying "by user".
- **Long paths** in headers are now always shortened to the last two components.


# 1.0.30 — 2026-09-11

## Features

- **Session header** redesigned as a single row that carries the overlay context and a Dashboard button.

## Bug Fixes

- **Elapsed-time display** now rolls over into hours (for example `3h14m`) instead of stopping at 60 minutes.
- **Dock Watchers and Loops rows** now show when the next run is due.
- **Workflow status** now shows a clickable row in the dock.

## Performance

- **Fixed lag in tmux** panes by omitting per-frame synchronized-update wrappers.


# 1.0.29 — 2026-09-11

## Bug Fixes

- **Tab navigation** in the pager now cycles cleanly among prompt, scrollback, and the entire dock instead of stopping at every dock section.
- **Ctrl+G** now hides or shows the dock in the pager; focus returns to scrollback when hiding.
- **Multiple concurrent `grok agent stdio`** processes against the same GROK_HOME no longer crash at startup.
- **MCP server connections** no longer fail against modern 2026-07-28 servers.
- **Fixed startup crash** on Windows when the Client-ProjFS optional feature is disabled.

## Performance

- **Agent dashboard** no longer wastes CPU repainting identical frames when only idle spinners or hidden rows are present.


# 1.0.28 — 2026-09-10

## Features

- **`/btw` now works mid-message**: the rest of the line becomes the side question and does not appear in the main transcript.
- **Per-model `reasoning_summary`** setting is now available in config.toml for Responses API endpoints.

## Bug Fixes

- **Auto-update** no longer removes a grok binary that is still in use by another process.
- **Prompt editor** no longer gets the cursor stuck or skips a row when pressing Up/Down across soft-wrapped lines.


# 1.0.27 — 2026-09-10

## Bug Fixes

- **Queued messages** no longer appear twice when sent immediately.
- **Dashboard spinners** no longer freeze when background tasks are running.


# 1.0.26 — 2026-09-09

## Features

- **Interject messages** to subagents now interrupt long-running background waits so the message arrives immediately.
- **Subagent message rows** in scrollback now indicate whether a message was sent as interject, queued, or steer.
- **Feedback form** is now available directly in minimal mode without switching to fullscreen.

## Bug Fixes

- **`/btw` side questions** now include attached images from the composer.
- **Background task reminders** no longer echo the original shell command.
- **Failed background tasks** no longer produce duplicate reminders after the model reads their output.
- **TPM rate-limit errors** during compaction are now retried instead of treated as context overflow.
- **Ctrl+C** early in a turn now correctly rewinds the prompt without races.
- **Dashboard arrow keys** now navigate the actions row from an empty input box.
- **MCP tool calls** on legacy servers now correctly advertise elicitation support.
- **Memory v2** now recovers from init failures and avoids stalled capture after repeated errors.


# 1.0.25 — 2026-09-09

## Features

- **Background task list** is now sent as a durable snapshot so clients can restore state after reconnect.
- **Hook runs** that succeed are now silent; only blocking or failing hooks show status or a single line.
- **Vim-mode Shift+J/K** now jump to the next or previous turn at the top of the viewport, matching the timeline arrows.
- **Double-click** now selects a radio button in settings choosers.
- **Dashboard actions row** (+ New Agent, Open Previous, Worktree) can now be navigated with arrow keys and activated with Enter.
- **Workflow tool** now supports pause and stop sources so agents can control background runs they launched.
- **/theme** picker now matches aliases such as transparent, dark, or system and still inserts the canonical name.

## Bug Fixes

- **Bash command output** shown in the pager is now the complete result instead of a truncated tail.
- **Voice dictation** now inserts text at the cursor instead of always appending to the end.
- **Headless prompts** now time out cleanly instead of hanging forever when the agent does not respond.
- **Scheduled tasks** now guide the agent to check results and clean up or update recurring schedules.
- **Fixed duplicate user prompt rendering** in some ACP clients by making live echo opt-in.
- **Fixed grok -c attaching the wrong empty session** after starting without --continue then resuming.
- **Dock and queue hover colors** now match the rest of the terminal instead of using the stronger dropdown highlight.
- **Fixed image upload errors** when reading Adobe SVGs that contain a PNG thumbnail prefix.
- **Dock subagent rows** now show the same [stop] label as other killable rows.
- **Fixed startup settings and model catalog** being shared or clobbered across concurrent boots or multiple accounts on the same machine.
- **Dock Tasks panel** now shows the spinner aligned with the header instead of nested under a rule.
- **/feedback <text>** now sends immediately again instead of starting a model turn; failures save to Drafts with a notice.

## Performance

- **Embedded launches** no longer time out while scanning the skills directory on slow filesystems.


# 1.0.24 — 2026-09-07

## Bug Fixes

- **Esc** no longer cancels a running turn and instead reminds you to use Ctrl+C.


# 1.0.23 — 2026-09-07

## Bug Fixes

- **Fixed bare URLs and emails** inside table cells so every wrapped line remains a working hyperlink.


# 1.0.22 — 2026-09-07

## Features

- **Built-in agent tools** now take precedence over user MCP servers when tool names collide.
- **Desktop app tools** are now available through a dedicated first-party MCP server.
- **Remote control pickers** can now group servers by device using announced device identity.
- **edit_file diffs** now show real file line numbers instead of starting at line 1.
- **Permission prompts for file edits** now auto-expand the diff row so you can see the change before approving.
- **Sending a message to a finished subagent** now continues that same subagent instead of failing.
- **Background subagent completion messages** now show the actual output instead of just a pointer to run the tool.
- **Dashboard** now shows a unified header with location picker and an actions row for creating agents or resuming previous sessions.
- **New `grok-workspaced daemon`** mode lets a long-lived process expose folders to the Computer Hub (separate from the Desktop-supervised sidecar).

## Bug Fixes

- **MCP connectors** now correctly show when they need re-authentication even if no tools are listed.
- **Extensions modal** shortcuts now work when the tab bar is focused, and invalid actions on headers show helpful messages.
- **Send now** button and shortcuts now work correctly when an automatic background turn is running.
- **Resumed sessions** now show the exact text you typed for mid-turn follow-ups instead of the wrapped system message.
- **`/skills`** command now updates the model with newly installed skills even when written from another NFS client.
- **Auto mode** no longer instantly runs destructive `git checkout --` commands; they now go through the model for safety.
- **Fixed a crash** that could occur when restoring pasted content after certain skill injections or rewinds.
- **Subagents and workflows** now respect the same bash timeout and auto-background settings configured for the parent session.
- **Fixed model backend selection** so an explicit chat_completions setting in config.toml is no longer overwritten by same-slug siblings.
- **Slash command advertisements** no longer spam repeatedly when your current directory is your home folder.
- **Foreground shell commands** no longer produce spurious background-task reminders or incorrect kill results.
- **Queued follow-up buttons** now appear in the order [Send now][edit][cancel].
- **`/compact instructions`** in the pager are now passed to the compaction backend (bare `/compact` is unchanged).
- **`/goal`** (and resume) after pressing Esc now correctly starts the goal planner instead of failing with "Planning failed".
- **run_terminal_cmd** now correctly tells the model that foreground commands are backgrounded after ~15s instead of the 120s timeout value.
- **Skill announcements** no longer repeat the same list of available skills multiple times in a session.
- **Monitor events** in multi-session processes now correctly wake idle sessions instead of being buffered until the next prompt.

## Performance

- **Opening or resuming sessions** no longer pauses the interface while reading MCP config files.


# 1.0.21 — 2026-09-04

## Bug Fixes

- **Permission mode** (auto / always-approve) now stays visible when the agent enters plan mode and is restored on exit.
- Typing, pasting, or pressing Shift+Tab on the welcome screen now immediately opens the session instead of creating a duplicate one.
- Dock sections, queue body, and reveal rows now stay within their documented height rules even on crowded terminals.


# 1.0.20 — 2026-09-04

## Bug Fixes

- **Scroll history** after sending a prompt no longer loses the reserved bottom padding or jumps the viewport unexpectedly.


# 1.0.19 — 2026-09-04

## Breaking Changes

- **Scheduled /loop tasks always run in the background**; they no longer inject turns into your conversation.

## Features

- **New terminal theme** option makes backgrounds transparent so the terminal's own colors show through.
- **MCP servers blocked by organization policy** now show clear messages and are refused before any config change.
- **Session resume now tells the model** what loops, subagents, and workflows were still running.
- **Dashboard** now shows newly dispatched sessions immediately instead of waiting for the store.
- **Headless sessions** (`grok -p`) now support the `--worktree` flag to run in a separate git worktree.
- **Dashboard** now says 'Open session' instead of 'Add session' for the session picker button.
- **Multi-line bash commands** now render with proper wrapping and highlighting when opening the block viewer.
- **/usage** now works from the dashboard and shows account allowance when no session is active.

## Bug Fixes

- **Fixed mid-turn UI freezes** when the terminal stops reading output.
- **Fixed crashes** that occurred when the terminal pane was closed while grok was exiting.
- **MCP server list** in minimal mode now correctly shows policy-blocked servers.
- URLs that wrap across multiple lines inside quotes or lists are now fully clickable.
- **The welcome screen** composer now grows taller when you paste multi-line text.
- **Enterprise policy files** are no longer deleted on startup when your team login is stored at a custom `GROK_AUTH_PATH`.
- **/feedback** now drops unsupported images with a notice (matching the modal) and never loses your report text on save errors.
- **Feedback drafts** keep their paragraph breaks when updated, and the modal no longer switches tabs unexpectedly.
- **Turn summary lines** no longer lose their spacing after background tasks finish.
- **Thinking blocks** no longer stay expanded after switching between fullscreen and minimal modes.

## Performance

- **First prompt after login is faster** when MCP servers are configured; they connect in the background.


# 1.0.18 — 2026-09-02

## Features

- **Managed policy** now blocks disallowed MCP servers and marketplace installs before any change is written.
- **Quoting** a previous message from the block viewer now works directly from the viewer.
- **Welcome screen** now stays until you actually send a prompt; a session is prepared in the background.
- **Feedback** is now collected in a centered modal instead of a bottom card.
- **Feedback submissions** now include structured type, task, and failure information.
- **Feedback modal** now has a Drafts tab to manage and send saved drafts.
- **Models** can now render interactive charts using Chart.js.
- **grok inspect** and **grok mcp doctor** now show managed policy details.
- **Image generation** now supports richer parameters via composer state.
- **Retry behavior** for rate-limited requests is now configurable per model.
- Models can now be configured with per-model mTLS client certificates for secure upstream connections.
- **`/feedback <text>`** now saves a local draft before invoking the feedback skill.

## Bug Fixes

- **Authentication recovery** during turns no longer fails immediately on transient errors.
- The session dock now aligns consistently, shows more rows inline when expanded, and reveals stop controls on hover.
- **Pastes** from rich-text sources no longer stay inline or corrupt the composer when they contain special Unicode line separators.
- **Mouse wheel** over a one-line prompt now scrolls the conversation history instead of being ignored.
- **Slash dropdowns** no longer truncate long command names when a short sibling is also visible.
- **Subagent sessions** now respect the retry and rate-limit settings of their selected model.
- **Session picker** now opens with Ctrl+R everywhere; redo uses Ctrl+Shift+Z or Alt+Z.
- **Subagent views** no longer duplicate the task prompt on first open after a live echo.
- **Strikethrough text** now renders correctly inside markdown tables in the pager.
- **Resume picker** now labels local sessions with a single disk walk instead of per-row lookups.

## Performance

- **Startup** no longer hangs on slow networks while fetching remote settings.
- **Responses** now complete faster; uploads and session bookkeeping happen in the background.
- **Startup** now makes fewer network requests by sharing settings fetches.
- **New sessions** start responding faster by running start hooks in the background.


# 1.0.17 — 2026-09-01

## Features

- **MCP tools** now support the 2026-07-28 multi-round-trip elicitation flow where a tool call can return an input_required state that is resumed on retry.
- **Next-prompt ghost suggestions** now appear less often and are more accurate by preferring silence when the next line is not obvious.

## Bug Fixes

- **Table cell selection** inside /btw panels now correctly highlights cells and copies as TSV instead of drawing a linear rectangle.


# 1.0.16 — 2026-09-01

## Breaking Changes

- **Enterprise policies** can now restrict which models users may select via signed requirements.toml.

## Features

- **MCP servers** can now be supplied at session bind time for workspace integrations.

## Bug Fixes

- **Long-running sessions** interrupted by expired tokens during network issues no longer lose work.
- **Slash command suggestions** with very long names no longer show blank labels in the dropdown.
- **Sending a message immediately after spawning a subagent** no longer fails while the child is still starting.
- **Loading spinners** inside the extensions modal (/mcps and other tabs) now animate correctly.
- **MCP server OAuth authentication** triggered from /mcps no longer deadlocks the session.

## Performance

- **Long-running subagent and task output waits** now default to a one-hour ceiling instead of ten minutes.


# 1.0.15 — 2026-08-31

## Features

- **Tip appears** after repeated scrollback drag-copies suggesting /copy and /export commands.

## Bug Fixes

- **Session close** is now faster because memory consolidation runs at the next launch instead of blocking exit.
- **Typed input including Enter** during pager startup is now preserved and correctly interpreted as submit or newline.
- **Dock panel** input and rollout now respect remote settings and correctly handle keyboard focus when hidden or empty.

## Performance

- **First reply latency** is reduced by opening the model connection in the background when a session starts.
- **Signed-in startup** is faster because an expired token refresh now begins in the background at agent spawn.
- **Creating a new session** returns faster; MCP tools and other startup work happen in the background.


# 1.0.14 — 2026-08-31

## Features

- **OIDC token refresh** is now proactive by default for better reliability.
- **PostToolUse hooks** can now provide feedback and context to the model after tool execution.
- **SDK-registered PostToolUse hooks** now provide model-facing feedback.
- **grok usage <session-id>** now shows persisted per-turn token and cost data.
- **Retry status** in composer and title now shows a short reason for the retry.
- **Models can now declare** a different identifier for each reasoning-effort level instead of always sending the same id.
- **Prompt suggestions** now respect remote configuration and default to the current session model.
- **Windows CLI downloads** are now ~70% smaller using the same compressed sidecars as macOS and Linux.

## Bug Fixes

- **grok inspect** now correctly shows Claude bypass locks as advisory rather than enforced.
- **Subagent sessions** no longer leak threads or file descriptors when the parent is busy.
- **Cold startup** no longer performs duplicate remote settings fetches.
- **Compaction failures** due to context size now degrade input instead of retrying identically.
- **--sandbox strict** now restricts writes to ~/.grok/sessions only.
- **Subagent spawning** now waits longer on a busy coordinator and shows clearer retry guidance instead of "unreachable".
- **Failed task and todo tool calls** now appear in the transcript instead of disappearing without a trace.
- **Composer status row** no longer collapses or flashes when using double-Enter to send now.
- **Session close** is no longer delayed by a single slow hook; each SessionEnd hook now has its own timeout.
- **Hook removal** in the extensions modal no longer offers actions that the handler will refuse.
- **Interjections** during a turn are now delivered atomically or not at all.
- **Subagent tasks** no longer get incorrectly cancelled when the parent session is waiting for completion.
- **Workflow detail view** now closes the overlay on X or outside click instead of returning to the run list.
- **Resuming subagents** now succeeds for larger transcripts that still fit the model context with headroom.

## Performance

- **Startup** now fetches remote settings only once per boot instead of potentially twice.
- **First message** on large repositories no longer waits on repository status scan.
- **Large session memory** no longer blocks the agent during turn completion or subagent spawning.
- **Signed-in CLI starts** faster by serving remote settings from a local cache on warm boots.


# 1.0.13 — 2026-08-28

## Features

- **Length-truncated responses** now continue automatically instead of failing the turn.
- **Hooks** can now ask the user to confirm a tool call instead of always allowing or denying.
- **Hooks** can now request deferral or add context shown to the model after a tool runs.
- **Session close** now records detailed timing data for performance analysis.
- **Credit limit upsell** now offers a Try Again button to retry the last prompt.
- **Pasted images** now show a live pixel preview in the prompt box on iTerm2.

## Bug Fixes

- **Transient inference failures** (stalls, drops, 5xx) now retry automatically instead of ending the turn.
- **Windows users** can now correctly open ~/.grok and worktree sessions.
- **Session data** is now more reliably saved after prompts and on power loss.
- **Compaction failures** now show the actual error instead of a generic message.
- **Truncation error messages** now show the right guidance instead of suggesting an unhelpful retry.
- **Truncated tool calls** are now executed instead of failing the turn when arguments are complete.
- **Images larger than 2000px** no longer brick sessions on many-image requests.
- **Wrapped hyperlinks** in the pager now remain fully clickable on Windows Terminal instead of only the first line.
- **Recurring scheduled tasks** now include a reminder to stop the monitor when work finishes.
- **Scheduled task IDs** are now full UUID strings, preventing collisions when tasks are created in the same millisecond.

## Performance

- **Subagent spawning** is faster when connections drop during bursts.
- **Session startup** with MCP servers is now much faster when auth is already configured.
- **MCP server startup** no longer stalls behind a fixed batch size.
- **CLI downloads** are now compressed, making fresh installs and updates substantially faster.


# 1.0.12 — 2026-08-27

## Features

- **Credit-limit upsell** now includes a Try Again option. Max-tier users see the same question modal without Upgrade tier.

## Bug Fixes

- **Copying wrapped table cells** no longer inserts unwanted spaces at line breaks.
- **MCP server connections** that fail transiently now retry instead of staying unavailable.
- **Waiting on subagents** after an interjection no longer blocks on unrelated background work.
- **Prompt blocks** now show friendly hook descriptions instead of internal IDs.
- **Truncation error message** no longer suggests asking for a shorter answer.
- **Context estimates** for conversations with reasoning are now more accurate.
- **Token usage** after rewinding or switching modes now matches the actual server-reported count instead of overestimating.
- **Context bar** now updates immediately when auto-compaction begins instead of waiting until it finishes.
- **File system monitoring** for .NET projects no longer creates excessive watchers on Linux.
- **Auto recap** no longer appears while a background task or monitor wake turn is streaming.

## Performance

- **Worktree creation** is faster by skipping stale reflog copies.


# 1.0.11 — 2026-08-26

## Features

- **Headless sessions** are now browsable in the resume picker without mixing into default history.
- **Default permission mode** for new interactive sessions is now configurable.
- **Turn duration** now appears in session history after resume.
- **Turn footers** (Worked for, cancelled, failed) now appear after /resume.
- **mkdir and touch** no longer prompt in auto mode or safe-command lists.
- **Subagent messages** are now allowed automatically in permission Auto mode.
- **Headless sessions** can now auto-allow permission prompts via a startup hint.

## Bug Fixes

- **Permission prompts** for common command chains and subcommands are now more reliable.
- **Blocked prompts** no longer appear in conversation history or scrollback after restart.
- **Background monitors** no longer have a 10-hour default timeout.
- **Mouse input** at the right margin no longer types characters into the prompt on certain terminals.
- **Pasting text ending in a newline** no longer accidentally submits the prompt on some terminals.
- **Auto mode** now shows a permission card when the classifier blocks an action on interactive sessions.
- **Image previews** no longer leave ghost artifacts when using the Kitty protocol on Warp.
- **Expanded Execute tool output** no longer snaps closed during live progress.
- **/voice** now falls back to parec/arecord on older PipeWire installs.

## Performance

- **Background command waits** now finish as soon as the process exits.


# 1.0.10 — 2026-08-24

## Performance

- **grok clone** now reuses matching local checkouts as linked worktrees for faster session creation.


# 1.0.9 — 2026-08-24

## Features

- **New sessions in the interactive TUI** now start in auto mode by default for fewer approval prompts.
- **/feedback now supports attaching images** (screenshots etc.).
- **The / slash menu** is now ordered by recency and groups skills below commands for easier navigation.
- **/workflow** now accepts --agent-budget N or an agent_budget JSON field to set child-agent limits.
- **/workflow** now accepts --effort LEVEL or an effort JSON field to set child reasoning effort.
- **MCP servers** now receive a grok-cli User-Agent header so providers can attribute traffic.
- **Dashboard** now lets you start new agents in Auto mode via Shift+Tab.
- **Workflows** now appear alongside skills in the model prompt so the agent can launch them by name.
- **`/edit-prompt`** now opens an external editor from the full TUI, not only minimal mode.
- **Up arrow** on an empty prompt now selects queued follow-ups first instead of opening history.
- **/plugin** now works as an alias for **/plugins** in the TUI.
- **Plugin-provided agents** now appear in the **/agents** modal and can be toggled.
- **/minimal** and **/fullscreen** now switch instantly inside the running session without restarting or losing the current turn.
- The configurable status line row now appears at the bottom of minimal mode as well as fullscreen.
- **grok clone** now fetches only the selected branch tip by default, with --full-history for legacy full clones.
- **grok clone** can now reuse a matching local checkout as a linked worktree instead of always fetching from the network.

## Bug Fixes

- **Subagent tasks** are more resilient to temporary rate limits and will retry instead of failing immediately.
- **Narrow allow rules for Bash commands** now correctly permit file writes without extra prompts.
- **Rules** containing markdown headings now render correctly in the prompt.
- **/new** and **/clear** now preserve the last-chosen reasoning effort instead of resetting to the model default.
- **Prompt dropdowns** (slash, history, file search) now have consistent left alignment with the composer.
- **History** now shows every command, slash command, and memory note in the order you typed them.
- **Collapsed tool-call rows** no longer show a vertical accent line beside the status diamond.
- **Hint text** above the prompt now lines up with the text you type.
- **Pressing Esc** while peeking at a conversation from the dashboard now closes open modals instead of leaving the view.
- **`/copy`** now preserves markdown formatting such as headings and code fences.
- **Sending a new message** while a shell command is running now moves the command to the background instead of cancelling it.
- **grok mcp add** now correctly treats bare http(s):// URLs as HTTP servers instead of stdio commands.
- Plan mode no longer incorrectly blocks subagent spawns as file edits outside plan.md.
- MCP servers that share a URL but have different names are now kept as separate entries instead of being collapsed.
- Subagents and workflow-spawned agents no longer receive the workflow tool that only top-level sessions can use.
- Memory results are now presented as historical context that should be verified against live sources before use.
- **Interactive grok sessions** now start in ask mode by default again instead of auto.
- **Minimal mode** no longer truncates subagent wake-turn replies after the first token.

## Performance

- **Concurrent subagents** no longer trigger rate-limit errors during bursts.
- **MCP server connections** no longer delay session startup when many servers are configured.


# 1.0.8 — 2026-08-20

## Features

- MCP servers can now ask for form input or URL consent through the same popup used for questions.
- Ctrl+S now stashes the current prompt draft so you can send something else and restore it later.
- ** /workflow** now autocompletes saved workflow names and shows only valid runs for pause/resume/stop/save.

## Bug Fixes

- Downloading a folder that contains only one file now produces a zip that still extracts as a folder.
- Failed tool calls for tools the model invented now clearly state the tool does not exist.
- **Status line** refresh timer now uses consistent naming and no longer shows errors on deliberately hidden rows.
- **Workflow agent rows** now display current context usage instead of cumulative token counts.
- **Follow-up messages** now send immediately while waiting on a subagent or task, including after using /btw.
- Subagents and workflow-spawned agents no longer see the workflow tool, which can only be launched from a top-level session.

## Performance

- Opening many subagents at once no longer freezes the interface while loading their history.
- **Concurrent subagents** now start much faster and no longer freeze the parent session.


# 1.0.7 — 2026-08-19

## Features

- Users hitting startup timeouts can now raise the connect budget with the `GROK_CONNECT_UI_TIMEOUT_SECS` environment variable.
- Permission prompts now show "Always allow" and "Never allow" options by default.
- Users can now delete scheduled background loops directly from the tray.
- **Status line command** scripts can now run on a timer via refresh_interval in config.toml.
- **Permission prompts** now offer a 'Never allow' choice for MCP tools and web-fetch domains that persists per project.
- **Workflows tab** added to the extensions modal (Ctrl+L or /plugins) listing installed workflows with name, source, and description.
- **New /workflows** command opens the Workflows catalog tab; use **/workflow runs** to view live workflow runs.
- **Bare /workflow** (or /workflow runs) now lists active and recent workflow runs with status and progress instead of usage help.
- **Workflows** row added to the Ctrl+P command palette, opening the Workflows catalog tab.

## Bug Fixes

- **MCP server connections** in non-interactive sessions no longer incorrectly require authentication for tokenless servers.
- Fixed startup timeouts caused by concurrent auth refreshes across multiple sessions.
- **Tool call loops** are interrupted earlier to avoid wasting time on repeated identical actions.
- **Subagents** no longer receive the ask-user-question tool.
- Bare email addresses are now turned into clickable mailto links in the pager.


# 1.0.6 — 2026-08-18

## Breaking Changes

- **Subagent spawning** no longer accepts capability_mode; tool access is now controlled only by agent type.

## Features

- **Shift+arrow keys** now extend text selections in the prompt like a standard text field.
- **Optional status line** can now display live session info or script output at the bottom of the pager.
- **grok clone** can now fetch a repo into a content store and mount a projected working tree.

## Bug Fixes

- **Fixed session startup hangs** on large or unhealthy git repositories.
- **Queued messages** during goals no longer starve, and editing queued prompts works reliably.
- **Consent notice** on first launch now shows clickable links and handles keyboard/mouse correctly.
- **Ctrl+C then edit** a prompt now correctly removes the original text from the conversation.
- **Double-clicking** a terminal command result now shows the complete output instead of a preview.
- **Consent notice links** are now stricter and more reliable on all terminals.
- **Video generation** now surfaces a clear ZDR error instead of raw API responses when output storage is required.
- **Project hooks** on Windows now correctly expand $CLAUDE_PROJECT_DIR when invoking PowerShell scripts.


# 1.0.5 — 2026-08-15

## Features

- **GROK_CONFIG** and **GROK_CONFIG_PATH** environment variables now let launchers override selected config settings without editing config.toml.
- **Worktrees** under ~/.grok/worktrees are now automatically reclaimed when safe, with strong safeguards that never delete a user's last copy.
- **Hook policy blocks** now correctly report "Turn blocked by a hook" instead of "Turn cancelled by user."
- **Image and video generation** now limits how many calls the model can request in one step to avoid overload.
- **Arabic and Persian text** can now be reordered correctly in the terminal UI. Turn on in /settings.
- **Reasoning effort** can now be supplied when an ACP client opens or resumes a session.
- **Session titles** now refresh early in the conversation and stay stable; /resume shows a recap and last-turn summary when available.
- **GROK_FORCE_LOGIN_TEAM_ID** environment variable now lets launchers restrict interactive login to one or more teams.
- **Preparing spinner** now shows readable labels such as "Writing file…" and "Writing edit…" for common tools.

## Bug Fixes

- **Tool calls** (shell, grep, list_dir) no longer fail for the rest of a session if /dev/null is removed.
- **Agent skill discovery** now resolves the user's home directory correctly on Windows.
- **MCP tool calls** now show clearer spinner text instead of the raw wire name while arguments are still arriving.
- **grok inspect** no longer crashes when its output is piped into a command that closes the pipe early.
- **Minimal mode** no longer truncates a still-streaming assistant reply when thinking blocks are interleaved.


# 1.0.4 — 2026-08-13

## Features

- **New StopCancelled hook event** now reports when a turn ends without completing (interrupt, permission reject, max turns, etc.).
- **Recurring /loop tasks** now show a one-line expiry notice in the transcript when they auto-expire after 7 days.
- **Web search** can now be restricted to allowed or excluded domains via [toolset.web_search] in config.toml.
- **Session search index** can now be disabled via GROK_SESSION_SEARCH or [features] session_search for hosts sharing $GROK_HOME.
- **Drag to select and copy** values on the /session-info tab; c and y shortcuts also work.
- **Double-click now selects a word** by default and triple-click selects the whole paragraph.
- **New follow-up behavior setting** lets queued messages send immediately as interjections instead of waiting for the turn to finish.
- Tool commands and MCP servers now receive a GROK_SESSION_ID environment variable matching the current session.
- Relative markdown links can now open existing files in your current working directory when no matching generated media is found.
- PreToolUse hooks can now rewrite a tool's input before it runs instead of only allowing or denying the call.

## Bug Fixes

- **Queued messages** no longer auto-submit while you are still editing them in the composer.
- **Sessions poisoned by rejected images** are now healed permanently so future turns succeed without retrying the bad image.
- **Auto permission mode** now correctly honors your explicit "always allow" grants and narrow allow rules from settings.
- **Subagent lifecycle events** are now preserved even when delivered out of order, ensuring all subagents appear correctly in the UI.
- **Keystrokes typed while Grok is starting** are now preserved in the composer instead of being lost.
- **Background tasks killed from the UI** now correctly wake the model when needed instead of staying parked after a single-task stop.
- **[stop]** / Ctrl+C inside a fullscreen subagent overlay now cancels the visible child session.
- **Hook failures** now show the first line of stderr output instead of only the exit code.
- **Pasting text or dragging images** while the scrollback pane is focused now focuses the composer and pastes there.
- **[stop]** / Ctrl+C inside a subagent drill-in view now stops the focused subagent instead of the root session.
- The dashboard shortcut now works with Ctrl+4 in terminals that do not support the Kitty keyboard protocol.
- Pasting image-only screenshots from tools like Flameshot now works on Linux without a clipboard error.
- Editing a queued prompt to a slash command like /btw now runs the command instead of sending the text to the model.
- Text typed while a plan is being generated is now preserved when the approval screen appears.
- **`grok du`** and worktree commands now work on Windows when only USERPROFILE is set.
- Permission-mode changes made on the welcome screen now correctly apply to the newly created session.

## Performance

- **Finished subagent transcripts** are now evicted from memory to reduce RAM usage and rebuilt from disk when reopened.


# 1.0.3 — 2026-08-12

## Features

- **/session-info** now lets you click any row to copy its value, with hover highlights and a copy-all shortcut.

## Performance

- **Subagent spawning** is dramatically faster when you have many sessions in ~/.grok.
- **TUI rendering** now automatically matches high-refresh displays (120 Hz+) for smoother scrolling and painting.


# 1.0.2 — 2026-08-11

## Features

- **Tool-call argument streaming** now shows a distinct spinner label instead of a generic "Waiting for response…" message.
- **Harness** now includes UI-verification instructions and project/user rules higher in prefix.
- **Large sessions** with images no longer exceed limits during compaction

## Bug Fixes

- **Fixed recovery** from server-rejected images so poisoned sessions no longer become permanently unusable.
- **Improved startup timeout messages** to show which step took longest, elapsed times, and actionable advice instead of a generic error.
- **Worktree copies** of large repos no longer inherit dangerous fetch specs or stale shallow grafts.
- **Privacy banner** can now be dismissed from Settings even when you are already opted out.
- **Status bar** now keeps showing your current model after a catalog refresh even if that model is no longer listed.
- **Grouped tool calls** now stay grouped even when hooks attach metadata, and show hook results in the header.
- **Cmd+click** on autolinks in Apple Terminal now opens the correct URL when multiple messages are visible.

# 1.0.1 — 2026-08-10

## Breaking Changes

- /rewind now only truncates conversation history instead of files as well and asks for confirmation by default.
- **Managed MCP servers** are now only available through the gateway catalog.

## Features

- **Subagent spawning** is now bounded; wide fan-outs queue instead of exhausting file descriptors.
- New `grok du` command shows disk usage of ~/.grok including worktrees and sessions.
- **Tools** now report whether they only read data, enabling safer restricted agents and subagents.
- **Sandbox workspace** sessions can now limit which bundled skills are advertised via caller config.
- **Renaming a session** from the dashboard now starts with the current title prefilled for easy editing.
- **/usage**, **/session-info**, and **/context** now open in a tabbed modal instead of adding text to the conversation.
- **grok trace** exports now bundle memory trace files for easier debugging.
- Session rename now enforces a 100-character limit, ghost-prefills the current title, and preserves manual titles across machines.
- New `/rename --auto` command unpins a manual session title so automatic titling resumes.
- Video generation from references now supports preset voices, single-image input, 1–15 s durations, and 4:3 / 3:4 aspect ratios.

## Bug Fixes

- **Sandbox config** entries ending in /** now correctly grant the parent directory instead of creating a literal ** subdirectory.
- **Failed alpha/enterprise updates** now suggest the matching GROK_CHANNEL reinstall command.
- **On Apple Silicon**, grok now installs the native arm64 build even from a Rosetta shell or x86_64 updater.
- **Skills** that share names with built-in commands now appear alongside them in the slash menu with qualified names.
- **Notebook** permission rules imported from Claude configs are now ignored with a warning instead of applying broadly.
- **Goal evaluation** at round end no longer fails due to timeouts.
- **Tool timeouts** no longer cause the agent to hang when child processes are stuck in D-state or hold pipes open.
- **Home** and **End** keys now move to the start or end of the current logical line even when the prompt is wrapped.
- **Worktree** sessions now correctly show their branch in the status bar.
- **Worktree** sessions now keep their status correctly when switching directories or resuming.
- **Worktree** status is no longer lost when opening the dashboard.
- **Recaps** are now written in the same language as your conversation.
- **Plugin suggestions** no longer flash incorrectly while typing.
- **Send Now** now works during active goals without cancelling the goal.
- **Headless sessions** now correctly wait for MCP servers when using delivery tools.
- **Non-interactive sessions** (`grok -p`) no longer fail when the agent asks for user input or plan approval.
- **read_file errors** for missing skills now suggest the correct registered path instead of a generic hint.
- **Session load and creation** can no longer freeze forever when `.envrc` evaluation blocks.
- **Upgraded installs** no longer silently run outdated platform skill instructions.
- **Deleting a session** now properly stops and waits for any running subagents before wiping history.
- **Permission and plan-approval notification hooks** no longer fire on auto-allowed tools.
- **Mid-turn steering** sent with double-Enter or Ctrl+Enter now correctly tells the model it arrived while work was in progress.
- **Scrollback drag selection** no longer gets stuck after the mouse button is released outside VS Code or Cursor terminals.
- **Video generation tools** now show a clear error explaining ZDR storage requirements instead of silently disappearing.
- **Esc on the cancel-turn panel** now closes the panel and keeps the current turn running as the shortcuts bar indicates.

## Performance

- **Git status** and diff operations no longer cause high CPU or memory use on large repositories.
- **Large git histories** no longer cause excessive memory use or unresponsiveness.
- **History search** no longer leaks background threads in long sessions with many subagents.
- **Resuming large sessions** is now significantly faster and the UI no longer shows an incomplete transcript while replay is still applying.


# 1.0.0 — 2026-08-07

## Features

- Dashboard rows show a short summary of what the agent did in the previous turn
- Extensions modal groups items alphabetically with collapsible Skills sections
- Grok skips the project-directory prompt when launched from home or other non-project directories
- `/feedback` opens a dedicated report box instead of prompt mode
- Auto theme detection works over SSH and inside tmux
- Markdown tables reflow inside cells on narrow panes instead of clipping
- Permission prompts show the complete script; long bash bodies expand with `Ctrl-F`

## Bug Fixes

- MCP tools that return images no longer drop or corrupt large screenshots
- Sandboxed Grok starts on large directories with many deny-glob matches
- Rapid send-now presses no longer lose earlier queued messages
- Esc and stop prevent background tasks from restarting the model after cancel
- Login no longer skips when an invalid API key is in the environment
- Model picker and command palette work while reviewing a plan
- Tab and Esc behave consistently on question, permission, and cancel-turn cards
- `/new` from the dashboard returns to the dashboard from an empty prompt
- Codebase restore no longer hangs on large or shallow git repositories
- Remote resume restores conversation only unless `--restore-code` is passed
- Copying CJK text with the mouse includes every character at the selection edges
- API errors appear as clean banners instead of raw JSON dumps
- Typing exit or quit in the dashboard exits the CLI
- Mode indicator (plan/agent/ask) stays in sync after resume and mode changes
- `/delete` returns to the dashboard when you delete a session opened from it
- Enter in the slash command menu runs the highlighted command
- Grok retries more server errors during outages
- Session-only slash commands show a message when used from the dashboard
- Queued prompts stay visible while waiting on subagents, and slash/image rows can be reordered
- Auto recaps no longer appear mid-turn or while busy
- `/btw` error messages wrap fully

## Performance

- Forking very large sessions no longer uses many times the session file size in memory
- Exiting an empty session is instant, even on slow networks


# 0.2.120 — 2026-08-03

## Bug Fixes

- **Model picker** now updates the status bar and /model menu immediately, even before the first prompt creates a session.
- **Changes panel** now refreshes after the agent commits on the current branch instead of showing stale unstaged files.
- **Background task** completions now report the full log size and read hint even when only a short prefix was captured.
- **GitHub export** on old hibernated sessions now shows a clear message to start a new chat instead of a generic error.


# 0.2.119 — 2026-08-02

## Features

- **Always allow** for bash commands now lets you edit a free-form glob pattern instead of only word-prefix scopes.
- **Long responses** now show a clickable arrow that jumps back to the start of the answer.
- **Auto mode** now auto-approves more common read-only git commands and harmless file appends.
- **Plan previews** now show Mermaid diagram buttons (Open Image, Copy Image Path, Copy Source).

## Bug Fixes

- **Gateway connections** now detect and recover from dead sockets more reliably.
- **Question cards** now let you Tab through answers instead of losing focus to the scrollback.
- **Resume picker** no longer tries to load a session from pasted garbage when you press Enter.
- **Background task** completion messages no longer grow unbounded when the task produced a huge log.
- **Plan viewer scrollbar** now responds to clicks on the border column and renders without dark stripes in Terminal.app.
- **Expired external auth provider** credentials now correctly trigger the interactive sign-in flow instead of a silent 401 loop.

## Performance

- **/btw** side questions now reuse the parent session’s cached prefix for faster responses.
- **Doctor** and tmux-backed startup are now faster when no live tmux processes remain.


# 0.2.118 — 2026-07-31

## Features

- **Sessions** can now be permanently deleted from the dashboard by pressing Ctrl+X twice on an idle row, or from the welcome list with d then y.
- **Keyboard shortcuts help** (Ctrl+.) now shows how to browse prompt history and search the conversation.
- **grok doctor** now warns when tmux is reducing colors and can fix the config.

## Bug Fixes

- **`/btw`** now retries on temporary model overload instead of failing immediately.
- **Session sharing** is temporarily disabled.
- **`[stop]`** / Ctrl+C during `/compact` now cancels instead of no-opping.
- **Automatic recaps** no longer appear twice after the same turn.
- **Background task wait timeout** descriptions and limits now match the client's actual configured ceiling.
- **Background tasks** no longer stay stuck as 'Running' in the tasks pane when they finish quickly.
- **Plan mode indicator** now disappears right after approving a plan instead of lingering.
- **Dragging the scrollbar** in the plan preview now works as expected.
- **Compaction** now correctly handles certain context-length errors from the inference API.


# 0.2.117 — 2026-07-30

## Features

- **GROK_EXTRA_CA_BUNDLE** env var allows adding custom TLS root certificates.

## Bug Fixes

- **Stop command** now terminates all background subagents from prior turns.
- **kill_task** tool now correctly reports when a task does not exist over ACP connections.
- **get_task_output** no longer waits the full timeout for already-finished tasks over ACP.
- **/usage** command and billing UI are hidden for enterprise auth setups.
- **Plan approval** no longer starts Build when pressing Enter without notes in revise mode.

## Performance

- **Terminal resize** is much faster on long conversations in fullscreen mode.


# 0.2.116 — 2026-07-30

## Features

- **Headless streaming output** now includes tool calls, results, and usage when using `--output-format streaming-json`.
- **New `/undo` slash command** restores files and chat to an earlier turn, same as `/rewind`.
- **Slash commands** are now correctly hidden or refused in minimal or fullscreen mode based on their declared support.

## Bug Fixes

- **Fixed repeated forced re-logins** after laptop sleep or network hiccups during token refresh.
- **Suppressed spurious history load warnings** on draft conversations that have no server history yet.
- **Settings enum pickers** now keep the selected radio button on the current value until you press Enter.
- **Deep-linked settings** such as `/privacy` now close the settings modal on Esc or Enter instead of returning to the list.


# 0.2.115 — 2026-07-29

## Features

- **Delete sessions from the dashboard and welcome list.** On the dashboard, press `Ctrl+X` twice (or hover a settled row and click `[✗]` twice); in the welcome and `/resume` lists, press `d` then `y`.

## Bug Fixes

- **Fixed chat history corruption** that could duplicate tool results or cause later 400 errors after repeated identical tool calls.
- **Fixed infinite redirect loops** in embedded previews when the browser blocks the required cookie.
- **Improved the action-stationarity nudge message** to avoid incorrectly claiming tool results were identical.
- **Fixed external auth provider commands** (`auth_provider_command`) not working on Windows.
- **Fixed incorrect 'Turn cancelled by user' messages** shown on internal send-now wake turns.
- **Fixed language server crashes** (e.g. Roslyn on every edit) and missing C# diagnostics; improved diagnostics reliability for other servers.

## Performance

- **Improved prompt caching** for long conversations, reducing repeated billing on growing transcripts.

# 0.2.114 — 2026-07-29

## Features

- **New `/delete` slash command** removes the current session's history after confirmation.

## Bug Fixes

- **Grok** no longer crashes on startup when the host machine has no free threads.


# 0.2.113 — 2026-07-28

## Features

- **MCP servers** can now be enabled or disabled directly from the CLI with `grok mcp enable <name>` and `grok mcp disable <name>`.
- **Full plan markdown** can now be copied to the clipboard with `y` during plan approval or preview.
- **Added support for the new SuperGrok Plus subscription tier** in authentication and feature gating.
- **Enabled automatic recovery** from repetitive loops in model output by default.

## Bug Fixes

- **Terminal command output** is no longer lost or duplicated when the gateway is unreachable.
- **Invalid MCP server entries** in config.toml no longer prevent Grok from starting; problems are shown in `grok inspect`.
- **SessionEnd hooks** now run on exit in non-leader TUI and headless sessions.
- **Paste chips** now display with the correct background in inline prompts and question inputs.
- **Pasted content chips** now behave consistently when editing answers in the question view.
- **Background task status** now shows only elapsed duration instead of absolute timestamps.
- **Session lists** no longer drop real sessions when the remote registry reports an outdated turn count of zero.
- **/loop** now stores prompts that include stop conditions so recurring tasks can terminate themselves when done.
- **Reduced spurious warning messages** for common auth and config scenarios.
- **Fixed conda activation** (and other sourced scripts that read $@) when using persistent or login-capture shells.
- **Fixed stuck background-task tray rows** after long foreground shell commands complete.
- **Agent subprocesses and idle inhibitors** are now cleaned up when the parent CLI process dies unexpectedly.
- **Fixed truncated plans** in minimal mode and improved visual separation between reasoning and output (including NO_COLOR).
- **Fixed credential loss** across multiple grok processes sharing the same auth file.
- **Fixed doubled Enter** and other keys on older Alacritty terminals.
- **Fixed false paywall** messages for free-tier and unmatched users.

## Performance

- **Cold start** shows the UI instantly while models and settings load in the background.
- **Large session forks and resumes** now use far less memory and avoid spikes.
- **Prevented thread exhaustion** on high-core shared machines by limiting the workspace daemon's worker threads.


# 0.2.112 — 2026-07-24

## Breaking Changes

- **CLI version policy** now has separate soft update floors/ceilings and hard startup requirements.

## Features

- **New /tutorial slash command** opens an opt-in nine-topic onboarding tour of Grok.
- **New tool_overrides option** lets you set date cutoffs and domain allowlists for the agent's built-in search tools.
- **New toolOverrides option** lets you set date cutoffs and domain allowlists for the agent's built-in search tools.
- **New config options** let you add query parameters or environment-backed headers to custom model providers and control which variables reach shell tools.
- **Terminal and environment fixes** are now consolidated under the `/doctor` command with clearer guidance.
- **Marketplace add** now rejects non-git URLs at add time instead of failing later.
- **Slash commands** can now show optional bracket tags (e.g. [new]) via config or remote settings.
- **Queued prompts** now offer an [edit] mouse button alongside Send now and cancel.
- **Voice shortcut** toggle in settings can disable the Ctrl+Space/F8 keybind without disabling voice entirely.
- **Image edit** can now use a remotely configured model slug instead of the hardcoded default.
- **`grok doctor fix`** can now repair common tmux clipboard and passthrough problems.
- **Per-provider auth helpers** now work on Windows and can run from a configurable working directory.
- **/resume** now shows only native Grok sessions by default and shows a hint when external sessions are hidden.
- **`grok --resume`** can now resume a session by its title as well as by ID.
- **Workflows overlay** now shows live per-agent progress and automatically follows the active phase.
- **Workflow runs** that failed can now be resumed; scratch file limits were also increased.
- **Hooks** can now be defined in config.toml in addition to JSON files.
- **Clicking** the "still running" status now opens the tasks pane.

## Bug Fixes

- **File attachments** now appear correctly when resuming or replaying conversations.
- **Terminal output** from remote clients is now recorded so read-file hints and monitors function correctly.
- **Background shell commands** now correctly report their real exit codes instead of always showing -1.
- **Marketplace source refreshes** no longer hang the TUI or trap you in the extensions modal.
- **Background task tray** now correctly clears killed tasks and keeps task descriptions after reconnect.
- **Dashboard overlay** now correctly returns after forking a dashboard-attached session.
- **Linux voice dictation** now works on PipeWire versions before 1.6.
- **Fork** from a rewound session now copies the correct live-branch history.
- **Account pane** now shows name and email even after the access token expires.
- **Voice mode** now lets you edit already-dictated text without closing the microphone.
- **Fixed startup hangs** on Linux after concurrent launches or rapid restarts.
- **MCP tools** now appear without restart after enrolling or updating a managed service.
- **Plugin subagents** now see the same MCP tools as the parent session.
- **Copy confirmations** now show shorter messages when the clipboard succeeds.
- **Repeated identical tool calls** now end the turn silently instead of showing a stop banner.
- **Web search** now defaults to grok-4.5.
- **Voice dictation** text is no longer dropped when pressing Enter to send.
- **Bash mode** (`!`) now shows yellow prefix and action label in minimal mode.
- **Parked turns** no longer spam duplicate "Worked for" markers in the transcript.


# 0.2.111 — 2026-07-22

## Features

- Users can now disable image generation and video generation tools (and their slash commands) via config.toml or environment variables.
- `/session-info` now displays whether the session uses OAuth or an API key and where to manage the account.
- You can now run `grok doctor fix` commands directly from inside the TUI instead of only from the CLI.

## Bug Fixes

- **Plugin subagents** now inherit the parent session’s connected MCP servers (default `mcpInheritance: all`), so `search_tool` / `use_tool` work the same as for local agents. Plugin agents still cannot declare their own MCP servers, hooks, or elevated permission modes.
- **`!cmd` commands** now allow up to one hour before timing out.
- **npm package** now installs the native binary under `$GROK_HOME/bin` (honoring the same override as the Rust CLI).
- **Startup warnings** now point to `/doctor` for details and fixes.
- **Dashboard hover and clicks** no longer miss the gaps between items in wide mode.
- **Shift/Alt+Enter** now inserts a newline while editing a queued prompt.
- **Queued prompt edits** under combine mode no longer lose changes due to premature hold release.
- Forking a session that used compaction no longer causes later rewinds to fail with missing checkpoint errors.
- When a permission prompt appears while viewing scrollback, focus now correctly moves to the prompt so you can answer.
- Pressing Esc once now cancels the current agent turn (except in fullscreen vim scrollback mode).
- Grok now automatically stops a turn that keeps repeating the exact same tool call many times in a row.
- Configs using either spelling of the workspace teleport disable flag now load and save correctly.
- Background subagent completion messages no longer leak into unrelated sessions when multiple sessions are active.
- When the auto-permission classifier times out or fails, Grok now shows a normal permission prompt instead of silently denying.
- **Managed MCP tools** no longer time out prematurely on slow operations like Notion updates.

## Performance

- Voice dictation on macOS now uses less memory by running capture in a temporary helper process.


# 0.2.110 — 2026-07-21

## Features

- **Removing MCP servers, plugins, or hook sources** in the Extensions modal now asks for confirmation (press y to proceed).

## Bug Fixes

- **Session creation failures** (including disk full) now show an error message instead of hanging on "Starting session…".
- **Auto-compact** that fails due to an expired token now lets you log in and automatically retry the compact + original prompt.


# 0.2.109 — 2026-07-21

## Features

- **/usage** now shows token counts and cost for the current session.
- **grok doctor fix ssh-wrap** can set up `grok wrap ssh` automatically for Bash, zsh, and fish.
- **[model_providers.<id>]** lets operators share gateway settings across custom models.
- **Reasoning effort** now accepts `max` as its own tier (above `xhigh`) when the model advertises it.
- **Queued follow-ups** can now be batched into a single model turn with the new combine_queued_prompts setting.
- **/doctor** is now the main in-app command for checking terminal, tmux, clipboard, and keyboard setup.
- **read_file** now returns full Markdown files inside skills/ directories without truncation.

## Bug Fixes

- **Voice dictation** now explains when the microphone delivered only silence (macOS permission) versus no speech detected.
- **Duplicate 'Worked for' markers** no longer stack in the transcript when background tasks defer during a parked turn.
- The idle status row now clearly says '1 subagent still running' instead of 'watching · 1 subagent' when background work remains.
- **Background /loop** iterations no longer overlap when descendant subagents are still running.


# 0.2.108 — 2026-07-21

## Features

- **Sessions** can now be resumed after moving the working directory or switching machines.
- **Ctrl+G** in minimal mode opens the current prompt draft in an external editor without sending it; fullscreen keeps the tasks pane.
- **grok doctor** checks terminal, tmux, clipboard, and keyboard setup without opening the TUI.

## Bug Fixes

- **Image paste** over grok wrap now works on headless remotes.

# 0.2.107 — 2026-07-20

## Features

- **Stop hooks** can now keep the agent running by feeding feedback back to the model instead of ending the turn.
- **Custom models** can now authenticate using rotating tokens fetched from a command, similar to credential helpers.
- **Feedback** now includes author details when provided, helping with follow-up.
- **Sessions** can now resume across hosts by mirroring transcripts to external storage like S3.
- **Sessions** can now be imported and resumed from mirrored state across hosts.
- **Auto mode** now continues after classifier blocks by telling the agent the reason, escalating only after repeated denials.
- **Session storage** can now flush after every frame (eager mode) instead of only at turn end.

## Bug Fixes

- **Ctrl+B** now backgrounds running commands; **Ctrl+G** toggles the tasks pane.
- **OAuth popups** in live preview now redirect correctly after login.
- **Git status** shown to the model at startup now includes unstaged and untracked files.
- **Tool descriptions** now stay correct when parameter names are randomized.
- **Minimal mode** now shows full reasoning in scrollback and collapses successful lookup results to one-line headers.
- **Empty commands** like `true` or bare `echo` now remind the model to stop and wait for background work instead of spinning.

## Performance

- **Recap summaries** after idle now load much faster by reusing the previous turn's cached context.


# 0.2.106 — 2026-07-18

## Features

- **Added GROK_CLIPBOARD_NO_OSC52** env var to stop clipboard sequences from appearing as garbage in unsupported terminals.
- **Scheduled tasks** can now be updated in place; one-time tasks are retired in favor of background commands.

## Bug Fixes

- **Copies** now always write a backup file so text remains recoverable when the terminal clipboard fails.
- **Syntax highlighting** in --minimal mode is now visible on light terminals.


# 0.2.105 — 2026-07-18

## Features

- **/btw** now works inside `grok --minimal`, showing answers in the live area and committing them to scrollback on Esc.
- **New Appearance setting** "Snap prompt to top on send" lets you keep the viewport where it is instead of jumping to the new prompt.
- **Default model** is now Grok 4.5 with high/medium/low reasoning effort and improved compaction settings.
- **New `/summarize` slash command** is now available as an alias for `/recap` to request an on-demand session summary.

## Bug Fixes

- **Local shell tools** now see the same environment variables, aliases, and functions as your login shell.
- **Syntax highlighting** in diffs and the file viewer no longer miscolors strings or comments that span multiple lines.
- **Global rules** from ~/.grok/rules and compatible vendor homes are now discovered correctly.
- **Background tasks** that finish after you press Ctrl+C no longer automatically resume the model.
- **Ctrl+\** out of the dashboard now returns you to the agent you came from.
- **MCP OAuth logins** now succeed against servers that require the RFC 9207 issuer parameter in the callback.
- **Agent dashboard** now shows fleet roster entries even when the local agent list is empty.
- **Long-session compaction** no longer fails on servers that reject tool_choice none when tools are attached.

## Performance

- **Scrolling** feels smoother and less jagged under load or over slow connections.


# 0.2.104 — 2026-07-17

## Features

- **Background work counts** now appear in a persistent status line instead of repeated transcript messages.

## Bug Fixes

- **Fixed authentication recovery** for idle sessions after token timeouts.
- **Retry failed** messages no longer contain raw HTML error pages.
- **Rate limit messages** now show the server detail without the wire prefix.
- **In-place prompt editing** is temporarily disabled due to scroll behavior issues.


# 0.2.103 — 2026-07-17

## Features

- **New require_sha option** prevents remote plugins from tracking mutable branches or tags.
- **Local sessions now inherit full rc environment, cwd, and exports** across tool calls (configurable).
- **MCP servers** from plugins can now require setup choices such as a regional site before connecting.
- Quitting a fullscreen session now shows the session title and last exchange above the resume command.
- **SSH sessions** now show a one-time tip recommending `grok wrap ssh <host>` for clipboard and terminal restore.

## Bug Fixes

- **Fixed GitHub PR status detection** when the gh CLI inherits forcing color environment variables.
- **Fixed a race** where an early cancel could permanently wedge a session's turn slot.
- **grok** and the agent binary now stay in sync even when no update is installed.
- **Copying** a multiline queued prompt now copies the complete text instead of a collapsed summary.
- **grok wrap** now restores the terminal after SSH disconnects or other abrupt child exits.
- **Voice speech-to-text** now works with per-model API keys in config.toml without requiring `grok login`.
- **Copy over SSH** or in containers now shows clearer feedback when delivery cannot be confirmed.
- **Local Bash sessions** no longer keep a persistent shell across calls, avoiding failures after directory deletion.


# 0.2.102 — 2026-07-16

## Breaking Changes

- **--minimal** and **--fullscreen** flags now apply only to the current session.

## Features

- **New /jump slash command** lets you quickly jump to any previous turn in the conversation.
- **New /timeline sidebar** shows a clickable tick rail for fast navigation between conversation turns.
- **grok login** now requests Grok Projects scopes so workspace listing works after consent.
- **Permission mode** can now be set fleet-wide via remote config when no local setting exists.
- **Edit tool output** has a setting to show a compact one-line summary instead of always-expanded diffs.
- **Tab completion** in !bash mode now works like a normal terminal (prefix fill, dropdown, directory drill-down).
- **Enterprise deployments** can now disable voice dictation via `requirements.toml` so `/voice` and Ctrl+Space are hidden for everyone.
- **User prompts** now appear bold only in `--minimal` mode; fullscreen keeps normal weight.
- **`grok plugin install`** now accepts a marketplace's registered name as a qualifier.
- Consecutive edits to the same file now collapse into a single scrollback row when collapsed edit blocks are enabled.
- Local sessions now inherit your shell environment variables and keep the current directory across commands.

## Bug Fixes

- **Login and re-login** no longer stack multiple device-code polls or leave stale flows running.
- **Background task tools** now render with correct icons and titles instead of the generic MCP wrench.
- **Task tool** now correctly validates and displays allowed model slugs for subagents.
- **Rewind** now correctly handles bash transcripts, permission follow-ups, and sessions that mix old and new prompt markers.
- **Re-login** during a session now immediately uses the new token instead of requiring a new session.
- **Terminal commands** using globs now behave the same on zsh as on bash and no longer fail with shell errors.
- **Installer** no longer replaces stowed shell configuration symlinks with plain files on upgrade.
- **Voice transcription** now works with enterprise API bases and API-key authentication.
- **Fixed crashes** on some network-mounted home directories by using a safer SQLite journal mode.
- **Home and End keys** now move to the ends of the current wrapped line in the prompt.
- **Arrow keys and Esc** now work correctly inside viewers opened from the dashboard.
- **Warns at startup** when user and project sandbox profiles define the same name differently.
- **Billing upgrade links** now show the full URL in the transcript (and copy it) when a browser cannot be opened.
- **Fixed Ctrl+Y yank** no longer working after sending a prompt.
- **No longer shows permission prompts** seconds after a turn was cancelled with Esc or Ctrl+C.
- **Page Up and Page Down** now move the highlighted entry to the top or bottom of the visible scrollback area.
- Conflicting project and user sandbox profiles now show a clear warning on the welcome screen.
- **OAuth login URLs** no longer contain duplicate referrer parameters.
- **File links** in official VS Code Remote-SSH terminals now use VS Code's native path handling.
- **Minimal mode** now shows the folder-trust prompt after sign-in when required.
- **Skills** whose names collide with built-in slash commands are now reachable via qualified names.
- **Fixed background task tracking** when using grok -p --no-wait-for-background so tasks are properly reaped on exit.
- **Rate limit errors (429)** now show specific server messages (capacity, team limits, free-usage) instead of generic upgrade prompts, with correct copy based on auth type.
- **`/copy` slash command** is now available in minimal mode.

## Performance

- **Improved recap and compaction** behavior.

# 0.2.101 — 2026-07-13

## Features

- **grok inspect** now shows effective compatibility settings for Cursor, Claude, and Codex sessions.
- **New setting** "Match display refresh rate" lets high-refresh displays run the TUI at native cadence.

## Bug Fixes

- **Parked subagent status** no longer duplicates or interleaves incorrectly in scrollback.
- **Status line** during waits now shows elapsed time before the queued-message hint.
- **Queued messages sent with Enter** now appear immediately instead of vanishing briefly.
- **Resume hint** after quitting minimal mode now prints the correct grok --minimal --resume command.
- **Rate-limit messages** now correctly direct API-key users to team plans instead of personal upgrades.


# 0.2.100 — 2026-07-13

## Features

- **Session picker** now discovers and resumes recent Claude Code, Codex, and Cursor sessions.
- **Welcome screen** now offers a one-click resume nudge for recent Claude, Codex, or Cursor sessions.

## Bug Fixes

- **Web fetch tool** preserves full truncated page content as readable artifacts instead of discarding it.
- **Multiline mode** now correctly sends the top queued message on empty Enter when a turn is running.
- **Queued commands** no longer disappear or delay when pressing Enter twice quickly during a running turn.
- **Minimal mode** text is now readable on dark terminals with proper contrast and highlighted user prompts.
- **Grok no longer crashes** when printing resume hints after the terminal pane has closed.
- **Long-running turns** with multiple waits now show updated status markers in the transcript instead of appearing stuck.
- **Claude and Cursor hooks** are now correctly disabled at session start when disabled in config.


# 0.2.99 — 2026-07-12

## Features

- **Multiline input** now works on the agent dashboard the same way it does in regular sessions.
- **PageUp and PageDown** now scroll the conversation while the prompt is focused.
- **Keyboard Shortcuts** modal now follows Vim mode navigation keys when enabled.


# 0.2.98 — 2026-07-12

## Breaking Changes

- You can now pin authentication to API key or OIDC in config.toml; the unpinned method is no longer tried automatically.

## Features

- **`/context`** now shows token costs for skills and MCP servers.
- **`env_key`** in config now accepts an array of environment variable names.
- Linux middle-click paste from the primary selection now works; clipboard errors are handled more reliably.
- **/terminal-setup** now shows your terminal's color support level and which themes are available.
- **grok setup --json** prints your team's managed configuration without installing it.
- Messages you type while the model waits on tasks now stay queued; pressing Enter twice sends them immediately by cancelling the current turn.
- **How-to Guides** modal now shows a tip linking to Ask Grok above the footer shortcuts.
- **Subagent** `task` and `spawn_subagent` tools now accept an optional `model` parameter in the CLI.
- **Keyboard Shortcuts** modal now lists the paste key binding for images under the Input section.

## Bug Fixes

- A `pre_tool_use` deny now feeds the reason back so the model can retry instead of cancelling the turn.
- Plan mode now strictly rejects edits outside the plan file, even under always-approve.
- **Web search** and X search no longer fail when both a local function tool and the backend hosted tool are active.
- **Content-filter refusals** from providers now show an explanation instead of ending silently with no output.
- **SQLite databases** no longer cause bus errors on network filesystems such as NFS.
- **Resuming** a session that is already open now focuses the existing view instead of creating duplicates.
- **Turn completion** markers in scrollback now read "Worked for …" instead of "Turn completed in …".
- **/btw** loading spinner now animates correctly when the main session is idle.
- **Mid-turn** wait spinners now correctly show "Waiting on task output…" instead of Thinking.
- **Scrollbar thumb** is now visible in the oscura-midnight theme.
- Status messages for background work now end with a period.
- **Editing queued prompts** no longer freezes the terminal or duplicates text into the composer.


# 0.2.97 — 2026-07-11

## Features

- **Headless JSON output** now includes token usage and cost per prompt and session.
- **SDK turns** now expose detailed token usage and cost information via Turn.usage.
- **Double-click or Enter** on a previous user message now lets you edit and resubmit it directly from the transcript.
- **Text selection** in scrollback now works better when starting on chrome, gaps, or while scrolling.
- **Shell commands using `rg`** (ripgrep) no longer require permission prompts by default.
- **Voice mode** is now available for API-key sessions.
- **New environment variables** allow tuning scroll and draw cadence for high-refresh displays.

## Bug Fixes

- **Background tasks** started by the model in headless mode are now killed on exit instead of leaking.
- **Agent process leaks** on failed spawns and missing-stdio teardown are now prevented.
- **Parked turn markers** no longer appear after interjections and now count down as background tasks finish.
- **The /context** tool definitions line no longer shows the cryptic disclaimer suffix.
- **Terminal release** waits boundedly for the process to exit and repeated waits share the drain grace.
- **Fixed a crash** when the agent asked a question on narrow terminals.
- **Fixed misleading keyboard hints** in the /mcps panel in minimal mode.
- **Clipboard copy** now reports success correctly when using iTerm2 over SSH.
- **Fixed scroll/input conflicts** when plan approval appeared over an open edit block.
- **Fixed frequent MCP and skills reloads** that could freeze sessions on devboxes.
- **MCP servers using HTTP** (such as HTTP MCP servers for Slack) now automatically recover from disconnects.
- **Next reset time** in /usage now shows in your local timezone instead of Pacific Time.


# 0.2.96 — 2026-07-10

## Features

- **System notifications** now carry structured kind/title/body for better rendering.
- **x.ai/pr/status** now reports whether an open PR is in the merge queue.
- **Compact mode** now activates automatically on very small terminals.
- **Up arrow** on an empty prompt now browses prompt history; `/history` searches it.
- **Stop hook runs** now appear inline on the turn-completed line instead of a separate block.
- **Subagent rows** now fold into verb-group headers and the tasks pane shows live activity labels.
- **Dashboard shortcuts** now advertise ? instead of Ctrl+. on terminals that cannot deliver the latter.
- **Double-clicking** scrollback while Text selection is fold/nav now shows a tip offering Ctrl+Y to enable Word select.
- **`grok worktree ls`** now works as a short alias for `grok worktree list`.
- **MCP tool output truncation** can now be set per-repo in `.grok/config.toml`.
- **Auto-send of queued follow-ups** during task waits can now be enabled fleet-wide via remote settings.
- **Welcome screen** now offers one-click resume of a recent Claude Code session via ctrl+u.

## Bug Fixes

- **Vim `l` key** now opens the selected agent detail view in the dashboard.
- **Terminal commands** with no args now run through a shell, matching the CLI.
- **Agent teardown** no longer crashes on slim Linux images that lack the ps command.
- **Esc** now dismisses an open /btw panel before backing out of a dashboard overlay.
- **Resumed grok.com chats** now use the conversation's last model instead of the gateway default.
- **JetBrains terminals on Windows** now default to minimal mode to avoid raw mouse-report leaks in the prompt.
- **Skill token highlights** now survive line wraps and the slash menu opens when typing / before existing text.
- **Truncated or tiny images** are now dropped before sending and previously poisoned sessions self-heal on restart.
- **Session switch hints** after `/new` or fork now show the working command in minimal mode.
- **Progress bars** in Ghostty and WezTerm now stop correctly for parked task waits.
- **`/effort`** now rejects levels the current model does not support instead of sending a bad value to the API.
- **`/recap`** on a fresh session now says "No messages yet" instead of failing.
- **Monitor and system messages** no longer appear as user prompts when resuming old sessions.
- **`/rewind`** completion now appears as a brief toast instead of a permanent transcript line.
- **Auto recaps** no longer appear under a newer user message when you start typing again.
- **Authentication retries** after token refresh no longer hang for minutes or days.
- **Text selection tip** no longer appears on the first double-click or on non-assistant blocks.
- **Skill slash commands** queued while a turn runs can now be sent immediately with Enter or the interject chord.
- **Drag text selection** now works inside the dashboard dispatch input box.
- **Multi-line paste chips** in dashboard inputs now support preview and expand like the main prompt.
- **Live previews** now always fetch the latest content without browser or CDN caching.


# 0.2.95 — 2026-07-09

## Features

- **Teams** can now ship default allowed commands via managed_config.toml (user deny rules still win).
- **Mid-turn interjections** now appear as normal user prompts (❯) instead of a separate cyan block.

## Bug Fixes

- **IME text input in Otty** no longer attaches unrelated clipboard images on every character.
- **Rewind** now fully removes the selected turn from both scrollback and the model's conversation history.
- **Queued prompts** now abort long blocking waits instead of waiting for the full timeout.
- **File links and media** now work for worktree sessions under ~/.grok/worktrees/.
- **Collapsed Read/Edit tool rows** now show only the filename instead of long absolute paths.
- **Clipboard copies on Wayland** now succeed even when the terminal loses focus mid-copy.
- **User messages queued** behind an auto-wake turn are no longer lost when the user presses Ctrl+C.
- **Slash completion** now shows sibling skills that share a frontmatter name and correctly sizes wrapped descriptions.
- **Single tool calls** that belong to a verb group now collapse into an aggregated header row.
- **Fixed sessions** that became permanently stuck after tool-use history corruption.
- **/always-approve** and **/auto** now toggle their mode on and off when run repeatedly.
- **Terminal command cards** on grok.com now correctly settle after foreground bash tasks.
- **Copy failure** toast now recommends trying /minimal for native terminal rendering.

## Performance

- **File watching** on Linux now uses far fewer system resources for large projects with many dependencies.


# 0.2.94 — 2026-07-09

## Features

- **/sessions** now opens the Agent Dashboard instead of a separate picker.
- **New /goal <objective>** slash command** is now available when the workspace supports it.
- **grok inspect** now lists skills from [skills].paths and correctly labels bundled vs user skills.
- **--minimal** and **--fullscreen** choices are now remembered for future plain grok launches.

## Bug Fixes

- **Queued bash commands** promoted at turn end now render their output instead of disappearing.
- **Xcode / Foundation ACP clients** can now drive grok agent stdio without silent parse drops on session/* calls.
- **read_file** now returns full single-line content (minified JSON, large dumps) instead of silently clipping at 2000 characters.
- **Background task** command preambles with newlines now render on separate lines instead of collapsing.
- **Text selections** now highlight uniformly even over inline code, links, and syntax-colored spans.
- **grok --minimal** now supports native drag-select on classic Windows conhost terminals.
- Skill tokens such as /pr-workflow are now highlighted teal when used mid-sentence.
- Fixed a crash when a filtered list shrinks while the filter is active.
- Scroll lines and scroll speed settings now support fine unit-step adjustments.
- Project-specific Claude plugins are no longer visible outside their project directory.
- First prompt no longer stalls for many seconds on large repositories while the filesystem watcher starts.


# 0.2.93 — 2026-07-08

## Breaking Changes

- **Esc** no longer cancels a running turn; use **Ctrl+C** instead. Double-Esc rewind now works while focused on scrollback.

## Features

- MCP permission prompts now show the planned arguments so you can judge what the tool will actually do.
- The "Managed by grok.com" link in the Extensions modal is now clickable and underlined.
- Dragging inside rendered markdown tables now selects whole cells or rectangular ranges and copies as TSV.
- Shift+Tab now goes straight to Plan mode when the plan-mode tip is showing.

## Bug Fixes

- **grok --minimal** now aligns the prompt, status bar, and messages flush-left with the welcome card.
- **/plugins** no longer lists never-installed Claude marketplace entries and now groups plugins by their real source.
- Successful image compression no longer leaves a permanent line in the transcript.
- **--no-ask-user** now also disables ask_user_question for subagents.
- **--no-ask-user** now also disables ask_user_question for subagents.
- **Fixed a crash** shortly after launch on some systems caused by the telemetry exporter.


# 0.2.92 — 2026-07-08

## Features

- **/minimal** and **/fullscreen** commands let you switch the current session between minimal and fullscreen modes.
- **ask_user_question tool** can now be enabled or disabled via config.toml, environment variables, or remote flags while defaulting to on.

## Bug Fixes

- **User-run shell commands** now display their complete output after finishing instead of silently dropping middle lines.
- **Edit tool output** now correctly highlights multi-line strings and scopes that previously spilled across hunks.
- **Always allow** grants for MCP, web_fetch and bash now take effect immediately in auto mode without re-prompting.
- **Cmd/Ctrl+click** on bare http(s) links now opens only once on Warp terminals.
- **Cmd/Ctrl+click** now works on imagine media paths and URLs that wrap across multiple terminal rows.
- **grok update** on Windows no longer fails when a previous .old executable is still running.

## Performance

- **Pasting images** on macOS is now ~65× faster by reading the pasteboard directly instead of via osascript.


# 0.2.91 — 2026-07-07

## Bug Fixes

- **Voice dictation indicator** and stop button now remain visible and clickable during plan mode review.
- **New Worktree dialog** now expands to show long names and scrolls with a leading … when the terminal is narrow.

# 0.2.90 — 2026-07-07

## Features

- **New /minimal and /fullscreen slash commands** let you switch the current session between minimal and fullscreen modes without quitting.
- **Session titles** from /rename now appear on the prompt box border after resume.
- **grok models** banner now correctly reports per-model API keys and deployment keys.
- MCP tool output size limit is now configurable via environment variable, config.toml, or remote settings (default unchanged).
- Chat conversations listed in the unified sidebar can now be renamed or deleted from the desktop app.
- You can now add a local directory as a plugin marketplace source with `grok plugin marketplace add`.
- **Auto permission mode** now prompts far less often on routine development commands.
- Short media paths the model prints (images/1.jpg) are now clickable and open the file.
- **Preview** now prefers common dev ports like 8080 when multiple HTTP servers are detected.

## Bug Fixes

- **Model list now refreshes** after upgrading from a free to a paid subscription tier.
- **Extensions modal** now shows clearer enable/disable and install hints that match what each key actually does.
- **Folder trust** no longer prompts for or scans the entire home directory when it is a git repo.
- **Code blocks** no longer lose their background shading on the final line of unterminated fences.
- **Plan mode** now activates immediately when toggled during an active turn instead of waiting for the next prompt.
- Clicking back into the terminal window now focuses the prompt immediately when a permission or plan-approval panel is waiting.
- Paths the model emits inside quotes no longer end with literal backslash-n characters and cause file-not-found errors.
- **Next reset** time shown by /usage is now correct during daylight saving time.
- The ask-user-question tool now waits up to 30 minutes by default before timing out.
- The free-usage paywall no longer offers a Try Again button.
- Inline images no longer bleed through when entering or leaving the fullscreen subagent view.
- The [Open Image] button under generated media is now colored like a link.
- **Preview routing** now auto-selects well-known ports like 8080 even with framework signals on obscure ports.
- **Login screen** now centers the authentication URL when it fits on one line.
- **Enter key** now queues follow-ups by default while the agent waits on task output.


# 0.2.89 — 2026-07-07

## Features

- **Voice dictation** now works on Linux (requires pipewire, pulseaudio-utils or alsa-utils).
- **New /auto slash command** switches to classifier permission mode; the menu now shows only the other mode.
- **--effort** and **--reasoning-effort** are now interchangeable CLI flags for setting reasoning effort.
- **Image edits** now use the higher-quality Imagine model for better output.

## Bug Fixes

- **Try Again** on the free-usage paywall now correctly resubmits after rate-limit retries.
- **Cursor** now respects your terminal's default blink style instead of always blinking.
- **Skill commands** in scrollback now highlight only the command name, not the arguments.
- **Plan files** now default to .grok/plan.md to match Grok conventions.
- **LaTeX math** renders correctly for display equations and complex subscripts.
- **Queue hint** in the terminal no longer shows incorrect bold text on part of the message.

## Performance

- **Git operations** like rebase no longer cause long pauses from repeated full-repo scans.


# 0.2.88 — 2026-07-06

## Features

- **Scrolling** feels smoother with better trackpad and wheel handling plus configurable speed and mode.
- **Session search** now returns tighter multi-word results and handles filenames and plurals better.
- **Session picker** now always searches conversation content for queries of two or more characters.
- **Tool call grouping** is now enabled by default, folding consecutive reads and searches into single rows.
- **Plugins tab** now supports `u` to update the selected plugin and shows non-blocking success feedback.
- **Reasoning effort** used for a session is now recorded in summary.json and conversation history.

## Bug Fixes

- **Session content search** now correctly indexes messages containing escape sequences like newlines and quotes.
- **Formatted links** now keep their link color when wrapped in bold, italic, or strikethrough.
- **Resuming sessions** no longer fails permanently when history files contain corrupted lines from interrupted writes.


# 0.2.87 — 2026-07-05

## Features

- **Subscription upgrades** are now detected automatically without restarting the CLI.
- **Bash permission prompts** now offer a "Never allow" choice that persists the deny rule.
- **New `/docs` slash command** opens How-to Guides picker, browses web docs, or jumps directly to a guide by title.
- **Per-model reasoning effort menus** are now configurable from the server and config.toml without a client release.
- **Finished thinking blocks** now fold into grouped tool-call rows when group_tool_verbs is enabled.

## Bug Fixes

- **--minimal** mode now always uses your terminal's own colors so text stays readable on any background.
- **Invalid fields** in [model.*] config blocks no longer cause the whole model to disappear from the picker.
- **File tools** no longer target paths with literal trailing newlines or whitespace from model output.
- **Fixed interactions** in no-freeform question modals so clicks below the last option no longer enter input mode.
- **Background tasks** now correctly wake the agent after a cancelled blocking wait instead of staying idle.
- **Copying quoted text** from rendered responses no longer includes the quote bar prefix in pretty mode.

# 0.2.86 — 2026-07-04

## Features

- **Voice language** setting now lets you pick speech-to-text language (including System) in the Editor settings.
- **Tab autocomplete** now suggests your next prompt as ghost text after each turn.
- **/usage** (and /cost) is now hidden for free and X Basic personal accounts.
- **Media generation** no longer hits per-session file limits; image and video byte budgets increased.

## Bug Fixes

- **--minimal** flag now shows in `grok --help`.
- **Session resume notifications** no longer appear when a workspace boots for the first time.
- **Claude-style Bash(cmd:*)** permission rules are now correctly translated to prefix matches.


# 0.2.85 — 2026-07-03

## Features

- **Pressing Enter** on an empty prompt now sends the top queued follow-up immediately while a turn is running.
- **Tool call grouping** can now be enabled via config.toml or settings to collapse consecutive read/search/list calls.
- **Consecutive tool calls** of the same kind can now be folded into a single row when group_tool_verbs is enabled.
- **Subagent conversations** now receive the same type-specific instructions in gateway chat as in the CLI.
- **Scheduled automation tasks** now show their header panel correctly in gateway chat sessions.
- **Promotional announcements** now appear with clickable CTAs and can be non-dismissible.
- **Subagents** now run in the background by default unless explicitly set to false.

## Bug Fixes

- **Permission prompts** now correctly wrap long bash commands while preserving structure and quotes.
- **Claude Code settings** with permissions.defaultMode are now correctly honored.
- **Project skills and commands** are now discovered even when their directories are gitignored.
- **Inline LaTeX math** with padding spaces now renders correctly instead of showing raw dollar signs.
- **Manual /recap** now works on the same turn and long auto recaps are hidden from view while still saved.
- **Always allow** for common commands like ls and git status now remembers just the command instead of extra arguments.


# 0.2.84 — 2026-07-03

## Features

- **Announcements** now update live during active sessions without restart or `/new`.
- **Hiding** an announcement no longer suppresses later criticals; new ones reappear automatically.
- **run_terminal_cmd** now requires a one-sentence `description` rationale in every invocation.
- **ask_user_question** timeout policy is now configurable in config.toml and `/settings`.
- **Ask-Question timeout** can now be toggled from `/settings` (Agent & Approval).
- **Thinking/reasoning blocks** are now shown by default while the model is working.
- **Critical announcements** now show a red title with a clickable [hide] button and aligned message.
- **Added remote_fetch option** under [features] in config.toml to disable all backend catalog and settings fetches for air-gapped environments.

## Bug Fixes

- **Images** pasted or read from GIF, BMP or TIFF files are now automatically converted so they work with image generation.
- **Queue panel** now shows action buttons on hover and the status bar displays a compact done/total task count.
- **Hook matchers** now correctly see the real MCP tool name instead of the internal dispatcher name.
- **Copy** now succeeds when running inside containers even when the terminal brand cannot be detected.
- **Tool result previews** no longer paint opaque panels in `grok --minimal`.
- **grok wrap** now correctly handles quoted strings and shell aliases.
- **Text selection** settings now correctly honor explicit keep_text_selection values even when legacy keys remain.
- **Fixed a freeze** that could occur when editing and sending the last message in the queue.
- **Fixed a startup crash** on minimal Linux systems lacking system CA certificates.

## Performance

- **Grep** now stops early on broad searches, returning faster results with far less memory use.
- **Idle CPU and memory** usage after long sessions or resume is now dramatically lower.


# 0.2.83 — 2026-07-02

## Features

- **Critical announcements** now appear in a top banner during active sessions with a hide command.
- **Pasting the same text again** next to a paste chip now expands the chip into editable text instead of duplicating it.
- **Paste preview** now shows a hint explaining how to expand the chip.


# 0.2.82 — 2026-07-02

## Features

- **Managed connectors links** now include the team ID when opening from a team session.
- **AGENTS.md files** are now discovered and shown for workspace/hub sessions.
- **Chat conversation titles** from the gateway are now shown in the sidebar.
- New `/effort` slash command changes reasoning effort on the active model.
- Double-click a pasted text chip to expand it into editable text.

## Bug Fixes

- **Skill descriptions** are now recovered correctly even when frontmatter YAML is malformed.
- **[Esc] hint** in /btw panels now stays visible even on narrow terminals.
- **Background monitors** now wake the agent on natural exit the same way bash tasks do.
- Long option labels in question prompts are now always visible instead of disappearing when unfocused.
- Pasted text preview now appears immediately after inserting a paste chip.
- Hex color codes now render as colored dots with no extra space.
- Pressing voice on the welcome screen now starts a new session.

# 0.2.81 — 2026-07-01

## Features

- **Chat sessions** no longer send workspace binding hints that belong to the backend.
- **New stream transforms** let hosts hide, unwrap, or rewrite tool calls for display without affecting agent transcripts.
- **cancel()** now accepts a timeout so a stuck turn cannot hang the session forever.
- **Run tool blocks** now show the model's description as the main title when provided.
- **Hex color codes** in prose now render as colored dots on truecolor terminals.
- **New setting** "Show thinking blocks" controls whether agent reasoning is visible in the scrollback.
- **Spinners** now show the description or short command of the task being waited on when available.

## Bug Fixes

- **Fixed sessions** that remained stuck on the thinking indicator after the model finished responding.
- **Mermaid diagrams** now correctly display angle brackets and symbols instead of literal HTML entities in labels.
- **Recap requests** no longer trigger context-length 400 errors on long conversations.

## Performance

- **Grep searches** now time out after 20 seconds by default (60s on WSL) instead of always waiting 60s.

# 0.2.80 — 2026-07-01

## Features

- **Command timeouts** can now be configured per-session with a foreground-only ceiling.
- **Background tasks and TODO lists** now survive compaction and remain visible to the model.
- **Voice dictation** STT feature: uses Ctrl+Space or F8, with optional hold-to-talk on supported terminals.
- **Contextual hints** can now be toggled individually for undo, plan mode, and image input.

## Bug Fixes

- **Subagent dialogs** now reliably show full transcripts on open and reopen.
- **Recap blocks** now copy only the summary body, not the header label.
- **Vim navigation keys** now type into dashboard prompts; modals properly handle Esc/Left.

## Performance

- **Network connections** are now more resilient to proxy/LB drops.

# 0.2.79 — 2026-06-30

## Features

- **Contextual hints** now show shortcuts like plan mode or clipboard paste when relevant.
- **Graceful shutdowns** now allow interrupted turns to resume with a configurable pause budget.
- **Grok.com chat sessions** now integrate fully with the gateway bridge for model catalog and resume.

## Bug Fixes

- **Question prompts** now time out after 6 minutes instead of blocking forever.
- **Fixed a crash** that could occur during conversation integrity repairs while a turn was active.

## Performance

- **Compaction** can now run part of its work in the background before it blocks the session.

# 0.2.78 — 2026-06-30

## Features

- **Chat sessions show the grok.com model catalog** in the picker.

## Bug Fixes

- **Tabs pasted into the prompt** now align correctly with proper cursor positioning.
- **Pasting images into dashboard peek replies** now works and survives turn cancellation.
- **Links in /btw panels** are now clickable and highlight on hover.
- **Prompt history is now saved** even on fast Ctrl+C quit.
- **Stuck scrollback text selection** can now be cleared with Esc or any non-drag input.
- **LaTeX math now renders** inside markdown tables in the TUI.
- **Background shell commands** started by the agent are now cleaned up when the CLI exits.

## Performance

- **`grok update`** downloads have a longer timeout.


# 0.2.77 — 2026-06-30

## Features

- **Pasting images** from the local clipboard now works when running commands through `grok wrap`.
- **Turn status spinner** now shows what the agent is waiting on (response, subagent, task output, etc.).
- **Double-click word selection** is now a discoverable option in the Text selection setting and stays in sync with highlight behavior.

## Bug Fixes

- **Credit limit errors** now show clearer upgrade or buy-credits messaging based on billing type.


# 0.2.76 — 2026-06-30

## Features

- **Auto permission mode** is now added to the top of Shift+Tab cycles and enabled by default in settings.
- **grok agent stdio** now checks for updates in the background like other modes.

## Performance

- **Idle sessions** no longer send repeated empty frames to the terminal, reducing CPU usage in the terminal emulator.


# 0.2.75 — 2026-06-29

## Features

- **Prompt history** (Up arrow / Ctrl+R) now shows only the current session's prompts, with the newest selected at the bottom.

# 0.2.74 — 2026-06-29

## Features

- **Esc now cancels a running turn immediately**; double-Esc clears prompt or opens rewind when idle.
- **grok wrap** now shows copy success over SSH and suggests native drag-select when paste fails.

## Bug Fixes

- **Clipboard copy** now succeeds reliably on Wayland and KDE desktops instead of showing false positives.


# 0.2.73 — 2026-06-28

## Features

- **Keep text selection highlight** setting added so drag selections stay visible until dismissed.

## Bug Fixes

- **Doubled lines** after tab switches or focus changes in tmux or editor terminals are now healed.
- **Clipboard copy** now only shows success when the pasteboard actually received the text via a trusted path.


# 0.2.72 — 2026-06-28

## Bug Fixes

- **No longer triggers browser login** at startup when an API key is already configured for inference.


# 0.2.71 — 2026-06-27

## Bug Fixes

- **Fixed `grok agent stdio` hangs** on Windows when used with persistent clients such as VS Code.


# 0.2.70 — 2026-06-27

## Breaking Changes

- **Added `grok wrap`** to run any command with local clipboard support.

## Features

- **Ctrl+4** now toggles the prompt queue on local macOS VS Code terminals.

## Bug Fixes

- **Session recaps** (/recap and return-from-away) now show the full summary instead of being cut off mid-sentence.
- **Vim mode** now focuses the prompt when you press / on a brand-new empty session.
- **Fixed `grok agent stdio` startup hangs** on Windows when used with persistent clients such as VS Code or grok-desktop.
- **`/mcps` list** no longer shows stale disabled entries when managed gateway tools are enabled.
- **Mermaid diagrams opened via [Open Image]** now render at higher resolution instead of terminal size.
- **Pressing `r` in scrollback** no longer accidentally rewinds the session.
- **Shortcuts cheatsheet** now shows Ctrl+X on terminals that cannot deliver Ctrl+.
- **Folder trust prompts** no longer re-appear for every standalone worktree clone.
- **Reasoning effort** no longer silently resets from a user-chosen value after catalog refreshes.
- **Fixed clipboard copy** inside editor terminals nested in tmux by emitting plain OSC 52.

# 0.2.69 — 2026-06-26

## Features

- The agent dashboard now shows each agent's model and mode in the peek panel, lets you cycle modes with Shift+Tab, collapses the Inactive section by default, and hides older idle agents behind a "N more" row.
- Tool usage cards for search, directory listing, file deletion and glob now render as distinct typed cards instead of generic MCP entries.
- The keyboard shortcuts help now shows richer descriptions and correctly scrolls wrapped text in the detail view.
- You can now pass --json-schema to grok -p and receive a validated JSON object instead of free text.
- **Ctrl+L** now interjects mid-turn in VS Code, Cursor, Windsurf, and Zed terminals.

## Bug Fixes

- Local plugins installed from your home directory are now automatically refreshed when you start a session, so new agents or skills added to the source appear immediately.
- The /context command now reports the same number of tool definitions that are actually sent to the model.
- In vim mode the agent dashboard peek no longer steals keyboard focus from the list, so j and k keep moving between agents.
- **/sessions** on the agent dashboard no longer freezes the interface.
- **Dashboard** now focuses the overview list immediately when agents exist.

# 0.2.68 — 2026-06-26

## Features

- **MCP servers** from host integrations can now be added, replaced, or removed without restarting the session.
- **Agent-run terminal commands** now set `GROK_AGENT=1` so host tools can tell them apart from interactive shells.

## Bug Fixes

- **Attached images** are now saved to real disk paths so the model can read them in any terminal.
- **/resume** now selects the correct model when a saved model name is ambiguous.
- **Slash and completion menus** no longer crash if the terminal is resized while open.


# 0.2.67 — 2026-06-25

## Features

- **Added --json-schema** flag for headless mode to constrain model output to a supplied JSON Schema.
- **Idle detection** can now ignore background tasks when the env flag is set (off by default).

## Bug Fixes

- **Preview panes** no longer hibernate while actively viewed or polled.
- **Manual /rename** now persists correctly and appears in /session-info even after auto title generation or resume.

## Performance

- **Find and grep** now transparently use faster bfs and ugrep binaries when present in the harness.


# 0.2.66 — 2026-06-25

## Features

- **Custom sandbox profiles** can now kernel-deny specific files and directories for reads/writes.
- **Marketplace plugins** in subdirectories of a git repo can now be installed and loaded correctly.
- **Folder trust prompt** now appears before starting a session when the feature is enabled.
- **Preview panes** no longer hibernate while actively viewed.
- **Keyboard shortcuts help** now expands inline for individual entries instead of only sections.
- **Idle detection** can now ignore background tasks when the env flag is set (off by default).
- **Sandbox deny lists** now accept glob patterns like **/*.pem** in addition to exact paths.

## Bug Fixes

- **Local MCP servers** now auto-recover after disconnects or session expiry.
- **OIDC sessions** with XAI_API_KEY present no longer lose refresh on idle.
- **Inline video previews** now show an install command only when the package manager is on PATH.
- **list_dir** now reliably shows all immediate child directories even inside large monorepos.
- **Clicking a model** in the dashboard /model dropdown no longer opens the wrong session.
- **Strikethrough** now only applies to ~~double tildes~~; single ~tildes~ render literally.
- **Session cycling** with Ctrl+[ / ] now switches from the session you are currently viewing.
- **Prompt history** (Up / Ctrl+R) now shows the complete recent list instead of a scrambled partial one.
- **Authentication** now correctly prefers the session method when both API key and cached token are present.
- **xychart-beta** diagrams with category labels now render correctly as images.


# 0.2.65 — 2026-06-24

## Features

- **grok -w --ref <branch>** now creates worktrees based on the specified ref instead of HEAD.

## Bug Fixes

- **Unidentified Windows consoles** are now treated as Windows Terminal for capability decisions.
- **Esc** in the dashboard input now moves focus to the list without clearing your typed draft.
- **Copying** a tool header now copies just the path or command, not the Read/Run label.
- **Execute activity** lines and headers no longer repeat a redundant cd into the session directory.
- **Inline video previews** now show an install hint instead of a spinner when ffmpeg is missing.

## Performance

- **Headless and stdio sessions** no longer start unnecessary filesystem watchers, saving CPU and IO.
- **Scrolling** feels more responsive in VS Code, Cursor, and Windsurf integrated terminals.


# 0.2.64 — 2026-06-24

## Features

- **Dashboard** now displays the current directory and branch; click or press Ctrl+L to change location, or Ctrl+W to dispatch new agents into fresh git worktrees.
- **/recap** now appears as a collapsible tool-style block with a loading spinner while generating.

## Bug Fixes

- **Dashboard** arrow keys open agent details and exit overlays; closing an agent now selects the neighboring row.
- **/usage** command and credit warnings are now hidden for API-key authentication.
- **MCP servers** from your user config no longer appear labeled as project-scoped when running from your home directory.


# 0.2.63 — 2026-06-23

## Bug Fixes

- **Fixed hook matchers** so pipe-list and alias patterns no longer silently over-match unrelated tool names.


# 0.2.62 — 2026-06-23

## Features

- **Hosts can now register hooks** over the agent connection instead of only on-disk files.
- **Prompt and /usage warnings** now correctly reflect prepaid credits and auto top-up status.
- **Desktop clients can now detect** when a terminal is busy running a foreground process.
- **TODO list** remains visible to the model after compaction so it can continue working on pending items.
- **/recap** is now available by default — it generates a quick summary of your current session so you can catch up on what's happened so far.

## Bug Fixes

- **MCP server connections** no longer time out during slow cold starts of stdio servers that download dependencies.
- **File paths containing spaces** (e.g. macOS app bundles) are now correctly turned into clickable hyperlinks in the terminal.
- **Resume** now correctly picks the most recently active session instead of one that only had metadata updates.
- **/goal** slash command now appears in the menu on the welcome screen before any prompt is sent when the feature is enabled.
- **Session picker** no longer shows a stale row highlight when keyboard focus moves to the search bar.
- **Usage percentages** in /usage and warnings now match backend flooring and show pay-as-you-go limits when applicable.
- **Team accounts** can now list sessions after re-login; previously returned 403 on conversations API.


# 0.2.61 — 2026-06-22

## Features

- **Closing a terminal tab** with a running process now shows a confirmation dialog instead of killing it immediately.
- **/usage** now shows prepaid credits balance and auto top-up status.
- **Clipboard copy** on Wayland now also tries wl-copy; per-leg outcomes are now logged for diagnostics.
- **Goal mode toggles and limits** can now be set in config.toml under the [features] table.
- **All /goal options** (toggles, limits, role models) are now configurable together in a [goal] table.
- **Clipboard copies** from VS Code over SSH now warn when non-ASCII text may be garbled.

## Bug Fixes

- **Focus reports** no longer leak as literal text when split across reads over SSH.
- **--disable-web-search** now honored in grok -p and grok agent; auxiliary model routing respects catalog overrides.
- **Focus events** now fire correctly for SSH-split focus reports.
- **Boolean tool flags** now accept "true"/"false"/"yes"/"no"/1/0 strings and numbers in addition to native booleans.
- **Session last-active timestamps** and message counts no longer regress under concurrent writers.
- **iTerm2** now always uses text/metadata image fallback instead of broken OSC 1337 overlays.
- **Model switches** no longer leave the prompt queue stuck after a reconnect.
- **Closing a terminal tab** with a running process no longer shows a confirmation dialog.
- **Custom agent profiles** now correctly use the harness required by their pinned model.
- **Subagents** under custom profiles now adopt the correct harness from the parent's model.
- **Changelog and release-notes** modals now scroll with the mouse wheel and arrow keys.


# 0.2.60 — 2026-06-21

## Features

- **/resume** now shows sessions from your current working directory's repo at the top of the list.
- **Too-wide Mermaid diagrams** now show a hint below the fallback box pointing to the Open Image button.
- **Cancel behavior for running subagents** can now be set to always stop or always continue in config.toml.

## Bug Fixes

- **Compaction** no longer hangs indefinitely when the summarizer stream stalls after the server has finished.
- **Slash command completion** now shows consistent suggestions and remembers recently used commands.
- **Queued prompts** now reappear reliably after deleting the last item and re-queuing.
- **Headless sessions** no longer produce authentication error noise from unauthenticated MCP servers.
- **Mermaid flowchart labels** with long identifiers are now kept whole instead of being cut mid-word.
- **Cmd+Backspace** now deletes only from the cursor to the start of the line instead of clearing the whole prompt.
- **Inline Mermaid previews** now break long identifiers at word boundaries instead of mid-segment.
- **Signed git commits** no longer corrupt the TUI by letting pinentry draw over the screen.
- **Arrow keys** now move the prompt cursor or open history while a /btw answer panel is visible.
- **Long option descriptions** in question prompts now expand fully when the row is focused.

## Performance

- **Large MCP tool results** are now truncated inline and saved to disk to avoid unnecessary context compaction.


# 0.2.59 — 2026-06-19

## Bug Fixes

- **Session recaps** no longer display doubled labels and manual recap now correctly suppresses the next automatic recap.


# 0.2.58 — 2026-06-19

## Bug Fixes

- Terminal command output files are now capped at 5 GB during execution and truncated to 64 MB after the process exits.
- Interjection messages now display the actual user text instead of a generic header.
- The legacy `agent` command is now kept in sync with `grok` after running `grok update`.
- Headless (`grok -p`) runs now wait for background tasks and subagents to finish before exiting.


# 0.2.57

## Features

- Improved resilience to network blips during long responses by resuming instead of failing the turn.
- **`grok plugin install <name>`** now resolves plugins from registered marketplaces instead of only local paths.

## Bug Fixes

- Fixed cases where long-running conversation compaction could hang indefinitely.
- Notification hooks now fire only for real user-attention events and no longer trigger constantly during tool use.
- Fixed literal display of HTML entities such as &lt; and &gt; in responses and tool output.
- **Typing `[`** in the pager prompt no longer appears delayed.
- **Copy** now tries all available Linux clipboard tools so paste works reliably in more terminals.


# 0.2.56

## Features

- **resume_from** now continues a finished sub-agent in place instead of forking a new conversation.
- **grok sessions delete <id>** command now lets you permanently remove a session from the CLI.

## Bug Fixes

- **MCP server connections** no longer get torn down during rapid config reloads.
- **Stale leader processes** are now cleaned up when leader mode is disabled via config or remote settings.
- **Sandbox profile** is now preserved when resuming sessions so commands continue to work as before.
- **list_dir** now shows more relevant files when a large directory appears early in alphabetical order.
- **Cancel button** in turn status always shows [stop]; queue pane highlight now follows theme changes.
- **grok quit** no longer hangs when background git or network tasks are slow.
- The token count shown after auto-compaction now matches the context bar exactly.
- The git branch icon now renders correctly in iTerm2 without a Nerd Font.
- **list_dir** now gives clearer guidance when a directory is too large, using the actual tool names available in your session.
- **Ctrl+Enter** now sends the prompt when the agent is idle (same behavior as Enter).
- **resume_from** now correctly continues a sub-agent in the same working directory it was using before.
- Files with non-ASCII names (e.g. Chinese) no longer crash the session when plan mode checks for markdown.
- Session lists (welcome screen, /resume, grok sessions list) are now sorted by the same activity time shown in the UI.
- **Fixed bash tool failures** when models send numeric arguments such as timeout as JSON strings instead of numbers.
- **Prevented crashes** during bash command output streaming when building progress frames.
- **Disabled inline image rendering** on iTerm2 terminals where scrollback overlays cannot be supported.

## Performance

- Fast tools like grep now show as completed immediately even when other tools in the same round are still running.
- Long sessions that display inline images no longer grow to multi-GB memory usage.


# 0.2.55

## Features

- **Added option** to fully disable the hunk tracker via --hunk-tracker-mode, GROK_HUNK_TRACKER, or config.

## Bug Fixes

- **Windows install scripts** now download and run cleanly via irm | iex without spurious BOM errors.
- **Tables and wide content** no longer leave stray characters next to timestamps in the scrollback.
- **Mermaid diagrams** now render node labels cleanly without HTML tags or raw markdown syntax.
- **MCP servers using HTTP** now recover automatically after temporary connection drops instead of becoming permanently unavailable.
- **Very long sessions** can now scroll all the way to the bottom of the conversation history.


# 0.2.54

## Features

- **Rewind** now works end-to-end across conversation and file state with proper CAS handling.

## Bug Fixes

- **Git branch icons** now render correctly on Windows without Nerd Fonts.
- **Mermaid diagrams** now render inline without the model suggesting external viewers.
- MCP connection errors now show the actual failure reason to the model.
- MCP servers with noisy stdout no longer disconnect unexpectedly.
- **Usage warnings** now always display "Usage left: N%" instead of varying between "Free credits left" and "Credits left".
- **Window title** no longer flashes or oscillates during permission prompts while the terminal window is focused.

## Performance

- **Fixed pager freezes** and 100% CPU usage when rendering very long agent reasoning outputs with thousands of styled spans.


# 0.2.53

## Bug Fixes

- Minor bug fixes.

# 0.2.52

## Features

- **Tool auto-approval (YOLO)** state is now tracked end-to-end in server-side agent sessions.
- **ER diagrams** now render as entity boxes with attributes and relationships in the TUI.
- New "Respect manual folds" setting keeps hand-expanded blocks stable while content streams in.
- **Ctrl+X** now stops running turns or closes sessions from inside the agent detail view.
- **Grok** can now export usage metrics and events to your own OpenTelemetry collector when enabled.
- **WezTerm users** now receive guidance when Shift+Enter fails because kitty keyboard protocol is disabled.
- **Long-running sessions** now tell the model when the local calendar date changes past midnight.
- **Agent Dashboard** now works without leader mode and shows local idle sessions from disk.

## Bug Fixes

- **Fixed oversized session replay logs** that prevented large sessions from loading.
- **MCP server connections** no longer flood reconnects on repeated stream errors.
- **ZDR and team upload flags** are now populated immediately on login instead of only after background refresh.
- **Mermaid PNG export** now handles quoted cardinalities in class diagrams and readable ER rows on dark theme.
- **Skill catalog** no longer shows duplicate "Use when:" labels and check-work skill now prompts the model to read its instructions.
- Compaction now rejects overly-short summaries that would discard real conversation state.
- Background tasks no longer emit spurious failure messages when a session is resumed.
- **Fixed Windows path handling** so external tools and model prompts receive clean paths without \?\ prefixes.
- **Images and media** no longer remain visible when switching from an agent view to the dashboard.
- **Clipboard paste** (Ctrl+V) now works for images on pure Wayland sessions.
- **Modals** such as /sessions no longer crash on narrow terminals.
- **ptyctl resize** now correctly notifies the child process.
- **Concurrent updates** to the same version no longer fail with permission or EEXIST errors.
- **Mermaid diagrams** containing CJK or other non-Latin text now render correctly instead of tofu boxes.
- **`grok dashboard`** now reliably opens the dashboard instead of silently falling through to a normal session.
- **Sessions** no longer remain blocked forever after a transient model catalog outage during reconnect.
- **Cancel** no longer leaves the interface stuck on "Cancelling…" after lost responses during reconnects.
- **Forked sessions** now retain the parent's full pre-compaction transcripts instead of only the compacted summary.
- **web_fetch** errors on GitHub hosts now recommend using the gh CLI when internal access is blocked.
- **MCP server connections** no longer hang when stdio servers emit undecodable lines.
- **Ctrl+C cancels** now complete in under 50 ms instead of blocking for seconds.
- **Repeated varied edit failures** on one file no longer trigger doom-loop warnings or terminations.

## Performance

- **Compaction** now reuses cached prompt prefix instead of full prefill.

# 0.2.51

## Breaking Changes

- **`grok mcp add`** now accepts positional arguments (e.g. `grok mcp add filesystem -- npx ...`), supports --scope project, and adds -e/-H flags for env/headers.

## Features

- **Mermaid flowcharts** now render subgraph blocks as titled frames with correct internal and cross-boundary edges.
- **Class diagrams** in Mermaid now render as proper UML boxes with attributes, methods and inheritance arrows instead of raw source.
- **Permission prompts** now accept a double-click on an option to submit it, matching the existing Enter and number-key shortcuts.
- **New /code-review slash command** now ships with the CLI and is always available.

## Bug Fixes

- **Plan mode exit reminders** no longer appear after the model has already started implementing the plan.
- **Expanded thinking blocks** in scrollback now remain expanded when the agent finishes them.
- **`grok update`** no longer downloads the same binary twice when multiple updaters or leader checks run concurrently.
- **Background task IDs** after /compact are now shown verbatim so the model can reference them correctly in later tool calls.
- **Typing /** while scrollback is focused now focuses the prompt and opens the slash-command dropdown.
- **Dashboard empty state** is now a single hint line; dispatch and peek placeholders appear only when unfocused.
- **Fixed memory leaks** that could cause the CLI to use tens of gigabytes during long sessions with many tool calls.
- **Login on SSH or headless machines** now tells you when the browser cannot be opened automatically and shows the URL to visit manually.
- **Fixed git clone failures** on Windows when the CLI tries to clone marketplace plugins into ~/.grok.

## Performance

- **Large code blocks** inside lists no longer cause multi-second UI stalls while streaming responses.


# 0.2.50

## Features

- **Mermaid flowcharts** now render edge crossings clearly instead of fusing unrelated connections.

## Bug Fixes

- **Sequence diagrams** with activate, autonumber, par, and more now render instead of showing parse errors.
- **MCP servers menu** and slash commands now work when starting grok outside a project directory.
- **Ctrl+W** in the prompt now deletes whole words like bash instead of stopping at punctuation.
- **Login** no longer quits when an authentication code contains the letter q.


# 0.2.49

## Features

- Marketplace plugin listings now show skills, MCP servers, and commands when the catalog is published.
- Mermaid flowcharts now render with fewer avoidable edge crossings.
- **stateDiagram** mermaid blocks now render as Unicode diagrams instead of source fallback.

## Bug Fixes

- **Skill reloads** no longer corrupt active tool calls or produce duplicate results in the conversation.
- **grok --resume** now correctly finds the real session instead of failing on empty image-only folders.
- Pasted images and relative paths now use the correct directory when resuming a session created elsewhere.
- **Mermaid flowcharts** now correctly render node groups, arrow endings, self-loops and line styles.
- **Fixed** "unknown session id" errors that occurred after the leader process crashed or was killed.
- **Pasted images** now survive interjections and queue edits instead of being dropped.
- **Managed MCP connectors** (Slack, Linear, etc.) now appear correctly when using leader mode.


# 0.2.47

## Features

- **stateDiagram** Mermaid blocks now render as diagrams instead of source fallback.

## Bug Fixes

- **Pasted images** now survive interjections and queue edits instead of being dropped.
- **Managed MCP connectors** (Slack, Linear, etc.) now appear correctly when using leader mode.


# 0.2.46

## Features

- **Mermaid flowcharts** now render with fewer avoidable edge crossings.

## Bug Fixes

- **Fixed `grok --resume`** failing on empty image-only session folders left by cross-directory pastes.
- **Fixed pasted images** and relative paths using the wrong directory after cross-cwd resume.
- **Fixed Mermaid flowcharts** that silently rendered wrong diagrams for & groups, circle/cross endings and self-loops.
- **Fixed zsh tab-completion** for subcommands after the optional prompt argument was added.
- **Fixed "unknown session id" errors** after the leader process crashed or was killed.
- **Fixed repeated auto-compaction attempts** when the session is credit-blocked or auth is non-refreshable.

## Performance

- **Parallel tool calls** on the same path (multiple greps etc.) now execute concurrently.


# 0.2.45

## Features

- **Mermaid diagrams** now render to images when you click Open in a code block (on by default).

## Bug Fixes

- **Fixed** rare conversation corruption when skills changed while a tool call was still running.
- **Fixed** `grok --resume` failing on empty image-only session folders left by cross-directory pastes.
- **Fixed** pasted images and relative paths using the wrong directory after resuming a session from another folder.
- **Welcome screen logo** no longer renders as invalid characters on legacy Windows command prompts and PowerShell.
- **Fixed** "unknown session id" errors that occurred after the leader process crashed or was killed.


# 0.2.44

## Features

- **K/J** keys now snap the viewport to the top of previous or next assistant responses.
- **J/K** (vim mode) now navigate between assistant responses in scrollback.
- **sequenceDiagram** mermaid blocks now render as Unicode lifeline diagrams instead of source fallback.

## Bug Fixes

- **Interjecting** while editing a queued prompt no longer strands the composer or blocks the queue.
- **Mid-turn interjections** now appear as separate user messages instead of being appended to tool results.
- **Project MCP config** touches no longer trigger repeated reload storms.

## Performance

- **Inference requests** recover faster from silent engine stalls instead of waiting the full idle timeout.


# 0.2.43

## Bug Fixes

- **ask_user_question** tool can now be enabled in allowlists without requiring plan-mode tools.
- **Shift+Tab** mode cycling (Normal → Plan → Auto-Approve) works again in the agent view.
- **Ctrl+C** now cancels a blocking `grok update` cleanly instead of leaving an orphaned download repainting the terminal.


# 0.2.42

## Bug Fixes

- **ask_user_question** tool can now be enabled in allowlists without requiring plan-mode tools.
- **MCP servers** provided at session start now persist across config hot-reloads.


# 0.2.41

## Features

- **Compaction completion message** now shows the before → after token reduction instead of only the final count.

## Bug Fixes

- **Fixed token count after compaction** so the displayed number no longer jumps back up on the next model response.
- **Fixed plugin skill loading** when a manifest lists skill directories directly instead of a parent skills/ folder.

## Performance

- **Fixed memory context injection** on resume so the prompt prefix stays byte-stable and KV cache is preserved.


# 0.2.40

## Features

- **`grok --debug`** now produces per-session log files under ~/.grok/debug/ even with a leader process.

## Bug Fixes

- **Doom-loop warnings** now correctly describe cycles and distinct edit failures instead of claiming identical arguments.
- **Model list changes** from config or cache now appear in already-connected TUI and IDE clients without restart.


# 0.2.39

## Features

- **run_terminal_cmd** can stream live stdout/stderr chunks when the workspace flag is enabled.
- **/session-info** now displays the current turn index.
- Server-synced and bundled skills are now discovered from launcher-injected directories.
- **Background `&` operator** is now allowed by default in terminal commands.

## Bug Fixes

- **Resumed subagents** no longer loop forever during auto-compaction on large context windows.
- **Background task** descriptions and & rejection messages now correctly name the real parameters.
- **Doom-loop detection** no longer falsely triggers on distinct failing tool calls.


# 0.2.38

## Features

- **Watching status line** now appears when background monitors, loops, or subagents can wake the agent.

## Bug Fixes

- **Default model selection** now correctly chooses the intended entry when multiple models share a slug.
- Minor bug fixes


# 0.2.37

## Features

- **MCP tool result queries** now list only command-line tools actually present on your system.
- **`grok update`** now restarts any older running leader so all clients get the new binary.
- **Long-running bash commands** that hit the timeout are now moved to the background by default instead of killed.

## Bug Fixes

- **Subagents** now correctly receive web_search and x_search tools from the parent session.


# 0.2.36

## Features

- **Large MCP tool results** are now saved with the correct extension and the model receives better hints for querying them.

## Bug Fixes

- **Fixed false-positive doom-loop terminations** when many parallel tool calls fail together in one batch.
- **Fixed a crash** that could occur during auto-compaction when resuming a session containing reasoning content.


# 0.2.35


# 0.2.34

## Features

- **`grok login`** now defaults to device code flow, which works reliably in SSH, WSL, VPN, and browser-restricted environments.

## Bug Fixes

- **Fixed a hang during auth refresh.**


# 0.2.33

## Bug Fixes

- **Fixed duplicate turn output** when attaching a second client to an active leader session.
- Fixed **Send now** on queued prompts.


# 0.2.32

## Features

- **Slash commands** from project plugins now appear correctly in every open conversation after a plugin change.

## Bug Fixes

- **Prompts submitted rapidly** now stay in correct submission order in the queue.

## Performance

- **Grep searches** on large repositories are now substantially faster and no longer hit the 60-second timeout.


# 0.2.31

## Bug Fixes

- **Marketplace skills** without proper descriptions are now hidden from listings instead of flooding the model with tables.
- **Prompts submitted rapidly** now stay in correct submission order in the queue.

## Performance

- **Grep searches** on large repositories are now substantially faster and no longer hit the 60-second timeout.


# 0.2.30

## Features

- A new plugin install suggestion appears above the prompt when you type a known marketplace plugin name or domain.

## Bug Fixes

- **Trace uploads** and remote session restores now succeed with a deployment key and no browser login.
- **Resumed sessions** no longer pad the sticky prompt with empty rows; cancelling a turn now keeps the rest of the prompt queue intact.
- Cancelling a running prompt no longer leaves the interface stuck on the cancelling spinner.


# 0.2.29

## Bug Fixes

- **`/rewind`** before a compaction boundary no longer leaves later prompts in context.

## Performance

- **Resuming large sessions** is now substantially faster with no data loss.


# 0.2.28

## Bug Fixes

- **Images** read via read_file are now downscaled even when small in bytes but large in pixels.
# 0.2.27

## Features

- **Image and video generation** tools now include the saved filename and session folder in their output.

## Bug Fixes

- **Monitor output** no longer appears as raw XML in the conversation view during leader sessions.
- **Windows commands** containing `&` are no longer incorrectly rejected by `run_terminal_cmd`.
- **Python -c** save-to-file reminder now suggests correct commands on Windows.


# 0.2.26

## Bug Fixes

- **Large pasted content** no longer triggers context-window errors or breaks compaction and memory flush.
- **API-key users** can now run `grok agent --leader` without forced interactive login or timeouts.
- **Compaction** no longer retries endlessly on credit, size, or auth failures; shows a clear message instead.
- **Windows PowerShell and cmd.exe** no longer falsely reject commands containing `&`.
- **web_fetch** no longer crashes the CLI on pages whose root element matches a cleaning selector.
# 0.2.25

## Bug Fixes

- **Session titles** now generate reliably even for very long initial messages.


# 0.2.24

## Bug Fixes

- Minor bug fixes
# 0.2.23

## Features

- **Leader sessions** can now be viewed and controlled from multiple clients with a live dashboard.
- **Sessions** can now be deleted directly from the /resume history picker.

## Bug Fixes

- **MCP plugin servers** with bundled OAuth client IDs now authenticate correctly.
# 0.2.22

## Bug Fixes

- **Authentication errors** with static API keys now surface a clear error instead of hanging the turn.


# 0.2.21

## Features

- **allowed_models** in config.toml now restricts which models appear in the picker and `/model` command.

## Bug Fixes

- **Code navigation** now returns correct results for secondary project windows with different working directories.
# 0.2.20

## Bug Fixes

- **MCP servers** declared in both a plugin's .mcp.json and plugin.json are now registered instead of dropped.
- **Git operations** now correctly target the repository for each session's working directory.
# 0.2.19

## Features

- **Monitors** now appear labeled in background-task reminders after compaction and can be terminated by name.

## Bug Fixes

- **Reading images** with text-only models no longer triggers repeated 400 errors that brick the session.


# 0.2.18

## Features

- **Official xAI plugin marketplace** now appears automatically in the Marketplace tab on first launch.
- **Image and video generation** now use api.x.ai directly for all users.
- **New image-to-video and reference-to-video tools** are now available for generating videos from images.
- **New imagine skill** provides prompt-craft and workflow guidance for image generation and editing tools.

## Bug Fixes

- **image_edit** now correctly resolves pasted or attached images referenced as [Image #N].
- **Background subagent completions** are no longer reported twice when the agent is idle.
- **Subagents** now use the same model as the parent session by default.


# 0.2.17

## Features

- **Image and video generation** tools now emit structured paths so the pager renders media without regex scraping.
- **Compaction summaries** now use a more detailed structure that improves recovery after context reset.
- **image_gen** can now be enabled via the harness model using [features] in config.toml or the GROK_IMAGE_GEN_HARNESS env var.
- **Improved config refresh** on new sessions from the shell.

## Bug Fixes

- **--restore-code** no longer detaches the source repository when resuming a forked-worktree session from a different directory.
- **Read tool** string coercion bug fixes.
- **ICO images** pasted or read from disk are now automatically converted to PNG before being sent to the model.
# 0.2.16

## Features

- **New segments compaction mode** writes per-segment markdown files that the model can read to recover pre-compaction detail.
- **Claude and Cursor compatibility scanning** (skills, rules, AGENTS.md) can now be toggled individually via env vars or config.toml.
- **grok inspect** now shows the resolved on/off state and source for every Claude/Cursor compatibility toggle.
- **Cursor MCP servers and hooks** are now discovered and can be disabled independently via GROK_CURSOR_MCPS_ENABLED / GROK_CURSOR_HOOKS_ENABLED.

## Bug Fixes

- **Streaming tool output** (bash/write_file) now renders completely in the pager instead of only the latest chunk.
- **Streaming bash tool output** now appears correctly in the pager scrollback.
- **Routing a native tool** (e.g. scheduler_create) through use_tool now gives a clear corrective error instead of an unrecoverable loop.
- **"Starting session..."** spinner no longer gets stuck when zero MCP servers are configured.
- **Subagents** now use the correct harness after switching models mid-session.
- **Fixed long startup delays** when an external auth provider binary hangs or fails.
- **Subagent conversations** no longer receive unrelated monitor events or background task completions from the parent.
- **The /loop command** now accepts natural-language intervals instead of always defaulting to 10 minutes.
- **Fixed blank output** on completed bash or code-execution cards after shell restart or reconnect.

## Performance

- **Large pasted images** no longer bust the prompt cache or exceed the 50 MiB request limit.


# 0.2.15

## Features

- **Permission prompts** now remember your last choice across tools and let you configure the first-prompt default in config.toml.


# 0.2.14

## Features

- **Generated images and videos** can now be opened directly from the terminal UI via buttons or clicks.
- **Background tasks panel** now groups items, supports collapsible sections, and has clearer styling for monitors and loops.

## Bug Fixes

- **Session titles** are now generated reliably using a fixed default model.
- **--permission-mode** now correctly overrides the permission_mode setting from config.toml when launching sessions.


# 0.2.13

## Bug Fixes

- Miscellaneous bug fixes


# 0.2.12

## Features

- **Computer connection status** now shows a connecting pill during terminal session initialization.
- **/check** and subagents now read and follow full AGENTS.md rules from the repo.

## Bug Fixes

- **--max-turns** now correctly counts tool-use cycles instead of total messages.
- **@-mention file search** now works again for local agent sessions.
- **Rendered images, files, and citations** now replay correctly in chunk-mode history.
- **`/context`** now displays the correct auto-compact threshold for the active model instead of always 85%.
- **Model responses** are no longer silently dropped when the gateway emits legacy channel values.
- **Prompt responses** no longer resolve before the turn's final output chunks reach the client.


# 0.2.11

## Bug Fixes

- Minor bug fixes
# 0.2.10

## Features

- **`/check`** has been renamed to **`/check-work`**; old command continues to work during transition.

## Bug Fixes

- **Images smaller than 8×8 pixels** are now rejected with a clear message instead of producing blocky results.


# 0.2.9

## Features

- **Added --device-code** as alias for device authentication and improved headless auth error messages.


# 0.2.8

## Features

- **New /login** slash command lets you re-authenticate from within a session without quitting.
- **Compaction summaries** now include the full transcript path so the model can reference prior details.
- **Cursor skills and rules** are now discovered alongside Grok and Claude directories.

## Bug Fixes

- **Fixed monitor tool** schema to show the correct 10-hour default timeout.
- **Fixed a panic** that could occur when installing marketplace plugins.


# 0.2.7

## Features

- **Image generation and image editing** can now be toggled independently via [features] in config.toml.

## Bug Fixes

- **Background tasks** started inside subagents now continue running after the subagent session ends.


# 0.2.6

## Bug Fixes

- **Background tasks** started inside subagents now continue running after the subagent session ends.
- **Image description** now reliably uses the grok-build model instead of falling back to the active session model.


# 0.2.5

## Bug Fixes

- **Drag-and-drop** and pasting images or files now works correctly on Windows.


# 0.2.4

## Features

- **image_gen** now uses the higher-quality grok-imagine-image-quality model.

## Bug Fixes

- **read_file** now correctly passes embedded base64 images to the model as vision tokens instead of truncated fragments.


# 0.2.3

## Features

- Memory system: /remember command, note modal with raw/enhanced preview, x.ai/memory/rewrite ACP extension, Ctrl+F fullscreen toggle for /memory modal.
- Agent configuration: /config-agents modal with agents, personas, and defaults.
- Goal classifier: end-to-end goal tracking with subagent-powered classification.


# 0.2.2


# 0.2.1

## Bug Fixes

- **Pasting or dropping images** now succeeds for truncated, CRC-corrupt, or tiny files instead of failing silently.


# 0.2.0

## Performance

- **Large chat sessions** now use substantially less memory and run faster during forks, rewinds, and compaction.


