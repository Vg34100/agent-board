# Agent Board Documentation

## Documentation Index

This folder is the living source of truth for how Agent Board works today and how to extend it.

Key docs
- [architecture.md](./architecture.md) — Deep internals: data flow, modules, events, extension points
- [feature-pattern.md](./feature-pattern.md) — Standard template to add new features (UI → command → HTTP/store)
- [roadmap.md](./roadmap.md) — Current status, priorities, and future phases
- [troubleshooting.md](./troubleshooting.md) — Common issues and fixes
- [store.md](./store.md) — Persistence patterns with `tauri-plugin-store`
- [lan-mode.md](./lan-mode.md) — Embedded HTTP server, invoke shim, and LAN usage
- [agent-events.md](./agent-events.md) — SSE events and message payloads
- [ui-style-guide.md](./ui-style-guide.md) — Visual and UX conventions
- [development-log.md](./development-log.md) — Running log of findings and decisions
- [immediate-next-steps.md](./immediate-next-steps.md) — Near‑term tasks and recently fixed items

## Current Status

Completed
- Desktop app (Tauri) with Leptos UI
- Projects and per‑project kanban boards (5 columns)
- Per‑task git worktrees (git2) stored in app data; open folder / open IDE actions
- Full persistence via store: projects, tasks, agent messages, agent processes, settings
- Agent processes: spawn Claude Code or Codex in the worktree; messages stream into sidebar
- Embedded Axum server that serves the same UI on LAN; HTTP invoke shim + SSE for events

Next up
- Drag & drop task moves
- Review/merge/PR flows from worktrees
- System tray polish and app automation

## Quick Start

Commands
```bash
cargo tauri dev                    # Run desktop + embedded server
cargo check                        # Fast type/compilation check
cargo clean && cargo tauri dev     # Clean rebuild if needed
```

Prereqs
- Rust stable, `wasm32-unknown-unknown` target
- `tauri-cli`, `trunk` installed
- Git installed; VS Code for IDE open action on Windows

## Project Structure

```
agent-board/
  src/                    # Leptos/WASM UI
    app.rs                # App root; dev banner; view switching
    pages/                # Projects, Kanban
    components/           # Modals and sidebar (agent chat & actions)
    models/               # Project, Task, enums
  src-tauri/              # Tauri + backend
    src/
      lib.rs              # Commands, plugins, setup (port + LAN title)
      web.rs              # Axum: /, /api/invoke, /api/events (SSE)
      git.rs              # Worktrees, open folder/IDE helpers
      agent.rs            # Spawn/track agents (Claude/Codex), parse output
  dist/                   # Built assets (embedded in release)
  docs/                   # This documentation
  index.html              # HTTP invoke shim + SSE bridge
  styles.css              # Minimal, programmer‑focused styling
```

## Design Principles
- Programmer‑centric, minimal UI (see `ui-style-guide.md`)
- Clean separation: UI (Leptos) ↔ commands (Tauri) ↔ services (Axum/git/agents)
- Everything evented: agent output broadcasts over SSE to UI

## Notes
- New repos are initialized with README + initial commit to support worktrees.
- LAN usage is unauthenticated by default — suitable for local networks only.
- See `architecture.md` for adding a new feature end‑to‑end (UI → command → store/HTTP).
