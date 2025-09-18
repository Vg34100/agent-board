# Agent Board

AI Kanban for orchestrating coding tasks with isolated git worktrees and agent processes. Desktop app (Tauri) with a Leptos/WASM UI, plus an embedded Axum HTTP server for LAN access.

## Highlights
- Projects and 5‑column kanban (ToDo, In Progress, In Review, Done, Cancelled)
- Per‑task git worktrees (git2) in app data; open folder / open IDE buttons
- Agent processes: spawn Claude Code or Codex in the worktree, stream output
- Persistence via `tauri-plugin-store` (projects, tasks, agent messages/processes, settings)
- Embedded HTTP server exposes the same UI on LAN; browser clients call Tauri commands via an HTTP invoke shim; SSE events for real‑time updates

## Architecture
- Frontend (Leptos/WASM)
  - Pages: `Projects`, `Kanban`
  - Components: task/project modals, task sidebar (agent chat/actions)
  - Compiled with Trunk; assets embedded with `rust-embed` in release
  - HTTP invoke shim in `index.html` maps `window.__TAURI__.core.invoke` to `POST /api/invoke` when running over http(s)
- Desktop Shell (Tauri)
  - `src-tauri/src/lib.rs` registers all commands and plugins
  - `tauri-plugin-store` for persistence; `tauri-plugin-opener` for OS integration
- Embedded Web (Axum)
  - `src-tauri/src/web.rs` serves UI and implements `POST /api/invoke` + `GET /api/events` (SSE)
  - Prefers port `17872`, falls back to an available port; shows LAN URL in window title
- Git Worktrees
  - `src-tauri/src/git.rs` uses git2 to create/remove worktrees and open locations/IDE
  - Worktrees live under app data: `<AppData>/com.video.agent-board/worktrees/<task_id>`
- Agent Runner
  - `src-tauri/src/agent.rs` spawns Claude or Codex CLI in the worktree, parses JSONL/text, persists messages/processes, broadcasts events
  - SSE event names: `agent_message_update`, `agent_process_status`

## Data Files (Store)
- `projects.json` → key `projects` (array)
- `tasks_{project_id}.json` → key `tasks` (array)
- `agent_messages_{task_id}.json` → key `messages` (array)
- `agent_processes.json` → key `processes` (array)
- `agent_settings.json` → key `settings` (object)

## Prerequisites
- Rust stable + target `wasm32-unknown-unknown`: `rustup target add wasm32-unknown-unknown`
- Tauri CLI: `cargo install tauri-cli`
- Trunk: `cargo install trunk`
- Git installed and available in PATH
- Claude or Codex CLI installed if you plan to use agent runs

## Development
- Start app: `cargo tauri dev`
- Force a clean dev: `cargo clean && cargo tauri dev`
- Optional verbose server logs: set `AGENT_BOARD_DEBUG=1`

Notes
- In dev, Axum serves `dist/` from disk; in release, files are embedded.
- The app window navigates to `http://127.0.0.1:<port>` so the desktop and browsers share the same UI/runtime.
- Browser clients use the HTTP invoke shim + SSE; they do not require native Tauri APIs.

## Using Worktrees
- Create a project (new or existing git repo). New repos are initialized with a README and initial commit.
- Create a task; press Start to create a worktree from the repository HEAD.
- Use “Open Folder” or “Open IDE” to inspect/edit the worktree.
- Removing a task can also remove its worktree; branches are cleaned up best‑effort.

## Agent Processes
- Choose a profile (Claude Code or Codex) per task.
- Start agent: spawns the respective CLI in the worktree with the task title/description as the initial prompt; messages stream to the sidebar.
- Multi‑turn: replying in the Agents tab continues the conversation by spawning a new process that carries forward context. The newest process appears immediately with `kind`/`start_time` and opens by default; older groups collapse automatically.
- Persistence: per‑process messages are stored under `agent_messages_{taskId}_{processId}.json` and hydrate on restart; a task‑level snapshot is also kept.
- UI listens to SSE (`agent_message_update`, `agent_process_status`) and keeps the view pinned to bottom while streaming if you’re already near the bottom.

## Troubleshooting
- “Unborn HEAD” when creating worktree: initialize repo (the app does this for new projects by creating README + initial commit).
- “Open IDE” on Windows: code detection tries multiple paths → `code.cmd` → `code`. Ensure VS Code is installed and in PATH.
- LAN blank page in dev: ensure `dist/index.html` exists (run at least one `trunk build` as part of dev).

## Docs
- See `docs/README.md` for the doc index, roadmap, and troubleshooting.
- See `docs/architecture.md` for deeper internals and extension points.
