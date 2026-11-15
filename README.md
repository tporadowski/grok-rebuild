# Grok Build fork

Unofficial fork of [Grok Build](https://github.com/xai-org/grok-build),
the terminal-based AI coding agent from SpaceXSI. The 1.0.45 source and
parts of this tree still say SpaceXAI. This repository is not affiliated
with SpaceXSI.

The program is a full-screen TUI that understands your codebase, edits files,
executes shell commands, searches the web, and manages long-running tasks —
interactively, headlessly for scripting/CI, or embedded in editors via the
Agent Client Protocol (ACP).

This tree starts from the Grok Build **1.0.45** source.
[`SOURCE_REV`](SOURCE_REV) records that monorepo commit
(`559751fdcec02d413e4c57c8832ab275e4f44980`). Two layers sit on top of it:

- **Fork additions**, listed in [What this fork adds](#what-this-fork-adds).
  These are this tree's own behavior.
- **Release-note changes** after 1.0.45, reimplemented from the official
  Grok Build release notes and tracked in
  [`docs/upstream-port/PLAN.md`](docs/upstream-port/PLAN.md). The package
  version is **1.0.87**, the stamp for how far those notes have been applied
  in this tree.

[What this fork adds](#what-this-fork-adds) ·
[Prebuilt binaries](#prebuilt-binaries) ·
[Building from source](#building-from-source) ·
[Documentation](#documentation) ·
[Repository layout](#repository-layout) ·
[Development](#development) ·
[Contributing](#contributing) ·
[License](#license)

## What this fork adds

These behaviors are this fork's own additions on the 1.0.45 source. The
release-note port above is separate from this list.

- **Copyable tool errors.** A collapsed Read, List, or Edit failure shows a
  short reason instead of a full path. Expanding a failed Edit also shows
  why it failed, the text it searched for, and the replacement it tried to
  apply. That error text can be selected and copied.
- **Session turn index.** Plain user prompts show the same 0-based turn
  number as `/session-info`, on the scrollback bubble (`Turn N`) and as the
  composer prefix. Bash, cron, and interjection prompts stay unlabeled.
- **Sticky Edit path.** When an expanded Edit diff is taller than the
  window, `Edit path/to/file` stays pinned at the top of that block while
  you scroll. This is separate from sticky user-prompt headers. Disable it
  with `[scrollback.blocks.edit] sticky_header = false` (default `true`).
- **Subagent session IDs.** A child session's ID is shown in its output
  window as soon as the session exists, and on the parent subagent row —
  including each member of a `Ran N subagents` group. Click the ID, or
  press `c` in the child window, to copy it.
- **Plugin hooks at session start.** Enabled, trusted plugin hooks are
  registered when a session starts, not only after a mid-session reload.
- **Full release notes.** `/release-notes` (alias `/changelog`) lists every
  shipped version, newest first, so a skipped upgrade is still in the
  modal. Notes published after this binary was built are prepended from
  the CDN. The welcome screen still shows only the current version's
  bullets.
- **Copy markdown source.** `Ctrl+Shift+Y` (or `F6`) copies an assistant
  reply as markdown — headings, bold, and code fences — not the rendered
  text. It copies the selected assistant message, or the latest one.
  Change the chord in Settings, or with `[ui].copy_markdown_shortcut`
  (`off` unbinds it).
- **Step through tool runs.** With the scrollback focused, `Shift+Down`
  collapses the tool section you leave and expands the next one.
  `Shift+Up` does the same toward the previous tool run. Successes,
  failures, and subagent rows are included. Messages and thinking are
  skipped. An open grouped header closes and the next tool section opens.
  An expanded tool body steps to the next visible tool and opens that
  body. Plain `Up`/`Down` still only move the selection, `Left`/`Right`
  still only fold the selected entry, and `Shift+Left`/`Shift+Right`
  still jump between turns.
- **Native Windows link.** Proto codegen skips the in-repo DotSlash
  `protoc` wrapper (it is not a Windows executable) and uses `protoc` on
  `PATH` or `$PROTOC`. Dependency output goes to a temp file, and the
  pager is linked with an 8 MiB main-thread stack so command-line parsing
  does not overflow.

A later merge of an upstream snapshot should keep this list.
[`/update-grok-local`](.grok/skills/update-grok-local/SKILL.md) records how.

## Prebuilt binaries

This fork does not publish release binaries yet.
[Build from source](#building-from-source).

SpaceXSI publishes official Grok Build binaries at [x.ai/cli](https://x.ai/cli).
That installer is their client. Their changelog is at
[x.ai/build/changelog](https://x.ai/build/changelog). The notes this tree has
followed are tracked in
[`docs/upstream-port/PLAN.md`](docs/upstream-port/PLAN.md).

## Building from source

Requirements:

- **Rust** — the toolchain is pinned by [`rust-toolchain.toml`](rust-toolchain.toml);
  `rustup` installs it automatically on first build.
- **[DotSlash](https://dotslash-cli.com)** — required so hermetic tools under
  [`bin/`](bin/) (notably [`bin/protoc`](bin/protoc)) can download and run.
  Install it and ensure `dotslash` is on your `PATH` **before** building:

  ```sh
  cargo install dotslash
  # or: prebuilt packages — https://dotslash-cli.com/docs/installation/
  /usr/bin/env dotslash --help   # sanity check
  ```

- **protoc** — proto codegen resolves [`bin/protoc`](bin/protoc) via DotSlash,
  or falls back to a `protoc` on `PATH` / `$PROTOC`.
- macOS and Linux are supported build hosts. This fork also links the pager
  natively on Windows; see [What this fork adds](#what-this-fork-adds).
  That path is not covered by upstream's release testing.

```sh
cargo run -p xai-grok-pager-bin              # build + launch the TUI
cargo build -p xai-grok-pager-bin --release  # release binary: target/release/xai-grok-pager
cargo check -p xai-grok-pager-bin            # fast validation
```

The binary artifact is named `xai-grok-pager`; official installs ship it as
`grok`. On first launch it opens your browser to authenticate — see the
[authentication guide](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

## Documentation

Official Grok Build documentation is at
[docs.x.ai/build/overview](https://docs.x.ai/build/overview). It describes
SpaceXSI's client.

The user guide that ships with this tree is in the pager crate:
[`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/)
— getting started, keyboard shortcuts, slash commands, configuration, theming,
MCP servers, skills, plugins, hooks, headless mode, sandboxing, and more.

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Composition-root package; builds the `xai-grok-pager` binary |
| `crates/codegen/xai-grok-pager` | The TUI: scrollback, prompt, modals, rendering |
| `crates/codegen/xai-grok-shell` | Agent runtime + leader/stdio/headless entry points |
| `crates/codegen/xai-grok-tools` | Tool implementations (terminal, file edit, search, ...) |
| `crates/codegen/xai-grok-workspace` | Host filesystem, VCS, execution, checkpoints |
| `crates/codegen/...` | The rest of the CLI crate closure (config, MCP, markdown, sandbox, ...) |
| `crates/common/`, `crates/build/`, `prod/mc/` | Small shared leaf crates pulled in by the closure |
| `third_party/` | Vendored upstream source (Mermaid diagram stack) — see below |

> [!IMPORTANT]
> The root `Cargo.toml` (workspace members, dependency versions, lints,
> profiles) is **generated** — treat it as read-only. Prefer editing per-crate
> `Cargo.toml` files.

## Development

```sh
cargo check -p <crate>        # always target specific crates; full-workspace builds are slow
cargo test -p xai-grok-config # per-crate tests
cargo clippy -p <crate>       # lint config: clippy.toml at the repo root
cargo fmt --all               # rustfmt.toml at the repo root
```

## Contributing

> [!NOTE]
> External contributions are not accepted. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

First-party code in this repository is licensed under the **Apache License,
Version 2.0** — see [`LICENSE`](LICENSE).

Third-party and vendored code remains under its original licenses. See:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git dependencies,
  bundled UI themes, and **in-tree source ports** (including openai/codex and
  sst/opencode tool implementations)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
  — crate-local notice for the codex and opencode ports (license texts +
  Apache §4(b) change notice)
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid-stack index
