---
name: port-grok-release
description: >
  Port official Grok Build release notes into this fork. Compare the
  package version with the alpha channel pointer (stable only when the
  user asks), fetch each missing version's changelog, extend
  docs/upstream-port/PLAN.md with a feasibility call on every bullet,
  commit that plan, capture each version's official system prompt from
  a throwaway run, implement every new row in its own commit, and after
  each version stamp the lockstep packages and CHANGELOG.md. Push a
  `ci-v<version>` tag to this fork so GitHub Actions builds Linux,
  Windows, and macOS. When that run is green, delete the ci tag and
  push `v<version>`. The run finishes every version in the fetch. Use when
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
This skill overrides four of its session rules:

- Work **version by version, oldest first**, not one wave per session.
- Finish **every** version in this fetch before any report to the user.
  This command does not arm `/goal`. The host does not start another
  turn when you stop. A status message is the end of the run. After
  each commit, the next action is a tool call on the next row. Do not
  send the user a count of what remains, a partial report, or a
  handoff. The completion report is the last thing you send, and only
  after the last tag.
- After a version's new rows are all terminal, **do** stamp that version
  and update `CHANGELOG.md`, in their own commit. PLAN.md's "do not commit
  a version bump" applies to item commits, not to this boundary commit.
  The `vV` tag is the GitHub Actions gate in section 7, not this commit.
- D-223's "embedded changelog stays 1.0.45" is history. The stamp commit
  inserts the version's own notes into `crates/codegen/xai-grok-shell/CHANGELOG.md`.

Never decompile an official `grok` binary. Never commit one, and never
install one into the user's `~/.grok`. The only download is the throwaway
capture in section 5. Never push the branch unless the user asks. Never
push a tag to the xAI remote. The only tags this skill pushes on its own
are `ci-vV` to the fork, and `vV` to the fork after that workflow is
green. Never change git identity. If `git commit` fails because
`user.name` is unset, stop and ask. Do not set it yourself. An annotated
tag needs that identity too.

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

If `v<version>` is not on the fork, the gate in section 7 is not done.
Run it on the current `HEAD` before fetching anything newer. Do not
stamp again. A `v<version>` tag already on the fork stays, including
tags made before this gate existed.

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

If every new row is terminal, skip section 6 and still run section 5 and
section 7 for each version, oldest first. A version whose every bullet
was `skipped` for "Minor bug fixes." still gets its prompt capture,
changelog section, version stamp, ci- gate, and `v` tag, so the
embedded history does not grow a hole.

## 5. Capture the official prompt

Release notes do not mention the system prompt or the built-in user rule.
Those live in the official binary. For the package version already in the
tree, then for each version `V` being ported, oldest first, download that
binary into a throwaway directory and run it so it writes the rendered
prompt. Do this before section 6 for that `V`. The package-version run is
only the diff baseline. Do not commit it.

The artifact is one file, the same one
`crates/codegen/xai-grok-pager/scripts/install.sh` fetches.
Platform is `linux-x86_64`, `linux-aarch64`, `macos-x86_64`, or
`macos-aarch64` from `uname`. URL, on the same CLI base as section 1:

```text
<base>/grok-<V>-<platform>
```

Try that URL with a `.zst` suffix, then `.gz`, then the raw file. Put the
binary outside the repo. `chmod +x` it and require `--version` to run.
Do not point the installer at the user's home.

Run it with a fresh `GROK_HOME` and an empty working directory that is
not inside this repo or any other git checkout, so this tree's `AGENTS.md`
is not part of the prompt. Write `[cli]\nauto_update = false` into that
home's `config.toml` before the run. Do not set `HOME`. Do not pass
through `XAI_API_KEY`.

A signed-out binary exits before it creates a session, so it never writes
the prompt. Copy only `auth.json` from the user's grok home (the default
`~/.grok`, or `$GROK_HOME` when that variable is already set) into the
throwaway home, mode `600`. Do not copy the rest of that home, and do not
print the file. If `auth.json` is missing, record that and continue. A
short headless prompt is enough. A signed-in run waits for the model, so
bound the wait:

