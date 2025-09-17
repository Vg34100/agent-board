# Troubleshooting Guide

## Common Build Errors

### Error: `leptos_router` features not found
```
error: failed to select a version for `leptos_router`.
package `leptos_router` does not have feature `csr`
```
Solution: Remove feature flags from leptos_router:
```toml
leptos_router = { version = "0.7" }
```

### Error: Component callback types
```
error[E0277]: the trait bound `Callback<String, String>: From<{closure}>` is not satisfied
```
Status: Resolved in current codebase. Use simple function types or boxed closures in component props (`Box<dyn Fn(...)>`), and prefer signals for shared state.

### Error: `cargo tauri dev` not found
Problem: Running a Node/JS command in a Rust app
Solution: Use `cargo tauri dev` (not npm/yarn).

### Error: Build pipeline failures
```
error from build pipeline
cargo call returned bad status: exit code 101
```
Solution: Fix Rust compilation errors first, then restart.

## Hot Reload Issues

### Changes not reflecting
1) Stop the dev server
2) `cargo check`
3) `cargo tauri dev`

### WASM build failures
- Incorrect import paths
- Missing target/tooling
- Type mismatches in components

## CSS Issues

### Layout not full screen
```css
body { margin: 0; padding: 0; height: 100vh; overflow: hidden; }
.app { height: 100vh; width: 100vw; }
```

### Columns not equal width
```css
.kanban-board { display: flex; flex: 1; }
.kanban-column { flex: 1; }
```

## Component Issues

### Event handlers not working
- Ensure `on:click` syntax
- Check handler signature
- Ensure component is mounted

### Signals not updating
- Use `set_signal.set(value)`
- Use inside reactive context (`move ||`)
- Avoid circular updates

## Build Performance

### Slow compilation
- `cargo check` for fast feedback
- Keep incremental builds
- Only `--release` for final builds

## Project Persistence Issues

### Projects not saving/loading
Symptoms: Empty project list, header stuck on “Loading…”.
Solutions:
1) Confirm commands are in `generate_handler![]`
2) Parameter naming: frontend JSON must match backend parameters (camelCase)
3) Store plugin registered
4) Check browser console for invoke errors

### Tasks not persisting per project
Symptoms: Same tasks across projects, tasks disappear.
Solutions:
1) File naming: `tasks_{project_id}.json`
2) Load tasks on mount per project id
3) Save on create/delete/update

## Git & Worktrees

### Worktree creation fails on new repo (unborn HEAD)
Cause: No initial commit.
Fix: Initialize repo with README and first commit (the app does this via `initialize_git_repo`). For existing repos, create one commit before starting worktrees.

### Worktree removal fails / branch not deleted
Behavior: Branch cleanup is best‑effort; worktree folder is removed regardless. If branch deletion fails, it’s logged and ignored. You can manually delete the branch `task/{id}` in the main repo.

### Opening worktree in IDE fails (Windows)
- VS Code detection tries known install paths, then `code.cmd`, then `code`.
- Ensure VS Code is installed. Add to PATH or install via official installer.
- If corporate environments restrict PATH, prefer the full `Code.exe` install.

## Embedded Server / LAN

### Blank page in dev over LAN
- Ensure `dist/index.html` exists (run `trunk build` once). In release it’s embedded.
- Check logs for `Axum server: starting accept loop` and the self‑test line.

### Port already in use
- App falls back to a random high port. The title shows the final LAN URL.

### Browser clients don’t respond to actions
- Watch `POST /api/invoke cmd=...` logs in the terminal for errors
- The invoke shim sends both parsed `args` and an `args_string` fallback; inspect logs in `web.rs` for parse failures

### Events not updating in browser
- The UI uses SSE via `/api/events`. Check console for `SSE connection established`.
- Ensure no reverse proxy strips SSE headers if you front it (not typical for LAN).

## Agents (Claude/Codex)

### CLI not found
- Claude tries `claude`, `claude.exe`, `claude.cmd`
- Codex tries `codex.cmd`, `npx @openai/codex exec`, then `codex`
- Ensure the CLI is installed and on PATH; Windows may require running from a shell where the tool is installed.

### Output not appearing in UI
- Check terminal for parsed JSONL lines; non‑JSON lines are treated as text
- SSE events: `agent_message_update` should be logged in console
- For long runs, watch for `agent_process_status` updates at completion

## When All Else Fails
1) Clean rebuild: `cargo clean && cargo tauri dev`
2) Delete and re‑create the Tauri store files for a project if corrupted
3) Minimal reproduction: comment out new code and re‑add incrementally
4) Verify Axum is bound and index is served (`GET /health`)
5) Enable verbose logs: set `AGENT_BOARD_DEBUG=1`
