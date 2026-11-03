# Launch flags and plain-folder projects (overnight run f-nebula-flags, 2026-10-29)

Protocol support Observatory is waiting on (see `~/Programming/observatory/LATER.md`, the items on
native plan mode and Home as a plain folder):

1. `CreateAgent.launch_flags`: a client can start a Claude session with `--permission-mode plan`
   (native plan mode) instead of adding a prompt postfix.
2. `AddProject.plain_folder`: a client can register a folder that is not a git repo as a project,
   so a session can run there without `git init`.

Branch `ov/f-nebula-flags`, pushed as `launch-flags`. Draft PR: https://github.com/yobazy/nebula/pull/1
(on the fork; a bare `gh pr create` picks the `upstream` remote AgentSystemLabs/nebula, so it was
opened with `--repo yobazy/nebula`).

## Plan / status

- [x] Protocol: `launch_flags: Vec<String>` on `CreateAgent`, `plain_folder: bool` on `AddProject`,
      both `#[serde(default)]`; `plain_folder` on the `Project` entity (also `#[serde(default)]`).
      `PROTOCOL_VERSION` 44 -> 45.
- [x] Allowlist module `crates/nebula-core/src/launch_flags.rs` (`validate`, `plan_mode`).
- [x] Daemon: validate flags in `create_agent`, pass them on the first spawn only
      (`agent_spawn_command_flagged`), skip prewarm adoption when set, refuse beside Claude Cloud.
- [x] Daemon: plain folders. Store column `projects.plain_folder` (migration 29),
      `add_project_with(.., plain_folder)`, worktree sync skips them, `create_worktree` and
      `pr_worktree` refuse them with "is a plain folder, not a git repository".
- [x] TUI: compiles against the new fields (sends empty/false); a root row with no branch reads
      `folder` on the band rule and the pane tab.
- [x] Tests: allowlist unit tests, old-wire-shape and JSON decode tests (protocol.rs), registry
      tests (allowlist refusals, no prewarm adoption, argv order, plain folder end to end), and an
      e2e socket test where a stub `claude` records `--permission-mode plan` in a plain folder.
- [x] Docs: new section at the end of `docs/sessions.md`.

## Decisions (made unattended)

- **Protocol bump to 45.** The repo bumps whenever "a daemon must know X before a client can send
  it" (see v42, v43). Without a bump, a new client talking to an old daemon would have plan mode
  silently dropped. The handshake is exact-match, so Observatory needs to repin `nebula-core` to the
  release that ships this (it already does that on every nebula upgrade).
