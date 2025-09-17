# Architecture

This document explains how the app is wired end‑to‑end so new contributors can confidently add features.

## Overview

- Desktop shell (Tauri) hosts the app and plugins.
- Embedded Axum server serves the same Leptos UI and an HTTP API for browsers on the LAN.
- Leptos/WASM UI calls Tauri commands directly (desktop) or via an HTTP invoke shim (browser).
- Git worktrees isolate task work.
- Agent processes (Claude/Codex CLI) run inside the worktree; output is parsed and broadcast over SSE to update the UI.

## Processes and Ports

- Tauri app boots and binds an Axum server:
  - Preferred port: `17872` (falls back to a random high port if taken)
  - LAN URL shown in window title (e.g., `agent-board - http://192.168.1.23:17872`)
- Health check: `GET /health` → `ok`

## Frontend (Leptos)

Paths
- `src/app.rs` — App root, dev banner, view switching
- `src/pages/{projects,kanban}.rs` — Views
- `src/components/*` — Modals, sidebar, UI elements
- `index.html` — HTTP invoke shim + SSE bridge for browser clients

Key ideas
- Calls `window.__TAURI__.core.invoke(cmd, args)` everywhere.
  - In desktop, native Tauri invoke is available.
  - In browser, `index.html` shims invoke to `POST /api/invoke`.
- Eventing via SSE:
  - Browser clients listen to `GET /api/events` for `agent_message_update` and `agent_process_status`.
  - Desktop webview also uses the HTTP/SSE path for consistency and to avoid restricted IPC on http origins.

## HTTP Server (Axum)

Paths
- `src-tauri/src/web.rs`

Responsibilities
- Serve `index.html` and static assets from `dist/` (embedded in release, on‑disk in dev).
- Implement `POST /api/invoke` to map `cmd` → Tauri commands in `lib.rs`.
- Expose `GET /api/events` (SSE) to broadcast events to browsers.

Utilities
- `debug_log()` honors `AGENT_BOARD_DEBUG=1` and suppresses noisy devtool requests.

## Tauri Commands

Paths
- `src-tauri/src/lib.rs`

Registered commands (selection)
- Filesystem/dir: `list_directory`, `get_parent_directory`, `get_home_directory`, `create_project_directory`
- Git: `initialize_git_repo`, `validate_git_repository`, `create_task_worktree`, `remove_task_worktree`, `open_worktree_location`, `open_worktree_in_ide`, `list_app_worktrees`
- Store: `load_*`/`save_*` for `projects`, `tasks`, `agent_messages`, `agent_processes`, `agent_settings`
- Agents: `start_agent_process`, `send_agent_message`, `get_process_list`, `get_process_details`, `get_agent_messages`, `kill_agent_process`
- Misc: `is_dev_mode`

Setup
- Plugins: `tauri-plugin-store`, `tauri-plugin-opener`
- On setup:
  - Binds listener (17872 or fallback)
  - Spawns Axum
  - Navigates desktop window to `http://127.0.0.1:<port>`
  - Emits `server_info` (optional)

## Git Worktrees

Paths
- `src-tauri/src/git.rs`

Behavior
- Worktrees root lives in app data under `worktrees/`.
- For task `{id}`:
  - Creates branch `task/{id}` from repository HEAD
  - Adds a named worktree in the app data folder
  - Removal cleans the folder and best‑effort deletes the task branch in the main repo
- Utilities to open the folder or the IDE; on Windows tries known VS Code locations, `code.cmd`, then `code`.

## Agents

Paths
- `src-tauri/src/agent.rs`

Behavior
- Spawns Claude or Codex CLI in the task worktree.
- Parses JSONL/text output to a normalized `AgentMessage` stream.
- Tracks processes and messages in memory, persists to store, and broadcasts updates via SSE.

Events
- `agent_message_update` — payload includes `process_id`, `task_id`, and the serialized `AgentMessage`.
- `agent_process_status` — payload includes `process_id` and status (`running`, `completed`, `failed`, `killed`).

Profiles
- Claude: tries `claude`, `claude.exe`, `claude.cmd` with flags to allow edits and stream JSON; warns if not found in PATH.
- Codex: tries `codex.cmd`, falls back to `npx @openai/codex exec`, then `codex` binaries.

## Persistence

Store files (keys)
- `projects.json` (`projects`)
- `tasks_{project_id}.json` (`tasks`)
- `agent_messages_{task_id}.json` (`messages`)
- `agent_processes.json` (`processes`)
- `agent_settings.json` (`settings`)

Frontend rules
- Always invoke Tauri commands; do not access store directly from WASM.
- Serialize arrays of objects carefully; `index.html` contains a Map→object fix for WASM types when running over HTTP.

## Adding a New Feature

Pattern
1) Define a Tauri command in `src-tauri/src/lib.rs` that returns/accepts `serde_json::Value` or typed data.
2) Register it in `invoke_handler!`.
3) If needed on LAN, extend `POST /api/invoke` dispatcher in `web.rs`.
4) Call it from Leptos via `invoke(cmd, serde_wasm_bindgen::to_value(&args)?)`.
5) Persist via store commands where appropriate.
6) If it emits updates, broadcast SSE via `web::broadcast_to_http`.

Tips
- Prefer simple, typed payloads at the command boundary and convert to JSON values near the store.
- Keep noisy logs behind `AGENT_BOARD_DEBUG` or only during development.
- Add troubleshooting notes for any OS‑specific behavior (e.g., Windows path probing).