```bash
env -u XAI_API_KEY GROK_HOME="$HOME_DIR" timeout 90 "$BIN" -p "ping" || true
```

Then find, under that `GROK_HOME`:

- `system_prompt.txt` — the rendered system prompt, written at session
  start.
- `chat_history.jsonl` — the `<user_info>` preamble, written when the
  first prompt is accepted. The built-in user rule is the `<user_rule>`
  whose name is empty.

`system_prompt.txt` is one rendered expansion. Dates, the kernel line,
the shell, and the workspace path change every run. Branches that were
off (memory, for example) are absent. It is not `templates/prompt.md`.

Diff `V` against the previous capture after ignoring those per-run lines.
When the stable prose or the built-in user rule changed, edit the source
so a render can still produce every branch:

- `crates/codegen/xai-grok-agent/templates/prompt.md` keeps its `${{ }}`
  placeholders and `{% %}` branches. Add or rewrite the prose this render
  shows. Do not delete a branch merely because this run left it out, and
  do not replace the file with `system_prompt.txt`.
- `built_in_user_rules()` in
  `crates/codegen/xai-grok-agent/src/prompt/user_message.rs` when that
  sentence changed.

From `crates/codegen/xai-grok-agent` run
`python3 scripts/encrypt_templates.py`, then:

```bash
cargo test -p xai-grok-agent --lib -- test_encrypted_templates_not_stale
```

One commit when those sources changed, and only those files:

```text
port: take the V system prompt
```

No commit when the stable prose matches. Delete the throwaway home and
the binary when the diff is recorded. If `system_prompt.txt` was never
written, record that in the decision log and continue. Do not invent the
missing text.

## 6. Implement

For the version just captured, follow wave order from PLAN.md (W1, W2,
W3, W6, W4, W5, W7–W15), then catalog order. Implement every `pending`
row of that version before the stamp. Do not switch versions in the
middle of a wave, and do not end the run while any row from this fetch
is still `pending` or `in_progress`.

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

The only reasons to leave a row unfinished are the hard stops already
in this skill: a dirty tree that is not this run, a `404` hole, a `vV`
tag that points somewhere else, or git identity being unset. A release
target that does not compile is not one of those stops. Anything else,
including a long list, is still this turn. Commit the row, then start
the next one. The next row is **Next**
in PLAN.md, in wave order. Do not skip ahead to an easier later row and
then stop.

A compaction does not end the run and is not a place to write a status.
The first action after it is that same next row.

Windows-only or macOS-only notes: implement the shared code, run the
Linux tests, and record that the platform itself was not verified.

## 7. Stamp the version

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
implementation in this commit. Do not put the `vV` tag on this commit yet.

### Compile, then tag

Linux, Windows, and macOS compile in `.github/workflows/release.yml`
when a `ci-*` tag is pushed. A `win-*` tag rebuilds only Windows, so
it is not this gate. `v*` is the release tag, and it is pushed only
after the `ci-*` run is green.

GitHub runs the `.github/workflows/release.yml` in the tagged commit
and checks that commit out. That commit has to contain the file, with
the `ci-*` trigger. Detect the fork the way
`.grok/skills/update-grok-local/SKILL.md` detects it, by push URL, not
by remote name. Every `gh` command takes `--repo` of that fork. Do not
trust `gh`'s default repo: `upstream` is the gh-resolved base and is
`xai-org/grok-build`. Never push a tag there.

On the commit to release, with a clean working tree, the trial tag is
`ci-vV` (so `1.0.80` is `ci-v1.0.80`). Lightweight. When that tag is
already on the fork and points at this commit, leave it there. A run
still in progress: watch that run. A finished green run: drop the
trial tag and push `vV`. A finished red run: fix as below, which
deletes the tag before pushing it again. When the tag points at
another commit, drop it, then use the create path.

The create path is only for a `ci-vV` that is absent on the fork. A
local tag that already points at this commit is pushed as it is.

```bash
CI_TAG="ci-v$V"
git rev-parse -q --verify "refs/tags/$CI_TAG" >/dev/null || git tag "$CI_TAG"
git push "$FORK_REMOTE" "refs/tags/$CI_TAG"
```

