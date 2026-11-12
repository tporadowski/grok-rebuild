---
name: port-grok-release
description: >
  Port official Grok Build release notes into this fork. Compare the
  package version with the alpha channel pointer (stable only when the
  user asks), fetch each missing version's changelog, extend
  docs/upstream-port/PLAN.md with a feasibility call on every bullet,
  commit that plan, implement each new row in its own commit, and after
  each version stamp the lockstep packages and CHANGELOG.md. Use when
  the user runs /port-grok-release, says "port the latest grok release",
  "update from release notes", "catch up to stable", "catch up to
  alpha", or asks to continue the upstream port plan from the official
  channel.
metadata:
  short-description: "Port official release notes into this fork"
---

# /port-grok-release — Port official release notes, keep fork behavior

xAI is not publishing `xai-org/grok-build` source. New behavior arrives as
release notes on the public channel. This skill turns those notes into
rows on the existing port plan, then implements them.

`docs/upstream-port/PLAN.md` is the only tracker and the only decision
log. Do not open a second plan. Read it from the top through the decision
log before editing it. Trust decisions already recorded.

Protected fork behavior lives in that file's W16 list and in
`.grok/skills/update-grok-local/SKILL.md` (fork themes). Read both before
judging a note. A note that would remove one of those behaviors is adapted
around the fork behavior, not taken as a deletion.

Follow PLAN.md for status words, the decision rules under "Decisions you
take while executing", catalog row shape, and the item commit subject.
This skill overrides three of its session rules:

- Work **version by version, oldest first**, not one wave per session.
- After a version's new rows are all terminal, **do** stamp that version
  and update `CHANGELOG.md`, in their own commit. PLAN.md's "do not commit
  a version bump" applies to item commits, not to this boundary commit.
- D-223's "embedded changelog stays 1.0.45" is history. The stamp commit
  inserts the version's own notes into `crates/codegen/xai-grok-shell/CHANGELOG.md`.

Never decompile an official `grok` binary. Never download or commit one.
Never push unless the user asks. Never change git identity. If `git commit`
fails because `user.name` is unset, stop and ask. Do not set it yourself.

Stay on the branch already checked out. Do not switch to `main`.

## Preconditions

Run from the repo root (the tree that contains `docs/upstream-port/PLAN.md`
and `crates/codegen/xai-grok-pager-bin`).

If the working tree is dirty with work that is not this run, stop and ask.
Do not stash or discard it.

Record before any edit:

```bash
git status -sb
git rev-parse --abbrev-ref HEAD
git rev-parse --short HEAD
```

## 1. Compare versions

Package version, the source of truth:

```bash
grep -E '^version = ' crates/codegen/xai-grok-version/Cargo.toml | head -1
```

Channel pointers (the body is a single version such as `1.0.77`):

```bash
curl -fsSL --max-time 20 https://x.ai/cli/stable
curl -fsSL --max-time 20 https://x.ai/cli/alpha
```

If `https://x.ai/cli` fails, use
`https://storage.googleapis.com/grok-build-public-artifacts/cli/stable`
and `.../alpha`. These are the same pointers `xai-grok-update` fetches
(`CLI_BASE_URL_PRIMARY`, then `CLI_BASE_URL_FALLBACK`).

**Channel:** `alpha`, unless the user explicitly asks for `stable`. Always
print both pointers. Plan only the selected channel.

Compare as semver, not as strings. There is a newer release only when the
selected pointer is greater than the package version.

- Selected pointer is not greater: print both pointers and the package
  version, and stop. Do not commit. Do not fetch notes for the other
  channel.
- Selected pointer is greater: continue. The versions to port are every
  published version strictly after the package version, up to and including
  the pointer.

**Which versions exist.** While the pointer stays on the same major.minor
as the package (the usual `1.0.N` line), walk the patch numbers from
package+1 through the pointer. If the major or minor changed, do not invent
the versions in between. Fetch the tip notes; if that file's own `# `
headings name the missing versions, use that list. Otherwise stop and ask.

A `404` for a version in the middle stops planning of every later version.
Report the hole. Do not stamp across it.

