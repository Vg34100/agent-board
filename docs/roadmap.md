# Agent Board Roadmap

## Current Status
- Core app (Tauri + Leptos) is working
- Projects and 5‑column kanban per project are implemented
- Git worktrees per task are implemented and stable
- Agent processes (Claude/Codex) spawn and stream into the UI
- Persistence is complete across projects, tasks, agent messages/processes, and settings
- Embedded LAN server with HTTP invoke shim and SSE is in place

## Near‑Term Focus
1. Kanban UX
   - Drag & drop moves across columns
   - Keyboard shortcuts (Esc to close modals, etc.)
2. Review Workflow
   - Diff view tab in sidebar
   - Merge/PR flows leveraging worktree branches
3. System Integration
   - System tray polish; window state persistence
   - Optional auto‑start with OS
4. Agent Conversation (Not Yet Implemented)
   - Add reply input in the Task Sidebar to continue a task as a conversation
   - Wire to `send_agent_message` (updates the active `process_id`)
   - Persist messages; rely on SSE (`agent_message_update`) to stream updates

## Medium‑Term
4. Git Operations
   - PR creation (GitHub API)
   - Merge/rebase helpers with safe prompts
5. Agent UX
   - Rich rendering of read/edit events with inline diffs
   - Cost/turns summaries and per‑task history
6. Reliability/Perf
   - Structured logging with debug flag
   - Background cleanup of orphaned worktrees

## Long‑Term
7. Data & Config
   - Optional SQLite for larger datasets
   - Import/export and workspace templates
8. Security
   - Optional LAN auth token + QR pairing
   - Sandbox toggles for agent execution

## Success Criteria
- Users can create projects/tasks, persist data, and move tasks fluently
- Starting a task creates a worktree and an agent session in one click
- Review tab shows diffs and supports merge/PR actions
- LAN clients can operate the app reliably without native IPC

## Notes for Contributors
- See `docs/architecture.md` for end‑to‑end patterns (UI → command → HTTP/store).
- Prefer adding a Tauri command + web invoke mapping for new capabilities.
- Keep UI clean and programmer‑centric as per `ui-style-guide.md`.