Do not push the branch. Find the `push` run whose `headSha` is that
commit:

```bash
gh run list --repo "$FORK_REPO" --workflow release.yml --limit 20 \
  --json databaseId,headSha,status,conclusion,url,createdAt,event
```

The run can take a few seconds to appear. Stay with it until it
finishes. The jobs run up to 240 minutes.

```bash
gh run watch "$ID" --repo "$FORK_REPO" --exit-status
```

A queued or in-progress run is not a result. Green means that run's
conclusion is `success`: every job, which is Linux, Windows, and macOS.

Drop the trial tag with this, on success and before every retry.
Deleting the git tag leaves the GitHub Release, and an existing release
keeps its target commit, so the release has to go too. If the release
was never created, delete the remote tag ref.

```bash
if gh release view "$CI_TAG" --repo "$FORK_REPO" >/dev/null 2>&1; then
  gh release delete "$CI_TAG" --repo "$FORK_REPO" --yes --cleanup-tag
else
  git push "$FORK_REMOTE" ":refs/tags/$CI_TAG"
fi
git tag -d "$CI_TAG"
```

When the run is green, drop the trial tag, then tag and push `vV`.
Annotated, message `vV`, same shape as the existing `v1.0.78` tag.
Push only that tag.

```bash
git tag -a "v$V" -m "v$V"
git push "$FORK_REMOTE" "refs/tags/v$V"
```

That commit is the stamp when the first run was green, otherwise the
last compile-fix commit. Do not move a `vV` that already exists. If the
fork already has it at this commit, leave it. If it points somewhere
else, stop before the next version and report both commits. The `v*`
push starts the release workflow on the commit the `ci-*` run already
built. Do not wait for that second run before the next version. If
that later run fails, rerun its failed jobs with
`gh run rerun "$ID" --repo "$FORK_REPO" --failed`.

When a `ci-*` job fails, read `gh run view "$ID" --repo "$FORK_REPO" --log-failed`.
Fix the compile in this run and commit it by itself. No tag in that
commit.

```text
port: make V compile
```

Drop the trial tag, point `ci-vV` at the new commit, push it, and watch
the new run. A fix for one OS can break another. Do not push `vV`, and
do not start the next version, while the `ci-*` run is red. If the tag
push or `gh` is rejected, stop and report the error.

Then go back to section 5 for the next version. Do not stamp `V+1`
before `vV` is on the fork, which means its prompt capture is done, its
rows are terminal, and the `ci-vV` run was green. The completion report
comes after the last version in the fetch.

Do not rebuild `target/release/xai-grok-pager` unless the user asks.
This gate does not build it. The binary embeds the version at compile
time and stays on the previous stamp until that build.

## Completion report

Send this only after every version in the fetch is stamped, its `ci-*`
run is green, that tag is deleted, and `vV` is on the fork, or after a
hard stop. A row still `pending` or `in_progress`, or a `ci-*` run that
is not green, means you are not here: go back to section 6 or the
gate. Do not send these headings early. The previous run did, after one
item, and the session ended with the rest of the catalog open.

1. **Channel:** package version, stable pointer, alpha pointer, which
   channel was planned.
2. **Notes:** versions fetched, any `404`, any JSON/markdown mismatch.
3. **Plan:** commit, new row counts by status. Or "no newer release".
4. **Prompt:** for each version, the `port: take the V system prompt`
   commit, or "unchanged", or "system_prompt.txt was not written".
5. **Items:** each id → `done` / `already` / `skipped` / `deferred`,
   with the commit.
6. **Stamps:** version-bump commits, the `ci-vV` run URL and its
   conclusion, any `port: make V compile` commit, and the pushed `vV`
   tag, or "not reached". A tag that already pointed elsewhere is named
   here. A version with no `vV` on the fork did not pass the gate.
7. **Fork:** any protected behavior a note touched, and whether it
   stayed.
8. **Unpushed:** `git status -sb`. The `ci-vV` tag is gone. `vV` is on
   the fork. The branch is pushed only if the user asks.
9. **Skill drift:** one line. If a URL, file, or rule above was wrong
   on this run, say what to change. Otherwise say the skill matched
   the run.
