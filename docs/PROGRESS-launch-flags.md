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
3. Plain folders still get nebula's managed hooks written into `<folder>/.claude/settings.local.json`,
   like any project. If Observatory's Home is your real home folder, that file is
   `~/.claude/settings.local.json`. Decide whether that's OK or whether Home should be a subfolder.
4. The TUI has no way to add a plain folder yet (it still offers `git init`). Add one if you want it.
5. `e2e_tui` has two tests broken on main (stale `^q: sessions` footer text). Worth a separate fix.
