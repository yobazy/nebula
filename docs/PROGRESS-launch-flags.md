# Launch flags and plain-folder projects (overnight run f-nebula-flags, 2026-10-29)

Protocol support Observatory is waiting on (see `~/Programming/observatory/LATER.md`):

1. `CreateAgent.launch_flags`: a client can start a Claude session with
   `--permission-mode plan` (native plan mode) instead of a prompt postfix.
2. `AddProject.plain_folder`: a client can register a plain folder (no git)
   as a project, so a session can run there without `git init`.

## Plan

- [ ] Protocol: `launch_flags: Vec<String>` on `CreateAgent` (`#[serde(default)]`),
      `plain_folder: bool` on `AddProject` (`#[serde(default)]`), `plain_folder` on
      the `Project` entity. Bump `PROTOCOL_VERSION` 44 -> 45.
- [ ] Daemon: validate flags against a per-harness allowlist, pass them on the
      first spawn only, skip prewarm adoption when flags are set.
- [ ] Daemon: plain-folder projects (store column + migration), worktree sync
      skips them, worktree creation refuses them with a clear message.
- [ ] Tests: unit tests for validation and argv, registry tests for
      create/refuse, e2e test over the socket.
- [ ] Docs: protocol notes in docs/ and README where AddProject is described.

## Decisions

(filled in as I go)

## Done

## Try it

## Needs Bazil