- **Old request shapes still decode.** Both new fields are trailing and `#[serde(default)]`, so a
  request built without them (positional msgpack or Observatory's JSON) decodes with flags empty
  and `plain_folder: false`. Tested in `protocol.rs`.
- **Flags are argv words, not a typed enum.** `Vec<String>` lets the daemon refuse an unknown flag
  with a clear reason instead of failing to decode the whole request, and adding a flag later is
  an allowlist change, not a wire change. They never reach a shell as text: they go into argv
  like `--model`, and the login-shell wrapper quotes them.
- **Allowlist is small on purpose.** Claude only: `--permission-mode` with `plan`, `acceptEdits`,
  `auto`, `manual`, `default`. `bypassPermissions` and `dontAsk` are refused so no client can start
  a session that skips permission prompts. Other harnesses refuse any flag for now. Values were
  taken from `claude --help` on this machine.
- **Flags apply to the first spawn only (not persisted).** A restart, resume, relocation or
  failed-resume respawn boots without them. Re-entering plan mode after the user approved a plan
  would be wrong. Trade-off: a daemon restart before the plan is approved resumes in normal mode.
- **Plain folder is an explicit opt-in.** A path inside a repo is still added as that repo. A `.git`
  in the folder or above it always takes the repo path, so a git hiccup can't make a repo plain.
  With `create_missing` too, a missing folder is created (no `git init`).
- **A plain folder stays plain.** The worktree sync skips it even if someone `git init`s it later.
  Remove and re-add it to make it a repo project.
- **Root row branch is the empty string.** The TUI shows `folder` for it.

## Try it

```sh
cd /data/tmp/overnight-2026-10-29/wt/f-nebula-flags
cargo test -p nebula-core launch_flags
cargo test -p nebula-core protocol
cargo test -p nebula-daemon --lib -- launch_flags plain_folder
cargo test -p nebula --test e2e_pty plain_folder
```

From a client (JSON as Observatory builds it):

```json
{"AddProject": {"req_id": 1, "path": "/Users/you/Home", "name": "Home", "create_missing": false, "plain_folder": true}}
{"CreateAgent": {"req_id": 2, "worktree": "<root row id>", "name": "plan", "kind": "claude",
  "model": null, "effort": null, "auto_title": true, "starting_prompt": "Plan the move",
  "launch_flags": ["--permission-mode", "plan"]}}
```

## Test status

`cargo test --workspace --no-fail-fast`: everything passes except tests that also fail or flake
without this branch:

- `e2e_tui`: `nebula_open_from_inside_a_session_raises_the_file_tabs` and
  `tui_drag_past_the_pane_top_autoscrolls_and_copies_the_run` wait for the footer text
  `^q: sessions`, which no longer appears anywhere in the TUI source. Already broken on main.
- `tunnel::tests::no_port_resolves_to_one_that_is_free` (AddrInUse while other runs held ports),
  `branch_switch::tests::enter_with_nothing_matching_creates_the_typed_branch` and
  `upgrade::tests::reads_the_protocol_of_the_first_nebula_on_path` (its stub prints a fixed `44`,
  not the constant) each failed once under load and pass when run on their own.

The installed nebula daemon and binary on this machine were not touched; every daemon in the tests
is a temp one on its own socket.

## Needs Bazil

1. Merge the PR, cut a release, then repin Observatory's `nebula-core` tag (protocol 45) and drop
   the plan postfix and the "Home needs to be a git folder" block. Its mock (`src/nebula/mock.ts`)
   needs the two fields too.
2. Is the allowlist right? Currently `--permission-mode` with plan / acceptEdits / auto / manual /
   default, Claude only. Add more flags (for example `--add-dir`) or other harnesses
   (Codex `--sandbox`) only when Observatory needs them.
3. ~~Plain folders still get nebula's managed hooks written into `<folder>/.claude/settings.local.json`.~~
   Done (decision 30B, 2026-10-30): Claude's hooks for a plain folder live in nebula's data dir.
   Cursor and custom harnesses still write into the folder; see the section below.
4. The TUI has no way to add a plain folder yet (it still offers `git init`). Add one if you want it.
5. `e2e_tui` has two tests broken on main (stale `^q: sessions` footer text). Worth a separate fix.

## Decisions applied 2026-10-30

Run `nebula` on 2026-10-30, worktree branch `ov/nebula`, pushed to `launch-flags`.

- **29A, keep the allowlist tight.** No change: Claude takes only `--permission-mode` (plan,
  acceptEdits, auto, manual, default); `bypassPermissions` and `dontAsk` stay refused.
- **30B, hooks for a plain-folder Home go to a subfolder, not the user's folder.** Changed.
  - Location: `<nebula data dir>/plain-folders/<project id>/claude-settings.json`
    (`~/.local/share/nebula/...` on Linux, `~/Library/Application Support/dev.nebula.nebula/...`
    on macOS). Why there: it is a folder nebula already owns, so nothing lands in the user's folder
    (if Home is `~`, the old path was the user's global `~/.claude/settings.local.json`); it is per
    project, so two plain folders never share a file; and `NEBULA_DATA_DIR` isolates it in tests and
    parallel daemons like every other nebula file. A subfolder inside Home (e.g. `Home/.nebula/`)
    was rejected because it would still write into the user's folder and Claude would not read it
    without `--settings` anyway.
  - How Claude finds it: every Claude spawn in a plain folder (first spawn, restart, resume,
    prewarm) passes `--settings <file>`. Claude merges hooks and `permissions.allow` from that file
    with the user's own settings, so user settings are untouched. The launch flags still apply to
    the first spawn only; `--settings` rides every spawn.
  - The file is rewritten (strip and rebuild nebula's groups) on every spawn, like the worktree
    copy, and `RemoveProject` deletes its folder. Repo projects are unchanged.
  - Code: `hooks::installer::{plain_folder_settings_dir, install_claude_hooks_outside}`,
    `Daemon::spawn_agent_pty`, `Daemon::remove_project`. Docs: end of `docs/sessions.md`.
  - Tests: `plain_folder_hooks_stay_out_of_the_folder` (installer unit test: same hooks and allow
    rules as a worktree, folder untouched, reinstall idempotent) and the e2e
    `plain_folder_project_runs_claude_with_launch_flags`, extended to check the stub `claude`
    receives `--settings <data dir>/plain-folders/<id>/claude-settings.json`, the file holds the
    hooks, `<folder>/.claude` does not exist, and removing the project deletes the file.
  - Left as is: Cursor sessions in a plain folder still write `<folder>/.cursor/hooks.json` and
    `.cursor/rules/nebula-title.mdc` (cursor-agent has no `--settings` equivalent), and a custom
    harness using the Claude hook dialect still writes `<folder>/.claude/settings.local.json`
    (its program may not accept `--settings`). Observatory only starts Claude there today.
- **31A, leave the TUI as is.** No change: the TUI still cannot add a plain folder.
- **32A, the PR stays on the fork.** No change: https://github.com/yobazy/nebula/pull/1.
- **3A, sentence case everywhere.** The new copy (docs paragraph, comments) has no all-caps
  letter-spaced labels. The capitalised words in `docs/sessions.md` (PLAIN FOLDER, DATA DIR, MANAGED
  HOOKS) are the repo's existing glossary-term convention, not labels, so they were kept.

Checks run (all on this machine, temp daemons only; the installed nebula daemon and binary were
not touched):

- `cargo test -p nebula-core -p nebula-daemon -p nebula-tui --no-fail-fast`: 43 + 320 + 1266 passed.
- `cargo test -p nebula --test e2e_pty`: 32 passed.
- `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets`: 11 warnings, all in files
  this branch does not touch (lifecycle.rs, pty/kitty.rs, nebula-tui event_loop/markdown/ui).
- `e2e_tui` not rerun; its two known failures on main are unrelated.

Try it:

```sh
cargo test -p nebula-daemon --lib -- plain_folder
cargo test -p nebula --test e2e_pty plain_folder
```

### Needs Bazil (2026-10-30)

1. Merge the PR, cut a release, and repin Observatory's `nebula-core` (protocol 45), as before.
2. Cursor in a plain folder still writes `.cursor/` into the folder. Fine while Observatory only
   runs Claude there; say if it should run hookless or refuse instead.