## 2. Fetch that version's own notes

For each version `V`:

```text
https://x.ai/cli/changelogs/V.external.md
https://x.ai/cli/changelogs/V.external.json
```

Use only `V`'s own section. A file that also contains older `# 1.0.x`
headings is bundling neighbors; ignore those sections. They are fetched
with their own version.

The JSON body is a list of `{category, description, breaking_change}`.
One object is one catalog row. `description` is the note text. Categories
seen on the channel are `features`, `fixes`, and `performance`.
`breaking_change: true` is a breaking change even when the category is
something else.

If the JSON is missing, split `V`'s own markdown into bullets under its
`## ` headings. If JSON and that markdown section disagree, use the
markdown bullets and record the mismatch in the decision log.

## 3. Assess every bullet, then extend the plan

Search this tree before assigning a status. The question is whether the
note is enough to implement the behavior here without inventing it.

| Result | Status | When |
|---|---|---|
| Implement | `pending` | The note names an observable behavior, and the subsystem is already in this tree. |
| Present | `already` | The final behavior is already here. Cite the file in a new decision. Do not rebuild it. |
| Not a behavior | `skipped` | "Minor bug fixes." or any note that does not say what changed. |
| Reversed | `skipped` | A later note in this fetch replaces it. Set `covered` to the later id. |
| Not implementable from the note | `deferred` | The behavior is real, but the note, the missing subsystem, or a protocol this tree does not already speak is not enough. Say what is missing. Do not invent it. |
| Fork conflict | `pending` or `deferred` | Read W16 and the update-grok-local fork themes. Keep the fork behavior. Leave `pending` only when the note can be adapted around it, and write that constraint in a decision. `deferred` when it cannot. If the note **is** the fork behavior, use `already`. |

Do not ask the user about a choice this table covers.

**Row shape** (same line the catalog already uses):

```text
id | status | wave | kind | flag | covered | note
```

- `id`: `v{patch}-{letter}{nn}`. `78` for `1.0.78`. Letter: `b` breaking,
  `f` features, `x` fixes, `p` performance. Number from `01` inside that
  version and letter. If that prefix already exists, continue after the
  highest number. Never reuse an id.
- `kind`: `Breaking Changes`, `Features`, `Bug Fixes`, or `Performance`,
  matching the catalog.
- `flag`: empty, unless the row is a skip. Then a short reason.
- `covered`: empty, unless a later row replaces this one.
- `note`: the changelog bullet text.

**Wave:** put the row in the existing wave whose "Look first" owns that
subsystem (W1 breaking, W2 CLI, W3 permissions and config, W6 memory, W4
hooks, W5 workflows, W7 worktrees, W8 sessions, W9 prompt and keyboard,
W10 transcript, W11 headless and tools, W12 MCP and images, W13 update
and sign-in, W14 remaining pager bugs, W15 performance). A leftover bug
is W14. A performance note is W15. Record a decision if the wave is a
guess.

Insert each new row into its wave's block in the catalog. Do not rewrite
rows that are already `done`, `already`, or `skipped`. A later note that
reverses a `done` row is a new row; the old row stays `done`, with a
decision pointing at the new id.

Then:

- Extend the plan title and opening paragraph so the covered range includes
  the new versions and names the channel and the date of this fetch.
- Recompute the summary table and each touched wave's counts.
- Set **Next** to the oldest new version's first `pending` id, or to
  "no pending rows" when every new row is `already`, `skipped`, or
  `deferred`.
- Append one decision: channel, versions fetched, and the counts of
  `pending` / `already` / `skipped` / `deferred`. Number it as the next
  `D-` id. Do not renumber.

## 4. Commit the plan

One commit, plan file only. No code, no version bump, no changelog.

```text
port: plan <first> through <last> from <channel>
```

Body: one line per version with its new-row counts. Wrap near 72 columns.

If every new row is terminal, skip implementation and still make the
stamp commits in section 6 for each version that has notes, oldest first.
A version whose every bullet was `skipped` for "Minor bug fixes." still
gets its changelog section and version stamp, so the embedded history
does not grow a hole.

## 5. Implement

Take the oldest version that still has `pending` rows. Inside it, follow
wave order from PLAN.md (W1, W2, W3, W6, W4, W5, W7–W15), then catalog
order.

For each `pending` row:

1. Search again. Reclassify to `already`, `skipped`, or `deferred` when
   section 3's table says so. Implement only when it stays `pending`.
2. Match the surrounding code. No new crate, no new framework.
3. Set the status word in the catalog in the same change. Append a
   decision when the note does not map cleanly, when a fork behavior is
   touched, and whenever the result is `deferred` or `already`. A
   straightforward implementation needs no decision.
4. Test the crate you changed: `cargo test -p <crate> -- <filter>`.
   Cargo takes one filter before `--`. Extra names go after `--`.
   Use the workspace toolchain in `rust-toolchain.toml`. If a build
   script cannot run `bin/protoc` because `dotslash` is missing, put
   `dotslash` on `PATH` and retry. Do not claim a test passed without
   its output.
5. This is a terminal app. Do not use a browser. Use unit tests. Add a
   pty scenario in `xai-grok-pager-pty-harness` only when the note is
   about keys, focus, or layout and there is no unit seam.
6. Commit that row by itself. Subject: `port: <id> <few words>`.
   A tracker-only outcome (no product code) is still its own commit.
   Do not put two rows in one commit. Do not put the version stamp in
   an item commit.

Clear `in_progress` before the session ends. If you stop mid-version,
commit finished rows first, refresh the summary and **Next**, and name
the next id in the report.

Windows-only or macOS-only notes: implement the shared code, run the
Linux tests, and record that the platform itself was not verified.

## 6. Stamp the version

When every row you added for version `V` is `done`, `already`, `skipped`,
or `deferred`:

1. Set `version` to `V` in:
   - `crates/codegen/xai-grok-version/Cargo.toml`
   - `crates/codegen/xai-grok-shell/Cargo.toml`
   - `crates/codegen/xai-grok-pager/Cargo.toml`
   - `crates/codegen/xai-grok-pager-bin/Cargo.toml`
   - the matching `version` fields of those four packages in `Cargo.lock`
2. Point `package_version_is_the_ported_release` in
   `crates/codegen/xai-grok-version/src/lib.rs` at `V`
   (`CARGO_PKG_VERSION` and `VERSION`).
3. Insert `V`'s own section into
   `crates/codegen/xai-grok-shell/CHANGELOG.md`, directly under
   `# Changelog`, newest first. Copy the `# V — date` heading and the
   body from that version's own `.external.md`. Leave neighbor sections
   out. Keep the file's gap of two blank lines between H1 sections.
4. Append a decision that the package version and the embedded changelog
   now include `V`. `/release-notes` still prepends CDN `# ` sections
   whose headings are absent from that file.
5. Update the plan's "through" line if `V` is now the latest ported
   release. Refresh **Next**.

One commit:

```text
port: stamp package version V
```

Body: say the embedded changelog gained `V`'s own section. No item
implementation in this commit.

Then start section 5 on the next version. Do not stamp `V+1` before its
rows are terminal.

Do not rebuild `target/release/xai-grok-pager` unless the user asks. The
binary embeds the version at compile time and stays on the previous
stamp until that build.

## Completion report

1. **Channel:** package version, stable pointer, alpha pointer, which
   channel was planned.
2. **Notes:** versions fetched, any `404`, any JSON/markdown mismatch.
3. **Plan:** commit, new row counts by status. Or "no newer release".
4. **Items:** each id → `done` / `already` / `skipped` / `deferred`,
   with the commit. Name anything left `pending`.
5. **Stamps:** version-bump commits, or "not reached".
6. **Fork:** any protected behavior a note touched, and whether it
   stayed.
7. **Unpushed:** `git status -sb`. Push only if the user asks.
8. **Skill drift:** one line. If a URL, file, or rule above was wrong
   on this run, say what to change. Otherwise say the skill matched
   the run.
